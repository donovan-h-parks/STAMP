//! Drives the whole interface without a window: loads example data, lets the
//! analyses finish, and checks what is drawn.
//!
//! Set `STAMP_SCREENSHOTS` to a directory to keep the rendered images.

use std::path::PathBuf;

use eframe::egui;
use stamp_gui::headless::{Image, Renderer};
use stamp_gui::platform::Event;
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
        // Most tests look at every feature; the filter that is on when the
        // application starts has a test of its own.
        harness.app.set_p_value_filter(None);
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
            if kind == PlotKind::Rarefaction {
                // The example holds fractional values, which cannot be
                // rarefied; the plot says so instead.
                let figure = harness.app.figure();
                assert!(
                    matches!(&figure.shapes[0], stamp_plot::Shape::Text { text, .. }
                        if text.contains("Rarefaction needs counts")),
                    "{mode:?}: {:?}",
                    figure.shapes.first()
                );
                continue;
            }
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

#[test]
fn specific_features_can_be_selected() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    assert_eq!(harness.app.results_table().rows.len(), 17);

    harness.app.open_select_features();
    // The window takes a few frames to appear in full.
    for _ in 0..4 {
        harness.settle();
    }
    let image = harness.screenshot("select_features");
    assert!(ink(&image, 330, 110, 900, 500) > 0.02);

    // Only the chosen features reach the table, the plot and the count.
    harness
        .app
        .set_selected_features(Some(&["Firmicutes", "Bacteroidetes"]));
    harness.app.set_results_only_active(true);
    harness.settle();
    assert_eq!(harness.app.results_table().rows.len(), 2);
    assert_eq!(harness.app.session.feature_counts(Mode::TwoGroups), (17, 2));
    let active: Vec<String> = harness
        .app
        .feature_table()
        .iter()
        .filter(|r| r.active)
        .map(|r| r.feature.clone())
        .collect();
    assert_eq!(active, ["Bacteroidetes", "Firmicutes"]);
    harness.screenshot("selected_features");

    // The other analyses keep all of their features.
    harness.app.set_mode(Mode::TwoSamples);
    harness.settle();
    assert_eq!(harness.app.results_table().rows.len(), 17);

    harness.app.set_mode(Mode::TwoGroups);
    harness.app.set_selected_features(None);
    harness.settle();
    assert_eq!(harness.app.results_table().rows.len(), 17);
}

#[test]
fn tables_are_sorted_by_their_headings() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let p_values = |app: &StampApp| -> Vec<f64> {
        app.feature_table()
            .iter()
            .map(|r| r.p_value.parse().unwrap())
            .collect()
    };

    // The p-value column: ascending, then descending.
    harness.app.sort_feature_table(2);
    harness.settle();
    let ascending = p_values(&harness.app);
    assert!(ascending.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(harness.app.feature_table()[0].feature, "Unclassified");
    harness.screenshot("sorted_tables");
    harness.app.sort_feature_table(2);
    let descending = p_values(&harness.app);
    assert!(descending.windows(2).all(|pair| pair[0] >= pair[1]));

    // The feature column: alphabetical, then reversed.
    harness.app.sort_feature_table(0);
    assert_eq!(harness.app.feature_table()[0].feature, "Acidobacteria");
    harness.app.sort_feature_table(0);
    assert_eq!(harness.app.feature_table()[0].feature, "Verrucomicrobia");

    // The results table sorts the same way, by any of its columns.
    let table = harness.app.results_table().clone();
    assert_eq!(
        harness.app.results_table_order(),
        (0..17).collect::<Vec<_>>()
    );
    harness.app.sort_results_table(0);
    let order = harness.app.results_table_order();
    assert_eq!(table.rows[order[0]][0], "Acidobacteria");
    harness.app.sort_results_table(0);
    let order = harness.app.results_table_order();
    assert_eq!(table.rows[order[0]][0], "Verrucomicrobia");
}

#[test]
fn switching_a_sample_off_keeps_the_selected_feature() {
    for mode in [Mode::TwoGroups, Mode::MultipleGroups] {
        let mut harness = Harness::enterotypes();
        harness.app.session.set_group_field("Nationality");
        harness.app.set_mode(mode);
        harness.app.set_plot_kind(PlotKind::Bar);
        harness.settle();
        harness.app.select_feature("Actinobacteria");
        harness.settle();

        harness
            .app
            .session
            .set_samples_active(&["AM-AD-1".to_string()], false);
        harness.settle();
        assert_eq!(
            harness.app.session.selected_feature.as_deref(),
            Some("Actinobacteria"),
            "{mode:?}"
        );

        // Nor does a filter that the feature fails change what is shown.
        harness.app.set_p_value_filter(Some(1e-9));
        harness.settle();
        harness
            .app
            .session
            .set_samples_active(&["AM-AD-2".to_string()], false);
        harness.settle();
        assert_eq!(
            harness.app.session.selected_feature.as_deref(),
            Some("Actinobacteria"),
            "{mode:?} with a filter"
        );
    }
}

#[test]
fn starts_with_the_p_value_filter_on() {
    let mut harness = Harness::new();
    harness.app.load_example();
    harness.settle();
    // One feature of the example differs at the 0.05 level.
    assert_eq!(harness.app.session.feature_counts(Mode::TwoGroups), (17, 1));
    assert_eq!(harness.app.session.groups.filter.significance, Some(0.05));
    // The results table lists every feature until asked for the active ones.
    assert_eq!(harness.app.results_table().rows.len(), 17);
    harness.app.set_results_only_active(true);
    harness.settle();
    assert_eq!(harness.app.results_table().rows.len(), 1);
    harness.app.show_results_table(true);
    harness.settle();
    harness.screenshot("results_only_active");
}

#[test]
fn status_bar_and_correction_information() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let status = harness.app.status_summary();
    assert!(
        status.starts_with("Parent categories: 1, Features: 17, Sequences in group 1: "),
        "{status}"
    );
    assert!(status.ends_with("Active features: 17"), "{status}");
    harness.app.set_mode(Mode::TwoSamples);
    harness.settle();
    assert!(harness
        .app
        .status_summary()
        .contains("Sequences in sample 2: "));

    // No correction: only the number of significant features.
    harness.app.set_mode(Mode::TwoGroups);
    harness.settle();
    assert_eq!(
        harness.app.correction_info(),
        ["Method: No correction", "Number of significant features: 1"]
    );
    harness.app.session.set_correction(
        Mode::TwoGroups,
        stamp_core::stats::correction::Correction::Bonferroni,
    );
    harness.app.show_correction_info(true);
    harness.settle();
    let info = harness.app.correction_info();
    assert_eq!(info[0], "Method: Bonferroni");
    assert!(
        info.iter()
            .any(|line| line.starts_with("Corrected error rate: 0.00")),
        "{info:?}"
    );
    harness.screenshot("correction_info");
}

#[test]
fn new_plot_options_change_the_plots() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    let texts = |figure: &stamp_plot::Figure| -> Vec<String> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                stamp_plot::Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    };

    // Heatmap: all features unless asked for the active ones; counts.
    harness.app.set_p_value_filter(Some(0.4));
    harness.app.set_plot_kind(PlotKind::Heatmap);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), 17);
    harness.screenshot("heatmap_options");
    let mut settings = harness.app.plot_settings().clone();
    settings.heatmap_only_active = true;
    settings.heatmap_sequence_counts = true;
    harness.app.set_plot_settings(settings);
    harness.settle();
    let active = harness.app.session.feature_counts(Mode::TwoGroups).1;
    assert!(active > 1 && active < 17);
    assert_eq!(harness.app.figure().regions.len(), active);
    assert!(texts(harness.app.figure()).contains(&"Number of sequences".to_string()));

    // PCA: the panels are chosen separately.
    harness.app.set_plot_kind(PlotKind::Pca);
    harness.settle();
    let samples = harness.app.figure().regions.len() / 3;
    harness.screenshot("pca_fixed_scale");
    let mut settings = harness.app.plot_settings().clone();
    settings.pca.show_pc1_vs_pc3 = false;
    settings.pca.fixed_scale = false;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), 2 * samples);
    assert_eq!(harness.app.figure().height, 6.0 * 72.0);

    // Two samples: counts in the bars, uncorrected p-values, all features.
    harness.app.set_mode(Mode::TwoSamples);
    harness.app.set_plot_kind(PlotKind::ExtendedErrorBar);
    harness.settle();
    assert!(texts(harness.app.figure()).contains(&"Proportion (%)".to_string()));
    let mut settings = harness.app.plot_settings().clone();
    settings.error_bar_sequence_counts = true;
    settings.p_value_histogram_corrected = false;
    settings.profile_bar_only_active = false;
    settings.multiple_comparison.log_scale = true;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert!(texts(harness.app.figure()).contains(&"Sequences".to_string()));
    harness.app.set_plot_kind(PlotKind::PValueHistogram);
    harness.settle();
    assert!(texts(harness.app.figure()).contains(&"p-value".to_string()));
    harness.app.set_plot_kind(PlotKind::MultipleComparison);
    harness.settle();
    assert!(texts(harness.app.figure()).contains(&"10".to_string()));
    harness.screenshot("multiple_comparison_log");

    // Post-hoc: pairs that do not differ are left out unless asked for.
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::PostHoc);
    harness.app.select_feature("Euryarchaeota");
    harness.settle();
    let significant = harness.app.figure().regions.len();
    let mut settings = harness.app.plot_settings().clone();
    settings.post_hoc_only_significant = false;
    harness.app.set_plot_settings(settings);
    harness.settle();
    let all = harness.app.figure().regions.len();
    assert!(significant < all, "{significant} of {all}");
}

fn opened(relative: &str) -> stamp_gui::platform::OpenedFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    stamp_gui::platform::OpenedFile {
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        bytes: std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    }
}

fn text_file(name: &str, text: &str) -> stamp_gui::platform::OpenedFile {
    stamp_gui::platform::OpenedFile {
        name: name.to_string(),
        bytes: text.as_bytes().to_vec(),
    }
}

#[test]
fn biom_tables_open_from_load_profile() {
    for file in ["otu_table_v2.biom", "otu_table_v1.biom"] {
        let mut harness = Harness::new();
        // A table with metadata asks which field gives the hierarchy.
        harness
            .app
            .open_profile_file(opened(&format!("../stamp-core/tests/data/{file}")));
        assert_eq!(harness.app.pending_profile(), Some("BIOM"), "{file}");
        assert!(harness.app.session.tree.is_none());
        harness.settle();
        harness.screenshot("biom_table");
        harness.app.accept_pending_profile();
        harness.settle();
        assert_eq!(harness.app.pending_profile(), None);
        assert_eq!(harness.app.notice(), None, "{file}");
        let tree = harness
            .app
            .session
            .tree
            .as_ref()
            .expect("the profile loaded");
        // Three levels of taxonomy, then the observation ids.
        assert_eq!(tree.hierarchy_headings().len(), 4, "{file}");
        assert_eq!(tree.sample_names(), ["S1", "S2", "S3", "S4"]);

        // The ids alone can be chosen instead.
        harness
            .app
            .open_profile_file(opened(&format!("../stamp-core/tests/data/{file}")));
        harness.app.choose_biom_field(None);
        harness.app.accept_pending_profile();
        let tree = harness.app.session.tree.as_ref().unwrap();
        assert_eq!(tree.hierarchy_headings(), ["Observation Ids"]);
    }

    // A table without metadata loads at once; one that is damaged says so.
    let mut harness = Harness::new();
    harness
        .app
        .open_profile_file(opened("../stamp-core/tests/data/plain_v2.biom"));
    assert_eq!(harness.app.pending_profile(), None);
    assert!(harness.app.session.tree.is_some());
    let mut broken = opened("../stamp-core/tests/data/plain_v2.biom");
    broken.bytes.truncate(600);
    harness.app.open_profile_file(broken);
    assert!(harness.app.notice().unwrap().contains("Could not load"));
}

#[test]
fn mothur_files_open_from_load_profile() {
    let taxonomy = "seq1\tBacteria(100);Firmicutes(98);Bacilli(90);\n\
        seq2\tBacteria(100);Firmicutes(95);\n\
        seq3\tBacteria(100);Proteobacteria(98);Gammaproteobacteria(91);\n";
    let groups = "seq1\tgut\nseq2\tsoil\nseq3\tsoil\n";
    let mut harness = Harness::new();
    harness
        .app
        .open_profile_file(text_file("final.taxonomy", taxonomy));
    assert_eq!(harness.app.pending_profile(), Some("mothur"));
    harness.settle();
    harness.screenshot("mothur_files");

    // Nothing happens until the groups file has been given.
    harness.app.accept_pending_profile();
    assert_eq!(harness.app.pending_profile(), Some("mothur"));
    harness.app.deliver(Event::Auxiliary(
        stamp_gui::platform::Auxiliary::MothurGroups,
        text_file("final.groups", groups),
    ));
    harness.settle();
    harness.app.accept_pending_profile();
    harness.settle();
    assert_eq!(harness.app.notice(), None);
    let tree = harness
        .app
        .session
        .tree
        .as_ref()
        .expect("the profile loaded");
    assert_eq!(tree.sample_names(), ["gut", "soil"]);

    // An ordinary profile still loads directly.
    harness.app.open_profile_file(opened(
        "../../examples/EnterotypesArumugam/Enterotypes.profile.spf",
    ));
    assert_eq!(harness.app.pending_profile(), None);
    assert_eq!(harness.app.session.tree.as_ref().unwrap().num_samples(), 39);
}

#[test]
fn shortcuts_plot_windows_and_remembered_settings() {
    let mut harness = Harness::enterotypes();
    harness.settle();

    // Ctrl+M shows the metadata table and Ctrl+W sends the plot to a window.
    let app = &mut harness.app;
    harness
        .renderer
        .press_shortcut(egui::Key::M, |ui| app.draw(ui));
    harness.settle();
    let with_table = harness.app.saved(&harness.renderer.ctx);
    assert!(with_table.show_metadata_table);
    let app = &mut harness.app;
    harness
        .renderer
        .press_shortcut(egui::Key::W, |ui| app.draw(ui));
    harness.app.send_plot_to_window();
    harness.settle();
    assert_eq!(harness.app.plot_window_count(), 2);
    let image = harness.screenshot("plot_windows");
    assert!(ink(&image, 0, 0, 1280, 800) > 0.05);

    // What is remembered survives being written out and read back.
    let mut settings = harness.app.plot_settings().clone();
    settings.min_p_exponent = -5;
    settings.pca.fixed_scale = false;
    settings.heatmap.color_map = stamp_plot::heatmap::ColorMap::Jet;
    settings.axes_color = stamp_plot::Color::rgb(10, 20, 30);
    harness.app.set_plot_settings(settings.clone());
    harness.app.set_zoom(2.0);
    let saved = harness.app.saved(&harness.renderer.ctx);
    let text = serde_json::to_string(&saved).unwrap();
    let read: stamp_gui::app::Saved = serde_json::from_str(&text).unwrap();
    assert_eq!(read, saved);
    // A file from an older version, lacking newer settings, still reads.
    let old: stamp_gui::app::Saved = serde_json::from_str(r#"{"zoom": 1.5}"#).unwrap();
    assert_eq!(old.zoom, 1.5);
    assert_eq!(old.plots, stamp_gui::plots::PlotSettings::default());

    let mut fresh = Harness::enterotypes();
    fresh.app.restore(&fresh.renderer.ctx, read);
    fresh.settle();
    assert_eq!(fresh.app.zoom(), 2.0);
    assert_eq!(fresh.app.plot_settings(), &settings);
    assert!(fresh.app.saved(&fresh.renderer.ctx).show_metadata_table);
}

#[test]
fn preferences_reach_the_tables_and_plots() {
    let mut harness = Harness::enterotypes();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.settle();
    // Some p-value of the example is below 1e-5.
    let bounded = |app: &StampApp| {
        app.feature_table()
            .iter()
            .filter(|r| r.p_value.starts_with('<'))
            .count()
    };
    assert_eq!(bounded(&harness.app), 1);
    let mut settings = harness.app.plot_settings().clone();
    settings.min_p_exponent = -5;
    let grey = stamp_plot::axes::AXES_COLOR;
    let (red, green) = (
        stamp_plot::Color::rgb(200, 0, 0),
        stamp_plot::Color::rgb(0, 150, 0),
    );
    settings.axes_color = red;
    settings.sample_colors[0] = green;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert!(bounded(&harness.app) > 1);
    assert!(harness
        .app
        .feature_table()
        .iter()
        .any(|r| r.p_value == "< 1e-5"));

    let uses = |figure: &stamp_plot::Figure, wanted: stamp_plot::Color| {
        figure.shapes.iter().any(|shape| match shape {
            stamp_plot::Shape::Line { stroke, .. } => stroke.color == wanted,
            stamp_plot::Shape::Rect { fill, .. } => *fill == Some(wanted),
            _ => false,
        })
    };
    harness.app.set_mode(Mode::TwoSamples);
    harness.settle();
    let figure = harness.app.figure();
    assert!(uses(figure, red) && uses(figure, green));
    assert!(!uses(figure, grey));
    assert!(!uses(figure, stamp_plot::extended_error_bar::SAMPLE1_COLOR));
    harness.screenshot("preference_colours");
}

#[test]
fn unticked_groups_and_inactive_samples_leave_the_lists() {
    let mut harness = Harness::enterotypes();
    harness.settle();
    assert_eq!(harness.app.session.groups.group1, "Enterotype 1");
    assert_eq!(
        harness.app.session.groups.group2.as_deref(),
        Some("Enterotype 2")
    );

    // Unticking the first group moves the comparison to the groups left.
    harness
        .app
        .session
        .multi
        .active_groups
        .retain(|g| g != "Enterotype 1");
    harness.settle();
    harness.settle();
    let groups = &harness.app.session.groups;
    assert_ne!(groups.group1, "Enterotype 1");
    assert_ne!(groups.group2.as_deref(), Some("Enterotype 1"));
    assert_ne!(Some(&groups.group1), groups.group2.as_ref());

    // A sample switched off in the metadata table leaves the sample lists.
    harness.app.set_mode(Mode::TwoSamples);
    harness.settle();
    let first = harness.app.session.samples.sample1.clone();
    harness
        .app
        .session
        .set_samples_active(std::slice::from_ref(&first), false);
    harness.settle();
    harness.settle();
    let samples = &harness.app.session.samples;
    assert_ne!(samples.sample1, first);
    assert_ne!(samples.sample1, samples.sample2);
}

#[test]
fn filter_is_named_for_q_values_under_a_false_discovery_rate() {
    // The label is drawn text; its width shows which one is in use.
    let mut harness = Harness::enterotypes();
    harness.settle();
    let before = harness.screenshot("p_value_filter_label");
    harness.app.session.set_correction(
        Mode::TwoGroups,
        stamp_core::stats::correction::Correction::BenjaminiHochberg,
    );
    harness.settle();
    let after = harness.screenshot("q_value_filter_label");
    let differs = (985..1110)
        .flat_map(|x| (335..360).map(move |y| (x, y)))
        .any(|(x, y)| before.pixel(x, y) != after.pixel(x, y));
    assert!(differs, "the filter label did not change");
}

#[test]
fn starts_with_the_tests_and_legends_of_stamp_v2() {
    use stamp_core::stats::ci::SampleCi;
    use stamp_core::stats::two_sample::SampleTest;
    use stamp_plot::axes::LegendPosition;

    let mut harness = Harness::new();
    harness.app.load_example();
    harness.settle();
    let samples = &harness.app.session.samples;
    assert_eq!(samples.analysis.test, SampleTest::GTestFisher);
    assert_eq!(
        samples.analysis.ci_method,
        SampleCi::DiffBetweenPropAsymptoticCc
    );
    // The first two samples in alphabetical order.
    let mut names = harness
        .app
        .session
        .tree
        .as_ref()
        .unwrap()
        .sample_names()
        .to_vec();
    names.sort();
    assert_eq!([&samples.sample1, &samples.sample2], [&names[0], &names[1]]);

    // Every plot with a legend starts with one, placed where it fits best.
    let settings = harness.app.plot_settings();
    for legend in [
        settings.stat_bar.legend,
        settings.profile_bar.legend,
        settings.sequence_histogram.legend,
        settings.bar.legend,
        settings.pca.legend,
        settings.heatmap.legend,
    ] {
        assert_eq!(legend, LegendPosition::Best);
    }
    assert_ne!(
        settings.error_bar.legend,
        stamp_plot::extended_error_bar::Legend::None
    );
}

#[test]
fn scatter_plot_tooltips_say_what_stamp_v2_did() {
    let mut harness = Harness::enterotypes();
    harness.app.set_plot_kind(PlotKind::Scatter);
    harness.settle();
    let tooltip = |app: &StampApp| {
        app.figure()
            .regions
            .iter()
            .find(|r| r.item == "Firmicutes")
            .expect("a point for Firmicutes")
            .tooltip
            .clone()
    };
    let groups = tooltip(&harness.app);
    for expected in [
        "Firmicutes\n\nEnterotype 1 mean proportion: ",
        "Enterotype 2 mean proportion: ",
        "Difference between mean proportions (%): 6.511",
        "Ratio of mean proportions: ",
        "p-value: 0.165\nCorrected p-value: 0.165",
    ] {
        assert!(
            groups.contains(expected),
            "missing {expected:?} in {groups:?}"
        );
    }

    harness.app.set_mode(Mode::TwoSamples);
    harness.app.set_plot_kind(PlotKind::Scatter);
    harness.settle();
    let samples = tooltip(&harness.app);
    let (s1, s2) = (
        harness.app.session.samples.sample1.clone(),
        harness.app.session.samples.sample2.clone(),
    );
    for expected in [
        format!("Sequences in {s1}: "),
        format!("Sequences in {s2}: "),
        format!("{s1} percentage: "),
        "Difference between proportions (%): ".to_string(),
        "Ratio of proportions: ".to_string(),
        "Corrected p-value: ".to_string(),
    ] {
        assert!(
            samples.contains(&expected),
            "missing {expected:?} in {samples:?}"
        );
    }
}

#[test]
fn rarefaction_curves_for_samples_and_groups() {
    let mut harness = Harness::new();
    harness
        .app
        .open_profile_file(opened("../../examples/CoalSeamGas/coal.spf"));
    harness.app.load_metadata(
        "coal.metadata.tsv",
        &example("CoalSeamGas/coal.metadata.tsv"),
    );
    harness.app.set_p_value_filter(None);
    assert_eq!(harness.app.notice(), None);
    // A level with enough categories for the curves to be worth looking at.
    harness.app.session.profile_level = "Family".to_string();
    harness.app.session.invalidate_all();
    harness.app.show_feature_table(false);
    let curves = |app: &StampApp| {
        app.figure()
            .shapes
            .iter()
            .filter(|s| {
                matches!(s, stamp_plot::Shape::Line { points, stroke }
                    if points.len() > 2 && stroke.dash.is_none())
            })
            .count()
    };

    // Two groups: a curve for each sample of the two groups.
    harness.app.set_mode(Mode::TwoGroups);
    harness.app.set_plot_kind(PlotKind::Rarefaction);
    harness.settle();
    let (in_groups, first_group) = {
        let (profile, _) = harness.app.session.groups.output.as_ref().unwrap();
        (
            profile.profile.sample_names.len(),
            profile.group_name1.clone(),
        )
    };
    assert!(in_groups >= 4);
    assert_eq!(curves(&harness.app), in_groups);
    let image = harness.screenshot("rarefaction_two_groups");
    assert!(ink(&image, 300, 120, 950, 700) > 0.01);
    // The curves take the colours of their groups.
    let color = harness.app.session.group_color(&first_group);
    assert!(harness
        .app
        .figure()
        .shapes
        .iter()
        .any(|s| matches!(s, stamp_plot::Shape::Line { stroke, .. } if stroke.color == color)));

    // Multiple groups: every sample of the ticked groups.
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Rarefaction);
    harness.settle();
    let in_groups = {
        let (profile, _) = harness.app.session.multi.output.as_ref().unwrap();
        profile.profile.sample_names.len()
    };
    assert_eq!(curves(&harness.app), in_groups);
    harness.screenshot("rarefaction_multiple_groups");

    // Two samples: two curves; Chao1 adds a dashed line for each.
    harness.app.set_mode(Mode::TwoSamples);
    harness.app.set_plot_kind(PlotKind::Rarefaction);
    harness.settle();
    assert_eq!(curves(&harness.app), 2);
    let mut settings = harness.app.plot_settings().clone();
    settings.rarefaction.show_chao1 = true;
    settings.rarefaction.truncate_to_smallest = true;
    harness.app.set_plot_settings(settings);
    harness.app.show_plot_config(true);
    harness.settle();
    let dashed = harness
        .app
        .figure()
        .shapes
        .iter()
        .filter(|s| {
            matches!(s, stamp_plot::Shape::Line { points, stroke }
                if points.len() == 2 && stroke.dash.is_some())
        })
        .count();
    assert_eq!(dashed, 2);
    harness.screenshot("rarefaction_two_samples");

    // The level of the hierarchy decides what is counted.
    let features = |app: &StampApp| -> f64 {
        app.figure()
            .regions
            .iter()
            .filter_map(|r| {
                let line = r
                    .tooltip
                    .lines()
                    .find(|l| l.starts_with("Features observed: "))?;
                line["Features observed: ".len()..].parse().ok()
            })
            .fold(0.0, f64::max)
    };
    let at_species = {
        harness.app.session.profile_level = "Species".to_string();
        harness.app.session.invalidate_all();
        harness.settle();
        harness.settle();
        features(&harness.app)
    };
    let at_phylum = {
        harness.app.session.profile_level = "Phylum".to_string();
        harness.app.session.invalidate_all();
        harness.settle();
        harness.settle();
        features(&harness.app)
    };
    // In this dataset few sequences are classified to species, and the
    // unclassified ones are not counted, so fewer species than phyla show.
    assert!(
        at_phylum >= 1.0 && at_species >= 1.0 && at_species != at_phylum,
        "{at_species} species, {at_phylum} phyla"
    );
}

#[test]
fn both_built_in_examples_load() {
    use stamp_gui::app::Example;
    assert_eq!(
        Example::ALL.map(Example::name),
        ["Enterotypes", "Coal seam gas"]
    );
    let mut harness = Harness::new();
    harness.app.load_example_data(Example::CoalSeamGas);
    harness.settle();
    assert_eq!(harness.app.notice(), None);
    let session = &harness.app.session;
    let tree = session.tree.as_ref().expect("the coal-seam profile loaded");
    assert_eq!(tree.num_samples(), 44);
    assert_eq!(tree.hierarchy_headings().len(), 7);
    assert_eq!(session.profile_name, "coal.spf");
    assert!(session.metadata.is_some());
    // It is the same data as the example file in the repository.
    let from_file = stamp_core::ProfileTree::read(&example("CoalSeamGas/coal.spf")).unwrap();
    assert_eq!(tree.num_leaves(), from_file.num_leaves());
    assert_eq!(
        tree.sample_total("573857"),
        from_file.sample_total("573857")
    );
    // Its counts are whole numbers, so it can be rarefied.
    harness.app.set_plot_kind(PlotKind::Rarefaction);
    harness.settle();
    assert!(harness.app.figure().shapes.len() > 10);

    harness.app.load_example_data(Example::Enterotypes);
    harness.settle();
    assert_eq!(harness.app.session.profile_name, "Enterotypes.profile.spf");
    assert_eq!(harness.app.session.tree.as_ref().unwrap().num_samples(), 39);
}
