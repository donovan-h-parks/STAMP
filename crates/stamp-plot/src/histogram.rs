//! Histograms: of p-values, and of the sequence counts of two samples.

use stamp_core::SampleProfile;

use crate::axes::{
    group_color, legend, scaled_ticks, x_axis, y_axis, Frame, LabelRotation, LegendPosition,
    AXES_COLOR, INCH,
};
use crate::axis::{LinearScale, Ticks};
use crate::figure::{Color, Figure, Stroke};

/// One series of a histogram: counts per bin, with a colour.
struct Series {
    counts: Vec<usize>,
    color: Color,
}

/// Edges of bins of `bin_width` from 0 that cover `max`; the last bin ends
/// at `max` or just past it and includes its upper edge. Edges are
/// multiples of the width rather than sums of it, whose rounding error put
/// a value of exactly `max` (a p-value of 1, say) in an extra bin past the
/// end of the axis, where it was not drawn.
fn bin_edges(max: f64, bin_width: f64) -> Vec<f64> {
    // Guard against a bin width so small that the plot could never be drawn.
    let bins = ((max / bin_width - 1e-9).ceil().max(1.0) as usize).min(100_000);
    let mut edges: Vec<f64> = (0..=bins).map(|i| i as f64 * bin_width).collect();
    if let Some(last) = edges.last_mut() {
        *last = last.max(max);
    }
    edges
}

fn count(values: &[f64], edges: &[f64]) -> Vec<usize> {
    let mut counts = vec![0usize; edges.len() - 1];
    let last = edges.len() - 2;
    for &value in values {
        if !value.is_finite() || value < edges[0] || value > edges[edges.len() - 1] {
            continue;
        }
        // The last bin includes its upper edge.
        let index = edges
            .partition_point(|e| *e <= value)
            .saturating_sub(1)
            .min(last);
        counts[index] += 1;
    }
    counts
}

/// The vertical scale of a histogram and its ticks.
#[allow(clippy::type_complexity)]
pub(crate) fn count_axis(
    frame: &Frame,
    most: usize,
    log: bool,
) -> (Box<dyn Fn(f64) -> f64>, Vec<(f64, String)>) {
    if log {
        // Counts on a log scale, starting just below one.
        let top = (most.max(1) as f64).log10().ceil().max(1.0);
        let scale = LinearScale::new(-0.3, top, frame.bottom, frame.top);
        let ticks = (0..=top as i32)
            .map(|power| {
                (
                    scale.map(f64::from(power)),
                    format!("{}", 10f64.powi(power)),
                )
            })
            .collect();
        (
            Box::new(move |count: f64| scale.map(count.max(0.5).log10())),
            ticks,
        )
    } else {
        let ticks = Ticks::covering(0.0, most.max(1) as f64);
        let scale = frame.y_scale(ticks.min, ticks.max);
        let labels = scaled_ticks(&ticks, &scale);
        (Box::new(move |count: f64| scale.map(count)), labels)
    }
}

/// Draw bars for one or more series sharing bins, side by side within a bin.
fn bars(
    figure: &mut Figure,
    frame: &Frame,
    x: &LinearScale,
    y: &dyn Fn(f64) -> f64,
    edges: &[f64],
    series: &[Series],
) {
    let outline = Stroke::solid(Color::BLACK, 0.5);
    for (s, one) in series.iter().enumerate() {
        for (i, &n) in one.counts.iter().enumerate() {
            if n == 0 {
                continue;
            }
            let (left, right) = (x.map(edges[i]), x.map(edges[i + 1]));
            if right < frame.left - 1e-9 || left > frame.right + 1e-9 {
                continue;
            }
            let (left, right) = (left.max(frame.left), right.min(frame.right));
            let slot = (right - left) / series.len() as f64;
            let top = y(n as f64);
            figure.rect(
                left + slot * s as f64,
                top,
                slot,
                frame.bottom - top,
                Some(one.color),
                Some(outline),
            );
        }
    }
}

/// Appearance of the p-value histogram. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PValueOptions {
    pub width: f64,
    pub height: f64,
    pub bin_width: f64,
    pub log_scale: bool,
    /// Label of the x axis: a [`crate::p_values::PValueKind`] label, such as
    /// `Corrected p-value`.
    pub label: String,
    /// A second, zoomed histogram of the smallest p-values in the top-right.
    pub inset: Option<Inset>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Inset {
    /// Size as a percentage of the main plot.
    pub width_percent: f64,
    pub height_percent: f64,
    pub bin_width: f64,
    /// Largest p-value shown.
    pub x_limit: f64,
    pub log_scale: bool,
}

impl Default for PValueOptions {
    fn default() -> Self {
        PValueOptions {
            width: 7.0,
            height: 7.0,
            bin_width: 0.01,
            log_scale: false,
            label: crate::p_values::PValueKind::Corrected.label().to_string(),
            inset: Some(Inset {
                width_percent: 60.0,
                height_percent: 60.0,
                bin_width: 0.002,
                x_limit: 0.05,
                log_scale: false,
            }),
        }
    }
}

const GREY: Color = Color::rgb(128, 128, 128);
const NOTHING: &str = "No active features or degenerate plot";

#[allow(clippy::too_many_arguments)]
fn one_histogram(
    figure: &mut Figure,
    frame: &Frame,
    values: &[f64],
    limit: f64,
    bin_width: f64,
    log: bool,
    x_label: Option<&str>,
    y_label: Option<&str>,
) {
    let edges = bin_edges(limit, bin_width);
    let counts = count(values, &edges);
    let most = counts.iter().copied().max().unwrap_or(0);
    let x_ticks = Ticks::inside(0.0, limit);
    let x = frame.x_scale(0.0, limit);
    let (y, y_ticks) = count_axis(frame, most, log);
    bars(
        figure,
        frame,
        &x,
        &*y,
        &edges,
        &[Series {
            counts,
            color: GREY,
        }],
    );
    x_axis(
        figure,
        frame,
        &scaled_ticks(&x_ticks, &x),
        x_label,
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(figure, frame, &y_ticks, y_label, AXES_COLOR);
}

/// Histogram of p-values between 0 and 1.
pub fn p_value_histogram(p_values: &[f64], options: &PValueOptions) -> Figure {
    let values: Vec<f64> = p_values.iter().copied().filter(|p| p.is_finite()).collect();
    if values.is_empty() || options.bin_width <= 0.0 {
        return Figure::message(NOTHING);
    }
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let frame = Frame {
        left: 0.5 * INCH,
        top: 0.2 * INCH,
        right: figure.width - 0.2 * INCH,
        bottom: figure.height - crate::axes::x_axis_room(),
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for a histogram");
    }
    one_histogram(
        &mut figure,
        &frame,
        &values,
        1.0,
        options.bin_width,
        options.log_scale,
        Some(&options.label),
        Some("Number of features"),
    );

    if let Some(inset) = &options.inset {
        let small: Vec<f64> = values
            .iter()
            .copied()
            .filter(|p| *p <= inset.x_limit)
            .collect();
        if !small.is_empty() && inset.bin_width > 0.0 && inset.x_limit > 0.0 {
            let (w, h) = (
                frame.width() * inset.width_percent / 100.0,
                frame.height() * inset.height_percent / 100.0,
            );
            let inner = Frame {
                left: frame.right - w,
                top: frame.top + 6.0,
                right: frame.right - 6.0,
                bottom: frame.top + h,
            };
            if inner.width() > 0.5 * INCH && inner.height() > 0.5 * INCH {
                figure.rect(
                    inner.left - 30.0,
                    inner.top - 4.0,
                    inner.width() + 36.0,
                    inner.height() + 22.0,
                    Some(Color::WHITE),
                    None,
                );
                one_histogram(
                    &mut figure,
                    &inner,
                    &small,
                    inset.x_limit,
                    inset.bin_width,
                    inset.log_scale,
                    None,
                    None,
                );
            }
        }
    }
    figure
}

/// Appearance of the sequence histogram. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SequenceOptions {
    pub width: f64,
    pub height: f64,
    /// Width of the bins; a fortieth of the range if `None`.
    pub bin_width: Option<f64>,
    pub log_scale: bool,
    /// Limits of the x axis; the full range if `None`.
    pub x_limits: Option<(f64, f64)>,
    pub legend: LegendPosition,
}

impl Default for SequenceOptions {
    fn default() -> Self {
        SequenceOptions {
            width: 7.0,
            height: 7.0,
            bin_width: None,
            log_scale: false,
            x_limits: None,
            legend: LegendPosition::UpperRight,
        }
    }
}

/// Histogram of the number of sequences assigned to each feature, for the
/// two samples of a profile.
pub fn sequence_histogram(profile: &SampleProfile, options: &SequenceOptions) -> Figure {
    let features = profile.profile.features();
    let series: Vec<Vec<f64>> = (0..2)
        .map(|s| features.iter().map(|f| f.counts[s]).collect())
        .collect();
    let all = series.iter().flatten().copied().filter(|v| v.is_finite());
    let (min, max) = all.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    if features.is_empty() || !min.is_finite() || max <= 0.0 {
        return Figure::message(NOTHING);
    }
    let bin_width = options
        .bin_width
        .filter(|w| *w > 0.0)
        .unwrap_or(((max - min) / 40.0).max(1e-9));
    let edges = bin_edges(max, bin_width);
    let counted: Vec<Series> = series
        .iter()
        .enumerate()
        .map(|(i, values)| Series {
            counts: count(values, &edges),
            color: group_color(i),
        })
        .collect();
    let most = counted
        .iter()
        .flat_map(|s| s.counts.iter().copied())
        .max()
        .unwrap_or(0);

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let frame = Frame {
        left: 0.5 * INCH,
        top: 0.2 * INCH,
        right: figure.width - 0.2 * INCH,
        bottom: figure.height - crate::axes::x_axis_room(),
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for a histogram");
    }
    let (x_min, x_max) = options
        .x_limits
        .filter(|(a, b)| b > a)
        .unwrap_or((0.0, edges[edges.len() - 1]));
    let x_ticks = Ticks::inside(x_min, x_max);
    let x = frame.x_scale(x_min, x_max);
    let (y, y_ticks) = count_axis(&frame, most, options.log_scale);
    bars(&mut figure, &frame, &x, &*y, &edges, &counted);
    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&x_ticks, &x),
        Some("Sequences"),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &y_ticks,
        Some("Number of features"),
        AXES_COLOR,
    );

    let names = &profile.profile.sample_names;
    let entries = vec![
        (names[0].clone(), group_color(0)),
        (names[1].clone(), group_color(1)),
    ];
    legend(&mut figure, &frame, &entries, options.legend);
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use stamp_core::{ProfileTree, Unclassified};

    fn rects(figure: &Figure, color: Color) -> usize {
        figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == color))
            .count()
    }

    #[test]
    fn binning() {
        let edges = bin_edges(1.0, 0.25);
        assert_eq!(edges, [0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(
            count(&[0.0, 0.1, 0.25, 0.99, 1.0, f64::NAN, -1.0, 1.5], &edges),
            [2, 1, 0, 2]
        );
        assert!(bin_edges(1.0, 1e-9).len() <= 100_001);
        // A value of exactly the limit is in the last bin, which ends at the
        // limit, for every bin width the window offers (a p-value of 1 used
        // to fall into a bin past the axis for widths of 0.1, 0.2, 0.25 and
        // 0.5, and 0.05 in the inset for 0.025 and 0.05).
        for (limit, width) in [
            (1.0, 0.1),
            (1.0, 0.2),
            (1.0, 0.25),
            (1.0, 0.5),
            (1.0, 0.01),
            (1.0, 0.03),
            (0.05, 0.025),
            (0.05, 0.05),
            (0.05, 0.002),
        ] {
            let edges = bin_edges(limit, width);
            assert!(*edges.last().unwrap() >= limit);
            assert!(edges[edges.len() - 2] < limit, "{limit} {width}");
            let counts = count(&[limit], &edges);
            assert_eq!(counts.last(), Some(&1), "{limit} {width}");
        }
    }

    #[test]
    fn p_value_histogram_with_and_without_inset() {
        let p: Vec<f64> = (0..200).map(|i| (i as f64 / 200.0).powi(2)).collect();
        let with_inset = p_value_histogram(&p, &PValueOptions::default());
        let without = p_value_histogram(
            &p,
            &PValueOptions {
                inset: None,
                ..PValueOptions::default()
            },
        );
        assert!(rects(&without, GREY) > 20);
        assert!(rects(&with_inset, GREY) > rects(&without, GREY));
        let labelled = |f: &Figure, needle: &str| {
            f.shapes
                .iter()
                .any(|s| matches!(s, Shape::Text { text, .. } if text == needle))
        };
        assert!(
            labelled(&without, "Corrected p-value") && labelled(&without, "Number of features")
        );

        let log = p_value_histogram(
            &p,
            &PValueOptions {
                log_scale: true,
                inset: None,
                ..PValueOptions::default()
            },
        );
        assert!(labelled(&log, "10") && labelled(&log, "1"));
        assert_eq!(
            p_value_histogram(&[], &PValueOptions::default())
                .shapes
                .len(),
            1
        );
        assert_eq!(
            p_value_histogram(&[f64::NAN], &PValueOptions::default())
                .shapes
                .len(),
            1
        );
    }

    #[test]
    fn sequence_histogram_has_both_samples() {
        let mut spf = String::from("F\tA\tB\n");
        for i in 0..50 {
            spf.push_str(&format!("f{i}\t{}\t{}\n", i * 10, (50 - i) * 7));
        }
        let tree = ProfileTree::read(&spf).unwrap();
        let profile = tree
            .sample_profile("A", "B", None, "F", Unclassified::Retain)
            .unwrap();
        let figure = sequence_histogram(&profile, &SequenceOptions::default());
        // Bars of both colours, plus one legend swatch each.
        assert!(rects(&figure, group_color(0)) > 10 && rects(&figure, group_color(1)) > 10);
        let wide = sequence_histogram(
            &profile,
            &SequenceOptions {
                bin_width: Some(250.0),
                legend: LegendPosition::None,
                ..SequenceOptions::default()
            },
        );
        assert_eq!(rects(&wide, group_color(0)), 2);
        let zoomed = sequence_histogram(
            &profile,
            &SequenceOptions {
                x_limits: Some((0.0, 100.0)),
                ..SequenceOptions::default()
            },
        );
        assert!(rects(&zoomed, group_color(0)) < rects(&figure, group_color(0)));
    }
}
