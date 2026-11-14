//! SVG export.

use std::fmt::Write;

use crate::figure::{Color, Figure, HAlign, Shape, Stroke};

/// Round to a precision that keeps files small without visible error.
fn n(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 {
        "0".to_string()
    } else {
        rounded.to_string()
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // Control characters are not allowed in XML.
            c if (c as u32) < 0x20 && c != '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn fill_attr(fill: Option<Color>) -> String {
    match fill {
        Some(color) if color.a == 255 => format!(" fill=\"{}\"", color.to_hex()),
        Some(color) => {
            format!(
                " fill=\"{}\" fill-opacity=\"{}\"",
                color.to_hex(),
                n(f64::from(color.a) / 255.0)
            )
        }
        None => " fill=\"none\"".to_string(),
    }
}

fn stroke_attr(stroke: Option<&Stroke>) -> String {
    let Some(stroke) = stroke else {
        return String::new();
    };
    let mut out = format!(
        " stroke=\"{}\" stroke-width=\"{}\"",
        stroke.color.to_hex(),
        n(stroke.width)
    );
    if stroke.color.a != 255 {
        let _ = write!(
            out,
            " stroke-opacity=\"{}\"",
            n(f64::from(stroke.color.a) / 255.0)
        );
    }
    if let Some((dash, gap)) = stroke.dash {
        let _ = write!(out, " stroke-dasharray=\"{} {}\"", n(dash), n(gap));
    }
    out
}

fn points_attr(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{},{}", n(*x), n(*y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render a figure as a standalone SVG document. Text stays as text, in the
/// figure's font, so it remains editable in vector-graphics programs.
pub fn to_svg(figure: &Figure) -> String {
    let (w, h) = (n(figure.width), n(figure.height));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}pt\" height=\"{h}pt\" viewBox=\"0 0 {w} {h}\" font-family=\"{}\">",
        figure.font.css_families()
    );
    let _ = writeln!(
        out,
        "<rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>",
        figure.background.to_hex()
    );

    for shape in &figure.shapes {
        match shape {
            Shape::Line { points, stroke } => {
                let _ = writeln!(
                    out,
                    "<polyline points=\"{}\" fill=\"none\"{}/>",
                    points_attr(points),
                    stroke_attr(Some(stroke))
                );
            }
            Shape::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
            } => {
                let _ = writeln!(
                    out,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}{}/>",
                    n(*x),
                    n(*y),
                    n(*width),
                    n(*height),
                    fill_attr(*fill),
                    stroke_attr(stroke.as_ref())
                );
            }
            Shape::Circle {
                cx,
                cy,
                radius,
                fill,
                stroke,
            } => {
                let _ = writeln!(
                    out,
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"{}{}/>",
                    n(*cx),
                    n(*cy),
                    n(*radius),
                    fill_attr(*fill),
                    stroke_attr(stroke.as_ref())
                );
            }
            Shape::Polygon {
                points,
                fill,
                stroke,
            } => {
                let _ = writeln!(
                    out,
                    "<polygon points=\"{}\"{}{}/>",
                    points_attr(points),
                    fill_attr(*fill),
                    stroke_attr(stroke.as_ref())
                );
            }
            Shape::Area { rings, fill, .. } => {
                let mut d = String::new();
                for ring in rings {
                    for (i, (x, y)) in ring.iter().enumerate() {
                        let _ = write!(d, "{}{},{}", if i == 0 { "M" } else { "L" }, n(*x), n(*y));
                    }
                    d.push('Z');
                }
                let _ = writeln!(
                    out,
                    "<path d=\"{d}\" fill-rule=\"evenodd\"{}/>",
                    fill_attr(Some(*fill))
                );
            }
            Shape::Text { x, y, text, style } => {
                let anchor = match style.h_align {
                    HAlign::Left => "start",
                    HAlign::Center => "middle",
                    HAlign::Right => "end",
                };
                // Rotation is about the anchor; the baseline offset applies
                // in the text's own (rotated) frame.
                let transform = if style.rotation != 0.0 {
                    format!(
                        " transform=\"rotate({} {} {})\"",
                        n(-style.rotation),
                        n(*x),
                        n(*y)
                    )
                } else {
                    String::new()
                };
                let _ = writeln!(
                    out,
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{anchor}\"{}{transform}>{}</text>",
                    n(*x),
                    n(*y + style.baseline_offset_in(figure.font)),
                    n(style.size),
                    fill_attr(Some(style.color)),
                    escape(text)
                );
            }
        }
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::{TextStyle, VAlign};

    #[test]
    fn writes_every_shape() {
        let mut figure = Figure::new(100.0, 50.0);
        figure.line(
            &[(0.0, 0.0), (10.0, 5.5)],
            Stroke::dashed(Color::BLACK, 1.0, 4.0, 2.0),
        );
        figure.rect(1.0, 2.0, 3.0, 4.0, Some(Color::rgb(230, 230, 230)), None);
        figure.circle(
            5.0,
            5.0,
            2.74,
            Some(Color::RED),
            Some(Stroke::solid(Color::BLACK, 0.5)),
        );
        figure.text(
            50.0,
            25.0,
            "a < b & \"c\"",
            TextStyle::new(8.0).align(HAlign::Right, VAlign::Baseline),
        );
        figure.text(90.0, 25.0, "p-value", TextStyle::new(8.0).rotated(90.0));
        figure.area(
            vec![vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)]],
            Color::rgb(0, 0, 255),
        );
        let svg = to_svg(&figure);
        assert!(svg.contains("<path d=\"M0,0L4,0L4,4Z\" fill-rule=\"evenodd\" fill=\"#0000ff\"/>"));

        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100pt\" height=\"50pt\" viewBox=\"0 0 100 50\""));
        assert!(svg.contains("<polyline points=\"0,0 10,5.5\" fill=\"none\" stroke=\"#000000\" stroke-width=\"1\" stroke-dasharray=\"4 2\"/>"));
        assert!(svg.contains("<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#e6e6e6\"/>"));
        assert!(svg.contains("<circle cx=\"5\" cy=\"5\" r=\"2.74\" fill=\"#ff0000\" stroke=\"#000000\" stroke-width=\"0.5\"/>"));
        assert!(svg
            .contains("text-anchor=\"end\" fill=\"#000000\">a &lt; b &amp; &quot;c&quot;</text>"));
        assert!(svg.contains("transform=\"rotate(-90 90 25)\">p-value</text>"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn transparent_colours() {
        let mut figure = Figure::new(10.0, 10.0);
        figure.rect(
            0.0,
            0.0,
            1.0,
            1.0,
            Some(Color {
                r: 0,
                g: 0,
                b: 255,
                a: 128,
            }),
            None,
        );
        assert!(to_svg(&figure).contains("fill=\"#0000ff\" fill-opacity=\"0.502\""));
    }

    #[test]
    fn the_font_of_a_figure_is_named_in_its_svg() {
        let svg = to_svg(&Figure::new(10.0, 10.0));
        assert!(svg.contains("font-family=\"Arial, Helvetica, Liberation Sans, sans-serif\""));
        let fonts = crate::axes::Fonts {
            family: crate::text::PlotFont::TimesNewRoman,
            ..crate::axes::Fonts::default()
        };
        let figure = crate::axes::with_fonts(fonts, || Figure::new(10.0, 10.0));
        assert_eq!(figure.font, crate::text::PlotFont::TimesNewRoman);
        assert!(to_svg(&figure)
            .contains("font-family=\"Times New Roman, Times, Liberation Serif, serif\""));
    }

    /// Vertical alignment uses the figure's font, whatever font is in force
    /// when the SVG is written.
    #[test]
    fn text_is_aligned_by_the_figure_font() {
        use crate::text::PlotFont;
        for font in PlotFont::ALL {
            let fonts = crate::axes::Fonts {
                family: font,
                ..crate::axes::Fonts::default()
            };
            let figure = crate::axes::with_fonts(fonts, || {
                let mut figure = Figure::new(100.0, 100.0);
                figure.text(
                    10.0,
                    50.0,
                    "Top",
                    TextStyle::new(10.0).align(HAlign::Left, VAlign::Top),
                );
                figure
            });
            let expected = 50.0 + font.ascent(10.0);
            let svg = to_svg(&figure);
            assert!(
                svg.contains(&format!("y=\"{}\"", n(expected))),
                "{font:?}: {svg}"
            );
        }
    }
}
