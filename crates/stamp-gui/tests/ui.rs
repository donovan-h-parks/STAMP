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

    /// Run frames until the analysis of the current mode, and the plot,
    /// have finished.
    fn settle(&mut self) {
        for _ in 0..2000 {
            let app = &mut self.app;
            self.renderer.run(2, |ui| app.draw(ui));
            if !self.app.is_busy() {
                // A finished analysis may start computing a slow plot.
                let app = &mut self.app;
                self.renderer.run(3, |ui| app.draw(ui));
                if !self.app.is_busy() {
                    return;
                }
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
        harness.app.session.error(Mode::TwoGroups).is_none(),
        "{:?}",
        harness.app.session.error(Mode::TwoGroups)
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
            if kind == PlotKind::Map {
                // The example metadata has no coordinates; the plot says
                // so (`samples_are_placed_on_a_map` uses some).
                let figure = harness.app.figure();
                assert!(
                    matches!(&figure.shapes[0], stamp_plot::Shape::Text { text, .. }
                        if text.contains("latitude or longitude")),
                    "{mode:?}: {:?}",
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
    harness.app.deliver(Event::Profile(vec![file(
        "small.spf",
        "F\tA\tB\nx\t1\t2\ny\t3\t4\n",
    )]));
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
    assert!(
        plot.width() < space.width() && plot.height() < space.height(),
        "{plot:?} in {space:?}"
    );
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

    // PCoA: the panels are chosen separately.
    harness.app.set_plot_kind(PlotKind::Pcoa);
    harness.settle();
    let samples = harness.app.figure().regions.len() / 3;
    harness.screenshot("pcoa_fixed_scale");
    let mut settings = harness.app.plot_settings().clone();
    settings.pcoa.show_pc1_vs_pc3 = false;
    settings.pcoa.fixed_scale = false;
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
    assert!(texts(harness.app.figure()).contains(&"Uncorrected p-value".to_string()));
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
fn singlem_and_sandpiper_profiles_open_from_load_profile() {
    let data = "../stamp-core/tests/data/singlem";
    let ranks = [
        "Domain", "Phylum", "Class", "Order", "Family", "Genus", "Species",
    ];

    // Several Sandpiper files chosen together make one profile.
    let mut harness = Harness::new();
    let runs = ["SRR28772463", "ERR4084076", "SRR28771658"];
    let files = runs
        .iter()
        .map(|run| opened(&format!("{data}/{run}.json")))
        .collect();
    harness.app.deliver(Event::Profile(files));
    harness.settle();
    assert_eq!(harness.app.notice(), None);
    let tree = harness
        .app
        .session
        .tree
        .as_ref()
        .expect("the profile loaded");
    assert_eq!(tree.hierarchy_headings(), ranks);
    assert_eq!(tree.sample_names(), runs);

    // A SingleM condensed profile holds several samples in one file.
    let mut harness = Harness::new();
    harness
        .app
        .open_profile_file(opened(&format!("{data}/singlem_condensed.tsv")));
    assert_eq!(harness.app.pending_profile(), None);
    assert_eq!(harness.app.notice(), None);
    let tree = harness
        .app
        .session
        .tree
        .as_ref()
        .expect("the profile loaded");
    assert_eq!(tree.hierarchy_headings(), ranks);
    assert_eq!(tree.sample_names(), ["SRR28772463", "ERR4084076"]);

    // One Sandpiper file is a single sample, which is not enough; mixing
    // formats is refused.
    let mut harness = Harness::new();
    harness
        .app
        .open_profile_file(opened(&format!("{data}/SRR28772463.json")));
    assert!(harness
        .app
        .notice()
        .unwrap()
        .contains("choose the files of all samples"));
    harness.app.deliver(Event::Profile(vec![
        opened(&format!("{data}/SRR28772463.json")),
        opened(&format!("{data}/singlem_condensed.tsv")),
    ]));
    harness.settle();
    assert!(harness
        .app
        .notice()
        .unwrap()
        .contains("only if they are Sandpiper profiles"));
    assert!(harness.app.session.tree.is_none());
}

#[test]
fn the_sample_filter_restricts_analyses_and_plots() {
    let mut harness = Harness::enterotypes();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.settle();
    let before = harness
        .app
        .session
        .multi
        .output
        .as_ref()
        .unwrap()
        .0
        .group_names
        .clone();
    assert!(before.len() >= 3);

    let japanese = ["japanese".to_string()];
    harness
        .app
        .set_sample_filter(Some(("Nationality", &japanese)));
    harness.settle();
    harness.screenshot("sample_filter");
    assert_eq!(harness.app.notice(), None);
    let metadata = harness.app.session.metadata.as_ref().unwrap();
    assert_eq!(metadata.num_selected(), 13);
    assert_eq!(metadata.active_samples().len(), 13);
    let (profile, _) = harness.app.session.multi.output.as_ref().unwrap();
    assert_eq!(
        profile.group_names,
        ["Enterotype 1", "Enterotype 3", "Infant"]
    );
    assert_eq!(profile.profile.sample_names.len(), 13);
    // The two-sample lists offer only the samples kept.
    harness.app.set_mode(Mode::TwoSamples);
    harness.settle();
    let s1 = harness.app.session.samples.sample1.clone();
    assert_eq!(
        harness
            .app
            .session
            .metadata
            .as_ref()
            .unwrap()
            .value(&s1, "Nationality"),
        Some("japanese")
    );

    // A misspelled field is reported; clearing the filter returns every
    // sample, and new metadata starts without one.
    harness.app.set_sample_filter(Some(("Nope", &japanese)));
    assert!(harness
        .app
        .notice()
        .unwrap()
        .contains("Unknown metadata field"));
    harness.app.set_sample_filter(None);
    harness.app.set_mode(Mode::MultipleGroups);
    harness.settle();
    let (profile, _) = harness.app.session.multi.output.as_ref().unwrap();
    assert_eq!(profile.group_names, before);
    harness
        .app
        .set_sample_filter(Some(("Nationality", &japanese)));
    harness.app.load_metadata(
        "Enterotypes.metadata.tsv",
        &example("EnterotypesArumugam/Enterotypes.metadata.tsv"),
    );
    assert_eq!(harness.app.session.sample_filter(), None);
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
    settings.pcoa.fixed_scale = false;
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
    use stamp_core::stats::correction::Correction;
    // The label is drawn text; its pixels show which one is in use.
    let mut harness = Harness::enterotypes();
    harness.settle();
    let label = |harness: &mut Harness, correction, name| {
        harness
            .app
            .session
            .set_correction(Mode::TwoGroups, correction);
        harness.settle();
        let shot = harness.screenshot(name);
        // The label's place in the Properties frame, wherever that is.
        let layout = harness.app.layout();
        let tile = stamp_gui::layout::tile_of(layout, stamp_gui::layout::Frame::Properties)
            .expect("the Properties frame");
        let frame = layout.tiles.rect(tile).expect("the frame was laid out");
        let (left, top) = (frame.min.x as usize, frame.min.y as usize);
        (left + 42..left + 140)
            .flat_map(|x| (top + 358..top + 380).map(move |y| (x, y)))
            .map(|(x, y)| shot.pixel(x, y))
            .collect::<Vec<_>>()
    };
    let none = label(&mut harness, Correction::None, "p_value_filter_label");
    let bonferroni = label(
        &mut harness,
        Correction::Bonferroni,
        "bonferroni_filter_label",
    );
    let bh = label(
        &mut harness,
        Correction::BenjaminiHochberg,
        "q_value_filter_label",
    );
    let storey = label(&mut harness, Correction::Storey, "storey_filter_label");
    // "p-value filter" with no correction and under Bonferroni; "q-value
    // filter" under both false discovery rates.
    assert!(
        none == bonferroni,
        "the filter label changed under Bonferroni"
    );
    assert!(
        none != bh,
        "the filter label did not change under Benjamini-Hochberg"
    );
    assert!(
        bh == storey,
        "the filter labels of the two FDR methods differ"
    );
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
        settings.pcoa.legend,
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
        // No correction: the one value, named as uncorrected.
        "Uncorrected p-value: 0.165",
    ] {
        assert!(
            groups.contains(expected),
            "missing {expected:?} in {groups:?}"
        );
    }
    assert!(!groups.contains("Corrected p-value"), "{groups:?}");
    // Under Storey's FDR, the q-value follows.
    harness.app.session.set_correction(
        Mode::TwoGroups,
        stamp_core::stats::correction::Correction::Storey,
    );
    harness.settle();
    let storey = tooltip(&harness.app);
    assert!(
        storey.contains("Uncorrected p-value: 0.165\nq-value: "),
        "{storey:?}"
    );

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
        "Uncorrected p-value: ".to_string(),
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
        [
            "Enterotypes",
            "Coal seam gas",
            "Eggerthellaceae (genera)",
            "Eggerthellaceae (focus species)"
        ]
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
fn eggerthellaceae_examples_load_with_their_own_settings() {
    use stamp_gui::app::Example;
    let mut harness = Harness::new();
    harness
        .app
        .load_example_data(Example::EggerthellaceaeGenera);
    harness.settle();
    assert_eq!(harness.app.notice(), None);
    let session = &harness.app.session;
    let tree = session.tree.as_ref().expect("the genus profile loaded");
    assert_eq!(tree.num_samples(), 1289);
    assert_eq!(tree.hierarchy_headings().len(), 6);
    // The same data as the file in the repository.
    let from_file =
        stamp_core::ProfileTree::read(&example("Eggerthellaceae/eggerthellaceae.spf")).unwrap();
    assert_eq!(tree.num_leaves(), from_file.num_leaves());
    assert_eq!(
        tree.sample_total("SAMD01733366"),
        from_file.sample_total("SAMD01733366")
    );
    // Its own starting settings: genera, compared across hosts.
    assert_eq!(session.profile_level, "Genus");
    assert_eq!(session.group_field.as_deref(), Some("Host_group"));
    assert_eq!(harness.app.mode, Mode::MultipleGroups);
    assert_eq!(session.multi.active_groups.len(), 13);
    assert_eq!(session.groups.group1, "Domestic horse");
    assert_eq!(
        session.groups.group2.as_deref(),
        Some("Feral horse (Sable Island)")
    );
    assert_eq!(session.correlation.field.as_deref(), Some("Equivita_relab"));
    assert!(session.error(Mode::MultipleGroups).is_none());
    // The multiple-group test ran over the genera (unclassified rows are
    // features too, as unclassified reads are retained by default).
    let (total, passing) = session.feature_counts(Mode::MultipleGroups);
    assert_eq!(total, 544, "{total} features");
    assert!(passing > 0 && passing < total);
    assert!(!harness.app.results_table().rows.is_empty());

    // The sample heatmap refuses this many samples and points to the
    // group heatmap, which draws them.
    harness.app.set_plot_kind(PlotKind::Heatmap);
    harness.settle();
    let texts = figure_texts(&harness.app);
    assert!(
        texts[0].contains("limited to 1000 samples, and there are 1289"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t.contains("group heatmap")));
    harness.app.set_plot_kind(PlotKind::GroupHeatmap);
    harness.settle();
    assert!(harness.app.figure().shapes.len() > 100);
    harness.screenshot("eggerthellaceae_group_heatmap");
    // Coverage is not counts, so there is nothing to rarefy.
    harness.app.set_plot_kind(PlotKind::Rarefaction);
    harness.settle();
    assert_eq!(figure_texts(&harness.app).len(), 1);

    harness
        .app
        .load_example_data(Example::EggerthellaceaeSpecies);
    harness.settle();
    assert_eq!(harness.app.notice(), None);
    let session = &harness.app.session;
    assert_eq!(session.profile_name, "eggerthellaceae_mgr_species.spf");
    assert_eq!(session.tree.as_ref().unwrap().num_samples(), 1289);
    assert_eq!(session.profile_level, "Species");
    assert_eq!(
        session.unclassified,
        stamp_core::Unclassified::FrequencyOnly
    );
    assert_eq!(harness.app.mode, Mode::MultipleGroups);
    // The named species only: the rest of the community and the coverage
    // placed only in a genus are not features.
    assert_eq!(session.feature_counts(Mode::MultipleGroups).0, 15);
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
    old.plots.pcoa.legend = LegendPosition::UpperRight;
    old.plots.bar.legend = LegendPosition::None;
    old.zoom = 2.5;
    let mut stored: serde_json::Value = serde_json::to_value(&old).unwrap();
    stored.as_object_mut().unwrap().remove("defaults_version");
    // Settings of a plot that no longer exists (the PCA plot) are ignored.
    let pcoa = stored["plots"]["pcoa"].clone();
    stored["plots"]
        .as_object_mut()
        .unwrap()
        .insert("pca".to_string(), pcoa);
    let read: Saved = serde_json::from_value(stored).unwrap();
    assert_eq!(read.defaults_version, 0);

    // The plots start from the current defaults; the rest is kept.
    let current = read.up_to_date();
    assert_eq!(current.defaults_version, DEFAULTS_VERSION);
    assert_eq!(current.plots.pcoa.legend, LegendPosition::Best);
    assert_eq!(current.plots.bar.legend, LegendPosition::Best);
    assert_eq!(current.zoom, 2.5);

    // Settings saved by this version are restored as they are.
    let mut chosen = harness.app.saved(&harness.renderer.ctx);
    chosen.plots.pcoa.legend = LegendPosition::LowerLeft;
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
    harness.app.set_plot_kind(PlotKind::Nmds);
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
    // Euclidean distances between proportions: what a PCA would show.
    settings.community_distance = Distance::Euclidean;
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    let shown = texts(&harness.app);
    assert!(shown.contains(&"Euclidean distance".to_string()));
    assert!(shown.iter().any(|t| t.starts_with("PCo1 (")));

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
    harness.app.set_plot_kind(PlotKind::Pcoa);
    let mut settings = harness.app.plot_settings().clone();
    settings.pcoa.legend = stamp_plot::axes::LegendPosition::UpperRight;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert_eq!(size_of(&harness.app, "Enterotype 1"), 11.0);

    // Arial is the font to start with; the preferences change it.
    assert_eq!(harness.app.figure().font, stamp_plot::text::PlotFont::Arial);
    let mut settings = harness.app.plot_settings().clone();
    settings.fonts.family = stamp_plot::text::PlotFont::TimesNewRoman;
    harness.app.set_plot_settings(settings);
    harness.app.show_preferences(true);
    harness.settle();
    assert_eq!(
        harness.app.figure().font,
        stamp_plot::text::PlotFont::TimesNewRoman
    );
    harness.screenshot("preferences_font_times");
    harness.app.show_preferences(false);

    // Configure plot has a frame of the plot's own font sizes.
    harness.app.set_plot_kind(PlotKind::Heatmap);
    harness.app.show_plot_config(true);
    harness.settle();
    harness.screenshot("configure_plot_fonts");
}

#[test]
fn the_plot_selector_lists_plots_by_group() {
    let mut harness = coal_with_pairs_and_depths();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.settle();
    let closed = harness.screenshot("plot_selector_closed");
    // The selector beneath the plot, at the left of the plot bar.
    let app = &mut harness.app;
    harness
        .renderer
        .click(egui::pos2(320.0, 712.0), |ui| app.draw(ui));
    let app = &mut harness.app;
    harness.renderer.run(30, |ui| app.draw(ui));
    let open = harness.screenshot("plot_selector_open");
    // The list opens over the plot.
    assert!(
        (0..closed.pixels.len()).any(|i| closed.pixels[i] != open.pixels[i]),
        "the selector did not open"
    );
    // The default plot is still the first offered: the PCoA.
    assert_eq!(harness.app.plot_kind(), PlotKind::Pcoa);
}

#[test]
fn plots_already_drawn_are_reused_until_something_changes() {
    let mut harness = coal_with_pairs_and_depths();
    harness.app.session.profile_level = "Genus".to_string();
    harness.app.session.invalidate_all();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Ancom);
    harness.settle();
    let ancom = harness.app.figure().clone();
    let built = harness.app.figures_built();

    // ANCOM does not depend on the selected feature: it is not redrawn.
    let other = harness.app.feature_table()[3].feature.clone();
    harness.app.select_feature(&other);
    harness.settle();
    assert_eq!(harness.app.figures_built(), built);
    assert_eq!(harness.app.figure().shapes.len(), ancom.shapes.len());

    // The box plot shows the selected feature: it is drawn for each one.
    harness.app.set_plot_kind(PlotKind::Box);
    harness.settle();
    let box_built = harness.app.figures_built();
    assert_eq!(box_built, built + 1);
    let first = harness.app.feature_table()[1].feature.clone();
    harness.app.select_feature(&first);
    harness.settle();
    assert_eq!(harness.app.figures_built(), box_built + 1);

    // Back to ANCOM: taken from the cache.
    harness.app.set_plot_kind(PlotKind::Ancom);
    harness.settle();
    assert_eq!(harness.app.figures_built(), box_built + 1);
    assert_eq!(harness.app.figure().regions.len(), ancom.regions.len());

    // A filter, a colour or a setting draws it again.
    harness.app.set_p_value_filter(Some(0.05));
    harness.settle();
    let after_filter = harness.app.figures_built();
    assert!(after_filter > box_built + 1);
    harness
        .app
        .session
        .set_group_color("Core", stamp_plot::Color::rgb(1, 2, 3));
    harness
        .app
        .set_plot_settings(harness.app.plot_settings().clone());
    harness.settle();
    assert!(harness.app.figures_built() > after_filter);
    let mut settings = harness.app.plot_settings().clone();
    settings.ancom.marker_size += 5.0;
    let before = harness.app.figures_built();
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert_eq!(harness.app.figures_built(), before + 1);
}

/// Text is placed by the font the figure was built in, not by the default:
/// right-aligned text ends at its anchor and a capital H set to the middle
/// is centred on it, in every font.
#[test]
fn plot_text_is_placed_by_its_own_font() {
    use stamp_plot::figure::{HAlign, TextStyle, VAlign};
    use stamp_plot::text::PlotFont;
    for font in PlotFont::ALL {
        let fonts = stamp_plot::axes::Fonts {
            family: font,
            ..Default::default()
        };
        let figure = stamp_plot::axes::with_fonts(fonts, || {
            let mut figure = stamp_plot::Figure::new(300.0, 120.0);
            figure.text(
                250.0,
                60.0,
                "HHHH",
                TextStyle::new(40.0).align(HAlign::Right, VAlign::Middle),
            );
            figure
        });
        assert_eq!(figure.font, font);
        let mut renderer = Renderer::new(300.0, 120.0, 1.0);
        stamp_gui::figure_painter::install_plot_font(&renderer.ctx);
        let paint = |ui: &mut egui::Ui| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    stamp_gui::figure_painter::show(ui, &figure, 1.0, egui::vec2(300.0, 120.0));
                });
        };
        renderer.run(3, paint);
        let image = renderer.render(paint);
        // The ink of the text: dark pixels.
        let dark: Vec<(usize, usize)> = (0..image.height)
            .flat_map(|y| (0..image.width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let p = image.pixel(x, y);
                (p.r() as u32 + p.g() as u32 + p.b() as u32) < 200
            })
            .collect();
        assert!(dark.len() > 200, "{font:?}: no text drawn");
        let right = dark.iter().map(|p| p.0).max().unwrap() as f64;
        let top = dark.iter().map(|p| p.1).min().unwrap() as f64;
        let bottom = dark.iter().map(|p| p.1).max().unwrap() as f64;
        // H has a little side bearing: its ink ends just short of the
        // advance width.
        assert!(
            (243.0..=251.0).contains(&right),
            "{font:?}: text ends at {right}, not at 250"
        );
        let middle = (top + bottom) / 2.0;
        assert!(
            (middle - 60.0).abs() <= 2.0,
            "{font:?}: capitals centred at {middle}, not at 60"
        );
    }
}

#[test]
fn group_heatmap_summarises_groups() {
    use stamp_core::stats::group_summary::GroupStatistic;
    use stamp_plot::group_heatmap::{ColumnOrder, Scale};
    let mut harness = Harness::new();
    harness
        .app
        .load_example_data(stamp_gui::app::Example::CoalSeamGas);
    harness.app.set_p_value_filter(None);
    harness.app.session.profile_level = "Genus".to_string();
    harness.app.session.set_group_field("Environment");
    harness.app.session.invalidate_all();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::GroupHeatmap);
    harness.settle();
    let shown = figure_texts(&harness.app);
    // A column per group, named with its number of samples.
    for group in ["Core (n=8)", "Deep cutting (n=10)", "Produced water (n=15)"] {
        assert!(
            shown.contains(&group.to_string()),
            "{group} not in {shown:?}"
        );
    }
    assert!(shown.contains(&"Mean proportion (%)".to_string()));
    assert!(shown
        .iter()
        .any(|t| t.starts_with("The 50 features of highest mean abundance, of")));

    // Prevalence on a log scale, groups ordered by a coarser field.
    let mut settings = harness.app.plot_settings().clone();
    settings.group_heatmap_summary.statistic = GroupStatistic::Prevalence;
    settings.group_heatmap_summary.max_features = Some(20);
    settings.group_heatmap.scale = Scale::Log10;
    settings.group_heatmap.column_order = ColumnOrder::Category;
    settings.group_heatmap.show_values = true;
    settings.group_heatmap_category_field = Some("Core vs. Water".to_string());
    harness.app.set_plot_settings(settings.clone());
    harness.app.show_plot_config(true);
    harness.settle();
    let shown = figure_texts(&harness.app);
    assert!(shown.contains(&"Prevalence (% of samples) (log scale)".to_string()));
    assert!(shown.contains(&"Core".to_string()));
    // Columns by category: the three Core groups, then Produced water.
    let x_of = |name: &str| {
        harness
            .app
            .figure()
            .shapes
            .iter()
            .find_map(|s| match s {
                stamp_plot::Shape::Text { text, x, .. } if text.starts_with(name) => Some(*x),
                _ => None,
            })
            .unwrap()
    };
    assert!(x_of("Shallow cutting") < x_of("Produced water"));
    assert!(x_of("Core (n=") < x_of("Deep cutting"));
    harness.screenshot("group_heatmap_settings");

    // The sample filter reaches the summaries.
    let water = ["Produced water".to_string()];
    harness
        .app
        .set_sample_filter(Some(("Core vs. Water", &["Core".to_string()])));
    harness.settle();
    let shown = figure_texts(&harness.app);
    assert!(!shown.iter().any(|t| t.starts_with("Produced water (n=")));
    assert!(shown.contains(&"Core (n=8)".to_string()));
    harness
        .app
        .set_sample_filter(Some(("Core vs. Water", &water)));
    harness.settle();
    assert!(figure_texts(&harness.app)
        .iter()
        .any(|t| t.contains("at least two groups") || t.starts_with("Produced water (n=15)")));
    harness.app.set_sample_filter(None);

    // Two groups.
    harness.app.set_mode(Mode::TwoGroups);
    harness.app.set_plot_kind(PlotKind::GroupHeatmap);
    harness.settle();
    assert!(harness.app.figure().regions.len() >= 2);
}

#[test]
fn co_occurrence_heatmap_and_one_feature() {
    use stamp_plot::co_occurrence_plot::View;
    let mut harness = Harness::new();
    harness
        .app
        .load_example_data(stamp_gui::app::Example::CoalSeamGas);
    harness.app.set_p_value_filter(None);
    harness.app.session.profile_level = "Genus".to_string();
    harness.app.session.set_group_field("Environment");
    harness.app.session.invalidate_all();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::CoOccurrence);
    harness.settle();
    let shown = figure_texts(&harness.app);
    assert!(shown.contains(&"Spearman rho (proportions)".to_string()));
    assert!(shown
        .iter()
        .any(|t| t.starts_with("44 samples; 30 features of highest mean abundance")));
    // 30 x 30 cells less the diagonal, each with a tooltip.
    assert_eq!(harness.app.figure().regions.len(), 30 * 29);

    let mut settings = harness.app.plot_settings().clone();
    settings.co_occurrence.clr = true;
    settings.co_occurrence.top = 10;
    settings.co_occurrence_features = "g__Methanobacterium\n\n".to_string();
    settings.co_occurrence_group = Some("Produced water".to_string());
    harness.app.set_plot_settings(settings.clone());
    harness.app.show_plot_config(true);
    harness.settle();
    let shown = figure_texts(&harness.app);
    assert!(shown.contains(&"Spearman rho (centred log ratios)".to_string()));
    assert!(
        shown
            .iter()
            .any(|t| t.starts_with("15 samples; 10 features")),
        "{shown:?}"
    );
    assert!(shown.contains(&"g__Methanobacterium".to_string()));
    harness.screenshot("co_occurrence_settings");

    // A misspelt feature is reported in the plot.
    settings.co_occurrence_features = "g__Nothing".to_string();
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert!(figure_texts(&harness.app).contains(&"Unknown feature 'g__Nothing'".to_string()));

    // The selected feature against every other.
    settings.co_occurrence_features.clear();
    settings.co_occurrence.view = View::OneFeature;
    harness.app.set_plot_settings(settings);
    harness.app.show_plot_config(false);
    harness.app.select_feature("g__Methanobacterium");
    harness.settle();
    let shown = figure_texts(&harness.app);
    assert!(shown.contains(&"Correlations with g__Methanobacterium".to_string()));
    harness.screenshot("co_occurrence_one_feature");

    // Correlation analyses offer it too.
    let mut harness = coal_with_pairs_and_depths();
    harness.app.set_mode(Mode::Correlation);
    harness.app.set_plot_kind(PlotKind::CoOccurrence);
    harness.settle();
    assert!(figure_texts(&harness.app)
        .iter()
        .any(|t| t.starts_with("Spearman rho")));
}

/// The settings of the group and co-occurrence heatmaps, drawn on their
/// own so that they can be looked at.
#[test]
fn heatmap_settings_dialogs() {
    use stamp_gui::plot_config::{show, Context};
    use stamp_gui::plots::PlotSettings;
    let context = Context {
        fields: vec!["Host_species".into(), "Host_order".into()],
        groups: vec!["Horse".into(), "Tapir".into()],
        ..Context::default()
    };
    for (kind, name, height) in [
        (PlotKind::GroupHeatmap, "dialog_group_heatmap", 1150.0),
        (PlotKind::CoOccurrence, "dialog_co_occurrence", 1050.0),
    ] {
        let mut renderer = Renderer::new(420.0, height, 1.0);
        let mut settings = PlotSettings::default();
        let image = renderer.render(|ui| {
            egui::Frame::window(ui.style()).show(ui, |ui| {
                show(ui, Mode::MultipleGroups, kind, &mut settings, &context);
            });
        });
        assert!(ink(&image, 0, 0, 420, 600) > 0.05);
        if let Ok(directory) = std::env::var("STAMP_SCREENSHOTS") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                PathBuf::from(directory).join(format!("{name}.png")),
                image.to_png(),
            )
            .unwrap();
        }
    }
}

#[test]
fn box_plot_of_several_features_and_the_prevalence_plot() {
    use stamp_gui::plots::BoxFeatures;
    use stamp_plot::feature_plots::{BoxLayout, GroupOrder};
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
    let mut harness = Harness::enterotypes();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Box);
    harness.settle();
    // Phyla, the profile level of these tests.
    harness.app.select_feature("Bacteroidetes");
    harness.settle();
    // One box per group (the three enterotypes and the unclassified).
    let groups = harness.app.figure().regions.len();
    assert!(groups >= 3);

    // Ctrl+click adds features to the list the box plot then shows, and a
    // second time removes them.
    harness.app.toggle_box_feature("Bacteroidetes");
    harness.app.toggle_box_feature("Firmicutes");
    harness.app.toggle_box_feature("Proteobacteria");
    harness.settle();
    assert_eq!(
        harness.app.plot_settings().box_features,
        BoxFeatures::Listed
    );
    assert_eq!(harness.app.figure().regions.len(), 3 * groups);
    harness.app.toggle_box_feature("Proteobacteria");
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), 2 * groups);
    assert_eq!(
        harness.app.plot_settings().box_feature_list,
        "Bacteroidetes\nFirmicutes"
    );

    // Log axis, points, counts, panels, ordered by a field.
    let mut settings = harness.app.plot_settings().clone();
    settings.box_plot.log_scale = true;
    settings.box_plot.show_points = true;
    settings.box_plot.show_counts = true;
    settings.box_plot.layout = BoxLayout::Panels;
    settings.box_plot.group_order = GroupOrder::Category;
    settings.box_category_field = Some("Enterotype".into());
    harness.app.set_plot_settings(settings);
    harness.settle();
    let figure = harness.app.figure().clone();
    assert_eq!(figure.regions.len(), 2 * groups);
    let shown = texts(&figure);
    assert!(shown.iter().any(|t| t == "0.1"), "{shown:?}");
    assert!(shown.iter().any(|t| t.starts_with("Enterotype 1 (n=")));
    assert!(figure
        .shapes
        .iter()
        .any(|s| matches!(s, stamp_plot::Shape::Circle { stroke: None, .. })));
    harness.screenshot("box_plot_several_features");

    // The most significant features.
    let mut settings = harness.app.plot_settings().clone();
    settings.box_features = BoxFeatures::MostSignificant;
    settings.box_feature_count = 4;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), 4 * groups);

    // Prevalence against mean abundance: a panel per group, the selected
    // feature highlighted and named.
    harness.app.set_plot_kind(PlotKind::PrevalenceAbundance);
    harness.settle();
    let figure = harness.app.figure().clone();
    assert!(!figure.regions.is_empty());
    let shown = texts(&figure);
    assert!(shown.iter().any(|t| t == "Bacteroidetes"), "{shown:?}");
    assert!(shown.iter().any(|t| t == "Mean relative abundance (%)"));
    assert!(figure
        .regions
        .iter()
        .any(|r| r.item == "Bacteroidetes" && r.tooltip.contains("present in")));
    harness.screenshot("prevalence_abundance");
    let mut settings = harness.app.plot_settings().clone();
    settings.prevalence.view = stamp_plot::prevalence_plot::View::Specificity;
    settings.prevalence_highlight_selected = false;
    harness.app.set_plot_settings(settings);
    harness.settle();
    let shown = texts(harness.app.figure());
    assert!(!shown.iter().any(|t| t == "Bacteroidetes"));
    assert!(shown.iter().any(|t| t.starts_with("Shared (specificity")));

    // Two groups too.
    harness.app.set_mode(Mode::TwoGroups);
    harness.app.set_plot_kind(PlotKind::PrevalenceAbundance);
    harness.settle();
    assert!(!harness.app.figure().regions.is_empty());
}

#[test]
fn box_and_prevalence_settings_dialogs() {
    use stamp_gui::plot_config::{show, Context};
    use stamp_gui::plots::{BoxFeatures, PlotSettings};
    let context = Context {
        fields: vec!["Host_group".into(), "Host_order".into()],
        groups: vec!["Horse".into(), "Tapir".into()],
        ..Context::default()
    };
    for (kind, name, height) in [
        (PlotKind::Box, "dialog_box_plot", 1100.0),
        (
            PlotKind::PrevalenceAbundance,
            "dialog_prevalence_abundance",
            1050.0,
        ),
    ] {
        let mut renderer = Renderer::new(420.0, height, 1.0);
        let mut settings = PlotSettings {
            box_features: BoxFeatures::Listed,
            box_feature_list: "Equivita\nTapirivita\nGorillivita".into(),
            ..PlotSettings::default()
        };
        let image = renderer.render(|ui| {
            egui::Frame::window(ui.style()).show(ui, |ui| {
                show(ui, Mode::MultipleGroups, kind, &mut settings, &context);
            });
        });
        assert!(ink(&image, 0, 0, 420, 600) > 0.05);
        if let Ok(directory) = std::env::var("STAMP_SCREENSHOTS") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                PathBuf::from(directory).join(format!("{name}.png")),
                image.to_png(),
            )
            .unwrap();
        }
    }
}

/// The ordinations (PCoA with its community tests, NMDS) are computed
/// once for their data and settings: changing only how the plot looks
/// (colours, shapes, marker size, opacity, legend, fonts) draws it again
/// from the kept layout.
#[test]
fn styling_an_ordination_does_not_compute_it_again() {
    use stamp_plot::ordination_style::ColorSource;
    let mut harness = coal_with_pairs_and_depths();
    harness.app.session.set_group_field("Environment");
    harness.app.session.invalidate_all();
    harness.app.set_mode(Mode::MultipleGroups);
    for kind in [PlotKind::Pcoa, PlotKind::Nmds] {
        harness.app.set_plot_kind(kind);
        harness.settle();
        let computed = harness.app.ordinations_computed();
        let built = harness.app.figures_built();
        let first = harness.app.figure().clone();
        assert!(!first.regions.is_empty(), "{kind:?}: no samples drawn");
        // The results table, slow to build for large profiles, is kept too.
        let table = harness.app.results_table().rows.as_ptr();
        assert!(!harness.app.results_table().rows.is_empty());

        let mut settings = harness.app.plot_settings().clone();
        for options in [&mut settings.pcoa, &mut settings.nmds] {
            options.color_by = ColorSource::Field("Depth".to_string());
            options.shape_by = Some("Company".to_string());
            options.marker_size *= 2.0;
            options.opacity = 0.5;
            options.legend = stamp_plot::axes::LegendPosition::Best;
            options.note_font_size += 1.0;
        }
        settings.fonts.tick += 1.0;
        harness.app.set_plot_settings(settings.clone());
        harness.settle();
        // Drawn again, differently, but not computed again.
        assert!(harness.app.figures_built() > built, "{kind:?}");
        assert_ne!(harness.app.figure().shapes, first.shapes, "{kind:?}");
        assert_eq!(harness.app.ordinations_computed(), computed, "{kind:?}");
        assert_eq!(harness.app.results_table().rows.as_ptr(), table, "{kind:?}");
        // The samples are where they were.
        let centres = |figure: &stamp_plot::Figure| -> Vec<String> {
            figure.regions.iter().map(|r| r.tooltip.clone()).collect()
        };
        assert_eq!(
            centres(harness.app.figure()).len(),
            centres(&first).len(),
            "{kind:?}"
        );

        // A change of what it is computed from computes it again.
        settings.community_distance = stamp_core::stats::diversity::Distance::Jaccard;
        harness.app.set_plot_settings(settings.clone());
        harness.settle();
        assert_eq!(harness.app.ordinations_computed(), computed + 1, "{kind:?}");
        settings.community_distance = stamp_core::stats::diversity::Distance::BrayCurtis;
        if kind == PlotKind::Pcoa {
            settings.community_permutations += 1;
            harness.app.set_plot_settings(settings.clone());
            harness.settle();
            assert_eq!(harness.app.ordinations_computed(), computed + 2);
        }
        // Back to the first look: from the kept layout again.
        let mut plain = settings.clone();
        plain.pcoa = Default::default();
        plain.nmds = Default::default();
        let before = harness.app.ordinations_computed();
        harness.app.set_plot_settings(plain);
        harness.settle();
        assert_eq!(harness.app.ordinations_computed(), before, "{kind:?}");
    }
}

/// PCoA and NMDS plots coloured by a metadata field (a number or a
/// category) or by the abundance of a feature, and shaped by another field.
#[test]
fn ordinations_are_coloured_and_shaped_by_other_fields() {
    use stamp_plot::ordination_style::{ColorScale, ColorSource};
    let mut harness = coal_with_pairs_and_depths();
    harness.app.session.set_group_field("Environment");
    harness.app.session.invalidate_all();
    harness.app.show_feature_table(false);
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Pcoa);
    harness.settle();
    // Coloured by group: the legend inside the frame names the groups.
    let plain = figure_texts(&harness.app);
    assert!(plain.contains(&"Core".to_string()));
    let samples = harness.app.figure().regions.len() / 3;

    // By a number: a colour bar titled with the field; the sample whose
    // depth is "unknown" is drawn hollow and counted.
    let mut settings = harness.app.plot_settings().clone();
    settings.pcoa.color_by = ColorSource::Field("Depth".to_string());
    settings.pcoa.shape_by = Some("Company".to_string());
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    let shown = figure_texts(&harness.app);
    for wanted in ["Depth", "No value (1)", "Company"] {
        assert!(
            shown.contains(&wanted.to_string()),
            "{wanted} not in {shown:?}"
        );
    }
    let tooltip = &harness.app.figure().regions[0].tooltip;
    assert!(
        tooltip.contains("Depth: ") && tooltip.contains("Company: "),
        "{tooltip}"
    );
    harness.screenshot("pcoa_coloured_by_depth");

    // The same field drawn by category.
    settings.pcoa.color_by = ColorSource::Category("Depth".to_string());
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert!(figure_texts(&harness.app)
        .iter()
        .any(|t| t.starts_with("10.0 (")));

    // By the abundance of the selected feature, on a log scale; changing
    // the selection changes the plot.
    let features: Vec<String> = harness
        .app
        .session
        .multi
        .output
        .as_ref()
        .unwrap()
        .1
        .rows
        .iter()
        .map(|r| r.feature.clone())
        .collect();
    harness.app.select_feature(&features[0]);
    settings.pcoa.color_by = ColorSource::SelectedFeature;
    settings.pcoa.color_scale = ColorScale::Log10;
    settings.pcoa.shape_by = None;
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    let first = format!("{} (%)", features[0]);
    assert!(
        figure_texts(&harness.app).contains(&first),
        "{:?}",
        figure_texts(&harness.app)
    );
    harness.app.select_feature(&features[1]);
    harness.settle();
    let second = format!("{} (%)", features[1]);
    assert!(figure_texts(&harness.app).contains(&second));
    // Coloured by abundance, the groups keep their shapes in the legend.
    assert!(figure_texts(&harness.app)
        .iter()
        .any(|t| t.starts_with("Core (")));

    // A feature by name, and a name that is not a feature.
    settings.pcoa.color_by = ColorSource::Feature(features[2].clone());
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert_eq!(harness.app.figure().regions.len(), 3 * samples);
    settings.pcoa.color_by = ColorSource::Feature("No such feature".to_string());
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert!(figure_texts(&harness.app)[0].contains("unknown feature"));

    // The community tests still compare the groups.
    settings.pcoa.color_by = ColorSource::Field("Company".to_string());
    settings.pcoa.show_pc3_vs_pc2 = false;
    settings.pcoa.show_pc1_vs_pc3 = false;
    harness.app.set_plot_settings(settings.clone());
    harness.app.set_plot_kind(PlotKind::Pcoa);
    harness.settle();
    let shown = figure_texts(&harness.app);
    assert!(shown
        .iter()
        .any(|t| t == "The tests compare the groups of Environment, not the colours"));
    assert!(shown.contains(&"Company".to_string()));
    assert_eq!(harness.app.figure().regions.len(), samples);
    harness.screenshot("pcoa_coloured_by_company");

    // NMDS too.
    settings.nmds.color_by = ColorSource::Field("Depth".to_string());
    harness.app.set_plot_settings(settings);
    harness.app.set_plot_kind(PlotKind::Nmds);
    harness.settle();
    assert!(figure_texts(&harness.app).contains(&"Depth".to_string()));
}

/// The colour and shape settings of the ordination plots, drawn on their
/// own so that they can be looked at.
#[test]
fn ordination_settings_dialogs() {
    use stamp_gui::plot_config::{show, Context};
    use stamp_gui::plots::PlotSettings;
    use stamp_plot::ordination_style::ColorSource;
    let context = Context {
        fields: vec![
            "Host_group".into(),
            "Host_order".into(),
            "Captivity".into(),
            "Latitude".into(),
        ],
        groups: vec!["Horse".into(), "Tapir".into()],
        numeric_fields: vec!["Latitude".into()],
        ..Context::default()
    };
    for (kind, name, height) in [
        (PlotKind::Nmds, "dialog_nmds", 1000.0),
        (PlotKind::Pcoa, "dialog_pcoa", 1100.0),
    ] {
        let mut renderer = Renderer::new(420.0, height, 1.0);
        let mut settings = PlotSettings::default();
        settings.nmds.color_by = ColorSource::Field("Latitude".into());
        settings.pcoa.color_by = ColorSource::Feature("Equivita".into());
        settings.pcoa.shape_by = Some("Captivity".into());
        let image = renderer.render(|ui| {
            egui::Frame::window(ui.style()).show(ui, |ui| {
                show(ui, Mode::MultipleGroups, kind, &mut settings, &context);
            });
        });
        assert!(ink(&image, 0, 0, 420, 600) > 0.05);
        // Showing the controls changes nothing.
        assert_eq!(
            settings.nmds.color_by,
            ColorSource::Field("Latitude".into())
        );
        assert_eq!(
            settings.pcoa.color_by,
            ColorSource::Feature("Equivita".into())
        );
        if let Ok(directory) = std::env::var("STAMP_SCREENSHOTS") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                PathBuf::from(directory).join(format!("{name}.png")),
                image.to_png(),
            )
            .unwrap();
        }
    }
}

/// A plot that takes a while to compute is computed in the background: the
/// window stays usable, the plot area shows what is being done and how far
/// it has got (after a moment, so that quick plots do not flicker), the
/// calculation can be cancelled, and the plot is drawn once it is done.
#[test]
fn a_slow_plot_shows_its_progress_and_can_be_cancelled() {
    let mut harness = coal_with_pairs_and_depths();
    harness.app.session.set_group_field("Environment");
    harness.app.session.invalidate_all();
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Composition);
    harness.settle();
    assert!(!harness.app.figure().regions.is_empty());

    // So many permutations that the tests take far longer than the test.
    let mut settings = harness.app.plot_settings().clone();
    settings.community_permutations = 50_000_000;
    harness.app.set_plot_settings(settings.clone());
    harness.app.set_plot_kind(PlotKind::Pcoa);
    let app = &mut harness.app;
    // A few frames: too soon for the progress to be shown.
    harness.renderer.run(3, |ui| app.draw(ui));
    assert!(harness.app.is_busy());
    let early = harness.screenshot("slow_plot_starting");
    // Long enough (in the time of the interface) for it to be shown.
    let app = &mut harness.app;
    harness.renderer.run(30, |ui| app.draw(ui));
    let (stage, fraction) = harness
        .app
        .plot_progress()
        .expect("the plot is being computed");
    assert!(
        [
            "Reading the samples",
            "Computing Bray-Curtis distances",
            "Finding the principal coordinates",
            "PERMANOVA"
        ]
        .contains(&stage.as_str()),
        "{stage}"
    );
    if stage == "PERMANOVA" {
        assert!(fraction.is_some_and(|f| f < 1.0));
    }
    let shown = harness.screenshot("slow_plot_progress");
    // The plot before is greyed out and a panel drawn over it: the picture
    // differs from that of a moment before.
    assert_ne!(early.pixels, shown.pixels);

    // The rest of the window still answers: another analysis setting can be
    // changed while the plot is computed.
    harness.app.set_p_value_filter(Some(0.05));
    let app = &mut harness.app;
    harness.renderer.run(2, |ui| app.draw(ui));
    assert!(harness.app.plot_progress().is_some());

    harness.app.cancel_plot();
    assert!(harness.app.plot_progress().is_none());
    assert!(!harness.app.is_busy());
    harness.settle();
    assert_eq!(
        figure_texts(&harness.app),
        ["The calculation of this plot was cancelled"]
    );
    harness.screenshot("slow_plot_cancelled");

    // Fewer permutations: computed, and drawn.
    settings.community_permutations = 99;
    harness.app.set_plot_settings(settings);
    harness.settle();
    assert!(!harness.app.figure().regions.is_empty());
    assert!(figure_texts(&harness.app)
        .iter()
        .any(|t| t.starts_with("PERMANOVA: pseudo-F")));
}

/// The map of sample locations: markers at the sites of the metadata's
/// latitudes and longitudes, with tooltips; it follows the sample filter,
/// and its settings dialog.
#[test]
fn samples_are_placed_on_a_map() {
    let mut harness = Harness::new();
    harness.app.load_profile(
        "Enterotypes.profile.spf",
        &example("EnterotypesArumugam/Enterotypes.profile.spf"),
    );
    harness.app.load_metadata(
        "Enterotypes.metadata.locations.tsv",
        &example("EnterotypesArumugam/Enterotypes.metadata.locations.tsv"),
    );
    harness.app.set_p_value_filter(None);
    harness.app.set_mode(Mode::MultipleGroups);
    assert!(PlotKind::available(Mode::MultipleGroups).contains(&PlotKind::Map));
    assert!(PlotKind::available(Mode::TwoGroups).contains(&PlotKind::Map));
    assert!(PlotKind::available(Mode::Correlation).contains(&PlotKind::Map));
    assert!(!PlotKind::available(Mode::TwoSamples).contains(&PlotKind::Map));
    harness.app.set_plot_kind(PlotKind::Map);
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
    let texts = figure_texts(&harness.app);
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with(&format!("{samples} samples at "))),
        "{texts:?}"
    );
    let figure = harness.app.figure();
    assert!(figure
        .regions
        .iter()
        .any(|r| r.tooltip.contains("35.68°N, 139.69°E")));
    harness.screenshot("map");

    // Coloured by a feature, on a log scale; sized by a numeric field is
    // refused for a text field.
    let mut settings = harness.app.plot_settings().clone();
    settings.map.color_by =
        stamp_plot::ordination_style::ColorSource::Feature("Bacteroidetes".into());
    settings.map.color_scale = stamp_plot::ordination_style::ColorScale::Log10;
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    let texts = figure_texts(&harness.app);
    assert!(
        texts.iter().any(|t| t == "Bacteroidetes (%), mean at site"),
        "{texts:?}"
    );
    settings.map.size_by = stamp_plot::map_plot::SizeSource::Field("Gender".into());
    harness.app.set_plot_settings(settings.clone());
    harness.settle();
    assert!(figure_texts(&harness.app)[0].contains("does not hold numbers"));
    settings.map.size_by = stamp_plot::map_plot::SizeSource::Count;
    harness.app.set_plot_settings(settings);

    // Only the samples kept by the filter.
    let japanese = ["japanese".to_string()];
    harness
        .app
        .set_sample_filter(Some(("Nationality", &japanese)));
    harness.settle();
    let texts = figure_texts(&harness.app);
    assert!(
        texts.iter().any(|t| t.starts_with("13 samples at 1 site")),
        "{texts:?}"
    );
}

#[test]
fn map_settings_dialog() {
    use stamp_gui::plot_config::{show, Context};
    use stamp_gui::plots::PlotSettings;
    use stamp_plot::map_plot::{Extent, SizeSource};
    let context = Context {
        fields: vec![
            "Host_group".into(),
            "Captivity".into(),
            "Latitude".into(),
            "Longitude".into(),
            "Depth".into(),
        ],
        groups: vec!["Horse".into(), "Tapir".into()],
        numeric_fields: vec!["Latitude".into(), "Longitude".into(), "Depth".into()],
        lat_field: Some("Latitude".into()),
        lon_field: Some("Longitude".into()),
    };
    let mut renderer = Renderer::new(420.0, 1010.0, 1.0);
    let mut settings = PlotSettings::default();
    settings.map.extent = Extent::Custom {
        west: -20.0,
        east: 55.0,
        south: -37.0,
        north: 39.0,
    };
    settings.map.color_by =
        stamp_plot::ordination_style::ColorSource::Feature("Gorillivita".into());
    settings.map.size_by = SizeSource::Field("Depth".into());
    let before = settings.clone();
    let image = renderer.render(|ui| {
        egui::Frame::window(ui.style()).show(ui, |ui| {
            show(
                ui,
                Mode::MultipleGroups,
                PlotKind::Map,
                &mut settings,
                &context,
            );
        });
    });
    assert!(ink(&image, 0, 0, 420, 900) > 0.05);
    // Showing the controls changes nothing.
    assert_eq!(settings, before);
    if let Ok(directory) = std::env::var("STAMP_SCREENSHOTS") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            PathBuf::from(directory).join("dialog_map.png"),
            image.to_png(),
        )
        .unwrap();
    }
}

/// The picture of a plot being computed in the documentation: the PCoA of
/// the large example dataset part-way through its community tests (999
/// permutations each). Taken only if a frame falls in the middle of a test,
/// which depends on the speed of the machine.
#[test]
fn a_plot_part_way_through_its_community_tests() {
    let mut harness = Harness::new();
    harness
        .app
        .load_profile("profiles.tsv", &example("LargeDataset/profiles.tsv"));
    harness
        .app
        .load_metadata("metadata.tsv", &example("LargeDataset/metadata.tsv"));
    harness.app.set_p_value_filter(None);
    harness.app.set_mode(Mode::MultipleGroups);
    harness.app.set_plot_kind(PlotKind::Composition);
    harness.settle();
    assert_eq!(harness.app.plot_settings().community_permutations, 999);
    harness.app.set_plot_kind(PlotKind::Pcoa);
    let mut seen = Vec::new();
    for _ in 0..3000 {
        let app = &mut harness.app;
        harness.renderer.run(1, |ui| app.draw(ui));
        let Some((stage, fraction)) = harness.app.plot_progress() else {
            if !harness.app.is_busy() {
                break;
            }
            continue;
        };
        seen.push(stage.clone());
        let tests = ["PERMANOVA", "ANOSIM", "PERMDISP"];
        if tests.contains(&stage.as_str()) && fraction.is_some_and(|f| (0.4..0.6).contains(&f)) {
            harness.screenshot("gui_plot_progress");
            break;
        }
    }
    assert!(!seen.is_empty(), "no progress was shown");
    harness.app.cancel_plot();
}

/// Where a frame was drawn in the last frame.
fn frame_rect(harness: &Harness, frame: stamp_gui::layout::Frame) -> egui::Rect {
    let layout = harness.app.layout();
    let tile = stamp_gui::layout::tile_of(layout, frame).expect("every frame is in the layout");
    layout.tiles.rect(tile).expect("the frame was laid out")
}

#[test]
fn frames_can_be_moved_and_the_layout_is_remembered() {
    use stamp_gui::layout::Frame;
    let mut harness = Harness::enterotypes();
    harness.app.show_metadata_table(true);
    harness.settle();
    // To start with, the Metadata table is beneath the plot.
    assert!(
        frame_rect(&harness, Frame::Metadata).min.y > frame_rect(&harness, Frame::Plot).max.y - 1.0
    );

    // Dragging its tab to the far left of the top row, as egui_tiles does.
    let layout = harness.app.layout();
    let metadata = stamp_gui::layout::tile_of(layout, Frame::Metadata).unwrap();
    let plot_tabs = layout
        .tiles
        .parent_of(stamp_gui::layout::tile_of(layout, Frame::Plot).unwrap())
        .unwrap();
    let top_row = layout.tiles.parent_of(plot_tabs).unwrap();
    harness
        .app
        .layout_mut()
        .move_tile_to_container(metadata, top_row, 0, false);
    harness.settle();
    let moved = frame_rect(&harness, Frame::Metadata);
    assert!(
        moved.max.x <= frame_rect(&harness, Frame::Profile).min.x + 1.0,
        "{moved:?}"
    );
    // A frame dropped into a row gets an ordinary width.
    assert!(moved.width() > 150.0, "{moved:?}");
    harness.screenshot("metadata_moved_left");

    // A new window restores the arrangement.
    let ctx = harness.renderer.ctx.clone();
    let saved = harness.app.saved(&ctx);
    let mut again = Harness::enterotypes();
    let ctx = again.renderer.ctx.clone();
    again.app.restore(&ctx, saved.clone());
    again.app.show_metadata_table(true);
    again.settle();
    assert!(
        frame_rect(&again, Frame::Metadata).max.x <= frame_rect(&again, Frame::Profile).min.x + 1.0
    );

    // View > Reset layout puts it back beneath the plot.
    again.app.reset_layout();
    again.settle();
    assert!(
        frame_rect(&again, Frame::Metadata).min.y > frame_rect(&again, Frame::Plot).max.y - 1.0
    );

    // A layout saved by another version of STAMP, or missing a frame, is
    // replaced by the default.
    let mut old = saved.clone();
    old.layout_version = 0;
    again.app.restore(&ctx, old);
    assert_eq!(*again.app.layout(), stamp_gui::layout::default_tree());
    let mut broken = saved;
    let mut tree = broken.layout.clone().unwrap();
    let results = stamp_gui::layout::tile_of(&tree, Frame::Results).unwrap();
    tree.remove_recursively(results);
    broken.layout = Some(tree);
    again.app.restore(&ctx, broken);
    assert!(stamp_gui::layout::is_complete(again.app.layout()));
}

#[test]
fn frames_are_hidden_and_shown_and_open_where_they_belong() {
    use stamp_gui::layout::Frame;
    let mut harness = Harness::enterotypes();
    harness.settle();
    let plot = frame_rect(&harness, Frame::Plot);
    let visible = |harness: &Harness, frame| {
        let layout = harness.app.layout();
        layout
            .tiles
            .is_visible(stamp_gui::layout::tile_of(layout, frame).unwrap())
    };
    // Hidden (as its tab's × does), the feature table gives its room to
    // the plot.
    assert!(visible(&harness, Frame::Features));
    harness.app.show_frame(Frame::Features, false);
    harness.settle();
    assert!(!visible(&harness, Frame::Features));
    assert!(frame_rect(&harness, Frame::Plot).width() > plot.width() + 200.0);
    harness.app.show_frame(Frame::Features, true);
    harness.settle();
    assert!(visible(&harness, Frame::Features));
    assert!((frame_rect(&harness, Frame::Plot).width() - plot.width()).abs() < 1.0);

    // Configure plot opens beside the plot, in front of Properties.
    assert!(!visible(&harness, Frame::ConfigurePlot));
    harness.app.show_plot_config(true);
    harness.settle();
    assert!(visible(&harness, Frame::ConfigurePlot));
    let configure = frame_rect(&harness, Frame::ConfigurePlot);
    assert!(configure.min.x > plot.max.x, "{configure:?}");
    harness.screenshot("configure_plot_docked");

    // The results table opens along the bottom.
    harness.app.show_results_table(true);
    harness.settle();
    let results = frame_rect(&harness, Frame::Results);
    assert!(
        results.min.y > frame_rect(&harness, Frame::Plot).max.y - 1.0,
        "{results:?}"
    );
    assert!(harness.app.frame_shown(Frame::Results));
    harness.screenshot("results_table_docked");
}
