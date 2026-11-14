//! ANCOM plot: for every feature, how many of its log ratios with the other
//! features differ between groups (W) against how much it differs on the
//! centred log-ratio scale.

use stamp_core::stats::ancom::AncomResult;

use crate::axes::{
    legend, scaled_ticks, x_axis, y_axis, y_axis_width, Frame, LabelRotation, LegendPosition,
    AXES_COLOR, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, HAlign, Stroke, VAlign};

/// Appearance of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Area of the markers, in square points.
    pub marker_size: f64,
    pub legend: LegendPosition,
    /// Features outlined in red.
    pub highlighted: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 6.0,
            marker_size: 20.0,
            legend: LegendPosition::Best,
            highlighted: Vec::new(),
        }
    }
}

/// Colour of features called different, and of the rest.
const DIFFERENT: Color = Color::rgb(215, 95, 75);
const SAME: Color = Color::rgb(200, 200, 200);

/// The plot of an ANCOM result whose rows belong to `features`, comparing
/// `groups`.
pub fn plot(
    result: &AncomResult,
    features: &[String],
    groups: &[String],
    options: &Options,
) -> Figure {
    if result.rows.is_empty() || result.rows.len() != features.len() {
        return Figure::message("No features to show");
    }
    let effect_name = AncomResult::effect_name(groups.len());
    let (low, high) = result
        .rows
        .iter()
        .map(|r| r.clr_effect)
        .filter(|v| v.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    if !low.is_finite() {
        return Figure::message("No features to show");
    }
    let x_ticks = if groups.len() == 2 {
        // Symmetric about no difference.
        let reach = low.abs().max(high.abs()).max(1e-6) * 1.08;
        Ticks::covering(-reach, reach)
    } else {
        Ticks::covering(0.0, high.max(1e-6) * 1.05)
    };
    let most = features.len().saturating_sub(1).max(1) as f64;
    let y_ticks = Ticks::covering(0.0, most);

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let frame = Frame {
        left: padding + y_axis_width(&y_ticks, true),
        top: padding + 0.25 * INCH,
        right: figure.width - padding,
        bottom: figure.height - padding - 0.4 * INCH,
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for an ANCOM plot");
    }
    let (x, y) = (
        frame.x_scale(x_ticks.min, x_ticks.max),
        frame.y_scale(y_ticks.min, y_ticks.max),
    );
    let dashed = Stroke::dashed(AXES_COLOR, 1.0, 5.0, 4.0);
    if let Some(cutoff) = result.cutoff {
        let at = y.map(cutoff);
        figure.line(&[(frame.left, at), (frame.right, at)], dashed);
    }
    if groups.len() == 2 {
        let at = x.map(0.0);
        figure.line(&[(at, frame.top), (at, frame.bottom)], dashed);
    }

    let radius = options.marker_size.max(0.0).sqrt() / 2.0;
    // Features called different are drawn last, on top.
    let mut order: Vec<usize> = (0..features.len()).collect();
    order.sort_by_key(|&i| result.rows[i].reject);
    for i in order {
        let row = &result.rows[i];
        if !row.clr_effect.is_finite() {
            continue;
        }
        let (cx, cy) = (x.map(row.clr_effect), y.map(row.w as f64));
        let edge = if options.highlighted.contains(&features[i]) {
            Stroke::solid(Color::RED, 2.0)
        } else {
            Stroke::solid(Color::BLACK, 0.5)
        };
        let fill = if row.reject { DIFFERENT } else { SAME };
        figure.circle(cx, cy, radius, Some(fill), Some(edge));
        figure.region(
            cx - radius - 1.0,
            cy - radius - 1.0,
            2.0 * radius + 2.0,
            2.0 * radius + 2.0,
            &features[i],
            format!(
                "{}\n\nW: {} of {}\n{effect_name}: {:.3}\nDifferent: {}",
                features[i],
                row.w,
                features.len() - 1,
                row.clr_effect,
                if row.reject { "yes" } else { "no" }
            ),
        );
    }

    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&x_ticks, &x),
        Some(effect_name),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some("W"),
        AXES_COLOR,
    );
    let summary = format!(
        "ANCOM ({}, alpha = {}): {} of {} features differ",
        result.test.name(),
        result.alpha,
        result.rejected(),
        features.len()
    );
    figure.text(
        frame.left,
        frame.top - 5.0,
        summary,
        crate::axes::label_style().align(HAlign::Left, VAlign::Baseline),
    );
    let mut entries = vec![("Different".to_string(), DIFFERENT)];
    entries.push(("Not different".to_string(), SAME));
    legend(&mut figure, &frame, &entries, options.legend);
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use stamp_core::stats::ancom::ancom;
    use stamp_core::stats::multi_group::MultiGroupTest;

    #[test]
    fn a_point_for_every_feature() {
        let samples: Vec<Vec<f64>> = vec![
            vec![10.0, 20.0, 30.0, 40.0],
            vec![12.0, 22.0, 28.0, 38.0],
            vec![11.0, 19.0, 31.0, 42.0],
            vec![90.0, 21.0, 29.0, 41.0],
            vec![95.0, 18.0, 32.0, 39.0],
            vec![88.0, 23.0, 27.0, 40.0],
        ];
        let result = ancom(&samples, &[0, 0, 0, 1, 1, 1], MultiGroupTest::Anova, 0.05).unwrap();
        let features: Vec<String> = ["a", "b", "c", "d"].map(String::from).to_vec();
        let groups = vec!["A".to_string(), "B".to_string()];
        let figure = plot(&result, &features, &groups, &Options::default());
        assert_eq!(figure.regions.len(), 4);
        assert_eq!(figure.regions.iter().filter(|r| r.item == "a").count(), 1);
        let texts: Vec<String> = figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"W".to_string()));
        assert!(texts.contains(&"Difference in mean CLR".to_string()));
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("ANCOM (ANOVA, alpha = 0.05): ")),
            "{texts:?}"
        );
        assert!(plot(&result, &features[..2], &groups, &Options::default())
            .regions
            .is_empty());
    }
}
