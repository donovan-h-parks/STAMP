//! Plots of a single feature across the samples of two or more groups: the
//! bar plot and the box plot.

use stamp_core::{GroupProfile, MultiGroupProfile};

use crate::axes::{
    self, corner_note, fonts, group_color, legend, x_axis, y_axis, Frame, LabelRotation,
    LegendPosition, AXES_COLOR, INCH, TITLE_SIZE,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, Stroke};
use crate::p_values::PValueKind;
use crate::text;

/// What is plotted for each sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Field {
    Proportion,
    SequenceCount,
}

impl Field {
    pub fn label(self) -> &'static str {
        match self {
            Field::Proportion => "Proportion of sequences (%)",
            Field::SequenceCount => "Number of sequences",
        }
    }
}

/// One group of samples with a value per sample.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub color: Color,
    pub samples: Vec<String>,
    pub values: Vec<f64>,
}

/// A feature's values in the samples of each group.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureData {
    pub feature: String,
    pub field: Field,
    pub groups: Vec<Group>,
    /// Formatted p-value to show in the corner, if any.
    pub p_value: Option<String>,
    /// What the p-value is, which names it; `None` for the p-value of a
    /// single test, written as "p".
    pub p_value_kind: Option<PValueKind>,
    /// Label of the value axis, where it is not one of the fields of a
    /// profile.
    pub value_label: Option<String>,
    /// The two groups hold paired samples: sample `i` of the first group
    /// belongs with sample `i` of the second (as in the profile of a paired
    /// test). Set by the caller.
    pub paired: bool,
}

impl FeatureData {
    pub(crate) fn axis_label(&self) -> &str {
        self.value_label.as_deref().unwrap_or(self.field.label())
    }
}

impl FeatureData {
    /// Values of a feature in a two-group profile.
    pub fn from_group_profile(
        profile: &GroupProfile,
        feature: &str,
        field: Field,
    ) -> Option<FeatureData> {
        let f = profile.profile.feature(feature)?;
        let (values1, values2) = match field {
            Field::Proportion => profile.percentages(f),
            Field::SequenceCount => {
                let (a, b) = profile.counts(f);
                (a.to_vec(), b.to_vec())
            }
        };
        let group = |name: &str, index: usize, samples: &[String], values: Vec<f64>| Group {
            name: name.to_string(),
            color: group_color(index),
            samples: samples.to_vec(),
            values,
        };
        Some(FeatureData {
            feature: feature.to_string(),
            field,
            groups: vec![
                group(
                    &profile.group_name1,
                    0,
                    profile.samples_in_group1(),
                    values1,
                ),
                group(
                    &profile.group_name2,
                    1,
                    profile.samples_in_group2(),
                    values2,
                ),
            ],
            p_value: None,
            p_value_kind: None,
            value_label: None,
            paired: false,
        })
    }

    /// Values of a feature in a multiple-group profile.
    pub fn from_multi_group_profile(
        profile: &MultiGroupProfile,
        feature: &str,
        field: Field,
    ) -> Option<FeatureData> {
        let f = profile.profile.feature(feature)?;
        let values: Vec<Vec<f64>> = match field {
            Field::Proportion => profile.percentages(f),
            Field::SequenceCount => profile.counts(f).into_iter().map(<[f64]>::to_vec).collect(),
        };
        let groups = profile
            .group_names
            .iter()
            .zip(profile.samples_in_groups())
            .zip(values)
            .enumerate()
            .map(|(index, ((name, samples), values))| Group {
                name: name.clone(),
                color: group_color(index),
                samples: samples.to_vec(),
                values,
            })
            .collect();
        Some(FeatureData {
            feature: feature.to_string(),
            field,
            groups,
            p_value: None,
            p_value_kind: None,
            value_label: None,
            paired: false,
        })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.groups.is_empty() || self.groups.iter().any(|g| g.values.is_empty())
    }

    fn max_value(&self) -> f64 {
        self.groups
            .iter()
            .flat_map(|g| g.values.iter().copied())
            .fold(0.0, f64::max)
    }
}

/// Appearance of the bar plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BarOptions {
    /// Width given to each sample.
    pub column_width: f64,
    pub height: f64,
    /// Draw a line at the mean of each group.
    pub show_averages: bool,
    pub show_p_value: bool,
    pub legend: LegendPosition,
    pub truncate_labels: Option<usize>,
    /// Size of the title (the feature's name) and the p-value, in points.
    pub title_font_size: f64,
    /// Logarithmic (base 10) value axis; samples without the feature get no
    /// bar and the axis is labelled 0 below a break.
    pub log_scale: bool,
}

impl Default for BarOptions {
    fn default() -> Self {
        BarOptions {
            column_width: 0.2,
            height: 6.0,
            show_averages: true,
            show_p_value: true,
            legend: LegendPosition::None,
            truncate_labels: Some(50),
            title_font_size: TITLE_SIZE,
            log_scale: false,
        }
    }
}

pub(crate) fn shorten(text_: &str, limit: Option<usize>) -> String {
    match limit {
        Some(length) => text::truncate(text_, length),
        None => text_.to_string(),
    }
}

pub(crate) const NOTHING: &str = "No active features or degenerate plot";

/// Bar plot: one bar per sample, grouped and coloured by group.
pub fn bar_plot(data: &FeatureData, options: &BarOptions) -> Figure {
    if data.is_empty() {
        return Figure::message(NOTHING);
    }
    let sample_labels: Vec<String> = data
        .groups
        .iter()
        .flat_map(|g| {
            g.samples
                .iter()
                .map(|s| shorten(s, options.truncate_labels))
        })
        .collect();
    let num_samples = sample_labels.len() as f64;

    let axis = if options.log_scale {
        crate::multi_box::ValueAxis::new(
            data.groups.iter().flat_map(|g| g.values.iter().copied()),
            true,
        )
    } else {
        None
    };
    let axis = axis.unwrap_or_else(|| {
        let ticks = Ticks::covering(0.0, data.max_value().max(f64::MIN_POSITIVE));
        crate::multi_box::ValueAxis {
            min: ticks.min,
            max: ticks.max,
            log: false,
            zero: None,
            break_at: None,
            ticks: ticks
                .positions
                .iter()
                .copied()
                .zip(ticks.labels())
                .collect(),
        }
    });
    let padding = 0.25 * INCH;
    let longest_label = sample_labels
        .iter()
        .map(|l| text::width(l, fonts().tick))
        .fold(0.0, f64::max);
    let column = options.column_width * INCH;
    let left =
        padding + axes::TICK_PAD + axis.label_width() + 4.0 + axes::line_height(fonts().axis_label);
    let frame = Frame {
        left,
        top: padding + options.title_font_size,
        right: left + column * num_samples,
        bottom: options.height * INCH - padding - longest_label - axes::TICK_PAD,
    };
    if frame.height() < 0.5 * INCH {
        return Figure::message("The plot is too short for its labels; increase the figure height");
    }
    let mut figure = Figure::new(frame.right + padding, options.height * INCH);

    let y = frame.y_scale(axis.min, axis.max);
    let bar_width = column * 0.9 / 2.0;
    let outline = Stroke::solid(Color::BLACK, 1.0);
    let mut x_ticks = Vec::new();
    let mut column_index = 0usize;
    for group in &data.groups {
        let group_left = frame.left + column * column_index as f64;
        if options.show_averages {
            let mean = group.values.iter().sum::<f64>() / group.values.len() as f64;
            let right = group_left + column * group.values.len() as f64;
            let at = y.map(axis.coord(mean));
            figure.line(
                &[(group_left, at), (right, at)],
                Stroke::solid(group.color, 1.0),
            );
        }
        for (sample, &value) in group.samples.iter().zip(&group.values) {
            let x = frame.left + column * (column_index as f64 + 0.25);
            // On a logarithmic axis a sample without the feature has no bar.
            if !(axis.log && value <= 0.0) {
                let top = y.map(axis.coord(value.max(0.0)));
                figure.rect(
                    x,
                    top,
                    bar_width,
                    frame.bottom - top,
                    Some(group.color),
                    Some(outline),
                );
            }
            let centre = x + bar_width / 2.0;
            x_ticks.push((centre, sample_labels[column_index].clone()));
            figure.region(
                x,
                frame.top,
                bar_width,
                frame.height(),
                sample,
                format!(
                    "{sample}\n{}\n{}: {value:.3}",
                    group.name,
                    data.axis_label()
                ),
            );
            column_index += 1;
        }
    }

    x_axis(
        &mut figure,
        &frame,
        &x_ticks,
        None,
        LabelRotation::Vertical,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &axis.scaled(&y),
        Some(&crate::axes::scale_label(
            data.axis_label(),
            options.log_scale,
        )),
        AXES_COLOR,
    );
    axis.draw_break(&mut figure, &frame, &y, false);
    axes::title(
        &mut figure,
        &frame,
        &shorten(&data.feature, options.truncate_labels),
        options.title_font_size,
    );
    if let (true, Some(p)) = (options.show_p_value, &data.p_value) {
        corner_note(
            &mut figure,
            &frame,
            &crate::multi_box::p_text(p, data.p_value_kind),
            options.title_font_size,
        );
    }
    let entries: Vec<(String, Color)> = data
        .groups
        .iter()
        .map(|g| (g.name.clone(), g.color))
        .collect();
    legend(&mut figure, &frame, &entries, options.legend);
    figure
}

/// How a box plot of several features is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BoxLayout {
    /// The features along the category axis, with a box per group side by
    /// side at each, coloured by group.
    Grouped,
    /// One panel per feature (small multiples), with the groups along the
    /// category axis of each.
    Panels,
}

/// The order of the groups in a box plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GroupOrder {
    /// As given (the order of the analysis's groups).
    AsGiven,
    Alphabetical,
    /// Highest median of the first feature first (ties by mean, then name).
    Median,
    /// By the category of each group in another metadata field (the most
    /// common value among its samples), then by name.
    Category,
}

/// Appearance of the box plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BoxOptions {
    pub width: f64,
    pub height: f64,
    /// Mark the mean of each group with a star.
    pub show_averages: bool,
    pub show_p_value: bool,
    pub truncate_labels: Option<usize>,
    /// Size of the title (the feature's name) and the p-value, in points.
    pub title_font_size: f64,
    /// Logarithmic (base 10) value axis. The quartiles and whiskers are
    /// those of the values themselves; zeros are drawn on a line labelled 0
    /// below a break in the axis.
    pub log_scale: bool,
    /// Draw every sample as a small, semi-transparent point spread across
    /// its box (which replaces the crosses of samples beyond the whiskers).
    pub show_points: bool,
    /// Diameter of those points, in points.
    pub point_size: f64,
    /// Write the number of samples of each group after its name.
    pub show_counts: bool,
    /// Boxes lying along a horizontal value axis.
    pub horizontal: bool,
    /// Layout of several features.
    pub layout: BoxLayout,
    /// With one panel per feature, the same value axis in every panel.
    pub shared_axis: bool,
    pub group_order: GroupOrder,
    /// For two groups of paired samples ([`FeatureData::paired`]): join the
    /// two samples of each pair with a line between the boxes, coloured by
    /// the group in which the value is higher. Several features are then
    /// drawn one panel each.
    pub show_pairs: bool,
}

impl Default for BoxOptions {
    fn default() -> Self {
        BoxOptions {
            width: 7.0,
            height: 7.0,
            show_averages: true,
            show_p_value: true,
            truncate_labels: Some(50),
            title_font_size: TITLE_SIZE,
            log_scale: false,
            show_points: false,
            point_size: 2.5,
            show_counts: false,
            horizontal: false,
            layout: BoxLayout::Grouped,
            shared_axis: true,
            group_order: GroupOrder::AsGiven,
            show_pairs: false,
        }
    }
}

/// Five-number summary of a group, with the points beyond the whiskers.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxStats {
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    /// Most extreme values within 1.5 interquartile ranges of the box.
    pub whisker_low: f64,
    pub whisker_high: f64,
    pub outliers: Vec<f64>,
}

/// Quantile by linear interpolation between order statistics.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    let position = q * (sorted.len() - 1) as f64;
    let (low, high) = (position.floor() as usize, position.ceil() as usize);
    sorted[low] + (position - low as f64) * (sorted[high] - sorted[low])
}

pub fn box_stats(values: &[f64]) -> Option<BoxStats> {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f64::total_cmp);
    let (q1, median, q3) = (
        quantile(&sorted, 0.25),
        quantile(&sorted, 0.5),
        quantile(&sorted, 0.75),
    );
    let reach = 1.5 * (q3 - q1);
    let inside = |v: &&f64| **v >= q1 - reach && **v <= q3 + reach;
    let whisker_low = sorted.iter().find(inside).copied().unwrap_or(q1).min(q1);
    let whisker_high = sorted
        .iter()
        .rev()
        .find(inside)
        .copied()
        .unwrap_or(q3)
        .max(q3);
    let outliers = sorted
        .iter()
        .copied()
        .filter(|v| *v < whisker_low || *v > whisker_high)
        .collect();
    Some(BoxStats {
        q1,
        median,
        q3,
        whisker_low,
        whisker_high,
        outliers,
    })
}

/// A five-pointed star, for marking means.
pub(crate) fn star(figure: &mut Figure, x: f64, y: f64, radius: f64) {
    let points = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { radius } else { radius * 0.4 };
            let angle = std::f64::consts::PI * (i as f64 / 5.0) - std::f64::consts::FRAC_PI_2;
            (x + r * angle.cos(), y + r * angle.sin())
        })
        .collect();
    figure.shapes.push(crate::figure::Shape::Polygon {
        points,
        fill: Some(Color::WHITE),
        stroke: Some(Stroke::solid(Color::BLACK, 0.5)),
    });
}

/// Box plot: the distribution of a feature in each group. See
/// [`crate::multi_box::plot`] for several features.
pub fn box_plot(data: &FeatureData, options: &BoxOptions) -> Figure {
    crate::multi_box::plot(std::slice::from_ref(data), None, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data() -> FeatureData {
        FeatureData {
            feature: "Bacteroides".to_string(),
            field: Field::Proportion,
            groups: vec![
                Group {
                    name: "Lean".to_string(),
                    color: group_color(0),
                    samples: vec!["L1".into(), "L2".into(), "L3".into()],
                    values: vec![10.0, 12.0, 14.0],
                },
                Group {
                    name: "Obese".to_string(),
                    color: group_color(1),
                    samples: vec!["O1".into(), "O2".into()],
                    values: vec![30.0, 34.0],
                },
            ],
            p_value: Some("0.012".to_string()),
            p_value_kind: None,
            value_label: None,
            paired: false,
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

    fn rect_count(figure: &Figure, color: Color) -> usize {
        figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == color))
            .count()
    }

    #[test]
    fn bar_plot_has_a_bar_per_sample() {
        let figure = bar_plot(&data(), &BarOptions::default());
        assert_eq!(rect_count(&figure, group_color(0)), 3);
        assert_eq!(rect_count(&figure, group_color(1)), 2);
        assert_eq!(figure.regions.len(), 5);
        assert!(figure.regions[3].tooltip.contains("Obese"));
        let texts = texts(&figure);
        for expected in [
            "Bacteroides",
            "p = 0.012",
            "Proportion of sequences (%)",
            "L1",
            "O2",
            "35",
        ] {
            assert!(
                texts.iter().any(|t| t == expected),
                "missing {expected} in {texts:?}"
            );
        }
        // A p-value of the analysis is named by its correction.
        let mut storey = data();
        storey.p_value_kind = Some(PValueKind::QValue);
        assert!(
            super::tests::texts(&bar_plot(&storey, &BarOptions::default()))
                .iter()
                .any(|t| t == "q-value = 0.012")
        );
        // Width grows with the number of samples.
        let mut more = data();
        more.groups[0].samples.push("L4".into());
        more.groups[0].values.push(11.0);
        assert!(bar_plot(&more, &BarOptions::default()).width > figure.width);
    }

    #[test]
    fn bar_plot_options() {
        let plain = BarOptions {
            show_averages: false,
            show_p_value: false,
            ..BarOptions::default()
        };
        let figure = bar_plot(&data(), &plain);
        assert!(!texts(&figure).iter().any(|t| t.starts_with("p = ")));
        let with_legend = bar_plot(
            &data(),
            &BarOptions {
                legend: LegendPosition::UpperLeft,
                ..plain.clone()
            },
        );
        assert_eq!(with_legend.shapes.len(), figure.shapes.len() + 4);
        assert_eq!(
            bar_plot(&data(), &BarOptions::default()).shapes.len(),
            figure.shapes.len() + 3
        );
    }

    #[test]
    fn bar_plot_on_a_logarithmic_axis() {
        let mut data = data();
        data.groups[0].values = vec![0.0, 0.02, 3.0];
        let figure = bar_plot(
            &data,
            &BarOptions {
                log_scale: true,
                ..BarOptions::default()
            },
        );
        // No bar for the sample without the feature.
        assert_eq!(rect_count(&figure, group_color(0)), 2);
        assert_eq!(figure.regions.len(), 5);
        let texts = texts(&figure);
        for expected in ["0", "0.01", "1", "100"] {
            assert!(
                texts.iter().any(|t| t == expected),
                "missing {expected} in {texts:?}"
            );
        }
    }

    #[test]
    fn box_statistics() {
        let s = box_stats(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0]).unwrap();
        assert_eq!((s.q1, s.median, s.q3), (3.25, 5.5, 7.75));
        assert_eq!((s.whisker_low, s.whisker_high), (1.0, 9.0));
        assert_eq!(s.outliers, [100.0]);
        let single = box_stats(&[4.0]).unwrap();
        assert_eq!((single.q1, single.whisker_high), (4.0, 4.0));
        assert!(box_stats(&[]).is_none());
        assert!(box_stats(&[f64::NAN]).is_none());
    }

    #[test]
    fn box_plot_has_a_box_per_group() {
        let figure = box_plot(&data(), &BoxOptions::default());
        assert_eq!(rect_count(&figure, group_color(0)), 1);
        assert_eq!(rect_count(&figure, group_color(1)), 1);
        assert_eq!(figure.regions.len(), 2);
        assert!(figure.regions[0].tooltip.contains("Median: 12.000"));
        let texts = texts(&figure);
        assert!(texts.contains(&"Lean".to_string()) && texts.contains(&"Obese".to_string()));
        assert!((figure.width - 504.0).abs() < 1e-9);
        // Stars mark the means.
        let stars = figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Polygon { points, .. } if points.len() == 10))
            .count();
        assert_eq!(stars, 2);
    }

    #[test]
    fn degenerate_data() {
        let mut empty = data();
        empty.groups[1].values.clear();
        assert_eq!(texts(&bar_plot(&empty, &BarOptions::default())), [NOTHING]);
        assert_eq!(texts(&box_plot(&empty, &BoxOptions::default())), [NOTHING]);
        let tiny = box_plot(
            &data(),
            &BoxOptions {
                width: 1.0,
                ..BoxOptions::default()
            },
        );
        assert!(texts(&tiny)[0].contains("too small"));
    }

    #[test]
    fn titles_follow_their_option_and_axes_the_shared_fonts() {
        let size = |figure: &Figure, wanted: &str| {
            figure
                .shapes
                .iter()
                .find_map(|s| match s {
                    Shape::Text { text, style, .. } if text == wanted => Some(style.size),
                    _ => None,
                })
                .unwrap()
        };
        let options = BoxOptions {
            title_font_size: 13.0,
            ..BoxOptions::default()
        };
        let figure = box_plot(&data(), &options);
        assert_eq!(size(&figure, "Bacteroides"), 13.0);
        assert_eq!(size(&figure, "p = 0.012"), 13.0);
        let shared = crate::axes::Fonts {
            tick: 10.0,
            axis_label: 12.0,
            legend: 9.0,
            ..crate::axes::Fonts::default()
        };
        let figure = crate::axes::with_fonts(shared, || box_plot(&data(), &BoxOptions::default()));
        assert_eq!(size(&figure, "Bacteroides"), crate::axes::TITLE_SIZE);
        assert_eq!(size(&figure, data().axis_label()), 12.0);
        assert_eq!(size(&figure, &data().groups[0].name), 10.0);
    }
}
