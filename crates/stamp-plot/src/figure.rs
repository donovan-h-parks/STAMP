//! The backend-neutral description of a plot.
//!
//! Coordinates are in points (1/72 inch) from the top-left corner, with y
//! increasing downwards.

use crate::text;

/// An RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    /// A shade of grey from 0 (black) to 1 (white).
    pub fn grey(level: f64) -> Color {
        let v = (level.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color::rgb(v, v, v)
    }

    /// Parse `#rrggbb` or `rrggbb`.
    pub fn from_hex(hex: &str) -> Option<Color> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Color::rgb(channel(0)?, channel(2)?, channel(4)?))
    }

    /// `#rrggbb`
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// How a line is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
    /// Lengths of dashes and gaps; `None` for a solid line.
    pub dash: Option<(f64, f64)>,
}

impl Stroke {
    pub fn solid(color: Color, width: f64) -> Stroke {
        Stroke {
            color,
            width,
            dash: None,
        }
    }

    pub fn dashed(color: Color, width: f64, dash: f64, gap: f64) -> Stroke {
        Stroke {
            color,
            width,
            dash: Some((dash, gap)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    /// The anchor is at the top of capital letters.
    Top,
    /// The anchor is at the middle of capital letters.
    Middle,
    /// The anchor is on the baseline.
    Baseline,
    /// The anchor is below the descenders.
    Bottom,
}

/// Appearance and alignment of a piece of text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// Font size in points.
    pub size: f64,
    pub color: Color,
    pub h_align: HAlign,
    pub v_align: VAlign,
    /// Rotation about the anchor, in degrees counter-clockwise.
    pub rotation: f64,
}

impl TextStyle {
    pub fn new(size: f64) -> TextStyle {
        TextStyle {
            size,
            color: Color::BLACK,
            h_align: HAlign::Left,
            v_align: VAlign::Baseline,
            rotation: 0.0,
        }
    }

    pub fn align(mut self, h: HAlign, v: VAlign) -> TextStyle {
        self.h_align = h;
        self.v_align = v;
        self
    }

    pub fn color(mut self, color: Color) -> TextStyle {
        self.color = color;
        self
    }

    pub fn rotated(mut self, degrees: f64) -> TextStyle {
        self.rotation = degrees;
        self
    }

    /// Vertical distance from the anchor down to the baseline, before
    /// rotation.
    pub fn baseline_offset(&self) -> f64 {
        self.baseline_offset_in(crate::axes::fonts().family)
    }

    /// As [`TextStyle::baseline_offset`], for text set in `font`. A figure's
    /// text is set in [`Figure::font`], whatever font is in force when the
    /// figure is drawn or saved.
    pub fn baseline_offset_in(&self, font: text::PlotFont) -> f64 {
        match self.v_align {
            VAlign::Top => font.ascent(self.size),
            VAlign::Middle => font.cap_height(self.size) / 2.0,
            VAlign::Baseline => 0.0,
            VAlign::Bottom => -font.descent(self.size),
        }
    }
}

/// A drawing primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A connected series of line segments.
    Line {
        points: Vec<(f64, f64)>,
        stroke: Stroke,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    Circle {
        cx: f64,
        cy: f64,
        radius: f64,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    Polygon {
        points: Vec<(f64, f64)>,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        style: TextStyle,
    },
}

/// An area of the figure that belongs to one data item, for tooltips and
/// selection.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The feature (or other item) this area shows.
    pub item: String,
    /// Text to show when the area is pointed at.
    pub tooltip: String,
}

impl Region {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

/// A complete plot: shapes in drawing order on a page of fixed size.
#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    /// Width and height in points.
    pub width: f64,
    pub height: f64,
    pub background: Color,
    pub shapes: Vec<Shape>,
    pub regions: Vec<Region>,
    /// The font of all its text: the one in force when it was made.
    pub font: crate::text::PlotFont,
}

impl Figure {
    /// Replace colours throughout the figure. `map` is given each colour
    /// and whether it is the fill of a shape (as opposed to a line, an
    /// outline or text), and returns the colour to use instead.
    pub fn map_colors(&mut self, map: impl Fn(Color, bool) -> Color) {
        for shape in &mut self.shapes {
            match shape {
                Shape::Line { stroke, .. } => stroke.color = map(stroke.color, false),
                Shape::Rect { fill, stroke, .. }
                | Shape::Circle { fill, stroke, .. }
                | Shape::Polygon { fill, stroke, .. } => {
                    if let Some(fill) = fill {
                        *fill = map(*fill, true);
                    }
                    if let Some(stroke) = stroke {
                        stroke.color = map(stroke.color, false);
                    }
                }
                Shape::Text { style, .. } => style.color = map(style.color, false),
            }
        }
    }

    pub fn new(width: f64, height: f64) -> Figure {
        Figure {
            width,
            height,
            background: Color::WHITE,
            shapes: Vec::new(),
            regions: Vec::new(),
            font: crate::axes::fonts().family,
        }
    }

    /// A figure of the given size in inches.
    pub fn with_size_inches(width: f64, height: f64) -> Figure {
        Figure::new(width * 72.0, height * 72.0)
    }

    pub fn line(&mut self, points: &[(f64, f64)], stroke: Stroke) {
        self.shapes.push(Shape::Line {
            points: points.to_vec(),
            stroke,
        });
    }

    pub fn rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    ) {
        self.shapes.push(Shape::Rect {
            x,
            y,
            width,
            height,
            fill,
            stroke,
        });
    }

    pub fn circle(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    ) {
        self.shapes.push(Shape::Circle {
            cx,
            cy,
            radius,
            fill,
            stroke,
        });
    }

    pub fn text(&mut self, x: f64, y: f64, text: impl Into<String>, style: TextStyle) {
        self.shapes.push(Shape::Text {
            x,
            y,
            text: text.into(),
            style,
        });
    }

    pub fn region(&mut self, x: f64, y: f64, width: f64, height: f64, item: &str, tooltip: String) {
        self.regions.push(Region {
            x,
            y,
            width,
            height,
            item: item.to_string(),
            tooltip,
        });
    }

    /// The topmost region containing a point.
    pub fn region_at(&self, x: f64, y: f64) -> Option<&Region> {
        self.regions.iter().rev().find(|r| r.contains(x, y))
    }

    /// A small figure carrying only a message, for when there is nothing to
    /// plot.
    pub fn message(text: &str) -> Figure {
        let mut figure = Figure::with_size_inches(6.0, 4.0);
        let style = TextStyle::new(8.0).align(HAlign::Center, VAlign::Middle);
        figure.text(figure.width / 2.0, figure.height / 2.0, text, style);
        figure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours() {
        assert_eq!(Color::from_hex("#80b1d3"), Some(Color::rgb(128, 177, 211)));
        assert_eq!(Color::from_hex("FDB462"), Some(Color::rgb(253, 180, 98)));
        assert_eq!(Color::rgb(128, 177, 211).to_hex(), "#80b1d3");
        assert_eq!(Color::from_hex("#12"), None);
        assert_eq!(Color::from_hex("#zzzzzz"), None);
        assert_eq!(Color::grey(0.9), Color::rgb(230, 230, 230));
    }

    #[test]
    fn regions() {
        let mut figure = Figure::new(100.0, 50.0);
        figure.region(0.0, 0.0, 100.0, 50.0, "all", String::new());
        figure.region(10.0, 10.0, 20.0, 20.0, "inner", String::new());
        assert_eq!(figure.region_at(15.0, 15.0).unwrap().item, "inner");
        assert_eq!(figure.region_at(50.0, 40.0).unwrap().item, "all");
        assert!(figure.region_at(150.0, 40.0).is_none());
    }

    #[test]
    fn text_baselines() {
        let style = TextStyle::new(10.0);
        assert_eq!(style.baseline_offset(), 0.0);
        assert!(style.align(HAlign::Left, VAlign::Top).baseline_offset() > 7.0);
        assert!(style.align(HAlign::Left, VAlign::Bottom).baseline_offset() < 0.0);
    }
}
