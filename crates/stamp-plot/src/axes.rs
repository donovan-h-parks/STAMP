//! Drawing of plot frames: axis lines, ticks, labels, legends and markers.

use crate::axis::{LinearScale, Ticks};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::text;

pub const INCH: f64 = 72.0;

/// Default size, in points, of the titles particular to a kind of plot (a
/// feature's name above its bar plot, say).
pub const TITLE_SIZE: f64 = 9.0;
/// Default size, in points, of other text particular to a kind of plot:
/// notes, statistics written on the plot, and labels inside it.
pub const NOTE_SIZE: f64 = 8.0;

/// Font sizes, in points, of the text that every plot shares. The sizes in
/// force while a figure is built are set with [`with_fonts`]; text that is
/// particular to one kind of plot is sized by that plot's options instead.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Fonts {
    /// Numbers and names at the ticks of an axis.
    pub tick: f64,
    /// The name of an axis.
    pub axis_label: f64,
    /// The text of a legend.
    pub legend: f64,
    /// The font of all plot text.
    pub family: crate::text::PlotFont,
}

impl Default for Fonts {
    fn default() -> Self {
        Fonts {
            tick: 8.0,
            axis_label: 9.0,
            legend: 9.0,
            family: crate::text::PlotFont::default(),
        }
    }
}

thread_local! {
    static FONTS: std::cell::Cell<Fonts> = std::cell::Cell::new(Fonts::default());
}

/// The shared font sizes in force.
pub fn fonts() -> Fonts {
    FONTS.with(std::cell::Cell::get)
}

/// Build something, a figure, with the shared font sizes `fonts`; the sizes
/// in force before are restored afterwards, even if building panics.
pub fn with_fonts<R>(fonts: Fonts, build: impl FnOnce() -> R) -> R {
    struct Restore(Fonts);
    impl Drop for Restore {
        fn drop(&mut self) {
            FONTS.with(|f| f.set(self.0));
        }
    }
    let _restore = Restore(FONTS.with(|f| f.replace(fonts)));
    build()
}

/// Text at the ticks of an axis.
pub fn tick_style() -> TextStyle {
    TextStyle::new(fonts().tick)
}

/// The name of an axis.
pub fn axis_label_style() -> TextStyle {
    TextStyle::new(fonts().axis_label)
}

/// The text of a legend.
pub fn legend_style() -> TextStyle {
    TextStyle::new(fonts().legend)
}

/// Height of a line of text of a size.
pub fn line_height(size: f64) -> f64 {
    text::ascent(size) + text::descent(size)
}
/// Default colour of axis lines and ticks.
pub const AXES_COLOR: Color = Color::rgb(0x7f, 0x7f, 0x7f);
pub const TICK_LENGTH: f64 = 4.0;
pub const TICK_PAD: f64 = 4.0;

/// Colours given to groups, in order. The first ten are STAMP v2's (which
/// drew every further group black); the next ten are the strong colours of
/// Tableau 10 without its grey, which the first ten already have, and a
/// yellow from ColorBrewer's Set3, so that twenty groups are told apart.
/// Groups beyond the twentieth take the colours again from the start
/// ([`group_color`]); plots with markers also give them other shapes.
pub const GROUP_COLORS: [Color; 20] = [
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
    Color::rgb(31, 119, 180),
    Color::rgb(255, 127, 14),
    Color::rgb(44, 160, 44),
    Color::rgb(214, 39, 40),
    Color::rgb(148, 103, 189),
    Color::rgb(140, 86, 75),
    Color::rgb(227, 119, 194),
    Color::rgb(188, 189, 34),
    Color::rgb(23, 190, 207),
    Color::rgb(255, 237, 111),
];

/// Colour of the group at `index`: the palette's colours in order, again
/// from the start after the twentieth.
pub fn group_color(index: usize) -> Color {
    GROUP_COLORS[index % GROUP_COLORS.len()]
}

/// A note for a plot whose groups (or categories) outnumber the palette,
/// so that some share a colour; `None` if every one has its own.
pub fn repeated_colors_note(count: usize) -> Option<String> {
    (count > GROUP_COLORS.len()).then(|| {
        format!(
            "{count} groups but {} colours: colours repeat after the {}th",
            GROUP_COLORS.len(),
            GROUP_COLORS.len()
        )
    })
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
    let sizes = fonts();
    let style = tick_style();
    let mut below = frame.bottom + TICK_PAD;
    let line_height = line_height(sizes.tick);
    let mut deepest: f64 = 0.0;
    for (x, tick_label) in ticks {
        if tick_label.is_empty() {
            continue;
        }
        let width = text::width(tick_label, sizes.tick);
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
            axis_label_style().align(HAlign::Center, VAlign::Top),
        );
        below += self::line_height(sizes.axis_label);
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
    let sizes = fonts();
    let mut widest: f64 = 0.0;
    for (y, tick_label) in ticks {
        figure.line(&[(frame.left, *y), (frame.left + TICK_LENGTH, *y)], stroke);
        figure.text(
            frame.left - TICK_PAD,
            *y,
            tick_label.clone(),
            tick_style().align(HAlign::Right, VAlign::Middle),
        );
        widest = widest.max(text::width(tick_label, sizes.tick));
    }
    let mut left = frame.left - TICK_PAD - widest;
    if let Some(label) = label {
        left -= 4.0;
        figure.text(
            left,
            (frame.top + frame.bottom) / 2.0,
            label,
            axis_label_style()
                .align(HAlign::Center, VAlign::Bottom)
                .rotated(90.0),
        );
        left -= line_height(sizes.axis_label);
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
    let sizes = fonts();
    let widest = ticks
        .labels()
        .iter()
        .map(|l| text::width(l, sizes.tick))
        .fold(0.0, f64::max);
    let label = if has_label {
        4.0 + line_height(sizes.axis_label)
    } else {
        0.0
    };
    TICK_PAD + widest + label
}

/// Height needed below a frame for horizontal x tick labels and an axis
/// label.
pub fn x_axis_height(has_label: bool) -> f64 {
    let sizes = fonts();
    let label = if has_label {
        3.0 + line_height(sizes.axis_label)
    } else {
        0.0
    };
    TICK_PAD + line_height(sizes.tick) + label
}

/// Room to leave below a frame for an x axis with horizontal tick labels
/// and an axis label: at least 0.4 inch, more for large fonts.
pub fn x_axis_room() -> f64 {
    (x_axis_height(true) + 2.0).max(0.4 * INCH)
}

/// Title centred above a frame, `size` points high.
pub fn title(figure: &mut Figure, frame: &Frame, text: &str, size: f64) {
    figure.text(
        (frame.left + frame.right) / 2.0,
        frame.top - 5.0,
        text,
        TextStyle::new(size).align(HAlign::Center, VAlign::Baseline),
    );
}

/// A note in the top-right corner of a frame, such as a p-value.
pub fn corner_note(figure: &mut Figure, frame: &Frame, text: &str, size: f64) {
    figure.text(
        frame.right,
        frame.top - 5.0,
        text,
        TextStyle::new(size).align(HAlign::Right, VAlign::Baseline),
    );
}

/// Where a legend goes within a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LegendPosition {
    None,
    /// Wherever the legend covers the least of what is drawn.
    Best,
    UpperRight,
    CentreRight,
    LowerRight,
    UpperLeft,
    CentreLeft,
    LowerLeft,
}

impl LegendPosition {
    /// Every choice, in the order STAMP v2 lists them.
    pub const ALL: [LegendPosition; 8] = [
        LegendPosition::Best,
        LegendPosition::UpperRight,
        LegendPosition::CentreRight,
        LegendPosition::LowerRight,
        LegendPosition::UpperLeft,
        LegendPosition::CentreLeft,
        LegendPosition::LowerLeft,
        LegendPosition::None,
    ];

    /// The fixed places, in the order "best" prefers them when they are
    /// equally clear.
    const PLACES: [LegendPosition; 6] = [
        LegendPosition::UpperRight,
        LegendPosition::UpperLeft,
        LegendPosition::LowerLeft,
        LegendPosition::LowerRight,
        LegendPosition::CentreRight,
        LegendPosition::CentreLeft,
    ];

    pub fn name(self) -> &'static str {
        match self {
            LegendPosition::None => "None",
            LegendPosition::Best => "Best",
            LegendPosition::UpperRight => "Upper right",
            LegendPosition::CentreRight => "Centre right",
            LegendPosition::LowerRight => "Lower right",
            LegendPosition::UpperLeft => "Upper left",
            LegendPosition::CentreLeft => "Centre left",
            LegendPosition::LowerLeft => "Lower left",
        }
    }

    /// Top-left corner of a legend of the given size at this place.
    fn origin(self, frame: &Frame, width: f64, height: f64) -> (f64, f64) {
        let margin = 8.0;
        let x = match self {
            LegendPosition::UpperLeft | LegendPosition::CentreLeft | LegendPosition::LowerLeft => {
                frame.left + margin
            }
            _ => frame.right - margin - width,
        };
        let y = match self {
            LegendPosition::UpperLeft | LegendPosition::UpperRight => frame.top + margin,
            LegendPosition::CentreLeft | LegendPosition::CentreRight => {
                (frame.top + frame.bottom - height) / 2.0
            }
            _ => frame.bottom - margin - height,
        };
        (x, y)
    }
}

/// True if the segment from `a` to `b` touches the box.
fn segment_meets_box(
    a: (f64, f64),
    b: (f64, f64),
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
) -> bool {
    // Liang-Barsky clipping: the part of the segment inside the box, if any.
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.0 - left),
        (dx, right - a.0),
        (-dy, a.1 - top),
        (dy, bottom - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else if p < 0.0 {
            t0 = t0.max(q / p);
        } else {
            t1 = t1.min(q / p);
        }
    }
    t0 <= t1
}

/// How much of what is already drawn a box would cover: the number of
/// shapes it touches. Text is ignored, as are shapes that fill the frame,
/// such as its background.
fn clutter(figure: &Figure, frame: &Frame, x: f64, y: f64, width: f64, height: f64) -> usize {
    use crate::figure::Shape;
    let (left, top, right, bottom) = (x, y, x + width, y + height);
    let overlaps = |l: f64, t: f64, r: f64, b: f64| l < right && r > left && t < bottom && b > top;
    let fills_frame = |l: f64, t: f64, r: f64, b: f64| {
        (r - l) >= 0.9 * frame.width() && (b - t) >= 0.9 * frame.height()
    };
    figure
        .shapes
        .iter()
        .filter(|shape| match shape {
            Shape::Line { points, .. } => points
                .windows(2)
                .any(|pair| segment_meets_box(pair[0], pair[1], left, top, right, bottom)),
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => {
                let (l, t, r, b) = (*x, *y, x + width, y + height);
                overlaps(l, t, r, b) && !fills_frame(l, t, r, b)
            }
            Shape::Circle { cx, cy, radius, .. } => {
                overlaps(cx - radius, cy - radius, cx + radius, cy + radius)
            }
            Shape::Polygon { points, .. } => {
                let fold = |pick: fn(&(f64, f64)) -> f64, start: f64, f: fn(f64, f64) -> f64| {
                    points.iter().map(pick).fold(start, f)
                };
                let (l, r) = (
                    fold(|p| p.0, f64::INFINITY, f64::min),
                    fold(|p| p.0, f64::NEG_INFINITY, f64::max),
                );
                let (t, b) = (
                    fold(|p| p.1, f64::INFINITY, f64::min),
                    fold(|p| p.1, f64::NEG_INFINITY, f64::max),
                );
                overlaps(l, t, r, b) && !fills_frame(l, t, r, b)
            }
            Shape::Area { rings, .. } => rings
                .iter()
                .flatten()
                .any(|&(x, y)| x > left && x < right && y > top && y < bottom),
            Shape::Text { .. } => false,
        })
        .count()
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
    Star,
    Cross,
    Plus,
    Pentagon,
}

impl Marker {
    /// Every shape, in the order they are given to categories: the six of
    /// [`Marker::for_group`] first.
    pub const ALL: [Marker; 10] = [
        Marker::Circle,
        Marker::Square,
        Marker::TriangleUp,
        Marker::Diamond,
        Marker::Hexagon,
        Marker::TriangleDown,
        Marker::Star,
        Marker::Cross,
        Marker::Plus,
        Marker::Pentagon,
    ];

    /// A distinct marker for each group, cycling after six.
    pub fn for_group(index: usize) -> Marker {
        Marker::ALL[index % 6]
    }
}

/// Draw a marker of the given area (in square points) with a black edge.
pub fn marker(figure: &mut Figure, kind: Marker, x: f64, y: f64, area: f64, fill: Color) {
    marker_styled(
        figure,
        kind,
        x,
        y,
        area,
        Some(fill),
        Stroke::solid(Color::BLACK, 0.75),
    );
}

/// Draw a marker of the given area (in square points), filled or hollow,
/// with an edge.
pub fn marker_styled(
    figure: &mut Figure,
    kind: Marker,
    x: f64,
    y: f64,
    area: f64,
    fill: Option<Color>,
    edge: Stroke,
) {
    let edge = Some(edge);
    let r = area.max(0.0).sqrt() / 2.0;
    let polygon = |figure: &mut Figure, points: Vec<(f64, f64)>| {
        figure.shapes.push(crate::figure::Shape::Polygon {
            points,
            fill,
            stroke: edge,
        });
    };
    // Points around the centre at radii `radii` in turn, starting straight
    // up, `count` in all.
    let around = |count: usize, radii: &[f64], turn: f64| -> Vec<(f64, f64)> {
        (0..count)
            .map(|i| {
                let angle = turn + std::f64::consts::TAU * i as f64 / count as f64;
                let radius = radii[i % radii.len()];
                (x + radius * angle.sin(), y - radius * angle.cos())
            })
            .collect()
    };
    match kind {
        Marker::Circle => figure.circle(x, y, r, fill, edge),
        Marker::Square => figure.rect(x - r, y - r, 2.0 * r, 2.0 * r, fill, edge),
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
        Marker::Star => polygon(figure, around(10, &[1.35 * r, 0.55 * r], 0.0)),
        Marker::Pentagon => polygon(figure, around(5, &[1.15 * r], 0.0)),
        Marker::Plus | Marker::Cross => {
            // A thick plus sign, turned by 45 degrees for the cross.
            let (a, b) = (1.25 * r, 0.42 * r);
            let arms = [
                (b, a),
                (b, b),
                (a, b),
                (a, -b),
                (b, -b),
                (b, -a),
                (-b, -a),
                (-b, -b),
                (-a, -b),
                (-a, b),
                (-b, b),
                (-b, a),
            ];
            let turn: f64 = if kind == Marker::Cross {
                std::f64::consts::FRAC_PI_4
            } else {
                0.0
            };
            let (sin, cos) = turn.sin_cos();
            let points = arms
                .iter()
                .map(|&(dx, dy)| (x + dx * cos - dy * sin, y + dx * sin + dy * cos))
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
    let size = fonts().legend;
    let (swatch_w, swatch_h, pad, line) = (14.0, 6.0, 5.0, size + 4.0);
    let widest = entries
        .iter()
        .map(|(name, _)| text::width(name, size))
        .fold(0.0, f64::max);
    let (width, height) = (swatch_w + pad + widest, line * entries.len() as f64);
    let place = match position {
        // The clearest place, taking the first of those equally clear.
        LegendPosition::Best => LegendPosition::PLACES
            .into_iter()
            .min_by_key(|place| {
                let (x, y) = place.origin(frame, width, height);
                clutter(figure, frame, x, y, width, height)
            })
            .unwrap_or(LegendPosition::UpperRight),
        fixed => fixed,
    };
    let (x, y) = place.origin(frame, width, height);
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
            legend_style().align(HAlign::Left, VAlign::Middle),
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
        // Twenty colours, the first ten STAMP v2's, then again from the
        // start.
        assert_eq!(group_color(9), Color::rgb(204, 235, 197));
        assert_eq!(group_color(10), Color::rgb(31, 119, 180));
        assert_eq!(group_color(20), group_color(0));
        assert_eq!(group_color(99), group_color(19));
        let mut distinct = GROUP_COLORS.to_vec();
        distinct.sort_by_key(|c| (c.r, c.g, c.b));
        distinct.dedup();
        assert_eq!(distinct.len(), 20);
        assert!(GROUP_COLORS.iter().all(|&c| c != Color::BLACK));
        assert_eq!(repeated_colors_note(20), None);
        assert!(repeated_colors_note(21).unwrap().contains("21 groups"));
        // Every shape draws one shape.
        let mut figure = Figure::new(50.0, 50.0);
        for kind in Marker::ALL {
            marker_styled(
                &mut figure,
                kind,
                25.0,
                25.0,
                30.0,
                None,
                Stroke::solid(Color::BLACK, 1.0),
            );
        }
        assert_eq!(figure.shapes.len(), Marker::ALL.len());
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
        for position in LegendPosition::ALL {
            let mut figure = Figure::new(200.0, 100.0);
            legend(&mut figure, &frame, &entries, position);
            if position == LegendPosition::None {
                continue;
            }
            assert_eq!(figure.shapes.len(), 4, "{}", position.name());
            for shape in &figure.shapes {
                if let crate::figure::Shape::Rect {
                    x,
                    y,
                    width,
                    height,
                    ..
                } = shape
                {
                    assert!(*x >= 0.0 && x + width <= 200.0 && *y >= 0.0 && y + height <= 100.0);
                }
            }
        }
        let mut figure = Figure::new(200.0, 100.0);
        legend(&mut figure, &frame, &entries, LegendPosition::None);
        assert!(figure.shapes.is_empty());
    }

    /// Where the first swatch of a legend was drawn.
    fn swatch(figure: &Figure, from: usize) -> (f64, f64) {
        match &figure.shapes[from] {
            crate::figure::Shape::Rect { x, y, .. } => (*x, *y),
            other => panic!("not a swatch: {other:?}"),
        }
    }

    #[test]
    fn best_legend_avoids_what_is_drawn() {
        let frame = Frame {
            left: 0.0,
            top: 0.0,
            right: 200.0,
            bottom: 100.0,
        };
        let entries = vec![("Lean".to_string(), group_color(0))];
        let fill = Some(Color::BLACK);

        // An empty frame: the upper right, as in matplotlib.
        let mut figure = Figure::new(200.0, 100.0);
        figure.rect(0.0, 0.0, 200.0, 100.0, Some(Color::WHITE), None);
        legend(&mut figure, &frame, &entries, LegendPosition::Best);
        let (x, y) = swatch(&figure, 1);
        assert!(x > 100.0 && y < 50.0, "({x}, {y})");

        // Bars in the upper right and upper left: the lower left.
        let mut figure = Figure::new(200.0, 100.0);
        figure.rect(150.0, 0.0, 50.0, 40.0, fill, None);
        figure.rect(0.0, 0.0, 50.0, 40.0, fill, None);
        legend(&mut figure, &frame, &entries, LegendPosition::Best);
        let (x, y) = swatch(&figure, 2);
        assert!(x < 100.0 && y > 50.0, "({x}, {y})");

        // A line through three corners and points in the fourth: a side.
        let mut figure = Figure::new(200.0, 100.0);
        figure.shapes.push(crate::figure::Shape::Line {
            points: vec![(20.0, 95.0), (20.0, 12.0), (190.0, 12.0)],
            stroke: Stroke::solid(Color::BLACK, 1.0),
        });
        figure.circle(180.0, 85.0, 3.0, fill, None);
        legend(&mut figure, &frame, &entries, LegendPosition::Best);
        let (x, y) = swatch(&figure, 2);
        assert!(x > 100.0 && (30.0..70.0).contains(&y), "({x}, {y})");
    }

    /// The size of each piece of text drawn with that content.
    fn size_of(figure: &Figure, wanted: &str) -> f64 {
        figure
            .shapes
            .iter()
            .find_map(|s| match s {
                crate::figure::Shape::Text { text, style, .. } if text == wanted => {
                    Some(style.size)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text {wanted}"))
    }

    fn axes_and_legend(figure: &mut Figure) {
        let frame = Frame {
            left: 60.0,
            top: 10.0,
            right: 300.0,
            bottom: 200.0,
        };
        let ticks = [(100.0, "1.5".to_string())];
        x_axis(
            figure,
            &frame,
            &ticks,
            Some("x label"),
            LabelRotation::Horizontal,
            AXES_COLOR,
        );
        y_axis(
            figure,
            &frame,
            &[(50.0, "7".to_string())],
            Some("y label"),
            AXES_COLOR,
        );
        legend(
            figure,
            &frame,
            &[("entry".to_string(), Color::BLACK)],
            LegendPosition::UpperRight,
        );
    }

    #[test]
    fn shared_text_is_sized_by_the_fonts_in_force() {
        // The defaults: ticks 8, axis labels and legends 9.
        let mut figure = Figure::new(320.0, 240.0);
        axes_and_legend(&mut figure);
        assert_eq!(size_of(&figure, "1.5"), 8.0);
        assert_eq!(size_of(&figure, "7"), 8.0);
        assert_eq!(size_of(&figure, "x label"), 9.0);
        assert_eq!(size_of(&figure, "y label"), 9.0);
        assert_eq!(size_of(&figure, "entry"), 9.0);

        let chosen = Fonts {
            tick: 11.0,
            axis_label: 14.0,
            legend: 6.5,
            ..Fonts::default()
        };
        let (mut figure, mut plain) = (Figure::new(320.0, 240.0), Figure::new(320.0, 240.0));
        let width = with_fonts(chosen, || {
            axes_and_legend(&mut figure);
            y_axis_width(&Ticks::covering(0.0, 100.0), true)
        });
        assert_eq!(size_of(&figure, "1.5"), 11.0);
        assert_eq!(size_of(&figure, "x label"), 14.0);
        assert_eq!(size_of(&figure, "y label"), 14.0);
        assert_eq!(size_of(&figure, "entry"), 6.5);
        // Larger text takes more room, and the sizes are restored after.
        assert!(width > y_axis_width(&Ticks::covering(0.0, 100.0), true));
        assert_eq!(fonts(), Fonts::default());
        axes_and_legend(&mut plain);
        assert_eq!(size_of(&plain, "1.5"), 8.0);
        assert!(x_axis_room() >= 0.4 * INCH);
        assert!(with_fonts(chosen, x_axis_room) > x_axis_room());
    }
}
