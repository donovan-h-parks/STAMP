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
            if mode == Mode::Correlation {
                // The example has no numeric metadata field to correlate
                // with; the plot says so instead.
                let figure = harness.app.figure();
                assert!(
                    matches!(&figure.shapes[0], stamp_plot::Shape::Text { text, .. }
                        if text.contains("holds a number")),
                    "{kind:?}: {:?}",
                    figure.shapes.first()
                );
                continue;
            }
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
        .flat_map(|x| (380..403).map(move |y| (x, y)))
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

#[test]
fn rarefaction_curves_for_whole_groups() {
    use stamp_plot::rarefaction::Curves;
    let mut harness = Harness::new();
    harness
        .app
        .load_example_data(stamp_gui::app::Example::CoalSeamGas);
    harness.app.set_p_value_filter(None);
    harness.app.session.profile_level = "Genus".to_string();
    harness.app.session.invalidate_all();
    harness.app.show_feature_table(false);
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Rarefaction);
    harness.settle();
    let groups = harness.app.session.multi.active_groups.len();
    assert!(groups >= 3);
    let lines = |app: &StampApp| {
        app.figure()
            .shapes
            .iter()
            .filter(|s| {
                matches!(s, stamp_plot::Shape::Line { points, stroke }
                    if points.len() > 2 && stroke.dash.is_none())
            })
            .count()
    };
    let per_sample = lines(&harness.app);
    assert!(per_sample > groups);

    // Pooled by sequences: one curve per group.
    let mut settings = harness.app.plot_settings().clone();
    settings.rarefaction.curves = Curves::GroupsPooled;
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert_eq!(lines(&harness.app), groups);
    harness.screenshot("rarefaction_pooled");

    // By number of samples: one curve per group, a mark per sample, and
    // the number of samples along the bottom.
    settings.rarefaction.curves = Curves::GroupsAccumulated;
    settings.rarefaction.show_chao1 = true;
    harness.app.set_plot_settings(settings);
    harness.app.show_plot_config(true);
    harness.settle();
    assert_eq!(lines(&harness.app), groups);
    let figure = harness.app.figure();
    let marks = figure
        .shapes
        .iter()
        .filter(|s| matches!(s, stamp_plot::Shape::Circle { .. }))
        .count();
    assert_eq!(marks, per_sample);
    assert!(figure
        .shapes
        .iter()
        .any(|s| matches!(s, stamp_plot::Shape::Text { text, .. } if text == "Number of samples")));
    // The eight Core samples: the question the curve is for.
    let core = figure
        .regions
        .iter()
        .rfind(|r| r.item == "Core")
        .expect("a curve for the Core group");
    assert!(
        core.tooltip.contains("8 of 8 samples: "),
        "{}",
        core.tooltip
    );
    harness.screenshot("rarefaction_by_samples");

    // Two samples always draw the two samples.
    harness.app.set_mode(Mode::TwoSamples);
    harness.app.set_plot_kind(PlotKind::Rarefaction);
    harness.settle();
    assert_eq!(lines(&harness.app), 2);
}

#[test]
fn remembered_plot_settings_do_not_outlive_their_defaults() {
    use stamp_gui::app::{Saved, DEFAULTS_VERSION};
    use stamp_plot::axes::LegendPosition;

    // Settings as an earlier version stored them: no version number, and
    // the legend positions that version started with.
    let harness = Harness::enterotypes();
    let mut old = harness.app.saved(&harness.renderer.ctx);
    old.plots.pca.legend = LegendPosition::UpperRight;
    old.plots.bar.legend = LegendPosition::None;
    old.zoom = 2.5;
    let mut stored: serde_json::Value = serde_json::to_value(&old).unwrap();
    stored.as_object_mut().unwrap().remove("defaults_version");
    let read: Saved = serde_json::from_value(stored).unwrap();
    assert_eq!(read.defaults_version, 0);

    // The plots start from the current defaults; the rest is kept.
    let current = read.up_to_date();
    assert_eq!(current.defaults_version, DEFAULTS_VERSION);
    assert_eq!(current.plots.pca.legend, LegendPosition::Best);
    assert_eq!(current.plots.bar.legend, LegendPosition::Best);
    assert_eq!(current.zoom, 2.5);

    // Settings saved by this version are restored as they are.
    let mut chosen = harness.app.saved(&harness.renderer.ctx);
    chosen.plots.pca.legend = LegendPosition::LowerLeft;
    assert_eq!(chosen.defaults_version, DEFAULTS_VERSION);
    assert_eq!(chosen.clone().up_to_date(), chosen);
}

#[test]
fn volcano_and_composition_plots() {
    let mut harness = Harness::enterotypes();
    harness.app.show_feature_table(false);
    harness.app.set_plot_kind(PlotKind::Volcano);
    harness.settle();
    // A point for each of the 17 phyla; the one significant phylum is
    // coloured, the rest muted.
    let figure = harness.app.figure().clone();
    assert_eq!(figure.regions.len(), 17);
    let coloured = |figure: &stamp_plot::Figure, color: stamp_plot::Color| {
        figure
            .shapes
            .iter()
            .filter(|s| matches!(s, stamp_plot::Shape::Circle { fill: Some(c), .. } if *c == color))
            .count()
    };
    let second_group = harness.app.session.group_color("Enterotype 2");
    assert_eq!(coloured(&figure, second_group), 1);
    let image = harness.screenshot("volcano");
    assert!(ink(&image, 300, 120, 950, 700) > 0.01);
    // Clicking a point's feature and highlighting work as for other plots.
    harness.app.set_highlight(Some("Phyla"), "Firmicutes");
    harness.settle();
    assert!(harness.app.figure().shapes.iter().any(|s| matches!(
        s,
        stamp_plot::Shape::Circle { stroke: Some(stroke), .. }
            if stroke.color == stamp_plot::Color::RED
    )));
    harness.app.set_highlight(None, "");

    // Composition: a bar per sample of the two groups, then one per group.
    harness.app.set_plot_kind(PlotKind::Composition);
    harness.settle();
    let bars = |app: &StampApp| {
        let mut names: Vec<String> = app
            .figure()
            .regions
            .iter()
            .filter_map(|r| r.tooltip.lines().nth(1).map(str::to_string))
            .collect();
        names.sort();
        names.dedup();
        names
    };
    assert_eq!(bars(&harness.app).len(), 8 + 6);
    assert!(harness
        .app
        .figure()
        .regions
        .iter()
        .any(|r| r.item == stamp_plot::composition::OTHER));
    harness.screenshot("composition_samples");
    let mut settings = harness.app.plot_settings().clone();
    settings.composition.bars = stamp_plot::composition::Bars::Groups;
    settings.composition.features_to_show = 5;
    harness.app.set_plot_settings(settings);
    harness.app.show_plot_config(true);
    harness.settle();
    assert_eq!(bars(&harness.app), ["Enterotype 1", "Enterotype 2"]);
    harness.screenshot("composition_groups");

    // Multiple groups and two samples draw them too.
    for mode in [Mode::MultipleGroups, Mode::TwoSamples] {
        harness.app.set_mode(mode);
        for kind in [PlotKind::Volcano, PlotKind::Composition] {
            harness.app.set_plot_kind(kind);
            harness.settle();
            // At least two bars of five features and "Other", or a point
            // for each phylum.
            assert!(
                harness.app.figure().regions.len() >= 12,
                "{mode:?} {kind:?}"
            );
        }
    }
}

#[test]
fn mann_whitney_and_dunn_can_be_chosen() {
    use stamp_core::stats::post_hoc::{PostHocP, PostHocTest};
    use stamp_core::stats::two_group::GroupTest;

    let mut harness = Harness::enterotypes();
    harness.settle();
    let welch: Vec<(String, f64)> = harness
        .app
        .session
        .groups
        .output
        .as_ref()
        .unwrap()
        .1
        .rows
        .iter()
        .map(|r| (r.feature.clone(), r.p_value))
        .collect();
    harness.app.session.groups.analysis.test = GroupTest::MannWhitney;
    harness.app.session.invalidate(Mode::TwoGroups);
    harness.settle();
    let (_, results) = harness.app.session.groups.output.as_ref().unwrap();
    assert_eq!(results.analysis.test, GroupTest::MannWhitney);
    assert_eq!(results.rows.len(), 17);
    // A different test: different p-values, the same differences in means.
    let p_of = |feature: &str| results.rows.iter().find(|r| r.feature == feature).unwrap();
    assert!(welch
        .iter()
        .any(|(f, p)| (p_of(f).p_value - p).abs() > 1e-6));
    for row in &results.rows {
        assert!((0.0..=1.0).contains(&row.p_value), "{row:?}");
        assert!(row.lower_ci <= row.effect_size + 1e-9 && row.effect_size <= row.upper_ci + 1e-9);
    }

    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.session.multi.post_hoc_test = PostHocTest::Dunn;
    harness.app.set_plot_kind(PlotKind::PostHoc);
    harness.app.select_feature("Euryarchaeota");
    harness.settle();
    let result = harness.app.session.post_hoc().unwrap().unwrap();
    assert_eq!(result.comparisons.len(), 10);
    for comparison in &result.comparisons {
        let PostHocP::Value(p) = comparison.p_value;
        assert!((0.0..=1.0).contains(&p));
    }
    harness.screenshot("post_hoc_dunn");
}

#[test]
fn community_plots_for_groups() {
    use stamp_core::stats::diversity::{AlphaMetric, Distance, Normalisation};
    let mut harness = Harness::new();
    harness
        .app
        .load_example_data(stamp_gui::app::Example::CoalSeamGas);
    harness.app.set_p_value_filter(None);
    harness.app.session.profile_level = "Genus".to_string();
    harness.app.session.invalidate_all();
    harness.app.show_feature_table(false);
    harness.app.set_mode(Mode::MultipleGroups);
    harness.settle();
    let samples = harness
        .app
        .session
        .multi
        .output
        .as_ref()
        .unwrap()
        .0
        .profile
        .sample_names
        .len();
    let texts = |app: &StampApp| -> Vec<String> {
        app.figure()
            .shapes
            .iter()
            .filter_map(|s| match s {
                stamp_plot::Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    };

    // Alpha diversity: a box per group, the measure on the value axis.
    harness.app.set_plot_kind(PlotKind::AlphaDiversity);
    harness.settle();
    let shown = texts(&harness.app);
    assert!(shown.contains(&"Shannon index".to_string()), "{shown:?}");
    assert!(shown.contains(&"Core".to_string()));
    harness.screenshot("alpha_diversity");
    let mut settings = harness.app.plot_settings().clone();
    settings.alpha_metric = AlphaMetric::Chao1;
    settings.community_normalisation = Normalisation::Rarefy;
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    let shown = texts(&harness.app);
    assert!(shown.iter().any(|t| t.contains("rarefied to")), "{shown:?}");

    // PCoA: a point per sample and the three community tests.
    settings.community_normalisation = Normalisation::Proportions;
    settings.community_distance = Distance::BrayCurtis;
    settings.pcoa.show_pc3_vs_pc2 = false;
    settings.pcoa.show_pc1_vs_pc3 = false;
    harness.app.set_plot_settings(settings.clone());
    harness.app.set_plot_kind(PlotKind::Pcoa);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), samples);
    let shown = texts(&harness.app);
    for start in [
        "PCo1 (",
        "PERMANOVA: pseudo-F = ",
        "ANOSIM: R = ",
        "PERMDISP: F = ",
    ] {
        assert!(
            shown.iter().any(|t| t.starts_with(start)),
            "missing {start} in {shown:?}"
        );
    }
    assert!(shown.contains(&"Bray-Curtis distance".to_string()));
    harness.screenshot("pcoa");
    // The same seed gives the same permutation p-values on every redraw.
    let first = shown.clone();
    harness.app.set_plot_kind(PlotKind::Pca);
    harness.settle();
    harness.app.set_plot_kind(PlotKind::Pcoa);
    harness.settle();
    assert_eq!(texts(&harness.app), first);
    // Another distance changes the picture; the tests can be left out.
    settings.community_distance = Distance::Aitchison;
    settings.pcoa_show_tests = false;
    harness.app.set_plot_settings(settings.clone());
    harness.app.show_plot_config(true);
    harness.settle();
    let shown = texts(&harness.app);
    assert!(shown.contains(&"Aitchison distance".to_string()));
    assert!(!shown.iter().any(|t| t.starts_with("PERMANOVA")));
    harness.screenshot("pcoa_settings");
    harness.app.show_plot_config(false);

    // Distances within each group and between groups.
    harness.app.set_plot_kind(PlotKind::DistanceBox);
    harness.settle();
    let shown = texts(&harness.app);
    assert!(shown.contains(&"Core".to_string()) && shown.contains(&"Between groups".to_string()));
    harness.screenshot("distance_box");

    // The enterotype example holds proportions: usable as they are, but
    // it cannot be rarefied.
    let mut harness = Harness::enterotypes();
    harness.app.set_plot_kind(PlotKind::Pcoa);
    harness.settle();
    assert!(harness.app.figure().regions.len() >= 14);
    let mut settings = harness.app.plot_settings().clone();
    settings.community_normalisation = Normalisation::Rarefy;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert!(texts(&harness.app)[0].contains("Rarefying needs counts"));
}

/// The coal-seam metadata with two more fields: "Subject", which pairs
/// each produced-water sample with one core sample, and "Depth", a number
/// for every sample but the last.
fn coal_metadata_with_pairs_and_depths() -> String {
    let text = example("CoalSeamGas/coal.metadata.tsv");
    let mut lines = text.lines();
    let heading = lines.next().unwrap();
    let column = heading
        .split('\t')
        .position(|h| h == "Core vs. Water")
        .unwrap();
    let rows: Vec<&str> = lines.filter(|l| !l.trim().is_empty()).collect();
    let (mut cores, mut waters) = (0, 0);
    let mut out = format!("{heading}\tSubject\tDepth\n");
    for (i, row) in rows.iter().enumerate() {
        let subject = if row.split('\t').nth(column) == Some("Core") {
            cores += 1;
            cores
        } else {
            waters += 1;
            waters
        };
        let depth = if i + 1 == rows.len() {
            "unknown".to_string()
        } else {
            format!("{:.1}", 10.0 + 7.5 * ((i * 7) % rows.len()) as f64)
        };
        out.push_str(&format!("{row}\ts{subject}\t{depth}\n"));
    }
    out
}

fn coal_with_pairs_and_depths() -> Harness {
    let mut harness = Harness::new();
    harness
        .app
        .load_profile("coal.spf", &example("CoalSeamGas/coal.spf"));
    harness
        .app
        .load_metadata("coal.metadata.tsv", &coal_metadata_with_pairs_and_depths());
    harness.app.set_p_value_filter(None);
    harness.app.session.profile_level = "Phylum".to_string();
    harness.app.session.invalidate_all();
    harness
}

fn figure_texts(app: &StampApp) -> Vec<String> {
    app.figure()
        .shapes
        .iter()
        .filter_map(|s| match s {
            stamp_plot::Shape::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn features_are_correlated_with_a_numeric_field() {
    use stamp_core::stats::correlation::CorrelationMethod;
    let mut harness = coal_with_pairs_and_depths();
    harness.app.set_mode(Mode::Correlation);
    harness.settle();
    // Only "Depth" holds numbers.
    assert_eq!(harness.app.session.numeric_fields(), ["Depth"]);
    let (profile, results) = harness.app.session.correlation.output.as_ref().unwrap();
    let samples = profile.values.len();
    assert_eq!(profile.without_value, 1);
    let features = results.rows.len();
    assert!(features > 5);
    assert!(harness
        .app
        .status_summary()
        .contains(&format!("Samples: {samples} (1 without a number left out)")));

    // The scatter plot of the selected feature: a point per sample, the
    // fitted line, and the correlation.
    assert_eq!(harness.app.plot_kind(), PlotKind::CorrelationScatter);
    assert_eq!(harness.app.figure().regions.len(), samples);
    let shown = figure_texts(&harness.app);
    assert!(shown.contains(&"Depth".to_string()), "{shown:?}");
    assert!(shown.iter().any(|t| t.starts_with("rho = ")), "{shown:?}");
    // Coloured by the group field.
    assert!(shown.contains(&"Core".to_string()));
    harness.screenshot("correlation_scatter");

    // The results table has a row per feature and the coefficient.
    harness.app.show_results_table(true);
    harness.settle();
    let table = harness.app.results_table();
    assert_eq!(table.rows.len(), features);
    assert!(table
        .headings
        .contains(&"Spearman rank correlation (rho)".to_string()));
    harness.screenshot("correlation_table");
    harness.app.show_results_table(false);

    // Pearson's r instead, and the options of the plot.
    harness.app.session.correlation.analysis.method = CorrelationMethod::Pearson;
    harness.app.session.invalidate(Mode::Correlation);
    let mut settings = harness.app.plot_settings().clone();
    settings.correlation.show_line = false;
    harness.app.set_plot_settings(settings);
    harness.app.show_plot_config(true);
    harness.settle();
    let shown = figure_texts(&harness.app);
    assert!(shown.iter().any(|t| t.starts_with("r = ")), "{shown:?}");
    harness.screenshot("correlation_settings");
    harness.app.show_plot_config(false);

    // The other plots of the mode.
    harness.app.set_plot_kind(PlotKind::Volcano);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), features);
    assert!(figure_texts(&harness.app).contains(&"Pearson correlation (r)".to_string()));
    harness.screenshot("correlation_volcano");
    harness.app.set_plot_kind(PlotKind::PValueHistogram);
    harness.settle();
    assert!(harness.app.figure().shapes.len() > 10);

    // Without a numeric field the mode says what it needs.
    let mut harness = Harness::enterotypes();
    harness.app.set_mode(Mode::Correlation);
    harness.settle();
    assert!(harness.app.session.numeric_fields().is_empty());
    assert!(figure_texts(&harness.app)[0].contains("holds a number"));
    harness.screenshot("correlation_no_field");
}

#[test]
fn paired_tests_and_the_log_ratio_transform() {
    use stamp_core::stats::results::Transform;
    use stamp_core::stats::two_group::GroupTest;
    let mut harness = coal_with_pairs_and_depths();
    harness.app.session.set_group_field("Core vs. Water");
    harness.app.set_mode(Mode::TwoGroups);
    harness.settle();
    let (profile, results) = harness.app.session.groups.output.as_ref().unwrap();
    assert_eq!(profile.samples_in_group1().len(), 29);
    let unpaired: Vec<(String, f64, f64)> = results
        .rows
        .iter()
        .map(|r| (r.feature.clone(), r.p_value, r.effect_size))
        .collect();

    // A paired test with no pair field explains itself on the plot.
    harness.app.session.groups.analysis.test = GroupTest::WilcoxonSignedRank;
    harness.app.session.invalidate(Mode::TwoGroups);
    harness.settle();
    assert!(figure_texts(&harness.app)[0].contains("form a pair"));

    // With the field: the 15 produced-water samples and their partners.
    harness.app.session.groups.pair_field = Some("Subject".to_string());
    harness.app.session.invalidate(Mode::TwoGroups);
    harness.settle();
    assert_eq!(
        harness.app.session.groups.pairing.as_deref(),
        Some("15 pairs; 14 samples without a partner left out")
    );
    let (profile, results) = harness.app.session.groups.output.as_ref().unwrap();
    assert_eq!(profile.samples_in_group1().len(), 15);
    assert_eq!(profile.samples_in_group2().len(), 15);
    assert!(results.rows.iter().any(|r| r.p_value < 1.0));
    harness.screenshot("paired_test");

    // The transform changes p-values and nothing else.
    harness.app.session.groups.analysis.test = GroupTest::Welch;
    harness.app.session.groups.analysis.transform = Transform::CentredLogRatio;
    harness.app.session.invalidate(Mode::TwoGroups);
    harness.settle();
    let (_, results) = harness.app.session.groups.output.as_ref().unwrap();
    let mut changed = 0;
    for (feature, p, effect) in &unpaired {
        let row = results.rows.iter().find(|r| r.feature == *feature).unwrap();
        assert_eq!(row.effect_size, *effect, "{feature}");
        changed += usize::from(row.p_value != *p);
    }
    assert!(changed > unpaired.len() / 2, "{changed}");
    harness.screenshot("transform");

    // Multiple groups have the transform too.
    harness.app.set_mode(Mode::MultipleGroups);
    harness.settle();
    let plain: Vec<f64> = harness
        .app
        .session
        .multi
        .output
        .as_ref()
        .unwrap()
        .1
        .rows
        .iter()
        .map(|r| r.p_value)
        .collect();
    harness.app.session.multi.analysis.transform = Transform::CentredLogRatio;
    harness.app.session.invalidate(Mode::MultipleGroups);
    harness.settle();
    let transformed: Vec<f64> = harness
        .app
        .session
        .multi
        .output
        .as_ref()
        .unwrap()
        .1
        .rows
        .iter()
        .map(|r| r.p_value)
        .collect();
    assert_ne!(plain, transformed);
}

#[test]
fn tier_four_plots() {
    use stamp_plot::shared_features::Style;
    let mut harness = coal_with_pairs_and_depths();
    harness.app.session.profile_level = "Genus".to_string();
    harness.app.session.invalidate_all();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.settle();
    let features = harness
        .app
        .session
        .multi
        .output
        .as_ref()
        .unwrap()
        .0
        .profile
        .num_features();
    let samples = harness
        .app
        .session
        .multi
        .output
        .as_ref()
        .unwrap()
        .0
        .profile
        .sample_names
        .len();

    // ANCOM: a point per feature, clicking one selects it.
    harness.app.set_plot_kind(PlotKind::Ancom);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), features);
    let summary = figure_texts(&harness.app)
        .into_iter()
        .find(|t| t.starts_with("ANCOM (ANOVA, alpha = 0.05): "))
        .unwrap();
    assert!(
        summary.ends_with(&format!("of {features} features differ")),
        "{summary}"
    );
    harness.screenshot("ancom");

    // NMDS of the samples.
    harness.app.set_plot_kind(PlotKind::Nmds);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), samples);
    assert!(figure_texts(&harness.app)
        .iter()
        .any(|t| t.starts_with("Stress = ")));
    harness.screenshot("nmds");

    // Shared features: UpSet for four groups; Venn is refused with a reason.
    harness.app.set_plot_kind(PlotKind::SharedFeatures);
    harness.settle();
    assert!(!harness.app.figure().regions.is_empty());
    harness.screenshot("shared_upset");
    let mut settings = harness.app.plot_settings().clone();
    settings.shared.style = Style::Venn;
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert!(figure_texts(&harness.app)[0].contains("two or three groups"));

    // Rank-abundance curves: one per sample, or per group.
    harness.app.set_plot_kind(PlotKind::RankAbundance);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), samples);
    harness.screenshot("rank_abundance");
    settings.rank_abundance.curves = stamp_plot::rank_abundance::Curves::Groups;
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), 4);

    // Two groups: a Venn diagram of the two.
    harness.app.session.set_group_field("Core vs. Water");
    harness.app.set_mode(Mode::TwoGroups);
    harness.app.set_plot_kind(PlotKind::SharedFeatures);
    harness.settle();
    let in_core = {
        let (profile, _) = harness.app.session.groups.output.as_ref().unwrap();
        let first = profile.samples_in_group1().len();
        let data = stamp_plot::composition::Data::from_profile(
            &profile.profile,
            (0..profile.profile.sample_names.len())
                .map(|i| usize::from(i >= first))
                .collect(),
            vec![profile.group_name1.clone(), profile.group_name2.clone()],
        );
        let shared = stamp_plot::shared_features::Data::from_composition(&data, 0.5);
        shared.presence.iter().filter(|p| p[0]).count()
    };
    assert!(figure_texts(&harness.app).contains(&format!("Core ({in_core})")));
    harness.screenshot("shared_venn");
    harness.app.show_plot_config(true);
    harness.settle();
    harness.screenshot("shared_venn_settings");
    harness.app.show_plot_config(false);

    // The Mantel test of community distances against differences in depth.
    harness.app.set_mode(Mode::Correlation);
    harness.app.set_plot_kind(PlotKind::Mantel);
    harness.settle();
    let n = harness
        .app
        .session
        .correlation
        .output
        .as_ref()
        .unwrap()
        .0
        .values
        .len();
    assert_eq!(harness.app.figure().regions.len(), n * (n - 1) / 2);
    let shown = figure_texts(&harness.app);
    assert!(
        shown.iter().any(|t| t.starts_with("Mantel test: rho = ")),
        "{shown:?}"
    );
    assert!(shown.contains(&"Difference in Depth".to_string()));
    harness.screenshot("mantel");
}

#[test]
fn font_sizes_are_set_in_the_preferences_and_per_plot() {
    let mut harness = Harness::enterotypes();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Box);
    harness.settle();
    let size_of = |app: &StampApp, wanted: &str| {
        app.figure()
            .shapes
            .iter()
            .find_map(|s| match s {
                stamp_plot::Shape::Text { text, style, .. } if text == wanted => Some(style.size),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text {wanted}"))
    };
    let feature = harness.app.session.selected_feature.clone().unwrap();
    // Ticks 8 points, axis labels 9, the title 9 to start with.
    assert_eq!(size_of(&harness.app, "Enterotype 1"), 8.0);
    assert_eq!(size_of(&harness.app, "Proportion of sequences (%)"), 9.0);
    assert_eq!(size_of(&harness.app, &feature), 9.0);

    // The preferences change the text every plot shares...
    let mut settings = harness.app.plot_settings().clone();
    settings.fonts.tick = 12.0;
    settings.fonts.axis_label = 14.0;
    settings.fonts.legend = 11.0;
    // ... and the plot's own window its particular text.
    settings.box_plot.title_font_size = 16.0;
    harness.app.set_plot_settings(settings);
    harness.app.show_preferences(true);
    harness.settle();
    assert_eq!(size_of(&harness.app, "Enterotype 1"), 12.0);
    assert_eq!(size_of(&harness.app, "Proportion of sequences (%)"), 14.0);
    assert_eq!(size_of(&harness.app, &feature), 16.0);
    harness.screenshot("preferences_fonts");
    harness.app.show_preferences(false);

    // Legends follow the preference.
    harness.app.set_plot_kind(PlotKind::Pca);
    let mut settings = harness.app.plot_settings().clone();
    settings.pca.legend = stamp_plot::axes::LegendPosition::UpperRight;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert_eq!(size_of(&harness.app, "Enterotype 1"), 11.0);

    // Configure plot has a frame of the plot's own font sizes.
    harness.app.set_plot_kind(PlotKind::Heatmap);
    harness.app.show_plot_config(true);
    harness.settle();
    harness.screenshot("configure_plot_fonts");
}
