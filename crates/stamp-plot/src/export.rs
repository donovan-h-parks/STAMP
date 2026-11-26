//! Export of figures to image files, as bytes.
//!
//! SVG and PDF are written directly from the figure ([`crate::svg`],
//! [`crate::pdf`]). PNG images are drawn from the SVG with resvg.

use crate::figure::Figure;
use crate::svg::to_svg;

/// Why an export failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportError(pub String);

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExportError {}

/// Image formats a figure can be exported to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Svg,
    Png,
    Pdf,
}

impl Format {
    /// The format for a file name, from its extension.
    pub fn from_file_name(name: &str) -> Option<Format> {
        let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
        match extension.as_str() {
            "svg" => Some(Format::Svg),
            "png" => Some(Format::Png),
            "pdf" => Some(Format::Pdf),
            _ => None,
        }
    }
}

/// Export a figure. `dpi` sets the resolution of PNG output and is ignored
/// for the vector formats.
pub fn export(figure: &Figure, format: Format, dpi: f64) -> Result<Vec<u8>, ExportError> {
    match format {
        Format::Svg => Ok(to_svg(figure).into_bytes()),
        Format::Png => to_png(figure, dpi),
        Format::Pdf => to_pdf(figure),
    }
}

/// Render a figure as a PNG image at the given resolution.
pub fn to_png(figure: &Figure, dpi: f64) -> Result<Vec<u8>, ExportError> {
    use resvg::{tiny_skia, usvg};

    if !(dpi.is_finite() && dpi > 0.0) {
        return Err(ExportError(format!("Invalid resolution: {dpi} dpi.")));
    }
    let mut options = usvg::Options::default();
    options
        .fontdb_mut()
        .load_font_data(figure.font.data().to_vec());
    options.font_family = figure.font.bundled_family().to_string();
    let tree = usvg::Tree::from_str(&to_svg(figure), &options)
        .map_err(|e| ExportError(format!("Could not render the plot: {e}")))?;

    let scale = dpi / 72.0;
    let width = (figure.width * scale).round().max(1.0);
    let height = (figure.height * scale).round().max(1.0);
    // Guard against images too large to allocate.
    if width * height > 400e6 {
        return Err(ExportError(format!(
            "The image would be {width} by {height} pixels, which is too large. Use a lower resolution or a vector format."
        )));
    }
    let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32)
        .ok_or_else(|| ExportError("Could not allocate the image.".to_string()))?;
    // The SVG is sized in points, which the renderer converts to its own
    // pixels; scale from that size to the requested one.
    let size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|e| ExportError(format!("Could not encode the image: {e}")))
}

/// Render a figure as a single-page PDF document (see [`crate::pdf`]).
pub fn to_pdf(figure: &Figure) -> Result<Vec<u8>, ExportError> {
    Ok(crate::pdf::to_pdf(figure))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::{Color, HAlign, TextStyle, VAlign};

    fn sample() -> Figure {
        let mut figure = Figure::new(72.0, 36.0);
        figure.rect(0.0, 0.0, 36.0, 36.0, Some(Color::RED), None);
        figure.text(
            54.0,
            18.0,
            "Ag",
            TextStyle::new(12.0).align(HAlign::Center, VAlign::Middle),
        );
        figure
    }

    #[test]
    fn formats_from_file_names() {
        assert_eq!(Format::from_file_name("plot.SVG"), Some(Format::Svg));
        assert_eq!(Format::from_file_name("a.b/plot.png"), Some(Format::Png));
        assert_eq!(Format::from_file_name("plot.pdf"), Some(Format::Pdf));
        assert_eq!(Format::from_file_name("plot.eps"), None);
        assert_eq!(Format::from_file_name("plot"), None);
    }

    #[test]
    fn png_has_the_requested_resolution_and_content() {
        use resvg::tiny_skia::Pixmap;

        let png = to_png(&sample(), 144.0).unwrap();
        let image = Pixmap::decode_png(&png).unwrap();
        assert_eq!((image.width(), image.height()), (144, 72));
        // Left half red, right half white with dark text pixels.
        let pixel = |x, y| image.pixel(x, y).unwrap();
        assert_eq!((pixel(10, 10).red(), pixel(10, 10).green()), (255, 0));
        assert_eq!(pixel(140, 5).green(), 255);
        let dark = (72..144)
            .flat_map(|x| (0..72).map(move |y| (x, y)))
            .filter(|&(x, y)| pixel(x, y).green() < 100)
            .count();
        assert!(dark > 50, "text was not drawn ({dark} dark pixels)");
        // The whole figure is scaled to the image: the red square ends at
        // the middle, not beyond it.
        assert_eq!((pixel(70, 60).red(), pixel(70, 60).green()), (255, 0));
        assert_eq!(pixel(74, 60).green(), 255);

        assert!(to_png(&sample(), 0.0).is_err());
        assert!(to_png(&Figure::new(72000.0, 72000.0), 600.0)
            .unwrap_err()
            .0
            .contains("too large"));
    }

    /// Every font is drawn, and draws differently: the bundled font is the
    /// one used, whatever is installed on the machine.
    #[test]
    fn png_text_is_drawn_in_the_figure_font() {
        use crate::text::PlotFont;
        let ink = |font: PlotFont| {
            let fonts = crate::axes::Fonts {
                family: font,
                ..crate::axes::Fonts::default()
            };
            let figure = crate::axes::with_fonts(fonts, sample);
            let png = to_png(&figure, 288.0).unwrap();
            let image = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
            image
                .pixels()
                .iter()
                .enumerate()
                .filter(|(i, p)| (i % 288) >= 144 && p.green() < 100)
                .map(|(i, _)| i)
                .collect::<Vec<usize>>()
        };
        let drawn: Vec<Vec<usize>> = PlotFont::ALL.into_iter().map(ink).collect();
        for (font, pixels) in PlotFont::ALL.iter().zip(&drawn) {
            assert!(pixels.len() > 100, "{font:?}: {} dark pixels", pixels.len());
        }
        for i in 0..drawn.len() {
            for j in i + 1..drawn.len() {
                assert_ne!(
                    drawn[i],
                    drawn[j],
                    "{:?} and {:?}",
                    PlotFont::ALL[i],
                    PlotFont::ALL[j]
                );
            }
        }
    }

    #[test]
    fn pdf_is_a_pdf() {
        let pdf = to_pdf(&sample()).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 500);
        // The page is the size of the figure, in points.
        let text = String::from_utf8_lossy(&pdf);
        assert!(
            text.contains("/MediaBox [0 0 72 36]"),
            "unexpected page size"
        );
    }

    #[test]
    fn svg_export() {
        let svg = export(&sample(), Format::Svg, 300.0).unwrap();
        assert!(String::from_utf8(svg).unwrap().contains(">Ag</text>"));
    }
}
