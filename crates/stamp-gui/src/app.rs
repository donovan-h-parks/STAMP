//! The main window.
//!
//! The layout follows STAMP v2: profile and group settings in one side
//! panel; the kind of analysis with its statistical properties and filters in
//! the other; the plot in the middle, with the table of features beside it
//! for plots of a single feature and the choice of plot beneath it.

use eframe::egui::{self, Color32, RichText};
use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::effect_size::{GroupEffectSize, SampleEffectSize};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::post_hoc::PostHocTest;
use stamp_core::stats::results::Transform;
use stamp_core::stats::results::{
    format_p_value, FilterOperator, GroupCountFilter, SampleCountFilter,
};
use stamp_core::stats::table::{
    correlation_table, group_table, multi_group_table, sample_table, Table,
};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::TestType;
use stamp_core::Unclassified;
use stamp_plot::Figure;

use crate::assistant;
use crate::figure_painter;
use crate::icons;
use crate::jobs::Job;
use crate::layout::{self, Frame};
use crate::platform::{self, Auxiliary, Event, OpenedFile, Platform, Request};
use crate::plot_config;
use crate::plots::{self, PlotKind, PlotSettings};
use crate::select_features::{FeatureSelection, Outcome};
use crate::session::{Mode, Relation, Session};

const COVERAGES: [f64; 5] = [0.90, 0.95, 0.98, 0.99, 0.999];

/// Zoom of the plot when the application starts, and after a reset. A plot
/// of the default width fits beside the panels in a 1280-pixel window.
/// Number of plots kept, so that returning to one is instant.
const FIGURE_CACHE_SIZE: usize = 6;
/// Plots with more shapes than this (large heatmaps) are not kept.
const FIGURE_CACHE_SHAPES: usize = 200_000;
const DEFAULT_ZOOM: f32 = 1.15;
/// Seconds a slow plot computes before its progress is shown, so that
/// quick ones do not make the window flicker.
const PROGRESS_DELAY: f64 = 0.3;
/// What the plot area says after the user cancelled the calculation of a plot.
const PLOT_CANCELLED: &str = "The calculation of this plot was cancelled";

/// Example data built into the application: gut enterotypes from Arumugam et
/// al. (2011).
const ENTEROTYPES_PROFILE: &str =
    include_str!("../../../examples/EnterotypesArumugam/Enterotypes.profile.spf");
const ENTEROTYPES_METADATA: &str =
    include_str!("../../../examples/EnterotypesArumugam/Enterotypes.metadata.tsv");
/// Microbial communities of coal seams, from An et al. (2013); a profile of
/// sequence counts down to species. Compressed when the program is built.
const COAL_PROFILE_GZ: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/coal.spf.gz"));
const COAL_METADATA: &str = include_str!("../../../examples/CoalSeamGas/coal.metadata.tsv");
/// Gut metagenomes containing the Eggerthellaceae genera Equivita,
/// Gorillivita and Tapirivita (Parks et al., 2026), profiled by SingleM and
/// taken from Sandpiper: a genus profile of 1,289 samples, a profile of the
/// species of these three genera, and their metadata. Compressed when the
/// program is built.
const EGGERTHELLACEAE_PROFILE_GZ: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/eggerthellaceae.spf.gz"));
const EGGERTHELLACEAE_SPECIES_GZ: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/eggerthellaceae_mgr_species.spf.gz"
));
const EGGERTHELLACEAE_METADATA_GZ: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/eggerthellaceae.metadata.tsv.gz"));

/// The text of an example compressed by the build script.
fn unpack(data: &[u8]) -> Result<String, String> {
    use std::io::Read;
    let mut text = String::new();
    flate2::read::GzDecoder::new(data)
        .read_to_string(&mut text)
        .map_err(|error| format!("The example could not be read: {error}"))?;
    Ok(text)
}

/// The datasets built into the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Example {
    /// Proportions of phyla and genera in gut samples of three enterotypes.
    Enterotypes,
    /// Counts of sequences in coal-seam samples, with a full taxonomy.
    CoalSeamGas,
    /// SingleM coverage of genera in 1,289 gut metagenomes of mammals.
    EggerthellaceaeGenera,
    /// The same samples, with the species of the three focus genera.
    EggerthellaceaeSpecies,
}

impl Example {
    pub const ALL: [Example; 4] = [
        Example::Enterotypes,
        Example::CoalSeamGas,
        Example::EggerthellaceaeGenera,
        Example::EggerthellaceaeSpecies,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Example::Enterotypes => "Enterotypes",
            Example::CoalSeamGas => "Coal seam gas",
            Example::EggerthellaceaeGenera => "Eggerthellaceae (genera)",
            Example::EggerthellaceaeSpecies => "Eggerthellaceae (focus species)",
        }
    }

    /// What the dataset is, for a tooltip.
    pub fn description(self) -> &'static str {
        match self {
            Example::Enterotypes => {
                "Gut enterotypes (Arumugam et al., 2011): proportions of phyla and genera in 39 samples"
            }
            Example::CoalSeamGas => {
                "Coal seam microbial communities (An et al., 2013): sequence counts in 44 samples, \
                 domain to species"
            }
            Example::EggerthellaceaeGenera => {
                "Gut metagenomes of mammals with the Eggerthellaceae genera Equivita, Gorillivita \
                 or Tapirivita (Parks et al., 2026): SingleM coverage of 544 genera in 1,289 \
                 samples from Sandpiper, grouped by host"
            }
            Example::EggerthellaceaeSpecies => {
                "The same 1,289 metagenomes: SingleM coverage of the species of Equivita, \
                 Gorillivita and Tapirivita, with the rest of the community used only for \
                 proportions"
            }
        }
    }
}

/// Width of the combo boxes in the side panels, in points.
const COMBO_WIDTH: f32 = 132.0;

/// A combo box over a fixed list of choices. Returns true if the choice
/// changed.
/// The plot selector: the plots of a mode under a heading for each group,
/// with the checks on the tests after a divider. Returns true if the user
/// chose another plot.
fn choose_plot(ui: &mut egui::Ui, mode: Mode, current: &mut PlotKind) -> bool {
    let mut changed = false;
    let combo = egui::ComboBox::from_id_salt("plot-kind")
        .selected_text(current.name())
        .width(COMBO_WIDTH)
        .truncate()
        // Tall enough for every plot of a mode without scrolling, where the
        // window has room.
        .height(ui.ctx().content_rect().height())
        .show_ui(ui, |ui| {
            ui.set_min_width(200.0);
            // Rows close together, so that a mode's plots fit without
            // scrolling.
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.spacing_mut().button_padding.y = 1.0;
            for (index, (group, kinds)) in PlotKind::grouped(mode).into_iter().enumerate() {
                if index > 0 {
                    ui.add_space(2.0);
                }
                // The size of the plot names beneath, in the strong (brighter)
                // text colour: egui's interface font has no bold.
                let heading = egui::TextStyle::Button.resolve(ui.style()).size;
                ui.label(egui::RichText::new(group.name()).strong().size(heading));
                let mut divided = false;
                for kind in kinds {
                    if kind.checks_the_tests() && !divided {
                        ui.separator();
                        divided = true;
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        if ui.selectable_label(*current == kind, kind.name()).clicked()
                            && *current != kind
                        {
                            *current = kind;
                            changed = true;
                        }
                    });
                }
            }
        });
    combo.response.on_hover_text(current.name());
    changed
}

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

/// Order of two table cells: numbers by value (a p-value shown as a bound,
/// such as `< 1e-15`, by its bound), ahead of text, which is compared
/// without regard to case.
fn compare_cells(a: &str, b: &str) -> std::cmp::Ordering {
    let number = |cell: &str| {
        cell.trim()
            .trim_start_matches('<')
            .trim()
            .parse::<f64>()
            .ok()
    };
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

/// The column a table is sorted by, chosen by clicking its heading.
/// Clicking the same heading again reverses the order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SortState {
    column: Option<usize>,
    descending: bool,
}

impl SortState {
    /// Sort by `column`, or reverse the order if it is already sorted by it.
    pub fn click(&mut self, column: usize) {
        if self.column == Some(column) {
            self.descending = !self.descending;
        } else {
            *self = SortState {
                column: Some(column),
                descending: false,
            };
        }
    }

    /// The order in which to show `count` rows, given the text of each
    /// row's cell in a column. Rows that compare equal keep their order.
    pub fn order<'a>(&self, count: usize, cell: impl Fn(usize, usize) -> &'a str) -> Vec<usize> {
        let mut order: Vec<usize> = (0..count).collect();
        if let Some(column) = self.column {
            order.sort_by(|&a, &b| {
                let ordering = compare_cells(cell(a, column), cell(b, column));
                if self.descending {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
        }
        order
    }

    /// A column heading that sorts the table when clicked. An arrow marks
    /// the sorted column: pointing up for ascending order.
    fn heading(&mut self, ui: &mut egui::Ui, column: usize, text: &str) {
        let mut clicked = false;
        // Wrapped, so that a long heading does not widen its column.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            if self.column == Some(column) {
                // Drawn rather than written: the fonts lack arrow glyphs.
                let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 12.0), egui::Sense::hover());
                let (centre, half) = (rect.center(), 3.5);
                let tip = if self.descending { half } else { -half };
                let points = vec![
                    egui::pos2(centre.x, centre.y + tip),
                    egui::pos2(centre.x + half, centre.y - tip),
                    egui::pos2(centre.x - half, centre.y - tip),
                ];
                let color = ui.visuals().strong_text_color();
                ui.painter().add(egui::Shape::convex_polygon(
                    points,
                    color,
                    egui::Stroke::NONE,
                ));
            }
            let label = egui::Label::new(RichText::new(text).strong())
                .selectable(false)
                .sense(egui::Sense::click());
            clicked = ui
                .add(label)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Click to sort by this column; click again to reverse the order")
                .clicked();
        });
        if clicked {
            self.click(column);
        }
    }
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
        CountFilterUi::starting_at(1.0)
    }
}

impl CountFilterUi {
    /// A filter that is off, with both thresholds at `threshold`.
    fn starting_at(threshold: f64) -> CountFilterUi {
        CountFilterUi {
            enabled: false,
            kind: 0,
            first: threshold,
            second: threshold,
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
            2 => GroupCountFilter::IndependentMaximum(self.first, self.second),
            _ => GroupCountFilter::IndependentMinimum(self.first, self.second),
        })
    }
}

/// The filters each analysis starts with, in the order of `Mode::ALL`. They
/// are those of STAMP v2: the multiple-group effect size, eta-squared, lies
/// between zero and one, so its filter starts at 0.8.
/// What the transform does, for the hover text of its control.
const TRANSFORM_HELP: &str = "Centred log-ratio: p-values are computed from the logarithm of \
    each feature relative to the geometric mean of its sample, which removes the constraint \
    that proportions sum to one. Effect sizes, confidence intervals and plots stay in \
    proportions.";

fn starting_filters() -> [FilterUi; 4] {
    let mut filters = [
        FilterUi::default(),
        FilterUi::default(),
        FilterUi::default(),
        FilterUi::default(),
    ];
    filters[mode_slot(Mode::MultipleGroups)].effect1_minimum = 0.8;
    // A correlation coefficient also lies within one of zero.
    filters[mode_slot(Mode::Correlation)].effect1_minimum = 0.5;
    filters
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
            p_enabled: true,
            p_value: 0.05,
            // The thresholds STAMP v2 starts with.
            sequences: CountFilterUi::starting_at(5.0),
            parent_sequences: CountFilterUi::starting_at(1.0),
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
    #[allow(clippy::too_many_arguments)]
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        measures: &[&str],
        count_kinds: &[&str],
        second_label: &str,
        active_features: (usize, usize),
        q_values: bool,
    ) -> bool {
        let before = self.clone();
        // A false discovery rate gives q-values; the plots name them so too.
        let p_label = if q_values {
            "q-value filter (>):"
        } else {
            "p-value filter (>):"
        };

        egui::Grid::new(format!("{id}-p-filter"))
            .num_columns(2)
            .show(ui, |ui| {
                ui.checkbox(&mut self.p_enabled, p_label);
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
    "independent, maximum",
    "independent, minimum",
];

/// Keyboard shortcuts, as in STAMP v2. In a browser some of them belong to
/// the browser itself and never reach the page.
mod shortcut {
    use eframe::egui::{Key, KeyboardShortcut, Modifiers};

    const fn command(key: Key) -> KeyboardShortcut {
        KeyboardShortcut::new(Modifiers::COMMAND, key)
    }
    pub const LOAD_PROFILE: KeyboardShortcut = command(Key::O);
    pub const SAVE_PLOT: KeyboardShortcut = command(Key::S);
    pub const EXIT: KeyboardShortcut = command(Key::Q);
    pub const PLOT_WINDOW: KeyboardShortcut = command(Key::W);
    pub const PROPERTIES: KeyboardShortcut = command(Key::P);
    pub const GROUPS: KeyboardShortcut = command(Key::L);
    pub const METADATA: KeyboardShortcut = command(Key::M);
    pub const RESULTS: KeyboardShortcut = command(Key::T);
}

/// A profile in another format that is being opened: the file, and what
/// still has to be chosen before a STAMP profile can be made from it.
enum PendingProfile {
    /// A BIOM table: which observation metadata gives the hierarchy.
    Biom {
        file: OpenedFile,
        fields: Vec<String>,
        chosen: Option<String>,
    },
    /// A mothur taxonomy file: its groups file and, optionally, names file.
    Mothur {
        taxonomy: OpenedFile,
        groups: Option<OpenedFile>,
        names: Option<OpenedFile>,
    },
}

/// A plot sent to a window of its own: a copy that stays as it was while
/// the main window moves on.
struct PlotWindow {
    id: u64,
    title: String,
    figure: Figure,
}

/// What is remembered from one run to the next: the layout of the window,
/// the preferences and the settings of the plots.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Saved {
    pub dark_mode: bool,
    pub show_profile_panel: bool,
    pub show_properties_panel: bool,
    pub show_feature_table: bool,
    pub show_metadata_table: bool,
    pub zoom: f32,
    pub seed: u64,
    pub replicates: usize,
    pub pseudocount: f64,
    pub other_samples_color: stamp_plot::Color,
    pub plots: PlotSettings,
    /// The [`DEFAULTS_VERSION`] the plot settings were saved under; zero
    /// for settings saved before there was one.
    pub defaults_version: u32,
    /// Open Claude Code without first showing what it will share ("Don't
    /// show this again").
    pub skip_claude_code_dialog: bool,
    /// Where the frames of the window are, and the [`layout::LAYOUT_VERSION`]
    /// it was saved under (zero before there was one).
    pub layout: Option<egui_tiles::Tree<Frame>>,
    pub layout_version: u32,
}

/// Identifies the starting values of the plot settings. Remembered plot
/// settings are only restored if they were saved under the same number;
/// otherwise the plots start from the current defaults. **Increase it
/// whenever a default in `PlotSettings::default()` changes**, or people who
/// have run an earlier version will keep the old value without having
/// chosen it.
///
/// 1: legends placed by "Best" (were at fixed corners, or absent).
/// 2: feature names and p-values of the extended error bar and post-hoc
///    plots at 9 points (were 8).
pub const DEFAULTS_VERSION: u32 = 2;

impl Saved {
    /// The same, with plot settings saved under other defaults replaced by
    /// the current ones. Everything else that is remembered is kept.
    pub fn up_to_date(mut self) -> Saved {
        if self.defaults_version != DEFAULTS_VERSION {
            self.plots = PlotSettings::default();
            self.defaults_version = DEFAULTS_VERSION;
        }
        self
    }
}

impl Default for Saved {
    fn default() -> Self {
        let session = Session::default();
        Saved {
            dark_mode: true,
            show_profile_panel: true,
            show_properties_panel: true,
            show_feature_table: true,
            show_metadata_table: false,
            zoom: DEFAULT_ZOOM,
            seed: session.seed,
            replicates: session.stats.replicates,
            pseudocount: session.stats.pseudocount,
            other_samples_color: session.other_samples_color,
            plots: PlotSettings::default(),
            // What settings read from an older store, lacking the field,
            // are taken to be.
            defaults_version: 0,
            skip_claude_code_dialog: false,
            layout: None,
            layout_version: 0,
        }
    }
}

/// Key under which [`Saved`] is stored.
const STORAGE_KEY: &str = "stamp";

/// The features chosen by hand for one kind of analysis ("Select specific
/// features"). The choice is kept while it is switched off.
#[derive(Debug, Clone, Default, PartialEq)]
struct SelectionUi {
    enabled: bool,
    /// `None` until features have been chosen: every feature is selected.
    chosen: Option<std::collections::HashSet<String>>,
}

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

impl FeatureRow {
    /// The cells of the row, in the order of the table's columns.
    fn cells(&self) -> [&str; 5] {
        [
            &self.feature,
            &self.effect_size,
            &self.p_value,
            &self.p_value_corrected,
            &self.note,
        ]
    }
}

/// The STAMP application: one analysis session and its window.
pub struct StampApp {
    pub session: Session,
    pub mode: Mode,
    plot_kinds: [PlotKind; 4],
    plot_settings: PlotSettings,
    zoom: f32,
    filters: [FilterUi; 4],
    selections: [SelectionUi; 4],
    /// The "Select features" window, while it is open, and the analysis it
    /// belongs to.
    select_dialog: Option<(Mode, FeatureSelection)>,
    /// The current plot and tables, rebuilt when `view_dirty` is set.
    figure: Figure,
    /// Plots drawn recently, by a key describing everything they were drawn
    /// from, oldest first.
    figure_cache: Vec<(String, Figure)>,
    /// Number of plots drawn, as opposed to taken from the cache.
    figures_built: usize,
    /// The slow part of the current plot, while it is computed, and when
    /// that started (in the time of the user interface, once known).
    plot_job: Option<(plots::RunningPlot, Option<f64>)>,
    /// The plot shown before, drawn greyed out while the current one is
    /// computed.
    stale_figure: Option<Figure>,
    /// The calculation the user cancelled, not to be started again until
    /// asked.
    cancelled_plot: Option<String>,
    /// A profile being read (and converted) from files, with what is said
    /// meanwhile and when that started.
    loading: Option<Loading>,
    table: Table,
    table_features: Vec<String>,
    /// What the results table was last built from (see `refresh_view`).
    table_key: String,
    /// A large results table still being built.
    table_pending: Option<TablePending>,
    /// For saving the results table once built.
    table_context: Option<egui::Context>,
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
    /// The sorting of the feature, results and metadata tables.
    feature_sort: SortState,
    table_sort: SortState,
    metadata_sort: SortState,
    show_metadata_table: bool,
    show_plot_config: bool,
    show_correction_info: bool,
    /// Show only the features that pass the filters in the results table.
    results_only_active: bool,
    /// The resolution being chosen for a PNG image, while that window is
    /// open.
    png_dpi: Option<f64>,
    plot_windows: Vec<PlotWindow>,
    next_plot_window: u64,
    pending_profile: Option<PendingProfile>,
    /// The theme of the last frame, for storing between runs.
    saved_theme_is_dark: bool,
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
    /// The assistant: its requests, log and (on the desktop) server.
    pub(crate) assistant: assistant::State,
    /// How things were before each change the assistant made, most recent
    /// last.
    assistant_undo: Vec<AssistantSnapshot>,
    /// Where the frames of the window are.
    layout: egui_tiles::Tree<Frame>,
}

/// What the assistant's tools can change, as it was before a change, so
/// that the change can be undone.
#[derive(Clone)]
struct AssistantSnapshot {
    mode: Mode,
    plot_kinds: [PlotKind; 4],
    plot_settings: PlotSettings,
    selected_feature: Option<String>,
    filters: [FilterUi; 4],
    analysis: AnalysisSettings,
    group_colors: std::collections::HashMap<String, stamp_plot::Color>,
    other_samples_color: stamp_plot::Color,
    switched_off: Vec<String>,
}

/// The settings of the analyses that the assistant can change.
#[derive(Clone, PartialEq)]
struct AnalysisSettings {
    profile_level: String,
    parent_level: Option<String>,
    group_field: Option<String>,
    samples: (String, String, stamp_core::stats::results::SampleAnalysis),
    groups: (
        String,
        Option<String>,
        Option<String>,
        stamp_core::stats::results::GroupAnalysis,
    ),
    multi: (
        Vec<String>,
        stamp_core::stats::results::MultiGroupAnalysis,
        PostHocTest,
        f64,
    ),
    correlation: (
        Option<String>,
        stamp_core::stats::results::CorrelationAnalysis,
    ),
}

impl AnalysisSettings {
    fn of(session: &Session) -> AnalysisSettings {
        AnalysisSettings {
            profile_level: session.profile_level.clone(),
            parent_level: session.parent_level.clone(),
            group_field: session.group_field.clone(),
            samples: (
                session.samples.sample1.clone(),
                session.samples.sample2.clone(),
                session.samples.analysis,
            ),
            groups: (
                session.groups.group1.clone(),
                session.groups.group2.clone(),
                session.groups.pair_field.clone(),
                session.groups.analysis,
            ),
            multi: (
                session.multi.active_groups.clone(),
                session.multi.analysis,
                session.multi.post_hoc_test,
                session.multi.post_hoc_coverage,
            ),
            correlation: (
                session.correlation.field.clone(),
                session.correlation.analysis,
            ),
        }
    }

    fn restore(self, session: &mut Session) {
        session.profile_level = self.profile_level;
        session.parent_level = self.parent_level;
        session.group_field = self.group_field;
        (
            session.samples.sample1,
            session.samples.sample2,
            session.samples.analysis,
        ) = self.samples;
        (
            session.groups.group1,
            session.groups.group2,
            session.groups.pair_field,
            session.groups.analysis,
        ) = self.groups;
        (
            session.multi.active_groups,
            session.multi.analysis,
            session.multi.post_hoc_test,
            session.multi.post_hoc_coverage,
        ) = self.multi;
        (session.correlation.field, session.correlation.analysis) = self.correlation;
    }
}

/// Most changes by the assistant that can be undone.
const MAX_ASSISTANT_UNDO: usize = 50;

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
                PlotKind::available(Mode::Correlation)[0],
            ],
            plot_settings: PlotSettings::default(),
            zoom: DEFAULT_ZOOM,
            filters: starting_filters(),
            selections: Default::default(),
            select_dialog: None,
            figure: Figure::message("Load a profile to begin"),
            figure_cache: Vec::new(),
            figures_built: 0,
            plot_job: None,
            stale_figure: None,
            cancelled_plot: None,
            loading: None,
            table: Table {
                headings: Vec::new(),
                rows: Vec::new(),
            },
            table_features: Vec::new(),
            table_key: String::new(),
            table_pending: None,
            table_context: None,
            feature_rows: Vec::new(),
            effect_heading: "Effect size",
            view_dirty: true,
            seen_generation: 0,
            show_profile_panel: true,
            show_properties_panel: true,
            show_feature_table: true,
            show_only_active_features: false,
            feature_sort: SortState::default(),
            table_sort: SortState::default(),
            metadata_sort: SortState::default(),
            show_metadata_table: false,
            show_plot_config: false,
            show_correction_info: false,
            results_only_active: false,
            png_dpi: None,
            plot_windows: Vec::new(),
            next_plot_window: 1,
            pending_profile: None,
            saved_theme_is_dark: true,
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
            assistant: assistant::State::default(),
            assistant_undo: Vec::new(),
            layout: layout::default_tree(),
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

    /// Number of plots drawn so far, not counting those reused unchanged.
    pub fn figures_built(&self) -> usize {
        self.figures_built
    }

    /// Number of ordination layouts (PCA, PCoA with its tests, NMDS)
    /// computed so far, not counting those reused when only the look of the
    /// plot changed.
    pub fn ordinations_computed(&self) -> usize {
        self.session.ordinations.computed()
    }

    /// While the slow part of the current plot is computed: what it is
    /// doing, and the fraction done (`None` when that cannot be told).
    pub fn plot_progress(&self) -> Option<(String, Option<f32>)> {
        let (job, _) = self.plot_job.as_ref()?;
        let progress = job.progress();
        let (_, total) = progress.counts();
        Some((progress.stage(), (total > 0).then(|| progress.fraction())))
    }

    /// True while a profile is read, or an analysis or the slow part of
    /// the plot is computed.
    pub fn is_busy(&self) -> bool {
        self.session.is_running(self.mode) || self.plot_job.is_some() || self.loading.is_some()
    }

    /// What is said while a profile is read from files.
    pub fn loading(&self) -> Option<&str> {
        self.loading.as_ref().map(|loading| loading.label.as_str())
    }

    /// Stop reading a profile, as the Cancel button beside its progress
    /// does: the profile loaded before stays.
    pub fn cancel_loading(&mut self) {
        self.loading = None;
    }

    /// Read (and convert) files that make a profile without asking
    /// anything, in a job: large ones take a while, in the browser most of
    /// all. Files that first need choices (BIOM, mothur) are opened at once.
    fn start_loading(&mut self, files: Vec<OpenedFile>) {
        use stamp_core::import::biom;
        let needs_choices = matches!(files.as_slice(), [file]
            if biom::looks_like_biom(&file.bytes)
                && !stamp_core::import::sandpiper::looks_like_sandpiper(&file.bytes)
            || file.name.to_lowercase().ends_with(".taxonomy"));
        if files.is_empty() || needs_choices {
            self.open_profile_files(files);
            return;
        }
        self.pending_profile = None;
        let label = match files.as_slice() {
            [file] => format!("Reading {}", file.name),
            _ => format!("Reading {} files", files.len()),
        };
        self.loading = Some(Loading::start(label, move || {
            let (name, spf, hint) = profile_text(&files).ok_or("no profile")??;
            match stamp_core::ProfileTree::read(&spf) {
                Ok(tree) => Ok((name, tree)),
                Err(error) => {
                    let mut notice = Some(format!("Could not load {name}:\n{error}"));
                    add_hint(&mut notice, hint);
                    Err(notice.unwrap_or_default())
                }
            }
        }));
    }

    /// Read a profile already converted, in a job.
    fn start_loading_text(&mut self, name: String, spf: String) {
        self.loading = Some(Loading::start(format!("Reading {name}"), move || {
            stamp_core::ProfileTree::read(&spf)
                .map(|tree| (name.clone(), tree))
                .map_err(|error| format!("Could not load {name}:\n{error}"))
        }));
    }

    /// Use the profile once read.
    fn poll_loading(&mut self, ctx: &egui::Context) {
        let Some(loading) = &mut self.loading else {
            return;
        };
        match loading.job.poll() {
            Err(true) => {
                if loading.started.is_none() {
                    loading.started = Some(ctx.input(|i| i.time));
                }
                let wait = if cfg!(target_arch = "wasm32") { 0 } else { 50 };
                ctx.request_repaint_after(std::time::Duration::from_millis(wait));
            }
            outcome => {
                self.loading = None;
                match outcome {
                    Ok(Ok((name, tree))) => {
                        self.notice = None;
                        self.use_profile(&name, Ok(tree));
                    }
                    Ok(Err(notice)) => self.notice = Some(notice),
                    _ => {
                        self.notice = Some("Reading the profile stopped unexpectedly.".to_string())
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    /// Stop computing the current plot, as its Cancel button does.
    pub fn cancel_plot(&mut self) {
        if let Some((job, _)) = self.plot_job.take() {
            job.cancel();
            self.cancelled_plot = Some(job.id());
            self.figure = Figure::message(PLOT_CANCELLED);
            self.stale_figure = None;
        }
    }

    /// Compute a cancelled plot after all, as its Compute button does.
    pub fn compute_plot(&mut self) {
        self.cancelled_plot = None;
        self.view_dirty = true;
    }

    /// Open or close the Preferences window, as Settings > Preferences does.
    pub fn show_preferences(&mut self, show: bool) {
        self.show_preferences = show;
    }

    /// Limit the current analysis to features chosen by hand, as "Select
    /// specific features" does; `None` returns to every feature.
    pub fn set_selected_features(&mut self, features: Option<&[&str]>) {
        let selection = &mut self.selections[mode_slot(self.mode)];
        selection.enabled = features.is_some();
        selection.chosen = features.map(|names| names.iter().map(|n| n.to_string()).collect());
        self.apply_filters();
    }

    /// Open the "Select features" window for the current analysis.
    pub fn open_select_features(&mut self) {
        let chosen = self.selections[mode_slot(self.mode)].chosen.as_ref();
        if let Some(profile) = self.current_profile() {
            self.select_dialog = Some((self.mode, FeatureSelection::new(profile, chosen)));
        }
    }

    /// Open the files chosen with File > Load profile: one profile in any
    /// format that is read, or several Sandpiper profiles (one per sample),
    /// which are combined.
    pub fn open_profile_files(&mut self, mut files: Vec<OpenedFile>) {
        if files.len() == 1 {
            let file = files.pop().expect("one file");
            self.open_profile_file(file);
            return;
        }
        if files.is_empty() {
            return;
        }
        self.pending_profile = None;
        match profile_text(&files) {
            Some(Ok((name, spf, hint))) => {
                self.notice = None;
                self.load_profile(&name, &spf);
                add_hint(&mut self.notice, hint);
            }
            Some(Err(notice)) => self.notice = Some(notice),
            None => {}
        }
    }

    /// Open a profile in any of the formats that are read: a STAMP profile,
    /// a SingleM condensed profile or a Sandpiper profile is loaded at once;
    /// a BIOM table (JSON or HDF5) or a mothur taxonomy file first asks for
    /// what it needs.
    pub fn open_profile_file(&mut self, file: OpenedFile) {
        use stamp_core::import::biom;
        self.pending_profile = None;
        if let Some(converted) = profile_text(std::slice::from_ref(&file)) {
            match converted {
                Ok((name, spf, hint)) => {
                    self.notice = None;
                    self.load_profile(&name, &spf);
                    add_hint(&mut self.notice, hint);
                }
                Err(notice) => self.notice = Some(notice),
            }
        } else if biom::looks_like_biom(&file.bytes) {
            match biom::metadata_fields(&file.bytes) {
                // Without metadata there is nothing to choose.
                Ok(fields) if fields.is_empty() => {
                    self.pending_profile = Some(PendingProfile::Biom {
                        file,
                        fields,
                        chosen: None,
                    });
                    self.accept_pending_profile();
                }
                Ok(fields) => {
                    // The fields the usual pipelines write are offered first.
                    let chosen = biom::COMMON_METADATA_FIELDS
                        .iter()
                        .map(|(field, _)| field.to_string())
                        .find(|field| fields.contains(field));
                    self.pending_profile = Some(PendingProfile::Biom {
                        file,
                        fields,
                        chosen,
                    });
                }
                Err(error) => {
                    self.notice = Some(format!("Could not load {}:\n{error}", file.name));
                }
            }
        } else if file.name.to_lowercase().ends_with(".taxonomy") {
            self.pending_profile = Some(PendingProfile::Mothur {
                taxonomy: file,
                groups: None,
                names: None,
            });
        }
    }

    /// The kind of profile waiting for a choice before it can be loaded:
    /// `"BIOM"` or `"mothur"`.
    pub fn pending_profile(&self) -> Option<&'static str> {
        match self.pending_profile {
            Some(PendingProfile::Biom { .. }) => Some("BIOM"),
            Some(PendingProfile::Mothur { .. }) => Some("mothur"),
            None => None,
        }
    }

    /// Choose the observation metadata that gives a BIOM table's hierarchy;
    /// `None` uses the observation ids alone.
    pub fn choose_biom_field(&mut self, field: Option<&str>) {
        if let Some(PendingProfile::Biom { chosen, .. }) = &mut self.pending_profile {
            *chosen = field.map(str::to_string);
        }
    }

    /// Make a STAMP profile from the file being opened, with the choices
    /// made so far, and load it.
    pub fn accept_pending_profile(&mut self) {
        use stamp_core::import::{biom, mothur};
        let (name, converted) = match self.pending_profile.take() {
            Some(PendingProfile::Biom { file, chosen, .. }) => {
                let spf = biom::to_spf(&file.bytes, chosen.as_deref());
                (file.name, spf)
            }
            Some(PendingProfile::Mothur {
                taxonomy,
                groups: Some(groups),
                names,
            }) => {
                let names = names.map(|file| file.text());
                let spf = mothur::to_spf(&taxonomy.text(), &groups.text(), names.as_deref());
                (taxonomy.name, spf)
            }
            // A groups file is required.
            other => {
                self.pending_profile = other;
                return;
            }
        };
        match converted {
            Ok(spf) => self.load_profile(&name, &spf),
            Err(error) => self.notice = Some(format!("Could not load {name}:\n{error}")),
        }
    }

    /// What would be remembered for the next run.
    pub fn saved(&self, ctx: &egui::Context) -> Saved {
        Saved {
            dark_mode: ctx.theme() == egui::Theme::Dark,
            show_profile_panel: self.show_profile_panel,
            show_properties_panel: self.show_properties_panel,
            show_feature_table: self.show_feature_table,
            show_metadata_table: self.show_metadata_table,
            zoom: self.zoom,
            seed: self.session.seed,
            replicates: self.session.stats.replicates,
            pseudocount: self.session.stats.pseudocount,
            other_samples_color: self.session.other_samples_color,
            plots: self.plot_settings.clone(),
            defaults_version: DEFAULTS_VERSION,
            skip_claude_code_dialog: self.skip_claude_code_dialog(),
            layout: Some(self.layout.clone()),
            layout_version: layout::LAYOUT_VERSION,
        }
    }

    /// Whether Assistant > Open Claude Code skips its dialog.
    pub fn skip_claude_code_dialog(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.assistant.skip_claude_code_dialog
        }
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// Skip (or show again) the dialog of Assistant > Open Claude Code, as
    /// its "Don't show this again" box and Preferences do.
    pub fn set_skip_claude_code_dialog(&mut self, skip: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.assistant.skip_claude_code_dialog = skip;
        }
        #[cfg(target_arch = "wasm32")]
        let _ = skip;
    }

    /// Return to what was remembered from an earlier run.
    pub fn restore(&mut self, ctx: &egui::Context, saved: Saved) {
        set_dark_mode(ctx, saved.dark_mode);
        self.show_profile_panel = saved.show_profile_panel;
        self.show_properties_panel = saved.show_properties_panel;
        self.show_feature_table = saved.show_feature_table;
        self.show_metadata_table = saved.show_metadata_table;
        self.set_zoom(saved.zoom);
        self.session.seed = saved.seed;
        self.session.stats.replicates = saved.replicates.clamp(10, 1_000_000);
        self.session.stats.pseudocount = saved.pseudocount.clamp(0.0, 10.0);
        self.session.other_samples_color = saved.other_samples_color;
        self.plot_settings = saved.plots;
        self.set_skip_claude_code_dialog(saved.skip_claude_code_dialog);
        // A layout from another version of STAMP, or missing a frame, is
        // replaced by the default.
        self.layout = saved
            .layout
            .filter(|tree| {
                saved.layout_version == layout::LAYOUT_VERSION && layout::is_complete(tree)
            })
            .unwrap_or_else(layout::default_tree);
        self.session.invalidate_all();
        self.view_dirty = true;
    }

    /// Restore what the last run stored, if anything.
    pub fn restore_from(&mut self, ctx: &egui::Context, storage: Option<&dyn eframe::Storage>) {
        if let Some(saved) = storage.and_then(|s| eframe::get_value::<Saved>(s, STORAGE_KEY)) {
            self.restore(ctx, saved.up_to_date());
        }
    }

    /// Send a copy of the current plot to a window of its own.
    pub fn send_plot_to_window(&mut self) {
        if self.session.tree.is_none() {
            return;
        }
        let id = self.next_plot_window;
        self.next_plot_window += 1;
        self.plot_windows.push(PlotWindow {
            id,
            title: format!("{} ({id})", self.plot_kind().name()),
            figure: self.figure.clone(),
        });
    }

    /// The number of plots open in windows of their own.
    pub fn plot_window_count(&self) -> usize {
        self.plot_windows.len()
    }

    /// Act on the keyboard shortcuts pressed this frame.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Keys typed into Claude Code are Claude Code's.
        if self.assistant_terminal_has_focus() {
            return;
        }
        let pressed =
            |shortcut: &egui::KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(shortcut));
        let loaded = self.session.tree.is_some();
        if pressed(&shortcut::LOAD_PROFILE) {
            self.platform.request(Request::OpenProfile, ctx);
        }
        if pressed(&shortcut::SAVE_PLOT) && loaded {
            self.png_dpi = Some(platform::DEFAULT_DPI);
        }
        if pressed(&shortcut::PLOT_WINDOW) {
            self.send_plot_to_window();
        }
        if pressed(&shortcut::PROPERTIES) {
            self.show_properties_panel = !self.show_properties_panel;
        }
        if pressed(&shortcut::GROUPS) {
            self.show_profile_panel = !self.show_profile_panel;
        }
        if pressed(&shortcut::METADATA) {
            self.show_metadata_table = !self.show_metadata_table;
        }
        if pressed(&shortcut::RESULTS) {
            self.show_table = !self.show_table;
        }
        if cfg!(not(target_arch = "wasm32")) && pressed(&shortcut::EXIT) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Switch the p-value filter of every analysis on, at a level, or off.
    pub fn set_p_value_filter(&mut self, level: Option<f64>) {
        for filter in &mut self.filters {
            filter.p_enabled = level.is_some();
            if let Some(level) = level {
                filter.p_value = level;
            }
        }
        self.apply_filters();
    }

    /// Show every feature in the results table, or only those that pass the
    /// filters.
    pub fn set_results_only_active(&mut self, only_active: bool) {
        self.results_only_active = only_active;
        self.view_dirty = true;
    }

    /// Open or close the window of information on the multiple test
    /// correction.
    pub fn show_correction_info(&mut self, show: bool) {
        self.show_correction_info = show;
    }

    /// The lines of the correction information window: the method, its
    /// summary values and any caution.
    pub fn correction_info(&self) -> Vec<String> {
        let s = &self.session;
        let summary = match self.mode {
            Mode::TwoSamples => s.samples.output.as_ref().map(|(_, r)| &r.correction),
            Mode::TwoGroups => s.groups.output.as_ref().map(|(_, r)| &r.correction),
            Mode::MultipleGroups => s.multi.output.as_ref().map(|(_, r)| &r.correction),
            Mode::Correlation => s.correlation.output.as_ref().map(|(_, r)| &r.correction),
        };
        let Some(summary) = summary else {
            return vec!["Run a hypothesis test first.".to_string()];
        };
        let mut lines = vec![format!("Method: {}", summary.method.name())];
        for (label, value) in &summary.info {
            let value = if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                format!("{value:.6}")
            };
            lines.push(format!("{label}: {value}"));
        }
        if let Some(warning) = summary.warning {
            lines.push(warning.to_string());
        }
        lines
    }

    /// The summary of the profile shown in the status bar, as in STAMP v2:
    /// parent categories, features and the sequences being compared.
    pub fn status_summary(&self) -> String {
        let Some(tree) = &self.session.tree else {
            return "No profile loaded".to_string();
        };
        // Profiles of proportions have fractional totals; whole numbers read
        // better here.
        let total = |samples: &[String]| -> String {
            let sum: f64 = samples.iter().filter_map(|s| tree.sample_total(s)).sum();
            format!("{sum:.0}")
        };
        let s = &self.session;
        let (profile, sequences) = match self.mode {
            Mode::TwoSamples => (
                s.samples.output.as_ref().map(|(p, _)| &p.profile),
                s.samples.output.as_ref().map(|(p, _)| {
                    let names = &p.profile.sample_names;
                    format!(
                        ", Sequences in sample 1: {}, Sequences in sample 2: {}",
                        total(&names[..1]),
                        total(&names[1..2])
                    )
                }),
            ),
            Mode::TwoGroups => (
                s.groups.output.as_ref().map(|(p, _)| &p.profile),
                s.groups.output.as_ref().map(|(p, _)| {
                    format!(
                        ", Sequences in group 1: {}, Sequences in group 2: {}",
                        total(p.samples_in_group1()),
                        total(p.samples_in_group2())
                    )
                }),
            ),
            Mode::MultipleGroups => (s.multi.output.as_ref().map(|(p, _)| &p.profile), None),
            Mode::Correlation => (
                s.correlation.output.as_ref().map(|(p, _)| &p.profile),
                s.correlation.output.as_ref().map(|(p, _)| {
                    let left_out = match p.without_value {
                        0 => String::new(),
                        n => format!(" ({n} without a number left out)"),
                    };
                    format!(", Samples: {}{left_out}", p.values.len())
                }),
            ),
        };
        let Some(profile) = profile else {
            return String::new();
        };
        let (_, active) = s.feature_counts(self.mode);
        format!(
            "Parent categories: {}, Features: {}{}, Active features: {active}",
            profile.num_parent_categories(),
            profile.num_features(),
            sequences.unwrap_or_default()
        )
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

    /// Draw the plot again, after a change to something it is drawn from.
    pub fn redraw_plot(&mut self) {
        self.view_dirty = true;
    }

    /// Change the profile level and the parent level (`None` for the
    /// entire sample), as the Profile panel does.
    pub fn set_profile_level(&mut self, level: &str, parent: Option<&str>) {
        self.session.profile_level = level.to_string();
        self.session.parent_level = parent.map(str::to_string);
        // Features chosen by hand belong to one profile level.
        self.selections = Default::default();
        self.select_dialog = None;
        self.apply_filters();
        self.session.invalidate_all();
        self.view_dirty = true;
    }

    /// True when nothing is being computed and the plot and tables are up
    /// to date: what the assistant waits for before answering.
    pub fn is_settled(&self) -> bool {
        !self.is_busy() && !self.view_dirty
    }

    /// Remember how things are, so that the change about to be made by
    /// the assistant can be undone.
    pub fn remember_for_undo(&mut self, _what: &str) {
        let snapshot = AssistantSnapshot {
            mode: self.mode,
            plot_kinds: self.plot_kinds,
            plot_settings: self.plot_settings.clone(),
            selected_feature: self.session.selected_feature.clone(),
            filters: self.filters.clone(),
            analysis: AnalysisSettings::of(&self.session),
            group_colors: self.session.chosen_group_colors(),
            other_samples_color: self.session.other_samples_color,
            switched_off: self.session.switched_off_samples(),
        };
        self.assistant_undo.push(snapshot);
        if self.assistant_undo.len() > MAX_ASSISTANT_UNDO {
            self.assistant_undo.remove(0);
        }
    }

    /// Whether there is a change by the assistant to undo.
    pub fn can_undo_assistant_change(&self) -> bool {
        !self.assistant_undo.is_empty()
    }

    /// Undo the last change the assistant made. False if there was none.
    pub fn undo_assistant_change(&mut self) -> bool {
        let Some(snapshot) = self.assistant_undo.pop() else {
            return false;
        };
        self.mode = snapshot.mode;
        self.plot_kinds = snapshot.plot_kinds;
        self.plot_settings = snapshot.plot_settings;
        self.session.selected_feature = snapshot.selected_feature;
        if snapshot.analysis != AnalysisSettings::of(&self.session) {
            if snapshot.analysis.profile_level != self.session.profile_level
                || snapshot.analysis.parent_level != self.session.parent_level
            {
                self.selections = Default::default();
                self.select_dialog = None;
            }
            snapshot.analysis.restore(&mut self.session);
            self.session.invalidate_all();
        }
        self.filters = snapshot.filters;
        self.session.set_chosen_group_colors(snapshot.group_colors);
        self.session
            .set_switched_off_samples(&snapshot.switched_off);
        self.session.other_samples_color = snapshot.other_samples_color;
        self.apply_filters();
        self.view_dirty = true;
        true
    }

    /// Hand the window a request from the assistant, as the server does.
    pub fn assistant_request(&mut self, request: assistant::Request) {
        self.assistant.queue.push_back((request, None));
    }

    /// Whether the Claude Code panel has the keyboard.
    pub fn assistant_terminal_has_focus(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.assistant.show_terminal
                && self
                    .assistant
                    .terminal
                    .as_ref()
                    .is_some_and(|t| t.has_focus())
        }
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// The text on the screen of the Claude Code panel, if it is running.
    pub fn assistant_terminal_text(&self) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.assistant.terminal.as_ref().map(|t| t.screen_text())
        }
        #[cfg(target_arch = "wasm32")]
        None
    }

    /// Stop sharing the session with Claude, as Assistant > Stop sharing
    /// does.
    pub fn assistant_stop_sharing(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.assistant.server = None;
        }
    }

    /// The assistant's tool calls so far, oldest first.
    pub fn assistant_log(&self) -> &[assistant::LogEntry] {
        &self.assistant.log
    }

    /// Answer the assistant's requests that can be answered now. Called
    /// each frame, and while the window is hidden.
    pub fn serve_assistant(&mut self, ctx: &egui::Context) {
        assistant::serve(self, ctx);
    }

    /// The profile of the analysis being shown.
    fn current_profile(&self) -> Option<&stamp_core::profile::Profile> {
        let s = &self.session;
        match self.mode {
            Mode::TwoSamples => s.samples.output.as_ref().map(|(p, _)| &p.profile),
            Mode::TwoGroups => s.groups.output.as_ref().map(|(p, _)| &p.profile),
            Mode::MultipleGroups => s.multi.output.as_ref().map(|(p, _)| &p.profile),
            Mode::Correlation => s.correlation.output.as_ref().map(|(p, _)| &p.profile),
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
    pub fn results_table(&mut self) -> &Table {
        while self.table_pending.is_some() {
            self.continue_table(f64::INFINITY);
        }
        &self.table
    }

    /// While a large results table is built: rows built, and rows.
    pub fn results_table_progress(&self) -> Option<(usize, usize)> {
        self.table_pending
            .as_ref()
            .map(|pending| (pending.next, pending.total))
    }

    /// Build more of a large results table, for up to `milliseconds`, then
    /// download it if that was asked for.
    fn continue_table(&mut self, milliseconds: f64) {
        let Some(pending) = &self.table_pending else {
            return;
        };
        let deadline = crate::jobs::now_ms() + milliseconds;
        let (mut next, total) = (pending.next, pending.total);
        let shown = match &pending.shown {
            Some(shown) => shown.clone(),
            None => std::rc::Rc::new(self.shown_rows()),
        };
        let mut built: Vec<Table> = Vec::new();
        // Rows at a time: enough to take a moment, few enough to stop soon.
        const STEP: usize = 16;
        while next < total {
            let end = (next + STEP).min(total).min(shown.len());
            let part = self.results_table_part(&shown[next.min(end)..end]);
            next = (next + STEP).min(total);
            built.push(part);
            if crate::jobs::now_ms() >= deadline {
                break;
            }
        }
        let Some(pending) = &mut self.table_pending else {
            return;
        };
        let first = pending.next == 0;
        pending.next = next;
        pending.shown = Some(shown);
        for part in built {
            if first && self.table.headings.is_empty() {
                // A different analysis has different columns.
                if part.headings != pending.headings {
                    self.table_sort = SortState::default();
                }
                self.table.headings = part.headings;
            }
            self.table.rows.extend(part.rows);
        }
        if next >= total {
            let download = pending.download;
            self.table_pending = None;
            if download {
                self.save_table();
            }
        }
    }

    /// The rows of the results table of the current analysis, as
    /// `refresh_view` chooses them: indices into the rows of its results.
    fn shown_rows(&self) -> Vec<usize> {
        let s = &self.session;
        macro_rules! shown {
            ($output:expr, $active:expr) => {
                match &$output {
                    Some((_, results)) if self.results_only_active => {
                        let index: std::collections::HashMap<&str, usize> = results
                            .rows
                            .iter()
                            .enumerate()
                            .map(|(i, r)| (r.feature.as_str(), i))
                            .collect();
                        $active
                            .iter()
                            .filter_map(|r| index.get(r.feature.as_str()).copied())
                            .collect()
                    }
                    Some((_, results)) => (0..results.rows.len()).collect(),
                    None => Vec::new(),
                }
            };
        }
        match self.mode {
            Mode::TwoSamples => shown!(s.samples.output, s.sample_rows()),
            Mode::TwoGroups => shown!(s.groups.output, s.group_rows()),
            Mode::MultipleGroups => shown!(s.multi.output, s.multi_rows()),
            Mode::Correlation => shown!(s.correlation.output, s.correlation_rows()),
        }
    }

    /// Some rows of the results table of the current analysis, given as
    /// indices into the rows of its results.
    fn results_table_part(&self, shown: &[usize]) -> Table {
        let s = &self.session;
        macro_rules! part {
            ($output:expr, $table:path) => {
                match &$output {
                    Some((profile, results)) => $table(
                        profile,
                        results,
                        shown.iter().filter_map(|&i| results.rows.get(i)),
                    ),
                    None => Table {
                        headings: Vec::new(),
                        rows: Vec::new(),
                    },
                }
            };
        }
        match self.mode {
            Mode::TwoSamples => part!(s.samples.output, sample_table),
            Mode::TwoGroups => part!(s.groups.output, group_table),
            Mode::MultipleGroups => part!(s.multi.output, multi_group_table),
            Mode::Correlation => part!(s.correlation.output, correlation_table),
        }
    }

    /// The features listed beside single-feature plots, honouring the "show
    /// only active features" choice.
    pub fn feature_table(&self) -> Vec<&FeatureRow> {
        let rows: Vec<&FeatureRow> = self
            .feature_rows
            .iter()
            .filter(|row| row.active || !self.show_only_active_features)
            .collect();
        self.feature_sort
            .order(rows.len(), |row, column| rows[row].cells()[column.min(4)])
            .into_iter()
            .map(|index| rows[index])
            .collect()
    }

    /// Sort the feature table by a column, as clicking its heading does;
    /// a second time reverses the order.
    pub fn sort_feature_table(&mut self, column: usize) {
        self.feature_sort.click(column);
    }

    /// The order in which the rows of the results table are shown.
    pub fn results_table_order(&self) -> Vec<usize> {
        let rows = &self.table.rows;
        self.table_sort.order(rows.len(), |row, column| {
            rows[row].get(column).map_or("", String::as_str)
        })
    }

    /// Sort the results table by a column, as clicking its heading does.
    pub fn sort_results_table(&mut self, column: usize) {
        self.table_sort.click(column);
    }

    pub fn set_show_only_active_features(&mut self, only_active: bool) {
        self.show_only_active_features = only_active;
    }

    /// Add a feature to the features listed for the box plot, or remove it
    /// if listed, as Ctrl+click in the feature table does; the box plot
    /// then shows the listed features.
    pub fn toggle_box_feature(&mut self, feature: &str) {
        let mut listed = plots::typed_features(&self.plot_settings.box_feature_list);
        match listed.iter().position(|f| f == feature) {
            Some(index) => {
                listed.remove(index);
            }
            None => listed.push(feature.to_string()),
        }
        self.plot_settings.box_feature_list = listed.join("\n");
        self.plot_settings.box_features = plots::BoxFeatures::Listed;
        self.view_dirty = true;
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

    /// Show a message for the user, until dismissed.
    pub fn show_notice(&mut self, text: impl Into<String>) {
        self.notice = Some(text.into());
    }

    /// Load a profile from the text of a file.
    pub fn load_profile(&mut self, name: &str, text: &str) {
        let tree = stamp_core::ProfileTree::read(text).map_err(|e| e.to_string());
        self.use_profile(name, tree);
    }

    /// Use a profile read from a file, or say why it could not be read.
    fn use_profile(&mut self, name: &str, tree: Result<stamp_core::ProfileTree, String>) {
        match tree.map(|tree| self.session.load_tree(name, tree)) {
            Ok(()) => {
                // Changes to another dataset cannot be undone in this one.
                self.assistant_undo.clear();
                self.filters = starting_filters();
                self.selections = Default::default();
                self.select_dialog = None;
                self.highlight_level = None;
                self.highlight_value.clear();
                self.metadata_filter = MetadataFilterUi::default();
                self.metadata_sort = SortState::default();
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
        self.load_example_data(Example::Enterotypes);
    }

    /// Load one of the datasets that ship inside the application.
    pub fn load_example_data(&mut self, example: Example) {
        match example {
            Example::Enterotypes => {
                self.load_profile("Enterotypes.profile.spf", ENTEROTYPES_PROFILE);
                self.load_metadata("Enterotypes.metadata.tsv", ENTEROTYPES_METADATA);
            }
            Example::CoalSeamGas => match unpack(COAL_PROFILE_GZ) {
                Ok(spf) => {
                    self.load_profile("coal.spf", &spf);
                    self.load_metadata("coal.metadata.tsv", COAL_METADATA);
                }
                Err(message) => self.notice = Some(message),
            },
            Example::EggerthellaceaeGenera | Example::EggerthellaceaeSpecies => {
                let genera = example == Example::EggerthellaceaeGenera;
                let (name, profile) = if genera {
                    ("eggerthellaceae.spf", EGGERTHELLACEAE_PROFILE_GZ)
                } else {
                    (
                        "eggerthellaceae_mgr_species.spf",
                        EGGERTHELLACEAE_SPECIES_GZ,
                    )
                };
                let texts = unpack(profile).and_then(|spf| {
                    unpack(EGGERTHELLACEAE_METADATA_GZ).map(|metadata| (spf, metadata))
                });
                match texts {
                    Ok((spf, metadata)) => {
                        self.load_profile(name, &spf);
                        self.load_metadata("eggerthellaceae.metadata.tsv", &metadata);
                        if self.notice.is_none() {
                            self.eggerthellaceae_settings(genera);
                        }
                    }
                    Err(message) => self.notice = Some(message),
                }
            }
        }
    }

    /// Settings the Eggerthellaceae examples start with: the lowest level,
    /// the samples grouped by host and compared across all hosts, and
    /// Equivita's relative abundance for correlations. The species profile
    /// keeps the rest of the community only for calculating proportions.
    fn eggerthellaceae_settings(&mut self, genera: bool) {
        let session = &mut self.session;
        session.profile_level = if genera { "Genus" } else { "Species" }.to_string();
        if !genera {
            session.unclassified = Unclassified::FrequencyOnly;
        }
        session.set_group_field("Host_group");
        // Two groups: domestic horses against the feral horses of Sable
        // Island, the two largest groups.
        session.groups.group1 = "Domestic horse".to_string();
        session.groups.group2 = Some("Feral horse (Sable Island)".to_string());
        session.correlation.field = Some("Equivita_relab".to_string());
        session.invalidate_all();
        self.apply_filters();
        self.mode = Mode::MultipleGroups;
        self.view_dirty = true;
    }

    /// Hand the application the outcome of a file request, as the file
    /// dialogs do.
    pub fn deliver(&self, event: Event) {
        self.platform.push(event);
    }

    /// Save the results table as it is shown, through the platform's save
    /// dialog or as a browser download.
    pub fn download_table(&mut self, ctx: &egui::Context) {
        self.table_context = Some(ctx.clone());
        match &mut self.table_pending {
            // Saved once built; the results table shows how far it has got.
            Some(pending) => {
                pending.download = true;
                self.show_table = true;
            }
            None => self.save_table(),
        }
    }

    fn save_table(&self) {
        let Some(ctx) = &self.table_context else {
            return;
        };
        let bytes = self.table.to_tsv().into_bytes();
        self.platform.request(
            Request::Save {
                name: "results.tsv".to_string(),
                bytes,
            },
            ctx,
        );
    }

    /// Save the plot as a PNG image of the given resolution, through the
    /// platform's save dialog or as a browser download.
    pub fn save_png(&mut self, ctx: &egui::Context, dpi: f64) {
        match platform::encode_plot(&self.figure, "plot.png", dpi) {
            Ok(bytes) => self.platform.request(
                Request::Save {
                    name: "plot.png".to_string(),
                    bytes,
                },
                ctx,
            ),
            Err(message) => self.notice = Some(message),
        }
    }

    /// Act on files that have been opened or converted since the last frame.
    fn handle_file_events(&mut self) {
        for event in self.platform.poll() {
            match event {
                Event::Profile(files) => self.start_loading(files),
                Event::Auxiliary(which, file) => {
                    if let Some(PendingProfile::Mothur { groups, names, .. }) =
                        &mut self.pending_profile
                    {
                        match which {
                            Auxiliary::MothurGroups => *groups = Some(file),
                            Auxiliary::MothurNames => *names = Some(file),
                        }
                    }
                }
                Event::Metadata(file) => self.load_metadata(&file.name, &file.text()),
                Event::Converted { name, spf } => self.start_loading_text(name, spf),
                Event::Failed(message) => self.notice = Some(message),
            }
        }
    }

    /// Copy the filter controls into the session's filters.
    fn apply_filters(&mut self) {
        let effect = |enabled: bool, minimum: f64| enabled.then_some(minimum);

        let selected = |mode: Mode| {
            let selection = &self.selections[mode_slot(mode)];
            selection.chosen.clone().filter(|_| selection.enabled)
        };
        self.session.samples.filter.selected_features = selected(Mode::TwoSamples);
        self.session.groups.filter.selected_features = selected(Mode::TwoGroups);
        self.session.multi.filter.selected_features = selected(Mode::MultipleGroups);
        self.session.correlation.filter.selected_features = selected(Mode::Correlation);

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

        let f = &self.filters[mode_slot(Mode::Correlation)];
        let filter = &mut self.session.correlation.filter;
        filter.significance = f.p_enabled.then_some(f.p_value);
        filter.min_coefficient = effect(f.effect1_enabled, f.effect1_minimum);
        self.view_dirty = true;
    }

    /// The metadata fields and the groups of the current analysis, offered
    /// by the settings of some plots.
    fn plot_config_context(&self) -> plot_config::Context {
        let groups = match self.mode {
            Mode::TwoGroups => self
                .session
                .groups
                .output
                .as_ref()
                .map(|(profile, _)| vec![profile.group_name1.clone(), profile.group_name2.clone()])
                .unwrap_or_default(),
            Mode::MultipleGroups => self
                .session
                .multi
                .output
                .as_ref()
                .map(|(profile, _)| profile.group_names.clone())
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let metadata = self.session.metadata.as_ref();
        plot_config::Context {
            fields: metadata.map(|m| m.fields().to_vec()).unwrap_or_default(),
            numeric_fields: metadata
                .map(|m| {
                    m.fields()
                        .iter()
                        .filter(|f| stamp_plot::ordination_style::is_numeric_field(m, f))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default(),
            groups,
            lat_field: metadata.and_then(|m| stamp_plot::map_plot::coordinate_fields(m).0),
            lon_field: metadata.and_then(|m| stamp_plot::map_plot::coordinate_fields(m).1),
        }
    }

    /// Rebuild the plot and the tables from the session.
    fn refresh_view(&mut self) {
        self.plot_settings.highlighted = self.highlighted_features();
        // A plot already drawn from the same data and settings is reused:
        // some plots (ANCOM, the community tests, NMDS, rarefaction) take a
        // while to compute, and the view is refreshed for many reasons.
        let kind = self.plot_kind();
        let feature = if kind.uses_selected_feature(self.mode)
            || kind == PlotKind::CorrelationScatter
            || kind == PlotKind::CoOccurrence
            || kind == PlotKind::PrevalenceAbundance
            || self.plot_settings.colors_by_selected_feature(kind)
        {
            self.session.selected_feature.clone()
        } else {
            None
        };
        let key = format!(
            "{:?}|{:?}|{feature:?}|{}|{:?}",
            self.mode,
            kind,
            self.session.plot_fingerprint(),
            self.plot_settings
        );
        if let Some(index) = self.figure_cache.iter().position(|(k, _)| *k == key) {
            let (key, figure) = self.figure_cache.remove(index);
            self.figure = figure.clone();
            self.figure_cache.push((key, figure));
            self.plot_job = None;
            self.stale_figure = None;
            self.cancelled_plot = None;
        } else {
            let figure = plots::build(&self.session, self.mode, kind, &self.plot_settings);
            self.figures_built += 1;
            match self.session.ordinations.take_request() {
                // The plot needs something slow computed first: in a job,
                // showing the plot before greyed out with the progress.
                Some(request) => {
                    if self.cancelled_plot.as_deref() == Some(request.id().as_str()) {
                        self.plot_job = None;
                        self.stale_figure = None;
                        self.figure = Figure::message(PLOT_CANCELLED);
                    } else {
                        let running = self
                            .plot_job
                            .as_ref()
                            .is_some_and(|(job, _)| request.is_running_in(job));
                        if !running {
                            // A job computing something else is cancelled
                            // as it is dropped.
                            self.plot_job = Some((request.start(), None));
                            self.cancelled_plot = None;
                        }
                        let shown = std::mem::replace(&mut self.figure, figure);
                        if !is_placeholder(&shown) {
                            self.stale_figure = Some(shown);
                        }
                    }
                }
                None => {
                    self.plot_job = None;
                    self.stale_figure = None;
                    self.cancelled_plot = None;
                    self.figure = figure;
                    // Very large figures (big heatmaps) are not kept, to
                    // bound the memory used.
                    if self.figure.shapes.len() <= FIGURE_CACHE_SHAPES {
                        if self.figure_cache.len() >= FIGURE_CACHE_SIZE {
                            self.figure_cache.remove(0);
                        }
                        self.figure_cache.push((key, self.figure.clone()));
                    }
                }
            }
        }
        let s = &self.session;
        let min_exponent = self.plot_settings.min_p_exponent;
        let feature_row =
            |feature: &str, effect: f64, p: f64, corrected: f64, note: &str| FeatureRow {
                feature: feature.to_string(),
                effect_size: format!("{effect:.3}"),
                p_value: format_p_value(p, min_exponent),
                p_value_corrected: format_p_value(corrected, min_exponent),
                note: note.to_string(),
                active: false,
            };

        // The results table holds every value of every sample, which takes
        // a while for large profiles: it is built again only when the
        // results, their filters or the choice of rows change, not when
        // only the plot does.
        let table_key = format!(
            "{:?}|{}|{}",
            self.mode,
            s.plot_fingerprint(),
            self.results_only_active
        );
        let rebuild_table = table_key != self.table_key;
        // A stand-in for the table when it is kept as it is.
        let unchanged_table = || Table {
            headings: Vec::new(),
            rows: Vec::new(),
        };

        // The features that pass the filters.
        let mut active: Vec<String> = Vec::new();
        let mut feature_rows = Vec::new();
        let mut effect_heading = "Effect size";
        // A table too large to build at once is built a few rows at a time
        // while the results table is open (see `continue_table`).
        let mut deferred = None;
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
                    active = rows.iter().map(|r| r.feature.clone()).collect();
                    let shown = if self.results_only_active {
                        rows
                    } else {
                        results.rows.iter().collect()
                    };
                    let features = shown.iter().map(|r| r.feature.clone()).collect();
                    let table = if rebuild_table {
                        sample_table(profile, results, shown)
                    } else {
                        unchanged_table()
                    };
                    (table, features)
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
                    active = rows.iter().map(|r| r.feature.clone()).collect();
                    let shown = if self.results_only_active {
                        rows
                    } else {
                        results.rows.iter().collect()
                    };
                    let features = shown.iter().map(|r| r.feature.clone()).collect();
                    let table = if !rebuild_table {
                        unchanged_table()
                    } else if is_large(shown.len(), profile.profile.sample_names.len()) {
                        deferred = Some(shown.len());
                        unchanged_table()
                    } else {
                        group_table(profile, results, shown)
                    };
                    (table, features)
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
                    active = rows.iter().map(|r| r.feature.clone()).collect();
                    let shown = if self.results_only_active {
                        rows
                    } else {
                        results.rows.iter().collect()
                    };
                    let features = shown.iter().map(|r| r.feature.clone()).collect();
                    let table = if !rebuild_table {
                        unchanged_table()
                    } else if is_large(shown.len(), profile.profile.sample_names.len()) {
                        deferred = Some(shown.len());
                        unchanged_table()
                    } else {
                        multi_group_table(profile, results, shown)
                    };
                    (table, features)
                }
                None => empty_table(),
            },
            Mode::Correlation => match &s.correlation.output {
                Some((profile, results)) => {
                    effect_heading = "Correlation";
                    feature_rows = results
                        .rows
                        .iter()
                        .map(|r| {
                            feature_row(
                                &r.feature,
                                r.coefficient,
                                r.p_value,
                                r.p_value_corrected,
                                &r.note,
                            )
                        })
                        .collect();
                    let rows = s.correlation_rows();
                    active = rows.iter().map(|r| r.feature.clone()).collect();
                    let shown = if self.results_only_active {
                        rows
                    } else {
                        results.rows.iter().collect()
                    };
                    let features = shown.iter().map(|r| r.feature.clone()).collect();
                    let table = if !rebuild_table {
                        unchanged_table()
                    } else if is_large(shown.len(), profile.profile.sample_names.len()) {
                        deferred = Some(shown.len());
                        unchanged_table()
                    } else {
                        correlation_table(profile, results, shown)
                    };
                    (table, features)
                }
                None => empty_table(),
            },
        };
        let active: std::collections::HashSet<String> = active.into_iter().collect();
        for row in feature_rows.iter_mut() {
            row.active = active.contains(&row.feature);
        }
        // The feature table starts in alphabetical order, as in STAMP v2.
        feature_rows.sort_by(|a, b| a.feature.cmp(&b.feature));

        if rebuild_table {
            self.table_pending = deferred.map(|total| TablePending {
                next: 0,
                total,
                headings: std::mem::take(&mut self.table.headings),
                download: false,
                shown: None,
            });
            if deferred.is_none() {
                // A different analysis has different columns.
                if table.headings != self.table.headings {
                    self.table_sort = SortState::default();
                }
                self.table = table;
            } else {
                self.table.rows.clear();
            }
            self.table_features = features;
            self.table_key = table_key;
        }
        self.feature_rows = feature_rows;
        self.effect_heading = effect_heading;
        self.view_dirty = false;
    }

    /// Draw the whole interface for one frame.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.handle_file_events();
        self.handle_shortcuts(ui.ctx());
        self.serve_assistant(&ui.ctx().clone());
        if self.session.update(self.mode) {
            // On the desktop the work is on another thread and is looked in
            // on now and then. In the browser each frame does a slice of
            // it, so the next frame should follow at once.
            let wait = if cfg!(target_arch = "wasm32") { 0 } else { 50 };
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(wait));
        }
        if self.session.generation != self.seen_generation {
            self.seen_generation = self.session.generation;
            self.view_dirty = true;
        }
        self.poll_plot_job(ui.ctx());
        self.poll_loading(ui.ctx());
        let download = self.table_pending.as_ref().is_some_and(|p| p.download);
        if self.table_pending.is_some() && (self.show_table || download) {
            // A slice of each frame, so that the window stays responsive.
            self.continue_table(30.0);
            ui.ctx().request_repaint();
        }

        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        if self.view_dirty {
            self.refresh_view();
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.frames(ui));
        self.windows(ui.ctx());
    }

    /// Whether a frame is shown, as the View menu (and the buttons that
    /// open frames) set it.
    pub fn frame_shown(&self, frame: Frame) -> bool {
        match frame {
            Frame::Plot => true,
            Frame::Profile => self.show_profile_panel,
            Frame::Properties => self.show_properties_panel,
            Frame::Features => self.show_feature_table && self.session.tree.is_some(),
            Frame::Metadata => self.show_metadata_table,
            Frame::Results => self.show_table,
            Frame::ConfigurePlot => self.show_plot_config,
            #[cfg(not(target_arch = "wasm32"))]
            Frame::ClaudeCode => self.assistant.show_terminal,
            #[cfg(target_arch = "wasm32")]
            Frame::ClaudeCode => false,
        }
    }

    /// Show or hide a frame, as the View menu and a frame's tab do.
    pub fn show_frame(&mut self, frame: Frame, shown: bool) {
        match frame {
            Frame::Plot => {}
            Frame::Profile => self.show_profile_panel = shown,
            Frame::Properties => self.show_properties_panel = shown,
            Frame::Features => self.show_feature_table = shown,
            Frame::Metadata => self.show_metadata_table = shown,
            Frame::Results => self.show_table = shown,
            Frame::ConfigurePlot => self.show_plot_config = shown,
            #[cfg(not(target_arch = "wasm32"))]
            Frame::ClaudeCode => self.assistant.show_terminal = shown,
            #[cfg(target_arch = "wasm32")]
            Frame::ClaudeCode => {}
        }
    }

    /// The arrangement of the frames.
    pub fn layout(&self) -> &egui_tiles::Tree<Frame> {
        &self.layout
    }

    /// Change the arrangement of the frames, as dragging their tabs does.
    pub fn layout_mut(&mut self) -> &mut egui_tiles::Tree<Frame> {
        &mut self.layout
    }

    /// Put the frames back where STAMP starts with them, as View > Reset
    /// layout does. Which frames are shown does not change.
    pub fn reset_layout(&mut self) {
        self.layout = layout::default_tree();
    }

    /// The frames around the plot, where the user has put them.
    fn frames(&mut self, ui: &mut egui::Ui) {
        // The tree is taken out while it is drawn, so that the frames can
        // change the window.
        let mut tree = std::mem::replace(
            &mut self.layout,
            egui_tiles::Tree::empty(egui::Id::unique("stamp-layout")),
        );
        if !layout::is_complete(&tree) {
            tree = layout::default_tree();
        }
        layout::sync_visibility(&mut tree, |frame| self.frame_shown(frame));
        let plot_tabs = layout::plot_tabs(&tree);
        let mut draw = |frame: Frame, ui: &mut egui::Ui| self.frame_ui(frame, ui);
        let mut behavior = layout::Behavior {
            draw: &mut draw,
            closed: Vec::new(),
            plot_tabs,
        };
        tree.ui(&mut behavior, ui);
        let closed = std::mem::take(&mut behavior.closed);
        self.layout = tree;
        for frame in closed {
            self.show_frame(frame, false);
        }
    }

    /// Draw one frame's contents.
    fn frame_ui(&mut self, frame: Frame, ui: &mut egui::Ui) {
        let padded = egui::Frame::new().inner_margin(egui::Margin::symmetric(6, 4));
        match frame {
            Frame::Plot => self.plot_panel(ui),
            Frame::Profile => {
                padded.show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.profile_panel(ui));
                });
            }
            Frame::Properties => {
                padded.show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.properties_panel(ui));
                });
            }
            Frame::Features => {
                padded.show(ui, |ui| self.feature_panel(ui));
            }
            Frame::Metadata => {
                padded.show(ui, |ui| self.metadata_panel(ui));
            }
            Frame::Results => {
                padded.show(ui, |ui| self.results_table_ui(ui));
            }
            Frame::ConfigurePlot => {
                padded.show(ui, |ui| self.plot_config_ui(ui));
            }
            Frame::ClaudeCode => {
                #[cfg(not(target_arch = "wasm32"))]
                assistant::window::terminal_frame(self, ui);
            }
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        // A menu item with an icon and, where it has one, its shortcut.
        let item = |icon: &str, text: &str, shortcut: Option<&egui::KeyboardShortcut>| {
            let button = egui::Button::new(icons::label(icon, text));
            match shortcut {
                Some(shortcut) => button.shortcut_text(ctx.format_shortcut(shortcut)),
                None => button,
            }
        };
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                let loaded = self.session.tree.is_some();
                let load = item(
                    icons::FOLDER_OPEN,
                    "Load profile...",
                    Some(&shortcut::LOAD_PROFILE),
                );
                if ui.add(load).clicked() {
                    self.platform.request(Request::OpenProfile, &ctx);
                }
                let metadata = item(icons::TAG, "Load metadata...", None);
                if ui.add_enabled(loaded, metadata).clicked() {
                    self.platform.request(Request::OpenMetadata, &ctx);
                }
                ui.menu_button(icons::label(icons::FLASK, "Open example data"), |ui| {
                    for example in Example::ALL {
                        let entry = ui
                            .button(example.name())
                            .on_hover_text(example.description());
                        if entry.clicked() {
                            self.load_example_data(example);
                        }
                    }
                });
                ui.separator();
                ui.add_enabled_ui(loaded, |ui| {
                    ui.menu_button(icons::label(icons::FLOPPY_DISK, "Save plot as"), |ui| {
                        let png = item(icons::FILE_PNG, "PNG image", Some(&shortcut::SAVE_PLOT));
                        if ui.add(png).clicked() {
                            // The resolution is asked for first.
                            self.png_dpi = Some(platform::DEFAULT_DPI);
                        }
                        for (icon, label, name) in [
                            (icons::FILE_SVG, "SVG image", "plot.svg"),
                            (icons::FILE_PDF, "PDF document", "plot.pdf"),
                        ] {
                            if !ui.add(item(icon, label, None)).clicked() {
                                continue;
                            }
                            match platform::encode_plot(&self.figure, name, platform::DEFAULT_DPI) {
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
                    });
                });
                let save_table = item(icons::DOWNLOAD, "Save results table...", None);
                if ui
                    .add_enabled(
                        !self.table.rows.is_empty() || self.table_pending.is_some(),
                        save_table,
                    )
                    .clicked()
                {
                    self.download_table(&ctx);
                }
                // A web page is closed with its tab.
                if cfg!(not(target_arch = "wasm32")) {
                    ui.separator();
                    let exit = item(icons::SIGN_OUT, "Exit", Some(&shortcut::EXIT));
                    if ui.add(exit).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
            ui.menu_button("View", |ui| {
                // A panel that is shown has its item drawn as selected.
                let toggle =
                    |ui: &mut egui::Ui,
                     shown: &mut bool,
                     icon: &str,
                     text: &str,
                     shortcut: Option<&egui::KeyboardShortcut>| {
                        if ui
                            .add(item(icon, text, shortcut).selected(*shown))
                            .clicked()
                        {
                            *shown = !*shown;
                        }
                    };
                toggle(
                    ui,
                    &mut self.show_profile_panel,
                    icons::SIDEBAR,
                    "Profile and groups",
                    Some(&shortcut::GROUPS),
                );
                toggle(
                    ui,
                    &mut self.show_properties_panel,
                    icons::SLIDERS,
                    "Properties",
                    Some(&shortcut::PROPERTIES),
                );
                toggle(
                    ui,
                    &mut self.show_feature_table,
                    icons::LIST,
                    "Feature table",
                    None,
                );
                toggle(
                    ui,
                    &mut self.show_metadata_table,
                    icons::ROWS,
                    "Metadata table",
                    Some(&shortcut::METADATA),
                );
                #[cfg(not(target_arch = "wasm32"))]
                toggle(
                    ui,
                    &mut self.assistant.show_terminal,
                    icons::TERMINAL,
                    "Claude Code",
                    None,
                );
                ui.separator();
                toggle(
                    ui,
                    &mut self.show_table,
                    icons::TABLE,
                    "Results table",
                    Some(&shortcut::RESULTS),
                );
                if ui
                    .add(item(icons::SIDEBAR, "Reset layout", None))
                    .on_hover_text("Put the frames back where STAMP starts with them")
                    .clicked()
                {
                    self.reset_layout();
                }
                ui.separator();
                let window = item(
                    icons::WINDOW_OUT,
                    "Send plot to window",
                    Some(&shortcut::PLOT_WINDOW),
                );
                if ui
                    .add_enabled(self.session.tree.is_some(), window)
                    .clicked()
                {
                    self.send_plot_to_window();
                }
            });
            ui.menu_button("Settings", |ui| {
                let dark = ctx.theme() == egui::Theme::Dark;
                if ui
                    .add(item(icons::MOON, "Dark mode", None).selected(dark))
                    .clicked()
                {
                    set_dark_mode(&ctx, !dark);
                }
                ui.separator();
                if ui.add(item(icons::GEAR, "Preferences...", None)).clicked() {
                    self.show_preferences = true;
                }
            });
            ui.menu_button("Assistant", |ui| assistant::window::menu(self, ui));
            ui.menu_button("Help", |ui| {
                if ui.add(item(icons::INFO, "About STAMP", None)).clicked() {
                    self.show_about = true;
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            assistant::window::status(self, ui);
            if let Some(label) = self.loading().map(str::to_string) {
                ui.add(egui::Spinner::new());
                ui.label(format!("{label}..."));
                if ui.small_button("Cancel").clicked() {
                    self.cancel_loading();
                }
                ui.separator();
            }
            if self.session.tree.is_none() {
                ui.label("No profile loaded");
                return;
            }
            ui.label(self.status_summary());
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
            if let Some((stage, fraction)) = self.plot_progress() {
                ui.separator();
                let text = format!("Plot: {}", progress_text(&stage, self.plot_job_counts()));
                let bar = match fraction {
                    Some(fraction) => egui::ProgressBar::new(fraction),
                    None => egui::ProgressBar::new(0.0).animate(true),
                };
                ui.add(bar.desired_width(260.0).text(text));
                if ui.small_button("Cancel").clicked() {
                    self.cancel_plot();
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
            // Features chosen by hand belong to one profile level.
            self.selections = Default::default();
            self.select_dialog = None;
            self.apply_filters();
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
        self.sample_filter_section(ui);
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
                let members = groups.samples(name).unwrap_or_default().join("\n");
                let response = ui
                    .add_enabled(size > 0, checkbox)
                    .on_hover_text(members)
                    .on_disabled_hover_text("Every sample of this group is switched off");
                if response.changed() {
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

    /// The Sample filter: a metadata field and the values whose samples
    /// every analysis and plot uses.
    fn sample_filter_section(&mut self, ui: &mut egui::Ui) {
        let Some(metadata) = &self.session.metadata else {
            return;
        };
        let current = self.session.sample_filter();
        let mut options = vec![(None, "None".to_string())];
        options.extend(
            metadata
                .fields()
                .iter()
                .map(|f| (Some(f.clone()), f.clone())),
        );
        // Each value of the chosen field with its number of samples.
        let values: Vec<(String, usize)> = current
            .as_ref()
            .map(|(field, _)| {
                metadata
                    .unique_values(field)
                    .into_iter()
                    .map(|value| {
                        let count = metadata
                            .samples()
                            .iter()
                            .filter(|s| metadata.value(s, field) == Some(value))
                            .count();
                        (value.to_string(), count)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let (kept, total) = (metadata.num_selected(), metadata.samples().len());

        let mut field = current.as_ref().map(|(f, _)| f.clone());
        let mut change: Option<Option<(String, Vec<String>)>> = None;
        ui.horizontal(|ui| {
            ui.label("Sample filter:").on_hover_text(
                "Use only the samples with the values ticked below in every analysis and plot",
            );
            if choose(ui, "sample-filter-field", &mut field, &options) {
                // A new field starts with every value ticked.
                change = Some(field.clone().map(|f| {
                    let all = self.session.metadata.as_ref().map_or(Vec::new(), |m| {
                        m.unique_values(&f)
                            .into_iter()
                            .map(str::to_string)
                            .collect()
                    });
                    (f, all)
                }));
            }
        });
        if let Some((field, chosen)) = &current {
            ui.indent("sample-filter-values", |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("sample-filter-values")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        for (value, count) in &values {
                            let mut on = chosen.contains(value);
                            let label = if value.trim().is_empty() {
                                "(empty)"
                            } else {
                                value.as_str()
                            };
                            if ui.checkbox(&mut on, format!("{label} ({count})")).changed() {
                                let mut next: Vec<String> = values
                                    .iter()
                                    .map(|(v, _)| v)
                                    .filter(|v| if *v == value { on } else { chosen.contains(v) })
                                    .cloned()
                                    .collect();
                                next.dedup();
                                change = Some(Some((field.clone(), next)));
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.small_button("All").clicked() {
                        let all = values.iter().map(|(v, _)| v.clone()).collect();
                        change = Some(Some((field.clone(), all)));
                    }
                    if ui.small_button("None").clicked() {
                        change = Some(Some((field.clone(), Vec::new())));
                    }
                    ui.label(format!("{kept} of {total} samples kept"));
                });
            });
            ui.separator();
        }
        if let Some(filter) = change {
            self.set_sample_filter(filter.as_ref().map(|(f, v)| (f.as_str(), v.as_slice())));
        }
    }

    /// Keep only the samples whose value of a metadata field is one of
    /// `values`, as the Sample filter does; `None` keeps every sample.
    pub fn set_sample_filter(&mut self, filter: Option<(&str, &[String])>) {
        if let Err(message) = self.session.set_sample_filter(filter) {
            self.notice = Some(message);
        }
        self.view_dirty = true;
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
        let mut show_info = false;

        match mode {
            Mode::TwoSamples => {
                // Samples switched off in the metadata table are not offered.
                let metadata = self.session.metadata.as_ref();
                let mut names: Vec<String> = self
                    .session
                    .tree
                    .as_ref()
                    .map(|t| t.sample_names().to_vec())
                    .unwrap_or_default();
                // In alphabetical order, as in STAMP v2.
                names.sort();
                let names: Vec<String> = names
                    .into_iter()
                    .filter(|name| {
                        metadata.is_none_or(|m| !m.samples().contains(name) || m.is_active(name))
                    })
                    .collect();
                if names.len() >= 2 {
                    let state = &mut self.session.samples;
                    if !names.contains(&state.sample1) {
                        let other = state.sample2.clone();
                        state.sample1 = names
                            .iter()
                            .find(|n| **n != other)
                            .cloned()
                            .unwrap_or_default();
                        rerun = true;
                    }
                    if !names.contains(&state.sample2) {
                        let other = state.sample1.clone();
                        state.sample2 = names
                            .iter()
                            .find(|n| **n != other)
                            .cloned()
                            .unwrap_or_default();
                        rerun = true;
                    }
                }
                let samples = strings(&names);
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
                        ui.label("");
                        show_info |= ui
                            .button("Correction info")
                            .on_hover_text("Summary of the multiple test correction")
                            .clicked();
                        ui.end_row();
                    });
            }
            Mode::TwoGroups => {
                let Some(groups) = self.session.grouping() else {
                    ui.label("Load a metadata file to compare groups.");
                    return;
                };
                // Groups unticked in the group list are not offered.
                let active = &self.session.multi.active_groups;
                let names: Vec<String> = groups
                    .names()
                    .filter(|name| active.iter().any(|a| a == name))
                    .map(str::to_string)
                    .collect();
                if names.is_empty() {
                    ui.label("Tick at least one group in the group list.");
                    return;
                }
                {
                    let state = &mut self.session.groups;
                    if !names.contains(&state.group1) {
                        let other = state.group2.clone();
                        state.group1 = names
                            .iter()
                            .find(|n| Some(*n) != other.as_ref())
                            .unwrap_or(&names[0])
                            .clone();
                        rerun = true;
                    }
                    if state
                        .group2
                        .as_ref()
                        .is_some_and(|second| !names.contains(second) || *second == state.group1)
                    {
                        // "All other samples" if no other group is left.
                        state.group2 = names.iter().find(|n| **n != state.group1).cloned();
                        rerun = true;
                    }
                }
                let mut seconds: Vec<(Option<String>, String)> =
                    names.iter().map(|n| (Some(n.clone()), n.clone())).collect();
                seconds.push((None, stamp_core::ALL_OTHER_SAMPLES.to_string()));
                // Any field can say which samples belong together.
                let mut pair_fields: Vec<(Option<String>, String)> =
                    vec![(None, "(choose a field)".to_string())];
                if let Some(metadata) = &self.session.metadata {
                    pair_fields.extend(
                        metadata
                            .fields()
                            .iter()
                            .filter(|f| Some(*f) != self.session.group_field.as_ref())
                            .map(|f| (Some(f.clone()), f.clone())),
                    );
                }
                let transforms = named(&Transform::ALL, Transform::name);
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
                        if state.analysis.test.is_paired() {
                            rerun |= row(ui, "Pair field:", |ui| {
                                choose(ui, "group-pair-field", &mut state.pair_field, &pair_fields)
                            });
                            if let Some(pairing) = &state.pairing {
                                ui.label("");
                                ui.weak(pairing);
                                ui.end_row();
                            }
                        }
                        rerun |= row(ui, "Type:", |ui| {
                            choose(ui, "group-type", &mut state.analysis.test_type, &test_types)
                        });
                        // White's test works on counts; it is not transformed.
                        ui.label("Transform:").on_hover_text(TRANSFORM_HELP);
                        ui.add_enabled_ui(state.analysis.test != GroupTest::White, |ui| {
                            rerun |= choose(
                                ui,
                                "group-transform",
                                &mut state.analysis.transform,
                                &transforms,
                            );
                        });
                        ui.end_row();
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
                        ui.label("");
                        show_info |= ui
                            .button("Correction info")
                            .on_hover_text("Summary of the multiple test correction")
                            .clicked();
                        ui.end_row();
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
                        ui.label("Transform:").on_hover_text(TRANSFORM_HELP);
                        let transforms = named(&Transform::ALL, Transform::name);
                        rerun |= choose(
                            ui,
                            "multi-transform",
                            &mut state.analysis.transform,
                            &transforms,
                        );
                        ui.end_row();
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
                        ui.label("");
                        show_info |= ui
                            .button("Correction info")
                            .on_hover_text("Summary of the multiple test correction")
                            .clicked();
                        ui.end_row();
                    });
                self.view_dirty |= post_hoc_changed;
            }
            Mode::Correlation => {
                if self.session.metadata.is_none() {
                    ui.label("Load a metadata file to correlate features with a field.");
                    return;
                }
                let fields = self.session.numeric_fields();
                if fields.is_empty() {
                    ui.label("No metadata field holds a number for at least three samples.");
                    return;
                }
                let fields: Vec<(Option<String>, String)> =
                    fields.into_iter().map(|f| (Some(f.clone()), f)).collect();
                let state = &mut self.session.correlation;
                if !fields.iter().any(|(field, _)| *field == state.field) {
                    state.field = fields[0].0.clone();
                    rerun = true;
                }
                egui::Grid::new("correlation-stats")
                    .num_columns(2)
                    .show(ui, |ui| {
                        rerun |= row(ui, "Numeric field:", |ui| {
                            choose(ui, "correlation-field", &mut state.field, &fields)
                        });
                        rerun |= row(ui, "Correlation:", |ui| {
                            let methods = named(&CorrelationMethod::ALL, CorrelationMethod::name);
                            choose(
                                ui,
                                "correlation-method",
                                &mut state.analysis.method,
                                &methods,
                            )
                        });
                        let mut chosen = state.analysis.correction;
                        if row(ui, "Multiple test correction:", |ui| {
                            choose(ui, "correlation-correction", &mut chosen, &corrections)
                        }) {
                            correction = Some(chosen);
                        }
                        ui.label("");
                        show_info |= ui
                            .button("Correction info")
                            .on_hover_text("Summary of the multiple test correction")
                            .clicked();
                        ui.end_row();
                    });
            }
        }
        if rerun {
            self.session.invalidate(mode);
        }
        self.show_correction_info |= show_info;
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
        let q_values = match mode {
            Mode::TwoSamples => self.session.samples.analysis.correction,
            Mode::TwoGroups => self.session.groups.analysis.correction,
            Mode::MultipleGroups => self.session.multi.analysis.correction,
            Mode::Correlation => self.session.correlation.analysis.correction,
        }
        .is_false_discovery_rate();
        let selection = &mut self.selections[mode_slot(mode)];
        let mut open_dialog = false;
        let mut selection_changed = false;
        ui.horizontal(|ui| {
            selection_changed = ui
                .checkbox(&mut selection.enabled, "Select specific features")
                .changed();
            let button = egui::Button::new("Select features");
            open_dialog = ui.add_enabled(selection.enabled, button).clicked();
        });
        ui.separator();
        if open_dialog {
            self.open_select_features();
        }
        let filter = &mut self.filters[mode_slot(mode)];
        let changed = selection_changed
            | match mode {
                Mode::TwoSamples => {
                    let measures: Vec<&str> =
                        SampleEffectSize::ALL.iter().map(|m| m.name()).collect();
                    filter.show(
                        ui,
                        "samples",
                        &measures,
                        &SAMPLE_COUNT_KINDS,
                        "Sample 2 (<):",
                        counts,
                        q_values,
                    )
                }
                Mode::TwoGroups => {
                    let measures: Vec<&str> =
                        GroupEffectSize::ALL.iter().map(|m| m.name()).collect();
                    filter.show(
                        ui,
                        "groups",
                        &measures,
                        &GROUP_COUNT_KINDS,
                        "Group 2 (<):",
                        counts,
                        q_values,
                    )
                }
                Mode::MultipleGroups => {
                    filter.show(ui, "multi", &["Eta-squared"], &[], "", counts, q_values)
                }
                Mode::Correlation => filter.show(
                    ui,
                    "correlation",
                    &["Correlation (absolute)"],
                    &[],
                    "",
                    counts,
                    q_values,
                ),
            };
        if changed {
            self.apply_filters();
        }
    }

    /// Collect the slow part of the plot once computed, and draw the plot.
    fn poll_plot_job(&mut self, ctx: &egui::Context) {
        let Some((job, started)) = &mut self.plot_job else {
            return;
        };
        match job.poll(&self.session.ordinations) {
            plots::PlotPoll::Running => {
                if started.is_none() {
                    *started = Some(ctx.input(|i| i.time));
                }
                // As for the analyses: in the browser each frame does a
                // slice of the work.
                let wait = if cfg!(target_arch = "wasm32") { 0 } else { 50 };
                ctx.request_repaint_after(std::time::Duration::from_millis(wait));
            }
            plots::PlotPoll::Finished => {
                self.plot_job = None;
                self.view_dirty = true;
                ctx.request_repaint();
            }
            plots::PlotPoll::Cancelled => self.plot_job = None,
            plots::PlotPoll::Failed => {
                self.plot_job = None;
                self.stale_figure = None;
                self.figure = Figure::message("The calculation of the plot stopped unexpectedly");
            }
        }
    }

    /// Over the plot area while the plot is computed (after a moment, so
    /// that quick plots do not flicker): the plot before greyed out, and in
    /// the middle what is being done, how far it has got, and a button to
    /// cancel it; or, once cancelled, a button to compute it after all.
    fn plot_overlay(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let now = ui.input(|i| i.time);
        if let Some(Loading {
            label,
            started: Some(started),
            ..
        }) = &self.loading
        {
            if now - started >= PROGRESS_DELAY {
                let label = label.clone();
                let mut cancel = false;
                egui::Area::new(egui::Id::unique("loading-progress"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(rect.center())
                    .pivot(egui::Align2::CENTER_CENTER)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(egui::Spinner::new());
                                ui.label(format!("{label}..."));
                                cancel = ui.button("Cancel").clicked();
                            });
                        });
                    });
                if cancel {
                    self.cancel_loading();
                }
                return;
            }
        }
        let computing = match &self.plot_job {
            Some((_, Some(started))) => now - started >= PROGRESS_DELAY,
            _ => false,
        };
        let cancelled = self.plot_job.is_none() && self.cancelled_plot.is_some();
        if !computing && !cancelled {
            return;
        }
        if computing {
            let veil = ui.visuals().panel_fill.gamma_multiply(0.75);
            ui.painter().rect_filled(rect, 0.0, veil);
        }
        let mut cancel = false;
        let mut compute = false;
        egui::Area::new(egui::Id::unique("plot-progress"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical_centered(|ui| {
                        if let Some((stage, fraction)) = self.plot_progress() {
                            ui.horizontal(|ui| {
                                ui.add(egui::Spinner::new());
                                ui.label(progress_text(&stage, self.plot_job_counts()));
                            });
                            let bar = match fraction {
                                Some(fraction) => {
                                    egui::ProgressBar::new(fraction).show_percentage()
                                }
                                None => egui::ProgressBar::new(0.0).animate(true),
                            };
                            ui.add(bar.desired_width(280.0));
                            cancel = ui.button("Cancel").clicked();
                        } else {
                            ui.label(PLOT_CANCELLED);
                            compute = ui.button("Compute again").clicked();
                        }
                    });
                });
            });
        if cancel {
            self.cancel_plot();
        }
        if compute {
            self.compute_plot();
        }
    }

    /// `(done, total)` of what the plot's job is doing.
    fn plot_job_counts(&self) -> (usize, usize) {
        self.plot_job
            .as_ref()
            .map_or((0, 0), |(job, _)| job.progress().counts())
    }

    /// The middle of the window: the plot, the choice of plot beneath it,
    /// and the table of features beside it for plots of a single feature.
    fn plot_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("plot-bar").show(ui, |ui| self.plot_bar(ui));

        egui::CentralPanel::default().show(ui, |ui| {
            // The plot is centred in the room left for it.
            let space = ui.available_size();
            let rect = ui.max_rect();
            // While the plot is computed, the one shown before.
            let figure = match (&self.plot_job, &self.stale_figure) {
                (Some(_), Some(stale)) => stale,
                _ => &self.figure,
            };
            let zoom = self.zoom;
            let response = egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| figure_painter::show(ui, figure, zoom, space))
                .inner;
            self.hovered = response.hovered;
            self.plot_placement = response.placement;
            if let Some(item) = response.clicked {
                // Clicking a feature selects it for the single-feature plots.
                if self.feature_rows.iter().any(|row| row.feature == item) {
                    self.select_feature(&item);
                }
            }
            self.plot_overlay(ui, rect);
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
            if choose_plot(ui, self.mode, &mut kind) {
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
            metadata_sort: sort,
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
            if !metadata.selection().is_empty() {
                ui.label(format!(
                    "(Sample filter: {})",
                    metadata.selection().describe()
                ));
            }
        });
        ui.separator();

        let order = sort.order(samples.len(), |row, column| match column {
            0 => samples[row].as_str(),
            _ => metadata
                .value(&samples[row], &fields[(column - 1).min(fields.len() - 1)])
                .unwrap_or_default(),
        });
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
                    header.col(|ui| sort.heading(ui, 0, "Sample"));
                    for (column, field) in fields.iter().enumerate() {
                        header.col(|ui| sort.heading(ui, column + 1, field));
                    }
                })
                .body(|body| {
                    body.rows(20.0, samples.len(), |mut table_row| {
                        let sample = &samples[order[table_row.index()]];
                        table_row.col(|ui| {
                            let mut active = metadata.is_active(sample);
                            let checkbox = egui::Checkbox::new(&mut active, sample.as_str());
                            let response = ui
                                .add_enabled(metadata.is_selected(sample), checkbox)
                                .on_disabled_hover_text("Left out by the Sample filter");
                            if response.changed() {
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
        let mut sort = self.feature_sort;
        let rows = self.feature_table();
        if rows.is_empty() {
            ui.label("No features to show.");
            return;
        }
        let selected = self.session.selected_feature.clone();
        // With the box plot showing listed features, those are marked too,
        // and Ctrl+click adds or removes one.
        let is_box = self.plot_kind() == PlotKind::Box;
        let listing = is_box && self.plot_settings.box_features == plots::BoxFeatures::Listed;
        let listed = if listing {
            plots::typed_features(&self.plot_settings.box_feature_list)
        } else {
            Vec::new()
        };
        let mut toggle = None;
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
                    for (column, heading) in headings.into_iter().enumerate() {
                        header.col(|ui| sort.heading(ui, column, heading));
                    }
                })
                .body(|body| {
                    body.rows(18.0, rows.len(), |mut table_row| {
                        let feature = rows[table_row.index()];
                        table_row.set_selected(
                            selected.as_deref() == Some(feature.feature.as_str())
                                || listed.contains(&feature.feature),
                        );
                        for cell in feature.cells() {
                            table_row.col(|ui| {
                                plain_label(ui, cell);
                            });
                        }
                        let response = table_row
                            .response()
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if response.clicked() {
                            if is_box && response.ctx.input(|i| i.modifiers.command) {
                                toggle = Some(feature.feature.clone());
                            } else {
                                select = Some(feature.feature.clone());
                            }
                        }
                    });
                });
        });
        self.feature_sort = sort;
        if let Some(feature) = select {
            self.select_feature(&feature);
        }
        if let Some(feature) = toggle {
            self.toggle_box_feature(&feature);
        }
    }

    /// The window that asks what a BIOM table or mothur taxonomy file needs
    /// before it becomes a profile.
    fn pending_profile_window(&mut self, ctx: &egui::Context) {
        let (mut accept, mut cancel) = (false, false);
        let mut ask = None;
        match &mut self.pending_profile {
            None => return,
            Some(PendingProfile::Biom {
                file,
                fields,
                chosen,
            }) => {
                egui::Window::new("BIOM table")
                    .collapsible(false)
                    .resizable(false)
                    .default_pos([400.0, 180.0])
                    .show(ctx, |ui| {
                        ui.label(RichText::new(&file.name).strong());
                        ui.horizontal(|ui| {
                            ui.label("Hierarchy from:");
                            let mut options: Vec<(Option<String>, String)> =
                                vec![(None, "Observation ids only".to_string())];
                            options.extend(fields.iter().map(|f| (Some(f.clone()), f.clone())));
                            choose(ui, "biom-field", chosen, &options);
                        });
                        ui.horizontal(|ui| {
                            accept = ui.button("OK").clicked();
                            cancel = ui.button("Cancel").clicked();
                        });
                    });
            }
            Some(PendingProfile::Mothur {
                taxonomy,
                groups,
                names,
            }) => {
                egui::Window::new("mothur files")
                    .collapsible(false)
                    .resizable(false)
                    .default_pos([400.0, 180.0])
                    .show(ctx, |ui| {
                        let name = |file: &Option<OpenedFile>, missing: &str| match file {
                            Some(file) => file.name.clone(),
                            None => missing.to_string(),
                        };
                        egui::Grid::new("mothur-files")
                            .num_columns(3)
                            .show(ui, |ui| {
                                ui.label("Taxonomy file:");
                                ui.label(RichText::new(&taxonomy.name).strong());
                                ui.end_row();
                                ui.label("Groups file:");
                                ui.label(name(groups, "not chosen"));
                                if ui.button("Choose...").clicked() {
                                    ask = Some(Auxiliary::MothurGroups);
                                }
                                ui.end_row();
                                ui.label("Names file (optional):");
                                ui.label(name(names, "none"));
                                if ui.button("Choose...").clicked() {
                                    ask = Some(Auxiliary::MothurNames);
                                }
                                ui.end_row();
                            });
                        ui.horizontal(|ui| {
                            let create = egui::Button::new("Create profile");
                            accept = ui.add_enabled(groups.is_some(), create).clicked();
                            cancel = ui.button("Cancel").clicked();
                        });
                    });
            }
        }
        if let Some(which) = ask {
            self.platform.request(Request::OpenAuxiliary(which), ctx);
        }
        if cancel {
            self.pending_profile = None;
        } else if accept {
            self.accept_pending_profile();
        }
    }

    /// The Results table frame: the rows that pass the filters, with
    /// Download and "Show only active features".
    fn results_table_ui(&mut self, ui: &mut egui::Ui) {
        let mut select = None;
        let mut download = false;
        let mut sort = self.table_sort;
        let order = self.results_table_order();
        let mut only_active_changed = false;
        egui::Frame::NONE.show(ui, |ui| {
            ui.horizontal(|ui| {
                download = ui
                    .add_enabled(!self.table.rows.is_empty(), egui::Button::new("Download"))
                    .on_hover_text("Save this table as a tab-separated file")
                    .clicked();
                only_active_changed = ui
                    .checkbox(&mut self.results_only_active, "Show only active features")
                    .changed();
                ui.label(format!("{} features", self.table.rows.len()));
            });
            if let Some(pending) = &self.table_pending {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label(format!(
                        "Building the table: {} of {} rows",
                        pending.next, pending.total
                    ));
                });
                ui.add(
                    egui::ProgressBar::new(pending.next as f32 / pending.total.max(1) as f32)
                        .desired_width(300.0),
                );
            }
            if self.table.rows.is_empty() {
                if self.table_pending.is_none() {
                    ui.label("No features pass the filters.");
                }
                return;
            }
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
                    // Only cells in view are drawn: a table of a thousand
                    // samples has thousands of columns, and laying out
                    // the text of all of them in every row slows every
                    // frame.
                    .header(20.0, |mut header| {
                        for (column, heading) in self.table.headings.iter().enumerate() {
                            header.col(|ui| {
                                if ui.is_rect_visible(ui.max_rect()) {
                                    sort.heading(ui, column, heading);
                                }
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(18.0, self.table.rows.len(), |mut row| {
                            let index = order[row.index()];
                            row.set_selected(
                                selected == self.table_features.get(index).map(String::as_str),
                            );
                            for cell in &self.table.rows[index] {
                                row.col(|ui| {
                                    if ui.is_rect_visible(ui.max_rect()) {
                                        plain_label(ui, shorten_number(cell));
                                    }
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
        self.table_sort = sort;
        self.view_dirty |= only_active_changed;
        if download {
            let ctx = ui.ctx().clone();
            self.download_table(&ctx);
        }
    }

    /// The Configure plot frame: the options of the plot shown.
    fn plot_config_ui(&mut self, ui: &mut egui::Ui) {
        let (mode, kind) = (self.mode, self.plot_kind());
        let context = self.plot_config_context();
        let (mut changed, mut reset, mut close) = (false, false, false);
        ui.strong(kind.name());
        ui.separator();
        let room = (ui.available_height() - 40.0).max(80.0);
        egui::ScrollArea::vertical()
            .max_height(room)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                changed = plot_config::show(ui, mode, kind, &mut self.plot_settings, &context);
            });
        ui.separator();
        ui.horizontal(|ui| {
            reset = ui.button("Restore defaults").clicked();
            close = ui.button("Close").clicked();
        });
        if reset {
            self.plot_settings.reset(mode, kind);
        }
        self.view_dirty |= changed || reset;
        if close {
            self.show_plot_config = false;
        }
    }

    fn windows(&mut self, ctx: &egui::Context) {
        let mut outcome = Outcome::Open;
        if let Some((_, selection)) = &mut self.select_dialog {
            let mut open = true;
            egui::Window::new("Select features")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos([330.0, 110.0])
                .show(ctx, |ui| outcome = selection.show(ui));
            if !open {
                outcome = Outcome::Cancelled;
            }
        }
        if outcome != Outcome::Open {
            if let (Outcome::Accepted, Some((mode, selection))) =
                (outcome, self.select_dialog.take())
            {
                self.selections[mode_slot(mode)].chosen = Some(selection.selected().clone());
                self.apply_filters();
            }
            self.select_dialog = None;
        }

        self.pending_profile_window(ctx);

        let lines = self.correction_info();
        egui::Window::new("Multiple test correction")
            .open(&mut self.show_correction_info)
            .resizable(false)
            .default_pos([420.0, 160.0])
            .show(ctx, |ui| {
                for line in &lines {
                    ui.label(line);
                }
            });

        if let Some(dpi) = &mut self.png_dpi {
            let (mut save, mut cancel) = (false, false);
            egui::Window::new("Save plot as PNG")
                .collapsible(false)
                .resizable(false)
                .default_pos([420.0, 200.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Resolution of the image:");
                        ui.add(
                            egui::DragValue::new(dpi)
                                .speed(5.0)
                                .range(50.0..=1200.0)
                                .suffix(" dpi"),
                        );
                    });
                    ui.horizontal(|ui| {
                        save = ui.button("Save").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            let dpi = *dpi;
            if save {
                self.save_png(ctx, dpi);
            }
            if save || cancel {
                self.png_dpi = None;
            }
        }

        let mut changed = false;
        let (mut redraw, mut restore_defaults) = (false, false);
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
                ui.separator();
                egui::Grid::new("preferences-display")
                    .num_columns(2)
                    .show(ui, |ui| {
                        redraw |= row(ui, "Smallest p-value reported:", |ui| {
                            let exponent =
                                egui::DragValue::new(&mut self.plot_settings.min_p_exponent)
                                    .range(-300..=-3)
                                    .prefix("1e");
                            ui.add(exponent).changed()
                        });
                        let color = |ui: &mut egui::Ui, color: &mut stamp_plot::Color| {
                            let mut rgb = [color.r, color.g, color.b];
                            let changed = ui.color_edit_button_srgb(&mut rgb).changed();
                            if changed {
                                *color = stamp_plot::Color::rgb(rgb[0], rgb[1], rgb[2]);
                            }
                            changed
                        };
                        let [sample1, sample2] = &mut self.plot_settings.sample_colors;
                        redraw |= row(ui, "Colour of sample 1:", |ui| color(ui, sample1));
                        redraw |= row(ui, "Colour of sample 2:", |ui| color(ui, sample2));
                        redraw |= row(ui, "Colour of \"All other samples\":", |ui| {
                            color(ui, &mut self.session.other_samples_color)
                        });
                        redraw |= row(ui, "Colour of axes:", |ui| {
                            color(ui, &mut self.plot_settings.axes_color)
                        });
                        let font = |ui: &mut egui::Ui, size: &mut f64| {
                            let drag = egui::DragValue::new(size)
                                .speed(0.25)
                                .range(4.0..=36.0)
                                .suffix(" pt");
                            ui.add(drag).changed()
                        };
                        let fonts = &mut self.plot_settings.fonts;
                        redraw |= row(ui, "Plot font:", |ui| {
                            let families = named(
                                &stamp_plot::text::PlotFont::ALL,
                                stamp_plot::text::PlotFont::name,
                            );
                            choose(ui, "plot-font", &mut fonts.family, &families)
                        });
                        redraw |= row(ui, "Axis label font size:", |ui| {
                            font(ui, &mut fonts.axis_label)
                        });
                        redraw |= row(ui, "Tick font size:", |ui| font(ui, &mut fonts.tick));
                        redraw |= row(ui, "Legend font size:", |ui| font(ui, &mut fonts.legend));
                    });
                ui.label("Font sizes particular to a plot are set in its Configure plot window.");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    let mut ask = !self.assistant.skip_claude_code_dialog;
                    if ui
                        .checkbox(
                            &mut ask,
                            "Explain what is shared before opening Claude Code",
                        )
                        .changed()
                    {
                        self.assistant.skip_claude_code_dialog = !ask;
                    }
                }
                if ui.button("Restore defaults").clicked() {
                    restore_defaults = true;
                }
            });
        if restore_defaults {
            let defaults = Saved::default();
            self.session.seed = defaults.seed;
            self.session.stats.replicates = defaults.replicates;
            self.session.stats.pseudocount = defaults.pseudocount;
            self.session.other_samples_color = defaults.other_samples_color;
            self.plot_settings.min_p_exponent = defaults.plots.min_p_exponent;
            self.plot_settings.sample_colors = defaults.plots.sample_colors;
            self.plot_settings.axes_color = defaults.plots.axes_color;
            self.plot_settings.fonts = defaults.plots.fonts;
            changed = true;
        }
        self.view_dirty |= redraw || restore_defaults;
        if changed {
            self.session.invalidate_all();
        }

        // Plots in windows of their own: separate windows on the desktop,
        // floating ones in the browser.
        let mut closed = Vec::new();
        for window in &self.plot_windows {
            let size = [
                (window.figure.width as f32 + 24.0).clamp(240.0, 1400.0),
                (window.figure.height as f32 + 60.0).clamp(200.0, 1000.0),
            ];
            let builder = egui::ViewportBuilder::default()
                .with_title(format!("STAMP - {}", window.title))
                .with_inner_size(size);
            let id = egui::ViewportId::from_hash_of(("plot-window", window.id));
            ctx.show_viewport_immediate(id, builder, |ui, class| {
                let embedded = class == egui::ViewportClass::EmbeddedWindow;
                let mut contents = |ui: &mut egui::Ui| {
                    if ui.button("Close").clicked() {
                        closed.push(window.id);
                    }
                    let space = ui.available_size();
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            figure_painter::show(ui, &window.figure, 1.0, space);
                        });
                };
                if embedded {
                    contents(ui);
                } else {
                    egui::CentralPanel::default().show(ui, contents);
                }
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    closed.push(window.id);
                }
            });
        }
        self.plot_windows
            .retain(|window| !closed.contains(&window.id));

        assistant::window::windows(self, ctx);

        egui::Window::new("About STAMP")
            .open(&mut self.show_about)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("STAMP");
                ui.label(format!("Version {}", stamp_core::VERSION));
                ui.label("Statistical analysis of taxonomic and functional profiles.");
                ui.label("Donovan Parks and Rob Beiko");
                ui.label("Released under the GNU General Public License v3.");
                ui.label("Maps made with Natural Earth (public domain).");
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
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Also called while the window is hidden, so that the assistant's
        // questions are still answered.
        self.serve_assistant(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The frame just drawn decides what `save` stores.
        self.saved_theme_is_dark = ui.ctx().theme() == egui::Theme::Dark;
        self.draw(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut saved = self.saved(&egui::Context::default());
        saved.dark_mode = self.saved_theme_is_dark;
        eframe::set_value(storage, STORAGE_KEY, &saved);
    }
}

/// Whether a figure is the placeholder drawn while a plot is computed or
/// after its calculation was cancelled.
fn is_placeholder(figure: &Figure) -> bool {
    figure.regions.is_empty()
        && figure.shapes.len() == 1
        && matches!(&figure.shapes[0], stamp_plot::Shape::Text { text, .. }
            if text == plots::COMPUTING || text == PLOT_CANCELLED)
}

/// What a job is doing, with its count when it has one:
/// "PERMANOVA 412/999".
fn progress_text(stage: &str, (done, total): (usize, usize)) -> String {
    let stage = if stage.is_empty() { "Computing" } else { stage };
    if total > 0 {
        format!("{stage} {}/{total}", done.min(total))
    } else {
        format!("{stage}...")
    }
}

/// A STAMP profile from files that can be loaded at once (a STAMP profile,
/// a SingleM condensed profile, one or more Sandpiper profiles): its name,
/// its text, and a hint to add should loading it fail. `None` for a single
/// file that first needs choices (BIOM, mothur); `Err` holds the message
/// for the user.
type ProfileText = Result<(String, String, Option<&'static str>), String>;

fn profile_text(files: &[OpenedFile]) -> Option<ProfileText> {
    use stamp_core::import::{biom, sandpiper, singlem, NamedText};
    if let [file] = files {
        let text = file.text();
        let failed = |error: String| Err(format!("Could not load {}:\n{error}", file.name));
        let named = [NamedText {
            name: &file.name,
            text: &text,
        }];
        // Checked before BIOM, which takes any JSON object for a table.
        return if sandpiper::looks_like_sandpiper(&file.bytes) {
            Some(match sandpiper::to_spf(&named) {
                Ok(spf) => Ok((
                    file.name.clone(),
                    spf,
                    Some(
                        "A Sandpiper file holds one sample: choose the files of all samples \
                         together.",
                    ),
                )),
                Err(error) => failed(error.to_string()),
            })
        } else if singlem::looks_like_singlem(&text) {
            Some(match singlem::to_spf(&named) {
                Ok(spf) => Ok((file.name.clone(), spf, None)),
                Err(error) => failed(error.to_string()),
            })
        } else if biom::looks_like_biom(&file.bytes)
            || file.name.to_lowercase().ends_with(".taxonomy")
        {
            None
        } else {
            Some(Ok((file.name.clone(), text, None)))
        };
    }
    if !files
        .iter()
        .all(|f| sandpiper::looks_like_sandpiper(&f.bytes))
    {
        return Some(Err(
            "Several files can be loaded together only if they are Sandpiper profiles, \
             one per sample. Choose a single file for other formats."
                .to_string(),
        ));
    }
    let texts: Vec<String> = files.iter().map(OpenedFile::text).collect();
    let named: Vec<NamedText<'_>> = files
        .iter()
        .zip(&texts)
        .map(|(file, text)| NamedText {
            name: &file.name,
            text,
        })
        .collect();
    let name = format!("{} Sandpiper profiles", files.len());
    Some(match sandpiper::to_spf(&named) {
        Ok(spf) => Ok((name, spf, None)),
        Err(error) => Err(format!("Could not load {name}:\n{error}")),
    })
}

/// Add `hint` to the message shown after a profile failed to load.
fn add_hint(notice: &mut Option<String>, hint: Option<&str>) {
    if let (Some(notice), Some(hint)) = (notice, hint) {
        notice.push('\n');
        notice.push_str(hint);
    }
}

/// A profile being read from files in a job.
struct Loading {
    label: String,
    job: Job<Result<(String, stamp_core::ProfileTree), String>>,
    /// When the job started, in the time of the user interface, once known.
    started: Option<f64>,
}

impl Loading {
    /// Do `read` in a job, saying `label` meanwhile (first, in the browser,
    /// so that the page shows it before the work begins).
    fn start(
        label: String,
        read: impl FnOnce() -> Result<(String, stamp_core::ProfileTree), String> + Send + 'static,
    ) -> Loading {
        let mut read = Some(read);
        let mut announced = false;
        let job = Job::spawn_steps(move |_, _| {
            if !std::mem::replace(&mut announced, true) {
                return None;
            }
            read.take().map(|read| read())
        });
        Loading {
            label,
            job,
            started: None,
        }
    }
}

/// A large results table being built a few rows at a time.
struct TablePending {
    /// Rows built so far, and rows.
    next: usize,
    total: usize,
    /// Headings of the table before, to tell whether the columns changed.
    headings: Vec<String>,
    /// Save the table once built.
    download: bool,
    /// The rows of the results shown, once found.
    shown: Option<std::rc::Rc<Vec<usize>>>,
}

/// Whether a results table of `rows` rows with columns for `samples`
/// samples takes long enough to build to be built a few rows at a time.
fn is_large(rows: usize, samples: usize) -> bool {
    rows * (3 * samples + 20) > 400_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_sort_by_a_clicked_column() {
        let rows = [
            ["beta", "0.5", "x"],
            ["Alpha", "< 1e-15", "x"],
            ["gamma", "0.03", "x"],
            ["delta", "nan", "x"],
        ];
        let cell = |row: usize, column: usize| rows[row][column];
        let mut sort = SortState::default();
        assert_eq!(sort.order(4, cell), [0, 1, 2, 3]);

        // Text ignores case; a second click reverses the order.
        sort.click(0);
        assert_eq!(sort.order(4, cell), [1, 0, 3, 2]);
        sort.click(0);
        assert_eq!(sort.order(4, cell), [2, 3, 0, 1]);

        // Numbers sort by value, a bound by its bound, and nan last.
        sort.click(1);
        assert_eq!(sort.order(4, cell), [1, 2, 0, 3]);
        sort.click(1);
        assert_eq!(sort.order(4, cell), [3, 0, 2, 1]);

        // Equal cells keep their order.
        sort.click(2);
        assert_eq!(sort.order(4, cell), [0, 1, 2, 3]);
        assert_eq!(
            compare_cells("10", "9"),
            std::cmp::Ordering::Greater,
            "numbers are not compared as text"
        );
    }

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
        // The kinds are listed in STAMP v2's order.
        assert_eq!(GROUP_COUNT_KINDS[2], "independent, maximum");
        assert_eq!(
            filter.group(),
            Some(GroupCountFilter::IndependentMaximum(5.0, 7.0))
        );
        filter.kind = 3;
        assert_eq!(
            filter.group(),
            Some(GroupCountFilter::IndependentMinimum(5.0, 7.0))
        );
    }

    #[test]
    fn filters_start_as_in_stamp_v2() {
        let filters = starting_filters();
        for filter in &filters {
            assert!(filter.p_enabled && filter.p_value == 0.05);
            assert!(!filter.sequences.enabled && !filter.effect1_enabled);
            assert_eq!(
                (filter.sequences.first, filter.sequences.second),
                (5.0, 5.0)
            );
            assert_eq!(filter.parent_sequences.first, 1.0);
            assert_eq!(filter.effect2_minimum, 2.0);
            assert_eq!(filter.operator, FilterOperator::Or);
        }
        assert_eq!(
            filters[mode_slot(Mode::MultipleGroups)].effect1_minimum,
            0.8
        );
        assert_eq!(filters[mode_slot(Mode::TwoGroups)].effect1_minimum, 1.0);
        assert_eq!(filters[mode_slot(Mode::TwoSamples)].effect1_minimum, 1.0);
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
            Some(GroupCountFilter::IndependentMinimum(10.0, 20.0))
        );
        assert_eq!(
            app.session.samples.filter.parent_sequences,
            Some(SampleCountFilter::Independent(3.0, 4.0))
        );
        assert_eq!(app.session.samples.filter.sequences, None);
    }
}
