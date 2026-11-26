//! The colour schemes of the window, chosen in Preferences: egui's dark and
//! light appearance and the four Catppuccin flavours.
//!
//! The Catppuccin colours and the way they are given to egui follow the
//! `catppuccin-egui` crate (MIT licence, © 2023 Catppuccin), which does not
//! support the version of egui STAMP uses; only the colours it needs are
//! copied. Plots keep their own colours whatever the scheme.

use eframe::egui::{self, style, Color32};

/// A colour scheme of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ColorScheme {
    #[default]
    Dark,
    Light,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
}

impl ColorScheme {
    pub const ALL: [ColorScheme; 6] = [
        ColorScheme::Dark,
        ColorScheme::Light,
        ColorScheme::CatppuccinLatte,
        ColorScheme::CatppuccinFrappe,
        ColorScheme::CatppuccinMacchiato,
        ColorScheme::CatppuccinMocha,
    ];

    /// As listed in Preferences.
    pub fn name(self) -> &'static str {
        match self {
            ColorScheme::Dark => "Dark",
            ColorScheme::Light => "Light",
            ColorScheme::CatppuccinLatte => "Catppuccin Latte (light)",
            ColorScheme::CatppuccinFrappe => "Catppuccin Frappé",
            ColorScheme::CatppuccinMacchiato => "Catppuccin Macchiato",
            ColorScheme::CatppuccinMocha => "Catppuccin Mocha",
        }
    }

    /// Light text on a dark background.
    pub fn is_dark(self) -> bool {
        !matches!(self, ColorScheme::Light | ColorScheme::CatppuccinLatte)
    }

    /// Show the whole window in this scheme. egui keeps one appearance for
    /// dark and one for light; the scheme replaces the one it belongs to,
    /// and that one is shown.
    pub fn apply(self, ctx: &egui::Context) {
        let theme = if self.is_dark() {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        ctx.set_visuals_of(theme, self.visuals());
        ctx.set_theme(theme);
    }

    /// The appearance of this scheme.
    pub fn visuals(self) -> egui::Visuals {
        match self {
            ColorScheme::Dark => egui::Visuals::dark(),
            ColorScheme::Light => black_text(egui::Visuals::light()),
            ColorScheme::CatppuccinLatte => LATTE.visuals(egui::Visuals::light(), false),
            ColorScheme::CatppuccinFrappe => FRAPPE.visuals(egui::Visuals::dark(), true),
            ColorScheme::CatppuccinMacchiato => MACCHIATO.visuals(egui::Visuals::dark(), true),
            ColorScheme::CatppuccinMocha => MOCHA.visuals(egui::Visuals::dark(), true),
        }
    }
}

/// In the light scheme, the text of controls that can be used is black;
/// egui's own light appearance draws it in grey. Controls that are switched
/// off are still faded.
fn black_text(mut visuals: egui::Visuals) -> egui::Visuals {
    let widgets = &mut visuals.widgets;
    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.fg_stroke.color = Color32::BLACK;
    }
    visuals
}

/// The Catppuccin colours STAMP uses, named as in Catppuccin's palette.
struct Palette {
    rosewater: Color32,
    maroon: Color32,
    peach: Color32,
    blue: Color32,
    text: Color32,
    overlay1: Color32,
    surface2: Color32,
    surface1: Color32,
    surface0: Color32,
    base: Color32,
    mantle: Color32,
    crust: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

const LATTE: Palette = Palette {
    rosewater: rgb(220, 138, 120),
    maroon: rgb(230, 69, 83),
    peach: rgb(254, 100, 11),
    blue: rgb(30, 102, 245),
    text: rgb(76, 79, 105),
    overlay1: rgb(140, 143, 161),
    surface2: rgb(172, 176, 190),
    surface1: rgb(188, 192, 204),
    surface0: rgb(204, 208, 218),
    base: rgb(239, 241, 245),
    mantle: rgb(230, 233, 239),
    crust: rgb(220, 224, 232),
};

const FRAPPE: Palette = Palette {
    rosewater: rgb(242, 213, 207),
    maroon: rgb(234, 153, 156),
    peach: rgb(239, 159, 118),
    blue: rgb(140, 170, 238),
    text: rgb(198, 208, 245),
    overlay1: rgb(131, 139, 167),
    surface2: rgb(98, 104, 128),
    surface1: rgb(81, 87, 109),
    surface0: rgb(65, 69, 89),
    base: rgb(48, 52, 70),
    mantle: rgb(41, 44, 60),
    crust: rgb(35, 38, 52),
};

const MACCHIATO: Palette = Palette {
    rosewater: rgb(244, 219, 214),
    maroon: rgb(238, 153, 160),
    peach: rgb(245, 169, 127),
    blue: rgb(138, 173, 244),
    text: rgb(202, 211, 245),
    overlay1: rgb(128, 135, 162),
    surface2: rgb(91, 96, 120),
    surface1: rgb(73, 77, 100),
    surface0: rgb(54, 58, 79),
    base: rgb(36, 39, 58),
    mantle: rgb(30, 32, 48),
    crust: rgb(24, 25, 38),
};

const MOCHA: Palette = Palette {
    rosewater: rgb(245, 224, 220),
    maroon: rgb(235, 160, 172),
    peach: rgb(250, 179, 135),
    blue: rgb(137, 180, 250),
    text: rgb(205, 214, 244),
    overlay1: rgb(127, 132, 156),
    surface2: rgb(88, 91, 112),
    surface1: rgb(69, 71, 90),
    surface0: rgb(49, 50, 68),
    base: rgb(30, 30, 46),
    mantle: rgb(24, 24, 37),
    crust: rgb(17, 17, 27),
};

impl Palette {
    /// egui's appearance in these colours, starting from `old` (egui's
    /// dark or light appearance) for everything that is not a colour.
    fn visuals(&self, old: egui::Visuals, dark: bool) -> egui::Visuals {
        let shadow = Color32::from_black_alpha(if dark { 96 } else { 25 });
        let widget = |old: style::WidgetVisuals, fill: Color32| style::WidgetVisuals {
            bg_fill: fill,
            weak_bg_fill: fill,
            bg_stroke: egui::Stroke {
                color: self.overlay1,
                ..old.bg_stroke
            },
            fg_stroke: egui::Stroke {
                color: self.text,
                ..old.fg_stroke
            },
            ..old
        };
        egui::Visuals {
            dark_mode: dark,
            hyperlink_color: self.rosewater,
            faint_bg_color: self.surface0,
            extreme_bg_color: self.crust,
            code_bg_color: self.mantle,
            warn_fg_color: self.peach,
            error_fg_color: self.maroon,
            window_fill: self.base,
            panel_fill: self.base,
            window_stroke: egui::Stroke {
                color: self.overlay1,
                ..old.window_stroke
            },
            widgets: style::Widgets {
                noninteractive: widget(old.widgets.noninteractive, self.base),
                inactive: widget(old.widgets.inactive, self.surface0),
                hovered: widget(old.widgets.hovered, self.surface2),
                active: widget(old.widgets.active, self.surface1),
                open: widget(old.widgets.open, self.surface0),
            },
            selection: style::Selection {
                bg_fill: self.blue.linear_multiply(if dark { 0.2 } else { 0.4 }),
                stroke: egui::Stroke {
                    color: self.text,
                    ..old.selection.stroke
                },
            },
            window_shadow: egui::epaint::Shadow {
                color: shadow,
                ..old.window_shadow
            },
            popup_shadow: egui::epaint::Shadow {
                color: shadow,
                ..old.popup_shadow
            },
            ..old
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_scheme_is_shown_in_its_own_colours() {
        let ctx = egui::Context::default();
        for scheme in ColorScheme::ALL {
            scheme.apply(&ctx);
            assert_eq!(ctx.theme() == egui::Theme::Dark, scheme.is_dark());
            let visuals = ctx.style_of(ctx.theme()).visuals.clone();
            assert_eq!(visuals.dark_mode, scheme.is_dark(), "{scheme:?}");
            assert_eq!(
                visuals.panel_fill,
                scheme.visuals().panel_fill,
                "{scheme:?}"
            );
        }
        // Going back to egui's own appearance undoes a Catppuccin one.
        ColorScheme::Dark.apply(&ctx);
        assert_eq!(
            ctx.style_of(ctx.theme()).visuals.panel_fill,
            egui::Visuals::dark().panel_fill
        );
        ColorScheme::Light.apply(&ctx);
        assert_eq!(
            ctx.style_of(ctx.theme())
                .visuals
                .widgets
                .inactive
                .fg_stroke
                .color,
            Color32::BLACK
        );
    }

    #[test]
    fn schemes_have_different_backgrounds() {
        let fills: Vec<Color32> = ColorScheme::ALL
            .iter()
            .map(|scheme| scheme.visuals().panel_fill)
            .collect();
        for (i, a) in fills.iter().enumerate() {
            for b in &fills[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
