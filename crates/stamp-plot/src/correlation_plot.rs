//! A feature against a number known for each sample: one point per sample,
//! with the least-squares line through them.

use stamp_core::stats::correlation::least_squares;
use stamp_core::CorrelationProfile;

use crate::axes::{
    group_color, legend, marker, marker_legend, scaled_ticks, x_axis, y_axis, y_axis_width, Frame,
    LabelRotation, LegendPosition, Marker, AXES_COLOR, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};

/// One sample.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub sample: String,
    /// The value of the metadata field.
    pub value: f64,
    /// Relative frequency of the feature (%).
    pub percent: f64,
    /// Index of the sample's group, if the samples are grouped.
    pub group: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub feature: String,
    /// Name of the metadata field.
    pub field: String,
    pub points: Vec<Point>,
    /// Names of the groups the points are coloured by; empty for none.
    pub groups: Vec<String>,
    /// Colour of each group; the default palette where missing.
    pub group_colors: Vec<Color>,
    /// The correlation and its p-value, written on the plot.
    pub statistics: Option<String>,
    /// Name of the values on the vertical axis; `None` for the proportion
    /// of the feature, in percent.
    pub value_label: Option<String>,
}

impl Data {
    /// The samples of a profile for one feature, not grouped. `None` if
    /// the profile has no such feature.
    pub fn from_profile(profile: &CorrelationProfile, feature: &str) -> Option<Data> {
        let found = profile.profile.feature(feature)?;
        let percentages = profile.profile.percentages(found);
        Some(Data {
            feature: feature.to_string(),
            field: profile.field.clone(),
            points: profile
                .profile
                .sample_names
                .iter()
                .zip(&profile.values)
                .zip(percentages)
                .map(|((sample, &value), percent)| Point {
                    sample: sample.clone(),
                    value,
                    percent,
                    group: None,
                })
                .collect(),
            groups: Vec::new(),
            group_colors: Vec::new(),
            statistics: None,
            value_label: None,
        })
    }

    fn color_of(&self, group: usize) -> Color {
        self.group_colors
            .get(group)
            .copied()
            .unwrap_or_else(|| group_color(group))
    }
}

/// Appearance of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Area of the markers, in square points.
    pub marker_size: f64,
    /// Draw the least-squares line through the points.
    pub show_line: bool,
    /// Write the correlation and its p-value on the plot.
    pub show_statistics: bool,
    /// Give each group its own marker shape as well as its colour.
    pub unique_shapes: bool,
    pub legend: LegendPosition,
    /// Size of the title (the feature's name), in points.
    pub title_font_size: f64,
    /// Size of the correlation and p-value written above the plot.
    pub note_font_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 6.0,
            marker_size: 30.0,
            show_line: true,
            show_statistics: true,
            unique_shapes: true,
            legend: LegendPosition::Best,
            title_font_size: crate::axes::TITLE_SIZE,
            note_font_size: crate::axes::NOTE_SIZE,
        }
    }
}

/// Colour of the points when the samples are not grouped.
const UNGROUPED: Color = Color::rgb(128, 177, 211);
/// Colour of the fitted line.
const LINE: Color = Color::rgb(60, 60, 60);

pub fn plot(data: &Data, options: &Options) -> Figure {
    let points: Vec<&Point> = data
        .points
        .iter()
        .filter(|p| p.value.is_finite() && p.percent.is_finite())
        .collect();
    if points.is_empty() {
        return Figure::message("No samples have a number in this field");
    }
    let range = |values: &mut dyn Iterator<Item = f64>| {
        values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        })
    };
    let (x_low, x_high) = range(&mut points.iter().map(|p| p.value));
    let (_, y_high) = range(&mut points.iter().map(|p| p.percent));
    // A little room on either side, so that no point sits on the frame.
    let x_pad = if x_high > x_low {
        0.04 * (x_high - x_low)
    } else {
        0.5
    };
    let x_ticks = Ticks::covering(x_low - x_pad, x_high + x_pad);
    let y_ticks = Ticks::covering(0.0, if y_high > 0.0 { y_high * 1.05 } else { 1.0 });

    let statistics_shown = options.show_statistics && data.statistics.is_some();
    let statistics_line = if statistics_shown {
        options.note_font_size + 4.0
    } else {
        0.0
    };
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let frame = Frame {
        left: padding + y_axis_width(&y_ticks, true),
        // A line for the title, and one for the correlation beneath it.
        top: padding + options.title_font_size + 9.0 + statistics_line,
        right: figure.width - padding,
        bottom: figure.height - padding - crate::axes::x_axis_room(),
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for a scatter plot");
    }
    let (x, y) = (
        frame.x_scale(x_ticks.min, x_ticks.max),
        frame.y_scale(y_ticks.min, y_ticks.max),
    );

    // The line first, so that the points lie on top of it.
    let xs: Vec<f64> = points.iter().map(|p| p.value).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.percent).collect();
    let fitted = least_squares(&xs, &ys);
    if let (true, Some((slope, intercept))) = (options.show_line, fitted) {
        // Across the values of the samples, kept inside the frame.
        let (mut from, mut to) = (x_low, x_high);
        if slope != 0.0 {
            let at_bottom = (y_ticks.min - intercept) / slope;
            let at_top = (y_ticks.max - intercept) / slope;
            from = from.max(at_bottom.min(at_top));
            to = to.min(at_bottom.max(at_top));
        }
        if to > from {
            figure.line(
                &[
                    (x.map(from), y.map(intercept + slope * from)),
                    (x.map(to), y.map(intercept + slope * to)),
                ],
                Stroke::solid(LINE, 1.5),
            );
        }
    }

    let radius = options.marker_size.max(0.0).sqrt() / 2.0;
    for point in &points {
        let (cx, cy) = (x.map(point.value), y.map(point.percent));
        let (color, shape) = match point.group {
            Some(group) if options.unique_shapes => {
                (data.color_of(group), Marker::for_group(group))
            }
            Some(group) => (data.color_of(group), Marker::for_group(0)),
            None => (UNGROUPED, Marker::for_group(0)),
        };
        marker(&mut figure, shape, cx, cy, options.marker_size, color);
        let group = point
            .group
            .and_then(|g| data.groups.get(g))
            .map(|name| format!("\nGroup: {name}"))
            .unwrap_or_default();
        figure.region(
            cx - radius - 1.0,
            cy - radius - 1.0,
            2.0 * radius + 2.0,
            2.0 * radius + 2.0,
            &point.sample,
            match &data.value_label {
                None => format!(
                    "{}{group}\n\n{}: {}\n{}: {:.3}%",
                    point.sample, data.field, point.value, data.feature, point.percent
                ),
                Some(label) => format!(
                    "{}{group}\n\n{}: {:.4}\n{label}: {:.4}",
                    point.sample, data.field, point.value, point.percent
                ),
            },
        );
    }

    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&x_ticks, &x),
        Some(&data.field),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some(
            data.value_label
                .as_deref()
                .unwrap_or("Proportion of sequences (%)"),
        ),
        AXES_COLOR,
    );
    // The title above the line of the correlation, so that the two never
    // overlap.
    figure.text(
        (frame.left + frame.right) / 2.0,
        frame.top - 5.0 - statistics_line,
        data.feature.clone(),
        TextStyle::new(options.title_font_size).align(HAlign::Center, VAlign::Baseline),
    );
    if let (true, Some(statistics)) = (options.show_statistics, &data.statistics) {
        figure.text(
            frame.left,
            frame.top - 5.0,
            statistics.clone(),
            TextStyle::new(options.note_font_size).align(HAlign::Left, VAlign::Baseline),
        );
    }
    let used: Vec<usize> = {
        let mut groups: Vec<usize> = points.iter().filter_map(|p| p.group).collect();
        groups.sort_unstable();
        groups.dedup();
        groups
    };
    let entries: Vec<(String, Color)> = used
        .iter()
        .filter_map(|&g| Some((data.groups.get(g)?.clone(), data.color_of(g))))
        .collect();
    if options.unique_shapes {
        let markers: Vec<Marker> = used.iter().map(|&g| Marker::for_group(g)).collect();
        marker_legend(
            &mut figure,
            &frame,
            &entries,
            &markers,
            options.marker_size,
            options.legend,
        );
    } else {
        legend(&mut figure, &frame, &entries, options.legend);
    }
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data() -> Data {
        let point = |sample: &str, value: f64, percent: f64, group: usize| Point {
            sample: sample.to_string(),
            value,
            percent,
            group: Some(group),
        };
        Data {
            feature: "Bacteroides".to_string(),
            field: "Depth (m)".to_string(),
            points: vec![
                point("S1", 1.0, 10.0, 0),
                point("S2", 2.0, 14.0, 0),
                point("S3", 3.0, 13.0, 1),
                point("S4", 4.0, 19.0, 1),
                point("S5", 5.0, 22.0, 1),
            ],
            groups: vec!["Shallow".to_string(), "Deep".to_string()],
            group_colors: Vec::new(),
            statistics: Some("rho = 0.900, p = 0.037".to_string()),
            value_label: None,
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

    fn lines(figure: &Figure, color: Color) -> usize {
        figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Line { stroke, .. } if stroke.color == color))
            .count()
    }

    #[test]
    fn the_legend_shows_the_shape_of_each_group() {
        let key_of = |figure: &Figure, name: &str| {
            let at = figure
                .shapes
                .iter()
                .rposition(|s| matches!(s, Shape::Text { text, .. } if text == name))
                .unwrap();
            figure.shapes[at - 1].clone()
        };
        let with = |unique_shapes: bool| {
            plot(
                &data(),
                &Options {
                    unique_shapes,
                    legend: LegendPosition::UpperLeft,
                    ..Options::default()
                },
            )
        };
        let shaped = with(true);
        assert!(matches!(key_of(&shaped, "Shallow"), Shape::Circle { .. }));
        assert!(matches!(
            key_of(&shaped, "Deep"),
            Shape::Rect { width, height, .. } if width == height
        ));
        let swatches = with(false);
        assert!(matches!(
            key_of(&swatches, "Deep"),
            Shape::Rect { width, height, .. } if width > height
        ));
    }

    #[test]
    fn a_point_for_every_sample_and_a_line_through_them() {
        let figure = plot(&data(), &Options::default());
        assert_eq!(figure.regions.len(), 5);
        assert!(figure.regions[2].tooltip.contains("Group: Deep"));
        assert!(figure.regions[2].tooltip.contains("Depth (m): 3"));
        assert_eq!(figure.regions[2].item, "S3");
        let texts = texts(&figure);
        for expected in [
            "Bacteroides",
            "Depth (m)",
            "Proportion of sequences (%)",
            "rho = 0.900, p = 0.037",
            "Shallow",
            "Deep",
        ] {
            assert!(
                texts.contains(&expected.to_string()),
                "{expected}: {texts:?}"
            );
        }
        assert_eq!(lines(&figure, LINE), 1);

        let bare = plot(
            &data(),
            &Options {
                show_line: false,
                show_statistics: false,
                legend: LegendPosition::None,
                ..Options::default()
            },
        );
        assert_eq!(lines(&bare, LINE), 0);
        let texts = self::texts(&bare);
        assert!(!texts.iter().any(|t| t.starts_with("rho")));
        assert!(!texts.contains(&"Shallow".to_string()));
    }

    #[test]
    fn degenerate_data() {
        // Without groups there is no legend; a constant field has no line.
        let mut flat = data();
        for point in &mut flat.points {
            point.group = None;
            point.value = 2.0;
        }
        flat.groups.clear();
        let figure = plot(&flat, &Options::default());
        assert_eq!(figure.regions.len(), 5);
        assert_eq!(lines(&figure, LINE), 0);
        assert!(!texts(&figure).contains(&"Shallow".to_string()));

        let mut none = data();
        none.points.clear();
        assert!(plot(&none, &Options::default()).regions.is_empty());
        let tiny = plot(
            &data(),
            &Options {
                width: 0.5,
                height: 0.5,
                ..Options::default()
            },
        );
        assert!(tiny.regions.is_empty());
    }
}
