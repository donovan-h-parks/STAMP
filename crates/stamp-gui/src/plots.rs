//! Choosing a plot and building its figure from the session's results.

use stamp_core::stats::diversity::{AlphaMetric, Distance, DistanceMatrix, Normalisation};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::results::format_p_value;
use stamp_core::stats::rng_from_seed;
use stamp_plot::axes::LegendPosition;
use stamp_plot::axes::AXES_COLOR;
use stamp_plot::community_plots::{self, Community};
use stamp_plot::extended_error_bar::{self, SAMPLE1_COLOR, SAMPLE2_COLOR};
use stamp_plot::p_values::{p_value_text, PValueKind};
use stamp_plot::{
    ancom_plot, co_occurrence_plot, composition, correlation_plot, feature_plots, group_heatmap,
    heatmap, histogram, map_plot, multi_box, ordination_plot, post_hoc, prevalence_plot,
    rank_abundance, rarefaction, sample_plots, scatter, shared_features, volcano, Color, Figure,
};

use crate::session::{Mode, Session};

/// The slow parts of plots already computed (the layouts of the ordination
/// plots, PCoA with its community tests, NMDS, the distances and Mantel
/// test, ANCOM), by what they were computed from, so that changing how a
/// plot looks (colours, shapes, sizes, fonts, the legend) draws it again
/// without computing it again.
///
/// A plot whose slow part is not kept asks for it to be computed
/// ([`take_request`](Self::take_request)) and is drawn as a placeholder; the
/// window computes it in a job (on another thread on the desktop, in time
/// slices in the browser), showing its progress, keeps the outcome here
/// and draws the plot again.
#[derive(Default)]
pub struct OrdinationCache {
    /// Most recently used last.
    entries: std::sync::Mutex<Vec<(OrdinationKey, Cached)>>,
    computed: std::sync::atomic::AtomicUsize,
    /// The slow part the last plot built needs, not yet kept.
    request: std::sync::Mutex<Option<PlotRequest>>,
    /// Compute what is needed at once, while the plot is built, instead of
    /// asking for it: for tests and scripts that build figures directly.
    synchronous: std::sync::atomic::AtomicBool,
}

/// What an ordination layout is computed from.
#[derive(Clone)]
struct OrdinationKey {
    /// The profiles the samples come from: the layout belongs to this
    /// data, not to another loaded later (held, so that it cannot be
    /// mistaken for one that takes its place in memory).
    tree: Option<std::sync::Arc<stamp_core::ProfileTree>>,
    /// Everything else: the plot, its inputs and settings.
    inputs: String,
}

impl PartialEq for OrdinationKey {
    fn eq(&self, other: &Self) -> bool {
        let same_tree = match (&self.tree, &other.tree) {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        same_tree && self.inputs == other.inputs
    }
}

/// The slow part of a plot.
#[derive(Clone)]
pub enum Cached {
    Community(Result<community_plots::Layout, String>),
    /// Distances between samples, and the depth they were rarefied to.
    Distances(std::sync::Arc<DistanceMatrix>, Option<u64>),
    Mantel {
        matrix: std::sync::Arc<DistanceMatrix>,
        differences: std::sync::Arc<DistanceMatrix>,
        test: stamp_core::stats::community::PermutationTest,
        depth: Option<u64>,
    },
    Ancom(Option<std::sync::Arc<stamp_core::stats::ancom::AncomResult>>),
    /// Nothing could be computed, for the reason given.
    Unavailable(String),
}

/// Work computing the slow part of a plot, in steps as [`Job::spawn_steps`]
/// takes them: `None` while there is more to do, `Some(None)` once
/// cancelled.
///
/// [`Job::spawn_steps`]: crate::jobs::Job::spawn_steps
type Work =
    Box<dyn FnMut(&crate::jobs::Progress, &dyn Fn() -> bool) -> Option<Option<Cached>> + Send>;

/// The slow part of a plot, to be computed.
pub struct PlotRequest {
    key: OrdinationKey,
    work: Work,
}

/// The slow part of a plot being computed.
pub struct RunningPlot {
    key: OrdinationKey,
    job: crate::jobs::Job<Option<Cached>>,
}

/// How a [`RunningPlot`] is getting on.
pub enum PlotPoll {
    Running,
    /// Finished, and kept in the cache.
    Finished,
    Cancelled,
    Failed,
}

impl PlotRequest {
    /// Start computing.
    pub fn start(self) -> RunningPlot {
        RunningPlot {
            key: self.key,
            job: crate::jobs::Job::spawn_steps(self.work),
        }
    }

    /// Whether this asks for what `running` computes.
    pub fn is_running_in(&self, running: &RunningPlot) -> bool {
        self.key == running.key
    }

    /// A text that tells requests apart.
    pub fn id(&self) -> String {
        self.key.inputs.clone()
    }
}

impl RunningPlot {
    /// A text that tells calculations apart, as [`PlotRequest::id`].
    pub fn id(&self) -> String {
        self.key.inputs.clone()
    }

    pub fn progress(&self) -> &crate::jobs::Progress {
        self.job.progress()
    }

    pub fn cancel(&self) {
        self.job.cancel();
    }

    /// Collect the outcome, if there is one, into `cache`.
    pub fn poll(&self, cache: &OrdinationCache) -> PlotPoll {
        match self.job.poll() {
            Ok(Some(value)) => {
                cache.insert(self.key.clone(), value);
                PlotPoll::Finished
            }
            Ok(None) => PlotPoll::Cancelled,
            Err(true) => PlotPoll::Running,
            Err(false) => PlotPoll::Failed,
        }
    }
}

/// Layouts kept.
const ORDINATION_CACHE_SIZE: usize = 6;

/// The text of the placeholder drawn while the slow part of a plot is
/// computed.
pub const COMPUTING: &str = "Computing the plot...";

/// How a run inside a [`Work`] paces itself: it reports what it is doing,
/// stops once cancelled, pauses when its time is up, and pauses too
/// whenever it starts something new, so that in the browser the page says
/// so before a piece of work that cannot be divided.
fn stage_pace<'a>(
    progress: &'a crate::jobs::Progress,
    out_of_time: &'a dyn Fn() -> bool,
    last: &'a mut String,
) -> impl FnMut(&str, usize, usize) -> stamp_core::stats::results::Control + 'a {
    use stamp_core::stats::results::Control;
    move |stage, done, total| {
        if !progress.report_stage(stage, done, total) {
            Control::Cancel
        } else if last != stage {
            last.clear();
            last.push_str(stage);
            Control::Pause
        } else if out_of_time() {
            Control::Pause
        } else {
            Control::Continue
        }
    }
}

impl OrdinationCache {
    /// The slow part for `key`, if kept. Otherwise it is computed now by
    /// `work`, if the cache is synchronous, or else asked for (see
    /// [`take_request`](Self::take_request)) and `None` returned.
    fn get_or_request(&self, key: OrdinationKey, work: impl FnOnce() -> Work) -> Option<Cached> {
        if let Some(value) = self.get(&key) {
            return Some(value);
        }
        let mut work = work();
        if self.synchronous.load(std::sync::atomic::Ordering::Relaxed) {
            let progress = crate::jobs::Progress::default();
            let value = loop {
                if let Some(value) = work(&progress, &|| false) {
                    break value?;
                }
            };
            self.insert(key, value.clone());
            return Some(value);
        }
        *self.request.lock().unwrap_or_else(|e| e.into_inner()) = Some(PlotRequest { key, work });
        None
    }

    /// The slow part for `key`, if kept, which makes it the most recently
    /// used.
    fn get(&self, key: &OrdinationKey) -> Option<Cached> {
        let mut entries = self.entries();
        let index = entries.iter().position(|(k, _)| k == key)?;
        let entry = entries.remove(index);
        let value = entry.1.clone();
        entries.push(entry);
        Some(value)
    }

    fn insert(&self, key: OrdinationKey, value: Cached) {
        self.computed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut entries = self.entries();
        entries.retain(|(k, _)| *k != key);
        if entries.len() >= ORDINATION_CACHE_SIZE {
            entries.remove(0);
        }
        entries.push((key, value));
    }

    fn entries(&self) -> std::sync::MutexGuard<'_, Vec<(OrdinationKey, Cached)>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The number of layouts computed so far, not counting those reused.
    pub fn computed(&self) -> usize {
        self.computed.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// What the plot built last needs computed, if anything; asked for
    /// once.
    pub fn take_request(&self) -> Option<PlotRequest> {
        self.request
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Compute slow parts while plots are built rather than asking for them.
    pub fn set_synchronous(&self, synchronous: bool) {
        self.synchronous
            .store(synchronous, std::sync::atomic::Ordering::Relaxed);
    }
}

/// A key from the hash of some numbers.
fn hash_of_values<'a>(values: impl Iterator<Item = &'a [f64]>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for row in values {
        row.len().hash(&mut hasher);
        for value in row {
            value.to_bits().hash(&mut hasher);
        }
    }
    hasher.finish()
}

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
    GroupHeatmap,
    CoOccurrence,
    PrevalenceAbundance,
    Map,
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
            PlotKind::GroupHeatmap => "Group heatmap",
            PlotKind::CoOccurrence => "Co-occurrence heatmap",
            PlotKind::PrevalenceAbundance => "Prevalence-abundance plot",
            PlotKind::Map => "Map of sample locations",
        }
    }

    /// The group the plot is listed under, by the question it answers.
    pub fn group(self) -> PlotGroup {
        use PlotKind::*;
        match self {
            ExtendedErrorBar | Volcano | Ancom | PValueHistogram | MultipleComparison => {
                PlotGroup::Differences
            }
            Bar | Box | PostHoc | CorrelationScatter => PlotGroup::OneFeature,
            Composition | ProfileBar | Heatmap | GroupHeatmap | CoOccurrence
            | PrevalenceAbundance | Scatter | SharedFeatures | SequenceHistogram => {
                PlotGroup::Composition
            }
            Rarefaction | RankAbundance | AlphaDiversity => PlotGroup::Diversity,
            Pcoa | Nmds | DistanceBox | Mantel => PlotGroup::Similarity,
            Map => PlotGroup::Samples,
        }
    }

    /// True for the plots that check the tests themselves rather than show
    /// their results; they are listed last in their group, after a divider.
    pub fn checks_the_tests(self) -> bool {
        matches!(
            self,
            PlotKind::PValueHistogram | PlotKind::MultipleComparison
        )
    }

    /// The plots offered in a mode, by group in the order of
    /// [`PlotGroup::ALL`]; within a group they keep the order of
    /// [`PlotKind::available`], with the checks on the tests last.
    pub fn grouped(mode: Mode) -> Vec<(PlotGroup, Vec<PlotKind>)> {
        PlotGroup::ALL
            .into_iter()
            .map(|group| {
                let mut kinds: Vec<PlotKind> = Self::available(mode)
                    .iter()
                    .copied()
                    .filter(|kind| kind.group() == group)
                    .collect();
                kinds.sort_by_key(|kind| kind.checks_the_tests());
                (group, kinds)
            })
            .filter(|(_, kinds)| !kinds.is_empty())
            .collect()
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
                PlotKind::Heatmap,
                PlotKind::GroupHeatmap,
                PlotKind::CoOccurrence,
                PlotKind::PrevalenceAbundance,
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
                PlotKind::Map,
            ],
            Mode::Correlation => &[
                PlotKind::CorrelationScatter,
                PlotKind::Volcano,
                PlotKind::PValueHistogram,
                PlotKind::Mantel,
                PlotKind::CoOccurrence,
                PlotKind::Map,
            ],
            Mode::MultipleGroups => &[
                PlotKind::Pcoa,
                PlotKind::Heatmap,
                PlotKind::GroupHeatmap,
                PlotKind::CoOccurrence,
                PlotKind::PrevalenceAbundance,
                PlotKind::Bar,
                PlotKind::Box,
                PlotKind::PostHoc,
                PlotKind::Rarefaction,
                PlotKind::Volcano,
                PlotKind::Composition,
                PlotKind::AlphaDiversity,
                PlotKind::Nmds,
                PlotKind::DistanceBox,
                PlotKind::Ancom,
                PlotKind::SharedFeatures,
                PlotKind::RankAbundance,
                PlotKind::Map,
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

/// Groups of plots, by the question they answer, as the plot selector lists
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotGroup {
    /// Which features differ, and how much?
    Differences,
    /// What does the selected feature look like?
    OneFeature,
    /// What are the samples made of?
    Composition,
    /// How rich and even are the samples?
    Diversity,
    /// How alike are the samples, and does it follow the groups or a
    /// variable?
    Similarity,
    /// Where were the samples taken?
    Samples,
}

impl PlotGroup {
    pub const ALL: [PlotGroup; 6] = [
        PlotGroup::Differences,
        PlotGroup::OneFeature,
        PlotGroup::Composition,
        PlotGroup::Diversity,
        PlotGroup::Similarity,
        PlotGroup::Samples,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PlotGroup::Differences => "Differences between features",
            PlotGroup::OneFeature => "One feature",
            PlotGroup::Composition => "Composition",
            PlotGroup::Diversity => "Diversity",
            PlotGroup::Similarity => "Similarity between samples",
            PlotGroup::Samples => "Where the samples are from",
        }
    }
}

/// Which features the box plot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BoxFeatures {
    /// The feature selected in the feature table.
    Selected,
    /// The most significant features that pass the filters.
    MostSignificant,
    /// The features typed in the settings (or Ctrl+clicked in the feature
    /// table).
    Listed,
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
    /// Which features the box plot shows: the selected one, the most
    /// significant `box_feature_count`, or those listed in
    /// `box_feature_list` (one per line); and the metadata field whose
    /// categories may order its groups.
    pub box_features: BoxFeatures,
    pub box_feature_count: usize,
    pub box_feature_list: String,
    pub box_category_field: Option<String>,
    /// What the group bar and box plots show for each sample.
    pub feature_field: feature_plots::Field,
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
    pub pcoa: ordination_plot::Options,
    /// Write the outcome of PERMANOVA, ANOSIM and PERMDISP above the PCoA
    /// plot.
    pub pcoa_show_tests: bool,
    pub distance_box: feature_plots::BoxOptions,
    pub nmds: ordination_plot::Options,
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
    /// The heatmap of groups: its appearance, what it summarises, whether
    /// only the features that pass the filters are drawn, and the metadata
    /// field whose categories label (and may order) the groups.
    pub group_heatmap: group_heatmap::Options,
    pub group_heatmap_summary: group_heatmap::Summary,
    pub group_heatmap_only_active: bool,
    pub group_heatmap_category_field: Option<String>,
    /// The co-occurrence heatmap: what is correlated, its appearance, the
    /// features always included (one per line, as typed), and the group to
    /// correlate within (`None`: every sample of the analysis).
    pub co_occurrence: co_occurrence_plot::Settings,
    pub co_occurrence_options: co_occurrence_plot::Options,
    pub co_occurrence_features: String,
    pub co_occurrence_group: Option<String>,
    /// The prevalence-abundance plot: its appearance and view, the
    /// smallest proportion (%) at which a feature counts as present,
    /// whether only active features are drawn, whether the selected
    /// feature is highlighted, and other features to highlight (one per
    /// line).
    pub prevalence: prevalence_plot::Options,
    pub prevalence_min_abundance: f64,
    pub prevalence_only_active: bool,
    pub prevalence_highlight_selected: bool,
    pub prevalence_highlight: String,
    /// The map of sample locations, and the metadata fields holding the
    /// latitudes and longitudes (`None`: a field named like one).
    pub map: map_plot::Options,
    pub map_lat_field: Option<String>,
    pub map_lon_field: Option<String>,
    /// Features drawn in red by the plots that support highlighting.
    #[serde(skip)]
    pub highlighted: Vec<String>,
    /// Preferences that apply to every plot: p-values below ten to this
    /// power are shown as a bound; the colour of axes; and the colours of
    /// the two samples of a two-sample analysis.
    pub min_p_exponent: i32,
    pub axes_color: Color,
    pub sample_colors: [Color; 2],
    /// Font sizes of the tick labels, axis labels and legends of every
    /// plot, set in the preferences.
    pub fonts: stamp_plot::axes::Fonts,
}

impl PlotSettings {
    /// True if the plot `kind` colours samples by the abundance of the
    /// selected feature, so that it changes with the selection.
    pub fn colors_by_selected_feature(&self, kind: PlotKind) -> bool {
        use stamp_plot::ordination_style::ColorSource;
        let options = match kind {
            PlotKind::Pcoa => &self.pcoa,
            PlotKind::Nmds => &self.nmds,
            PlotKind::Map => {
                return self.map.color_by == ColorSource::SelectedFeature
                    || self.map.size_by == map_plot::SizeSource::SelectedFeature
            }
            _ => return false,
        };
        options.color_by == stamp_plot::ordination_style::ColorSource::SelectedFeature
    }
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
            box_features: BoxFeatures::Selected,
            box_feature_count: 4,
            box_feature_list: String::new(),
            box_category_field: None,
            feature_field: feature_plots::Field::Proportion,
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
            pcoa: ordination_plot::Options {
                legend,
                ..Default::default()
            },
            pcoa_show_tests: true,
            distance_box: feature_plots::BoxOptions {
                height: 6.0,
                ..Default::default()
            },
            nmds: ordination_plot::Options {
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
            group_heatmap: group_heatmap::Options::default(),
            group_heatmap_summary: group_heatmap::Summary::default(),
            group_heatmap_only_active: false,
            group_heatmap_category_field: None,
            co_occurrence: co_occurrence_plot::Settings::default(),
            co_occurrence_options: co_occurrence_plot::Options::default(),
            co_occurrence_features: String::new(),
            co_occurrence_group: None,
            prevalence: prevalence_plot::Options::default(),
            prevalence_min_abundance: 0.0,
            prevalence_only_active: false,
            prevalence_highlight_selected: true,
            prevalence_highlight: String::new(),
            map: map_plot::Options::default(),
            map_lat_field: None,
            map_lon_field: None,
            highlighted: Vec::new(),
            min_p_exponent: -15,
            axes_color: AXES_COLOR,
            sample_colors: [SAMPLE1_COLOR, SAMPLE2_COLOR],
            fonts: stamp_plot::axes::Fonts::default(),
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
                self.box_features = defaults.box_features;
                self.box_feature_count = defaults.box_feature_count;
                self.box_feature_list = defaults.box_feature_list;
                self.box_category_field = defaults.box_category_field;
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
            PlotKind::GroupHeatmap => {
                self.group_heatmap = defaults.group_heatmap;
                self.group_heatmap_summary = defaults.group_heatmap_summary;
                self.group_heatmap_only_active = defaults.group_heatmap_only_active;
                self.group_heatmap_category_field = defaults.group_heatmap_category_field;
            }
            PlotKind::PrevalenceAbundance => {
                self.prevalence = defaults.prevalence;
                self.prevalence_min_abundance = defaults.prevalence_min_abundance;
                self.prevalence_only_active = defaults.prevalence_only_active;
                self.prevalence_highlight_selected = defaults.prevalence_highlight_selected;
                self.prevalence_highlight = defaults.prevalence_highlight;
            }
            PlotKind::Map => {
                self.map = defaults.map;
                self.map_lat_field = defaults.map_lat_field;
                self.map_lon_field = defaults.map_lon_field;
            }
            PlotKind::CoOccurrence => {
                self.co_occurrence = defaults.co_occurrence;
                self.co_occurrence_options = defaults.co_occurrence_options;
                self.co_occurrence_features = defaults.co_occurrence_features;
                self.co_occurrence_group = defaults.co_occurrence_group;
            }
        }
    }
}

/// The group heatmap of a profile whose sample `i` is in group
/// `sample_groups[i]`; `active` are the features that pass the filters.
fn group_heatmap_figure(
    session: &Session,
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    groups: &[String],
    active: &[String],
    settings: &PlotSettings,
) -> Figure {
    let mut data = group_heatmap::Data::from_profile(
        profile,
        sample_groups,
        groups,
        settings.group_heatmap_only_active.then_some(active),
        &settings.group_heatmap_summary,
    );
    let mut options = settings.group_heatmap.clone();
    let field = settings
        .group_heatmap_category_field
        .as_ref()
        .and_then(|field| {
            let metadata = session.metadata.as_ref()?;
            metadata
                .fields()
                .iter()
                .any(|f| f == field)
                .then_some((metadata, field))
        });
    match field {
        Some((metadata, field)) => {
            let members = |group: &str| -> Vec<String> {
                profile
                    .sample_names
                    .iter()
                    .zip(sample_groups)
                    .filter(|(_, &g)| groups.get(g).is_some_and(|name| name == group))
                    .map(|(sample, _)| sample.clone())
                    .collect()
            };
            data.set_categories(metadata, field, &members);
        }
        None => {
            // Without a field there is nothing to order the groups by.
            if options.column_order == group_heatmap::ColumnOrder::Category {
                options.column_order = group_heatmap::ColumnOrder::Alphabetical;
            }
        }
    }
    group_heatmap::plot(&data, &options)
}

/// The features typed in the co-occurrence settings, one per line.
pub fn typed_features(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// The co-occurrence heatmap (or the correlations of the selected feature)
/// of a profile, within one group if the settings name one of `groups`.
fn co_occurrence_figure(
    session: &Session,
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    groups: &[String],
    settings: &PlotSettings,
) -> Figure {
    let columns: Option<Vec<usize>> = settings
        .co_occurrence_group
        .as_ref()
        .and_then(|name| groups.iter().position(|g| g == name))
        .map(|group| {
            (0..sample_groups.len())
                .filter(|&i| sample_groups[i] == group)
                .collect()
        });
    let co = co_occurrence_plot::Settings {
        forced: typed_features(&settings.co_occurrence_features),
        ..settings.co_occurrence.clone()
    };
    let options = &settings.co_occurrence_options;
    let samples = columns
        .as_ref()
        .map_or(profile.sample_names.len(), Vec::len);
    match co.view {
        co_occurrence_plot::View::Matrix => {
            match co_occurrence_plot::Data::compute(profile, columns.as_deref(), &co) {
                Ok(data) => co_occurrence_plot::matrix_plot(&data, &co, options),
                Err(message) => Figure::message(&message),
            }
        }
        co_occurrence_plot::View::OneFeature => {
            let Some(feature) = &session.selected_feature else {
                return Figure::message(NO_FEATURE);
            };
            match co_occurrence_plot::compute_partners(profile, columns.as_deref(), feature, &co) {
                Ok(found) => {
                    co_occurrence_plot::partner_plot(feature, &found, samples, co.clr, &co, options)
                }
                Err(message) => Figure::message(&message),
            }
        }
    }
}

/// The features of the box plot: the selected one, the most significant of
/// `ranked` (features that pass the filters, most significant first), or
/// those listed in the settings.
pub fn box_feature_names(
    settings: &PlotSettings,
    selected: Option<&str>,
    ranked: &[String],
) -> Vec<String> {
    match settings.box_features {
        BoxFeatures::Selected => selected.map(str::to_string).into_iter().collect(),
        BoxFeatures::MostSignificant => ranked
            .iter()
            .take(settings.box_feature_count.max(1))
            .cloned()
            .collect(),
        BoxFeatures::Listed => typed_features(&settings.box_feature_list),
    }
}

/// The box plot of the features chosen in the settings; `data_of` gives a
/// feature's values in each group (`None` for an unknown feature), and
/// `group_samples` the samples of each group, by name.
fn box_figure(
    session: &Session,
    settings: &PlotSettings,
    ranked: &[String],
    data_of: &dyn Fn(&str) -> Option<feature_plots::FeatureData>,
    group_samples: &dyn Fn(&str) -> Vec<String>,
) -> Figure {
    let names = box_feature_names(settings, session.selected_feature.as_deref(), ranked);
    let data: Vec<feature_plots::FeatureData> = names.iter().filter_map(|n| data_of(n)).collect();
    if data.is_empty() {
        return Figure::message(match settings.box_features {
            BoxFeatures::Selected => NO_FEATURE,
            BoxFeatures::MostSignificant => "No feature passes the filters",
            BoxFeatures::Listed => {
                "List features in Configure plot, or Ctrl+click them in the feature table"
            }
        });
    }
    let categories = settings.box_category_field.as_ref().and_then(|field| {
        let metadata = session.metadata.as_ref()?;
        if !metadata.fields().iter().any(|f| f == field) {
            return None;
        }
        let lists: Vec<Vec<String>> = data[0]
            .groups
            .iter()
            .map(|g| group_samples(&g.name))
            .collect();
        let slices: Vec<&[String]> = lists.iter().map(Vec::as_slice).collect();
        Some(stamp_core::stats::group_summary::majority_values(
            metadata, &slices, field,
        ))
    });
    multi_box::plot(&data, categories.as_deref(), &settings.box_plot)
}

/// The prevalence-abundance plot of a profile whose sample `i` is in group
/// `sample_groups[i]`; `active` are the features that pass the filters.
fn prevalence_figure(
    session: &Session,
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    groups: &[String],
    colors: Vec<Color>,
    active: &[String],
    settings: &PlotSettings,
) -> Figure {
    let mut data = prevalence_plot::Data::from_profile(
        profile,
        sample_groups,
        groups,
        settings.prevalence_only_active.then_some(active),
        settings.prevalence_min_abundance,
    );
    data.group_colors = colors;
    let mut highlighted = typed_features(&settings.prevalence_highlight);
    highlighted.extend(settings.highlighted.iter().cloned());
    if settings.prevalence_highlight_selected {
        highlighted.extend(session.selected_feature.iter().cloned());
    }
    highlighted.sort();
    highlighted.dedup();
    data.highlighted = highlighted;
    prevalence_plot::plot(&data, &settings.prevalence)
}

const NO_RESULTS: &str = "Load a profile to begin";
const NO_METADATA: &str = "Load a metadata file to compare groups of samples";
const NO_FEATURE: &str = "Select a feature in the feature table";

/// Build the figure for the current results of `mode`.
pub fn build(session: &Session, mode: Mode, kind: PlotKind, settings: &PlotSettings) -> Figure {
    let mut figure = stamp_plot::axes::with_fonts(settings.fonts, || {
        build_plain(session, mode, kind, settings)
    });
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

/// The map of where the samples of `profile` were taken, sample `i` being
/// in group `sample_groups[i]` of `groups`, drawn in `colors`.
fn map_figure(
    session: &Session,
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    groups: Vec<String>,
    colors: Vec<Color>,
    settings: &PlotSettings,
) -> Figure {
    let Some(metadata) = session.metadata.as_ref() else {
        return Figure::message(NO_METADATA);
    };
    let (lat, lon) = map_fields(metadata, settings);
    let (Some(lat), Some(lon)) = (lat, lon) else {
        return Figure::message(
            "No metadata field looks like a latitude or longitude; choose them in Configure plot",
        );
    };
    let options = &settings.map;
    let styling = match stamp_plot::ordination_style::resolve(
        &options.color_by,
        options.shape_by.as_deref(),
        Some(metadata),
        Some(profile),
        session.selected_feature.as_deref(),
        session.group_field.as_deref(),
    ) {
        Ok(styling) => styling,
        Err(message) => return Figure::message(&message),
    };
    let sizes = match options.size_by.resolve(
        Some(metadata),
        Some(profile),
        session.selected_feature.as_deref(),
    ) {
        Ok(sizes) => sizes,
        Err(message) => return Figure::message(&message),
    };
    match map_plot::Data::new(
        &profile.sample_names,
        sample_groups,
        groups,
        metadata,
        &lat,
        &lon,
    ) {
        Ok(data) => map_plot::plot(
            &map_plot::Data {
                group_colors: colors,
                styling,
                sizes,
                ..data
            },
            options,
        ),
        Err(message) => Figure::message(&message),
    }
}

/// The latitude and longitude fields of the map: those chosen, or else
/// fields named like them.
pub fn map_fields(
    metadata: &stamp_core::Metadata,
    settings: &PlotSettings,
) -> (Option<String>, Option<String>) {
    let (lat, lon) = map_plot::coordinate_fields(metadata);
    let chosen = |field: &Option<String>| {
        field
            .clone()
            .filter(|f| metadata.fields().iter().any(|m| m == f))
    };
    (
        chosen(&settings.map_lat_field).or(lat),
        chosen(&settings.map_lon_field).or(lon),
    )
}

/// What colours and shapes the samples of an ordination plot, as its
/// options ask, from the session's metadata and `profile` (the analysis
/// profile, for a feature's relative abundance).
fn ordination_styling(
    session: &Session,
    options: &ordination_plot::Options,
    profile: &stamp_core::Profile,
) -> Result<stamp_plot::ordination_style::Styling, String> {
    stamp_plot::ordination_style::resolve(
        &options.color_by,
        options.shape_by.as_deref(),
        session.metadata.as_ref(),
        Some(profile),
        session.selected_feature.as_deref(),
        session.group_field.as_deref(),
    )
}

/// One of the plots of whole communities, for some samples and their groups.
#[allow(clippy::too_many_arguments)]
fn community_plot(
    session: &Session,
    kind: PlotKind,
    samples: &[String],
    sample_groups: Vec<usize>,
    groups: Vec<String>,
    colors: Vec<Color>,
    profile: &stamp_core::Profile,
    settings: &PlotSettings,
) -> Figure {
    let Some(tree) = &session.tree else {
        return Figure::message(NO_RESULTS);
    };
    // What colours and shapes the samples of the ordinations, found first
    // so that a mistake (a feature name being typed) is reported at once.
    let ordination = match kind {
        PlotKind::Pcoa => Some(&settings.pcoa),
        PlotKind::Nmds => Some(&settings.nmds),
        _ => None,
    };
    let styling = match ordination.map(|options| ordination_styling(session, options, profile)) {
        Some(Ok(styling)) => styling,
        Some(Err(message)) => return Figure::message(&message),
        None => Default::default(),
    };
    let distance = settings.community_distance;
    // The slow plots (ordinations, distances) are kept by what they are
    // computed from, so that a change of colours, shapes or sizes does not
    // compute them again, and computed in a job.
    if matches!(
        kind,
        PlotKind::Pcoa | PlotKind::Nmds | PlotKind::DistanceBox
    ) {
        let tests = (kind == PlotKind::Pcoa && settings.pcoa_show_tests)
            .then_some(settings.community_permutations);
        let job = match kind {
            PlotKind::Pcoa => community_plots::CommunityJob::Pcoa { tests },
            PlotKind::Nmds => community_plots::CommunityJob::Nmds,
            _ => community_plots::CommunityJob::Distances,
        };
        let key = OrdinationKey {
            tree: Some(tree.clone()),
            inputs: format!(
                "{kind:?}|{}|{:?}|{}|{distance:?}|{tests:?}|{samples:?}|{sample_groups:?}",
                session.profile_level, settings.community_normalisation, session.seed,
            ),
        };
        let work = || {
            community_work(
                session,
                tree,
                samples,
                sample_groups.clone(),
                groups.clone(),
                settings,
                job,
            )
        };
        let cached = match session.ordinations.get_or_request(key, work) {
            Some(cached) => cached,
            None => return Figure::message(COMPUTING),
        };
        // What drawing needs of the community: its samples, groups and how
        // they look, not their abundances.
        let mut community = Community {
            samples: samples.to_vec(),
            sample_groups,
            groups,
            group_colors: colors,
            styling,
            table: stamp_core::stats::diversity::CommunityTable {
                abundances: Vec::new(),
                depth: None,
            },
        };
        return match (kind, cached) {
            (PlotKind::Pcoa, Cached::Community(Ok(layout))) => {
                community_plots::draw_pcoa(&community, &layout, &settings.pcoa)
            }
            (PlotKind::Nmds, Cached::Community(Ok(layout))) => {
                community_plots::draw_nmds(&community, &layout, &settings.nmds)
            }
            (_, Cached::Distances(matrix, depth)) => {
                community.table.depth = depth;
                community_plots::draw_distance_box(
                    &community,
                    distance,
                    &matrix,
                    &settings.distance_box,
                )
            }
            (_, Cached::Community(Err(message)) | Cached::Unavailable(message)) => {
                Figure::message(&message)
            }
            _ => Figure::message(NO_RESULTS),
        };
    }
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
    community_plots::alpha_plot(&community, settings.alpha_metric, &settings.alpha_box)
}

/// The work of computing `job` for some samples of the session's profiles,
/// in pieces: first the community table (with the session's seed, so that
/// the same samples are drawn when rarefying and the same permutations
/// made every time), then what the job needs.
fn community_work(
    session: &Session,
    tree: &std::sync::Arc<stamp_core::ProfileTree>,
    samples: &[String],
    sample_groups: Vec<usize>,
    groups: Vec<String>,
    settings: &PlotSettings,
    job: community_plots::CommunityJob,
) -> Work {
    use stamp_core::stats::results::Step;
    let tree = tree.clone();
    let samples = samples.to_vec();
    let level = session.profile_level.clone();
    let (normalisation, distance, seed) = (
        settings.community_normalisation,
        settings.community_distance,
        session.seed,
    );
    let mut job = Some(job);
    let mut run: Option<community_plots::CommunityRun> = None;
    let mut last = String::new();
    Box::new(move |progress, out_of_time| {
        if run.is_none() {
            let stage = "Reading the samples";
            if !progress.report_stage(stage, 0, 0) {
                return Some(None);
            }
            if last != stage {
                // Said first, so that the page shows it.
                last = stage.to_string();
                return None;
            }
            let mut rng = rng_from_seed(seed);
            match Community::from_tree(
                &tree,
                &samples,
                &level,
                sample_groups.clone(),
                groups.clone(),
                normalisation,
                &mut rng,
            ) {
                Ok(community) => {
                    let job = job.take()?;
                    run = Some(community_plots::CommunityRun::new(
                        community, distance, job, rng,
                    ));
                }
                Err(message) => return Some(Some(Cached::Unavailable(message))),
            }
        }
        let current = run.as_mut()?;
        let depth = current.depth();
        match current.advance(&mut stage_pace(progress, out_of_time, &mut last)) {
            Step::Done(output) => Some(Some(match output {
                community_plots::CommunityOutput::Layout(layout) => Cached::Community(layout),
                community_plots::CommunityOutput::Distances(matrix) => {
                    Cached::Distances(std::sync::Arc::new(matrix), depth)
                }
                community_plots::CommunityOutput::Mantel {
                    matrix,
                    differences,
                    test,
                } => Cached::Mantel {
                    matrix: std::sync::Arc::new(matrix),
                    differences: std::sync::Arc::new(differences),
                    test,
                    depth,
                },
                community_plots::CommunityOutput::Unavailable(message) => {
                    Cached::Unavailable(message)
                }
            })),
            Step::Paused => None,
            Step::Cancelled => Some(None),
        }
    })
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
/// `sample_groups[i]`; ANCOM itself is computed in a job, a feature at a
/// time.
fn ancom_figure(
    session: &Session,
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    groups: &[String],
    settings: &PlotSettings,
) -> Figure {
    use stamp_core::stats::ancom::AncomRun;
    use stamp_core::stats::results::Step;
    let features = profile.features();
    let samples: Vec<Vec<f64>> = (0..profile.sample_names.len())
        .map(|s| features.iter().map(|f| f.counts[s]).collect())
        .collect();
    let (test, alpha) = (settings.ancom_test, settings.ancom_alpha);
    let hash = hash_of_values(samples.iter().map(Vec::as_slice));
    let key = OrdinationKey {
        tree: None,
        inputs: format!("ANCOM|{test:?}|{alpha}|{sample_groups:?}|{hash:016x}"),
    };
    let work = || -> Work {
        let sample_groups = sample_groups.to_vec();
        let mut samples = Some(samples);
        let mut run: Option<AncomRun> = None;
        let mut last = String::new();
        Box::new(move |progress, out_of_time| {
            if run.is_none() {
                let stage = "Preparing ANCOM";
                if !progress.report_stage(stage, 0, 0) {
                    return Some(None);
                }
                if last != stage {
                    last = stage.to_string();
                    return None;
                }
                match AncomRun::new(&samples.take()?, &sample_groups, test, alpha) {
                    Some(new) => run = Some(new),
                    None => return Some(Some(Cached::Ancom(None))),
                }
            }
            let current = run.as_mut()?;
            let mut pace = stage_pace(progress, out_of_time, &mut last);
            match current
                .advance(&mut |done, total| pace("ANCOM: log ratios of feature", done, total))
            {
                Step::Done(result) => Some(Some(Cached::Ancom(Some(std::sync::Arc::new(result))))),
                Step::Paused => None,
                Step::Cancelled => Some(None),
            }
        })
    };
    match session.ordinations.get_or_request(key, work) {
        Some(Cached::Ancom(Some(result))) => {
            let names: Vec<String> = features.iter().map(|f| f.name.clone()).collect();
            let options = ancom_plot::Options {
                highlighted: settings.highlighted.clone(),
                ..settings.ancom.clone()
            };
            ancom_plot::plot(&result, &names, groups, &options)
        }
        Some(_) => Figure::message(
            "ANCOM needs two groups with samples, a group of more than one sample, and two features",
        ),
        None => Figure::message(COMPUTING),
    }
}

/// The Mantel plot of a correlation analysis: the test computed in a job.
fn mantel_figure(
    session: &Session,
    profile: &stamp_core::CorrelationProfile,
    results: &stamp_core::stats::results::CorrelationResults,
    settings: &PlotSettings,
) -> Figure {
    let Some(tree) = &session.tree else {
        return Figure::message(NO_RESULTS);
    };
    let samples = &profile.profile.sample_names;
    let (distance, method, permutations) = (
        settings.community_distance,
        results.analysis.method,
        settings.community_permutations,
    );
    let key = OrdinationKey {
        tree: Some(tree.clone()),
        inputs: format!(
            "Mantel|{}|{:?}|{}|{distance:?}|{method:?}|{permutations}|{samples:?}|{:x}",
            session.profile_level,
            settings.community_normalisation,
            session.seed,
            hash_of_values(std::iter::once(profile.values.as_slice())),
        ),
    };
    let work = || {
        community_work(
            session,
            tree,
            samples,
            vec![0; samples.len()],
            vec![String::new()],
            settings,
            community_plots::CommunityJob::Mantel {
                values: profile.values.clone(),
                method,
                permutations,
            },
        )
    };
    match session.ordinations.get_or_request(key, work) {
        Some(Cached::Mantel {
            matrix,
            differences,
            test,
            depth,
        }) => {
            let community = Community {
                samples: samples.clone(),
                sample_groups: vec![0; samples.len()],
                groups: vec![String::new()],
                group_colors: Vec::new(),
                styling: Default::default(),
                table: stamp_core::stats::diversity::CommunityTable {
                    abundances: Vec::new(),
                    depth,
                },
            };
            community_plots::draw_mantel(
                &community,
                &profile.field,
                distance,
                method,
                &matrix,
                &differences,
                &test,
                &settings.mantel,
            )
        }
        Some(Cached::Unavailable(message)) => Figure::message(&message),
        Some(_) => Figure::message(NO_RESULTS),
        None => Figure::message(COMPUTING),
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
                return Figure::message(
                    session
                        .error(Mode::TwoSamples)
                        .unwrap_or("Running the two-sample analysis..."),
                );
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
                    PValueKind::of(results.correction.method),
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
                    data.add_p_values(PValueKind::of(results.correction.method), |feature| {
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
                    let label = PValueKind::shown(results.correction.method, corrected).label();
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
                        .error(Mode::TwoGroups)
                        .unwrap_or("Running the two-group analysis..."),
                );
            };
            let rows = session.group_rows();
            let color1 = session.group_color(&profile.group_name1);
            let color2 = session.group_color(&profile.group_name2);
            let data_of = |name: &str| {
                let mut data =
                    feature_plots::FeatureData::from_group_profile(profile, name, field)?;
                data.groups[0].color = color1;
                data.groups[1].color = color2;
                data.p_value = results
                    .rows
                    .iter()
                    .find(|r| r.feature == name)
                    .map(|r| format_p_value(r.p_value_corrected, settings.min_p_exponent));
                data.p_value_kind = Some(PValueKind::of(results.correction.method));
                Some(data)
            };
            let feature_data = || data_of(feature?);
            let ranked: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
            let first = profile.samples_in_group1().len();
            let two_sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
                .map(|i| usize::from(i >= first))
                .collect();
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
                PlotKind::Box => box_figure(session, settings, &ranked, &data_of, &|group| {
                    if group == profile.group_name1 {
                        profile.samples_in_group1().to_vec()
                    } else {
                        profile.samples_in_group2().to_vec()
                    }
                }),
                PlotKind::PrevalenceAbundance => prevalence_figure(
                    session,
                    &profile.profile,
                    &two_sample_groups,
                    &[profile.group_name1.clone(), profile.group_name2.clone()],
                    vec![color1, color2],
                    &ranked,
                    settings,
                ),
                PlotKind::Scatter => {
                    let mut data = scatter::Data {
                        x_color: color1,
                        y_color: color2,
                        ..scatter::Data::from_group_profile(profile, settings.group_spread)
                    };
                    data.add_p_values(PValueKind::of(results.correction.method), |feature| {
                        let row = results.rows.iter().find(|r| r.feature == feature)?;
                        Some((p_text(row.p_value), p_text(row.p_value_corrected)))
                    });
                    scatter::plot(&data, &scatter_options)
                }
                PlotKind::Map => map_figure(
                    session,
                    &profile.profile,
                    &two_sample_groups,
                    vec![profile.group_name1.clone(), profile.group_name2.clone()],
                    vec![color1, color2],
                    settings,
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
                PlotKind::GroupHeatmap | PlotKind::CoOccurrence => {
                    let first = profile.samples_in_group1().len();
                    let sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
                        .map(|i| usize::from(i >= first))
                        .collect();
                    let groups = [profile.group_name1.clone(), profile.group_name2.clone()];
                    if kind == PlotKind::GroupHeatmap {
                        let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                        group_heatmap_figure(
                            session,
                            &profile.profile,
                            &sample_groups,
                            &groups,
                            &names,
                            settings,
                        )
                    } else {
                        co_occurrence_figure(
                            session,
                            &profile.profile,
                            &sample_groups,
                            &groups,
                            settings,
                        )
                    }
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
                        session,
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
                        &profile.profile,
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
                        .error(Mode::Correlation)
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
                                    "{} = {:.3}, {} (n = {})",
                                    results.analysis.method.coefficient_name(),
                                    r.coefficient,
                                    p_value_text(
                                        &p_text(r.p_value_corrected),
                                        Some(PValueKind::of(results.correction.method))
                                    ),
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
                    let corrected = settings.p_value_histogram_corrected;
                    let values: Vec<f64> = if corrected {
                        results.rows.iter().map(|r| r.p_value_corrected).collect()
                    } else {
                        results.rows.iter().map(|r| r.p_value).collect()
                    };
                    let label = PValueKind::shown(results.correction.method, corrected).label();
                    histogram::p_value_histogram(
                        &values,
                        &histogram::PValueOptions {
                            label: label.to_string(),
                            ..settings.p_value_histogram.clone()
                        },
                    )
                }
                PlotKind::Mantel => mantel_figure(session, profile, results, settings),
                PlotKind::Map => {
                    // By the groups of the group field, if any.
                    let samples = &profile.profile.sample_names;
                    let (sample_groups, groups) = match session.grouping() {
                        Some(grouping) => {
                            let mut names: Vec<String> =
                                grouping.names().map(str::to_string).collect();
                            let mut sample_groups: Vec<usize> = samples
                                .iter()
                                .map(|sample| {
                                    names
                                        .iter()
                                        .position(|g| {
                                            grouping
                                                .samples(g)
                                                .is_some_and(|members| members.contains(sample))
                                        })
                                        .unwrap_or(usize::MAX)
                                })
                                .collect();
                            if sample_groups.contains(&usize::MAX) {
                                names.push("No group".to_string());
                                for g in &mut sample_groups {
                                    if *g == usize::MAX {
                                        *g = names.len() - 1;
                                    }
                                }
                            }
                            (sample_groups, names)
                        }
                        None => (vec![0; samples.len()], vec!["Samples".to_string()]),
                    };
                    let colors = groups.iter().map(|g| session.group_color(g)).collect();
                    map_figure(
                        session,
                        &profile.profile,
                        &sample_groups,
                        groups,
                        colors,
                        settings,
                    )
                }
                PlotKind::CoOccurrence => co_occurrence_figure(
                    session,
                    &profile.profile,
                    &vec![0; profile.profile.sample_names.len()],
                    &[],
                    settings,
                ),
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
                        .error(Mode::MultipleGroups)
                        .unwrap_or("Running the multiple-group analysis..."),
                );
            };
            let rows = session.multi_rows();
            let colors: Vec<Color> = profile
                .group_names
                .iter()
                .map(|g| session.group_color(g))
                .collect();
            let data_of = |name: &str| {
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
                data.p_value_kind = Some(PValueKind::of(results.correction.method));
                Some(data)
            };
            let feature_data = || data_of(feature?);
            let ranked: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
            match kind {
                PlotKind::Map => {
                    let sample_groups: Vec<usize> = profile
                        .samples_in_groups()
                        .iter()
                        .enumerate()
                        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                        .collect();
                    map_figure(
                        session,
                        &profile.profile,
                        &sample_groups,
                        profile.group_names.clone(),
                        colors.clone(),
                        settings,
                    )
                }
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
                PlotKind::GroupHeatmap | PlotKind::CoOccurrence => {
                    let sample_groups: Vec<usize> = profile
                        .samples_in_groups()
                        .iter()
                        .enumerate()
                        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                        .collect();
                    if kind == PlotKind::GroupHeatmap {
                        let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                        group_heatmap_figure(
                            session,
                            &profile.profile,
                            &sample_groups,
                            &profile.group_names,
                            &names,
                            settings,
                        )
                    } else {
                        co_occurrence_figure(
                            session,
                            &profile.profile,
                            &sample_groups,
                            &profile.group_names,
                            settings,
                        )
                    }
                }
                PlotKind::Bar => match feature_data() {
                    Some(data) => feature_plots::bar_plot(&data, &settings.bar),
                    None => Figure::message(NO_FEATURE),
                },
                PlotKind::Box => box_figure(session, settings, &ranked, &data_of, &|group| {
                    profile
                        .group_names
                        .iter()
                        .position(|g| g == group)
                        .map(|i| profile.samples_in_groups()[i].to_vec())
                        .unwrap_or_default()
                }),
                PlotKind::PrevalenceAbundance => {
                    let sample_groups: Vec<usize> = profile
                        .samples_in_groups()
                        .iter()
                        .enumerate()
                        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                        .collect();
                    prevalence_figure(
                        session,
                        &profile.profile,
                        &sample_groups,
                        &profile.group_names,
                        colors.clone(),
                        &ranked,
                        settings,
                    )
                }
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
                    ancom_figure(
                        session,
                        &profile.profile,
                        &groups,
                        &profile.group_names,
                        settings,
                    )
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
                    &profile.profile,
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
        // The slow parts of plots computed as they are built.
        session.ordinations.set_synchronous(true);
        session
    }

    /// A slow plot asks for its slow part to be computed and is drawn as a
    /// placeholder until it has been.
    #[test]
    fn slow_plots_are_computed_in_a_job() {
        let session = session();
        session.ordinations.set_synchronous(false);
        let settings = PlotSettings::default();
        for kind in [
            PlotKind::Pcoa,
            PlotKind::Nmds,
            PlotKind::DistanceBox,
            PlotKind::Ancom,
        ] {
            let figure = build(&session, Mode::MultipleGroups, kind, &settings);
            assert_eq!(figure.shapes.len(), 1, "{kind:?}");
            let request = session.ordinations.take_request().expect("a request");
            assert!(session.ordinations.take_request().is_none());
            let running = request.start();
            let mut stages = Vec::new();
            loop {
                let stage = running.progress().stage();
                if stages.last() != Some(&stage) {
                    stages.push(stage);
                }
                match running.poll(&session.ordinations) {
                    PlotPoll::Finished => break,
                    PlotPoll::Running => std::thread::sleep(std::time::Duration::from_millis(1)),
                    _ => panic!("{kind:?}: the calculation failed"),
                }
            }
            assert!(stages.iter().any(|s| !s.is_empty()), "{kind:?}: {stages:?}");
            let figure = build(&session, Mode::MultipleGroups, kind, &settings);
            assert!(!figure.regions.is_empty(), "{kind:?}");
            assert!(session.ordinations.take_request().is_none(), "{kind:?}");
        }
    }

    #[test]
    fn every_plot_of_every_mode_draws_something() {
        let session = session();
        for mode in Mode::ALL {
            for &kind in PlotKind::available(mode) {
                let figure = build(&session, mode, kind, &PlotSettings::default());
                if kind == PlotKind::Map {
                    // The test metadata has no coordinates; the map is
                    // tested with data that has them in tests/ui.rs.
                    assert!(
                        matches!(&figure.shapes[..], [stamp_plot::Shape::Text { text, .. }]
                        if text.contains("latitude or longitude"))
                    );
                    continue;
                }
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
                PlotKind::Pcoa,
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
            (Mode::TwoGroups, PlotKind::Pcoa),
            (Mode::TwoGroups, PlotKind::Heatmap),
            (Mode::MultipleGroups, PlotKind::Pcoa),
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

    #[test]
    fn every_plot_is_listed_once_under_its_group() {
        for mode in Mode::ALL {
            let grouped = PlotKind::grouped(mode);
            let listed: Vec<PlotKind> = grouped.iter().flat_map(|(_, k)| k.clone()).collect();
            assert_eq!(listed.len(), PlotKind::available(mode).len(), "{mode:?}");
            for kind in PlotKind::available(mode) {
                assert_eq!(listed.iter().filter(|k| *k == kind).count(), 1);
            }
            for (group, kinds) in &grouped {
                assert!(kinds.iter().all(|k| k.group() == *group));
                // Checks on the tests come last in their group.
                let first_check = kinds.iter().position(|k| k.checks_the_tests());
                if let Some(at) = first_check {
                    assert!(kinds[at..].iter().all(|k| k.checks_the_tests()));
                }
            }
            // Groups appear in their fixed order.
            let order: Vec<usize> = grouped
                .iter()
                .map(|(g, _)| PlotGroup::ALL.iter().position(|a| a == g).unwrap())
                .collect();
            assert!(order.windows(2).all(|w| w[0] < w[1]));
        }
        // Two samples have no groups of samples to compare.
        let two_samples: Vec<PlotGroup> = PlotKind::grouped(Mode::TwoSamples)
            .iter()
            .map(|(g, _)| *g)
            .collect();
        assert!(!two_samples.contains(&PlotGroup::Similarity));
        let differences = &PlotKind::grouped(Mode::TwoSamples)[0].1;
        assert_eq!(
            differences,
            &[
                PlotKind::ExtendedErrorBar,
                PlotKind::Volcano,
                PlotKind::PValueHistogram,
                PlotKind::MultipleComparison
            ]
        );
    }
}
