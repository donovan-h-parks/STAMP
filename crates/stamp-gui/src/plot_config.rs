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
}

impl Rows<'_> {
    fn number(
        &mut self,
        label: &str,
        value: &mut f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        self.ui.label(label);
        let drag = egui::DragValue::new(value).speed(speed).range(range);
        self.changed |= self.ui.add(drag).changed();
        self.ui.end_row();
    }

    /// A length in inches.
    fn inches(&mut self, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>) {
        self.ui.label(label);
        let drag = egui::DragValue::new(value)
            .speed(0.05)
            .range(range)
            .suffix(" in");
        self.changed |= self.ui.add(drag).changed();
        self.ui.end_row();
    }

    fn size(&mut self, width: &mut f64, height: &mut f64) {
        self.inches("Figure width:", width, 2.0..=40.0);
        self.inches("Figure height:", height, 2.0..=40.0);
    }

    fn count(&mut self, label: &str, value: &mut usize, range: std::ops::RangeInclusive<usize>) {
        self.ui.label(label);
        self.changed |= self
            .ui
            .add(egui::DragValue::new(value).range(range))
            .changed();
        self.ui.end_row();
    }

    fn check(&mut self, label: &str, value: &mut bool) {
        self.changed |= self.ui.checkbox(value, label).changed();
        self.ui.end_row();
    }

    fn choice<T: PartialEq + Clone>(&mut self, label: &str, value: &mut T, options: &[(T, &str)]) {
        self.ui.label(label);
        let options: Vec<(T, String)> = options
            .iter()
            .map(|(option, name)| (option.clone(), name.to_string()))
            .collect();
        self.changed |= choose(self.ui, &format!("config-{label}"), value, &options);
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
        self.choice("Legend position:", value, &positions);
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

    fn error_bar(&mut self, options: &mut extended_error_bar::Options, groups: bool, legend: bool) {
        self.inches("Figure width:", &mut options.width, 2.0..=40.0);
        self.inches("Row height:", &mut options.row_height, 0.05..=2.0);
        {
            self.choice(
                "Sort features by:",
                &mut options.sort_by,
                &[
                    (SortBy::PValues, "p-values"),
                    (SortBy::EffectSizes, "Effect sizes"),
                    (SortBy::FeatureLabels, "Feature labels"),
                ],
            );
        }
        self.check("Show bar plot", &mut options.show_bars);
        if groups {
            self.check("Show standard deviation", &mut options.show_std_dev);
            self.number("End cap size:", &mut options.end_cap_size, 0.1, 0.0..=20.0);
        }
        self.check("Show p-values", &mut options.show_p_values);
        self.check(
            "Show corrected p-values",
            &mut options.show_corrected_p_values,
        );
        self.number("Marker size:", &mut options.marker_size, 1.0, 1.0..=500.0);
        self.limits(
            "Fixed effect-size axis:",
            &mut options.effect_limits,
            (-10.0, 10.0),
        );
        if !legend {
            self.truncate(&mut options.truncate_labels);
            return;
        }
        self.choice(
            "Legend position:",
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
        self.truncate(&mut options.truncate_labels);
    }
}

/// The controls for the settings of one plot. Returns true if a setting
/// changed.
pub fn show(ui: &mut egui::Ui, mode: Mode, kind: PlotKind, settings: &mut PlotSettings) -> bool {
    let mut changed = false;
    egui::Grid::new("plot-config")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            let mut rows = Rows { ui, changed: false };
            match kind {
                PlotKind::ExtendedErrorBar => {
                    if mode == Mode::TwoSamples {
                        rows.choice(
                            "Bars show:",
                            &mut settings.error_bar_sequence_counts,
                            &[(false, "Proportion (%)"), (true, "Sequences")],
                        );
                    }
                    rows.error_bar(&mut settings.error_bar, mode != Mode::TwoSamples, true)
                }
                PlotKind::PostHoc => {
                    rows.check(
                        "Filter rows above the nominal level",
                        &mut settings.post_hoc_only_significant,
                    );
                    rows.error_bar(&mut settings.post_hoc, true, false)
                }
                PlotKind::Bar if mode == Mode::TwoSamples => {
                    let o = &mut settings.stat_bar;
                    rows.inches("Figure width:", &mut o.width, 2.0..=40.0);
                    rows.inches("Row height:", &mut o.row_height, 0.05..=2.0);
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
                    rows.check("Sort features by the plotted value", &mut o.sort);
                    rows.legend(&mut o.legend);
                    rows.truncate(&mut o.truncate_labels);
                }
                PlotKind::Bar => {
                    let o = &mut settings.bar;
                    rows.inches("Width per sample:", &mut o.column_width, 0.05..=2.0);
                    rows.inches("Figure height:", &mut o.height, 2.0..=40.0);
                    rows.field(&mut settings.feature_field);
                    rows.check("Show group means", &mut o.show_averages);
                    rows.check("Show p-value", &mut o.show_p_value);
                    rows.legend(&mut o.legend);
                    rows.truncate(&mut o.truncate_labels);
                }
                PlotKind::Box => {
                    let o = &mut settings.box_plot;
                    rows.size(&mut o.width, &mut o.height);
                    rows.field(&mut settings.feature_field);
                    rows.check("Show group means", &mut o.show_averages);
                    rows.check("Show p-value", &mut o.show_p_value);
                    rows.truncate(&mut o.truncate_labels);
                }
                PlotKind::ProfileBar => {
                    let o = &mut settings.profile_bar;
                    rows.inches("Width per feature:", &mut o.column_width, 0.05..=2.0);
                    rows.inches("Figure height:", &mut o.height, 2.0..=40.0);
                    rows.choice(
                        "Field to plot:",
                        &mut o.sequence_counts,
                        &[
                            (false, "Proportion of sequences (%)"),
                            (true, "Number of sequences"),
                        ],
                    );
                    rows.check(
                        "Only active features",
                        &mut settings.profile_bar_only_active,
                    );
                    rows.count("Features to show:", &mut o.features_to_show, 1..=1000);
                    rows.number(
                        "Bar width (%):",
                        &mut o.bar_width_percent,
                        1.0,
                        10.0..=100.0,
                    );
                    rows.check("Show confidence intervals", &mut o.show_cis);
                    rows.number("End cap size:", &mut o.end_cap_size, 0.1, 0.0..=20.0);
                    rows.check("Mark significant features", &mut o.show_p_values);
                    rows.number(
                        "p-value threshold:",
                        &mut o.p_value_threshold,
                        0.001,
                        0.0..=1.0,
                    );
                    rows.legend(&mut o.legend);
                    rows.truncate(&mut o.truncate_labels);
                }
                PlotKind::Scatter => {
                    let o = &mut settings.scatter;
                    rows.size(&mut o.width, &mut o.height);
                    if mode == Mode::TwoSamples {
                        rows.check("Show confidence intervals", &mut o.show_spread);
                    } else {
                        rows.check("Show spread of each group", &mut o.show_spread);
                        rows.choice(
                            "Spread shown:",
                            &mut settings.group_spread,
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
                    rows.check("Show histograms", &mut o.show_histograms);
                    rows.count("Histogram bins:", &mut o.bins, 2..=500);
                    rows.inches("Histogram size:", &mut o.histogram_size, 0.1..=5.0);
                    rows.check("Show R\u{b2} value", &mut o.show_r2);
                }
                PlotKind::SequenceHistogram => {
                    let o = &mut settings.sequence_histogram;
                    rows.size(&mut o.width, &mut o.height);
                    rows.optional("Bin width:", &mut o.bin_width, 10.0, 1.0, 1.0..=1e9);
                    rows.limits("Fixed x axis:", &mut o.x_limits, (0.0, 1000.0));
                    rows.check("Logarithmic y axis", &mut o.log_scale);
                    rows.legend(&mut o.legend);
                }
                PlotKind::PValueHistogram => {
                    let o = &mut settings.p_value_histogram;
                    rows.size(&mut o.width, &mut o.height);
                    rows.choice(
                        "Field to plot:",
                        &mut settings.p_value_histogram_corrected,
                        &[(false, "p-values"), (true, "p-values (corrected)")],
                    );
                    rows.number("Bin width:", &mut o.bin_width, 0.001, 0.001..=0.5);
                    rows.check("Logarithmic y axis", &mut o.log_scale);
                    let mut inset = o.inset.is_some();
                    rows.check("Show inset of small p-values", &mut inset);
                    if inset != o.inset.is_some() {
                        o.inset = inset.then(|| {
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
                        });
                    }
                    if let Some(inset) = &mut o.inset {
                        rows.number(
                            "Inset width (%):",
                            &mut inset.width_percent,
                            1.0,
                            10.0..=90.0,
                        );
                        rows.number(
                            "Inset height (%):",
                            &mut inset.height_percent,
                            1.0,
                            10.0..=90.0,
                        );
                        rows.number(
                            "Inset bin width:",
                            &mut inset.bin_width,
                            0.0005,
                            0.0001..=0.5,
                        );
                        rows.number(
                            "Inset largest p-value:",
                            &mut inset.x_limit,
                            0.005,
                            0.001..=1.0,
                        );
                        rows.check("Inset logarithmic y axis", &mut inset.log_scale);
                    }
                }
                PlotKind::MultipleComparison => {
                    let o = &mut settings.multiple_comparison;
                    rows.size(&mut o.width, &mut o.height);
                    rows.number("Bin width:", &mut o.bin_width, 0.001, 0.001..=0.5);
                    rows.check("Histogram: logarithmic y axis", &mut o.log_scale);
                    rows.number(
                        "Histogram: largest p-value:",
                        &mut o.histogram_limit,
                        0.01,
                        0.01..=1.0,
                    );
                    rows.number(
                        "Correction: largest p-value:",
                        &mut o.correction_limit,
                        0.01,
                        0.01..=1.0,
                    );
                    rows.number(
                        "Significant: largest p-value:",
                        &mut o.significant_limit,
                        0.01,
                        0.01..=1.0,
                    );
                }
                PlotKind::Pca => {
                    let o = &mut settings.pca;
                    rows.size(&mut o.width, &mut o.height);
                    rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
                    rows.check("Show PC3 vs. PC2", &mut o.show_pc3_vs_pc2);
                    rows.check("Show PC1 vs. PC3", &mut o.show_pc1_vs_pc3);
                    rows.check("Rotate PC3 labels", &mut o.rotate_pc3_labels);
                    rows.check("Fixed length per unit distance", &mut o.fixed_scale);
                    rows.check("Unique marker shape for each group", &mut o.unique_shapes);
                    rows.legend(&mut o.legend);
                }
                PlotKind::Heatmap => {
                    let o = &mut settings.heatmap;
                    rows.size(&mut o.width, &mut o.height);
                    rows.choice(
                        "Field to plot:",
                        &mut settings.heatmap_sequence_counts,
                        &[
                            (false, "Proportion of sequences (%)"),
                            (true, "Number of sequences"),
                        ],
                    );
                    rows.check(
                        "Plot only active features",
                        &mut settings.heatmap_only_active,
                    );
                    rows.order("Order features by:", &mut o.row_order);
                    rows.check("Show feature dendrogram", &mut o.show_row_dendrogram);
                    rows.inches(
                        "Feature dendrogram width:",
                        &mut o.row_dendrogram_width,
                        0.1..=10.0,
                    );
                    rows.order("Order samples by:", &mut o.column_order);
                    rows.check("Show sample dendrogram", &mut o.show_column_dendrogram);
                    rows.inches(
                        "Sample dendrogram height:",
                        &mut o.column_dendrogram_height,
                        0.1..=10.0,
                    );
                    let maps: Vec<(ColorMap, &str)> =
                        ColorMap::ALL.iter().map(|&m| (m, m.name())).collect();
                    rows.choice("Colour map:", &mut o.color_map, &maps);
                    rows.check("Show colour scale", &mut o.show_color_scale);
                    rows.check("Colour clusters of features", &mut o.show_row_clusters);
                    rows.number(
                        "Feature cluster threshold:",
                        &mut o.row_cluster_threshold,
                        0.01,
                        0.0..=1.0,
                    );
                    rows.legend(&mut o.legend);
                    rows.truncate(&mut o.truncate_labels);
                }
            }
            changed = rows.changed;
        });
    changed
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
