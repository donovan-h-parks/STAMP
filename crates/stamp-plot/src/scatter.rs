//! Scatter plot of every feature's abundance in one sample or group against
//! the other, with optional spread bars and marginal histograms.

use stamp_core::stats::wilson::wilson_interval;
use stamp_core::{GroupProfile, SampleProfile};

use crate::axes::{
    group_color, scaled_ticks, tick_style, x_axis, y_axis, y_axis_width, Frame, LabelRotation,
    AXES_COLOR, INCH, NOTE_SIZE,
};
use crate::axis::{LinearScale, Ticks};
use crate::figure::TextStyle;
use crate::figure::{Color, Figure, HAlign, Stroke, VAlign};

/// One feature.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub feature: String,
    pub x: f64,
    pub y: f64,
    /// Range drawn around the point in each direction, if any.
    pub x_spread: Option<(f64, f64)>,
    pub y_spread: Option<(f64, f64)>,
    /// What is shown when the point is pointed at; a short summary is used
    /// if this is empty.
    pub tooltip: String,
}

/// The last lines of a tooltip: the difference and ratio of two values.
fn comparison(x: f64, y: f64, what: &str) -> String {
    let ratio = if y != 0.0 {
        format!("{:.3}", x / y)
    } else {
        "undefined".to_string()
    };
    format!(
        "Difference between {what} (%): {:.3}\nRatio of {what}: {ratio}",
        x - y
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub points: Vec<Point>,
    pub x_label: String,
    pub y_label: String,
    /// Colours of the x and y sample or group.
    pub x_color: Color,
    pub y_color: Color,
}

/// How the spread of a feature within a group is summarised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Spread {
    None,
    StandardDeviation,
    TwoStandardDeviations,
    /// The 25th and 75th percentiles.
    Quartiles,
    /// The 9th and 91st percentiles.
    Percentiles9And91,
    /// The 2nd and 98th percentiles.
    Percentiles2And98,
    MinimumAndMaximum,
}

/// Quantile as computed by SciPy's `mquantiles` with its default parameters.
fn mquantile(sorted: &[f64], probability: f64) -> f64 {
    let n = sorted.len() as f64;
    let position = n * probability + 0.4 + 0.2 * probability;
    let k = position.floor().clamp(1.0, (n - 1.0).max(1.0));
    let gamma = (position - k).clamp(0.0, 1.0);
    let low = sorted[(k as usize - 1).min(sorted.len() - 1)];
    let high = sorted[(k as usize).min(sorted.len() - 1)];
    (1.0 - gamma) * low + gamma * high
}

fn spread_of(values: &[f64], method: Spread) -> Option<(f64, f64)> {
    if values.is_empty() {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sd =
        (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / values.len() as f64).sqrt();
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentiles = |low: f64, high: f64| (mquantile(&sorted, low), mquantile(&sorted, high));
    let (low, high) = match method {
        Spread::None => return None,
        Spread::StandardDeviation => (mean - sd, mean + sd),
        Spread::TwoStandardDeviations => (mean - 2.0 * sd, mean + 2.0 * sd),
        Spread::Quartiles => percentiles(0.25, 0.75),
        Spread::Percentiles9And91 => percentiles(0.09, 0.91),
        Spread::Percentiles2And98 => percentiles(0.02, 0.98),
        Spread::MinimumAndMaximum => (sorted[0], sorted[sorted.len() - 1]),
    };
    Some((low.max(0.0), high.min(100.0)))
}

impl Data {
    /// Proportions (%) of every feature in two samples, with 95% Wilson
    /// confidence intervals.
    pub fn from_sample_profile(profile: &SampleProfile) -> Data {
        let interval = |seq: f64, parent: f64| {
            let (low, high, _) = wilson_interval(seq, parent, 1.96);
            Some(((low * 100.0).max(0.0), (high * 100.0).min(100.0)))
        };
        let points = profile
            .profile
            .features()
            .iter()
            .map(|feature| {
                let c = profile.counts(feature);
                let (x, y) = c.percentages();
                let [s1, s2] = [
                    &profile.profile.sample_names[0],
                    &profile.profile.sample_names[1],
                ];
                // As in STAMP v2: counts, percentages, then how they compare.
                let tooltip = format!(
                    "{}\n\nSequences in {s1}: {}\nSequences in {s2}: {}\n\n\
                     {s1} percentage: {x:.3}\n{s2} percentage: {y:.3}\n\n{}",
                    feature.name,
                    c.seq1,
                    c.seq2,
                    comparison(x, y, "proportions")
                );
                Point {
                    feature: feature.name.clone(),
                    x,
                    y,
                    x_spread: interval(c.seq1, c.parent_seq1),
                    y_spread: interval(c.seq2, c.parent_seq2),
                    tooltip,
                }
            })
            .collect();
        Data {
            points,
            x_label: format!("{} (%)", profile.profile.sample_names[0]),
            y_label: format!("{} (%)", profile.profile.sample_names[1]),
            x_color: group_color(0),
            y_color: group_color(1),
        }
    }

    /// Mean proportions (%) of every feature in two groups, with the spread
    /// of each group's samples.
    pub fn from_group_profile(profile: &GroupProfile, spread: Spread) -> Data {
        let mean = |v: &[f64]| {
            if v.is_empty() {
                0.0
            } else {
                v.iter().sum::<f64>() / v.len() as f64
            }
        };
        let points = profile
            .profile
            .features()
            .iter()
            .map(|feature| {
                let (g1, g2) = profile.percentages(feature);
                let (x, y) = (mean(&g1), mean(&g2));
                let tooltip = format!(
                    "{}\n\n{} mean proportion: {x:.3}\n{} mean proportion: {y:.3}\n\n{}",
                    feature.name,
                    profile.group_name1,
                    profile.group_name2,
                    comparison(x, y, "mean proportions")
                );
                Point {
                    feature: feature.name.clone(),
                    x,
                    y,
                    x_spread: spread_of(&g1, spread),
                    y_spread: spread_of(&g2, spread),
                    tooltip,
                }
            })
            .collect();
        Data {
            points,
            x_label: format!("{} (%)", profile.group_name1),
            y_label: format!("{} (%)", profile.group_name2),
            x_color: group_color(0),
            y_color: group_color(1),
        }
    }
}

impl Data {
    /// Add each feature's p-value and corrected p-value to its tooltip.
    /// `p_values` gives the two, already formatted, for a feature that was
    /// tested.
    pub fn add_p_values(&mut self, p_values: impl Fn(&str) -> Option<(String, String)>) {
        for point in &mut self.points {
            if let Some((p, corrected)) = p_values(&point.feature) {
                point
                    .tooltip
                    .push_str(&format!("\n\np-value: {p}\nCorrected p-value: {corrected}"));
            }
        }
    }
}

/// Appearance of the scatter plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Draw the spread (confidence interval) of each point.
    pub show_spread: bool,
    /// Draw histograms of the two axes along the top and right.
    pub show_histograms: bool,
    pub bins: usize,
    /// Size of the histograms.
    pub histogram_size: f64,
    /// Area of the markers, in square points.
    pub marker_size: f64,
    /// Show the squared correlation coefficient.
    pub show_r2: bool,
    /// Size of the R-squared note, in points.
    pub note_font_size: f64,
    /// Features outlined in red.
    pub highlighted: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 7.0,
            show_spread: true,
            show_histograms: true,
            bins: 30,
            histogram_size: 0.5,
            marker_size: 20.0,
            show_r2: true,
            note_font_size: NOTE_SIZE,
            highlighted: Vec::new(),
        }
    }
}

/// Squared Pearson correlation of the points; `None` if undefined.
pub fn r_squared(points: &[Point]) -> Option<f64> {
    let n = points.len() as f64;
    if points.len() < 2 {
        return None;
    }
    let (mx, my) = (
        points.iter().map(|p| p.x).sum::<f64>() / n,
        points.iter().map(|p| p.y).sum::<f64>() / n,
    );
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for p in points {
        sxy += (p.x - mx) * (p.y - my);
        sxx += (p.x - mx) * (p.x - mx);
        syy += (p.y - my) * (p.y - my);
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy * sxy / (sxx * syy))
}

/// Counts of values in `bins` equal bins spanning 0 to `max`.
fn bin_counts(values: impl Iterator<Item = f64>, max: f64, bins: usize) -> Vec<usize> {
    let mut counts = vec![0usize; bins];
    for value in values {
        if value.is_finite() && value >= 0.0 && max > 0.0 {
            let index = ((value / max * bins as f64) as usize).min(bins - 1);
            counts[index] += 1;
        }
    }
    counts
}

pub fn plot(data: &Data, options: &Options) -> Figure {
    let largest = data
        .points
        .iter()
        .flat_map(|p| [p.x, p.y])
        .filter(|v| v.is_finite())
        .fold(0.0, f64::max);
    if data.points.is_empty() || largest <= 0.0 {
        return Figure::message("No active features or degenerate plot");
    }
    let limit = largest * 1.05;
    let ticks = Ticks::inside(0.0, limit);

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.25 * INCH;
    let histogram = if options.show_histograms {
        options.histogram_size * INCH
    } else {
        0.0
    };
    let frame = Frame {
        left: padding + y_axis_width(&ticks, true),
        top: padding + histogram,
        right: figure.width - padding - histogram,
        bottom: figure.height - padding - 0.35 * INCH,
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for a scatter plot");
    }
    let (x, y) = (frame.x_scale(0.0, limit), frame.y_scale(0.0, limit));
    let grey = Stroke::solid(AXES_COLOR, 1.0);

    // Line of equal abundance.
    figure.line(
        &[(x.map(0.0), y.map(0.0)), (x.map(limit), y.map(limit))],
        Stroke::dashed(AXES_COLOR, 1.0, 5.0, 4.0),
    );

    if options.show_spread {
        for p in &data.points {
            if let Some((low, high)) = p.x_spread {
                figure.line(
                    &[
                        (x.map(low.min(limit)), y.map(p.y)),
                        (x.map(high.min(limit)), y.map(p.y)),
                    ],
                    grey,
                );
            }
            if let Some((low, high)) = p.y_spread {
                figure.line(
                    &[
                        (x.map(p.x), y.map(low.min(limit))),
                        (x.map(p.x), y.map(high.min(limit))),
                    ],
                    grey,
                );
            }
        }
    }

    let radius = options.marker_size.max(0.0).sqrt() / 2.0;
    for p in &data.points {
        if !(p.x.is_finite() && p.y.is_finite()) {
            continue;
        }
        let color = if p.y > p.x {
            data.y_color
        } else {
            data.x_color
        };
        let (cx, cy) = (x.map(p.x), y.map(p.y));
        let edge = if options.highlighted.contains(&p.feature) {
            Stroke::solid(Color::RED, 2.0)
        } else {
            Stroke::solid(Color::BLACK, 0.75)
        };
        figure.circle(cx, cy, radius, Some(color), Some(edge));
        figure.region(
            cx - radius - 1.0,
            cy - radius - 1.0,
            2.0 * radius + 2.0,
            2.0 * radius + 2.0,
            &p.feature,
            if p.tooltip.is_empty() {
                format!(
                    "{}\n{}: {:.3}\n{}: {:.3}\nDifference: {:.3}",
                    p.feature,
                    data.x_label,
                    p.x,
                    data.y_label,
                    p.y,
                    p.x - p.y
                )
            } else {
                p.tooltip.clone()
            },
        );
    }

    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&ticks, &x),
        Some(&data.x_label),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&ticks, &y),
        Some(&data.y_label),
        AXES_COLOR,
    );

    if options.show_r2 {
        if let Some(r2) = r_squared(&data.points) {
            figure.text(
                frame.left + 8.0,
                frame.top + 8.0,
                format!("R\u{00b2} = {r2:.3}"),
                TextStyle::new(options.note_font_size).align(HAlign::Left, VAlign::Top),
            );
        }
    }

    if options.show_histograms && options.bins > 0 {
        let outline = Stroke::solid(Color::BLACK, 0.5);
        // The histograms' counts are labelled like ticks.
        let style = tick_style();
        // Top: distribution of the x values.
        let counts = bin_counts(data.points.iter().map(|p| p.x), limit, options.bins);
        let most = counts.iter().copied().max().unwrap_or(0).max(1) as f64;
        let top = Frame {
            left: frame.left,
            top: padding,
            right: frame.right,
            bottom: frame.top - 6.0,
        };
        let height = LinearScale::new(0.0, most * 1.05, top.bottom, top.top);
        let bin_width = top.width() / options.bins as f64;
        for (i, &count) in counts.iter().enumerate() {
            if count > 0 {
                let bar_top = height.map(count as f64);
                figure.rect(
                    top.left + bin_width * i as f64,
                    bar_top,
                    bin_width,
                    top.bottom - bar_top,
                    Some(data.x_color),
                    Some(outline),
                );
            }
        }
        figure.line(&[(top.left, top.bottom), (top.right, top.bottom)], grey);
        figure.text(
            top.left - 4.0,
            top.bottom,
            "0",
            style.align(HAlign::Right, VAlign::Baseline),
        );
        figure.text(
            top.left - 4.0,
            height.map(most),
            format!("{most}"),
            style.align(HAlign::Right, VAlign::Middle),
        );

        // Right: distribution of the y values.
        let counts = bin_counts(data.points.iter().map(|p| p.y), limit, options.bins);
        let most = counts.iter().copied().max().unwrap_or(0).max(1) as f64;
        let right = Frame {
            left: frame.right + 6.0,
            top: frame.top,
            right: figure.width - padding,
            bottom: frame.bottom,
        };
        let length = LinearScale::new(0.0, most * 1.05, right.left, right.right);
        let bin_height = right.height() / options.bins as f64;
        for (i, &count) in counts.iter().enumerate() {
            if count > 0 {
                let bar_bottom = right.bottom - bin_height * i as f64;
                figure.rect(
                    right.left,
                    bar_bottom - bin_height,
                    length.map(count as f64) - right.left,
                    bin_height,
                    Some(data.y_color),
                    Some(outline),
                );
            }
        }
        figure.line(&[(right.left, right.top), (right.left, right.bottom)], grey);
        figure.text(
            right.left,
            right.bottom + 4.0,
            "0",
            style.align(HAlign::Center, VAlign::Top),
        );
        figure.text(
            length.map(most),
            right.bottom + 4.0,
            format!("{most}"),
            style.align(HAlign::Center, VAlign::Top),
        );
    }
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn point(name: &str, x: f64, y: f64) -> Point {
        Point {
            feature: name.to_string(),
            x,
            y,
            x_spread: Some((x * 0.9, x * 1.1)),
            y_spread: Some((y * 0.9, y * 1.1)),
            tooltip: String::new(),
        }
    }

    fn data() -> Data {
        Data {
            points: vec![
                point("a", 1.0, 2.0),
                point("b", 3.0, 2.5),
                point("c", 0.5, 0.4),
                point("d", 4.0, 4.4),
            ],
            x_label: "Australia (%)".to_string(),
            y_label: "USA (%)".to_string(),
            x_color: group_color(0),
            y_color: group_color(1),
        }
    }

    fn circles(figure: &Figure) -> Vec<Color> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Circle { fill, .. } => *fill,
                _ => None,
            })
            .collect()
    }

    #[test]
    fn points_take_the_colour_of_the_more_abundant_side() {
        let figure = plot(&data(), &Options::default());
        assert_eq!(
            circles(&figure),
            [
                group_color(1),
                group_color(0),
                group_color(0),
                group_color(1)
            ]
        );
        assert_eq!(figure.regions.len(), 4);
        assert!(figure
            .region_at(figure.regions[1].x + 2.0, figure.regions[1].y + 2.0)
            .unwrap()
            .tooltip
            .starts_with("b\n"));
        let has_text = |needle: &str| {
            figure
                .shapes
                .iter()
                .any(|s| matches!(s, Shape::Text { text, .. } if text.contains(needle)))
        };
        assert!(has_text("R\u{00b2} = ") && has_text("Australia (%)") && has_text("USA (%)"));
    }

    #[test]
    fn optional_parts_change_what_is_drawn() {
        let full = plot(&data(), &Options::default()).shapes.len();
        let bare = Options {
            show_spread: false,
            show_histograms: false,
            show_r2: false,
            ..Options::default()
        };
        let bare_count = plot(&data(), &bare).shapes.len();
        assert!(bare_count < full);
        assert_eq!(
            plot(
                &data(),
                &Options {
                    show_spread: true,
                    ..bare.clone()
                }
            )
            .shapes
            .len(),
            bare_count + 8
        );
        let highlighted = plot(
            &data(),
            &Options {
                highlighted: vec!["a".to_string()],
                ..bare
            },
        );
        assert!(highlighted.shapes.iter().any(
            |s| matches!(s, Shape::Circle { stroke: Some(st), .. } if st.color == Color::RED)
        ));
    }

    #[test]
    fn correlation_and_spread() {
        let line: Vec<Point> = (1..=5)
            .map(|i| point("p", i as f64, 2.0 * i as f64 + 1.0))
            .collect();
        assert!((r_squared(&line).unwrap() - 1.0).abs() < 1e-12);
        assert!(r_squared(&line[..1]).is_none());
        assert!(r_squared(&[point("a", 1.0, 1.0), point("b", 1.0, 2.0)]).is_none());

        let values = [10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(
            spread_of(&values, Spread::MinimumAndMaximum),
            Some((10.0, 50.0))
        );
        assert_eq!(spread_of(&values, Spread::None), None);
        let (low, high) = spread_of(&values, Spread::StandardDeviation).unwrap();
        assert!(
            (low - (30.0 - 200f64.sqrt())).abs() < 1e-12
                && (high - (30.0 + 200f64.sqrt())).abs() < 1e-12
        );
        // Reference values from scipy.stats.mstats.mquantiles.
        let close = |actual: Option<(f64, f64)>, expected: (f64, f64)| {
            let (low, high) = actual.unwrap();
            assert!(
                (low - expected.0).abs() < 1e-9 && (high - expected.1).abs() < 1e-9,
                "{low} {high}"
            );
        };
        close(spread_of(&values, Spread::Quartiles), (17.0, 43.0));
        close(spread_of(&values, Spread::Percentiles2And98), (10.0, 50.0));
        close(
            spread_of(&[3.0, 1.0], Spread::Percentiles9And91),
            (1.0, 3.0),
        );
        assert_eq!(
            spread_of(&[150.0, 160.0], Spread::MinimumAndMaximum),
            Some((150.0, 100.0))
        );
    }

    #[test]
    fn degenerate_data() {
        let mut empty = data();
        empty.points.clear();
        assert_eq!(plot(&empty, &Options::default()).shapes.len(), 1);
        let zeros = Data {
            points: vec![point("a", 0.0, 0.0)],
            ..data()
        };
        assert_eq!(plot(&zeros, &Options::default()).shapes.len(), 1);
        assert_eq!(
            plot(
                &data(),
                &Options {
                    width: 1.0,
                    ..Options::default()
                }
            )
            .shapes
            .len(),
            1
        );
    }
}
