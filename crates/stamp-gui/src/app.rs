//! The main window.

use eframe::egui::{self, Color32, RichText};
use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::effect_size::{GroupEffectSize, SampleEffectSize};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::post_hoc::PostHocTest;
use stamp_core::stats::results::{FilterOperator, GroupCountFilter, SampleCountFilter};
use stamp_core::stats::table::{group_table, multi_group_table, sample_table, Table};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::TestType;
use stamp_core::Unclassified;
use stamp_plot::extended_error_bar::SortBy;
use stamp_plot::Figure;

use crate::figure_painter;
use crate::platform::{self, Event, Platform, Request, Source};
use crate::plots::{self, PlotKind, PlotSettings};
use crate::session::{Mode, Session};

const COVERAGES: [f64; 5] = [0.90, 0.95, 0.98, 0.99, 0.999];

/// Zoom of the plot when the application starts, and after a reset. A plot
/// of the default width fits beside the panels in a 1280-pixel window.
const DEFAULT_ZOOM: f32 = 1.15;

/// Example data built into the application: gut enterotypes from Arumugam et
/// al. (2011).
const EXAMPLE_PROFILE: &str =
    include_str!("../../../examples/EnterotypesArumugam/Enterotypes.profile.spf");
const EXAMPLE_METADATA: &str =
    include_str!("../../../examples/EnterotypesArumugam/Enterotypes.metadata.tsv");

/// A combo box over a fixed list of choices. Returns true if the choice
/// changed.
fn choose<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut T,
    options: &[(T, String)],
) -> bool {
    let mut changed = false;
    let selected = options
        .iter()
        .find(|(value, _)| value == current)
        .map(|(_, name)| name.clone())
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected)
        .width(170.0)
        .show_ui(ui, |ui| {
            for (value, name) in options {
                if ui
                    .selectable_label(current == value, name.as_str())
                    .clicked()
                    && current != value
                {
                    *current = value.clone();
                    changed = true;
                }
            }
        });
    changed
}

fn named<T: Copy>(items: &[T], name: impl Fn(T) -> &'static str) -> Vec<(T, String)> {
    items
        .iter()
        .map(|&item| (item, name(item).to_string()))
        .collect()
}

fn strings(items: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<(String, String)> {
    items
        .into_iter()
        .map(|s| (s.as_ref().to_string(), s.as_ref().to_string()))
        .collect()
}

/// A labelled row of a settings grid.
fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    ui.label(label);
    let changed = add(ui);
    ui.end_row();
    changed
}

/// The user's filter settings, kept even while a filter is switched off.
#[derive(Debug, Clone, PartialEq)]
struct FilterUi {
    p_enabled: bool,
    p_value: f64,
    effect1_enabled: bool,
    effect1_measure: usize,
    effect1_minimum: f64,
    effect2_enabled: bool,
    effect2_measure: usize,
    effect2_minimum: f64,
    operator: FilterOperator,
    counts_enabled: bool,
    counts_kind: usize,
    counts_first: f64,
    counts_second: f64,
}

impl Default for FilterUi {
    fn default() -> Self {
        FilterUi {
            p_enabled: false,
            p_value: 0.05,
            effect1_enabled: false,
            effect1_measure: 0,
            effect1_minimum: 1.0,
            effect2_enabled: false,
            effect2_measure: 1,
            effect2_minimum: 2.0,
            operator: FilterOperator::Or,
            counts_enabled: false,
            counts_kind: 0,
            counts_first: 1.0,
            counts_second: 1.0,
        }
    }
}

impl FilterUi {
    /// Controls for the filters. `measures` names the effect-size measures
    /// and `count_kinds` the sequence-count filters; empty lists hide those
    /// parts.
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        measures: &[&str],
        count_kinds: &[&str],
    ) -> bool {
        let before = self.clone();
        egui::Grid::new(format!("{id}-filters"))
            .num_columns(2)
            .show(ui, |ui| {
                ui.checkbox(&mut self.p_enabled, "p-value filter (>)");
                ui.add_enabled(
                    self.p_enabled,
                    egui::DragValue::new(&mut self.p_value)
                        .speed(0.001)
                        .range(0.0..=1.0),
                );
                ui.end_row();

                if !measures.is_empty() {
                    let options: Vec<(usize, String)> = measures
                        .iter()
                        .enumerate()
                        .map(|(i, m)| (i, m.to_string()))
                        .collect();
                    for (index, enabled, measure, minimum) in [
                        (
                            1,
                            &mut self.effect1_enabled,
                            &mut self.effect1_measure,
                            &mut self.effect1_minimum,
                        ),
                        (
                            2,
                            &mut self.effect2_enabled,
                            &mut self.effect2_measure,
                            &mut self.effect2_minimum,
                        ),
                    ] {
                        ui.checkbox(enabled, format!("Effect size filter {index} (<)"));
                        ui.add_enabled(
                            *enabled,
                            egui::DragValue::new(minimum)
                                .speed(0.05)
                                .range(0.0..=f64::MAX),
                        );
                        ui.end_row();
                        if measures.len() > 1 {
                            ui.label("");
                            ui.add_enabled_ui(*enabled, |ui| {
                                choose(ui, &format!("{id}-effect{index}"), measure, &options)
                            });
                            ui.end_row();
                        }
                    }
                    if measures.len() > 1 {
                        ui.label("Remove a feature failing");
                        let both = self.effect1_enabled && self.effect2_enabled;
                        ui.add_enabled_ui(both, |ui| {
                            choose(
                                ui,
                                &format!("{id}-operator"),
                                &mut self.operator,
                                &[
                                    (FilterOperator::Or, "either filter".to_string()),
                                    (FilterOperator::And, "both filters".to_string()),
                                ],
                            )
                        });
                        ui.end_row();
                    }
                }

                if !count_kinds.is_empty() {
                    ui.checkbox(&mut self.counts_enabled, "Sequence filter");
                    let options: Vec<(usize, String)> = count_kinds
                        .iter()
                        .enumerate()
                        .map(|(i, k)| (i, k.to_string()))
                        .collect();
                    ui.add_enabled_ui(self.counts_enabled, |ui| {
                        choose(ui, &format!("{id}-counts"), &mut self.counts_kind, &options)
                    });
                    ui.end_row();
                    ui.label("");
                    ui.add_enabled_ui(self.counts_enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.counts_first)
                                    .speed(1.0)
                                    .range(0.0..=f64::MAX),
                            );
                            if count_kinds[self.counts_kind].starts_with("independent") {
                                ui.add(
                                    egui::DragValue::new(&mut self.counts_second)
                                        .speed(1.0)
                                        .range(0.0..=f64::MAX),
                                );
                            }
                        });
                    });
                    ui.end_row();
                }
            });
        *self != before
    }
}

const SAMPLE_COUNT_KINDS: [&str; 3] = ["maximum", "minimum", "independent"];
const GROUP_COUNT_KINDS: [&str; 4] = [
    "maximum",
    "minimum",
    "independent, minimum",
    "independent, maximum",
];

/// The STAMP application: one analysis session and its window.
pub struct StampApp {
    pub session: Session,
    pub mode: Mode,
    plot_kinds: [PlotKind; 3],
    pub plot_settings: PlotSettings,
    zoom: f32,
    filters: [FilterUi; 3],
    /// The current plot and table, rebuilt when `view_dirty` is set.
    figure: Figure,
    table: Table,
    table_features: Vec<String>,
    view_dirty: bool,
    seen_generation: u64,
    show_table: bool,
    show_about: bool,
    show_preferences: bool,
    /// A message for the user, shown until dismissed.
    notice: Option<String>,
    platform: Platform,
    /// The plot item under the pointer in the last frame.
    pub hovered: Option<String>,
    /// Where the plot was drawn in the last frame, and the area it was
    /// centred in.
    pub plot_placement: Option<(egui::Rect, egui::Rect)>,
}

fn mode_slot(mode: Mode) -> usize {
    Mode::ALL.iter().position(|m| *m == mode).unwrap_or(0)
}

impl StampApp {
    /// Create the application. Registers the plot font with `ctx`.
    pub fn new(ctx: &egui::Context) -> StampApp {
        figure_painter::install_plot_font(ctx);
        StampApp {
            session: Session::default(),
            mode: Mode::TwoSamples,
            plot_kinds: [
                PlotKind::available(Mode::MultipleGroups)[0],
                PlotKind::available(Mode::TwoGroups)[0],
                PlotKind::available(Mode::TwoSamples)[0],
            ],
            plot_settings: PlotSettings::default(),
            zoom: DEFAULT_ZOOM,
            filters: [
                FilterUi::default(),
                FilterUi::default(),
                FilterUi::default(),
            ],
            figure: Figure::message("Load a profile to begin"),
            table: Table {
                headings: Vec::new(),
                rows: Vec::new(),
            },
            table_features: Vec::new(),
            view_dirty: true,
            seen_generation: 0,
            show_table: false,
            show_about: false,
            show_preferences: false,
            notice: None,
            platform: Platform::default(),
            hovered: None,
            plot_placement: None,
        }
    }

    /// The plot shown for the current mode.
    pub fn plot_kind(&self) -> PlotKind {
        self.plot_kinds[mode_slot(self.mode)]
    }

    pub fn set_plot_kind(&mut self, kind: PlotKind) {
        self.plot_kinds[mode_slot(self.mode)] = kind;
        self.view_dirty = true;
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.view_dirty = true;
    }

    pub fn show_results_table(&mut self, show: bool) {
        self.show_table = show;
    }

    /// The figure currently shown.
    pub fn figure(&self) -> &Figure {
        &self.figure
    }

    /// Load a profile from the text of a file.
    pub fn load_profile(&mut self, name: &str, text: &str) {
        match self.session.load_profile(name, text) {
            Ok(()) => {
                self.filters = Default::default();
                self.mode = Mode::TwoSamples;
                self.apply_filters();
            }
            Err(message) => self.notice = Some(format!("Could not load {name}:\n{message}")),
        }
        self.view_dirty = true;
    }

    /// Load metadata from the text of a file.
    pub fn load_metadata(&mut self, name: &str, text: &str) {
        match self.session.load_metadata(name, text) {
            Ok(()) => {
                self.mode = Mode::TwoGroups;
                if !self.session.warnings.is_empty() {
                    self.notice = Some(self.session.warnings.join("\n\n"));
                }
            }
            Err(message) => self.notice = Some(format!("Could not load {name}:\n{message}")),
        }
        self.view_dirty = true;
    }

    /// Copy the filter controls into the session's filters.
    fn apply_filters(&mut self) {
        let effect = |enabled: bool, minimum: f64| enabled.then_some(minimum);

        let f = &self.filters[mode_slot(Mode::TwoSamples)];
        let filter = &mut self.session.samples.filter;
        filter.significance = f.p_enabled.then_some(f.p_value);
        filter.effect_size1 = effect(f.effect1_enabled, f.effect1_minimum)
            .map(|m| (SampleEffectSize::ALL[f.effect1_measure], m));
        filter.effect_size2 = effect(f.effect2_enabled, f.effect2_minimum)
            .map(|m| (SampleEffectSize::ALL[f.effect2_measure], m));
        filter.operator = f.operator;
        filter.sequences = f.counts_enabled.then_some(match f.counts_kind {
            0 => SampleCountFilter::Maximum(f.counts_first),
            1 => SampleCountFilter::Minimum(f.counts_first),
            _ => SampleCountFilter::Independent(f.counts_first, f.counts_second),
        });

        let f = &self.filters[mode_slot(Mode::TwoGroups)];
        let filter = &mut self.session.groups.filter;
        filter.significance = f.p_enabled.then_some(f.p_value);
        let measure = |index: usize| GroupEffectSize::ALL[index.min(1)];
        filter.effect_size1 =
            effect(f.effect1_enabled, f.effect1_minimum).map(|m| (measure(f.effect1_measure), m));
        filter.effect_size2 =
            effect(f.effect2_enabled, f.effect2_minimum).map(|m| (measure(f.effect2_measure), m));
        filter.operator = f.operator;
        filter.sequences = f.counts_enabled.then_some(match f.counts_kind {
            0 => GroupCountFilter::Maximum(f.counts_first),
            1 => GroupCountFilter::Minimum(f.counts_first),
            2 => GroupCountFilter::IndependentMinimum(f.counts_first, f.counts_second),
            _ => GroupCountFilter::IndependentMaximum(f.counts_first, f.counts_second),
        });

        let f = &self.filters[mode_slot(Mode::MultipleGroups)];
        let filter = &mut self.session.multi.filter;
        filter.significance = f.p_enabled.then_some(f.p_value);
        filter.min_effect_size = effect(f.effect1_enabled, f.effect1_minimum);
        self.view_dirty = true;
    }

    /// Rebuild the plot and the results table from the session.
    fn refresh_view(&mut self) {
        self.figure = plots::build(
            &self.session,
            self.mode,
            self.plot_kind(),
            &self.plot_settings,
        );
        let s = &self.session;
        let (table, features): (Table, Vec<String>) = match self.mode {
            Mode::TwoSamples => match &s.samples.output {
                Some((profile, results)) => {
                    let rows = s.sample_rows();
                    let features = rows.iter().map(|r| r.feature.clone()).collect();
                    (sample_table(profile, results, rows), features)
                }
                None => empty_table(),
            },
            Mode::TwoGroups => match &s.groups.output {
                Some((profile, results)) => {
                    let rows = s.group_rows();
                    let features = rows.iter().map(|r| r.feature.clone()).collect();
                    (group_table(profile, results, rows), features)
                }
                None => empty_table(),
            },
            Mode::MultipleGroups => match &s.multi.output {
                Some((profile, results)) => {
                    let rows = s.multi_rows();
                    let features = rows.iter().map(|r| r.feature.clone()).collect();
                    (multi_group_table(profile, results, rows), features)
                }
                None => empty_table(),
            },
        };
        self.table = table;
        self.table_features = features;
        self.view_dirty = false;
    }

    /// Draw the whole interface for one frame.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.handle_file_events();
        if self.session.update(self.mode) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        if self.session.generation != self.seen_generation {
            self.seen_generation = self.session.generation;
            self.view_dirty = true;
        }

        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("profile")
            .default_size(215.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.profile_panel(ui));
            });
        egui::Panel::right("properties")
            .default_size(285.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.properties_panel(ui));
            });
        egui::CentralPanel::default().show(ui, |ui| self.plot_panel(ui));
        self.windows(ui.ctx());
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                let loaded = self.session.tree.is_some();
                if ui.button("Load profile...").clicked() {
                    self.platform.request(Request::OpenProfile, &ctx);
                }
                if ui
                    .add_enabled(loaded, egui::Button::new("Load metadata..."))
                    .clicked()
                {
                    self.platform.request(Request::OpenMetadata, &ctx);
                }
                if ui.button("Open example data").clicked() {
                    self.load_example();
                }
                ui.menu_button("Create profile from", |ui| {
                    for source in Source::ALL {
                        if ui.button(source.name()).clicked() {
                            self.platform.request(Request::Convert(source), &ctx);
                        }
                    }
                });
                ui.separator();
                ui.add_enabled_ui(loaded, |ui| {
                    ui.menu_button("Save plot as", |ui| {
                        for (label, name) in [
                            ("PNG image", "plot.png"),
                            ("SVG image", "plot.svg"),
                            ("PDF document", "plot.pdf"),
                        ] {
                            if ui.button(label).clicked() {
                                match platform::encode_plot(&self.figure, name) {
                                    Ok(bytes) => self.platform.request(
                                        Request::Save {
                                            name: name.to_string(),
                                            bytes,
                                        },
                                        &ctx,
                                    ),
                                    Err(message) => self.notice = Some(message),
                                }
                            }
                        }
                    });
                });
                if ui
                    .add_enabled(
                        !self.table.rows.is_empty(),
                        egui::Button::new("Save results table..."),
                    )
                    .clicked()
                {
                    self.download_table(&ctx);
                }
                // A web page is closed with its tab.
                if cfg!(not(target_arch = "wasm32")) {
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_table, "Results table");
            });
            ui.menu_button("Settings", |ui| {
                let mut dark = ctx.theme() == egui::Theme::Dark;
                if ui.checkbox(&mut dark, "Dark mode").changed() {
                    set_dark_mode(&ctx, dark);
                }
                ui.separator();
                if ui.button("Preferences...").clicked() {
                    self.show_preferences = true;
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("About STAMP").clicked() {
                    self.show_about = true;
                }
            });
        });
    }

    /// Save the results table as it is shown, through the platform's save
    /// dialog or as a browser download.
    pub fn download_table(&self, ctx: &egui::Context) {
        let bytes = self.table.to_tsv().into_bytes();
        self.platform.request(
            Request::Save {
                name: "results.tsv".to_string(),
                bytes,
            },
            ctx,
        );
    }

    /// The zoom of the plot; 1 shows one figure point per screen point.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Return the plot to the zoom it starts with.
    pub fn reset_zoom(&mut self) {
        self.zoom = DEFAULT_ZOOM;
    }

    /// The results table as shown: the rows that pass the filters.
    pub fn results_table(&self) -> &Table {
        &self.table
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.5, 4.0);
    }

    /// Load the example that ships inside the application.
    pub fn load_example(&mut self) {
        self.load_profile("Enterotypes.profile.spf", EXAMPLE_PROFILE);
        self.load_metadata("Enterotypes.metadata.tsv", EXAMPLE_METADATA);
    }

    /// Hand the application the outcome of a file request, as the file
    /// dialogs do.
    pub fn deliver(&self, event: Event) {
        self.platform.push(event);
    }

    /// The message currently shown to the user, if any.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Act on files that have been opened or converted since the last frame.
    fn handle_file_events(&mut self) {
        for event in self.platform.poll() {
            match event {
                Event::Profile(file) => self.load_profile(&file.name, &file.text()),
                Event::Metadata(file) => self.load_metadata(&file.name, &file.text()),
                Event::Converted { name, spf } => self.load_profile(&name, &spf),
                Event::Failed(message) => self.notice = Some(message),
            }
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.session.tree.is_none() {
                ui.label("No profile loaded");
                return;
            }
            let (total, active) = self.session.feature_counts(self.mode);
            ui.label(format!("{active} of {total} features shown"));
            if let Some(progress) = self.session.progress(self.mode) {
                ui.separator();
                ui.add(
                    egui::ProgressBar::new(progress)
                        .desired_width(160.0)
                        .text("Running test..."),
                );
                if ui.small_button("Cancel").clicked() {
                    self.session.cancel(self.mode);
                }
            }
            if let Some(error) = &self.session.error {
                ui.separator();
                ui.colored_label(Color32::from_rgb(170, 0, 0), error);
            }
        });
    }

    fn profile_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Profile");
        let Some(tree) = self.session.tree.clone() else {
            ui.label("Use File > Load profile to open a STAMP profile.");
            return;
        };
        ui.label(RichText::new(&self.session.profile_name).strong());
        ui.label(format!(
            "{} samples, {} levels",
            tree.num_samples(),
            tree.hierarchy_headings().len()
        ));
        ui.add_space(6.0);

        let headings = tree.hierarchy_headings();
        let mut changed = false;
        egui::Grid::new("profile-grid")
            .num_columns(2)
            .show(ui, |ui| {
                changed |= row(ui, "Profile level:", |ui| {
                    choose(
                        ui,
                        "profile-level",
                        &mut self.session.profile_level,
                        &strings(headings),
                    )
                });
                let depth = headings
                    .iter()
                    .position(|h| *h == self.session.profile_level)
                    .unwrap_or(0);
                let mut parents: Vec<(Option<String>, String)> =
                    vec![(None, "Entire sample".to_string())];
                parents.extend(
                    headings[..depth]
                        .iter()
                        .map(|h| (Some(h.clone()), h.clone())),
                );
                // The parent must stay above the profile level.
                if !parents
                    .iter()
                    .any(|(value, _)| *value == self.session.parent_level)
                {
                    self.session.parent_level = None;
                    changed = true;
                }
                changed |= row(ui, "Parent level:", |ui| {
                    choose(ui, "parent-level", &mut self.session.parent_level, &parents)
                });
                changed |= row(ui, "Unclassified:", |ui| {
                    choose(
                        ui,
                        "unclassified",
                        &mut self.session.unclassified,
                        &[
                            (
                                Unclassified::Retain,
                                "Retain unclassified reads".to_string(),
                            ),
                            (
                                Unclassified::Remove,
                                "Remove unclassified reads".to_string(),
                            ),
                            (
                                Unclassified::FrequencyOnly,
                                "Use only for calculating frequency profiles".to_string(),
                            ),
                        ],
                    )
                });
            });
        if changed {
            self.session.invalidate_all();
        }

        ui.add_space(10.0);
        ui.heading("Groups");
        let Some(metadata) = &self.session.metadata else {
            ui.label("Use File > Load metadata to define groups of samples.");
            return;
        };
        ui.label(RichText::new(&self.session.metadata_name).strong());
        let fields = strings(metadata.fields());
        let mut field = self.session.group_field.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Group field:");
            if choose(ui, "group-field", &mut field, &fields) {
                self.session.set_group_field(&field);
            }
        });
        if let Some(groups) = self.session.grouping() {
            let names: Vec<String> = groups.names().map(str::to_string).collect();
            for name in &names {
                let size = groups.samples(name).map_or(0, <[String]>::len);
                let position = self
                    .session
                    .multi
                    .active_groups
                    .iter()
                    .position(|g| g == name);
                ui.horizontal(|ui| {
                    // The swatch opens a colour picker for the group.
                    let color = self.session.group_color(name);
                    let mut rgb = [color.r, color.g, color.b];
                    let picker = ui
                        .color_edit_button_srgb(&mut rgb)
                        .on_hover_text("Change the colour of this group");
                    if picker.changed() {
                        self.session
                            .set_group_color(name, stamp_plot::Color::rgb(rgb[0], rgb[1], rgb[2]));
                    }
                    let mut active = position.is_some();
                    if ui
                        .add_enabled(
                            size > 0,
                            egui::Checkbox::new(&mut active, format!("{name} ({size})")),
                        )
                        .changed()
                    {
                        if active {
                            self.session.multi.active_groups.push(name.clone());
                            self.session.multi.active_groups.sort();
                        } else {
                            self.session.multi.active_groups.retain(|g| g != name);
                        }
                        self.session.invalidate(Mode::MultipleGroups);
                    }
                });
            }
        }
    }

    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Statistics");
        if self.session.tree.is_none() {
            return;
        }
        let corrections = named(&Correction::ALL, Correction::name);
        let test_types = named(&TestType::ALL, TestType::name);
        let coverages: Vec<(f64, String)> =
            COVERAGES.iter().map(|&c| (c, format!("{c}"))).collect();
        let mode = self.mode;
        let mut rerun = false;
        let mut correction = None;

        match mode {
            Mode::TwoSamples => {
                let samples = strings(
                    self.session
                        .tree
                        .as_ref()
                        .map(|t| t.sample_names().to_vec())
                        .unwrap_or_default(),
                );
                let state = &mut self.session.samples;
                egui::Grid::new("sample-stats")
                    .num_columns(2)
                    .show(ui, |ui| {
                        rerun |= row(ui, "Sample 1:", |ui| {
                            choose(ui, "sample1", &mut state.sample1, &samples)
                        });
                        rerun |= row(ui, "Sample 2:", |ui| {
                            choose(ui, "sample2", &mut state.sample2, &samples)
                        });
                        rerun |= row(ui, "Statistical test:", |ui| {
                            choose(
                                ui,
                                "sample-test",
                                &mut state.analysis.test,
                                &named(&SampleTest::ALL, SampleTest::name),
                            )
                        });
                        // Tests without a one-sided p-value are always two-sided.
                        if !state.analysis.test.has_one_sided()
                            && state.analysis.test_type == TestType::OneSided
                        {
                            state.analysis.test_type = TestType::TwoSided;
                            rerun = true;
                        }
                        ui.label("Type:");
                        ui.add_enabled_ui(state.analysis.test.has_one_sided(), |ui| {
                            rerun |= choose(
                                ui,
                                "sample-type",
                                &mut state.analysis.test_type,
                                &test_types,
                            );
                        });
                        ui.end_row();
                        rerun |= row(ui, "CI method:", |ui| {
                            choose(
                                ui,
                                "sample-ci",
                                &mut state.analysis.ci_method,
                                &named(&SampleCi::ALL, SampleCi::name),
                            )
                        });
                        rerun |= row(ui, "Coverage:", |ui| {
                            choose(
                                ui,
                                "sample-coverage",
                                &mut state.analysis.coverage,
                                &coverages,
                            )
                        });
                        let mut chosen = state.analysis.correction;
                        if row(ui, "Multiple test correction:", |ui| {
                            choose(ui, "sample-correction", &mut chosen, &corrections)
                        }) {
                            correction = Some(chosen);
                        }
                    });
            }
            Mode::TwoGroups => {
                let Some(groups) = self.session.grouping() else {
                    ui.label("Load a metadata file to compare groups.");
                    return;
                };
                let names: Vec<String> = groups.names().map(str::to_string).collect();
                let mut seconds: Vec<(Option<String>, String)> =
                    names.iter().map(|n| (Some(n.clone()), n.clone())).collect();
                seconds.push((None, stamp_core::ALL_OTHER_SAMPLES.to_string()));
                let state = &mut self.session.groups;
                egui::Grid::new("group-stats")
                    .num_columns(2)
                    .show(ui, |ui| {
                        rerun |= row(ui, "Group 1:", |ui| {
                            choose(ui, "group1", &mut state.group1, &strings(&names))
                        });
                        rerun |= row(ui, "Group 2:", |ui| {
                            choose(ui, "group2", &mut state.group2, &seconds)
                        });
                        rerun |= row(ui, "Statistical test:", |ui| {
                            choose(
                                ui,
                                "group-test",
                                &mut state.analysis.test,
                                &named(&GroupTest::ALL, GroupTest::name),
                            )
                        });
                        rerun |= row(ui, "Type:", |ui| {
                            choose(ui, "group-type", &mut state.analysis.test_type, &test_types)
                        });
                        row(ui, "CI method:", |ui| {
                            ui.label(state.analysis.test.ci_method());
                            false
                        });
                        rerun |= row(ui, "Coverage:", |ui| {
                            choose(
                                ui,
                                "group-coverage",
                                &mut state.analysis.coverage,
                                &coverages,
                            )
                        });
                        let mut chosen = state.analysis.correction;
                        if row(ui, "Multiple test correction:", |ui| {
                            choose(ui, "group-correction", &mut chosen, &corrections)
                        }) {
                            correction = Some(chosen);
                        }
                    });
            }
            Mode::MultipleGroups => {
                if self.session.metadata.is_none() {
                    ui.label("Load a metadata file to compare groups.");
                    return;
                }
                let state = &mut self.session.multi;
                let mut post_hoc_changed = false;
                egui::Grid::new("multi-stats")
                    .num_columns(2)
                    .show(ui, |ui| {
                        rerun |= row(ui, "Statistical test:", |ui| {
                            choose(
                                ui,
                                "multi-test",
                                &mut state.analysis.test,
                                &named(&MultiGroupTest::ALL, MultiGroupTest::name),
                            )
                        });
                        row(ui, "Effect size:", |ui| {
                            ui.label(state.analysis.effect_size.name());
                            false
                        });
                        let mut chosen = state.analysis.correction;
                        if row(ui, "Multiple test correction:", |ui| {
                            choose(ui, "multi-correction", &mut chosen, &corrections)
                        }) {
                            correction = Some(chosen);
                        }
                        post_hoc_changed |= row(ui, "Post-hoc test:", |ui| {
                            choose(
                                ui,
                                "post-hoc-test",
                                &mut state.post_hoc_test,
                                &named(&PostHocTest::ALL, PostHocTest::name),
                            )
                        });
                        post_hoc_changed |= row(ui, "Post-hoc coverage:", |ui| {
                            choose(
                                ui,
                                "post-hoc-coverage",
                                &mut state.post_hoc_coverage,
                                &coverages,
                            )
                        });
                    });
                self.view_dirty |= post_hoc_changed;
            }
        }
        if rerun {
            self.session.invalidate(mode);
        }
        if let Some(correction) = correction {
            self.session.set_correction(mode, correction);
        }

        ui.add_space(10.0);
        ui.heading("Filtering");
        let slot = mode_slot(mode);
        let changed = match mode {
            Mode::TwoSamples => {
                let measures: Vec<&str> = SampleEffectSize::ALL.iter().map(|m| m.name()).collect();
                self.filters[slot].show(ui, "samples", &measures, &SAMPLE_COUNT_KINDS)
            }
            Mode::TwoGroups => {
                let measures: Vec<&str> = GroupEffectSize::ALL.iter().map(|m| m.name()).collect();
                self.filters[slot].show(ui, "groups", &measures, &GROUP_COUNT_KINDS)
            }
            Mode::MultipleGroups => self.filters[slot].show(ui, "multi", &["Eta-squared"], &[]),
        };
        if changed {
            self.apply_filters();
        }
    }

    fn plot_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for mode in Mode::ALL {
                if ui
                    .selectable_label(self.mode == mode, mode.name())
                    .clicked()
                    && self.mode != mode
                {
                    self.set_mode(mode);
                }
            }
        });
        ui.separator();

        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Plot:");
            let mut kind = self.plot_kind();
            if choose(
                ui,
                "plot-kind",
                &mut kind,
                &named(PlotKind::available(self.mode), PlotKind::name),
            ) {
                self.set_plot_kind(kind);
            }
            ui.separator();
            ui.label("Width:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut self.plot_settings.width)
                        .speed(0.1)
                        .range(2.0..=40.0)
                        .suffix(" in"),
                )
                .changed();
            ui.label("Height:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut self.plot_settings.height)
                        .speed(0.1)
                        .range(2.0..=40.0)
                        .suffix(" in"),
                )
                .changed();
            ui.label("Sort by:");
            changed |= choose(
                ui,
                "plot-sort",
                &mut self.plot_settings.sort_by,
                &[
                    (SortBy::PValues, "p-values".to_string()),
                    (SortBy::EffectSizes, "Effect sizes".to_string()),
                    (SortBy::FeatureLabels, "Feature labels".to_string()),
                ],
            );
            changed |= ui
                .checkbox(&mut self.plot_settings.show_legend, "Legend")
                .changed();
            ui.separator();
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.zoom, 0.5..=4.0).show_value(false));
            let reset = ui.add_enabled(self.zoom != DEFAULT_ZOOM, egui::Button::new("Reset"));
            if reset.on_hover_text("Return to the default zoom").clicked() {
                self.reset_zoom();
            }
        });
        if self.plot_kind().uses_selected_feature(self.mode) {
            ui.horizontal(|ui| {
                ui.label("Feature:");
                let mut feature = self.session.selected_feature.clone().unwrap_or_default();
                if choose(
                    ui,
                    "selected-feature",
                    &mut feature,
                    &strings(&self.table_features),
                ) {
                    self.session.selected_feature = Some(feature);
                    changed = true;
                }
            });
        }
        self.view_dirty |= changed;
        ui.separator();

        if self.view_dirty {
            self.refresh_view();
        }
        // The plot is centred in the room left for it.
        let space = ui.available_size();
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let response = figure_painter::show(ui, &self.figure, self.zoom, space);
                self.hovered = response.hovered;
                self.plot_placement = response.placement;
                if let Some(item) = response.clicked {
                    // Clicking a feature selects it for the single-feature plots.
                    if self.table_features.contains(&item) {
                        self.session.selected_feature = Some(item);
                        self.view_dirty = true;
                    }
                }
            });
    }

    fn windows(&mut self, ctx: &egui::Context) {
        let mut select = None;
        let mut download = false;
        egui::Window::new("Results table")
            .open(&mut self.show_table)
            .default_size([760.0, 320.0])
            .show(ctx, |ui| {
                if self.table.rows.is_empty() {
                    ui.label("No features pass the filters.");
                    return;
                }
                ui.horizontal(|ui| {
                    download = ui
                        .button("Download")
                        .on_hover_text("Save this table as a tab-separated file")
                        .clicked();
                    ui.label(format!("{} features", self.table.rows.len()));
                });
                ui.separator();
                use egui_extras::{Column, TableBuilder};
                let selected = self.session.selected_feature.as_deref();
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .sense(egui::Sense::click())
                        .columns(
                            Column::auto().at_least(70.0).clip(true).resizable(true),
                            self.table.headings.len(),
                        )
                        .header(20.0, |mut header| {
                            for heading in &self.table.headings {
                                header.col(|ui| {
                                    ui.strong(heading);
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(18.0, self.table.rows.len(), |mut row| {
                                let index = row.index();
                                row.set_selected(
                                    selected == self.table_features.get(index).map(String::as_str),
                                );
                                for cell in &self.table.rows[index] {
                                    row.col(|ui| {
                                        ui.label(shorten_number(cell));
                                    });
                                }
                                if row.response().clicked() {
                                    select = self.table_features.get(index).cloned();
                                }
                            });
                        });
                });
            });
        if let Some(feature) = select {
            self.session.selected_feature = Some(feature);
            self.view_dirty = true;
        }
        if download {
            self.download_table(ctx);
        }

        let mut changed = false;
        egui::Window::new("Preferences")
            .open(&mut self.show_preferences)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("preferences")
                    .num_columns(2)
                    .show(ui, |ui| {
                        changed |= row(ui, "Random seed:", |ui| {
                            ui.add(egui::DragValue::new(&mut self.session.seed))
                                .changed()
                        });
                        changed |= row(ui, "Replicates:", |ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.session.stats.replicates)
                                    .range(10..=1_000_000),
                            )
                            .changed()
                        });
                        changed |= row(ui, "Pseudocount:", |ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.session.stats.pseudocount)
                                    .speed(0.05)
                                    .range(0.0..=10.0),
                            )
                            .changed()
                        });
                    });
                ui.label("The seed makes bootstrap and permutation results repeatable.");
            });
        if changed {
            self.session.invalidate_all();
        }

        egui::Window::new("About STAMP")
            .open(&mut self.show_about)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("STAMP");
                ui.label(format!("Version {}", stamp_core::VERSION));
                ui.label("Statistical analysis of taxonomic and functional profiles.");
                ui.label("Donovan Parks and Rob Beiko");
                ui.label("Released under the GNU General Public License v3.");
            });

        let mut dismiss = false;
        if let Some(notice) = &self.notice {
            egui::Window::new("STAMP")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(notice);
                    dismiss = ui.button("OK").clicked();
                });
        }
        if dismiss {
            self.notice = None;
        }
    }
}

/// Switch the whole interface between its dark and light appearance.
pub fn set_dark_mode(ctx: &egui::Context, dark: bool) {
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
}

/// Numbers in the table are shown with six significant digits; the saved
/// table keeps full precision.
fn shorten_number(cell: &str) -> String {
    match cell.parse::<f64>() {
        Ok(value) if cell.len() > 9 && value.is_finite() => {
            let magnitude = value.abs();
            if magnitude != 0.0 && !(1e-4..1e7).contains(&magnitude) {
                format!("{value:.3e}")
            } else {
                let text = format!("{value:.6}");
                text.trim_end_matches('0').trim_end_matches('.').to_string()
            }
        }
        _ => cell.to_string(),
    }
}

fn empty_table() -> (Table, Vec<String>) {
    (
        Table {
            headings: Vec::new(),
            rows: Vec::new(),
        },
        Vec::new(),
    )
}

impl eframe::App for StampApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_numbers_are_shortened_for_display() {
        assert_eq!(shorten_number("0.123456789012"), "0.123457");
        assert_eq!(shorten_number("1.1420258496325005e-18"), "1.142e-18");
        assert_eq!(shorten_number("12345.678901234"), "12345.678901");
        assert_eq!(shorten_number("5"), "5");
        assert_eq!(shorten_number("Bacteroides"), "Bacteroides");
        assert_eq!(shorten_number("0.5"), "0.5");
    }
}
