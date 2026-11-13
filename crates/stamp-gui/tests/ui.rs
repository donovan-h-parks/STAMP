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
    // The plot has the room beside the panels to itself.
    harness.app.show_feature_table(false);
    harness.settle();
    let plain = harness.screenshot("before_hover");
    // Point at the first row of the plot, wherever the plot has been placed.
    let (plot, _) = harness.app.plot_placement.expect("the plot was drawn");
    let row = &harness.app.figure().regions[0];
    let zoom = harness.app.zoom();
    let pointer = plot.min
        + egui::vec2(
            (row.x + row.width * 0.75) as f32,
            (row.y + row.height / 2.0) as f32,
        ) * zoom;
    harness.renderer.move_pointer(pointer);
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

#[test]
fn built_in_example_and_opened_files_are_loaded() {
    use stamp_gui::platform::{Event, OpenedFile};

    let mut harness = Harness::new();
    harness.app.load_example();
    harness.settle();
    assert_eq!(harness.app.session.profile_name, "Enterotypes.profile.spf");
    assert_eq!(harness.app.session.feature_counts(Mode::TwoGroups).0, 17);

    // Files arrive from the file dialogs as events, on the desktop and in
    // the browser alike.
    let file = |name: &str, text: &str| OpenedFile {
        name: name.to_string(),
        bytes: text.as_bytes().to_vec(),
    };
    harness.app.deliver(Event::Profile(file(
        "small.spf",
        "F\tA\tB\nx\t1\t2\ny\t3\t4\n",
    )));
    harness.settle();
    assert_eq!(harness.app.session.profile_name, "small.spf");
    assert!(harness.app.session.metadata.is_none());
    assert_eq!(harness.app.session.feature_counts(Mode::TwoSamples).0, 2);

    harness.app.deliver(Event::Converted {
        name: "converted.spf".to_string(),
        spf: "F\tA\tB\nx\t1\t2\n".to_string(),
    });
    harness.settle();
    assert_eq!(harness.app.session.profile_name, "converted.spf");

    harness
        .app
        .deliver(Event::Failed("The conversion failed.".to_string()));
    harness.settle();
    assert_eq!(harness.app.notice(), Some("The conversion failed."));
    harness
        .app
        .deliver(Event::Metadata(file("bad.tsv", "Id\tG\nnobody\tx\n")));
    harness.settle();
    assert!(harness
        .app
        .notice()
        .unwrap()
        .contains("Could not load bad.tsv"));
}

#[test]
fn plot_is_centred_in_its_space() {
    let mut harness = Harness::enterotypes();
    // The plot has the room beside the panels to itself.
    harness.app.show_feature_table(false);
    harness.settle();
    let (plot, space) = harness.app.plot_placement.expect("the plot was drawn");
    // Smaller than its space: equal margins on both sides, in both directions.
    assert!(plot.width() < space.width() && plot.height() < space.height());
    assert!(
        (plot.center().x - space.center().x).abs() <= 1.0,
        "{plot:?} in {space:?}"
    );
    assert!(
        (plot.center().y - space.center().y).abs() <= 1.0,
        "{plot:?} in {space:?}"
    );
    let image = harness.screenshot("centred");
    // The page is white; the margin left of it is not.
    let y = plot.center().y as usize;
    assert_eq!(
        image.pixel(plot.left() as usize + 3, y),
        egui::Color32::WHITE
    );
    assert_ne!(
        image.pixel(plot.left() as usize - 6, y),
        egui::Color32::WHITE
    );

    // Larger than its space: it starts at the top-left and scrolls.
    harness.app.set_zoom(4.0);
    harness.settle();
    let (plot, space) = harness.app.plot_placement.unwrap();
    assert!(
        (plot.min - space.min).length() <= 1.0 && (plot.max - space.max).length() <= 1.0,
        "{plot:?} in {space:?}"
    );

    // The reset returns to the starting zoom, and the centred layout.
    let start = Harness::enterotypes().app.zoom();
    harness.app.reset_zoom();
    harness.settle();
    assert_eq!(harness.app.zoom(), start);
    let (plot, space) = harness.app.plot_placement.unwrap();
    assert!((plot.center().x - space.center().x).abs() <= 1.0);
    harness.screenshot("after_reset");
}

#[test]
fn light_and_dark_modes() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let brightness = |image: &Image| {
        // A patch of the left panel, away from any text.
        let pixel = image.pixel(140, 600);
        u32::from(pixel.r()) + u32::from(pixel.g()) + u32::from(pixel.b())
    };

    stamp_gui::app::set_dark_mode(&harness.renderer.ctx, true);
    harness.settle();
    let dark = harness.screenshot("dark_mode");
    stamp_gui::app::set_dark_mode(&harness.renderer.ctx, false);
    harness.settle();
    let light = harness.screenshot("light_mode");
    assert!(brightness(&dark) < 200, "dark mode panel is bright");
    assert!(brightness(&light) > 600, "light mode panel is dark");
    // The plot's page stays white in both.
    let (plot, _) = harness.app.plot_placement.unwrap();
    let corner = (plot.left() as usize + 3, plot.top() as usize + 3);
    assert_eq!(dark.pixel(corner.0, corner.1), egui::Color32::WHITE);
    assert_eq!(light.pixel(corner.0, corner.1), egui::Color32::WHITE);
}

#[test]
fn group_colours_can_be_changed() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let pink = stamp_plot::Color::rgb(255, 0, 200);
    let uses_pink = |app: &StampApp| {
        app.figure()
            .shapes
            .iter()
            .any(|s| matches!(s, stamp_plot::Shape::Rect { fill: Some(c), .. } if *c == pink))
    };
    assert!(!uses_pink(&harness.app));
    harness.app.session.set_group_color("Enterotype 1", pink);
    harness.settle();
    assert!(
        uses_pink(&harness.app),
        "the plot did not pick up the new colour"
    );

    // The swatch beside the group in the left panel shows it too.
    let image = harness.screenshot("group_colour");
    let pink_pixels = (0..260)
        .flat_map(|x| (180..420).map(move |y| (x, y)))
        .filter(|&(x, y)| image.pixel(x, y) == egui::Color32::from_rgb(255, 0, 200))
        .count();
    assert!(
        pink_pixels > 30,
        "no pink swatch in the group list ({pink_pixels} pixels)"
    );
}

#[test]
fn results_table_holds_what_would_be_downloaded() {
    let mut harness = Harness::enterotypes();
    harness.app.show_results_table(true);
    harness.settle();
    let table = harness.app.results_table();
    assert_eq!(table.rows.len(), 17);
    let tsv = table.to_tsv();
    assert!(tsv.starts_with("Phyla\tEnterotype 1: mean rel. freq. (%)"));
    assert_eq!(tsv.lines().count(), 18);
    harness.screenshot("table_download");
}

#[test]
fn feature_table_lists_and_selects_features() {
    let mut harness = Harness::enterotypes();
    harness.app.set_plot_kind(PlotKind::Box);
    harness.settle();

    // Every feature, in alphabetical order, with its key statistics.
    let names: Vec<String> = harness
        .app
        .feature_table()
        .iter()
        .map(|r| r.feature.clone())
        .collect();
    assert_eq!(names.len(), 17);
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    let table = harness.app.feature_table();
    let row = table.iter().find(|r| r.feature == "Bacteroidetes").unwrap();
    assert_eq!(
        (row.p_value.as_str(), row.p_value_corrected.as_str()),
        ("0.320", "0.320")
    );
    assert!(row.effect_size.parse::<f64>().unwrap() > 5.0);
    assert!(row.active);
    let image = harness.screenshot("feature_table");
    assert!(ink(&image, 0, 0, 1280, 800) > 0.05);

    // Selecting a feature in the table redraws the plot for it.
    let before = harness.app.figure().clone();
    harness.app.select_feature("Firmicutes");
    harness.settle();
    assert_ne!(*harness.app.figure(), before);
    let title = harness
        .app
        .figure()
        .shapes
        .iter()
        .any(|s| matches!(s, stamp_plot::Shape::Text { text, .. } if text == "Firmicutes"));
    assert!(title, "the box plot does not show the selected feature");

    // "Show only active features" follows the filters.
    harness.app.session.groups.filter.significance = Some(0.05);
    harness.app.set_plot_kind(PlotKind::Box);
    harness.app.set_show_only_active_features(true);
    harness.settle();
    let active: Vec<String> = harness
        .app
        .feature_table()
        .iter()
        .map(|r| r.feature.clone())
        .collect();
    assert_eq!(active, ["Unclassified"]);
    harness.app.set_show_only_active_features(false);
    assert_eq!(harness.app.feature_table().len(), 17);
}

#[test]
fn side_panels_can_be_hidden() {
    let mut harness = Harness::enterotypes();
    // The plot has the room beside the panels to itself.
    harness.app.show_feature_table(false);
    harness.settle();
    let (_, with_panels) = harness.app.plot_placement.unwrap();

    harness.app.show_side_panels(false, true);
    harness.settle();
    let (_, without_left) = harness.app.plot_placement.unwrap();
    assert!(without_left.left() < with_panels.left() - 100.0);
    assert!((without_left.right() - with_panels.right()).abs() < 2.0);

    harness.app.show_side_panels(false, false);
    harness.settle();
    let (plot, whole) = harness.app.plot_placement.unwrap();
    assert!(whole.width() > 1200.0, "{whole:?}");
    assert!((plot.center().x - whole.center().x).abs() <= 1.0);
    harness.screenshot("no_side_panels");
}

#[test]
fn light_mode_text_is_black() {
    let mut harness = Harness::enterotypes();
    stamp_gui::app::set_dark_mode(&harness.renderer.ctx, false);
    harness.settle();
    let image = harness.screenshot("light_text");
    // Labels and combo-box text in the properties panel are drawn in black.
    let black = (900..1270)
        .flat_map(|x| (30..420).map(move |y| (x, y)))
        .filter(|&(x, y)| image.pixel(x, y) == egui::Color32::BLACK)
        .count();
    assert!(
        black > 300,
        "only {black} black pixels in the properties panel"
    );
}

#[test]
fn metadata_table_switches_samples_off() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let (_, tall) = harness.app.plot_placement.unwrap();
    harness.app.show_metadata_table(true);
    harness.settle();
    // The table takes its room from the bottom of the window.
    let (_, short) = harness.app.plot_placement.unwrap();
    assert!(short.height() < tall.height() - 100.0);
    let image = harness.screenshot("metadata_table");
    assert!(ink(&image, 0, 560, 1280, 760) > 0.01);

    // Leaving samples out changes the groups, and so the results.
    let before = harness.app.results_table().clone();
    let session = &mut harness.app.session;
    let field = session.group_field.clone().unwrap();
    let group = session.groups.group1.clone();
    let members = session.samples_matching(&field, stamp_gui::session::Relation::Equal, &group);
    assert_eq!(members.len(), 8);
    session.set_samples_active(&members[..3], false);
    harness.settle();
    assert_eq!(
        harness
            .app
            .session
            .grouping()
            .unwrap()
            .samples(&group)
            .unwrap()
            .len(),
        5
    );
    assert_ne!(*harness.app.results_table(), before);
    harness.screenshot("metadata_table_samples_off");
}

#[test]
fn configure_plot_changes_the_plot() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    harness.app.show_plot_config(true);
    harness.settle();
    let image = harness.screenshot("configure_plot");
    assert!(ink(&image, 0, 0, 1280, 800) > 0.05);

    let before = harness.app.figure().clone();
    let mut settings = harness.app.plot_settings().clone();
    settings.error_bar.width = 9.0;
    settings.error_bar.show_p_values = false;
    harness.app.set_plot_settings(settings);
    harness.settle();
    let after = harness.app.figure();
    assert!((after.width - before.width - 2.0 * 72.0).abs() < 1.0);
    let p_values = |figure: &stamp_plot::Figure| {
        figure
            .shapes
            .iter()
            .any(|s| matches!(s, stamp_plot::Shape::Text { text, .. } if text == "0.037"))
    };
    assert!(p_values(&before) && !p_values(after));

    // Every plot has its own window of settings.
    for mode in Mode::ALL {
        harness.app.set_mode(mode);
        for &kind in PlotKind::available(mode) {
            harness.app.set_plot_kind(kind);
            harness.settle();
            let name = kind.name().to_lowercase().replace([' ', '-'], "_");
            let mode_name = mode.name().to_lowercase().replace(' ', "_");
            harness.screenshot(&format!("configure_{mode_name}_{name}"));
        }
    }
}

#[test]
fn features_are_highlighted_in_red() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let choices = harness.app.highlight_choices("Phyla");
    assert_eq!(choices.len(), 17);
    assert!(choices.contains(&"Firmicutes".to_string()));
    let red = |figure: &stamp_plot::Figure| -> Vec<String> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                stamp_plot::Shape::Text { text, style, .. }
                    if style.color == stamp_plot::Color::RED =>
                {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect()
    };
    assert!(red(harness.app.figure()).is_empty());

    harness.app.set_highlight(Some("Phyla"), "Firmicutes");
    harness.settle();
    assert_eq!(harness.app.highlighted_features(), ["Firmicutes"]);
    assert_eq!(red(harness.app.figure()), ["Firmicutes"]);
    harness.screenshot("highlight");

    harness.app.set_highlight(None, "");
    harness.settle();
    assert!(red(harness.app.figure()).is_empty());
}
