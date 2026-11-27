//! Rank-abundance curves: each sample's features from the most abundant to
//! the least, against their proportion of the sample, on a logarithmic
//! scale. A steep curve is a community dominated by a few features; a flat
//! one is even.

use crate::axes::{
    legend, scaled_ticks, x_axis, y_axis, y_axis_width, Frame, LabelRotation, LegendPosition,
    AXES_COLOR, INCH,
};
use crate::axis::{LinearScale, Ticks};
use crate::composition;
use crate::figure::{Color, Figure, Stroke};

/// What each curve stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Curves {
    Samples,
    /// The mean proportions of each group's samples.
    Groups,
}

/// Appearance of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    pub curves: Curves,
    /// Proportions on a logarithmic scale.
    pub log_scale: bool,
    pub line_width: f64,
    pub legend: LegendPosition,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 5.0,
            curves: Curves::Samples,
            log_scale: true,
            line_width: 1.5,
            legend: LegendPosition::Best,
        }
    }
}

/// Ticks at the powers of ten covering `low` to `high` (both above zero),
/// as their logarithms, labelled with the value.
fn log_ticks(low: f64, high: f64) -> (Ticks, Vec<(f64, String)>) {
    let (first, last) = (low.log10().floor() as i32, high.log10().ceil() as i32);
    let last = last.max(first + 1);
    let ticks = Ticks::within(first as f64, last as f64, 1.0);
    let labels = (first..=last)
        .map(|k| {
            let value = 10f64.powi(k);
            let text = if k < 0 {
                format!("{value:.*}", (-k) as usize)
            } else {
                format!("{value:.0}")
            };
            (k as f64, text)
        })
        .collect();
    (ticks, labels)
}

pub fn plot(data: &composition::Data, options: &Options) -> Figure {
    let shown = match options.curves {
        Curves::Samples => data.clone(),
        Curves::Groups => data.by_group(),
    };
    // Every feature of the data: unclassified ones are among them or not
    // as the data was made.
    let kept: Vec<usize> = (0..shown.features.len()).collect();
    // Each curve: its name, its group, and its proportions largest first.
    let curves: Vec<(String, usize, Vec<f64>)> = (0..shown.samples.len())
        .map(|s| {
            let mut values: Vec<f64> = kept
                .iter()
                .map(|&f| shown.values[f][s])
                .filter(|v| *v > 0.0)
                .collect();
            values.sort_by(|a, b| b.total_cmp(a));
            (
                shown.samples[s].clone(),
                shown.sample_groups.get(s).copied().unwrap_or(0),
                values,
            )
        })
        .filter(|(_, _, values)| !values.is_empty())
        .collect();
    if curves.is_empty() {
        return Figure::message("No features to show");
    }
    let longest = curves.iter().map(|c| c.2.len()).max().unwrap_or(1);
    let highest = curves.iter().map(|c| c.2[0]).fold(0.0, f64::max);
    let lowest = curves
        .iter()
        .filter_map(|c| c.2.last().copied())
        .fold(f64::INFINITY, f64::min);

    let x_ticks = Ticks::covering(1.0, longest.max(2) as f64);
    let (y_ticks, y_labels) = if options.log_scale {
        log_ticks(lowest, highest)
    } else {
        let ticks = Ticks::covering(0.0, highest * 1.05);
        (ticks, Vec::new())
    };
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let frame = Frame {
        left: padding + y_axis_width(&y_ticks, true) + 0.1 * INCH,
        top: padding,
        right: figure.width - padding,
        bottom: figure.height - padding - crate::axes::x_axis_room(),
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for rank-abundance curves");
    }
    let x = frame.x_scale(x_ticks.min, x_ticks.max);
    let y: LinearScale = frame.y_scale(y_ticks.min, y_ticks.max);
    let place = |value: f64| {
        if options.log_scale {
            y.map(value.log10())
        } else {
            y.map(value)
        }
    };
    for (name, group, values) in &curves {
        let color = shown.group_color(*group);
        let points: Vec<(f64, f64)> = values
            .iter()
            .enumerate()
            .map(|(rank, &value)| (x.map((rank + 1) as f64), place(value)))
            .collect();
        if points.len() == 1 {
            figure.circle(points[0].0, points[0].1, 2.0, Some(color), None);
        } else {
            figure.line(&points, Stroke::solid(color, options.line_width));
        }
        let (cx, cy) = points[0];
        let group_name = match options.curves {
            Curves::Samples => shown
                .groups
                .get(*group)
                .map(|g| format!("\nGroup: {g}"))
                .unwrap_or_default(),
            Curves::Groups => String::new(),
        };
        figure.region(
            cx - 4.0,
            cy - 4.0,
            8.0,
            8.0,
            name,
            format!(
                "{name}{group_name}\n\nFeatures present: {}\nMost abundant: {:.3}%",
                values.len(),
                values[0]
            ),
        );
    }

    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&x_ticks, &x),
        Some("Rank"),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    let y_ticks_drawn: Vec<(f64, String)> = if options.log_scale {
        y_labels
            .iter()
            .map(|(v, t)| (y.map(*v), t.clone()))
            .collect()
    } else {
        scaled_ticks(&y_ticks, &y)
    };
    y_axis(
        &mut figure,
        &frame,
        &y_ticks_drawn,
        Some("Proportions (%)"),
        AXES_COLOR,
    );
    let mut used: Vec<usize> = curves.iter().map(|c| c.1).collect();
    used.sort_unstable();
    used.dedup();
    let entries: Vec<(String, Color)> = used
        .iter()
        .filter_map(|&g| Some((shown.groups.get(g)?.clone(), shown.group_color(g))))
        .collect();
    legend(&mut figure, &frame, &entries, options.legend);
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data() -> composition::Data {
        composition::Data {
            features: ["a", "b", "c"].map(String::from).to_vec(),
            samples: ["S1", "S2", "S3"].map(String::from).to_vec(),
            values: vec![
                vec![60.0, 10.0, 30.0],
                vec![30.0, 0.0, 30.0],
                vec![5.0, 80.0, 30.0],
            ],
            sample_groups: vec![0, 0, 1],
            groups: vec!["A".to_string(), "B".to_string()],
            group_colors: Vec::new(),
            sort_keys: Vec::new(),
            unclassified: Vec::new(),
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

    #[test]
    fn a_curve_for_every_sample_or_group() {
        let figure = plot(&data(), &Options::default());
        assert_eq!(figure.regions.len(), 3);
        // S2 has two of the three features.
        assert!(figure.regions[1].tooltip.contains("Features present: 2"));
        assert!(figure.regions[0].tooltip.contains("Group: A"));
        let shown = texts(&figure);
        for label in ["Rank", "Proportions (%)", "1", "10", "100", "A", "B"] {
            assert!(shown.contains(&label.to_string()), "{label}: {shown:?}");
        }
        let groups = plot(
            &data(),
            &Options {
                curves: Curves::Groups,
                log_scale: false,
                ..Options::default()
            },
        );
        assert_eq!(groups.regions.len(), 2);
        assert!(groups.regions[0].tooltip.starts_with('A'));
        assert!(!texts(&groups).contains(&"100".to_string()) || !groups.regions.is_empty());
    }

    #[test]
    fn log_ticks_cover_the_values() {
        let (ticks, labels) = log_ticks(0.05, 60.0);
        assert_eq!((ticks.min, ticks.max), (-2.0, 2.0));
        let names: Vec<&str> = labels.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(names, ["0.01", "0.1", "1", "10", "100"]);
        let (ticks, _) = log_ticks(10.0, 10.0);
        assert!(ticks.max > ticks.min);
    }

    #[test]
    fn nothing_to_show() {
        let mut empty = data();
        empty.values = vec![vec![0.0; 3]; 4];
        assert!(plot(&empty, &Options::default()).regions.is_empty());
    }
}
