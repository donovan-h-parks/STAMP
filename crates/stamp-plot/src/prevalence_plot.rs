//! Prevalence against mean abundance (an occupancy-abundance plot): one
//! point per feature, placed by its mean relative abundance (logarithmic
//! axis) and the percentage of samples in which it is present.
//!
//! Three views answer "which features belong to one group and which are
//! found everywhere":
//!
//! * **one panel per group**, on the same axes, so that a feature can be
//!   followed from group to group (highlighted features are labelled in
//!   every panel);
//! * **all groups in one panel**, coloured by group, or only one chosen
//!   group;
//! * **specificity**: each feature once, at the group where it is most
//!   prevalent, coloured by that group if its specificity (prevalence there
//!   minus its highest prevalence in any other group) reaches a threshold
//!   and grey otherwise; coloured points are the specialists of a group,
//!   grey points are shared between groups.

use stamp_core::stats::prevalence::{profile_occupancy, FeatureOccupancy, MeanOver, Occupancy};
use stamp_core::{GroupProfile, MultiGroupProfile, Profile};

use crate::axes::{
    fonts, group_color, legend, line_height, x_axis, y_axis, Frame, LabelRotation, LegendPosition,
    AXES_COLOR, INCH, NOTE_SIZE, TITLE_SIZE,
};
use crate::axis::{LinearScale, Ticks};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::group_heatmap::format_significant;
use crate::multi_box::ValueAxis;
use crate::text;

/// What is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum View {
    /// One panel per group, on the same axes.
    Panels,
    /// Every group (or the one chosen) in one panel, coloured by group.
    Combined,
    /// Each feature once, at the group where it is most prevalent, coloured
    /// by that group if it is specific to it.
    Specificity,
}

impl View {
    pub const ALL: [View; 3] = [View::Panels, View::Combined, View::Specificity];

    pub fn name(self) -> &'static str {
        match self {
            View::Panels => "One panel per group",
            View::Combined => "All groups in one panel",
            View::Specificity => "Specificity (each feature at its main group)",
        }
    }
}

/// Appearance and view of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    pub view: View,
    /// The samples the mean abundance is taken over.
    pub mean_over: MeanOver,
    /// In the combined view, show only this group.
    pub group: Option<String>,
    /// In the specificity view, the smallest specificity (percentage
    /// points) at which a feature is coloured by its group.
    pub specificity: f64,
    /// Diameter of a point, in points.
    pub point_size: f64,
    /// Opacity of the points that are not highlighted, from 0 to 1.
    pub opacity: f64,
    /// Name highlighted features beside their points.
    pub label_highlighted: bool,
    /// Legend of the groups, in the combined and specificity views.
    pub legend: LegendPosition,
    /// Size of the panel titles and of the names of highlighted features.
    pub title_font_size: f64,
    pub label_font_size: f64,
    pub truncate_labels: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 8.0,
            height: 7.0,
            view: View::Panels,
            mean_over: MeanOver::AllSamples,
            group: None,
            specificity: 50.0,
            point_size: 3.5,
            opacity: 0.45,
            label_highlighted: true,
            legend: LegendPosition::Best,
            title_font_size: TITLE_SIZE,
            label_font_size: NOTE_SIZE,
            truncate_labels: Some(40),
        }
    }
}

/// The features of a profile in each group.
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub features: Vec<FeatureOccupancy>,
    pub groups: Vec<String>,
    pub group_colors: Vec<Color>,
    /// Features drawn in red and named.
    pub highlighted: Vec<String>,
    /// Smallest relative abundance (%) at which a feature counts as
    /// present.
    pub min_abundance: f64,
}

impl Data {
    /// The features of a profile whose sample `i` is in group
    /// `sample_groups[i]` (an index into `groups`); `features` restricts
    /// them.
    pub fn from_profile(
        profile: &Profile,
        sample_groups: &[usize],
        groups: &[String],
        features: Option<&[String]>,
        min_abundance: f64,
    ) -> Data {
        Data {
            features: profile_occupancy(
                profile,
                sample_groups,
                groups.len(),
                min_abundance,
                features,
            ),
            groups: groups.to_vec(),
            group_colors: (0..groups.len()).map(group_color).collect(),
            highlighted: Vec::new(),
            min_abundance,
        }
    }

    pub fn from_group_profile(
        profile: &GroupProfile,
        features: Option<&[String]>,
        min_abundance: f64,
    ) -> Data {
        let first = profile.samples_in_group1().len();
        let sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
            .map(|i| usize::from(i >= first))
            .collect();
        Data::from_profile(
            &profile.profile,
            &sample_groups,
            &[profile.group_name1.clone(), profile.group_name2.clone()],
            features,
            min_abundance,
        )
    }

    pub fn from_multi_group_profile(
        profile: &MultiGroupProfile,
        features: Option<&[String]>,
        min_abundance: f64,
    ) -> Data {
        let sample_groups: Vec<usize> = profile
            .samples_in_groups()
            .iter()
            .enumerate()
            .flat_map(|(g, members)| std::iter::repeat_n(g, members.len()))
            .collect();
        Data::from_profile(
            &profile.profile,
            &sample_groups,
            &profile.group_names,
            features,
            min_abundance,
        )
    }

    fn y_label(&self) -> String {
        if self.min_abundance > 0.0 {
            format!(
                "Prevalence (% of samples with \u{2265} {}%)",
                short_number(self.min_abundance)
            )
        } else {
            "Prevalence (% of samples)".to_string()
        }
    }
}

/// A number as short text: 50, 0.1, 0.005.
fn short_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        let text = format!("{value}");
        if text.len() > 8 {
            format_significant(value, 3)
        } else {
            text
        }
    }
}

fn x_label(over: MeanOver) -> &'static str {
    match over {
        MeanOver::AllSamples => "Mean relative abundance (%)",
        MeanOver::PresentSamples => "Mean relative abundance where present (%)",
    }
}

fn shorten(name: &str, limit: Option<usize>) -> String {
    match limit {
        Some(length) => text::truncate(name, length),
        None => name.to_string(),
    }
}

/// A point to draw.
struct Point {
    feature: usize,
    group: usize,
    x: f64,
    y: f64,
    color: Color,
    highlighted: bool,
    tooltip: String,
}

fn percent(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else if value.abs() >= 1.0 {
        format!("{value:.2}")
    } else {
        format_significant(value, 3)
    }
}

fn describe(feature: &FeatureOccupancy, group: &str, o: &Occupancy) -> String {
    format!(
        "{}\n{group}: present in {} of {} samples ({:.1}%)\nMean abundance: {}% (where present: {}%)",
        feature.feature,
        o.present,
        o.samples,
        o.prevalence,
        percent(o.mean_all),
        percent(o.mean_present)
    )
}

/// The panels to draw: their titles and points.
fn panels_of(data: &Data, options: &Options) -> Vec<(Option<String>, Vec<Point>)> {
    let over = options.mean_over;
    let highlighted = |name: &str| data.highlighted.iter().any(|h| h == name);
    let point = |f: usize, g: usize, color: Color, tooltip: String| -> Option<Point> {
        let o = &data.features[f].groups[g];
        let mean = o.mean(over);
        (o.present > 0 && mean > 0.0).then(|| Point {
            feature: f,
            group: g,
            x: mean,
            y: o.prevalence,
            color,
            highlighted: highlighted(&data.features[f].feature),
            tooltip,
        })
    };
    let groups_with_samples: Vec<usize> = (0..data.groups.len())
        .filter(|&g| {
            data.features
                .first()
                .is_some_and(|f| f.groups[g].samples > 0)
        })
        .collect();
    let color = |g: usize| data.group_colors.get(g).copied().unwrap_or(Color::BLACK);
    match options.view {
        View::Panels => groups_with_samples
            .iter()
            .map(|&g| {
                let samples = data.features[0].groups[g].samples;
                let title = format!("{} (n={samples})", data.groups[g]);
                let points = (0..data.features.len())
                    .filter_map(|f| {
                        let text = describe(
                            &data.features[f],
                            &data.groups[g],
                            &data.features[f].groups[g],
                        );
                        point(f, g, color(g), text)
                    })
                    .collect();
                (Some(title), points)
            })
            .collect(),
        View::Combined => {
            let chosen: Vec<usize> = match &options.group {
                Some(name) => groups_with_samples
                    .iter()
                    .copied()
                    .filter(|&g| &data.groups[g] == name)
                    .collect(),
                None => groups_with_samples.clone(),
            };
            let chosen = if chosen.is_empty() {
                groups_with_samples
            } else {
                chosen
            };
            let mut points = Vec::new();
            for &g in &chosen {
                for f in 0..data.features.len() {
                    let text = describe(
                        &data.features[f],
                        &data.groups[g],
                        &data.features[f].groups[g],
                    );
                    points.extend(point(f, g, color(g), text));
                }
            }
            let title = (chosen.len() == 1).then(|| {
                let g = chosen[0];
                format!(
                    "{} (n={})",
                    data.groups[g], data.features[0].groups[g].samples
                )
            });
            vec![(title, points)]
        }
        View::Specificity => {
            let mut points = Vec::new();
            for (f, feature) in data.features.iter().enumerate() {
                let Some(home) = feature.home_group() else {
                    continue;
                };
                let specificity = feature.specificity();
                let shade = if specificity >= options.specificity {
                    color(home)
                } else {
                    SHARED
                };
                let mut text = format!(
                    "{}\nMost prevalent in {}\nSpecificity: {specificity:.1} percentage points",
                    feature.feature, data.groups[home]
                );
                let mut others: Vec<(usize, &Occupancy)> = feature
                    .groups
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| o.samples > 0)
                    .collect();
                others.sort_by(|a, b| b.1.prevalence.total_cmp(&a.1.prevalence));
                for (g, o) in others.iter().take(12) {
                    text.push_str(&format!(
                        "\n{}: {:.1}% of {}, mean {}%",
                        data.groups[*g],
                        o.prevalence,
                        o.samples,
                        percent(o.mean(over))
                    ));
                }
                points.extend(point(f, home, shade, text));
            }
            // Shared features first, so the specialists are drawn on top.
            points.sort_by_key(|p| p.color != SHARED);
            vec![(None, points)]
        }
    }
}

/// Colour of the features shared between groups in the specificity view.
const SHARED: Color = Color::rgb(179, 179, 179);

const NOTHING: &str = "No feature is present in the samples of the groups";

pub fn plot(data: &Data, options: &Options) -> Figure {
    let panels = panels_of(data, options);
    let all: Vec<f64> = panels
        .iter()
        .flat_map(|(_, points)| points.iter().map(|p| p.x))
        .collect();
    let Some(x_axis_values) = ValueAxis::new(all, true) else {
        return Figure::message(NOTHING);
    };
    let sizes = fonts();
    let padding = 0.2 * INCH;
    let n = panels.len();
    let columns = (n as f64).sqrt().ceil().max(1.0) as usize;
    let rows = n.div_ceil(columns);
    let y_ticks = Ticks::within(0.0, 100.0, 20.0);
    let y_labels: Vec<(f64, String)> = y_ticks
        .positions
        .iter()
        .copied()
        .zip(y_ticks.labels())
        .collect();
    let y_label_width = y_labels
        .iter()
        .map(|(_, l)| text::width(l, sizes.tick))
        .fold(0.0, f64::max);
    let any_title = panels.iter().any(|(t, _)| t.is_some());
    let title_room = if any_title {
        options.title_font_size * 1.3 + 6.0
    } else {
        0.0
    };
    let axis_name = 4.0 + line_height(sizes.axis_label);
    let left_first = 4.0 + y_label_width + axis_name;
    let left_other = 4.0 + y_label_width + 6.0;
    let bottom_room = 4.0 + line_height(sizes.tick) + 3.0 + line_height(sizes.axis_label) + 4.0;
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let gap = 0.1 * INCH;
    let panel_width = (figure.width
        - 2.0 * padding
        - left_first
        - left_other * (columns - 1) as f64
        - gap * (columns - 1) as f64)
        / columns as f64;
    let panel_height = (figure.height - padding) / rows as f64 - title_room - bottom_room;
    if panel_width < 0.7 * INCH || panel_height < 0.7 * INCH {
        return Figure::message("The figure is too small for the prevalence-abundance plot");
    }
    let mut first_frame = None;
    for (i, (title, points)) in panels.iter().enumerate() {
        let (r, c) = (i / columns, i % columns);
        let left = padding + left_first + c as f64 * (left_other + panel_width + gap);
        let top = padding.max(title_room) + r as f64 * (title_room + panel_height + bottom_room);
        let frame = Frame {
            left,
            top,
            right: left + panel_width,
            bottom: top + panel_height,
        };
        first_frame.get_or_insert(frame);
        let x = frame.x_scale(x_axis_values.min, x_axis_values.max);
        let labelled = x_axis_values.fitted(frame.width());
        let y = LinearScale::new(-3.0, 103.0, frame.bottom, frame.top);
        // Light grid lines at the powers of ten and the prevalence ticks.
        let grid = Stroke::solid(Color::grey(0.92), 0.6);
        let (low, high) = (
            x_axis_values.min.round() as i32,
            x_axis_values.max.round() as i32,
        );
        for position in (low..=high).map(f64::from) {
            let px = x.map(position);
            figure.line(&[(px, frame.top), (px, frame.bottom)], grid);
        }
        for (position, _) in &y_labels {
            let py = y.map(*position);
            figure.line(&[(frame.left, py), (frame.right, py)], grid);
        }
        let radius = options.point_size.max(0.5) / 2.0;
        let alpha = (options.opacity.clamp(0.02, 1.0) * 255.0).round() as u8;
        let mut placed: Vec<(f64, f64, f64, f64)> = Vec::new();
        let mut labels = Vec::new();
        for point in points.iter().filter(|p| !p.highlighted) {
            let (px, py) = (x.map(point.x.log10()), y.map(point.y));
            // In the specificity view the specialists stand out, nearly
            // opaque, over the shared features.
            let specialist = options.view == View::Specificity && point.color != SHARED;
            let a = if specialist { alpha.max(217) } else { alpha };
            figure.circle(px, py, radius, Some(Color { a, ..point.color }), None);
            let item = &data.features[point.feature].feature;
            figure.region(
                px - radius,
                py - radius,
                2.0 * radius,
                2.0 * radius,
                item,
                point.tooltip.clone(),
            );
        }
        for point in points.iter().filter(|p| p.highlighted) {
            let (px, py) = (x.map(point.x.log10()), y.map(point.y));
            let r = radius * 1.5;
            figure.circle(
                px,
                py,
                r,
                Some(Color::RED),
                Some(Stroke::solid(Color::BLACK, 0.75)),
            );
            let item = &data.features[point.feature].feature;
            figure.region(
                px - r,
                py - r,
                2.0 * r,
                2.0 * r,
                item,
                point.tooltip.clone(),
            );
            if options.label_highlighted {
                let mut name = shorten(item, options.truncate_labels);
                if options.view == View::Combined && title.is_none() {
                    name = format!("{name} ({})", data.groups[point.group]);
                }
                labels.push((px, py, r, name));
            }
        }
        // Names of highlighted features beside their points, moved down a
        // line at a time to avoid each other.
        let size = options.label_font_size;
        let line = size * 1.2;
        for (px, py, r, name) in labels {
            let width = text::width(&name, size);
            let right_side = px + r + 2.0 + width <= frame.right;
            let x0 = if right_side {
                px + r + 2.0
            } else {
                px - r - 2.0 - width
            };
            let mut y0 = py - line / 2.0;
            for _ in 0..6 {
                let clash = placed.iter().any(|&(a, b, w, h)| {
                    x0 < a + w && a < x0 + width && y0 < b + h && b < y0 + line
                });
                if !clash {
                    break;
                }
                y0 += line;
            }
            placed.push((x0, y0, width, line));
            figure.text(
                x0,
                y0 + line / 2.0,
                name,
                TextStyle::new(size)
                    .align(HAlign::Left, VAlign::Middle)
                    .color(Color::rgb(170, 0, 0)),
            );
        }
        // Frame on all four sides, then the axes.
        figure.rect(
            frame.left,
            frame.top,
            frame.width(),
            frame.height(),
            None,
            Some(Stroke::solid(AXES_COLOR, 1.0)),
        );
        let bottom_row = i + columns >= n;
        x_axis(
            &mut figure,
            &frame,
            &labelled.scaled(&x),
            bottom_row.then_some(x_label(options.mean_over)),
            LabelRotation::Horizontal,
            AXES_COLOR,
        );
        let y_name = data.y_label();
        y_axis(
            &mut figure,
            &frame,
            &y_labels
                .iter()
                .map(|(p, l)| (y.map(*p), l.clone()))
                .collect::<Vec<_>>(),
            (c == 0).then_some(y_name.as_str()),
            AXES_COLOR,
        );
        if let Some(title) = title {
            figure.text(
                (frame.left + frame.right) / 2.0,
                frame.top - 5.0,
                title.clone(),
                TextStyle::new(options.title_font_size).align(HAlign::Center, VAlign::Baseline),
            );
        }
    }
    // A legend of the groups where colours tell them apart.
    if let Some(frame) = first_frame {
        let entries: Vec<(String, Color)> = match options.view {
            View::Panels => Vec::new(),
            View::Combined if panels[0].0.is_some() => Vec::new(),
            View::Combined | View::Specificity => {
                let mut shown: Vec<usize> = panels[0].1.iter().map(|p| p.group).collect();
                if options.view == View::Specificity {
                    shown = panels[0]
                        .1
                        .iter()
                        .filter(|p| p.color != SHARED)
                        .map(|p| p.group)
                        .collect();
                }
                shown.sort_unstable();
                shown.dedup();
                let mut entries: Vec<(String, Color)> = shown
                    .iter()
                    .map(|&g| (data.groups[g].clone(), data.group_colors[g]))
                    .collect();
                if options.view == View::Specificity {
                    entries.push((
                        format!(
                            "Shared (specificity < {})",
                            short_number(options.specificity)
                        ),
                        SHARED,
                    ));
                }
                entries
            }
        };
        legend(&mut figure, &frame, &entries, options.legend);
    }
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use stamp_core::stats::prevalence::occupancy;

    fn feature(name: &str, values: &[f64], groups: &[usize]) -> FeatureOccupancy {
        let (per_group, overall) = occupancy(values, groups, 3, 0.0);
        FeatureOccupancy {
            feature: name.to_string(),
            groups: per_group,
            overall,
        }
    }

    fn data() -> Data {
        // Horse: 4 samples, Tapir: 2, Ape: 2.
        let g = [0, 0, 0, 0, 1, 1, 2, 2];
        Data {
            features: vec![
                feature("Equivita", &[1.0, 2.0, 0.5, 3.0, 0.0, 0.0, 0.0, 0.0], &g),
                feature("Tapirivita", &[0.0, 0.0, 0.0, 0.01, 0.4, 0.2, 0.0, 0.0], &g),
                feature("Everywhere", &[5.0, 4.0, 3.0, 6.0, 2.0, 5.0, 1.0, 9.0], &g),
                feature("Nowhere", &[0.0; 8], &g),
            ],
            groups: vec!["Horse".into(), "Tapir".into(), "Ape".into()],
            group_colors: (0..3).map(group_color).collect(),
            highlighted: vec!["Tapirivita".into()],
            min_abundance: 0.0,
        }
    }

    fn texts(figure: &Figure) -> Vec<String> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn red_points(figure: &Figure) -> usize {
        figure
            .shapes
            .iter()
            .filter(|s| {
                matches!(
                    s,
                    Shape::Circle {
                        fill: Some(Color::RED),
                        ..
                    }
                )
            })
            .count()
    }

    #[test]
    fn a_panel_per_group() {
        let figure = plot(&data(), &Options::default());
        // Horse: 3 features present; Tapir: 2; Ape: 1.
        assert_eq!(figure.regions.len(), 6);
        // Tapirivita is in Horse (once) and Tapir.
        assert_eq!(red_points(&figure), 2);
        let texts = texts(&figure);
        for wanted in [
            "Horse (n=4)",
            "Tapir (n=2)",
            "Ape (n=2)",
            "Tapirivita",
            "0.01",
            "100",
            "Prevalence (% of samples)",
        ] {
            assert!(
                texts.iter().any(|t| t == wanted),
                "missing {wanted} in {texts:?}"
            );
        }
        let tapir = figure
            .regions
            .iter()
            .find(|r| r.item == "Tapirivita" && r.tooltip.contains("Tapir:"))
            .unwrap();
        assert!(tapir.tooltip.contains("present in 2 of 2 samples (100.0%)"));
    }

    #[test]
    fn combined_and_one_group() {
        let options = Options {
            view: View::Combined,
            ..Options::default()
        };
        let figure = plot(&data(), &options);
        assert_eq!(figure.regions.len(), 6);
        assert!(texts(&figure).iter().any(|t| t == "Tapirivita (Tapir)"));
        let one = plot(
            &data(),
            &Options {
                group: Some("Tapir".into()),
                ..options
            },
        );
        assert_eq!(one.regions.len(), 2);
        assert!(texts(&one).iter().any(|t| t == "Tapir (n=2)"));
    }

    #[test]
    fn specificity_colours_specialists() {
        let options = Options {
            view: View::Specificity,
            ..Options::default()
        };
        let figure = plot(&data(), &options);
        // Each feature present somewhere once.
        assert_eq!(figure.regions.len(), 3);
        let equivita = figure
            .regions
            .iter()
            .find(|r| r.item == "Equivita")
            .unwrap();
        assert!(equivita.tooltip.contains("Most prevalent in Horse"));
        assert!(equivita.tooltip.contains("Specificity: 100.0"));
        // Everywhere: 100% in every group, specificity 0: grey.
        let grey = Color {
            a: (0.45f64 * 255.0).round() as u8,
            ..SHARED
        };
        assert!(figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Circle { fill: Some(c), .. } if *c == grey)));
        let texts = texts(&figure);
        assert!(texts.iter().any(|t| t == "Shared (specificity < 50)"));
        assert!(texts.iter().any(|t| t == "Horse"));
    }

    #[test]
    fn mean_where_present_and_nothing_to_show() {
        let options = Options {
            mean_over: MeanOver::PresentSamples,
            ..Options::default()
        };
        let figure = plot(&data(), &options);
        assert!(texts(&figure)
            .iter()
            .any(|t| t == "Mean relative abundance where present (%)"));
        let mut empty = data();
        empty.features.clear();
        assert_eq!(texts(&plot(&empty, &Options::default())), [NOTHING]);
        let tiny = plot(
            &data(),
            &Options {
                width: 1.0,
                ..Options::default()
            },
        );
        assert!(texts(&tiny)[0].contains("too small"));
    }
}
