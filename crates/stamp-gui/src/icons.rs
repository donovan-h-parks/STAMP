//! Icons for menu items, from the Phosphor Icons font (MIT licence; see
//! `assets/Phosphor-LICENSE.txt`). Each constant is the character the font
//! draws as that icon, so an icon is written as part of a label's text.
//!
//! The font compiled in holds only these icons (6 KB instead of 480 KB for
//! the whole font). After adding
//! one, run `scripts/icons/subset.sh`, which cuts the full font in
//! `scripts/icons/` down to the characters named in this file; the test
//! below fails until then.

use eframe::egui;

const FONT_NAME: &str = "phosphor";

pub const FOLDER_OPEN: &str = "\u{E256}";
pub const FILE: &str = "\u{E230}";
pub const TAG: &str = "\u{E478}";
pub const FLASK: &str = "\u{E79E}";
pub const FLOPPY_DISK: &str = "\u{E248}";
pub const FILE_PNG: &str = "\u{EB18}";
pub const FILE_SVG: &str = "\u{ED08}";
pub const FILE_PDF: &str = "\u{E702}";
pub const DOWNLOAD: &str = "\u{E20C}";
pub const SIGN_OUT: &str = "\u{E42A}";
pub const SIDEBAR: &str = "\u{EC24}";
pub const SLIDERS: &str = "\u{E434}";
pub const LIST: &str = "\u{E2F2}";
pub const ROWS: &str = "\u{E5A2}";
pub const TABLE: &str = "\u{E476}";
pub const WINDOW_OUT: &str = "\u{E5DE}";
pub const MOON: &str = "\u{E330}";
pub const GEAR: &str = "\u{E270}";
pub const INFO: &str = "\u{E2CE}";
pub const TERMINAL: &str = "\u{EAE8}";
pub const CLIPBOARD_TEXT: &str = "\u{E198}";
pub const IMAGE: &str = "\u{E2CA}";
pub const PLUGS: &str = "\u{EB56}";
pub const PULSE: &str = "\u{E000}";
pub const ARROW_COUNTER_CLOCKWISE: &str = "\u{E038}";

/// Every icon, for checking that the font has them all.
pub const ALL: [&str; 25] = [
    FOLDER_OPEN,
    FILE,
    TAG,
    FLASK,
    FLOPPY_DISK,
    FILE_PNG,
    FILE_SVG,
    FILE_PDF,
    DOWNLOAD,
    SIGN_OUT,
    SIDEBAR,
    SLIDERS,
    LIST,
    ROWS,
    TABLE,
    WINDOW_OUT,
    MOON,
    GEAR,
    INFO,
    TERMINAL,
    CLIPBOARD_TEXT,
    IMAGE,
    PLUGS,
    PULSE,
    ARROW_COUNTER_CLOCKWISE,
];

/// The text of a menu item: an icon, then its label.
pub fn label(icon: &str, text: &str) -> String {
    format!("{icon}  {text}")
}

/// Add the icon font behind the text font, so that labels can mix the two.
pub fn add_to_fonts(fonts: &mut egui::FontDefinitions) {
    fonts.font_data.insert(
        FONT_NAME.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Phosphor.ttf"
        ))),
    );
    let proportional = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    // Second, so that it never replaces a character of the text font.
    proportional.insert(proportional.len().min(1), FONT_NAME.to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_in_the_font_and_text_keeps_its_own() {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        add_to_fonts(&mut fonts);
        // The text font stays first.
        let proportional = &fonts.families[&egui::FontFamily::Proportional];
        assert_eq!(proportional[1], FONT_NAME);
        ctx.set_fonts(fonts);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::FontId::proportional(14.0);
            ui.ctx().fonts_mut(|fonts| {
                for icon in ALL {
                    assert!(fonts.has_glyphs(&font, icon), "missing {icon:?}");
                }
                assert!(fonts.has_glyphs(&font, "Load profile file..."));
            });
        });
        assert_eq!(
            label(GEAR, "Preferences..."),
            format!("{GEAR}  Preferences...")
        );
    }
}
