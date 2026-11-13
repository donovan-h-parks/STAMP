//! The "Configure plot" window: the settings of the plot being shown.
//!
//! Changes apply as they are made, so their effect can be seen at once.

use eframe::egui;
use stamp_core::stats::cluster::Linkage;
use stamp_plot::axes::LegendPosition;
use stamp_plot::extended_error_bar::{self, SortBy};
use stamp_plot::feature_plots::Field;
use stamp_plot::heatmap::{ColorMap, Order};
use stamp_plot::histogram::Inset;
use stamp_plot::sample_plots::BarField;
use stamp_plot::scatter::Spread;

use crate::app::choose;
use crate::plots::{PlotKind, PlotSettings};
use crate::session::Mode;

/// The rows of a settings grid. Each method adds one labelled row and
/// records whether its value changed.
struct Rows<'a> {
    ui: &'a mut egui::Ui,
    changed: bool,
    /// Tells apart controls with the same label in different groups.
    salt: &'a str,
    /// Controls added while this is set are switched off.
    disabled: bool,
}

impl Rows<'_> {
    fn number(
        &mut self,
        label: &str,
        value: &mut f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        self.text(label);
        let drag = egui::DragValue::new(value).speed(speed).range(range);
        self.changed |= self.put(drag);
        self.ui.end_row();
    }

    /// A length in inches.
    fn inches(&mut self, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>) {
        self.text(label);
        let drag = egui::DragValue::new(value)
            .speed(0.05)
            .range(range)
            .suffix(" in");
        self.changed |= self.put(drag);
        self.ui.end_row();
    }

    fn size(&mut self, width: &mut f64, height: &mut f64) {
        self.inches("Width:", width, 2.0..=40.0);
        self.inches("Height:", height, 2.0..=40.0);
    }

    fn count(&mut self, label: &str, value: &mut usize, range: std::ops::RangeInclusive<usize>) {
        self.text(label);
        self.changed |= self.put(egui::DragValue::new(value).range(range));
        self.ui.end_row();
    }

    /// A label, faded if the controls are switched off.
    fn text(&mut self, label: &str) {
        self.ui.add_enabled(!self.disabled, egui::Label::new(label));
    }

    /// A control, switched off if the controls are. Returns true if its
    /// value changed.
    fn put(&mut self, widget: impl egui::Widget) -> bool {
        self.ui.add_enabled(!self.disabled, widget).changed()
    }

    /// Run `add` with the controls it adds switched off unless `enabled`.
    /// An option that depends on another stays in view this way.
    fn when(&mut self, enabled: bool, add: impl FnOnce(&mut Rows)) {
        let before = self.disabled;
        self.disabled |= !enabled;
        add(self);
        self.disabled = before;
    }

    fn number_if(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        self.when(enabled, |rows| rows.number(label, value, speed, range));
    }

    fn inches_if(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        self.when(enabled, |rows| rows.inches(label, value, range));
    }

    fn count_if(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut usize,
        range: std::ops::RangeInclusive<usize>,
    ) {
        self.when(enabled, |rows| rows.count(label, value, range));
    }

    fn check_if(&mut self, enabled: bool, label: &str, value: &mut bool) {
        self.when(enabled, |rows| rows.check(label, value));
    }

    fn choice_if<T: PartialEq + Clone>(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut T,
        options: &[(T, &str)],
    ) {
        self.when(enabled, |rows| rows.choice(label, value, options));
    }

    fn check(&mut self, label: &str, value: &mut bool) {
        self.changed |= self.put(egui::Checkbox::new(value, label));
        self.ui.end_row();
    }

    fn choice<T: PartialEq + Clone>(&mut self, label: &str, value: &mut T, options: &[(T, &str)]) {
        self.text(label);
        let options: Vec<(T, String)> = options
            .iter()
            .map(|(option, name)| (option.clone(), name.to_string()))
            .collect();
        let id = format!("config-{}-{label}", self.salt);
        let mut changed = false;
        self.ui.add_enabled_ui(!self.disabled, |ui| {
            changed = choose(ui, &id, value, &options);
        });
        self.changed |= changed;
        self.ui.end_row();
    }

    /// A value that is either chosen automatically or set by hand.
    fn optional(
        &mut self,
        label: &str,
        value: &mut Option<f64>,
        default: f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        let mut set = value.is_some();
        if self.ui.checkbox(&mut set, label).changed() {
            *value = set.then_some(default);
            self.changed = true;
        }
        match value {
            Some(number) => {
                let drag = egui::DragValue::new(number).speed(speed).range(range);
                self.changed |= self.ui.add(drag).changed();
            }
            None => {
                self.ui.weak("automatic");
            }
        }
        self.ui.end_row();
    }

    /// Lower and upper limits of an axis, or automatic limits.
    fn limits(&mut self, label: &str, value: &mut Option<(f64, f64)>, default: (f64, f64)) {
        let mut set = value.is_some();
        if self.ui.checkbox(&mut set, label).changed() {
            *value = set.then_some(default);
            self.changed = true;
        }
        match value {
            Some((low, high)) => {
                let mut changed = false;
                self.ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(low).speed(0.1)).changed();
                    ui.label("to");
                    changed |= ui.add(egui::DragValue::new(high).speed(0.1)).changed();
                });
                if *high <= *low {
                    *high = *low + 1.0;
                }
                self.changed |= changed;
            }
            None => {
                self.ui.weak("automatic");
            }
        }
        self.ui.end_row();
    }

    /// Shortening of long feature names.
    fn truncate(&mut self, value: &mut Option<usize>) {
        let mut set = value.is_some();
        if self
            .ui
            .checkbox(&mut set, "Truncate feature names to:")
            .changed()
        {
            *value = set.then_some(50);
            self.changed = true;
        }
        match value {
            Some(length) => {
                let drag = egui::DragValue::new(length)
                    .range(5..=500)
                    .suffix(" characters");
                self.changed |= self.ui.add(drag).changed();
            }
            None => {
                self.ui.weak("not truncated");
            }
        }
        self.ui.end_row();
    }

    fn legend(&mut self, value: &mut LegendPosition) {
        let positions: Vec<(LegendPosition, &str)> = LegendPosition::ALL
            .iter()
            .map(|&position| (position, position.name()))
            .collect();
        self.choice("Position:", value, &positions);
    }

    fn field(&mut self, value: &mut Field) {
        self.choice(
            "Field to plot:",
            value,
            &[
                (Field::Proportion, "Proportion of sequences (%)"),
                (Field::SequenceCount, "Number of sequences"),
            ],
        );
    }

    fn order(&mut self, label: &str, value: &mut Order) {
        self.choice(
            label,
            value,
            &[
                (Order::Cluster(Linkage::Average), "Cluster: average (UPGMA)"),
                (
                    Order::Cluster(Linkage::Nearest),
                    "Cluster: nearest neighbour",
                ),
                (
                    Order::Cluster(Linkage::Furthest),
                    "Cluster: furthest neighbour",
                ),
                (Order::Cluster(Linkage::Centroid), "Cluster: centroid"),
                (Order::Cluster(Linkage::Ward), "Cluster: Ward"),
                (Order::Alphabetical, "Alphabetical"),
                (Order::MeanAbundance, "Mean abundance"),
                (Order::Original, "As in the profile"),
            ],
        );
    }
}

/// The settings window of one plot: rows of options, some gathered in
/// titled frames as STAMP v2 groups them.
struct Dialog<'a> {
    ui: &'a mut egui::Ui,
    changed: bool,
}

impl Dialog<'_> {
    fn grid(&mut self, ui_id: &str, framed: bool, add: impl FnOnce(&mut Rows)) {
        let changed = &mut self.changed;
        let contents = |ui: &mut egui::Ui| {
            if framed {
                ui.strong(ui_id);
            }
            egui::Grid::new(format!("plot-config-{ui_id}"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    let mut rows = Rows {
                        ui,
                        changed: false,
                        salt: ui_id,
                        disabled: false,
                    };
                    add(&mut rows);
                    *changed |= rows.changed;
                });
        };
        if framed {
            egui::Frame::group(self.ui.style()).show(self.ui, |ui| {
                ui.set_width(GROUP_WIDTH);
                contents(ui);
            });
        } else {
            contents(self.ui);
        }
    }

    /// Options that stand on their own.
    fn plain(&mut self, id: &str, add: impl FnOnce(&mut Rows)) {
        self.grid(id, false, add);
    }

    /// Related options in a frame with a title.
    fn group(&mut self, title: &str, add: impl FnOnce(&mut Rows)) {
        self.grid(title, true, add);
    }

    fn legend(&mut self, value: &mut LegendPosition) {
        self.group("Legend position", |rows| rows.legend(value));
    }

    fn truncate(&mut self, value: &mut Option<usize>) {
        self.plain("labels", |rows| rows.truncate(value));
    }

    /// The extended error bar plot and, with `post_hoc`, the post-hoc plot,
    /// which lays its pairs of groups out the same way.
    fn error_bar(
        &mut self,
        options: &mut extended_error_bar::Options,
        mode: Mode,
        sequence_counts: Option<&mut bool>,
        only_significant: Option<&mut bool>,
    ) {
        let groups = mode != Mode::TwoSamples;
        let post_hoc = only_significant.is_some();
        self.group("Sorting", |rows| {
            let labels = if post_hoc {
                "Group labels"
            } else {
                "Feature labels"
            };
            rows.choice(
                "Field:",
                &mut options.sort_by,
                &[
                    (SortBy::PValues, "p-values"),
                    (SortBy::EffectSizes, "Effect sizes"),
                    (SortBy::FeatureLabels, labels),
                ],
            );
        });
        if !post_hoc {
            self.group("Multiple comparisons", |rows| {
                rows.check(
                    "Show corrected p-values",
                    &mut options.show_corrected_p_values,
                );
            });
        }
        self.group("Figure size", |rows| {
            rows.inches("Width:", &mut options.width, 2.0..=40.0);
            rows.inches("Row height:", &mut options.row_height, 0.05..=2.0);
        });
        self.group("Error bar x-axis limits", |rows| {
            rows.limits(
                "Use custom limits:",
                &mut options.effect_limits,
                (-10.0, 10.0),
            );
        });
        self.group("Figure elements", |rows| {
            rows.check("Show bar subplot", &mut options.show_bars);
            if let Some(counts) = sequence_counts {
                rows.choice(
                    "Bar subplot shows:",
                    counts,
                    &[(false, "Proportion (%)"), (true, "Sequences")],
                );
            }
            if groups {
                rows.check("Show std. dev. on bar subplot", &mut options.show_std_dev);
                rows.number_if(
                    options.show_std_dev,
                    "End cap size:",
                    &mut options.end_cap_size,
                    0.1,
                    0.0..=20.0,
                );
            }
            rows.check("Show p-value labels", &mut options.show_p_values);
            rows.number("Marker size:", &mut options.marker_size, 1.0, 1.0..=500.0);
        });
        if let Some(only_significant) = only_significant {
            self.group("Filter", |rows| {
                rows.check("Filter rows above nominal level", only_significant);
            });
        } else {
            self.group("Legend position", |rows| {
                rows.choice(
                    "Position:",
                    &mut options.legend,
                    &[
                        (extended_error_bar::Legend::None, "None"),
                        (extended_error_bar::Legend::UpperLeft, "Upper left"),
                        (extended_error_bar::Legend::UpperCenter, "Upper centre"),
                        (extended_error_bar::Legend::UpperRight, "Upper right"),
                        (extended_error_bar::Legend::LowerLeft, "Lower left"),
                        (extended_error_bar::Legend::LowerCenter, "Lower centre"),
                        (extended_error_bar::Legend::LowerRight, "Lower right"),
                    ],
                );
            });
        }
        self.truncate(&mut options.truncate_labels);
    }
}

/// Width of the frames around groups of options, so that they line up.
const GROUP_WIDTH: f32 = 330.0;

/// The controls for the settings of one plot. Returns true if a setting
/// changed.
pub fn show(ui: &mut egui::Ui, mode: Mode, kind: PlotKind, settings: &mut PlotSettings) -> bool {
    let mut dialog = Dialog { ui, changed: false };
    match kind {
        PlotKind::ExtendedErrorBar => dialog.error_bar(
            &mut settings.error_bar,
            mode,
            (mode == Mode::TwoSamples).then_some(&mut settings.error_bar_sequence_counts),
            None,
        ),
        PlotKind::PostHoc => dialog.error_bar(
            &mut settings.post_hoc,
            mode,
            None,
            Some(&mut settings.post_hoc_only_significant),
        ),
        PlotKind::Bar if mode == Mode::TwoSamples => {
            let o = &mut settings.stat_bar;
            dialog.plain("general", |rows| {
                rows.choice(
                    "Field to plot:",
                    &mut o.field,
                    &[
                        (BarField::Proportions, "Proportion of sequences (%)"),
                        (BarField::Sequences, "Number of sequences"),
                        (BarField::ParentSequences, "Number of parent sequences"),
                        (BarField::PValues, "p-values"),
                        (BarField::CorrectedPValues, "Corrected p-values"),
                        (BarField::EffectSize, "Effect size"),
                    ],
                );
                rows.check("Sort values", &mut o.sort);
            });
            dialog.group("Figure size", |rows| {
                rows.inches("Width:", &mut o.width, 2.0..=40.0);
                rows.inches("Row height:", &mut o.row_height, 0.05..=2.0);
            });
            dialog.legend(&mut o.legend);
            dialog.truncate(&mut o.truncate_labels);
        }
        PlotKind::Bar => {
            let o = &mut settings.bar;
            let field = &mut settings.feature_field;
            dialog.plain("general", |rows| {
                rows.field(field);
                rows.check("Show average of each group", &mut o.show_averages);
                rows.check("Show p-value", &mut o.show_p_value);
            });
            dialog.group("Figure size", |rows| {
                rows.inches("Height:", &mut o.height, 2.0..=40.0);
                rows.inches("Column width:", &mut o.column_width, 0.05..=2.0);
            });
            dialog.legend(&mut o.legend);
            dialog.truncate(&mut o.truncate_labels);
        }
        PlotKind::Box => {
            let o = &mut settings.box_plot;
            let field = &mut settings.feature_field;
            dialog.plain("general", |rows| {
                rows.field(field);
                rows.check("Show average of each group", &mut o.show_averages);
                rows.check("Show p-value", &mut o.show_p_value);
            });
            dialog.group("Figure size", |rows| {
                rows.inches("Height:", &mut o.height, 2.0..=40.0);
                rows.inches("Width:", &mut o.width, 2.0..=40.0);
            });
            dialog.truncate(&mut o.truncate_labels);
        }
        PlotKind::ProfileBar => {
            let o = &mut settings.profile_bar;
            let only_active = &mut settings.profile_bar_only_active;
            dialog.plain("general", |rows| {
                rows.choice(
                    "Field to plot:",
                    &mut o.sequence_counts,
                    &[
                        (false, "Proportion of sequences (%)"),
                        (true, "Number of sequences"),
                    ],
                );
                rows.count(
                    "Maximum # features to show:",
                    &mut o.features_to_show,
                    1..=1000,
                );
                rows.check("Only show active features", only_active);
            });
            dialog.group("Figure size", |rows| {
                rows.inches("Height:", &mut o.height, 2.0..=40.0);
                rows.inches("Column width:", &mut o.column_width, 0.05..=2.0);
                rows.number(
                    "Bar width (%):",
                    &mut o.bar_width_percent,
                    1.0,
                    10.0..=100.0,
                );
            });
            dialog.group("Confidence intervals", |rows| {
                rows.check("Show CIs", &mut o.show_cis);
                rows.number_if(
                    o.show_cis,
                    "End cap size:",
                    &mut o.end_cap_size,
                    0.1,
                    0.0..=20.0,
                );
            });
            dialog.group("Mark significant features", |rows| {
                rows.check("Mark features with a p-value <=", &mut o.show_p_values);
                rows.number_if(
                    o.show_p_values,
                    "p-value:",
                    &mut o.p_value_threshold,
                    0.001,
                    0.0..=1.0,
                );
            });
            dialog.legend(&mut o.legend);
            dialog.truncate(&mut o.truncate_labels);
        }
        PlotKind::Scatter => {
            let o = &mut settings.scatter;
            let spread = &mut settings.group_spread;
            dialog.plain("general", |rows| {
                if mode == Mode::TwoSamples {
                    rows.check("Show CIs", &mut o.show_spread);
                } else {
                    rows.check("Show spread of each group", &mut o.show_spread);
                    rows.choice_if(
                        o.show_spread,
                        "Spread:",
                        spread,
                        &[
                            (Spread::StandardDeviation, "1 standard deviation"),
                            (Spread::TwoStandardDeviations, "2 standard deviations"),
                            (Spread::Quartiles, "25th and 75th percentiles"),
                            (Spread::Percentiles9And91, "9th and 91st percentiles"),
                            (Spread::Percentiles2And98, "2nd and 98th percentiles"),
                            (Spread::MinimumAndMaximum, "Minimum and maximum"),
                        ],
                    );
                }
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
                rows.check("Show R\u{b2} value", &mut o.show_r2);
            });
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Histogram", |rows| {
                rows.check("Show histograms", &mut o.show_histograms);
                rows.count_if(o.show_histograms, "Bins:", &mut o.bins, 2..=500);
                rows.inches_if(
                    o.show_histograms,
                    "Height:",
                    &mut o.histogram_size,
                    0.1..=5.0,
                );
            });
        }
        PlotKind::SequenceHistogram => {
            let o = &mut settings.sequence_histogram;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Histogram properties", |rows| {
                rows.optional("Custom bin width:", &mut o.bin_width, 10.0, 1.0, 1.0..=1e9);
                rows.check("Show y-axis as log scale", &mut o.log_scale);
            });
            dialog.group("X-axis properties", |rows| {
                rows.limits("Custom x-axis extents:", &mut o.x_limits, (0.0, 1000.0));
            });
            dialog.legend(&mut o.legend);
        }
        PlotKind::PValueHistogram => {
            let o = &mut settings.p_value_histogram;
            let corrected = &mut settings.p_value_histogram_corrected;
            dialog.plain("general", |rows| {
                rows.choice(
                    "Field to plot:",
                    corrected,
                    &[(false, "p-values"), (true, "p-values (corrected)")],
                );
            });
            dialog.group("Main figure size", |rows| {
                rows.size(&mut o.width, &mut o.height)
            });
            dialog.group("Main histogram properties", |rows| {
                rows.number("Bin width:", &mut o.bin_width, 0.001, 0.001..=0.5);
                rows.check("Show y-axis as log scale", &mut o.log_scale);
            });
            // The inset keeps its settings while it is switched off.
            let mut shown = o.inset.is_some();
            let defaults = || {
                PlotSettings::default()
                    .p_value_histogram
                    .inset
                    .unwrap_or(Inset {
                        width_percent: 60.0,
                        height_percent: 60.0,
                        bin_width: 0.002,
                        x_limit: 0.05,
                        log_scale: false,
                    })
            };
            let mut inset = o.inset.clone().unwrap_or_else(defaults);
            dialog.group("Inset figure size", |rows| {
                rows.check("Show inset", &mut shown);
                rows.number_if(
                    shown,
                    "Width (%):",
                    &mut inset.width_percent,
                    1.0,
                    10.0..=90.0,
                );
                rows.number_if(
                    shown,
                    "Height (%):",
                    &mut inset.height_percent,
                    1.0,
                    10.0..=90.0,
                );
            });
            dialog.group("Inset histogram properties", |rows| {
                rows.number_if(
                    shown,
                    "Bin width:",
                    &mut inset.bin_width,
                    0.0005,
                    0.0001..=0.5,
                );
                rows.number_if(
                    shown,
                    "x-axis limit:",
                    &mut inset.x_limit,
                    0.005,
                    0.001..=1.0,
                );
                rows.check_if(shown, "Show y-axis as log scale", &mut inset.log_scale);
            });
            o.inset = shown.then_some(inset);
        }
        PlotKind::MultipleComparison => {
            let o = &mut settings.multiple_comparison;
            dialog.group("Main figure size", |rows| {
                rows.size(&mut o.width, &mut o.height)
            });
            dialog.group("Histogram plot", |rows| {
                rows.number("Bin width:", &mut o.bin_width, 0.001, 0.001..=0.5);
                rows.number("x-axis limit:", &mut o.histogram_limit, 0.01, 0.01..=1.0);
                rows.check("Show y-axis as log scale", &mut o.log_scale);
            });
            dialog.group("Correction plot", |rows| {
                rows.number("x-axis limit:", &mut o.correction_limit, 0.01, 0.01..=1.0);
            });
            dialog.group("Significant features plot", |rows| {
                rows.number("x-axis limit:", &mut o.significant_limit, 0.01, 0.01..=1.0);
            });
        }
        PlotKind::Pca => {
            let o = &mut settings.pca;
            dialog.plain("general", |rows| {
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
                rows.check("Assign unique shapes", &mut o.unique_shapes);
                rows.check_if(
                    o.show_pc3_vs_pc2,
                    "Rotate PC3 labels",
                    &mut o.rotate_pc3_labels,
                );
            });
            dialog.group("Figure size", |rows| {
                rows.check(
                    "Fixed pixels per unit distance (recommended)",
                    &mut o.fixed_scale,
                );
                rows.inches("Width:", &mut o.width, 2.0..=40.0);
                // The height follows from the width while the scale is fixed.
                rows.inches_if(!o.fixed_scale, "Height:", &mut o.height, 2.0..=40.0);
            });
            dialog.group("Plots to show", |rows| {
                // The main panel is always drawn.
                rows.check_if(false, "PC1 vs. PC2", &mut true);
                rows.check("PC3 vs. PC2", &mut o.show_pc3_vs_pc2);
                rows.check("PC1 vs. PC3", &mut o.show_pc1_vs_pc3);
            });
            dialog.legend(&mut o.legend);
        }
        PlotKind::Heatmap => {
            let o = &mut settings.heatmap;
            let counts = &mut settings.heatmap_sequence_counts;
            let only_active = &mut settings.heatmap_only_active;
            dialog.plain("general", |rows| {
                rows.choice(
                    "Field to plot:",
                    counts,
                    &[
                        (false, "Proportion of sequences (%)"),
                        (true, "Number of sequences"),
                    ],
                );
                rows.check("Plot only active features", only_active);
                let maps: Vec<(ColorMap, &str)> =
                    ColorMap::ALL.iter().map(|&m| (m, m.name())).collect();
                rows.choice("Colour map:", &mut o.color_map, &maps);
                rows.check("Show colour scale", &mut o.show_color_scale);
            });
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Sort rows", |rows| {
                rows.order("Method:", &mut o.row_order);
                let clustered = matches!(o.row_order, Order::Cluster(_));
                rows.check_if(clustered, "Show dendrogram", &mut o.show_row_dendrogram);
                let tree = clustered && o.show_row_dendrogram;
                rows.inches_if(
                    tree,
                    "Width (inches):",
                    &mut o.row_dendrogram_width,
                    0.1..=10.0,
                );
                rows.check_if(tree, "Colour clusters", &mut o.show_row_clusters);
                rows.number_if(
                    tree && o.show_row_clusters,
                    "Dendrogram clustering threshold:",
                    &mut o.row_cluster_threshold,
                    0.01,
                    0.0..=1.0,
                );
            });
            dialog.group("Sort columns", |rows| {
                rows.order("Method:", &mut o.column_order);
                let clustered = matches!(o.column_order, Order::Cluster(_));
                rows.check_if(clustered, "Show dendrogram", &mut o.show_column_dendrogram);
                rows.inches_if(
                    clustered && o.show_column_dendrogram,
                    "Height (inches):",
                    &mut o.column_dendrogram_height,
                    0.1..=10.0,
                );
            });
            dialog.legend(&mut o.legend);
            dialog.truncate(&mut o.truncate_labels);
        }
    }
    dialog.changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_plot_has_settings_and_can_be_reset() {
        let ctx = egui::Context::default();
        for mode in Mode::ALL {
            for &kind in PlotKind::available(mode) {
                let mut settings = PlotSettings::default();
                let mut drawn = false;
                let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let before = ui.min_rect();
                    assert!(!show(ui, mode, kind, &mut settings));
                    drawn = ui.min_rect().height() > before.height() + 40.0;
                });
                assert!(drawn, "{mode:?} {kind:?} has no settings");
                // Nothing changes without the user doing something.
                assert_eq!(settings, PlotSettings::default());

                settings.error_bar.width = 12.0;
                settings.post_hoc.width = 12.0;
                settings.reset(mode, kind);
                match kind {
                    PlotKind::ExtendedErrorBar => assert_eq!(settings.error_bar.width, 7.0),
                    PlotKind::PostHoc => assert_eq!(settings.post_hoc.width, 7.0),
                    _ => assert_eq!(settings.error_bar.width, 12.0),
                }
            }
        }
    }
}
