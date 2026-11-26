//! The fonts of plot text, bundled so that plots look and measure the same
//! everywhere, whatever fonts are installed.

use std::sync::OnceLock;

/// The fonts plot text can be set in. Each is bundled as a free font with
/// the same letter widths as the font it is named for, and SVG files name
/// that font first, so viewers that have it use it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum PlotFont {
    /// The usual font of scientific figures (Liberation Sans, with Arial's
    /// and Helvetica's letter widths).
    #[default]
    Arial,
    /// A serif font (Liberation Serif, with the widths of Times New Roman).
    TimesNewRoman,
    /// A fixed-width font (Liberation Mono, with the widths of Courier New).
    CourierNew,
    /// The font of STAMP v2's plots (matplotlib's default).
    DejaVuSans,
}

impl PlotFont {
    pub const ALL: [PlotFont; 4] = [
        PlotFont::Arial,
        PlotFont::TimesNewRoman,
        PlotFont::CourierNew,
        PlotFont::DejaVuSans,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PlotFont::Arial => "Arial",
            PlotFont::TimesNewRoman => "Times New Roman",
            PlotFont::CourierNew => "Courier New",
            PlotFont::DejaVuSans => "DejaVu Sans",
        }
    }

    /// The bundled font file.
    pub fn data(self) -> &'static [u8] {
        match self {
            PlotFont::Arial => include_bytes!("../fonts/LiberationSans-Regular.ttf"),
            PlotFont::TimesNewRoman => include_bytes!("../fonts/LiberationSerif-Regular.ttf"),
            PlotFont::CourierNew => include_bytes!("../fonts/LiberationMono-Regular.ttf"),
            PlotFont::DejaVuSans => include_bytes!("../fonts/DejaVuSans.ttf"),
        }
    }

    /// Family name of the bundled font.
    pub fn bundled_family(self) -> &'static str {
        match self {
            PlotFont::Arial => "Liberation Sans",
            PlotFont::TimesNewRoman => "Liberation Serif",
            PlotFont::CourierNew => "Liberation Mono",
            PlotFont::DejaVuSans => "DejaVu Sans",
        }
    }

    /// The font families an SVG file asks for, best first: the font itself,
    /// fonts with the same widths, the bundled font, and a generic family.
    pub fn css_families(self) -> &'static str {
        match self {
            PlotFont::Arial => "Arial, Helvetica, Liberation Sans, sans-serif",
            PlotFont::TimesNewRoman => "Times New Roman, Times, Liberation Serif, serif",
            PlotFont::CourierNew => "Courier New, Courier, Liberation Mono, monospace",
            PlotFont::DejaVuSans => "DejaVu Sans, Verdana, sans-serif",
        }
    }

    /// A short name without spaces, for registering the font.
    pub fn key(self) -> &'static str {
        match self {
            PlotFont::Arial => "arial",
            PlotFont::TimesNewRoman => "times-new-roman",
            PlotFont::CourierNew => "courier-new",
            PlotFont::DejaVuSans => "dejavu-sans",
        }
    }

    pub(crate) fn face(self) -> &'static ttf_parser::Face<'static> {
        static FACES: [OnceLock<ttf_parser::Face<'static>>; 4] = [
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
        ];
        let index = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        FACES[index]
            .get_or_init(|| ttf_parser::Face::parse(self.data(), 0).expect("bundled font is valid"))
    }
}

/// The face of the font in force (see [`crate::axes::with_fonts`]).
#[cfg(test)]
fn face() -> &'static ttf_parser::Face<'static> {
    crate::axes::fonts().family.face()
}

impl PlotFont {
    /// Width of a single line of text at the given font size, in the same
    /// unit as the size.
    pub fn width(self, text: &str, size: f64) -> f64 {
        let face = self.face();
        let units: u32 = text
            .chars()
            .map(|c| {
                let glyph = face.glyph_index(c).unwrap_or(ttf_parser::GlyphId(0));
                u32::from(face.glyph_hor_advance(glyph).unwrap_or(0))
            })
            .sum();
        f64::from(units) / f64::from(face.units_per_em()) * size
    }

    /// Height of capital letters above the baseline.
    pub fn cap_height(self, size: f64) -> f64 {
        let face = self.face();
        let height = face
            .capital_height()
            .unwrap_or((f32::from(face.units_per_em()) * 0.73) as i16);
        f64::from(height) / f64::from(face.units_per_em()) * size
    }

    /// Distance from the baseline to the top of the tallest letters.
    pub fn ascent(self, size: f64) -> f64 {
        // Lower-case ascenders (as in "d") are slightly taller than capitals.
        self.cap_height(size) * 1.04
    }

    /// Distance from the baseline to the bottom of letters such as "p": the
    /// font's descender.
    pub fn descent(self, size: f64) -> f64 {
        let face = self.face();
        -f64::from(face.descender()) / f64::from(face.units_per_em()) * size
    }

    /// The font's ascender line above the baseline: where a text layout
    /// (egui's, for one) puts the top of a line.
    pub fn line_ascender(self, size: f64) -> f64 {
        let face = self.face();
        f64::from(face.ascender()) / f64::from(face.units_per_em()) * size
    }
}

/// Width of a single line of text in the font in force, at the given font
/// size, in the same unit as the size.
pub fn width(text: &str, size: f64) -> f64 {
    crate::axes::fonts().family.width(text, size)
}

/// Height of capital letters above the baseline, in the font in force.
pub fn cap_height(size: f64) -> f64 {
    crate::axes::fonts().family.cap_height(size)
}

/// Distance from the baseline to the top of the tallest letters, in the font
/// in force.
pub fn ascent(size: f64) -> f64 {
    crate::axes::fonts().family.ascent(size)
}

/// Distance from the baseline to the bottom of letters such as "p", in the
/// font in force.
pub fn descent(size: f64) -> f64 {
    crate::axes::fonts().family.descent(size)
}

/// Shorten text to `max_chars` characters followed by an ellipsis, if it is
/// more than three characters longer than that.
pub fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars + 3 {
        let mut shortened: String = text.chars().take(max_chars).collect();
        shortened.push_str("...");
        shortened
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::axes::{with_fonts, Fonts};

    fn in_font<R>(font: PlotFont, measure: impl FnOnce() -> R) -> R {
        with_fonts(
            Fonts {
                family: font,
                ..Fonts::default()
            },
            measure,
        )
    }

    /// Digit widths, in ems, of the fonts each bundled font stands in for,
    /// from Adobe's standard metrics (Helvetica, Times-Roman and Courier
    /// AFM files: 556, 500 and 600 units of 1000), which Arial, Times New
    /// Roman and Courier New share; and DejaVu Sans's own (1303 of 2048).
    #[test]
    fn bundled_fonts_have_the_widths_of_the_fonts_they_stand_for() {
        for (font, digit) in [
            (PlotFont::Arial, 0.556),
            (PlotFont::TimesNewRoman, 0.5),
            (PlotFont::CourierNew, 0.6),
            (PlotFont::DejaVuSans, 1303.0 / 2048.0),
        ] {
            let one = in_font(font, || width("0", 10.0));
            assert!((one - 10.0 * digit).abs() < 0.005, "{font:?}: {one}");
        }
        // A fixed-width font: every letter as wide as the next.
        in_font(PlotFont::CourierNew, || {
            assert!((width("W", 10.0) - width("i", 10.0)).abs() < 1e-9);
        });
        // Arial is the default.
        assert_eq!(PlotFont::default(), PlotFont::Arial);
        assert!((width("0", 10.0) - 5.56).abs() < 0.005);
    }

    #[test]
    fn measures_text() {
        for font in PlotFont::ALL {
            in_font(font, || {
                assert_eq!(width("", 8.0), 0.0);
                let one = width("0", 10.0);
                assert!((width("00", 10.0) - 2.0 * one).abs() < 1e-9);
                assert!((width("abc", 16.0) - 2.0 * width("abc", 8.0)).abs() < 1e-9);
                // The glyphs the plots rely on are present.
                for c in ['≥', '−', '%', 'é', '²', '±'] {
                    assert!(face().glyph_index(c).is_some(), "{font:?}: {c}");
                }
            });
        }
        in_font(PlotFont::Arial, || {
            assert!(width("W", 8.0) > width("i", 8.0))
        });
    }

    #[test]
    fn vertical_metrics() {
        for font in PlotFont::ALL {
            in_font(font, || {
                let cap = cap_height(10.0);
                assert!((5.5..8.0).contains(&cap), "{font:?}: {cap}");
                assert!(ascent(10.0) > cap);
                assert!(descent(10.0) > 0.0);
            });
        }
        // DejaVu Sans capitals are 0.729 em high.
        in_font(PlotFont::DejaVuSans, || {
            assert!((cap_height(10.0) - 7.29).abs() < 0.01)
        });
    }

    #[test]
    fn fonts_have_distinct_names() {
        for font in PlotFont::ALL {
            assert!(font.css_families().starts_with(font.name()));
            assert!(font.css_families().contains(font.bundled_family()));
            assert!(!font.key().contains(' '));
        }
    }

    #[test]
    fn truncates_long_labels() {
        assert_eq!(truncate("abcdefghij", 5), "abcde...");
        assert_eq!(truncate("abcdefgh", 5), "abcdefgh");
        assert_eq!(truncate("naïve café terrace", 6), "naïve ...");
    }
}
