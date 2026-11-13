//! The main window.
//!
//! The layout follows STAMP v2: profile and group settings in one side
//! panel; the kind of analysis with its statistical properties and filters in
//! the other; the plot in the middle, with the table of features beside it
//! for plots of a single feature and the choice of plot beneath it.

use eframe::egui::{self, Color32, RichText};
use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::effect_size::{GroupEffectSize, SampleEffectSize};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::post_hoc::PostHocTest;
use stamp_core::stats::results::{
    format_p_value, FilterOperator, GroupCountFilter, SampleCountFilter,
};
use stamp_core::stats::table::{group_table, multi_group_table, sample_table, Table};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::TestType;
use stamp_core::Unclassified;
use stamp_plot::Figure;

use crate::figure_painter;
use crate::platform::{self, Event, Platform, Request, Source};
use crate::plot_config;
use crate::plots::{self, PlotKind, PlotSettings};
use crate::session::{Mode, Relation, Session};

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

/// Width of the combo boxes in the side panels, in points.
const COMBO_WIDTH: f32 = 132.0;

/// A combo box over a fixed list of choices. Returns true if the choice
/// changed.
pub(crate) fn choose<T: PartialEq + Clone>(
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
    // A fixed width keeps the side panels narrow; long choices are cut short
    // in the box and shown in full when pointed at.
    let combo = egui::ComboBox::from_id_salt(id)
        .selected_text(selected.clone())
        .width(COMBO_WIDTH)
        .truncate()
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
    combo.response.on_hover_text(selected);
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

/// Text in a table cell. Unlike an ordinary label its text cannot be
/// selected, so the pointer keeps its arrow and a click anywhere in the cell
/// reaches the row.
fn plain_label(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) {
    ui.add(egui::Label::new(text).selectable(false));
}

/// A labelled row of a settings grid.
fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    ui.label(label);
    let changed = add(ui);
    ui.end_row();
    changed
}

/// A section of a side panel that can be folded away, open to begin with.
fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(RichText::new(title).heading())
        .default_open(true)
        .show(ui, add);
    ui.add_space(4.0);
}

/// A sequence-count filter as the user set it: whether it is on, its kind
/// (an index into the kinds offered) and its one or two thresholds.
#[derive(Debug, Clone, PartialEq)]
struct CountFilterUi {
    enabled: bool,
    kind: usize,
    first: f64,
    second: f64,
}

impl Default for CountFilterUi {
    fn default() -> Self {
        CountFilterUi {
            enabled: false,
            kind: 0,
            first: 1.0,
            second: 1.0,
        }
    }
}

impl CountFilterUi {
    /// Rows of a filter grid: the kind with its on/off box, then the
    /// thresholds. `second_label` names the second sample or group.
    fn rows(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        label: &str,
        kinds: &[&str],
        second_label: &str,
    ) {
        let options: Vec<(usize, String)> = kinds
            .iter()
            .enumerate()
            .map(|(i, k)| (i, k.to_string()))
            .collect();
        ui.checkbox(&mut self.enabled, label);
        ui.add_enabled_ui(self.enabled, |ui| choose(ui, id, &mut self.kind, &options));
        ui.end_row();

        let independent = kinds[self.kind].starts_with("independent");
        let first_label = if independent {
            second_label.replace('2', "1")
        } else {
            // "Maximum (<):" or "Minimum (<):"
            let mut kind = kinds[self.kind].to_string();
            kind[..1].make_ascii_uppercase();
            format!("{kind} (<):")
        };
        ui.add_enabled(self.enabled, egui::Label::new(first_label));
        ui.add_enabled(
            self.enabled,
            egui::DragValue::new(&mut self.first)
                .speed(1.0)
                .range(0.0..=f64::MAX),
        );
        ui.end_row();
        if independent {
            ui.add_enabled(self.enabled, egui::Label::new(second_label));
            ui.add_enabled(
                self.enabled,
                egui::DragValue::new(&mut self.second)
                    .speed(1.0)
                    .range(0.0..=f64::MAX),
            );
            ui.end_row();
        }
    }

    fn sample(&self) -> Option<SampleCountFilter> {
        self.enabled.then_some(match self.kind {
            0 => SampleCountFilter::Maximum(self.first),
            1 => SampleCountFilter::Minimum(self.first),
            _ => SampleCountFilter::Independent(self.first, self.second),
        })
    }

    fn group(&self) -> Option<GroupCountFilter> {
        self.enabled.then_some(match self.kind {
            0 => GroupCountFilter::Maximum(self.first),
            1 => GroupCountFilter::Minimum(self.first),
            2 => GroupCountFilter::IndependentMinimum(self.first, self.second),
            _ => GroupCountFilter::IndependentMaximum(self.first, self.second),
        })
    }
}

/// The user's filter settings, kept even while a filter is switched off.
#[derive(Debug, Clone, PartialEq)]
struct FilterUi {
    p_enabled: bool,
    p_value: f64,
    sequences: CountFilterUi,
    parent_sequences: CountFilterUi,
    effect1_enabled: bool,
    effect1_measure: usize,
    effect1_minimum: f64,
    effect2_enabled: bool,
    effect2_measure: usize,
    effect2_minimum: f64,
    operator: FilterOperator,
}

impl Default for FilterUi {
    fn default() -> Self {
        FilterUi {
            p_enabled: false,
            p_value: 0.05,
            sequences: CountFilterUi::default(),
            parent_sequences: CountFilterUi::default(),
            effect1_enabled: false,
            effect1_measure: 0,
            effect1_minimum: 1.0,
            effect2_enabled: false,
            effect2_measure: 1,
            effect2_minimum: 2.0,
            operator: FilterOperator::Or,
        }
    }
}

impl FilterUi {
    /// Controls for the filters, in the order of STAMP v2: the p-value
    /// filter, the sequence filters, the effect-size filters and the number
    /// of features left, each set apart by a line.
    ///
    /// `measures` names the effect-size measures and `count_kinds` the
    /// sequence-count filters; an empty `count_kinds` hides those filters.
    /// `second_label` is "Sample 2 (<):" or "Group 2 (<):".
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        measures: &[&str],
        count_kinds: &[&str],
        second_label: &str,
        active_features: (usize, usize),
    ) -> bool {
        let before = self.clone();

        egui::Grid::new(format!("{id}-p-filter"))
            .num_columns(2)
            .show(ui, |ui| {
                ui.checkbox(&mut self.p_enabled, "p-value filter (>):");
                ui.add_enabled(
                    self.p_enabled,
                    egui::DragValue::new(&mut self.p_value)
                        .speed(0.001)
                        .range(0.0..=1.0),
                );
                ui.end_row();
            });

        if !count_kinds.is_empty() {
            ui.separator();
            egui::Grid::new(format!("{id}-count-filters"))
                .num_columns(2)
                .show(ui, |ui| {
                    self.sequences.rows(
                        ui,
                        &format!("{id}-seq"),
                        "Sequence filter:",
                        count_kinds,
                        second_label,
                    );
                    self.parent_sequences.rows(
                        ui,
                        &format!("{id}-parent-seq"),
                        "Parent seq. filter:",
                        count_kinds,
                        second_label,
                    );
                });
        }

        ui.separator();
        let options: Vec<(usize, String)> = measures
            .iter()
            .enumerate()
            .map(|(i, m)| (i, m.to_string()))
            .collect();
        let two = measures.len() > 1;
        egui::Grid::new(format!("{id}-effect-filters"))
            .num_columns(2)
            .show(ui, |ui| {
                let label = if two {
                    "Effect size filter 1:"
                } else {
                    "Effect size filter (<):"
                };
                ui.checkbox(&mut self.effect1_enabled, label);
                if two {
                    ui.add_enabled_ui(self.effect1_enabled, |ui| {
                        choose(
                            ui,
                            &format!("{id}-effect1"),
                            &mut self.effect1_measure,
                            &options,
                        )
                    });
                    ui.end_row();
                    ui.add_enabled(self.effect1_enabled, egui::Label::new("Effect size (<):"));
                }
                ui.add_enabled(
                    self.effect1_enabled,
                    egui::DragValue::new(&mut self.effect1_minimum)
                        .speed(0.05)
                        .range(0.0..=f64::MAX),
                );
                ui.end_row();

                if two {
                    let both = self.effect1_enabled && self.effect2_enabled;
                    ui.add_enabled(both, egui::Label::new("Remove a feature failing"));
                    ui.add_enabled_ui(both, |ui| {
                        choose(
                            ui,
                            &format!("{id}-operator"),
                            &mut self.operator,
                            &[
                                (FilterOperator::Or, "either filter (OR)".to_string()),
                                (FilterOperator::And, "both filters (AND)".to_string()),
                            ],
                        )
                    });
                    ui.end_row();

                    ui.checkbox(&mut self.effect2_enabled, "Effect size filter 2:");
                    ui.add_enabled_ui(self.effect2_enabled, |ui| {
                        choose(
                            ui,
                            &format!("{id}-effect2"),
                            &mut self.effect2_measure,
                            &options,
                        )
                    });
                    ui.end_row();
                    ui.add_enabled(self.effect2_enabled, egui::Label::new("Effect size (<):"));
                    ui.add_enabled(
                        self.effect2_enabled,
                        egui::DragValue::new(&mut self.effect2_minimum)
                            .speed(0.05)
                            .range(0.0..=f64::MAX),
                    );
                    ui.end_row();
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Number active features:");
            let (total, active) = active_features;
            ui.label(RichText::new(active.to_string()).strong())
                .on_hover_text(format!("{active} of {total} features pass the filters"));
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

/// The rule in the metadata table for switching samples on or off: add or
/// remove all samples whose `field` stands in `relation` to `value`.
#[derive(Debug, Clone, PartialEq)]
struct MetadataFilterUi {
    add: bool,
    field: String,
    relation: Relation,
    value: String,
}

impl Default for MetadataFilterUi {
    fn default() -> Self {
        MetadataFilterUi {
            add: true,
            field: String::new(),
            relation: Relation::Equal,
            value: String::new(),
        }
    }
}

/// One row of the table of features shown beside the plot.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureRow {
    pub feature: String,
    pub effect_size: String,
    pub p_value: String,
    pub p_value_corrected: String,
    pub note: String,
    /// Whether the feature passes the filters.
    pub active: bool,
}

/// The STAMP application: one analysis session and its window.
pub struct StampApp {
    pub session: Session,
    pub mode: Mode,
    plot_kinds: [PlotKind; 3],
    plot_settings: PlotSettings,
    zoom: f32,
    filters: [FilterUi; 3],
    /// The current plot and tables, rebuilt when `view_dirty` is set.
    figure: Figure,
    table: Table,
    table_features: Vec<String>,
    /// Every feature of the current analysis, in alphabetical order.
    feature_rows: Vec<FeatureRow>,
    /// Heading of the effect-size column of the feature table.
    effect_heading: &'static str,
    view_dirty: bool,
    seen_generation: u64,
    /// Which parts of the window are shown (the View menu).
    show_profile_panel: bool,
    show_properties_panel: bool,
    show_feature_table: bool,
    show_only_active_features: bool,
    show_metadata_table: bool,
    show_plot_config: bool,
    metadata_filter: MetadataFilterUi,
    /// The hierarchical level and the name at that level whose features are
    /// drawn in red.
    highlight_level: Option<String>,
    highlight_value: String,
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

fn empty_table() -> (Table, Vec<String>) {
    (
        Table {
            headings: Vec::new(),
            rows: Vec::new(),
        },
        Vec::new(),
    )
}

/// Switch the whole interface between its dark and light appearance.
pub fn set_dark_mode(ctx: &egui::Context, dark: bool) {
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
}

/// In the light theme, the text of controls that can be used is black;
/// egui's own light theme draws it in grey. Controls that are switched off
/// are still faded.
fn use_black_text_in_light_mode(ctx: &egui::Context) {
    ctx.style_mut_of(egui::Theme::Light, |style| {
        let widgets = &mut style.visuals.widgets;
        for state in [
            &mut widgets.noninteractive,
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            state.fg_stroke.color = Color32::BLACK;
        }
    });
}

impl StampApp {
    /// Create the application. Registers the plot font with `ctx`.
    pub fn new(ctx: &egui::Context) -> StampApp {
        figure_painter::install_plot_font(ctx);
        use_black_text_in_light_mode(ctx);
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
            feature_rows: Vec::new(),
            effect_heading: "Effect size",
            view_dirty: true,
            seen_generation: 0,
            show_profile_panel: true,
            show_properties_panel: true,
            show_feature_table: true,
            show_only_active_features: false,
            show_metadata_table: false,
            show_plot_config: false,
            metadata_filter: MetadataFilterUi::default(),
            highlight_level: None,
            highlight_value: String::new(),
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

    /// Show or hide the side panels, as the View menu does: the profile and
    /// groups panel, and the properties (statistics and filtering) panel.
    pub fn show_side_panels(&mut self, profile: bool, properties: bool) {
        self.show_profile_panel = profile;
        self.show_properties_panel = properties;
    }

    /// Show or hide the table of features beside the plot.
    pub fn show_feature_table(&mut self, show: bool) {
        self.show_feature_table = show;
    }

    /// Show or hide the metadata table along the bottom of the window.
    pub fn show_metadata_table(&mut self, show: bool) {
        self.show_metadata_table = show;
    }

    /// Open or close the "Configure plot" window.
    pub fn show_plot_config(&mut self, show: bool) {
        self.show_plot_config = show;
    }

    /// The settings of every plot.
    pub fn plot_settings(&self) -> &PlotSettings {
        &self.plot_settings
    }

    /// Replace the settings of the plots, as the "Configure plot" window
    /// does.
    pub fn set_plot_settings(&mut self, settings: PlotSettings) {
        self.plot_settings = settings;
        self.view_dirty = true;
    }

    /// The profile of the analysis being shown.
    fn current_profile(&self) -> Option<&stamp_core::profile::Profile> {
        let s = &self.session;
        match self.mode {
            Mode::TwoSamples => s.samples.output.as_ref().map(|(p, _)| &p.profile),
            Mode::TwoGroups => s.groups.output.as_ref().map(|(p, _)| &p.profile),
            Mode::MultipleGroups => s.multi.output.as_ref().map(|(p, _)| &p.profile),
        }
    }

    /// The names found at a hierarchical level of the current profile, in
    /// alphabetical order: the choices for highlighting.
    pub fn highlight_choices(&self, level: &str) -> Vec<String> {
        let Some(profile) = self.current_profile() else {
            return Vec::new();
        };
        let Some(index) = profile.hierarchy_headings.iter().position(|h| h == level) else {
            return Vec::new();
        };
        let mut names: Vec<String> = profile
            .features()
            .iter()
            .filter_map(|f| f.hierarchy.get(index).cloned())
            .collect();
        names.sort_by(|a, b| (a.to_lowercase(), a).cmp(&(b.to_lowercase(), b)));
        names.dedup();
        names
    }

    /// Draw in red the features that fall under `value` at a hierarchical
    /// level, in the plots that support it. `None` highlights nothing.
    pub fn set_highlight(&mut self, level: Option<&str>, value: &str) {
        self.highlight_level = level.map(str::to_string);
        self.highlight_value = value.to_string();
        self.view_dirty = true;
    }

    /// The features currently highlighted.
    pub fn highlighted_features(&self) -> Vec<String> {
        let (Some(level), Some(profile)) = (&self.highlight_level, self.current_profile()) else {
            return Vec::new();
        };
        let Some(index) = profile.hierarchy_headings.iter().position(|h| h == level) else {
            return Vec::new();
        };
        profile
            .features()
            .iter()
            .filter(|f| f.hierarchy.get(index) == Some(&self.highlight_value))
            .map(|f| f.name.clone())
            .collect()
    }

    /// The figure currently shown.
    pub fn figure(&self) -> &Figure {
        &self.figure
    }

    /// The results table as shown: the rows that pass the filters.
    pub fn results_table(&self) -> &Table {
        &self.table
    }

    /// The features listed beside single-feature plots, honouring the "show
    /// only active features" choice.
    pub fn feature_table(&self) -> Vec<&FeatureRow> {
        self.feature_rows
            .iter()
            .filter(|row| row.active || !self.show_only_active_features)
            .collect()
    }

    pub fn set_show_only_active_features(&mut self, only_active: bool) {
        self.show_only_active_features = only_active;
    }

    /// Select the feature shown by the single-feature plots.
    pub fn select_feature(&mut self, feature: &str) {
        self.session.selected_feature = Some(feature.to_string());
        self.view_dirty = true;
    }

    /// The zoom of the plot; 1 shows one figure point per screen point.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.5, 4.0);
    }

    /// Return the plot to the zoom it starts with.
    pub fn reset_zoom(&mut self) {
        self.zoom = DEFAULT_ZOOM;
    }

    /// The message currently shown to the user, if any.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Load a profile from the text of a file.
    pub fn load_profile(&mut self, name: &str, text: &str) {
        match self.session.load_profile(name, text) {
            Ok(()) => {
                self.filters = Default::default();
                self.highlight_level = None;
                self.highlight_value.clear();
                self.metadata_filter = MetadataFilterUi::default();
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
        filter.sequences = f.sequences.sample();
        filter.parent_sequences = f.parent_sequences.sample();

        let f = &self.filters[mode_slot(Mode::TwoGroups)];
        let filter = &mut self.session.groups.filter;
        filter.significance = f.p_enabled.then_some(f.p_value);
        let measure = |index: usize| GroupEffectSize::ALL[index.min(1)];
        filter.effect_size1 =
            effect(f.effect1_enabled, f.effect1_minimum).map(|m| (measure(f.effect1_measure), m));
        filter.effect_size2 =
            effect(f.effect2_enabled, f.effect2_minimum).map(|m| (measure(f.effect2_measure), m));
        filter.operator = f.operator;
        filter.sequences = f.sequences.group();
        filter.parent_sequences = f.parent_sequences.group();

        let f = &self.filters[mode_slot(Mode::MultipleGroups)];
        let filter = &mut self.session.multi.filter;
        filter.significance = f.p_enabled.then_some(f.p_value);
        filter.min_effect_size = effect(f.effect1_enabled, f.effect1_minimum);
        self.view_dirty = true;
    }

    /// Rebuild the plot and the tables from the session.
    fn refresh_view(&mut self) {
        self.plot_settings.highlighted = self.highlighted_features();
        self.figure = plots::build(
            &self.session,
            self.mode,
            self.plot_kind(),
            &self.plot_settings,
        );
        let s = &self.session;
        let feature_row =
            |feature: &str, effect: f64, p: f64, corrected: f64, note: &str| FeatureRow {
                feature: feature.to_string(),
                effect_size: format!("{effect:.3}"),
                p_value: format_p_value(p, -15),
                p_value_corrected: format_p_value(corrected, -15),
                note: note.to_string(),
                active: false,
            };

        let mut feature_rows = Vec::new();
        let mut effect_heading = "Effect size";
        let (table, features): (Table, Vec<String>) = match self.mode {
            Mode::TwoSamples => match &s.samples.output {
                Some((profile, results)) => {
                    feature_rows = results
                        .rows
                        .iter()
                        .map(|r| {
                            feature_row(
                                &r.feature,
                                r.effect_size,
                                r.p_value,
                                r.p_value_corrected,
                                &r.note,
                            )
                        })
                        .collect();
                    let rows = s.sample_rows();
                    let features = rows.iter().map(|r| r.feature.clone()).collect();
                    (sample_table(profile, results, rows), features)
                }
                None => empty_table(),
            },
            Mode::TwoGroups => match &s.groups.output {
                Some((profile, results)) => {
                    effect_heading = "Diff. between means";
                    feature_rows = results
                        .rows
                        .iter()
                        .map(|r| {
                            feature_row(
                                &r.feature,
                                r.effect_size,
                                r.p_value,
                                r.p_value_corrected,
                                &r.note,
                            )
                        })
                        .collect();
                    let rows = s.group_rows();
                    let features = rows.iter().map(|r| r.feature.clone()).collect();
                    (group_table(profile, results, rows), features)
                }
                None => empty_table(),
            },
            Mode::MultipleGroups => match &s.multi.output {
                Some((profile, results)) => {
                    effect_heading = "Eta-squared";
                    feature_rows = results
                        .rows
                        .iter()
                        .map(|r| {
                            feature_row(
                                &r.feature,
                                r.effect_size,
                                r.p_value,
                                r.p_value_corrected,
                                &r.note,
                            )
                        })
                        .collect();
                    let rows = s.multi_rows();
                    let features = rows.iter().map(|r| r.feature.clone()).collect();
                    (multi_group_table(profile, results, rows), features)
                }
                None => empty_table(),
            },
        };
        for row in feature_rows.iter_mut() {
            row.active = features.contains(&row.feature);
        }
        // The feature table starts in alphabetical order, as in STAMP v2.
        feature_rows.sort_by(|a, b| a.feature.cmp(&b.feature));

        self.table = table;
        self.table_features = features;
        self.feature_rows = feature_rows;
        self.effect_heading = effect_heading;
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
        if self.show_metadata_table {
            egui::Panel::bottom("metadata")
                .resizable(true)
                .default_size(220.0)
                .show(ui, |ui| self.metadata_panel(ui));
        }
        if self.show_profile_panel {
            egui::Panel::left("profile")
                .default_size(215.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.profile_panel(ui));
                });
        }
        if self.show_properties_panel {
            egui::Panel::right("properties")
                .default_size(285.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.properties_panel(ui));
                });
        }
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
                let save_table = egui::Button::new("Save results table...");
                if ui
                    .add_enabled(!self.table.rows.is_empty(), save_table)
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
                ui.checkbox(&mut self.show_profile_panel, "Profile and groups");
                ui.checkbox(&mut self.show_properties_panel, "Properties");
                ui.checkbox(&mut self.show_feature_table, "Feature table");
                ui.checkbox(&mut self.show_metadata_table, "Metadata table");
                ui.separator();
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

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.session.tree.is_none() {
                ui.label("No profile loaded");
                return;
            }
            let (total, active) = self.session.feature_counts(self.mode);
            ui.label(format!("Features: {total}, active: {active}"));
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
        section(ui, "Profile", |ui| self.profile_section(ui));
        section(ui, "Groups", |ui| self.groups_section(ui));
    }

    fn profile_section(&mut self, ui: &mut egui::Ui) {
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
                changed |= row(ui, "Profile level:", |ui| {
                    choose(
                        ui,
                        "profile-level",
                        &mut self.session.profile_level,
                        &strings(headings),
                    )
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
    }

    fn groups_section(&mut self, ui: &mut egui::Ui) {
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
        let Some(groups) = self.session.grouping() else {
            return;
        };
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
                let checkbox = egui::Checkbox::new(&mut active, format!("{name} ({size})"));
                if ui.add_enabled(size > 0, checkbox).changed() {
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

    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        // The kind of analysis decides what the sections below contain.
        ui.horizontal_wrapped(|ui| {
            for mode in Mode::ALL {
                let tab = ui.selectable_label(self.mode == mode, mode.name());
                if tab.clicked() && self.mode != mode {
                    self.set_mode(mode);
                }
            }
        });
        ui.separator();
        section(ui, "Statistics", |ui| self.statistics_section(ui));
        section(ui, "Filtering", |ui| self.filtering_section(ui));
    }

    fn statistics_section(&mut self, ui: &mut egui::Ui) {
        if self.session.tree.is_none() {
            ui.label("Load a profile to choose a statistical test.");
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
                            let tests = named(&SampleTest::ALL, SampleTest::name);
                            choose(ui, "sample-test", &mut state.analysis.test, &tests)
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
                            let methods = named(&SampleCi::ALL, SampleCi::name);
                            choose(ui, "sample-ci", &mut state.analysis.ci_method, &methods)
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
                            let tests = named(&GroupTest::ALL, GroupTest::name);
                            choose(ui, "group-test", &mut state.analysis.test, &tests)
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
                            let tests = named(&MultiGroupTest::ALL, MultiGroupTest::name);
                            choose(ui, "multi-test", &mut state.analysis.test, &tests)
                        });
                        post_hoc_changed |= row(ui, "Post-hoc test:", |ui| {
                            let tests = named(&PostHocTest::ALL, PostHocTest::name);
                            choose(ui, "post-hoc-test", &mut state.post_hoc_test, &tests)
                        });
                        post_hoc_changed |= row(ui, "Post-hoc coverage:", |ui| {
                            choose(
                                ui,
                                "post-hoc-coverage",
                                &mut state.post_hoc_coverage,
                                &coverages,
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
    }

    fn filtering_section(&mut self, ui: &mut egui::Ui) {
        let mode = self.mode;
        let available = match mode {
            Mode::TwoSamples => self.session.tree.is_some(),
            _ => self.session.metadata.is_some(),
        };
        if !available {
            ui.label("Filters apply once an analysis has been run.");
            return;
        }
        let counts = self.session.feature_counts(mode);
        let filter = &mut self.filters[mode_slot(mode)];
        let changed = match mode {
            Mode::TwoSamples => {
                let measures: Vec<&str> = SampleEffectSize::ALL.iter().map(|m| m.name()).collect();
                filter.show(
                    ui,
                    "samples",
                    &measures,
                    &SAMPLE_COUNT_KINDS,
                    "Sample 2 (<):",
                    counts,
                )
            }
            Mode::TwoGroups => {
                let measures: Vec<&str> = GroupEffectSize::ALL.iter().map(|m| m.name()).collect();
                filter.show(
                    ui,
                    "groups",
                    &measures,
                    &GROUP_COUNT_KINDS,
                    "Group 2 (<):",
                    counts,
                )
            }
            Mode::MultipleGroups => filter.show(ui, "multi", &["Eta-squared"], &[], "", counts),
        };
        if changed {
            self.apply_filters();
        }
    }

    /// The middle of the window: the plot, the choice of plot beneath it,
    /// and the table of features beside it for plots of a single feature.
    fn plot_panel(&mut self, ui: &mut egui::Ui) {
        if self.view_dirty {
            self.refresh_view();
        }
        egui::Panel::bottom("plot-bar").show(ui, |ui| self.plot_bar(ui));
        if self.show_feature_table && self.session.tree.is_some() {
            egui::Panel::right("features")
                .default_size(270.0)
                .show(ui, |ui| self.feature_panel(ui));
        }

        egui::CentralPanel::default().show(ui, |ui| {
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
                        if self.feature_rows.iter().any(|row| row.feature == item) {
                            self.select_feature(&item);
                        }
                    }
                });
        });
    }

    /// The controls beneath the plot, in one row where there is room: the
    /// choice of plot with its settings, the zoom, and the features to
    /// highlight.
    fn plot_bar(&mut self, ui: &mut egui::Ui) {
        let headings: Vec<String> = self
            .current_profile()
            .map(|p| p.hierarchy_headings.clone())
            .unwrap_or_default();
        let supported = self.plot_kind().supports_highlight(self.mode) && !headings.is_empty();

        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            // A group of controls stays together: it starts a new row when
            // the window is too narrow for it, and otherwise follows a
            // dividing line.
            let divide = |ui: &mut egui::Ui, room: f32| {
                if ui.max_rect().right() - ui.cursor().left() < room {
                    ui.end_row();
                } else {
                    ui.separator();
                }
            };

            let mut kind = self.plot_kind();
            let kinds = named(PlotKind::available(self.mode), PlotKind::name);
            if choose(ui, "plot-kind", &mut kind, &kinds) {
                self.set_plot_kind(kind);
            }
            let configure = ui
                .button("Configure plot")
                .on_hover_text("Change the size and appearance of this plot");
            if configure.clicked() {
                self.show_plot_config = true;
            }

            divide(ui, 240.0);
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.zoom, 0.5..=4.0).show_value(false));
            let reset = ui.add_enabled(self.zoom != DEFAULT_ZOOM, egui::Button::new("Reset"));
            if reset.on_hover_text("Return to the default zoom").clicked() {
                self.reset_zoom();
            }

            divide(ui, 400.0);
            // Everything from here on is switched off for plots that cannot
            // highlight features.
            if !supported {
                ui.disable();
            }
            ui.label("Highlight:")
                .on_hover_text("Draw the features of one category in red");
            let mut levels: Vec<(Option<String>, String)> = vec![(None, "None".to_string())];
            levels.extend(headings.iter().map(|h| (Some(h.clone()), h.clone())));
            // A level above the profile level may have gone.
            let mut level = self
                .highlight_level
                .clone()
                .filter(|level| headings.contains(level));
            let level_changed = choose(ui, "highlight-level", &mut level, &levels);
            let Some(level) = level else {
                if self.highlight_level.is_some() {
                    self.set_highlight(None, "");
                }
                // The second box is shown, empty, until a level is chosen.
                ui.add_enabled_ui(false, |ui| {
                    choose(ui, "highlight-feature", &mut String::new(), &[])
                });
                return;
            };
            let choices = self.highlight_choices(&level);
            let mut value = self.highlight_value.clone();
            if level_changed || !choices.contains(&value) {
                value = choices.first().cloned().unwrap_or_default();
            }
            choose(ui, "highlight-feature", &mut value, &strings(&choices));
            if self.highlight_level.as_deref() != Some(level.as_str())
                || self.highlight_value != value
            {
                self.set_highlight(Some(&level), &value);
            }
        });
        ui.add_space(2.0);
    }

    /// The table of sample metadata. Unticking a sample leaves it out of the
    /// group analyses; the filter ticks or unticks samples by their metadata.
    fn metadata_panel(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};

        let StampApp {
            session,
            metadata_filter: filter,
            ..
        } = self;
        ui.add_space(2.0);
        let Some(metadata) = &session.metadata else {
            ui.label("Use File > Load metadata to see the metadata of the samples.");
            return;
        };
        let fields = metadata.fields();
        let samples = metadata.samples();
        if !fields.contains(&filter.field) {
            filter.field = fields.first().cloned().unwrap_or_default();
        }
        let mut values: Vec<String> = metadata
            .unique_values(&filter.field)
            .into_iter()
            .map(str::to_string)
            .collect();
        if metadata.is_numeric(&filter.field) {
            let number = |text: &String| text.trim().parse::<f64>().unwrap_or(f64::NAN);
            values.sort_by(|a, b| number(a).total_cmp(&number(b)));
        } else {
            values.sort_by_key(|value| value.to_lowercase());
        }
        if !values.contains(&filter.value) {
            filter.value = values.first().cloned().unwrap_or_default();
        }

        // Changes are collected here and made once the table is drawn.
        let mut switch: Option<(Vec<String>, bool)> = None;
        ui.horizontal_wrapped(|ui| {
            ui.strong("Metadata");
            ui.separator();
            let all = ui
                .button("Select all")
                .on_hover_text("Set all samples as active");
            if all.clicked() {
                switch = Some((samples.to_vec(), true));
            }
            let none = ui
                .button("Remove all")
                .on_hover_text("Remove all samples from the active set");
            if none.clicked() {
                switch = Some((samples.to_vec(), false));
            }
            ui.separator();
            ui.label("Filter:");
            choose(
                ui,
                "metadata-add",
                &mut filter.add,
                &[(true, "Add".to_string()), (false, "Remove".to_string())],
            );
            ui.label("all samples where");
            choose(ui, "metadata-field", &mut filter.field, &strings(fields));
            let relations = named(&Relation::ALL, Relation::name);
            choose(ui, "metadata-relation", &mut filter.relation, &relations);
            choose(ui, "metadata-value", &mut filter.value, &strings(&values));
            let apply = ui
                .button("Apply")
                .on_hover_text("Apply the filtering criteria");
            if apply.clicked() {
                let matching =
                    session.samples_matching(&filter.field, filter.relation, &filter.value);
                switch = Some((matching, filter.add));
            }
            ui.separator();
            ui.label(format!(
                "{} of {} samples active",
                metadata.active_samples().len(),
                samples.len()
            ));
        });
        ui.separator();

        egui::ScrollArea::horizontal().show(ui, |ui| {
            TableBuilder::new(ui)
                .id_salt("metadata-table")
                .striped(true)
                .column(Column::auto().at_least(150.0).resizable(true))
                .columns(
                    Column::auto().at_least(80.0).clip(true).resizable(true),
                    fields.len(),
                )
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Sample");
                    });
                    for field in fields {
                        header.col(|ui| {
                            ui.strong(field);
                        });
                    }
                })
                .body(|body| {
                    body.rows(20.0, samples.len(), |mut table_row| {
                        let sample = &samples[table_row.index()];
                        table_row.col(|ui| {
                            let mut active = metadata.is_active(sample);
                            if ui.checkbox(&mut active, sample).changed() {
                                switch = Some((vec![sample.clone()], active));
                            }
                        });
                        for field in fields {
                            table_row.col(|ui| {
                                plain_label(ui, metadata.value(sample, field).unwrap_or_default());
                            });
                        }
                    });
                });
        });
        if let Some((samples, active)) = switch {
            session.set_samples_active(&samples, active);
        }
    }

    /// The table of features beside a single-feature plot. Clicking a row
    /// shows that feature in the plot.
    fn feature_panel(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};

        ui.add_space(2.0);
        ui.checkbox(
            &mut self.show_only_active_features,
            "Show only active features",
        );
        ui.separator();
        let rows: Vec<&FeatureRow> = self
            .feature_rows
            .iter()
            .filter(|row| row.active || !self.show_only_active_features)
            .collect();
        if rows.is_empty() {
            ui.label("No features to show.");
            return;
        }
        let selected = self.session.selected_feature.clone();
        let headings = [
            "Feature",
            self.effect_heading,
            "p-value",
            "Corrected p-value",
            "Note",
        ];
        let mut select = None;
        egui::ScrollArea::horizontal().show(ui, |ui| {
            TableBuilder::new(ui)
                .id_salt("feature-table")
                .striped(true)
                .sense(egui::Sense::click())
                .column(Column::auto().at_least(110.0).clip(true).resizable(true))
                .columns(Column::auto().at_least(60.0).resizable(true), 3)
                .column(Column::remainder().at_least(60.0).clip(true))
                .header(20.0, |mut header| {
                    for heading in headings {
                        header.col(|ui| {
                            ui.strong(heading);
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, rows.len(), |mut table_row| {
                        let feature = rows[table_row.index()];
                        table_row
                            .set_selected(selected.as_deref() == Some(feature.feature.as_str()));
                        for cell in [
                            &feature.feature,
                            &feature.effect_size,
                            &feature.p_value,
                            &feature.p_value_corrected,
                            &feature.note,
                        ] {
                            table_row.col(|ui| {
                                plain_label(ui, cell);
                            });
                        }
                        let response = table_row
                            .response()
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if response.clicked() {
                            select = Some(feature.feature.clone());
                        }
                    });
                });
        });
        if let Some(feature) = select {
            self.select_feature(&feature);
        }
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
                        .id_salt("results-table")
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
                                        plain_label(ui, shorten_number(cell));
                                    });
                                }
                                let response = row
                                    .response()
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                if response.clicked() {
                                    select = self.table_features.get(index).cloned();
                                }
                            });
                        });
                });
            });
        if let Some(feature) = select {
            self.select_feature(&feature);
        }
        if download {
            self.download_table(ctx);
        }

        let (mode, kind) = (self.mode, self.plot_kind());
        let mut open = self.show_plot_config;
        let (mut changed, mut reset, mut close) = (false, false, false);
        egui::Window::new("Configure plot")
            .open(&mut open)
            .default_pos([300.0, 90.0])
            .resizable(false)
            .show(ctx, |ui| {
                ui.strong(kind.name());
                ui.separator();
                changed = plot_config::show(ui, mode, kind, &mut self.plot_settings);
                ui.separator();
                ui.horizontal(|ui| {
                    reset = ui.button("Restore defaults").clicked();
                    close = ui.button("Close").clicked();
                });
            });
        if reset {
            self.plot_settings.reset(mode, kind);
        }
        self.view_dirty |= changed || reset;
        self.show_plot_config = open && !close;

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
                            let replicates =
                                egui::DragValue::new(&mut self.session.stats.replicates)
                                    .range(10..=1_000_000);
                            ui.add(replicates).changed()
                        });
                        changed |= row(ui, "Pseudocount:", |ui| {
                            let pseudocount =
                                egui::DragValue::new(&mut self.session.stats.pseudocount)
                                    .speed(0.05)
                                    .range(0.0..=10.0);
                            ui.add(pseudocount).changed()
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

    #[test]
    fn count_filters_translate_to_the_core_filters() {
        let mut filter = CountFilterUi::default();
        assert_eq!(filter.sample(), None);
        assert_eq!(filter.group(), None);
        filter.enabled = true;
        filter.first = 5.0;
        filter.second = 7.0;
        assert_eq!(filter.sample(), Some(SampleCountFilter::Maximum(5.0)));
        filter.kind = 2;
        assert_eq!(
            filter.sample(),
            Some(SampleCountFilter::Independent(5.0, 7.0))
        );
        assert_eq!(
            filter.group(),
            Some(GroupCountFilter::IndependentMinimum(5.0, 7.0))
        );
        filter.kind = 3;
        assert_eq!(
            filter.group(),
            Some(GroupCountFilter::IndependentMaximum(5.0, 7.0))
        );
    }

    #[test]
    fn filter_controls_reach_the_session() {
        let mut app = StampApp::new(&egui::Context::default());
        app.load_example();

        let groups = &mut app.filters[mode_slot(Mode::TwoGroups)];
        groups.p_enabled = true;
        groups.p_value = 0.01;
        groups.parent_sequences = CountFilterUi {
            enabled: true,
            kind: 1,
            first: 500.0,
            second: 0.0,
        };
        groups.sequences = CountFilterUi {
            enabled: true,
            kind: 3,
            first: 10.0,
            second: 20.0,
        };
        let samples = &mut app.filters[mode_slot(Mode::TwoSamples)];
        samples.parent_sequences = CountFilterUi {
            enabled: true,
            kind: 2,
            first: 3.0,
            second: 4.0,
        };
        app.apply_filters();

        let filter = &app.session.groups.filter;
        assert_eq!(filter.significance, Some(0.01));
        assert_eq!(
            filter.parent_sequences,
            Some(GroupCountFilter::Minimum(500.0))
        );
        assert_eq!(
            filter.sequences,
            Some(GroupCountFilter::IndependentMaximum(10.0, 20.0))
        );
        assert_eq!(
            app.session.samples.filter.parent_sequences,
            Some(SampleCountFilter::Independent(3.0, 4.0))
        );
        assert_eq!(app.session.samples.filter.sequences, None);
    }
}
