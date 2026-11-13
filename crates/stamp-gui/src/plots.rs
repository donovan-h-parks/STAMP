//! Choosing a plot and building its figure from the session's results.

use stamp_core::stats::results::format_p_value;
use stamp_plot::axes::LegendPosition;
use stamp_plot::extended_error_bar;
use stamp_plot::{
    feature_plots, heatmap, histogram, pca_plot, post_hoc, sample_plots, scatter, Color, Figure,
};

use crate::session::{Mode, Session};

/// The plots, each available in one or more modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotKind {
    ExtendedErrorBar,
    Bar,
    Box,
    ProfileBar,
    Scatter,
    SequenceHistogram,
    PValueHistogram,
    MultipleComparison,
    Pca,
    Heatmap,
    PostHoc,
}

impl PlotKind {
    pub fn name(self) -> &'static str {
        match self {
            PlotKind::ExtendedErrorBar => "Extended error bar",
            PlotKind::Bar => "Bar plot",
            PlotKind::Box => "Box plot",
            PlotKind::ProfileBar => "Profile bar plot",
            PlotKind::Scatter => "Scatter plot",
            PlotKind::SequenceHistogram => "Sequence histogram",
            PlotKind::PValueHistogram => "p-value histogram",
            PlotKind::MultipleComparison => "Multiple comparison plots",
            PlotKind::Pca => "PCA plot",
            PlotKind::Heatmap => "Heatmap plot",
            PlotKind::PostHoc => "Post-hoc plot",
        }
    }

    /// The plots offered in a mode; the first is the default.
    pub fn available(mode: Mode) -> &'static [PlotKind] {
        match mode {
            Mode::TwoSamples => &[
                PlotKind::ExtendedErrorBar,
                PlotKind::Bar,
                PlotKind::ProfileBar,
                PlotKind::Scatter,
                PlotKind::SequenceHistogram,
                PlotKind::PValueHistogram,
                PlotKind::MultipleComparison,
            ],
            Mode::TwoGroups => &[
                PlotKind::ExtendedErrorBar,
                PlotKind::Bar,
                PlotKind::Box,
                PlotKind::Scatter,
                PlotKind::Pca,
                PlotKind::Heatmap,
            ],
            Mode::MultipleGroups => &[
                PlotKind::Pca,
                PlotKind::Heatmap,
                PlotKind::Bar,
                PlotKind::Box,
                PlotKind::PostHoc,
            ],
        }
    }

    /// True if the plot can draw chosen features in red.
    pub fn supports_highlight(self, mode: Mode) -> bool {
        match self {
            PlotKind::ExtendedErrorBar | PlotKind::Scatter | PlotKind::ProfileBar => true,
            PlotKind::Bar => mode == Mode::TwoSamples,
            _ => false,
        }
    }

    /// True if the plot shows the selected feature only.
    pub fn uses_selected_feature(self, mode: Mode) -> bool {
        match self {
            PlotKind::Box | PlotKind::PostHoc => true,
            PlotKind::Bar => mode != Mode::TwoSamples,
            _ => false,
        }
    }
}

/// The settings of every plot, as set in the "Configure plot" window.
/// Lengths are in inches.
#[derive(Debug, Clone, PartialEq)]
pub struct PlotSettings {
    pub error_bar: extended_error_bar::Options,
    /// Show sequence counts instead of proportions in the bars of the
    /// two-sample extended error bar plot.
    pub error_bar_sequence_counts: bool,
    pub post_hoc: extended_error_bar::Options,
    /// Leave pairs of groups that do not differ significantly out of the
    /// post-hoc plot.
    pub post_hoc_only_significant: bool,
    /// The two-sample bar plot.
    pub stat_bar: sample_plots::StatBarOptions,
    pub profile_bar: sample_plots::ProfileBarOptions,
    /// Draw only the features that pass the filters in the profile bar plot.
    pub profile_bar_only_active: bool,
    pub scatter: scatter::Options,
    /// What the error bars of the two-group scatter plot span.
    pub group_spread: scatter::Spread,
    pub sequence_histogram: histogram::SequenceOptions,
    pub p_value_histogram: histogram::PValueOptions,
    /// Plot corrected rather than uncorrected p-values in the histogram.
    pub p_value_histogram_corrected: bool,
    pub multiple_comparison: sample_plots::MultCompOptions,
    /// The bar plot of one feature across groups.
    pub bar: feature_plots::BarOptions,
    pub box_plot: feature_plots::BoxOptions,
    /// What the group bar and box plots show for each sample.
    pub feature_field: feature_plots::Field,
    pub pca: pca_plot::Options,
    pub heatmap: heatmap::Options,
    /// Draw only the features that pass the filters in the heatmap.
    pub heatmap_only_active: bool,
    /// Show sequence counts instead of proportions in the heatmap.
    pub heatmap_sequence_counts: bool,
    /// Features drawn in red by the plots that support highlighting.
    pub highlighted: Vec<String>,
}

impl Default for PlotSettings {
    fn default() -> Self {
        let legend = LegendPosition::UpperRight;
        PlotSettings {
            error_bar: extended_error_bar::Options {
                legend: extended_error_bar::Legend::LowerCenter,
                ..Default::default()
            },
            error_bar_sequence_counts: false,
            post_hoc: extended_error_bar::Options::default(),
            post_hoc_only_significant: true,
            stat_bar: sample_plots::StatBarOptions {
                legend,
                ..Default::default()
            },
            profile_bar: sample_plots::ProfileBarOptions::default(),
            profile_bar_only_active: true,
            scatter: scatter::Options {
                height: 6.0,
                ..Default::default()
            },
            group_spread: scatter::Spread::Percentiles2And98,
            sequence_histogram: histogram::SequenceOptions {
                height: 6.0,
                ..Default::default()
            },
            p_value_histogram: histogram::PValueOptions {
                height: 6.0,
                ..Default::default()
            },
            p_value_histogram_corrected: true,
            multiple_comparison: sample_plots::MultCompOptions::default(),
            bar: feature_plots::BarOptions {
                legend,
                ..Default::default()
            },
            box_plot: feature_plots::BoxOptions {
                height: 6.0,
                ..Default::default()
            },
            feature_field: feature_plots::Field::Proportion,
            pca: pca_plot::Options {
                legend,
                ..Default::default()
            },
            heatmap: heatmap::Options {
                height: 6.0,
                ..Default::default()
            },
            heatmap_only_active: false,
            heatmap_sequence_counts: false,
            highlighted: Vec::new(),
        }
    }
}

impl PlotSettings {
    /// Return the settings of one plot to their defaults.
    pub fn reset(&mut self, mode: Mode, kind: PlotKind) {
        let defaults = PlotSettings::default();
        match kind {
            PlotKind::ExtendedErrorBar => {
                self.error_bar = defaults.error_bar;
                self.error_bar_sequence_counts = defaults.error_bar_sequence_counts;
            }
            PlotKind::PostHoc => {
                self.post_hoc = defaults.post_hoc;
                self.post_hoc_only_significant = defaults.post_hoc_only_significant;
            }
            PlotKind::Bar if mode == Mode::TwoSamples => self.stat_bar = defaults.stat_bar,
            PlotKind::Bar => {
                self.bar = defaults.bar;
                self.feature_field = defaults.feature_field;
            }
            PlotKind::Box => {
                self.box_plot = defaults.box_plot;
                self.feature_field = defaults.feature_field;
            }
            PlotKind::ProfileBar => {
                self.profile_bar = defaults.profile_bar;
                self.profile_bar_only_active = defaults.profile_bar_only_active;
            }
            PlotKind::Scatter => {
                self.scatter = defaults.scatter;
                self.group_spread = defaults.group_spread;
            }
            PlotKind::SequenceHistogram => self.sequence_histogram = defaults.sequence_histogram,
            PlotKind::PValueHistogram => {
                self.p_value_histogram = defaults.p_value_histogram;
                self.p_value_histogram_corrected = defaults.p_value_histogram_corrected;
            }
            PlotKind::MultipleComparison => self.multiple_comparison = defaults.multiple_comparison,
            PlotKind::Pca => self.pca = defaults.pca,
            PlotKind::Heatmap => {
                self.heatmap = defaults.heatmap;
                self.heatmap_only_active = defaults.heatmap_only_active;
                self.heatmap_sequence_counts = defaults.heatmap_sequence_counts;
            }
        }
    }
}

const NO_RESULTS: &str = "Load a profile to begin";
const NO_METADATA: &str = "Load a metadata file to compare groups of samples";
const NO_FEATURE: &str = "Select a feature in the feature table";

/// Build the figure for the current results of `mode`.
pub fn build(session: &Session, mode: Mode, kind: PlotKind, settings: &PlotSettings) -> Figure {
    if session.tree.is_none() {
        return Figure::message(NO_RESULTS);
    }
    let highlighted = || settings.highlighted.clone();
    let error_bar = extended_error_bar::Options {
        highlighted: highlighted(),
        ..settings.error_bar.clone()
    };
    let scatter_options = scatter::Options {
        highlighted: highlighted(),
        ..settings.scatter.clone()
    };
    let field = settings.feature_field;
    let feature = session.selected_feature.as_deref();

    match mode {
        Mode::TwoSamples => {
            let Some((profile, results)) = &session.samples.output else {
                return Figure::message("Running the two-sample analysis...");
            };
            let rows = session.sample_rows();
            match kind {
                PlotKind::ExtendedErrorBar => {
                    let mut data = extended_error_bar::Data::from_sample_results(
                        profile,
                        results,
                        rows.iter().copied(),
                    );
                    if settings.error_bar_sequence_counts {
                        data = data.with_sequence_counts(rows);
                    }
                    extended_error_bar::plot(&data, &error_bar)
                }
                PlotKind::Bar => sample_plots::stat_bar(
                    profile,
                    results,
                    rows,
                    &sample_plots::StatBarOptions {
                        highlighted: highlighted(),
                        ..settings.stat_bar.clone()
                    },
                ),
                PlotKind::ProfileBar => sample_plots::profile_bar(
                    profile,
                    if settings.profile_bar_only_active {
                        rows
                    } else {
                        results.rows.iter().collect()
                    },
                    &sample_plots::ProfileBarOptions {
                        highlighted: highlighted(),
                        ..settings.profile_bar.clone()
                    },
                ),
                PlotKind::Scatter => scatter::plot(
                    &scatter::Data::from_sample_profile(profile),
                    &scatter_options,
                ),
                PlotKind::SequenceHistogram => {
                    histogram::sequence_histogram(profile, &settings.sequence_histogram)
                }
                PlotKind::PValueHistogram => {
                    let corrected = settings.p_value_histogram_corrected;
                    let p: Vec<f64> = results
                        .rows
                        .iter()
                        .map(|r| {
                            if corrected {
                                r.p_value_corrected
                            } else {
                                r.p_value
                            }
                        })
                        .collect();
                    let label = if corrected {
                        "p-value (corrected)"
                    } else {
                        "p-value"
                    };
                    histogram::p_value_histogram(
                        &p,
                        &histogram::PValueOptions {
                            label: label.to_string(),
                            ..settings.p_value_histogram.clone()
                        },
                    )
                }
                PlotKind::MultipleComparison => {
                    sample_plots::multiple_comparison(results, &settings.multiple_comparison)
                }
                _ => Figure::message("This plot is not available for two samples"),
            }
        }
        Mode::TwoGroups => {
            if session.metadata.is_none() {
                return Figure::message(NO_METADATA);
            }
            let Some((profile, results)) = &session.groups.output else {
                return Figure::message("Running the two-group analysis...");
            };
            let rows = session.group_rows();
            let color1 = session.group_color(&profile.group_name1);
            let color2 = session.group_color(&profile.group_name2);
            let feature_data = || {
                let name = feature?;
                let mut data =
                    feature_plots::FeatureData::from_group_profile(profile, name, field)?;
                data.groups[0].color = color1;
                data.groups[1].color = color2;
                data.p_value = results
                    .rows
                    .iter()
                    .find(|r| r.feature == name)
                    .map(|r| format_p_value(r.p_value_corrected, -15));
                Some(data)
            };
            match kind {
                PlotKind::ExtendedErrorBar => {
                    let mut data =
                        extended_error_bar::Data::from_group_results(profile, results, rows);
                    (data.color1, data.color2) = (color1, color2);
                    extended_error_bar::plot(&data, &error_bar)
                }
                PlotKind::Bar => match feature_data() {
                    Some(data) => feature_plots::bar_plot(&data, &settings.bar),
                    None => Figure::message(NO_FEATURE),
                },
                PlotKind::Box => match feature_data() {
                    Some(data) => feature_plots::box_plot(&data, &settings.box_plot),
                    None => Figure::message(NO_FEATURE),
                },
                PlotKind::Scatter => scatter::plot(
                    &scatter::Data {
                        x_color: color1,
                        y_color: color2,
                        ..scatter::Data::from_group_profile(profile, settings.group_spread)
                    },
                    &scatter_options,
                ),
                PlotKind::Pca => pca_plot::plot(
                    &pca_plot::Data {
                        group_colors: vec![color1, color2],
                        ..pca_plot::Data::from_group_profile(profile)
                    },
                    &settings.pca,
                ),
                PlotKind::Heatmap => {
                    let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                    heatmap::plot(
                        &heatmap::Data {
                            group_colors: vec![color1, color2],
                            ..heatmap::Data::from_group_profile_field(
                                profile,
                                settings.heatmap_only_active.then_some(names.as_slice()),
                                settings.heatmap_sequence_counts,
                            )
                        },
                        &settings.heatmap,
                    )
                }
                _ => Figure::message("This plot is not available for two groups"),
            }
        }
        Mode::MultipleGroups => {
            if session.metadata.is_none() {
                return Figure::message(NO_METADATA);
            }
            let Some((profile, results)) = &session.multi.output else {
                return Figure::message(
                    session
                        .error
                        .as_deref()
                        .unwrap_or("Running the multiple-group analysis..."),
                );
            };
            let rows = session.multi_rows();
            let colors: Vec<Color> = profile
                .group_names
                .iter()
                .map(|g| session.group_color(g))
                .collect();
            let feature_data = || {
                let name = feature?;
                let mut data =
                    feature_plots::FeatureData::from_multi_group_profile(profile, name, field)?;
                for (group, color) in data.groups.iter_mut().zip(&colors) {
                    group.color = *color;
                }
                data.p_value = results
                    .rows
                    .iter()
                    .find(|r| r.feature == name)
                    .map(|r| format_p_value(r.p_value_corrected, -15));
                Some(data)
            };
            match kind {
                PlotKind::Pca => pca_plot::plot(
                    &pca_plot::Data {
                        group_colors: colors.clone(),
                        ..pca_plot::Data::from_multi_group_profile(profile)
                    },
                    &settings.pca,
                ),
                PlotKind::Heatmap => {
                    let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                    heatmap::plot(
                        &heatmap::Data {
                            group_colors: colors.clone(),
                            ..heatmap::Data::from_multi_group_profile_field(
                                profile,
                                settings.heatmap_only_active.then_some(names.as_slice()),
                                settings.heatmap_sequence_counts,
                            )
                        },
                        &settings.heatmap,
                    )
                }
                PlotKind::Bar => match feature_data() {
                    Some(data) => feature_plots::bar_plot(&data, &settings.bar),
                    None => Figure::message(NO_FEATURE),
                },
                PlotKind::Box => match feature_data() {
                    Some(data) => feature_plots::box_plot(&data, &settings.box_plot),
                    None => Figure::message(NO_FEATURE),
                },
                PlotKind::PostHoc => match (feature, session.post_hoc()) {
                    (Some(name), Some(Ok(result))) => post_hoc::plot_with_colors(
                        profile,
                        name,
                        &result,
                        session.multi.post_hoc_coverage,
                        settings.post_hoc_only_significant,
                        &colors,
                        &settings.post_hoc,
                    ),
                    (_, Some(Err(message))) => Figure::message(&message),
                    _ => Figure::message(NO_FEATURE),
                },
                _ => Figure::message("This plot is not available for multiple groups"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        let spf = "F\tA1\tA2\tA3\tB1\tB2\tB3\tC1\tC2\tC3\n\
            x\t10\t12\t11\t30\t33\t31\t20\t21\t19\n\
            y\t50\t48\t52\t50\t49\t51\t50\t50\t50\n\
            z\t40\t40\t37\t20\t18\t18\t30\t29\t31\n\
            w\t5\t9\t2\t8\t3\t6\t4\t7\t5\n";
        let meta = "Id\tGroup\nA1\tA\nA2\tA\nA3\tA\nB1\tB\nB2\tB\nB3\tB\nC1\tC\nC2\tC\nC3\tC\n";
        let mut session = Session::default();
        session.load_profile("t.spf", spf).unwrap();
        session.load_metadata("t.tsv", meta).unwrap();
        for mode in Mode::ALL {
            session.run_to_completion(mode);
        }
        session
    }

    #[test]
    fn every_plot_of_every_mode_draws_something() {
        let session = session();
        for mode in Mode::ALL {
            for &kind in PlotKind::available(mode) {
                let figure = build(&session, mode, kind, &PlotSettings::default());
                assert!(
                    figure.shapes.len() > 5,
                    "{mode:?} {kind:?} drew {} shapes",
                    figure.shapes.len()
                );
                assert!(!kind.name().is_empty());
            }
        }
    }

    #[test]
    fn explains_what_is_missing() {
        let message = |figure: Figure| match &figure.shapes[0] {
            stamp_plot::Shape::Text { text, .. } => text.clone(),
            _ => String::new(),
        };
        let empty = Session::default();
        assert_eq!(
            message(build(
                &empty,
                Mode::TwoSamples,
                PlotKind::Scatter,
                &PlotSettings::default()
            )),
            NO_RESULTS
        );

        let mut no_metadata = Session::default();
        no_metadata
            .load_profile("t", "F\tA\tB\nx\t1\t2\ny\t3\t4\n")
            .unwrap();
        assert_eq!(
            message(build(
                &no_metadata,
                Mode::TwoGroups,
                PlotKind::Pca,
                &PlotSettings::default()
            )),
            NO_METADATA
        );

        let mut session = session();
        session.selected_feature = None;
        assert_eq!(
            message(build(
                &session,
                Mode::TwoGroups,
                PlotKind::Box,
                &PlotSettings::default()
            )),
            NO_FEATURE
        );
        assert!(PlotKind::Bar.uses_selected_feature(Mode::TwoGroups));
        assert!(!PlotKind::Bar.uses_selected_feature(Mode::TwoSamples));
    }

    #[test]
    fn group_colours_chosen_by_the_user_reach_the_plots() {
        let mut session = session();
        let pink = Color::rgb(255, 0, 200);
        session.set_group_color("B", pink);
        let uses_pink = |figure: &Figure| {
            figure.shapes.iter().any(|shape| match shape {
                stamp_plot::Shape::Rect { fill, .. }
                | stamp_plot::Shape::Circle { fill, .. }
                | stamp_plot::Shape::Polygon { fill, .. } => *fill == Some(pink),
                _ => false,
            })
        };
        for (mode, kind) in [
            (Mode::TwoGroups, PlotKind::ExtendedErrorBar),
            (Mode::TwoGroups, PlotKind::Bar),
            (Mode::TwoGroups, PlotKind::Box),
            (Mode::TwoGroups, PlotKind::Scatter),
            (Mode::TwoGroups, PlotKind::Pca),
            (Mode::TwoGroups, PlotKind::Heatmap),
            (Mode::MultipleGroups, PlotKind::Pca),
            (Mode::MultipleGroups, PlotKind::Heatmap),
            (Mode::MultipleGroups, PlotKind::Bar),
            (Mode::MultipleGroups, PlotKind::Box),
            (Mode::MultipleGroups, PlotKind::PostHoc),
        ] {
            let figure = build(&session, mode, kind, &PlotSettings::default());
            assert!(
                uses_pink(&figure),
                "{mode:?} {kind:?} ignores the chosen colour"
            );
        }
        // Groups without a chosen colour keep theirs from the palette.
        assert_eq!(session.group_color("A"), stamp_plot::axes::group_color(0));
        assert_eq!(session.group_color("C"), stamp_plot::axes::group_color(2));
    }
}
