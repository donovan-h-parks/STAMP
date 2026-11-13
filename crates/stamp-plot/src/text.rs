//! Text measurement with the bundled font.

use std::sync::OnceLock;

/// The font used for all plot text.
pub const FONT_DATA: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");
/// Family name of the bundled font, with fallbacks for viewers that lack it.
pub const FONT_FAMILY: &str = "DejaVu Sans";
pub const FONT_FAMILY_WITH_FALLBACKS: &str = "DejaVu Sans, Verdana, Arial, sans-serif";

fn face() -> &'static ttf_parser::Face<'static> {
    static FACE: OnceLock<ttf_parser::Face<'static>> = OnceLock::new();
    FACE.get_or_init(|| ttf_parser::Face::parse(FONT_DATA, 0).expect("bundled font is valid"))
}

/// Width of a single line of text at the given font size, in the same unit as
/// the size.
pub fn width(text: &str, size: f64) -> f64 {
    let face = face();
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
pub fn cap_height(size: f64) -> f64 {
    let face = face();
    let height = face.capital_height().unwrap_or(1493);
    f64::from(height) / f64::from(face.units_per_em()) * size
}

/// Distance from the baseline to the top of the tallest letters.
pub fn ascent(size: f64) -> f64 {
    // Lower-case ascenders (as in "d") are slightly taller than capitals.
    cap_height(size) * 1.04
}

/// Distance from the baseline to the bottom of letters such as "p".
pub fn descent(size: f64) -> f64 {
    0.236 * size
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

    #[test]
    fn measures_text() {
        assert_eq!(width("", 8.0), 0.0);
        let one = width("0", 10.0);
        // DejaVu Sans digits are 0.636 em wide.
        assert!((one - 6.36).abs() < 0.01, "{one}");
        assert!((width("00", 10.0) - 2.0 * one).abs() < 1e-9);
        assert!(width("W", 8.0) > width("i", 8.0));
        assert!((width("abc", 16.0) - 2.0 * width("abc", 8.0)).abs() < 1e-9);
        // The glyphs the plots rely on are present.
        for c in ['≥', '−', '%', 'é'] {
            assert!(face().glyph_index(c).is_some(), "{c}");
        }
    }

    #[test]
    fn vertical_metrics() {
        assert!((cap_height(10.0) - 7.29).abs() < 0.01);
        assert!(ascent(10.0) > cap_height(10.0));
        assert!(descent(10.0) > 0.0);
    }

    #[test]
    fn truncates_long_labels() {
        assert_eq!(truncate("abcdefghij", 5), "abcde...");
        assert_eq!(truncate("abcdefgh", 5), "abcdefgh");
        assert_eq!(truncate("naïve café terrace", 6), "naïve ...");
    }
}
