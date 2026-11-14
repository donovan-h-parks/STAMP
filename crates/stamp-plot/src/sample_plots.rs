//! Plots of two-sample results: the profile bar plot, the bar plot of a
//! chosen statistic, and the multiple-comparison plots.

use stamp_core::stats::results::{SampleResults, SampleRow};
use stamp_core::stats::wilson::wilson_interval;
use stamp_core::SampleProfile;

use crate::axes::{
    self, fonts, group_color, legend, scaled_ticks, tick_style, x_axis, y_axis, y_axis_width,
    Frame, LabelRotation, LegendPosition, AXES_COLOR, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, HAlign, Stroke, VAlign};
use crate::text;

const NOTHING: &str = "No active features or degenerate plot";

fn shorten(name: &str, limit: Option<usize>) -> String {
    match limit {
        Some(length) => text::truncate(name, length),
        None => name.to_string(),
    }
}

// ------------------------------------------------------------ profile bar

/// Appearance of the profile bar plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ProfileBarOptions {
    /// Width given to each feature.
    pub column_width: f64,
    pub height: f64,
    /// Plot sequence counts instead of proportions.
    pub sequence_counts: bool,
    /// Draw 95% Wilson confidence intervals.
    pub show_cis: bool,
    /// Size of the caps on the confidence intervals, in points.
    pub end_cap_size: f64,
    /// Number of features shown, most abundant first.
    pub features_to_show: usize,
    /// Width of each pair of bars as a percentage of the column.
    pub bar_width_percent: f64,
    /// Mark features whose corrected p-value is at or below the threshold.
    pub show_p_values: bool,
    pub p_value_threshold: f64,
    pub legend: LegendPosition,
    pub truncate_labels: Option<usize>,
    pub highlighted: Vec<String>,
}

impl Default for ProfileBarOptions {
    fn default() -> Self {
        ProfileBarOptions {
            column_width: 0.25,
            height: 6.0,
            sequence_counts: false,
            show_cis: true,
            end_cap_size: 0.0,
            features_to_show: 50,
            bar_width_percent: 80.0,
            show_p_values: true,
            p_value_threshold: 0.05,
            legend: LegendPosition::UpperRight,
            truncate_labels: Some(50),
            highlighted: Vec::new(),
        }
    }
}

/// Profile bar plot: the abundance of the most abundant features in both
/// samples. `rows` selects the features considered, typically the filtered
/// results.
pub fn profile_bar<'a>(
    profile: &SampleProfile,
    rows: impl IntoIterator<Item = &'a SampleRow>,
    options: &ProfileBarOptions,
) -> Figure {
    let mut rows: Vec<&SampleRow> = rows.into_iter().collect();
    rows.sort_by(|a, b| {
        b.seq1
            .total_cmp(&a.seq1)
            .then_with(|| a.feature.cmp(&b.feature))
    });
    rows.truncate(options.features_to_show);
    if rows.is_empty() {
        return Figure::message(NOTHING);
    }

    // Value and the length of the lower half of its confidence interval.
    let value = |seq: f64, parent: f64| -> (f64, f64) {
        let (low, _, p) = wilson_interval(seq, parent, 1.96);
        let scale = if options.sequence_counts {
            parent
        } else {
            100.0
        };
        let shown = if options.sequence_counts {
            seq
        } else {
            seq * 100.0 / parent.max(1.0)
        };
        (shown, ((p - low) * scale).max(0.0))
    };
    let bars: Vec<[(f64, f64); 2]> = rows
        .iter()
        .map(|r| [value(r.seq1, r.parent_seq1), value(r.seq2, r.parent_seq2)])
        .collect();
    let top_value = bars
        .iter()
        .flatten()
        .map(|(v, e)| v + if options.show_cis { *e } else { 0.0 })
        .fold(0.0, f64::max);
    let ticks = Ticks::covering(0.0, top_value.max(f64::MIN_POSITIVE));

    let labels: Vec<String> = rows
        .iter()
        .map(|r| shorten(&r.feature, options.truncate_labels))
        .collect();
    let longest = labels
        .iter()
        .map(|l| text::width(l, fonts().tick))
        .fold(0.0, f64::max);
    let padding = 0.25 * INCH;
    let column = options.column_width * INCH;
    let left = padding + y_axis_width(&ticks, true);
    let frame = Frame {
        left,
        top: padding,
        right: left + column * rows.len() as f64,
        bottom: options.height * INCH - padding - longest - axes::TICK_PAD,
    };
    if frame.height() < 0.5 * INCH {
        return Figure::message("The plot is too short for its labels; increase the figure height");
    }
    let mut figure = Figure::new(frame.right + padding, options.height * INCH);
    let y = frame.y_scale(ticks.min, ticks.max);
    let outline = Stroke::solid(Color::BLACK, 1.0);
    let bar_width = column * (options.bar_width_percent / 100.0).clamp(0.05, 1.0) / 2.0;

    let mut x_ticks = Vec::new();
    for (i, (row, pair)) in rows.iter().zip(&bars).enumerate() {
        let start = frame.left + column * i as f64 + (column - 2.0 * bar_width) / 2.0;
        for (s, &(shown, error)) in pair.iter().enumerate() {
            let x = start + bar_width * s as f64;
            let top = y.map(shown);
            figure.rect(
                x,
                top,
                bar_width,
                frame.bottom - top,
                Some(group_color(s)),
                Some(outline),
            );
            if options.show_cis && error > 0.0 {
                let centre = x + bar_width / 2.0;
                let (low, high) = (y.map((shown - error).max(0.0)), y.map(shown + error));
                figure.line(&[(centre, low), (centre, high)], outline);
                if options.end_cap_size > 0.0 {
                    for cap in [low, high] {
                        figure.line(
                            &[
                                (centre - options.end_cap_size, cap),
                                (centre + options.end_cap_size, cap),
                            ],
                            outline,
                        );
                    }
                }
            }
        }
        let centre = start + bar_width;
        if options.show_p_values && row.p_value_corrected <= options.p_value_threshold {
            let tallest = pair
                .iter()
                .map(|(v, e)| v + if options.show_cis { *e } else { 0.0 })
                .fold(0.0, f64::max);
            figure.text(
                centre,
                y.map(tallest) - 2.0,
                "*",
                tick_style().align(HAlign::Center, VAlign::Baseline),
            );
        }
        x_ticks.push((centre, labels[i].clone()));
        figure.region(
            frame.left + column * i as f64,
            frame.top,
            column,
            frame.height(),
            &row.feature,
            format!(
                "{}\n{}: {:.3}\n{}: {:.3}\np-value (corrected): {:.3e}",
                row.feature,
                profile.profile.sample_names[0],
                pair[0].0,
                profile.profile.sample_names[1],
                pair[1].0,
                row.p_value_corrected
            ),
        );
    }

    x_axis(
        &mut figure,
        &frame,
        &x_ticks,
        None,
        LabelRotation::Vertical,
        AXES_COLOR,
    );
    // Highlighted features are drawn again in red over their labels.
    for (i, row) in rows.iter().enumerate() {
        if options.highlighted.contains(&row.feature) {
            figure.text(
                x_ticks[i].0,
                frame.bottom + axes::TICK_PAD,
                labels[i].clone(),
                tick_style()
                    .align(HAlign::Right, VAlign::Middle)
                    .rotated(90.0)
                    .color(Color::RED),
            );
        }
    }
    let y_label = if options.sequence_counts {
        "Number of sequences"
    } else {
        "Proportion of sequences (%)"
    };
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&ticks, &y),
        Some(y_label),
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

// ---------------------------------------------------- bar plot of a field

/// The statistic shown by the two-sample bar plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BarField {
    Sequences,
    ParentSequences,
    Proportions,
    PValues,
    CorrectedPValues,
    EffectSize,
}

/// Appearance of the two-sample bar plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct StatBarOptions {
    pub width: f64,
    pub row_height: f64,
    pub field: BarField,
    /// Order the features by the plotted value.
    pub sort: bool,
    pub legend: LegendPosition,
    pub truncate_labels: Option<usize>,
    pub highlighted: Vec<String>,
}

impl Default for StatBarOptions {
    fn default() -> Self {
        StatBarOptions {
            width: 7.0,
            row_height: 0.2,
            field: BarField::Proportions,
            sort: true,
            legend: LegendPosition::LowerRight,
            truncate_labels: Some(50),
            highlighted: Vec::new(),
        }
    }
}

/// Horizontal bars of one statistic for each feature: a pair of bars for
/// per-sample statistics, a single bar otherwise.
pub fn stat_bar<'a>(
    profile: &SampleProfile,
    results: &SampleResults,
    rows: impl IntoIterator<Item = &'a SampleRow>,
    options: &StatBarOptions,
) -> Figure {
    let ci = results.analysis.ci_method;
    let log = options.field == BarField::EffectSize && ci.is_ratio();
    let values = |r: &SampleRow| -> (f64, Option<f64>) {
        match options.field {
            BarField::Sequences => (r.seq1, Some(r.seq2)),
            BarField::ParentSequences => (r.parent_seq1, Some(r.parent_seq2)),
            BarField::Proportions => (r.rel_freq1, Some(r.rel_freq2)),
            BarField::PValues => (r.p_value, None),
            BarField::CorrectedPValues => (r.p_value_corrected, None),
            BarField::EffectSize => (
                if log {
                    r.effect_size.log10()
                } else {
                    r.effect_size
                },
                None,
            ),
        }
    };
    let x_label = match options.field {
        BarField::Sequences => "Number of sequences".to_string(),
        BarField::ParentSequences => "Number of parental sequences".to_string(),
        BarField::Proportions => "Proportion of sequences (%)".to_string(),
        BarField::PValues => "p-values".to_string(),
        BarField::CorrectedPValues => "p-values (corrected)".to_string(),
        BarField::EffectSize if log => format!("log({})", ci.plot_label()),
        BarField::EffectSize => ci.plot_label().to_string(),
    };

    // Rows are listed top to bottom.
    let mut rows: Vec<&SampleRow> = rows.into_iter().collect();
    if options.sort {
        match options.field {
            BarField::PValues | BarField::CorrectedPValues => {
                rows.sort_by(|a, b| values(a).0.total_cmp(&values(b).0))
            }
            BarField::EffectSize => {
                rows.sort_by(|a, b| values(b).0.abs().total_cmp(&values(a).0.abs()))
            }
            _ => rows.sort_by(|a, b| values(b).0.total_cmp(&values(a).0)),
        }
    }
    if rows.is_empty() {
        return Figure::message(NOTHING);
    }

    let all: Vec<f64> = rows
        .iter()
        .flat_map(|r| {
            let (a, b) = values(r);
            [Some(a), b]
        })
        .flatten()
        .filter(|v| v.is_finite())
        .collect();
    let (low, high) = all
        .iter()
        .fold((0.0f64, 0.0f64), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let ticks = Ticks::covering(low, if high > low { high } else { low + 1.0 });

    let labels: Vec<String> = rows
        .iter()
        .map(|r| shorten(&r.feature, options.truncate_labels))
        .collect();
    let label_width = labels
        .iter()
        .map(|l| text::width(l, fonts().tick))
        .fold(0.0, f64::max);
    let padding = 0.2 * INCH;
    let n = rows.len() as f64;
    let mut figure = Figure::new(
        options.width * INCH,
        n * options.row_height * INCH + padding + crate::axes::x_axis_room(),
    );
    let frame = Frame {
        left: label_width + 0.1 * INCH,
        top: padding,
        right: figure.width - padding,
        bottom: figure.height - crate::axes::x_axis_room(),
    };
    if frame.width() < INCH {
        return Figure::message("The plot is too narrow for its labels; increase the figure width");
    }
    let x = frame.x_scale(ticks.min, ticks.max);
    let row_height = frame.height() / n;
    let outline = Stroke::solid(Color::BLACK, 1.0);
    let origin = x.map(0.0f64.clamp(ticks.min, ticks.max));

    for (i, row) in rows.iter().enumerate() {
        let centre = frame.top + row_height * (i as f64 + 0.5);
        let (first, second) = values(row);
        let bar = row_height * 0.35;
        let mut draw = |value: f64, top: f64, height: f64, color: Color| {
            if value.is_finite() {
                let end = x.map(value);
                figure.rect(
                    origin.min(end),
                    top,
                    (end - origin).abs(),
                    height,
                    Some(color),
                    Some(outline),
                );
            }
        };
        match second {
            Some(second) => {
                draw(first, centre - bar, bar, group_color(0));
                draw(second, centre, bar, group_color(1));
            }
            None => draw(first, centre - bar, 2.0 * bar, Color::grey(0.5)),
        }
        let color = if options.highlighted.contains(&row.feature) {
            Color::RED
        } else {
            Color::BLACK
        };
        figure.text(
            frame.left - axes::TICK_PAD,
            centre,
            labels[i].clone(),
            tick_style()
                .align(HAlign::Right, VAlign::Middle)
                .color(color),
        );
        figure.region(
            0.0,
            centre - row_height / 2.0,
            figure.width,
            row_height,
            &row.feature,
            format!("{}\n{x_label}: {first:.4}", row.feature),
        );
    }
    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&ticks, &x),
        Some(&x_label),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    figure.line(
        &[(frame.left, frame.top), (frame.left, frame.bottom)],
        Stroke::solid(AXES_COLOR, 1.0),
    );
    if matches!(
        options.field,
        BarField::Sequences | BarField::ParentSequences | BarField::Proportions
    ) {
        let names = &profile.profile.sample_names;
        let entries = vec![
            (names[0].clone(), group_color(0)),
            (names[1].clone(), group_color(1)),
        ];
        legend(&mut figure, &frame, &entries, options.legend);
    }
    figure
}

// ------------------------------------------------- multiple comparisons

/// Appearance of the multiple-comparison plots. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MultCompOptions {
    pub width: f64,
    pub height: f64,
    pub bin_width: f64,
    /// Count the features of the histogram on a logarithmic axis.
    pub log_scale: bool,
    /// Largest corrected p-value in the histogram.
    pub histogram_limit: f64,
    /// Largest corrected p-value in the corrected-against-uncorrected plot.
    pub correction_limit: f64,
    /// Largest corrected p-value in the plot of significant features.
    pub significant_limit: f64,
}

impl Default for MultCompOptions {
    fn default() -> Self {
        MultCompOptions {
            width: 7.0,
            height: 3.5,
            bin_width: 0.01,
            log_scale: false,
            histogram_limit: 0.1,
            correction_limit: 1.0,
            significant_limit: 0.1,
        }
    }
}

/// Three views of a multiple-test correction: a histogram of corrected
/// p-values, corrected against uncorrected p-values, and the number of
/// significant features at each significance level.
pub fn multiple_comparison(results: &SampleResults, options: &MultCompOptions) -> Figure {
    let mut pairs: Vec<(f64, f64)> = results
        .rows
        .iter()
        .map(|r| (r.p_value_corrected, r.p_value))
        .filter(|(c, p)| c.is_finite() && p.is_finite())
        .collect();
    if pairs.is_empty() || options.bin_width <= 0.0 {
        return Figure::message(NOTHING);
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let x_label = if results.correction.method.is_false_discovery_rate() {
        "q-value"
    } else {
        "corrected p-value"
    };

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let (left_margin, gap) = (0.55 * INCH, 0.6 * INCH);
    let panel_width = (figure.width - left_margin - 0.2 * INCH - 2.0 * gap) / 3.0;
    if panel_width < 0.7 * INCH || figure.height < 1.5 * INCH {
        return Figure::message("The figure is too small for these plots");
    }
    let panel_bottom = figure.height - 0.45 * INCH;
    let panel = |index: usize| {
        let left = left_margin + (panel_width + gap) * index as f64;
        Frame {
            left,
            top: 0.2 * INCH,
            right: left + panel_width,
            bottom: panel_bottom,
        }
    };
    let grey = Color::grey(0.5);

    // 1. Histogram of corrected values.
    let frame = panel(0);
    let limit = options.histogram_limit;
    let bins = ((limit / options.bin_width).ceil() as usize).clamp(1, 10_000);
    let mut counts = vec![0usize; bins];
    for (corrected, _) in pairs.iter().filter(|(c, _)| *c <= limit) {
        counts[((corrected / options.bin_width) as usize).min(bins - 1)] += 1;
    }
    let most = counts.iter().copied().max().unwrap_or(0);
    let x = frame.x_scale(0.0, limit);
    let (y, y_tick_labels) = crate::histogram::count_axis(&frame, most, options.log_scale);
    for (i, &n) in counts.iter().enumerate() {
        if n > 0 {
            let (a, b) = (
                x.map(i as f64 * options.bin_width),
                x.map(((i + 1) as f64 * options.bin_width).min(limit)),
            );
            figure.rect(
                a,
                y(n as f64),
                b - a,
                frame.bottom - y(n as f64),
                Some(grey),
                Some(Stroke::solid(Color::BLACK, 0.5)),
            );
        }
    }
    let few_ticks = |limit: f64| {
        let t = Ticks::inside(0.0, limit);
        // Narrow panels: label every other tick.
        let labels = t.labels();
        t.positions
            .iter()
            .zip(labels)
            .enumerate()
            .map(|(i, (p, l))| (*p, if i % 2 == 0 { l } else { String::new() }))
            .collect::<Vec<_>>()
    };
    let x_ticks: Vec<(f64, String)> = few_ticks(limit)
        .into_iter()
        .map(|(p, l)| (x.map(p), l))
        .collect();
    x_axis(
        &mut figure,
        &frame,
        &x_ticks,
        Some(x_label),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &y_tick_labels,
        Some("Number of features"),
        AXES_COLOR,
    );

    // 2. Corrected against uncorrected values.
    let frame = panel(1);
    let limit = options.correction_limit;
    let shown: Vec<&(f64, f64)> = pairs.iter().filter(|(c, _)| *c <= limit).collect();
    let y_ticks = Ticks::covering(
        0.0,
        shown
            .iter()
            .map(|(_, p)| *p)
            .fold(f64::MIN_POSITIVE, f64::max),
    );
    let (x, y) = (
        frame.x_scale(0.0, limit),
        frame.y_scale(y_ticks.min, y_ticks.max),
    );
    for (corrected, raw) in shown {
        figure.circle(
            x.map(*corrected),
            y.map(*raw),
            2.0,
            Some(grey),
            Some(Stroke::solid(Color::BLACK, 0.5)),
        );
    }
    let x_ticks: Vec<(f64, String)> = few_ticks(limit)
        .into_iter()
        .map(|(p, l)| (x.map(p), l))
        .collect();
    x_axis(
        &mut figure,
        &frame,
        &x_ticks,
        Some(x_label),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some("p-value"),
        AXES_COLOR,
    );

    // 3. Number of significant features at each level.
    let frame = panel(2);
    let limit = options.significant_limit;
    let within = pairs.iter().filter(|(c, _)| *c <= limit).count();
    let y_ticks = Ticks::covering(0.0, (within.max(1)) as f64);
    let (x, y) = (
        frame.x_scale(0.0, limit),
        frame.y_scale(y_ticks.min, y_ticks.max),
    );
    let mut points: Vec<(f64, f64)> = vec![(x.map(0.0), y.map(0.0))];
    for (i, (corrected, _)) in pairs.iter().take(within).enumerate() {
        // A step up at each corrected value.
        points.push((x.map(*corrected), y.map(i as f64)));
        points.push((x.map(*corrected), y.map(i as f64 + 1.0)));
    }
    points.push((x.map(limit), y.map(within as f64)));
    figure.line(&points, Stroke::solid(Color::BLACK, 1.0));
    let x_ticks: Vec<(f64, String)> = few_ticks(limit)
        .into_iter()
        .map(|(p, l)| (x.map(p), l))
        .collect();
    x_axis(
        &mut figure,
        &frame,
        &x_ticks,
        Some(x_label),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some("# significant features"),
        AXES_COLOR,
    );
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use stamp_core::stats::ci::SampleCi;
    use stamp_core::stats::correction::Correction;
    use stamp_core::stats::results::{run_sample_analysis, SampleAnalysis};
    use stamp_core::stats::two_sample::SampleTest;
    use stamp_core::stats::{rng_from_seed, StatsSettings, TestType};
    use stamp_core::{ProfileTree, Unclassified};

    fn setup(ci_method: SampleCi) -> (SampleProfile, SampleResults) {
        let mut spf = String::from("F\tA\tB\n");
        for i in 1..=12 {
            spf.push_str(&format!("feature {i}\t{}\t{}\n", i * i * 10, (13 - i) * 40));
        }
        let tree = ProfileTree::read(&spf).unwrap();
        let profile = tree
            .sample_profile("A", "B", None, "F", Unclassified::Retain)
            .unwrap();
        let analysis = SampleAnalysis {
            test: SampleTest::Fisher,
            test_type: TestType::TwoSided,
            ci_method,
            coverage: 0.95,
            correction: Correction::BenjaminiHochberg,
        };
        let results = run_sample_analysis(
            &profile,
            analysis,
            &StatsSettings::default(),
            &mut rng_from_seed(1),
            &mut |_, _| true,
        )
        .unwrap();
        (profile, results)
    }

    fn rects(figure: &Figure, color: Color) -> usize {
        figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == color))
            .count()
    }

    fn has_text(figure: &Figure, needle: &str) -> bool {
        figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Text { text, .. } if text == needle))
    }

    #[test]
    fn profile_bar_shows_the_most_abundant_features() {
        let (profile, results) = setup(SampleCi::DiffBetweenPropAsymptotic);
        let figure = profile_bar(
            &profile,
            &results.rows,
            &ProfileBarOptions {
                legend: LegendPosition::None,
                ..Default::default()
            },
        );
        assert_eq!(rects(&figure, group_color(0)), 12);
        assert_eq!(rects(&figure, group_color(1)), 12);
        // Most abundant in the first sample comes first.
        assert_eq!(figure.regions[0].item, "feature 12");
        assert!(has_text(&figure, "*") && has_text(&figure, "Proportion of sequences (%)"));

        let top3 = profile_bar(
            &profile,
            &results.rows,
            &ProfileBarOptions {
                features_to_show: 3,
                legend: LegendPosition::None,
                ..Default::default()
            },
        );
        assert_eq!(rects(&top3, group_color(0)), 3);
        assert!(top3.width < figure.width);
        let counts = profile_bar(
            &profile,
            &results.rows,
            &ProfileBarOptions {
                sequence_counts: true,
                show_cis: false,
                show_p_values: false,
                ..Default::default()
            },
        );
        assert!(has_text(&counts, "Number of sequences") && !has_text(&counts, "*"));
        assert_eq!(
            profile_bar(&profile, [], &ProfileBarOptions::default())
                .shapes
                .len(),
            1
        );
    }

    #[test]
    fn stat_bar_fields() {
        let (profile, results) = setup(SampleCi::DiffBetweenPropAsymptotic);
        let paired = stat_bar(
            &profile,
            &results,
            &results.rows,
            &StatBarOptions {
                legend: LegendPosition::None,
                ..Default::default()
            },
        );
        assert_eq!(
            (
                rects(&paired, group_color(0)),
                rects(&paired, group_color(1))
            ),
            (12, 12)
        );
        assert!((paired.height - (12.0 * 0.2 + 0.6) * 72.0).abs() < 1e-9);

        let p = stat_bar(
            &profile,
            &results,
            &results.rows,
            &StatBarOptions {
                field: BarField::CorrectedPValues,
                ..Default::default()
            },
        );
        assert_eq!(rects(&p, Color::grey(0.5)), 12);
        assert!(has_text(&p, "p-values (corrected)"));
        // Smallest p-value at the top.
        assert_eq!(p.regions[0].item, results.rows[0].feature);

        let effect = stat_bar(
            &profile,
            &results,
            &results.rows,
            &StatBarOptions {
                field: BarField::EffectSize,
                ..Default::default()
            },
        );
        assert!(has_text(&effect, "Difference between proportions (%)"));
        let (profile, ratio) = setup(SampleCi::RatioProportions);
        let effect = stat_bar(
            &profile,
            &ratio,
            &ratio.rows,
            &StatBarOptions {
                field: BarField::EffectSize,
                ..Default::default()
            },
        );
        assert!(has_text(&effect, "log(Ratio of proportions)"));
    }

    #[test]
    fn multiple_comparison_panels() {
        let (_, results) = setup(SampleCi::DiffBetweenPropAsymptotic);
        let figure = multiple_comparison(&results, &MultCompOptions::default());
        for label in [
            "Number of features",
            "p-value",
            "# significant features",
            "q-value",
        ] {
            assert!(has_text(&figure, label), "missing {label}");
        }
        let circles = figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Circle { .. }))
            .count();
        assert_eq!(circles, 12);
        assert_eq!(
            multiple_comparison(
                &results,
                &MultCompOptions {
                    width: 2.0,
                    ..Default::default()
                }
            )
            .shapes
            .len(),
            1
        );
    }
}
