//! Plots of a single feature across the samples of two or more groups: the
//! bar plot and the box plot.

use stamp_core::{GroupProfile, MultiGroupProfile};

use crate::axes::{
    self, corner_note, group_color, label_style, legend, scaled_ticks, x_axis, y_axis,
    y_axis_width, Frame, LabelRotation, LegendPosition, AXES_COLOR, FONT_SIZE, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, HAlign, Stroke, VAlign};
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
        })
    }

    fn is_empty(&self) -> bool {
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
        }
    }
}

fn shorten(text_: &str, limit: Option<usize>) -> String {
    match limit {
        Some(length) => text::truncate(text_, length),
        None => text_.to_string(),
    }
}

const NOTHING: &str = "No active features or degenerate plot";

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

    let ticks = Ticks::covering(0.0, data.max_value().max(f64::MIN_POSITIVE));
    let padding = 0.25 * INCH;
    let longest_label = sample_labels
        .iter()
        .map(|l| text::width(l, FONT_SIZE))
        .fold(0.0, f64::max);
    let column = options.column_width * INCH;
    let left = padding + y_axis_width(&ticks, true);
    let frame = Frame {
        left,
        top: padding + FONT_SIZE,
        right: left + column * num_samples,
        bottom: options.height * INCH - padding - longest_label - axes::TICK_PAD,
    };
    if frame.height() < 0.5 * INCH {
        return Figure::message("The plot is too short for its labels; increase the figure height");
    }
    let mut figure = Figure::new(frame.right + padding, options.height * INCH);

    let y = frame.y_scale(ticks.min, ticks.max);
    let bar_width = column * 0.9 / 2.0;
    let outline = Stroke::solid(Color::BLACK, 1.0);
    let mut x_ticks = Vec::new();
    let mut column_index = 0usize;
    for group in &data.groups {
        let group_left = frame.left + column * column_index as f64;
        if options.show_averages {
            let mean = group.values.iter().sum::<f64>() / group.values.len() as f64;
            let right = group_left + column * group.values.len() as f64;
            figure.line(
                &[(group_left, y.map(mean)), (right, y.map(mean))],
                Stroke::solid(group.color, 1.0),
            );
        }
        for (sample, &value) in group.samples.iter().zip(&group.values) {
            let x = frame.left + column * (column_index as f64 + 0.25);
            let top = y.map(value.max(0.0));
            figure.rect(
                x,
                top,
                bar_width,
                frame.bottom - top,
                Some(group.color),
                Some(outline),
            );
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
                    data.field.label()
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
        &scaled_ticks(&ticks, &y),
        Some(data.field.label()),
        AXES_COLOR,
    );
    axes::title(
        &mut figure,
        &frame,
        &shorten(&data.feature, options.truncate_labels),
    );
    if let (true, Some(p)) = (options.show_p_value, &data.p_value) {
        corner_note(&mut figure, &frame, &format!("p = {p}"));
    }
    let entries: Vec<(String, Color)> = data
        .groups
        .iter()
        .map(|g| (g.name.clone(), g.color))
        .collect();
    legend(&mut figure, &frame, &entries, options.legend);
    figure
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
}

impl Default for BoxOptions {
    fn default() -> Self {
        BoxOptions {
            width: 7.0,
            height: 7.0,
            show_averages: true,
            show_p_value: true,
            truncate_labels: Some(50),
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
fn star(figure: &mut Figure, x: f64, y: f64, radius: f64) {
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

/// Box plot: the distribution of a feature in each group.
pub fn box_plot(data: &FeatureData, options: &BoxOptions) -> Figure {
    if data.is_empty() {
        return Figure::message(NOTHING);
    }
    let stats: Vec<Option<BoxStats>> = data.groups.iter().map(|g| box_stats(&g.values)).collect();
    let (low, high) = data
        .groups
        .iter()
        .flat_map(|g| g.values.iter().copied())
        .filter(|v| v.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    if !low.is_finite() {
        return Figure::message(NOTHING);
    }
    let ticks = Ticks::covering(low, high);

    let padding = 0.25 * INCH;
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let frame = Frame {
        left: padding + y_axis_width(&ticks, true),
        top: 2.0 * padding,
        right: figure.width - 2.0 * padding,
        bottom: figure.height - padding - 0.3 * INCH,
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for a box plot");
    }

    let n = data.groups.len() as f64;
    let x = frame.x_scale(0.5, n + 0.5);
    let y = frame.y_scale(ticks.min, ticks.max);
    let black = Stroke::solid(Color::BLACK, 1.0);
    let half_box = (x.map(1.0) - x.map(0.0)) * 0.25;
    let mut x_ticks = Vec::new();

    for (index, (group, stats)) in data.groups.iter().zip(&stats).enumerate() {
        let centre = x.map(index as f64 + 1.0);
        x_ticks.push((centre, shorten(&group.name, options.truncate_labels)));
        let Some(s) = stats else { continue };

        // Whiskers with caps, then the box and its median.
        for (from, to) in [(s.q1, s.whisker_low), (s.q3, s.whisker_high)] {
            figure.line(&[(centre, y.map(from)), (centre, y.map(to))], black);
            figure.line(
                &[
                    (centre - half_box / 2.0, y.map(to)),
                    (centre + half_box / 2.0, y.map(to)),
                ],
                black,
            );
        }
        figure.rect(
            centre - half_box,
            y.map(s.q3),
            2.0 * half_box,
            y.map(s.q1) - y.map(s.q3),
            Some(group.color),
            Some(black),
        );
        figure.line(
            &[
                (centre - half_box, y.map(s.median)),
                (centre + half_box, y.map(s.median)),
            ],
            black,
        );
        for &outlier in &s.outliers {
            let (px, py, arm) = (centre, y.map(outlier), 3.0);
            figure.line(&[(px - arm, py), (px + arm, py)], black);
            figure.line(&[(px, py - arm), (px, py + arm)], black);
        }
        let mean = group.values.iter().sum::<f64>() / group.values.len() as f64;
        if options.show_averages {
            star(&mut figure, centre, y.map(mean), 4.5);
        }
        figure.region(
            centre - half_box,
            frame.top,
            2.0 * half_box,
            frame.height(),
            &group.name,
            format!(
                "{}\nMedian: {:.3}\nMean: {mean:.3}\nQuartiles: {:.3} to {:.3}\nSamples: {}",
                group.name,
                s.median,
                s.q1,
                s.q3,
                group.values.len()
            ),
        );
    }

    x_axis(
        &mut figure,
        &frame,
        &x_ticks,
        None,
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&ticks, &y),
        Some(data.field.label()),
        AXES_COLOR,
    );
    axes::title(
        &mut figure,
        &frame,
        &shorten(&data.feature, options.truncate_labels),
    );
    if let (true, Some(p)) = (options.show_p_value, &data.p_value) {
        figure.text(
            frame.right,
            frame.top - 5.0,
            format!("p = {p}"),
            label_style().align(HAlign::Right, VAlign::Baseline),
        );
    }
    figure
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
}
