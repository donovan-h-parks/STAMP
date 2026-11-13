//! Drives the whole interface without a window: loads example data, lets the
//! analyses finish, and checks what is drawn.
//!
//! Set `STAMP_SCREENSHOTS` to a directory to keep the rendered images.

use std::path::PathBuf;

use eframe::egui;
use stamp_gui::headless::{Image, Renderer};
use stamp_gui::plots::PlotKind;
use stamp_gui::session::Mode;
use stamp_gui::StampApp;

fn example(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(relative);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    stamp_core::text::decode(&bytes).into_owned()
}

struct Harness {
    renderer: Renderer,
    app: StampApp,
}

impl Harness {
    fn new() -> Harness {
        let renderer = Renderer::new(1280.0, 800.0, 1.0);
        let app = StampApp::new(&renderer.ctx);
        Harness { renderer, app }
    }

    fn enterotypes() -> Harness {
        let mut harness = Harness::new();
        harness.app.load_profile(
            "Enterotypes.profile.spf",
            &example("EnterotypesArumugam/Enterotypes.profile.spf"),
        );
        harness.app.load_metadata(
            "Enterotypes.metadata.tsv",
            &example("EnterotypesArumugam/Enterotypes.metadata.tsv"),
        );
        harness
    }

    /// Run frames until the analysis of the current mode has finished.
    fn settle(&mut self) {
        for _ in 0..2000 {
            let app = &mut self.app;
            self.renderer.run(2, |ui| app.draw(ui));
            if !self.app.session.is_running(self.app.mode) {
                let app = &mut self.app;
                self.renderer.run(3, |ui| app.draw(ui));
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the analysis did not finish");
    }

    fn screenshot(&mut self, name: &str) -> Image {
        let app = &mut self.app;
        let image = self.renderer.render(|ui| app.draw(ui));
        if let Ok(directory) = std::env::var("STAMP_SCREENSHOTS") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                PathBuf::from(directory).join(format!("{name}.png")),
                image.to_png(),
            )
            .unwrap();
        }
        image
    }
}

/// Fraction of pixels that are not the most common colour.
fn ink(image: &Image, x0: usize, y0: usize, x1: usize, y1: usize) -> f64 {
    let mut counts = std::collections::HashMap::new();
    let mut total = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            *counts.entry(image.pixel(x, y)).or_insert(0usize) += 1;
            total += 1;
        }
    }
    let background = counts.values().copied().max().unwrap_or(0);
    1.0 - background as f64 / total as f64
}

#[test]
fn empty_window_explains_how_to_start() {
    let mut harness = Harness::new();
    harness.settle();
    let image = harness.screenshot("empty");
    assert_eq!((image.width, image.height), (1280, 800));
    assert!(ink(&image, 0, 0, 1280, 800) > 0.005, "the window is blank");
    assert_eq!(harness.app.figure().shapes.len(), 1);
}

#[test]
fn two_group_analysis_draws_an_extended_error_bar_plot() {
    let mut harness = Harness::enterotypes();
    assert_eq!(harness.app.mode, Mode::TwoGroups);
    harness.settle();
    assert!(
        harness.app.session.error.is_none(),
        "{:?}",
        harness.app.session.error
    );
    assert_eq!(harness.app.session.feature_counts(Mode::TwoGroups).0, 17);
    assert_eq!(harness.app.session.multi.active_groups.len(), 5);
    assert_eq!(harness.app.plot_kind(), PlotKind::ExtendedErrorBar);
    assert_eq!(harness.app.figure().regions.len(), 17);

    let image = harness.screenshot("two_groups");
    // The plot occupies the centre of the window.
    assert!(ink(&image, 300, 150, 900, 500) > 0.03, "no plot was drawn");
}

#[test]
fn every_plot_of_every_mode_renders() {
    let mut harness = Harness::enterotypes();
    for mode in Mode::ALL {
        harness.app.set_mode(mode);
        harness.settle();
        for &kind in PlotKind::available(mode) {
            harness.app.set_plot_kind(kind);
            harness.settle();
            let name = format!("{}_{}", mode.name(), kind.name())
                .to_lowercase()
                .replace([' ', '-'], "_");
            let image = harness.screenshot(&name);
            assert!(
                harness.app.figure().shapes.len() > 5,
                "{mode:?} {kind:?} drew nothing"
            );
            assert!(
                ink(&image, 300, 120, 950, 700) > 0.01,
                "{mode:?} {kind:?} left the plot area blank"
            );
        }
    }
}

#[test]
fn results_table_and_feature_selection() {
    let mut harness = Harness::enterotypes();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Box);
    harness.app.show_results_table(true);
    harness.settle();
    let first = harness
        .app
        .session
        .selected_feature
        .clone()
        .expect("a feature is selected");

    let image = harness.screenshot("results_table");
    assert!(ink(&image, 0, 0, 1280, 800) > 0.05);

    // Selecting another feature redraws the single-feature plot.
    let before = harness.app.figure().clone();
    let other = harness
        .app
        .session
        .multi_rows()
        .iter()
        .map(|r| r.feature.clone())
        .find(|f| *f != first)
        .unwrap();
    harness.app.session.selected_feature = Some(other);
    harness.app.set_plot_kind(PlotKind::Box);
    harness.settle();
    assert_ne!(*harness.app.figure(), before);
}

#[test]
fn hovering_a_feature_shows_its_tooltip() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let plain = harness.screenshot("before_hover");
    // Point at the first row of the plot, then let the tooltip appear.
    harness.renderer.move_pointer(egui::pos2(520.0, 160.0));
    for _ in 0..90 {
        let app = &mut harness.app;
        harness.renderer.run(1, |ui| app.draw(ui));
    }
    let hovered = harness.screenshot("hover");
    assert_eq!(harness.app.hovered.as_deref(), Some("Unclassified"));
    let changed = (0..plain.pixels.len())
        .filter(|&i| plain.pixels[i] != hovered.pixels[i])
        .count();
    assert!(
        changed > 3000,
        "no tooltip appeared ({changed} pixels changed)"
    );
}

#[test]
fn problems_are_shown_to_the_user() {
    let mut harness = Harness::new();
    harness
        .app
        .load_profile("Autism.spf", &example("FinegoldAutism/Autism.spf"));
    harness.settle();
    let image = harness.screenshot("load_error");
    assert!(harness.app.session.tree.is_none());
    // The message box is drawn in the middle of the window.
    assert!(ink(&image, 300, 300, 980, 500) > 0.02);
}
