//! Drawing of plot frames: axis lines, ticks, labels, legends and markers.

use crate::axis::{LinearScale, Ticks};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::text;

/// Font size of all plot text, in points.
pub const FONT_SIZE: f64 = 8.0;
pub const INCH: f64 = 72.0;
/// Default colour of axis lines and ticks.
pub const AXES_COLOR: Color = Color::rgb(0x7f, 0x7f, 0x7f);
pub const TICK_LENGTH: f64 = 4.0;
pub const TICK_PAD: f64 = 4.0;

/// Colours given to groups, in order; further groups are black.
pub const GROUP_COLORS: [Color; 10] = [
    Color::rgb(128, 177, 211),
    Color::rgb(253, 180, 98),
    Color::rgb(179, 222, 105),
    Color::rgb(190, 186, 218),
    Color::rgb(141, 211, 199),
    Color::rgb(251, 128, 114),
    Color::rgb(252, 205, 229),
    Color::rgb(127, 127, 127),
    Color::rgb(188, 128, 189),
    Color::rgb(204, 235, 197),
];

/// Colour of the group at `index`.
pub fn group_color(index: usize) -> Color {
    GROUP_COLORS.get(index).copied().unwrap_or(Color::BLACK)
}

pub fn label_style() -> TextStyle {
    TextStyle::new(FONT_SIZE)
}

/// The rectangle a plot's data is drawn in, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Frame {
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    pub fn x_scale(&self, min: f64, max: f64) -> LinearScale {
        LinearScale::new(min, max, self.left, self.right)
    }

    /// A y scale that grows upwards.
    pub fn y_scale(&self, min: f64, max: f64) -> LinearScale {
        LinearScale::new(min, max, self.bottom, self.top)
    }
}

/// How x tick labels are turned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRotation {
    Horizontal,
    /// Reading upwards, ending at the tick.
    Vertical,
    /// At 45 degrees, ending at the tick.
    Diagonal,
}

/// Bottom axis: line, ticks pointing into the plot, tick labels and an
/// optional axis label. Returns the y coordinate below everything drawn.
pub fn x_axis(
    figure: &mut Figure,
    frame: &Frame,
    ticks: &[(f64, String)],
    label: Option<&str>,
    rotation: LabelRotation,
    color: Color,
) -> f64 {
    let stroke = Stroke::solid(color, 1.0);
    figure.line(
        &[(frame.left, frame.bottom), (frame.right, frame.bottom)],
        stroke,
    );
    let style = label_style();
    let mut below = frame.bottom + TICK_PAD;
    let line_height = text::ascent(FONT_SIZE) + text::descent(FONT_SIZE);
    let mut deepest: f64 = 0.0;
    for (x, tick_label) in ticks {
        if tick_label.is_empty() {
            continue;
        }
        let width = text::width(tick_label, FONT_SIZE);
        match rotation {
            LabelRotation::Horizontal => {
                figure.line(
                    &[(*x, frame.bottom), (*x, frame.bottom - TICK_LENGTH)],
                    stroke,
                );
                figure.text(
                    *x,
                    below,
                    tick_label.clone(),
                    style.align(HAlign::Center, VAlign::Top),
                );
                deepest = deepest.max(line_height);
            }
            LabelRotation::Vertical => {
                figure.text(
                    *x,
                    below,
                    tick_label.clone(),
                    style.align(HAlign::Right, VAlign::Middle).rotated(90.0),
                );
                deepest = deepest.max(width);
            }
            LabelRotation::Diagonal => {
                figure.line(
                    &[(*x, frame.bottom), (*x, frame.bottom - TICK_LENGTH)],
                    stroke,
                );
                figure.text(
                    *x,
                    below,
                    tick_label.clone(),
                    style.align(HAlign::Right, VAlign::Top).rotated(45.0),
                );
                deepest = deepest.max((width + line_height) * std::f64::consts::FRAC_1_SQRT_2);
            }
        }
    }
    below += deepest;
    if let Some(label) = label {
        below += 3.0;
        figure.text(
            (frame.left + frame.right) / 2.0,
            below,
            label,
            style.align(HAlign::Center, VAlign::Top),
        );
        below += line_height;
    }
    below
}

/// Left axis: line, ticks pointing into the plot, tick labels and an optional
/// axis label reading upwards. Returns the x coordinate left of everything
/// drawn.
pub fn y_axis(
    figure: &mut Figure,
    frame: &Frame,
    ticks: &[(f64, String)],
    label: Option<&str>,
    color: Color,
) -> f64 {
    let stroke = Stroke::solid(color, 1.0);
    figure.line(
        &[(frame.left, frame.top), (frame.left, frame.bottom)],
        stroke,
    );
    let style = label_style();
    let mut widest: f64 = 0.0;
    for (y, tick_label) in ticks {
        figure.line(&[(frame.left, *y), (frame.left + TICK_LENGTH, *y)], stroke);
        figure.text(
            frame.left - TICK_PAD,
            *y,
            tick_label.clone(),
            style.align(HAlign::Right, VAlign::Middle),
        );
        widest = widest.max(text::width(tick_label, FONT_SIZE));
    }
    let mut left = frame.left - TICK_PAD - widest;
    if let Some(label) = label {
        left -= 4.0;
        figure.text(
            left,
            (frame.top + frame.bottom) / 2.0,
            label,
            style.align(HAlign::Center, VAlign::Bottom).rotated(90.0),
        );
        left -= text::ascent(FONT_SIZE) + text::descent(FONT_SIZE);
    }
    left
}

/// Tick positions in figure coordinates paired with their labels.
pub fn scaled_ticks(ticks: &Ticks, scale: &LinearScale) -> Vec<(f64, String)> {
    ticks
        .positions
        .iter()
        .map(|&p| scale.map(p))
        .zip(ticks.labels())
        .collect()
}

/// Width needed left of a frame for y tick labels and an axis label.
pub fn y_axis_width(ticks: &Ticks, has_label: bool) -> f64 {
    let widest = ticks
        .labels()
        .iter()
        .map(|l| text::width(l, FONT_SIZE))
        .fold(0.0, f64::max);
    let label = if has_label {
        4.0 + text::ascent(FONT_SIZE) + text::descent(FONT_SIZE)
    } else {
        0.0
    };
    TICK_PAD + widest + label
}

/// Title centred above a frame.
pub fn title(figure: &mut Figure, frame: &Frame, text: &str) {
    figure.text(
        (frame.left + frame.right) / 2.0,
        frame.top - 5.0,
        text,
        label_style().align(HAlign::Center, VAlign::Baseline),
    );
}

/// A note in the top-right corner of a frame, such as a p-value.
pub fn corner_note(figure: &mut Figure, frame: &Frame, text: &str) {
    figure.text(
        frame.right,
        frame.top - 5.0,
        text,
        label_style().align(HAlign::Right, VAlign::Baseline),
    );
}

/// Where a legend goes within a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendPosition {
    None,
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
}

/// Shapes of scatter-plot markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    Circle,
    Square,
    TriangleUp,
    TriangleDown,
    Diamond,
    Hexagon,
}

impl Marker {
    /// A distinct marker for each group, cycling after six.
    pub fn for_group(index: usize) -> Marker {
        [
            Marker::Circle,
            Marker::Square,
            Marker::TriangleUp,
            Marker::Diamond,
            Marker::Hexagon,
            Marker::TriangleDown,
        ][index % 6]
    }
}

/// Draw a marker of the given area (in square points) with a black edge.
pub fn marker(figure: &mut Figure, kind: Marker, x: f64, y: f64, area: f64, fill: Color) {
    let edge = Some(Stroke::solid(Color::BLACK, 0.75));
    let r = area.max(0.0).sqrt() / 2.0;
    let polygon = |figure: &mut Figure, points: Vec<(f64, f64)>| {
        figure.shapes.push(crate::figure::Shape::Polygon {
            points,
            fill: Some(fill),
            stroke: edge,
        });
    };
    match kind {
        Marker::Circle => figure.circle(x, y, r, Some(fill), edge),
        Marker::Square => figure.rect(x - r, y - r, 2.0 * r, 2.0 * r, Some(fill), edge),
        Marker::TriangleUp => polygon(figure, vec![(x, y - r), (x + r, y + r), (x - r, y + r)]),
        Marker::TriangleDown => polygon(figure, vec![(x, y + r), (x + r, y - r), (x - r, y - r)]),
        Marker::Diamond => polygon(
            figure,
            vec![
                (x, y - 1.3 * r),
                (x + 0.8 * r, y),
                (x, y + 1.3 * r),
                (x - 0.8 * r, y),
            ],
        ),
        Marker::Hexagon => {
            let points = (0..6)
                .map(|i| {
                    let angle =
                        std::f64::consts::FRAC_PI_3 * i as f64 + std::f64::consts::FRAC_PI_6;
                    (x + 1.1 * r * angle.cos(), y + 1.1 * r * angle.sin())
                })
                .collect();
            polygon(figure, points);
        }
    }
}

/// A vertical list of colour swatches with names, in a corner of a frame.
pub fn legend(
    figure: &mut Figure,
    frame: &Frame,
    entries: &[(String, Color)],
    position: LegendPosition,
) {
    if position == LegendPosition::None || entries.is_empty() {
        return;
    }
    let (swatch_w, swatch_h, pad, line) = (14.0, 6.0, 5.0, FONT_SIZE + 4.0);
    let widest = entries
        .iter()
        .map(|(name, _)| text::width(name, FONT_SIZE))
        .fold(0.0, f64::max);
    let (width, height) = (swatch_w + pad + widest, line * entries.len() as f64);
    let margin = 8.0;
    let x = match position {
        LegendPosition::UpperLeft | LegendPosition::LowerLeft => frame.left + margin,
        _ => frame.right - margin - width,
    };
    let y = match position {
        LegendPosition::UpperLeft | LegendPosition::UpperRight => frame.top + margin,
        _ => frame.bottom - margin - height,
    };
    for (i, (name, color)) in entries.iter().enumerate() {
        let centre = y + line * (i as f64 + 0.5);
        figure.rect(
            x,
            centre - swatch_h / 2.0,
            swatch_w,
            swatch_h,
            Some(*color),
            Some(Stroke::solid(Color::BLACK, 1.0)),
        );
        figure.text(
            x + swatch_w + pad,
            centre,
            name.clone(),
            label_style().align(HAlign::Left, VAlign::Middle),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_scales() {
        let frame = Frame {
            left: 10.0,
            top: 20.0,
            right: 110.0,
            bottom: 220.0,
        };
        assert_eq!((frame.width(), frame.height()), (100.0, 200.0));
        assert_eq!(frame.x_scale(0.0, 1.0).map(0.5), 60.0);
        assert_eq!(frame.y_scale(0.0, 1.0).map(1.0), 20.0);
    }

    #[test]
    fn axes_report_the_space_they_use() {
        let frame = Frame {
            left: 50.0,
            top: 10.0,
            right: 150.0,
            bottom: 110.0,
        };
        let mut figure = Figure::new(200.0, 200.0);
        let ticks = vec![(60.0, "a".to_string()), (140.0, "long label".to_string())];
        let plain = x_axis(
            &mut figure,
            &frame,
            &ticks,
            None,
            LabelRotation::Horizontal,
            AXES_COLOR,
        );
        let labelled = x_axis(
            &mut figure,
            &frame,
            &ticks,
            Some("x"),
            LabelRotation::Horizontal,
            AXES_COLOR,
        );
        let upright = x_axis(
            &mut figure,
            &frame,
            &ticks,
            None,
            LabelRotation::Vertical,
            AXES_COLOR,
        );
        assert!(plain > frame.bottom && labelled > plain && upright > plain);

        let left = y_axis(
            &mut figure,
            &frame,
            &[(50.0, "0.5".to_string())],
            Some("y"),
            AXES_COLOR,
        );
        assert!(left < frame.left - 15.0);
        let ticks = Ticks::covering(0.0, 1.0);
        assert!((frame.left - left - y_axis_width(&ticks, true)).abs() < 3.0);
    }

    #[test]
    fn markers_and_colours() {
        let mut figure = Figure::new(50.0, 50.0);
        for i in 0..7 {
            marker(
                &mut figure,
                Marker::for_group(i),
                25.0,
                25.0,
                30.0,
                group_color(i),
            );
        }
        assert_eq!(figure.shapes.len(), 7);
        assert_eq!(Marker::for_group(6), Marker::Circle);
        assert_eq!(group_color(0), Color::rgb(128, 177, 211));
        assert_eq!(group_color(99), Color::BLACK);
    }

    #[test]
    fn legend_fits_in_its_corner() {
        let frame = Frame {
            left: 0.0,
            top: 0.0,
            right: 200.0,
            bottom: 100.0,
        };
        let entries = vec![
            ("Lean".to_string(), group_color(0)),
            ("Obese".to_string(), group_color(1)),
        ];
        for position in [
            LegendPosition::UpperLeft,
            LegendPosition::UpperRight,
            LegendPosition::LowerLeft,
            LegendPosition::LowerRight,
        ] {
            let mut figure = Figure::new(200.0, 100.0);
            legend(&mut figure, &frame, &entries, position);
            assert_eq!(figure.shapes.len(), 4);
        }
        let mut figure = Figure::new(200.0, 100.0);
        legend(&mut figure, &frame, &entries, LegendPosition::None);
        assert!(figure.shapes.is_empty());
    }
}
