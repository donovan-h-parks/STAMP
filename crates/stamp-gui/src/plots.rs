//! Choosing a plot and building its figure from the session's results.

use stamp_core::stats::ancom::ancom;
use stamp_core::stats::diversity::{AlphaMetric, Distance, Normalisation};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::results::format_p_value;
use stamp_core::stats::rng_from_seed;
use stamp_plot::axes::LegendPosition;
use stamp_plot::axes::AXES_COLOR;
use stamp_plot::community_plots::{self, Community};
use stamp_plot::extended_error_bar::{self, SAMPLE1_COLOR, SAMPLE2_COLOR};
use stamp_plot::{
    ancom_plot, composition, correlation_plot, feature_plots, heatmap, histogram, pca_plot,
    post_hoc, rank_abundance, rarefaction, sample_plots, scatter, shared_features, volcano, Color,
    Figure,
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
    Rarefaction,
    Volcano,
    Composition,
    AlphaDiversity,
    Pcoa,
    DistanceBox,
    CorrelationScatter,
    Nmds,
    SharedFeatures,
    RankAbundance,
    Ancom,
    Mantel,
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
            PlotKind::Rarefaction => "Rarefaction curves",
            PlotKind::Volcano => "Volcano plot",
            PlotKind::Composition => "Composition bar plot",
            PlotKind::AlphaDiversity => "Alpha diversity box plot",
            PlotKind::Pcoa => "PCoA plot",
            PlotKind::DistanceBox => "Distance box plot",
            PlotKind::CorrelationScatter => "Correlation scatter plot",
            PlotKind::Nmds => "NMDS plot",
            PlotKind::SharedFeatures => "Shared features plot",
            PlotKind::RankAbundance => "Rank-abundance curves",
            PlotKind::Ancom => "ANCOM plot",
            PlotKind::Mantel => "Mantel test plot",
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
                PlotKind::Rarefaction,
                PlotKind::Volcano,
                PlotKind::Composition,
                PlotKind::SharedFeatures,
                PlotKind::RankAbundance,
            ],
            Mode::TwoGroups => &[
                PlotKind::ExtendedErrorBar,
                PlotKind::Bar,
                PlotKind::Box,
                PlotKind::Scatter,
                PlotKind::Pca,
                PlotKind::Heatmap,
                PlotKind::Rarefaction,
                PlotKind::Volcano,
                PlotKind::Composition,
                PlotKind::AlphaDiversity,
                PlotKind::Pcoa,
                PlotKind::Nmds,
                PlotKind::DistanceBox,
                PlotKind::Ancom,
                PlotKind::SharedFeatures,
                PlotKind::RankAbundance,
            ],
            Mode::Correlation => &[
                PlotKind::CorrelationScatter,
                PlotKind::Volcano,
                PlotKind::PValueHistogram,
                PlotKind::Mantel,
            ],
            Mode::MultipleGroups => &[
                PlotKind::Pca,
                PlotKind::Heatmap,
                PlotKind::Bar,
                PlotKind::Box,
                PlotKind::PostHoc,
                PlotKind::Rarefaction,
                PlotKind::Volcano,
                PlotKind::Composition,
                PlotKind::AlphaDiversity,
                PlotKind::Pcoa,
                PlotKind::Nmds,
                PlotKind::DistanceBox,
                PlotKind::Ancom,
                PlotKind::SharedFeatures,
                PlotKind::RankAbundance,
            ],
        }
    }

    /// True if the plot can draw chosen features in red.
    pub fn supports_highlight(self, mode: Mode) -> bool {
        match self {
            PlotKind::ExtendedErrorBar
            | PlotKind::Scatter
            | PlotKind::ProfileBar
            | PlotKind::Volcano
            | PlotKind::Ancom => true,
            PlotKind::Bar => mode == Mode::TwoSamples,
            _ => false,
        }
    }

    /// True if the plot shows the selected feature only.
    pub fn uses_selected_feature(self, mode: Mode) -> bool {
        match self {
            PlotKind::Box | PlotKind::PostHoc | PlotKind::CorrelationScatter => true,
            PlotKind::Bar => mode != Mode::TwoSamples,
            _ => false,
        }
    }
}

/// The settings of every plot, as set in the "Configure plot" window.
/// Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
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
    pub rarefaction: rarefaction::Options,
    pub volcano: volcano::Options,
    pub composition: composition::Options,
    pub correlation: correlation_plot::Options,
    /// How samples are made comparable for the diversity plots, the
    /// distance between samples they use, and the number of permutations
    /// of the community tests.
    pub community_normalisation: Normalisation,
    pub community_distance: Distance,
    pub community_permutations: usize,
    pub alpha_metric: AlphaMetric,
    pub alpha_box: feature_plots::BoxOptions,
    pub pcoa: pca_plot::Options,
    /// Write the outcome of PERMANOVA, ANOSIM and PERMDISP above the PCoA
    /// plot.
    pub pcoa_show_tests: bool,
    pub distance_box: feature_plots::BoxOptions,
    pub nmds: pca_plot::Options,
    pub mantel: correlation_plot::Options,
    pub shared: shared_features::Options,
    pub rank_abundance: rank_abundance::Options,
    pub ancom: ancom_plot::Options,
    /// The test ANCOM applies to each log ratio, and its significance
    /// level.
    pub ancom_test: MultiGroupTest,
    pub ancom_alpha: f64,
    /// Draw only the features that pass the filters in the heatmap.
    pub heatmap_only_active: bool,
    /// Show sequence counts instead of proportions in the heatmap.
    pub heatmap_sequence_counts: bool,
    /// Features drawn in red by the plots that support highlighting.
    #[serde(skip)]
    pub highlighted: Vec<String>,
    /// Preferences that apply to every plot: p-values below ten to this
    /// power are shown as a bound; the colour of axes; and the colours of
    /// the two samples of a two-sample analysis.
    pub min_p_exponent: i32,
    pub axes_color: Color,
    pub sample_colors: [Color; 2],
}

impl Default for PlotSettings {
    fn default() -> Self {
        // Every plot that has a legend starts with one, placed where it
        // covers least.
        let legend = LegendPosition::Best;
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
            profile_bar: sample_plots::ProfileBarOptions {
                legend,
                ..Default::default()
            },
            profile_bar_only_active: true,
            scatter: scatter::Options {
                height: 6.0,
                ..Default::default()
            },
            group_spread: scatter::Spread::Percentiles2And98,
            sequence_histogram: histogram::SequenceOptions {
                legend,
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
                legend,
                height: 6.0,
                ..Default::default()
            },
            rarefaction: rarefaction::Options::default(),
            volcano: volcano::Options::default(),
            correlation: correlation_plot::Options::default(),
            composition: composition::Options::default(),
            community_normalisation: Normalisation::Proportions,
            community_distance: Distance::BrayCurtis,
            community_permutations: 999,
            alpha_metric: AlphaMetric::Shannon,
            alpha_box: feature_plots::BoxOptions {
                height: 6.0,
                ..Default::default()
            },
            pcoa: pca_plot::Options {
                legend,
                ..Default::default()
            },
            pcoa_show_tests: true,
            distance_box: feature_plots::BoxOptions {
                height: 6.0,
                ..Default::default()
            },
            nmds: pca_plot::Options {
                legend,
                show_pc3_vs_pc2: false,
                show_pc1_vs_pc3: false,
                ..Default::default()
            },
            mantel: correlation_plot::Options {
                unique_shapes: false,
                legend: LegendPosition::None,
                ..Default::default()
            },
            shared: shared_features::Options::default(),
            rank_abundance: rank_abundance::Options::default(),
            ancom: ancom_plot::Options::default(),
            ancom_test: MultiGroupTest::Anova,
            ancom_alpha: 0.05,
            heatmap_only_active: false,
            heatmap_sequence_counts: false,
            highlighted: Vec::new(),
            min_p_exponent: -15,
            axes_color: AXES_COLOR,
            sample_colors: [SAMPLE1_COLOR, SAMPLE2_COLOR],
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
            PlotKind::Rarefaction => self.rarefaction = defaults.rarefaction,
            PlotKind::Volcano => self.volcano = defaults.volcano,
            PlotKind::CorrelationScatter => self.correlation = defaults.correlation,
            PlotKind::Composition => self.composition = defaults.composition,
            PlotKind::SharedFeatures => self.shared = defaults.shared,
            PlotKind::RankAbundance => self.rank_abundance = defaults.rank_abundance,
            PlotKind::Ancom => {
                self.ancom = defaults.ancom;
                self.ancom_test = defaults.ancom_test;
                self.ancom_alpha = defaults.ancom_alpha;
            }
            PlotKind::AlphaDiversity
            | PlotKind::Pcoa
            | PlotKind::Nmds
            | PlotKind::DistanceBox
            | PlotKind::Mantel => {
                match kind {
                    PlotKind::Nmds => self.nmds = defaults.nmds,
                    PlotKind::Mantel => self.mantel = defaults.mantel,
                    PlotKind::AlphaDiversity => {
                        self.alpha_metric = defaults.alpha_metric;
                        self.alpha_box = defaults.alpha_box;
                    }
                    PlotKind::Pcoa => {
                        self.pcoa = defaults.pcoa;
                        self.pcoa_show_tests = defaults.pcoa_show_tests;
                    }
                    _ => self.distance_box = defaults.distance_box,
                }
                // The plots of whole communities share these.
                self.community_normalisation = defaults.community_normalisation;
                self.community_distance = defaults.community_distance;
                self.community_permutations = defaults.community_permutations;
            }
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
    let mut figure = build_plain(session, mode, kind, settings);
    // The colours chosen in the preferences replace the built-in ones: the
    // grey of axes wherever it draws lines or text, and, for two samples,
    // the colours the plots give the first and second sample.
    let samples = settings.sample_colors;
    let recolor_samples = mode == Mode::TwoSamples && samples != [SAMPLE1_COLOR, SAMPLE2_COLOR];
    if recolor_samples || settings.axes_color != AXES_COLOR {
        figure.map_colors(|color, fill| {
            if !fill && color == AXES_COLOR {
                settings.axes_color
            } else if recolor_samples && color == SAMPLE1_COLOR {
                samples[0]
            } else if recolor_samples && color == SAMPLE2_COLOR {
                samples[1]
            } else {
                color
            }
        });
    }
    figure
}

/// One of the plots of whole communities, for some samples and their groups.
fn community_plot(
    session: &Session,
    kind: PlotKind,
    samples: &[String],
    sample_groups: Vec<usize>,
    groups: Vec<String>,
    colors: Vec<Color>,
    settings: &PlotSettings,
) -> Figure {
    let Some(tree) = &session.tree else {
        return Figure::message(NO_RESULTS);
    };
    // The same seed as the tests, so that the same samples are drawn and
    // the same permutations made every time.
    let mut rng = rng_from_seed(session.seed);
    let community = Community::from_tree(
        tree,
        samples,
        &session.profile_level,
        sample_groups,
        groups,
        settings.community_normalisation,
        &mut rng,
    );
    let mut community = match community {
        Ok(community) => community,
        Err(message) => return Figure::message(&message),
    };
    community.group_colors = colors;
    let distance = settings.community_distance;
    match kind {
        PlotKind::AlphaDiversity => {
            community_plots::alpha_plot(&community, settings.alpha_metric, &settings.alpha_box)
        }
        PlotKind::Pcoa => community_plots::pcoa_plot(
            &community,
            distance,
            settings
                .pcoa_show_tests
                .then_some((settings.community_permutations, &mut rng)),
            &settings.pcoa,
        ),
        PlotKind::Nmds => community_plots::nmds_plot(&community, distance, &settings.nmds),
        _ => community_plots::distance_box_plot(&community, distance, &settings.distance_box),
    }
}

/// The plots drawn from the proportions of every feature in some samples
/// and their groups: shared features and rank-abundance curves.
fn proportion_plot(kind: PlotKind, data: &composition::Data, settings: &PlotSettings) -> Figure {
    match kind {
        PlotKind::SharedFeatures => {
            let shared = shared_features::Data::from_composition(
                data,
                settings.shared.min_prevalence / 100.0,
            );
            shared_features::plot(&shared, &settings.shared)
        }
        _ => rank_abundance::plot(data, &settings.rank_abundance),
    }
}

/// The ANCOM plot of a profile whose sample `i` is in group
/// `sample_groups[i]`.
fn ancom_figure(
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    groups: &[String],
    settings: &PlotSettings,
) -> Figure {
    let features = profile.features();
    let samples: Vec<Vec<f64>> = (0..profile.sample_names.len())
        .map(|s| features.iter().map(|f| f.counts[s]).collect())
        .collect();
    match ancom(&samples, sample_groups, settings.ancom_test, settings.ancom_alpha) {
        Some(result) => {
            let names: Vec<String> = features.iter().map(|f| f.name.clone()).collect();
            let options = ancom_plot::Options {
                highlighted: settings.highlighted.clone(),
                ..settings.ancom.clone()
            };
            ancom_plot::plot(&result, &names, groups, &options)
        }
        None => Figure::message(
            "ANCOM needs two groups with samples, a group of more than one sample, and two features",
        ),
    }
}

/// The rarefaction curves of some samples, coloured by group.
fn rarefaction_curves(
    session: &Session,
    samples: &[String],
    sample_groups: Vec<usize>,
    groups: Vec<String>,
    colors: Vec<Color>,
    settings: &PlotSettings,
) -> Figure {
    let Some(tree) = &session.tree else {
        return Figure::message(NO_RESULTS);
    };
    match rarefaction::Data::from_tree(tree, samples, &session.profile_level, sample_groups, groups)
    {
        Ok(mut data) => {
            data.group_colors = colors;
            rarefaction::plot(&data, &settings.rarefaction)
        }
        Err(message) => Figure::message(&message),
    }
}

fn build_plain(session: &Session, mode: Mode, kind: PlotKind, settings: &PlotSettings) -> Figure {
    if session.tree.is_none() {
        return Figure::message(NO_RESULTS);
    }
    let highlighted = || settings.highlighted.clone();
    let error_bar = extended_error_bar::Options {
        min_p_value_exponent: settings.min_p_exponent,
        highlighted: highlighted(),
        ..settings.error_bar.clone()
    };
    let scatter_options = scatter::Options {
        highlighted: highlighted(),
        ..settings.scatter.clone()
    };
    let p_text = |p: f64| format_p_value(p, settings.min_p_exponent);
    let volcano_options = volcano::Options {
        highlighted: highlighted(),
        min_p_value_exponent: settings.min_p_exponent,
        ..settings.volcano.clone()
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
                PlotKind::Scatter => {
                    let mut data = scatter::Data::from_sample_profile(profile);
                    data.add_p_values(|feature| {
                        let row = results.rows.iter().find(|r| r.feature == feature)?;
                        Some((p_text(row.p_value), p_text(row.p_value_corrected)))
                    });
                    scatter::plot(&data, &scatter_options)
                }
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
                PlotKind::Rarefaction => {
                    // Each sample is its own "group"; their colours are the
                    // two sample colours.
                    let names = profile.profile.sample_names.clone();
                    // With two samples there are only the samples to draw.
                    let mut settings = settings.clone();
                    settings.rarefaction.curves = rarefaction::Curves::Samples;
                    rarefaction_curves(
                        session,
                        &names,
                        vec![0, 1],
                        names.clone(),
                        vec![SAMPLE1_COLOR, SAMPLE2_COLOR],
                        &settings,
                    )
                }
                PlotKind::Volcano => volcano::plot(
                    &volcano::Data::from_sample_results(profile, results),
                    &volcano_options,
                ),
                PlotKind::Composition | PlotKind::SharedFeatures | PlotKind::RankAbundance => {
                    let mut data = composition::Data::from_profile(
                        &profile.profile,
                        vec![0, 1],
                        profile.profile.sample_names.clone(),
                    );
                    data.group_colors = vec![SAMPLE1_COLOR, SAMPLE2_COLOR];
                    if matches!(kind, PlotKind::SharedFeatures | PlotKind::RankAbundance) {
                        return proportion_plot(kind, &data, settings);
                    }
                    // With two samples there are only the samples to draw.
                    composition::plot(
                        &data,
                        &composition::Options {
                            bars: composition::Bars::Samples,
                            ..settings.composition.clone()
                        },
                    )
                }
                _ => Figure::message("This plot is not available for two samples"),
            }
        }
        Mode::TwoGroups => {
            if session.metadata.is_none() {
                return Figure::message(NO_METADATA);
            }
            let Some((profile, results)) = &session.groups.output else {
                return Figure::message(
                    session
                        .error
                        .as_deref()
                        .unwrap_or("Running the two-group analysis..."),
                );
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
                    .map(|r| format_p_value(r.p_value_corrected, settings.min_p_exponent));
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
                PlotKind::Scatter => {
                    let mut data = scatter::Data {
                        x_color: color1,
                        y_color: color2,
                        ..scatter::Data::from_group_profile(profile, settings.group_spread)
                    };
                    data.add_p_values(|feature| {
                        let row = results.rows.iter().find(|r| r.feature == feature)?;
                        Some((p_text(row.p_value), p_text(row.p_value_corrected)))
                    });
                    scatter::plot(&data, &scatter_options)
                }
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
                PlotKind::Rarefaction => {
                    let first = profile.samples_in_group1().len();
                    let samples = &profile.profile.sample_names;
                    rarefaction_curves(
                        session,
                        samples,
                        (0..samples.len())
                            .map(|i| usize::from(i >= first))
                            .collect(),
                        vec![profile.group_name1.clone(), profile.group_name2.clone()],
                        vec![color1, color2],
                        settings,
                    )
                }
                PlotKind::Volcano => {
                    let mut data = volcano::Data::from_group_results(profile, results);
                    data.sides = Some([
                        (profile.group_name1.clone(), color1),
                        (profile.group_name2.clone(), color2),
                    ]);
                    volcano::plot(&data, &volcano_options)
                }
                PlotKind::Composition | PlotKind::SharedFeatures | PlotKind::RankAbundance => {
                    let first = profile.samples_in_group1().len();
                    let samples = profile.profile.sample_names.len();
                    let mut data = composition::Data::from_profile(
                        &profile.profile,
                        (0..samples).map(|i| usize::from(i >= first)).collect(),
                        vec![profile.group_name1.clone(), profile.group_name2.clone()],
                    );
                    data.group_colors = vec![color1, color2];
                    if kind == PlotKind::Composition {
                        composition::plot(&data, &settings.composition)
                    } else {
                        proportion_plot(kind, &data, settings)
                    }
                }
                PlotKind::Ancom => {
                    let first = profile.samples_in_group1().len();
                    let groups: Vec<usize> = (0..profile.profile.sample_names.len())
                        .map(|i| usize::from(i >= first))
                        .collect();
                    ancom_figure(
                        &profile.profile,
                        &groups,
                        &[profile.group_name1.clone(), profile.group_name2.clone()],
                        settings,
                    )
                }
                PlotKind::AlphaDiversity
                | PlotKind::Pcoa
                | PlotKind::Nmds
                | PlotKind::DistanceBox => {
                    let first = profile.samples_in_group1().len();
                    let samples = &profile.profile.sample_names;
                    community_plot(
                        session,
                        kind,
                        samples,
                        (0..samples.len())
                            .map(|i| usize::from(i >= first))
                            .collect(),
                        vec![profile.group_name1.clone(), profile.group_name2.clone()],
                        vec![color1, color2],
                        settings,
                    )
                }
                _ => Figure::message("This plot is not available for two groups"),
            }
        }
        Mode::Correlation => {
            if session.metadata.is_none() {
                return Figure::message(NO_METADATA);
            }
            if session.correlation.field.is_none() {
                return Figure::message(
                    "No metadata field holds a number for at least three samples",
                );
            }
            let Some((profile, results)) = &session.correlation.output else {
                return Figure::message(
                    session
                        .error
                        .as_deref()
                        .unwrap_or("Running the correlation analysis..."),
                );
            };
            match kind {
                PlotKind::CorrelationScatter => {
                    let Some(mut data) = feature
                        .and_then(|name| correlation_plot::Data::from_profile(profile, name))
                    else {
                        return Figure::message(NO_FEATURE);
                    };
                    // Coloured by the groups of the group field, if any.
                    if let Some(groups) = session.grouping() {
                        let names: Vec<String> = groups.names().map(str::to_string).collect();
                        for point in &mut data.points {
                            point.group = names.iter().position(|g| {
                                groups
                                    .samples(g)
                                    .is_some_and(|members| members.contains(&point.sample))
                            });
                        }
                        data.group_colors = names.iter().map(|g| session.group_color(g)).collect();
                        data.groups = names;
                    }
                    data.statistics =
                        results
                            .rows
                            .iter()
                            .find(|r| r.feature == data.feature)
                            .map(|r| {
                                format!(
                                    "{} = {:.3}, p = {} (n = {})",
                                    results.analysis.method.coefficient_name(),
                                    r.coefficient,
                                    p_text(r.p_value_corrected),
                                    profile.values.len()
                                )
                            });
                    correlation_plot::plot(&data, &settings.correlation)
                }
                PlotKind::Volcano => volcano::plot(
                    &volcano::Data::from_correlation_results(results, &profile.field),
                    &volcano_options,
                ),
                PlotKind::PValueHistogram => {
                    let values: Vec<f64> = if settings.p_value_histogram_corrected {
                        results.rows.iter().map(|r| r.p_value_corrected).collect()
                    } else {
                        results.rows.iter().map(|r| r.p_value).collect()
                    };
                    histogram::p_value_histogram(&values, &settings.p_value_histogram)
                }
                PlotKind::Mantel => {
                    let Some(tree) = &session.tree else {
                        return Figure::message(NO_RESULTS);
                    };
                    let samples = &profile.profile.sample_names;
                    let mut rng = rng_from_seed(session.seed);
                    match Community::from_tree(
                        tree,
                        samples,
                        &session.profile_level,
                        vec![0; samples.len()],
                        vec![String::new()],
                        settings.community_normalisation,
                        &mut rng,
                    ) {
                        Ok(community) => community_plots::mantel_plot(
                            &community,
                            &profile.values,
                            &profile.field,
                            settings.community_distance,
                            results.analysis.method,
                            settings.community_permutations,
                            &mut rng,
                            &settings.mantel,
                        ),
                        Err(message) => Figure::message(&message),
                    }
                }
                _ => Figure::message("This plot is not available for correlations"),
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
                    .map(|r| format_p_value(r.p_value_corrected, settings.min_p_exponent));
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
                PlotKind::Rarefaction => rarefaction_curves(
                    session,
                    &profile.profile.sample_names,
                    profile
                        .samples_in_groups()
                        .iter()
                        .enumerate()
                        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                        .collect(),
                    profile.group_names.clone(),
                    colors.clone(),
                    settings,
                ),
                PlotKind::Volcano => volcano::plot(
                    &volcano::Data::from_multi_group_results(results),
                    &volcano_options,
                ),
                PlotKind::Composition | PlotKind::SharedFeatures | PlotKind::RankAbundance => {
                    let mut data = composition::Data::from_profile(
                        &profile.profile,
                        profile
                            .samples_in_groups()
                            .iter()
                            .enumerate()
                            .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                            .collect(),
                        profile.group_names.clone(),
                    );
                    data.group_colors = colors.clone();
                    if kind == PlotKind::Composition {
                        composition::plot(&data, &settings.composition)
                    } else {
                        proportion_plot(kind, &data, settings)
                    }
                }
                PlotKind::Ancom => {
                    let groups: Vec<usize> = profile
                        .samples_in_groups()
                        .iter()
                        .enumerate()
                        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                        .collect();
                    ancom_figure(&profile.profile, &groups, &profile.group_names, settings)
                }
                PlotKind::AlphaDiversity
                | PlotKind::Pcoa
                | PlotKind::Nmds
                | PlotKind::DistanceBox => community_plot(
                    session,
                    kind,
                    &profile.profile.sample_names,
                    profile
                        .samples_in_groups()
                        .iter()
                        .enumerate()
                        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                        .collect(),
                    profile.group_names.clone(),
                    colors.clone(),
                    settings,
                ),
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
        let meta = "Id\tGroup\tDepth\nA1\tA\t1\nA2\tA\t2\nA3\tA\t4\nB1\tB\t3\nB2\tB\t6\n\
            B3\tB\t5\nC1\tC\t9\nC2\tC\t7\nC3\tC\t8\n";
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
