//! Volcano plot: for every feature, how large the difference is against how
//! significant it is.

use stamp_core::stats::results::{
    format_p_value, CorrelationResults, GroupResults, MultiGroupResults, SampleResults,
};
use stamp_core::{GroupProfile, SampleProfile};

use crate::axes::{
    group_color, legend, scaled_ticks, x_axis, y_axis, y_axis_width, Frame, LabelRotation,
    LegendPosition, AXES_COLOR, INCH,
};
use crate::axis::Ticks;
use crate::extended_error_bar::{SAMPLE1_COLOR, SAMPLE2_COLOR};
use crate::figure::{Color, Figure, Stroke};
use crate::p_values::{tooltip_lines, PValueKind};

/// One feature.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub feature: String,
    pub effect_size: f64,
    pub p_value: f64,
    pub p_value_corrected: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub points: Vec<Point>,
    /// Label of the effect-size axis.
    pub effect_label: String,
    /// The effect size that means "no difference": zero for a difference
    /// and for the logarithm of a ratio.
    pub no_effect: f64,
    /// The two samples or groups compared, with their colours. A feature
    /// that is more abundant in the first lies to the right. `None` when
    /// the effect size has no direction, as for several groups.
    pub sides: Option<[(String, Color); 2]>,
    /// What the corrected p-values are, which names them.
    pub corrected_kind: PValueKind,
    /// For a ratio, plotted as its base-2 logarithm, the name of the ratio,
    /// which tooltips give alongside.
    pub log2_of: Option<String>,
}

impl Data {
    pub fn from_sample_results(profile: &SampleProfile, results: &SampleResults) -> Data {
        let ci = results.analysis.ci_method;
        let names = &profile.profile.sample_names;
        // A ratio is plotted as its logarithm, so that twice and half as
        // abundant lie equally far from no effect, on either side.
        let ratio = ci.is_ratio();
        Data {
            points: results
                .rows
                .iter()
                .map(|r| Point {
                    feature: r.feature.clone(),
                    effect_size: if ratio {
                        r.effect_size.log2()
                    } else {
                        r.effect_size
                    },
                    p_value: r.p_value,
                    p_value_corrected: r.p_value_corrected,
                })
                .collect(),
            effect_label: if ratio {
                format!("log2({})", ci.plot_label())
            } else {
                ci.plot_label().to_string()
            },
            no_effect: 0.0,
            log2_of: ratio.then(|| ci.plot_label().to_string()),
            sides: Some([
                (names[0].clone(), SAMPLE1_COLOR),
                (names[1].clone(), SAMPLE2_COLOR),
            ]),
            corrected_kind: PValueKind::of(results.correction.method),
        }
    }

    pub fn from_group_results(profile: &GroupProfile, results: &GroupResults) -> Data {
        Data {
            points: results
                .rows
                .iter()
                .map(|r| Point {
                    feature: r.feature.clone(),
                    effect_size: r.effect_size,
                    p_value: r.p_value,
                    p_value_corrected: r.p_value_corrected,
                })
                .collect(),
            effect_label: "Difference in mean proportions (%)".to_string(),
            no_effect: 0.0,
            sides: Some([
                (profile.group_name1.clone(), group_color(0)),
                (profile.group_name2.clone(), group_color(1)),
            ]),
            corrected_kind: PValueKind::of(results.correction.method),
            log2_of: None,
        }
    }

    /// Correlation coefficients against their p-values. `field` is the
    /// metadata field the features were correlated with.
    pub fn from_correlation_results(results: &CorrelationResults, field: &str) -> Data {
        Data {
            points: results
                .rows
                .iter()
                .map(|r| Point {
                    feature: r.feature.clone(),
                    effect_size: r.coefficient,
                    p_value: r.p_value,
                    p_value_corrected: r.p_value_corrected,
                })
                .collect(),
            effect_label: format!(
                "{} ({})",
                results.analysis.method.name(),
                results.analysis.method.coefficient_name()
            ),
            no_effect: 0.0,
            sides: Some([
                (format!("samples with a high {field}"), POSITIVE),
                (format!("samples with a low {field}"), NEGATIVE),
            ]),
            corrected_kind: PValueKind::of(results.correction.method),
            log2_of: None,
        }
    }

    pub fn from_multi_group_results(results: &MultiGroupResults) -> Data {
        Data {
            points: results
                .rows
                .iter()
                .map(|r| Point {
                    feature: r.feature.clone(),
                    effect_size: r.effect_size,
                    p_value: r.p_value,
                    p_value_corrected: r.p_value_corrected,
                })
                .collect(),
            effect_label: results.analysis.effect_size.name().to_string(),
            no_effect: 0.0,
            sides: None,
            corrected_kind: PValueKind::of(results.correction.method),
            log2_of: None,
        }
    }
}

/// Appearance of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Plot corrected rather than uncorrected p-values.
    pub corrected: bool,
    /// Features at or below this p-value count as significant; a dashed
    /// line marks it. `None` treats every feature as significant.
    pub significance: Option<f64>,
    /// Features must also differ by at least this much from no effect to
    /// count; dashed lines mark it.
    pub min_effect: Option<f64>,
    /// Area of the markers, in square points.
    pub marker_size: f64,
    pub legend: LegendPosition,
    /// Features outlined in red.
    pub highlighted: Vec<String>,
    /// p-values below ten to this power are drawn at it, and shown as a
    /// bound.
    pub min_p_value_exponent: i32,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 6.0,
            corrected: true,
            significance: Some(0.05),
            min_effect: None,
            marker_size: 20.0,
            legend: LegendPosition::Best,
            highlighted: Vec::new(),
            min_p_value_exponent: -15,
        }
    }
}

/// Colours of features that rise and that fall with a metadata field.
const POSITIVE: Color = Color::rgb(215, 95, 75);
const NEGATIVE: Color = Color::rgb(90, 140, 200);
/// Colour of the features that are not significant.
const MUTED: Color = Color::rgb(200, 200, 200);
/// Colour of significant features when the effect size has no direction.
const UNDIRECTED: Color = Color::rgb(128, 177, 211);

pub fn plot(data: &Data, options: &Options) -> Figure {
    let floor = 10f64.powi(options.min_p_value_exponent);
    let p_of = |point: &Point| {
        if options.corrected {
            point.p_value_corrected
        } else {
            point.p_value
        }
    };
    // Height of a feature: -log10 of its p-value, which cannot be zero.
    let height_of = |p: f64| -p.max(floor).log10();
    let points: Vec<&Point> = data
        .points
        .iter()
        .filter(|p| p.effect_size.is_finite() && p_of(p).is_finite())
        .collect();
    if points.is_empty() {
        return Figure::message("No active features or degenerate plot");
    }

    // The effect axis is symmetric about "no effect" when there are two
    // sides, so that left and right can be compared at a glance.
    let (low, high) = points
        .iter()
        .map(|p| p.effect_size)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    let x_ticks = if data.sides.is_some() {
        let reach = (high - data.no_effect)
            .abs()
            .max((low - data.no_effect).abs())
            .max(options.min_effect.unwrap_or(0.0))
            * 1.08;
        let reach = if reach > 0.0 { reach } else { 1.0 };
        Ticks::covering(data.no_effect - reach, data.no_effect + reach)
    } else {
        Ticks::covering(low.min(data.no_effect), high.max(data.no_effect))
    };
    let tallest = points
        .iter()
        .map(|p| height_of(p_of(p)))
        .chain(options.significance.map(height_of))
        .fold(1.0, f64::max);
    let y_ticks = Ticks::covering(0.0, tallest * 1.05);

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let frame = Frame {
        left: padding + y_axis_width(&y_ticks, true),
        top: padding,
        right: figure.width - padding,
        bottom: figure.height - padding - crate::axes::x_axis_room(),
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for a volcano plot");
    }
    let (x, y) = (
        frame.x_scale(x_ticks.min, x_ticks.max),
        frame.y_scale(y_ticks.min, y_ticks.max),
    );

    // The thresholds, drawn first so that the points lie on top of them.
    let dashed = Stroke::dashed(AXES_COLOR, 1.0, 5.0, 4.0);
    if let Some(level) = options.significance {
        let at = y.map(height_of(level));
        figure.line(&[(frame.left, at), (frame.right, at)], dashed);
    }
    let effect_lines: Vec<f64> = match (options.min_effect, data.sides.is_some()) {
        (Some(minimum), true) => vec![data.no_effect - minimum, data.no_effect + minimum],
        (Some(minimum), false) => vec![data.no_effect + minimum],
        (None, true) => vec![data.no_effect],
        (None, false) => Vec::new(),
    };
    for value in effect_lines {
        if (x_ticks.min..=x_ticks.max).contains(&value) {
            let at = x.map(value);
            figure.line(&[(at, frame.top), (at, frame.bottom)], dashed);
        }
    }

    let significant = |point: &Point| {
        options
            .significance
            .is_none_or(|level| p_of(point) <= level)
            && options
                .min_effect
                .is_none_or(|minimum| (point.effect_size - data.no_effect).abs() >= minimum)
    };
    let p_name = if options.corrected {
        data.corrected_kind
    } else {
        PValueKind::Uncorrected
    }
    .label();
    let radius = options.marker_size.max(0.0).sqrt() / 2.0;
    // Features that are not significant first, so that those that are lie
    // on top where they crowd together.
    let mut ordered = points.clone();
    ordered.sort_by_key(|p| significant(p));
    for point in ordered {
        let color = if !significant(point) {
            MUTED
        } else {
            match &data.sides {
                Some([(_, first), (_, second)]) => {
                    if point.effect_size >= data.no_effect {
                        *first
                    } else {
                        *second
                    }
                }
                None => UNDIRECTED,
            }
        };
        let edge = if options.highlighted.contains(&point.feature) {
            Stroke::solid(Color::RED, 2.0)
        } else {
            Stroke::solid(Color::BLACK, 0.5)
        };
        let (cx, cy) = (x.map(point.effect_size), y.map(height_of(p_of(point))));
        figure.circle(cx, cy, radius, Some(color), Some(edge));
        figure.region(
            cx - radius - 1.0,
            cy - radius - 1.0,
            2.0 * radius + 2.0,
            2.0 * radius + 2.0,
            &point.feature,
            format!(
                "{}\n\n{}\n{}",
                point.feature,
                match &data.log2_of {
                    Some(ratio) => format!(
                        "{ratio}: {:.3}\n{}: {:.3}",
                        point.effect_size.exp2(),
                        data.effect_label,
                        point.effect_size
                    ),
                    None => format!("{}: {:.3}", data.effect_label, point.effect_size),
                },
                tooltip_lines(
                    &format_p_value(point.p_value, options.min_p_value_exponent),
                    &format_p_value(point.p_value_corrected, options.min_p_value_exponent),
                    data.corrected_kind,
                ),
            ),
        );
    }

    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&x_ticks, &x),
        Some(&data.effect_label),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some(&format!("\u{2212}log10({p_name})")),
        AXES_COLOR,
    );
    if let Some(sides) = &data.sides {
        // Named for what a feature on each side has more of.
        let entries: Vec<(String, Color)> = sides
            .iter()
            .map(|(name, color)| (format!("More in {name}"), *color))
            .collect();
        legend(&mut figure, &frame, &entries, options.legend);
    }
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn point(name: &str, effect: f64, p: f64, corrected: f64) -> Point {
        Point {
            feature: name.to_string(),
            effect_size: effect,
            p_value: p,
            p_value_corrected: corrected,
        }
    }

    fn data() -> Data {
        Data {
            points: vec![
                point("up", 4.0, 0.0001, 0.001),
                point("down", -3.0, 0.002, 0.01),
                point("small", 0.2, 0.001, 0.004),
                point("unsure", 2.0, 0.04, 0.2),
                point("certain", 1.0, 0.0, 0.0),
                point("broken", f64::NAN, 0.5, 0.5),
            ],
            effect_label: "Difference in mean proportions (%)".to_string(),
            no_effect: 0.0,
            sides: Some([
                ("Lean".to_string(), group_color(0)),
                ("Obese".to_string(), group_color(1)),
            ]),
            corrected_kind: PValueKind::Corrected,
            log2_of: None,
        }
    }

    fn circles(figure: &Figure) -> Vec<(f64, f64, Color)> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Circle {
                    cx,
                    cy,
                    fill: Some(fill),
                    ..
                } => Some((*cx, *cy, *fill)),
                _ => None,
            })
            .collect()
    }

    fn position(figure: &Figure, feature: &str) -> (f64, f64) {
        let region = figure.regions.iter().find(|r| r.item == feature).unwrap();
        (
            region.x + region.width / 2.0,
            region.y + region.height / 2.0,
        )
    }

    fn colour_of(figure: &Figure, feature: &str) -> Color {
        let (x, y) = position(figure, feature);
        circles(figure)
            .into_iter()
            .find(|(cx, cy, _)| (cx - x).abs() < 1e-6 && (cy - y).abs() < 1e-6)
            .unwrap()
            .2
    }

    #[test]
    fn features_are_placed_by_effect_and_significance() {
        let figure = plot(&data(), &Options::default());
        // Five features can be drawn; the one without an effect size cannot.
        assert_eq!(figure.regions.len(), 5);
        let (up, down) = (position(&figure, "up"), position(&figure, "down"));
        // More in the first group lies to the right, and the axis is
        // symmetric about no difference.
        assert!(up.0 > down.0);
        // "certain" (+1) and "small" (+0.2) lie between them, in order.
        let (certain, small) = (position(&figure, "certain"), position(&figure, "small"));
        assert!(down.0 < small.0 && small.0 < certain.0 && certain.0 < up.0);
        // More significant is higher on the page (smaller y).
        assert!(up.1 < down.1 && down.1 < position(&figure, "unsure").1);
        // A p-value of zero is drawn at the floor rather than at infinity.
        assert!(position(&figure, "certain").1 >= 0.0);

        assert_eq!(colour_of(&figure, "up"), group_color(0));
        assert_eq!(colour_of(&figure, "down"), group_color(1));
        assert_eq!(colour_of(&figure, "unsure"), MUTED);
        let texts: Vec<&str> = figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        for expected in [
            "Difference in mean proportions (%)",
            "\u{2212}log10(Corrected p-value)",
            "More in Lean",
            "More in Obese",
        ] {
            assert!(texts.contains(&expected), "missing {expected}");
        }
        let tooltip = &figure
            .regions
            .iter()
            .find(|r| r.item == "up")
            .unwrap()
            .tooltip;
        assert!(
            tooltip.contains("Uncorrected p-value: 1.00e-4\nCorrected p-value: 1.00e-3"),
            "{tooltip}"
        );
    }

    #[test]
    fn ratios_are_plotted_as_their_logarithm() {
        use stamp_core::stats::ci::SampleCi;
        use stamp_core::stats::results::{run_sample_analysis, SampleAnalysis};
        let tree =
            stamp_core::ProfileTree::read("F\ta\tb\nup\t40\t10\ndown\t10\t40\nsame\t50\t50\n")
                .unwrap();
        let profile = tree
            .sample_profile("a", "b", None, "F", stamp_core::Unclassified::Retain)
            .unwrap();
        let analysis = SampleAnalysis {
            test: stamp_core::stats::two_sample::SampleTest::Fisher,
            test_type: stamp_core::stats::TestType::TwoSided,
            ci_method: SampleCi::RatioProportions,
            coverage: 0.95,
            correction: stamp_core::stats::correction::Correction::None,
        };
        let results = run_sample_analysis(
            &profile,
            analysis,
            &stamp_core::stats::StatsSettings::default(),
            &mut stamp_core::stats::rng_from_seed(1),
            &mut |_, _| true,
        )
        .unwrap();
        let data = Data::from_sample_results(&profile, &results);
        assert_eq!(data.effect_label, "log2(Ratio of proportions)");
        assert_eq!(data.no_effect, 0.0);
        let effect = |name: &str| {
            data.points
                .iter()
                .find(|p| p.feature == name)
                .unwrap()
                .effect_size
        };
        // Four times and a quarter as abundant lie equally far from no
        // effect, on either side.
        assert!((effect("up") - 2.0).abs() < 1e-9);
        assert!((effect("down") + 2.0).abs() < 1e-9);
        let figure = plot(&data, &Options::default());
        let tooltip = &figure
            .regions
            .iter()
            .find(|r| r.item == "up")
            .unwrap()
            .tooltip;
        assert!(
            tooltip.contains("Ratio of proportions: 4.000\nlog2(Ratio of proportions): 2.000"),
            "{tooltip}"
        );
    }

    #[test]
    fn thresholds_decide_what_is_coloured() {
        // With a minimum effect, a small but significant difference is muted.
        let strict = Options {
            min_effect: Some(1.5),
            ..Options::default()
        };
        let figure = plot(&data(), &strict);
        assert_eq!(colour_of(&figure, "small"), MUTED);
        assert_eq!(colour_of(&figure, "up"), group_color(0));
        assert_eq!(colour_of(&figure, "certain"), MUTED);
        // Uncorrected p-values: "unsure" (0.04) now passes 0.05.
        let uncorrected = Options {
            corrected: false,
            ..Options::default()
        };
        let figure = plot(&data(), &uncorrected);
        assert_eq!(colour_of(&figure, "unsure"), group_color(0));
        assert!(figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Text { text, .. } if text == "\u{2212}log10(Uncorrected p-value)")));
        // Without a level everything is coloured, and no line is drawn for it.
        let dashes = |figure: &Figure| {
            figure
                .shapes
                .iter()
                .filter(|s| matches!(s, Shape::Line { stroke, .. } if stroke.dash.is_some()))
                .count()
        };
        let none = Options {
            significance: None,
            ..Options::default()
        };
        assert_eq!(colour_of(&plot(&data(), &none), "unsure"), group_color(0));
        assert_eq!(dashes(&plot(&data(), &Options::default())), 2);
        assert_eq!(dashes(&plot(&data(), &none)), 1);
        assert_eq!(dashes(&plot(&data(), &strict)), 3);

        // Highlighted features are outlined in red.
        let highlighted = plot(
            &data(),
            &Options {
                highlighted: vec!["down".to_string()],
                ..Options::default()
            },
        );
        assert!(highlighted.shapes.iter().any(|s| matches!(
            s,
            Shape::Circle { stroke: Some(stroke), .. } if stroke.color == Color::RED
        )));
    }

    #[test]
    fn undirected_effects_and_empty_data() {
        let mut several = data();
        several.sides = None;
        several.effect_label = "Eta-squared".to_string();
        several.corrected_kind = PValueKind::QValue;
        for point in &mut several.points {
            point.effect_size = point.effect_size.abs() / 5.0;
        }
        let figure = plot(&several, &Options::default());
        assert_eq!(colour_of(&figure, "up"), UNDIRECTED);
        assert!(figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Text { text, .. } if text == "\u{2212}log10(q-value)")));
        assert!(!figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Text { text, .. } if text.starts_with("More in"))));

        let empty = Data {
            points: Vec::new(),
            ..data()
        };
        assert_eq!(plot(&empty, &Options::default()).shapes.len(), 1);
    }
}
