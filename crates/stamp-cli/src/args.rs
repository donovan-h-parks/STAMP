//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::effect_size::{GroupEffectSize, SampleEffectSize};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::post_hoc::PostHocTest;
use stamp_core::stats::results::{FilterOperator, Transform};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::TestType;
use stamp_core::Unclassified;

/// Statistical analysis of taxonomic and functional profiles.
#[derive(Parser)]
#[command(name = "stamp-cli", version = stamp_core::VERSION, propagate_version = true)]
pub struct Cli {
    /// Do not show the progress of slow steps (shown on standard error,
    /// only when it is a terminal)
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Describe a profile: its hierarchical levels, samples and metadata fields
    Info(InfoArgs),
    /// Compare the features of two samples
    TwoSamples(TwoSamplesArgs),
    /// Compare the features of two groups of samples
    TwoGroups(TwoGroupsArgs),
    /// Compare the features of several groups of samples
    MultiGroups(MultiGroupsArgs),
    /// Compare every pair of groups for one feature
    PostHoc(PostHocArgs),
    /// Correlate every feature with a numeric metadata field
    Correlation(CorrelationArgs),
    /// Find features that differ between groups by ANCOM
    Ancom(AncomArgs),
    /// Correlate features with each other across samples (co-occurrence)
    CoOccurrence(CoOccurrenceArgs),
    /// Convert another tool's output to a STAMP profile
    Convert(ConvertArgs),
}

#[derive(Args)]
pub struct InfoArgs {
    /// STAMP profile file
    #[arg(long, short)]
    pub profile: PathBuf,
    /// Metadata file describing the samples
    #[arg(long, short)]
    pub metadata: Option<PathBuf>,
    #[command(flatten)]
    pub sample_filter: SampleFilterArgs,
}

/// Choosing samples by their metadata values.
#[derive(Args)]
pub struct SampleFilterArgs {
    /// Keep only samples whose metadata meet a condition: FIELD=VALUE, or for
    /// numbers FIELD<X, FIELD<=X, FIELD>X or FIELD>=X (may be repeated;
    /// values given for one field are alternatives, different fields must all
    /// match). Quote it: --where "Site=deep sea"
    #[arg(long = "where", value_name = "CONDITION")]
    pub where_: Vec<String>,
    /// Leave out samples with this metadata value, FIELD=VALUE (may be repeated)
    #[arg(long = "where-not", value_name = "FIELD=VALUE")]
    pub where_not: Vec<String>,
}

/// Which features to analyse and where to write the results.
#[derive(Args)]
pub struct ProfileArgs {
    /// STAMP profile file
    #[arg(long, short)]
    pub profile: PathBuf,
    /// Hierarchical level to analyse [default: the most specific level]
    #[arg(long, value_name = "HEADING")]
    pub profile_level: Option<String>,
    /// Level that relative frequencies are taken against [default: the entire sample]
    #[arg(long, value_name = "HEADING")]
    pub parent_level: Option<String>,
    /// Treatment of unclassified reads
    #[arg(long, value_enum, default_value_t = UnclassifiedArg::Retain)]
    pub unclassified: UnclassifiedArg,
    /// Write the results table to this file instead of standard output
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

/// Samples and the metadata field that groups them.
#[derive(Args)]
pub struct MetadataArgs {
    /// Metadata file describing the samples
    #[arg(long, short)]
    pub metadata: PathBuf,
    /// Metadata field that defines the groups
    #[arg(long, short)]
    pub field: String,
    /// Sample to leave out of the analysis (may be repeated)
    #[arg(long = "exclude-sample", value_name = "SAMPLE")]
    pub excluded_samples: Vec<String>,
    #[command(flatten)]
    pub sample_filter: SampleFilterArgs,
}

/// Settings of the resampling methods.
#[derive(Args)]
pub struct ResamplingArgs {
    /// Seed of the random number generator used by resampling methods
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    /// Replicates for bootstrap and permutation methods
    #[arg(long, default_value_t = 1000)]
    pub replicates: usize,
    /// Pseudocount used where a ratio would otherwise be undefined
    #[arg(long, default_value_t = 0.5)]
    pub pseudocount: f64,
}

/// Options for drawing the results as a plot.
#[derive(Args)]
pub struct PlotArgs {
    /// Also draw the (filtered) results as a plot, saved to this file; the
    /// extension chooses the format: .png, .svg or .pdf
    #[arg(long, value_name = "FILE")]
    pub plot: Option<PathBuf>,
    /// Kind of plot [default: extended-error-bar for two samples or groups,
    /// heatmap for several groups, post-hoc for the post-hoc command]
    #[arg(long, value_enum, value_name = "TYPE")]
    pub plot_type: Option<PlotTypeArg>,
    /// Feature shown by the bar, box and correlation scatter plots [default:
    /// the most significant]; may be repeated for a box plot of several
    /// features
    #[arg(long, value_name = "FEATURE")]
    pub plot_feature: Vec<String>,
    /// Height of the plot, in inches, for plots of fixed height
    #[arg(long, value_name = "INCHES")]
    pub plot_height: Option<f64>,
    /// Resolution of a PNG plot, in dots per inch
    #[arg(long, default_value_t = 300.0, value_name = "DPI")]
    pub plot_dpi: f64,
    /// Width of the plot, in inches
    #[arg(long, default_value_t = 7.0, value_name = "INCHES")]
    pub plot_width: f64,
    /// Height of each feature's row in the plot, in inches
    #[arg(long, default_value_t = 0.2, value_name = "INCHES")]
    pub plot_row_height: f64,
    /// Order of the features in the plot, top to bottom
    #[arg(long, value_enum, default_value_t = PlotSortArg::PValues)]
    pub plot_sort_by: PlotSortArg,
    /// Where to put a legend naming the two samples or groups
    #[arg(long, value_enum, default_value_t = PlotLegendArg::None)]
    pub plot_legend: PlotLegendArg,
    /// Show uncorrected p-values in the plot
    #[arg(long)]
    pub plot_uncorrected_p_values: bool,
    /// Font size of the plot's tick labels, in points
    #[arg(long, default_value_t = 8.0, value_name = "POINTS")]
    pub tick_font_size: f64,
    /// Font size of the plot's axis labels, in points
    #[arg(long, default_value_t = 9.0, value_name = "POINTS")]
    pub axis_label_font_size: f64,
    /// Font size of the plot's legend, in points
    #[arg(long, default_value_t = 9.0, value_name = "POINTS")]
    pub legend_font_size: f64,
    /// Font of the plot's text
    #[arg(long, value_enum, default_value_t = FontArg::Arial)]
    pub font: FontArg,
    /// Measure of the alpha diversity plot
    #[arg(long, value_enum, default_value_t = AlphaMetricArg::Shannon)]
    pub alpha_metric: AlphaMetricArg,
    /// Distance between samples for the PCoA and distance box plots
    #[arg(long, value_enum, default_value_t = DistanceArg::BrayCurtis)]
    pub distance: DistanceArg,
    /// Rarefy every sample to the depth of the smallest before measuring
    /// diversity or distances (needs counts of sequences)
    #[arg(long)]
    pub rarefy: bool,
    /// What each curve of a rarefaction plot stands for
    #[arg(long, value_enum, default_value_t = RarefactionCurvesArg::Samples)]
    pub rarefaction_curves: RarefactionCurvesArg,
    /// What each curve of a rank-abundance plot stands for
    #[arg(long, value_enum, default_value_t = RankCurvesArg::Samples)]
    pub rank_curves: RankCurvesArg,
    /// Style of the shared-features plot (Venn: two or three groups)
    #[arg(long, value_enum, default_value_t = SharedStyleArg::Upset)]
    pub shared_style: SharedStyleArg,
    /// For the shared-features plot: a feature is present in a group if found
    /// in at least this percentage of the group's samples
    #[arg(long, default_value_t = 50.0, value_name = "PERCENT")]
    pub min_prevalence: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum FontArg {
    /// Arial (bundled as Liberation Sans, with the same letter widths)
    Arial,
    /// Times New Roman (bundled as Liberation Serif)
    TimesNewRoman,
    /// Courier New (bundled as Liberation Mono)
    CourierNew,
    /// DejaVu Sans, the font of STAMP v2's plots
    DejavuSans,
}

impl From<FontArg> for stamp_plot::text::PlotFont {
    fn from(value: FontArg) -> Self {
        use stamp_plot::text::PlotFont;
        match value {
            FontArg::Arial => PlotFont::Arial,
            FontArg::TimesNewRoman => PlotFont::TimesNewRoman,
            FontArg::CourierNew => PlotFont::CourierNew,
            FontArg::DejavuSans => PlotFont::DejaVuSans,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum RankCurvesArg {
    /// Each sample
    Samples,
    /// The mean proportions of each group
    Groups,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum SharedStyleArg {
    Upset,
    Venn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum AlphaMetricArg {
    /// Number of features present
    Observed,
    Shannon,
    Simpson,
    Chao1,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum DistanceArg {
    BrayCurtis,
    Jaccard,
    Aitchison,
    /// Euclidean distance between proportions (a PCoA of it is a PCA)
    Euclidean,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum RarefactionCurvesArg {
    /// Each sample, against the number of its sequences read
    Samples,
    /// Each group, against the sequences read from all of its samples together
    Pooled,
    /// Each group, against the number of its samples
    Accumulated,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum PlotTypeArg {
    /// Proportions, effect sizes with confidence intervals, and p-values
    ExtendedErrorBar,
    /// One feature in every sample (groups), or one statistic per feature (two samples)
    Bar,
    /// Distribution of one feature in each group
    Box,
    /// Most abundant features in two samples
    ProfileBar,
    /// Every feature in one sample or group against the other
    Scatter,
    /// Sequence counts of the features of two samples
    SequenceHistogram,
    /// Distribution of the p-values (corrected, unless --plot-uncorrected-p-values)
    PValueHistogram,
    /// Three views of the multiple-test correction
    MultipleComparison,
    /// Features against samples, clustered
    Heatmap,
    /// Every pair of groups for one feature
    PostHoc,
    /// Expected number of features against sequences read, per sample
    Rarefaction,
    /// Effect size against significance for every feature
    Volcano,
    /// Stacked bars of the most abundant features of each sample
    Composition,
    /// Diversity within the samples of each group; in the correlation
    /// command, the diversity of each sample against the numeric field
    AlphaDiversity,
    /// Samples placed by the distances between them, with community tests
    Pcoa,
    /// Distances between samples within and between groups
    DistanceBox,
    /// A feature against the numeric field, with a fitted line (correlation)
    CorrelationScatter,
    /// Samples placed by non-metric multidimensional scaling of a distance
    Nmds,
    /// Features present in each group, and in which groups together
    SharedFeatures,
    /// Each sample's features from most to least abundant
    RankAbundance,
    /// ANCOM: W against the difference in centred log ratios, per feature
    Ancom,
    /// Community distance against difference in the numeric field (correlation)
    Mantel,
    /// Features against groups: each cell a summary of the feature in the
    /// group (mean, median, maximum or prevalence); see the --heatmap-* options
    GroupHeatmap,
    /// Prevalence against mean abundance of every feature, per group; see the
    /// --prevalence-* options
    PrevalenceAbundance,
    /// Where the samples were taken, from latitude and longitude fields of
    /// the metadata, on a map; see the --map-* options and --color-by
    Map,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum PlotSortArg {
    PValues,
    EffectSizes,
    FeatureLabels,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum PlotLegendArg {
    None,
    UpperLeft,
    UpperCenter,
    UpperRight,
    LowerLeft,
    LowerCenter,
    LowerRight,
}

#[derive(Args)]
pub struct TwoSamplesArgs {
    #[command(flatten)]
    pub profile: ProfileArgs,
    /// First sample
    #[arg(long)]
    pub sample1: String,
    /// Second sample
    #[arg(long)]
    pub sample2: String,
    /// Statistical test
    #[arg(long, value_enum, default_value_t = SampleTestArg::Fisher)]
    pub test: SampleTestArg,
    /// One-sided or two-sided p-values
    #[arg(long = "type", value_enum, default_value_t = TestTypeArg::TwoSided)]
    pub test_type: TestTypeArg,
    /// Confidence-interval method
    #[arg(long, value_enum, default_value_t = SampleCiArg::DpNewcombeWilson)]
    pub ci: SampleCiArg,
    /// Nominal coverage of the confidence interval
    #[arg(long, default_value_t = 0.95)]
    pub coverage: f64,
    /// Multiple-test correction
    #[arg(long, value_enum, default_value_t = CorrectionArg::None)]
    pub correction: CorrectionArg,
    /// Keep only features with a corrected p-value at or below this level
    #[arg(long, value_name = "P")]
    pub significance: Option<f64>,
    /// Sequence-count filter: maximum=N, minimum=N or independent=N1,N2
    #[arg(long, value_name = "FILTER")]
    pub sequence_filter: Option<String>,
    /// Parent sequence-count filter, in the same form as --sequence-filter
    #[arg(long, value_name = "FILTER")]
    pub parent_sequence_filter: Option<String>,
    /// Effect-size filter as MEASURE=MINIMUM, with MEASURE one of
    /// diff-between-prop, odds-ratio, ratio-proportions (at most two)
    #[arg(long = "effect-size-filter", value_name = "FILTER")]
    pub effect_size_filters: Vec<String>,
    /// With two effect-size filters: 'or' removes a feature that fails either,
    /// 'and' removes it only if it fails both
    #[arg(long, value_enum, default_value_t = OperatorArg::Or)]
    pub effect_size_operator: OperatorArg,
    #[command(flatten)]
    pub resampling: ResamplingArgs,
    #[command(flatten)]
    pub plot: PlotArgs,
}

#[derive(Args)]
pub struct TwoGroupsArgs {
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub metadata: MetadataArgs,
    /// First group
    #[arg(long)]
    pub group1: String,
    /// Second group [default: all other samples]
    #[arg(long)]
    pub group2: Option<String>,
    /// Statistical test
    #[arg(long, value_enum, default_value_t = GroupTestArg::Welch)]
    pub test: GroupTestArg,
    /// For the paired tests: the metadata field that says which sample of
    /// the first group belongs with which sample of the second (the subject
    /// both came from, say). Samples without exactly one partner are left out
    #[arg(long, value_name = "FIELD")]
    pub pair_field: Option<String>,
    /// Transform applied before the test; changes the p-values only
    #[arg(long, value_enum, default_value_t = TransformArg::None)]
    pub transform: TransformArg,
    /// One-sided or two-sided p-values
    #[arg(long = "type", value_enum, default_value_t = TestTypeArg::TwoSided)]
    pub test_type: TestTypeArg,
    /// Nominal coverage of the confidence interval
    #[arg(long, default_value_t = 0.95)]
    pub coverage: f64,
    /// Multiple-test correction
    #[arg(long, value_enum, default_value_t = CorrectionArg::None)]
    pub correction: CorrectionArg,
    /// Keep only features with a corrected p-value at or below this level
    #[arg(long, value_name = "P")]
    pub significance: Option<f64>,
    /// Sequence-count filter: maximum=N, minimum=N, independent-minimum=N1,N2
    /// or independent-maximum=N1,N2
    #[arg(long, value_name = "FILTER")]
    pub sequence_filter: Option<String>,
    /// Parent sequence-count filter, in the same form as --sequence-filter
    #[arg(long, value_name = "FILTER")]
    pub parent_sequence_filter: Option<String>,
    /// Effect-size filter as MEASURE=MINIMUM, with MEASURE one of
    /// diff-between-prop, ratio-proportions (at most two)
    #[arg(long = "effect-size-filter", value_name = "FILTER")]
    pub effect_size_filters: Vec<String>,
    /// With two effect-size filters: 'or' removes a feature that fails either,
    /// 'and' removes it only if it fails both
    #[arg(long, value_enum, default_value_t = OperatorArg::Or)]
    pub effect_size_operator: OperatorArg,
    #[command(flatten)]
    pub resampling: ResamplingArgs,
    #[command(flatten)]
    pub plot: PlotArgs,
    #[command(flatten)]
    pub heatmap: HeatmapArgs,
    #[command(flatten)]
    pub boxes: BoxArgs,
    #[command(flatten)]
    pub prevalence: PrevalenceArgs,
    #[command(flatten)]
    pub ordination: OrdinationArgs,
    #[command(flatten)]
    pub map: MapArgs,
}

#[derive(Args)]
pub struct MultiGroupsArgs {
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub metadata: MetadataArgs,
    /// Group to include (may be repeated) [default: every group]
    #[arg(long = "group", value_name = "GROUP")]
    pub groups: Vec<String>,
    /// Statistical test
    #[arg(long, value_enum, default_value_t = MultiGroupTestArg::Anova)]
    pub test: MultiGroupTestArg,
    /// For the Friedman test: the metadata field that says which samples
    /// came from the same subject. Subjects without exactly one sample in
    /// every group are left out
    #[arg(long, value_name = "FIELD")]
    pub pair_field: Option<String>,
    /// Transform applied before the test; changes the p-values only
    #[arg(long, value_enum, default_value_t = TransformArg::None)]
    pub transform: TransformArg,
    /// Multiple-test correction
    #[arg(long, value_enum, default_value_t = CorrectionArg::None)]
    pub correction: CorrectionArg,
    /// Keep only features with a corrected p-value at or below this level
    #[arg(long, value_name = "P")]
    pub significance: Option<f64>,
    /// Keep only features with an eta-squared effect size of at least this value
    #[arg(long, value_name = "MINIMUM")]
    pub min_effect_size: Option<f64>,
    /// Seed of the random number generator (used by the Storey correction
    /// and by the community tests and rarefying of the PCoA, NMDS and
    /// distance box plots)
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    #[command(flatten)]
    pub plot: PlotArgs,
    #[command(flatten)]
    pub heatmap: HeatmapArgs,
    #[command(flatten)]
    pub boxes: BoxArgs,
    #[command(flatten)]
    pub prevalence: PrevalenceArgs,
    #[command(flatten)]
    pub ordination: OrdinationArgs,
    #[command(flatten)]
    pub map: MapArgs,
}

#[derive(Args)]
pub struct CorrelationArgs {
    #[command(flatten)]
    pub profile: ProfileArgs,
    /// Metadata file describing the samples
    #[arg(long, short)]
    pub metadata: PathBuf,
    /// Metadata field holding a number for each sample; samples without a
    /// number are left out
    #[arg(long, short)]
    pub field: String,
    /// Sample to leave out of the analysis (may be repeated)
    #[arg(long = "exclude-sample", value_name = "SAMPLE")]
    pub excluded_samples: Vec<String>,
    #[command(flatten)]
    pub sample_filter: SampleFilterArgs,
    /// Measure of correlation
    #[arg(long, value_enum, default_value_t = CorrelationMethodArg::Spearman)]
    pub method: CorrelationMethodArg,
    /// Multiple-test correction
    #[arg(long, value_enum, default_value_t = CorrectionArg::None)]
    pub correction: CorrectionArg,
    /// Keep only features with a corrected p-value at or below this level
    #[arg(long, value_name = "P")]
    pub significance: Option<f64>,
    /// Keep only features whose correlation is at least this far from zero
    #[arg(long, value_name = "MINIMUM")]
    pub min_correlation: Option<f64>,
    /// Metadata field whose groups colour the points of the scatter plot
    #[arg(long, value_name = "FIELD")]
    pub group_field: Option<String>,
    /// Seed of the random number generator (used by the Storey correction)
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    #[command(flatten)]
    pub plot: PlotArgs,
    #[command(flatten)]
    pub ordination: OrdinationArgs,
    #[command(flatten)]
    pub map: MapArgs,
}

#[derive(Args)]
pub struct AncomArgs {
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub metadata: MetadataArgs,
    /// Group to include (may be repeated) [default: every group]
    #[arg(long = "group", value_name = "GROUP")]
    pub groups: Vec<String>,
    /// Test applied to the log ratio of every pair of features
    #[arg(long, value_enum, default_value_t = MultiGroupTestArg::Anova)]
    pub test: MultiGroupTestArg,
    /// Significance level of each log-ratio test, after the Holm-Bonferroni
    /// correction of each feature's tests
    #[arg(long, default_value_t = 0.05)]
    pub alpha: f64,
    #[command(flatten)]
    pub plot: PlotArgs,
}

#[derive(Args)]
pub struct PostHocArgs {
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub metadata: MetadataArgs,
    /// Group to include (may be repeated) [default: every group]
    #[arg(long = "group", value_name = "GROUP")]
    pub groups: Vec<String>,
    /// Feature to compare between groups (may be repeated)
    #[arg(long = "feature", value_name = "FEATURE", required = true)]
    pub features: Vec<String>,
    /// Post-hoc test
    #[arg(long, value_enum, default_value_t = PostHocTestArg::TukeyKramer)]
    pub test: PostHocTestArg,
    /// For the matched post-hoc test: the metadata field that says which
    /// samples came from the same subject
    #[arg(long, value_name = "FIELD")]
    pub pair_field: Option<String>,
    /// Nominal coverage of the confidence intervals
    #[arg(long, default_value_t = 0.95)]
    pub coverage: f64,
    #[command(flatten)]
    pub plot: PlotArgs,
}

#[derive(Args)]
pub struct ConvertArgs {
    #[command(subcommand)]
    pub format: ConvertFormat,
}

#[derive(Subcommand)]
pub enum ConvertFormat {
    /// MG-RAST functional or taxonomic abundance table
    Mgrast {
        /// MG-RAST table (.tsv)
        input: PathBuf,
        /// Heading for a hierarchical level, in order (repeat for each level)
        #[arg(long = "heading", value_name = "HEADING")]
        headings: Vec<String>,
        #[command(flatten)]
        output: OutputArg,
    },
    /// CoMet profiles, one file per sample
    Comet {
        /// CoMet result files
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[command(flatten)]
        output: OutputArg,
    },
    /// RITA classifications, one file per sample
    Rita {
        /// RITA result files
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Classification group to count (may be repeated) [default: all groups]
        #[arg(long = "group", value_name = "GROUP")]
        groups: Vec<String>,
        #[command(flatten)]
        output: OutputArg,
    },
    /// mothur taxonomy, groups and optional names files
    Mothur {
        /// Taxonomy file
        #[arg(long)]
        taxonomy: PathBuf,
        /// Groups file
        #[arg(long)]
        groups: PathBuf,
        /// Names file
        #[arg(long)]
        names: Option<PathBuf>,
        #[command(flatten)]
        output: OutputArg,
    },
    /// BIOM table (JSON or HDF5 format, optionally gzip-compressed)
    Biom {
        /// BIOM file
        input: PathBuf,
        /// Observation metadata field giving the hierarchy, such as taxonomy
        /// or KEGG_Pathways [default: observation ids only]
        #[arg(long, value_name = "FIELD")]
        metadata_field: Option<String>,
        #[command(flatten)]
        output: OutputArg,
    },
    /// SingleM condensed taxonomic profiles (`singlem pipe -p` or
    /// `singlem summarise`); each file may hold several samples. Values are
    /// coverages, in seven ranks from Domain to Species
    Singlem {
        /// SingleM condensed profile files
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[command(flatten)]
        output: OutputArg,
    },
    /// Sandpiper condensed profiles (JSON from the Sandpiper API), one file
    /// per sample. Values are coverages, in seven ranks from Domain to Species
    Sandpiper {
        /// Sandpiper JSON files
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[command(flatten)]
        output: OutputArg,
    },
    /// Add COG categories to an IMG/M COG profile
    Cog {
        /// IMG/M profile with COG ids, annotations and counts
        input: PathBuf,
        /// Give a COG with several category codes to each of its categories,
        /// instead of treating the combined code as its own category
        #[arg(long)]
        assign_to_each_code: bool,
        #[command(flatten)]
        output: OutputArg,
    },
}

#[derive(Args)]
pub struct OutputArg {
    /// Write the STAMP profile to this file instead of standard output
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

/// Options of the heatmaps: the group heatmap (`--plot-type group-heatmap`)
/// and, for the colours and order, the heatmap of samples.
#[derive(Args)]
pub struct HeatmapArgs {
    /// Group heatmap: what each cell shows, a summary of the feature's relative
    /// abundance in the group's samples
    #[arg(long, value_enum, default_value_t = GroupStatisticArg::Mean, value_name = "STATISTIC")]
    pub heatmap_statistic: GroupStatisticArg,
    /// Group heatmap, prevalence: a feature counts as present in a sample when
    /// its relative abundance is above zero and at least this percentage
    #[arg(long, default_value_t = 0.0, value_name = "PERCENT")]
    pub heatmap_min_abundance: f64,
    /// Group heatmap: show only this many features, those of highest mean
    /// abundance across all samples (0: every feature that passes the filters)
    #[arg(long, default_value_t = 50, value_name = "N")]
    pub heatmap_top: usize,
    /// Group heatmap: scale of the colours
    #[arg(long, value_enum, default_value_t = ScaleArg::Linear)]
    pub heatmap_scale: ScaleArg,
    /// Group heatmap, log10 scale: smallest value shown; smaller (non-zero)
    /// values take its colour
    #[arg(long, default_value_t = 0.001, value_name = "VALUE")]
    pub heatmap_log_floor: f64,
    /// Group heatmap: draw cells whose value is zero in the colour scale's
    /// lowest colour instead of grey
    #[arg(long)]
    pub heatmap_no_grey_zeros: bool,
    /// Group heatmap: write each cell's value in it
    #[arg(long)]
    pub heatmap_values: bool,
    /// Group heatmap: format of the values written in the cells
    #[arg(long, value_enum, default_value_t = NumberFormatArg::Decimals)]
    pub heatmap_value_format: NumberFormatArg,
    /// Group heatmap: decimals (or significant digits) of the values in the cells
    #[arg(long, default_value_t = 1, value_name = "N")]
    pub heatmap_value_digits: usize,
    /// Order of the rows (features)
    #[arg(long, value_enum, default_value_t = RowOrderArg::Cluster)]
    pub heatmap_row_order: RowOrderArg,
    /// Order of the columns (groups or samples); category orders groups by
    /// --heatmap-order-field [default: category with --heatmap-order-field,
    /// otherwise cluster]
    #[arg(long, value_enum)]
    pub heatmap_column_order: Option<ColumnOrderArg>,
    /// Group heatmap: metadata field whose value (the most common among a
    /// group's samples) is shown as a coloured strip above each group, and
    /// orders the groups with --heatmap-column-order category; for example
    /// Host_order for groups of Host_species
    #[arg(long, value_name = "FIELD")]
    pub heatmap_order_field: Option<String>,
    /// Linkage of the hierarchical clustering
    #[arg(long, value_enum, default_value_t = LinkageArg::Average)]
    pub heatmap_linkage: LinkageArg,
    /// Leave out the dendrograms of clustered rows and columns
    #[arg(long)]
    pub heatmap_no_dendrograms: bool,
    /// Colour scale of the cells
    #[arg(long, value_enum, value_name = "MAP")]
    pub heatmap_color_map: Option<ColorMapArg>,
    /// Group heatmap: width of each cell, in inches; the figure is sized to fit
    /// its cells unless --heatmap-fixed-size is given
    #[arg(long, default_value_t = 0.22, value_name = "INCHES")]
    pub heatmap_cell_width: f64,
    /// Group heatmap: height of each cell, in inches
    #[arg(long, default_value_t = 0.16, value_name = "INCHES")]
    pub heatmap_cell_height: f64,
    /// Group heatmap: use --plot-width and --plot-height for the figure
    /// instead of fitting it to its cells
    #[arg(long)]
    pub heatmap_fixed_size: bool,
    /// Group heatmap: also write the value of every cell (features by groups)
    /// to this tab-separated file
    #[arg(long, value_name = "FILE")]
    pub heatmap_table: Option<PathBuf>,
}

/// Options of the ordination plots (`--plot-type pcoa`, `nmds`):
/// what colours and shapes the samples.
#[derive(Args)]
pub struct OrdinationArgs {
    /// PCoA and NMDS plots: what colours the samples: group (the groups
    /// of the analysis), field:NAME (a metadata field; a colour scale if its
    /// values are numbers, else a colour per value), category:NAME (a colour
    /// per value even for numbers) or feature:NAME (the relative abundance,
    /// %, of a feature at the profile level)
    #[arg(long, default_value = "group", value_name = "SOURCE")]
    pub color_by: String,
    /// PCoA and NMDS plots: categorical metadata field that gives the
    /// samples their marker shapes [default: the groups]
    #[arg(long, value_name = "FIELD")]
    pub shape_by: Option<String>,
    /// Colouring by a number: linear or logarithmic (log10) colour scale;
    /// zeros are drawn at half the smallest positive value
    #[arg(long, value_enum, default_value_t = ColorScaleArg::Linear)]
    pub color_scale: ColorScaleArg,
    /// Colouring by a number: the colour map
    #[arg(long, value_enum, default_value_t = ColorMapArg::Viridis)]
    pub color_map: ColorMapArg,
    /// Colouring by a number: which samples are drawn on top
    #[arg(long, value_enum, default_value_t = DrawOrderArg::HighOnTop)]
    pub draw_order: DrawOrderArg,
    /// PCoA and NMDS plots: area of the markers, in square points
    #[arg(long, default_value_t = 30.0, value_name = "POINTS")]
    pub marker_size: f64,
    /// PCoA and NMDS plots: opacity of the markers, from 0 to 1
    #[arg(long, default_value_t = 1.0, value_name = "OPACITY")]
    pub opacity: f64,
}

/// Options of the map of sample locations (`--plot-type map`). Markers are
/// coloured and shaped by --color-by and --shape-by (with --color-scale,
/// --color-map, --draw-order, --marker-size and --opacity), as in the
/// ordination plots.
#[derive(Args)]
pub struct MapArgs {
    /// Map: metadata field holding each sample's latitude in decimal degrees
    /// [default: a field named latitude or lat]
    #[arg(long, value_name = "FIELD")]
    pub map_lat_field: Option<String>,
    /// Map: metadata field holding each sample's longitude in decimal degrees
    /// [default: a field named longitude, lon, long or lng]
    #[arg(long, value_name = "FIELD")]
    pub map_lon_field: Option<String>,
    /// Map: the area shown: auto (around the samples), world, europe, africa,
    /// asia, north-america, south-america, oceania, or WEST,EAST,SOUTH,NORTH
    /// in degrees (e.g. -10,30,35,60)
    #[arg(
        long,
        default_value = "auto",
        value_name = "EXTENT",
        allow_hyphen_values = true
    )]
    pub map_extent: String,
    /// Map: projection
    #[arg(long, value_enum, default_value_t = MapProjectionArg::EqualEarth)]
    pub map_projection: MapProjectionArg,
    /// Map: detail of the coastlines and borders: 110m for the world, 50m for
    /// regions [default: auto, by the width of the area]
    #[arg(long, value_enum, default_value_t = MapResolutionArg::Auto)]
    pub map_resolution: MapResolutionArg,
    /// Map: what sizes the markers: count (samples at the site), fixed,
    /// field:NAME (a numeric metadata field) or feature:NAME (the relative
    /// abundance, %, of a feature)
    #[arg(long, default_value = "count", value_name = "SOURCE")]
    pub map_size_by: String,
    /// Map: sizes from the logarithms of the numbers
    #[arg(long)]
    pub map_size_log: bool,
    /// Map: area of the largest markers, in square points (--marker-size is
    /// that of the smallest)
    #[arg(long, default_value_t = 400.0, value_name = "POINTS")]
    pub map_max_marker_size: f64,
    /// Map: one marker per site for the samples with the same colour and
    /// shape (the default)
    #[arg(long, overrides_with = "no_map_aggregate")]
    pub map_aggregate: bool,
    /// Map: one marker per sample
    #[arg(long)]
    pub no_map_aggregate: bool,
    /// Map: samples whose coordinates round to the same multiple of this many
    /// degrees are at the same site [default: about 3 points on the map,
    /// e.g. 2 degrees for the world]
    #[arg(long, value_name = "DEGREES")]
    pub map_site_precision: Option<f64>,
    /// Map: how the numbers of the samples at a site colour its marker
    #[arg(long, value_enum, default_value_t = MapSiteValueArg::Mean)]
    pub map_site_value: MapSiteValueArg,
    /// Map, one marker per sample: spread the samples at a site around it
    #[arg(long)]
    pub map_jitter: bool,
    /// Map: draw lines of longitude and latitude (the default)
    #[arg(long, overrides_with = "no_map_graticule")]
    pub map_graticule: bool,
    /// Map: no lines of longitude and latitude
    #[arg(long)]
    pub no_map_graticule: bool,
    /// Map: leave the sea white
    #[arg(long)]
    pub no_map_ocean: bool,
    /// Map: no outlines of countries and coasts (the land is still filled)
    #[arg(long)]
    pub no_map_borders: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum MapProjectionArg {
    /// Equal Earth: true areas
    EqualEarth,
    Robinson,
    /// Longitude and latitude as x and y
    Equirectangular,
    /// Mercator, to 85° north and south
    Mercator,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum MapResolutionArg {
    Auto,
    #[value(name = "110m")]
    Low,
    #[value(name = "50m")]
    Medium,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum MapSiteValueArg {
    Mean,
    #[value(alias = "maximum")]
    Max,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum ColorScaleArg {
    Linear,
    Log10,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum DrawOrderArg {
    /// Highest values drawn last, on top
    HighOnTop,
    /// Lowest values drawn last, on top
    LowOnTop,
    /// In the order of the samples
    SampleOrder,
}

/// Options of the box plot (`--plot-type box`) and the bar plot of one
/// feature.
#[derive(Args)]
pub struct BoxArgs {
    /// Box plot without --plot-feature: show the N most significant features
    #[arg(long, default_value_t = 1, value_name = "N")]
    pub box_top: usize,
    /// Box plot of several features: a box per group at each feature, or one
    /// panel per feature
    #[arg(long, value_enum, default_value_t = BoxLayoutArg::Grouped)]
    pub box_layout: BoxLayoutArg,
    /// Box plot with one panel per feature: give each panel its own value axis
    #[arg(long)]
    pub box_independent_axes: bool,
    /// Box plot: logarithmic (log10) value axis; zeros are drawn on a line
    /// labelled 0 below a break in the axis
    #[arg(long)]
    pub box_log: bool,
    /// Box plot: draw every sample as a small semi-transparent point
    #[arg(long)]
    pub box_points: bool,
    /// Box plot: write the number of samples after each group's name
    #[arg(long)]
    pub box_counts: bool,
    /// Box plot: horizontal boxes (groups or features down the side)
    #[arg(long)]
    pub box_horizontal: bool,
    /// Box plot of a paired test (two-groups with --pair-field): join the
    /// two samples of each pair with a line between the boxes, coloured by
    /// the group in which the value is higher; several features get a panel
    /// each
    #[arg(long)]
    pub box_pairs: bool,
    /// Box plot: order of the groups [default: field with --box-order-field,
    /// otherwise given]
    #[arg(long, value_enum)]
    pub box_group_order: Option<GroupOrderArg>,
    /// Box plot: metadata field whose value (the most common among a group's
    /// samples) orders the groups, for example Host_order for groups of hosts
    #[arg(long, value_name = "FIELD")]
    pub box_order_field: Option<String>,
    /// Bar plot of one feature in every sample: logarithmic (log10) value axis
    #[arg(long)]
    pub bar_log: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum BoxLayoutArg {
    /// Features along the axis, a box per group (coloured) at each
    Grouped,
    /// One panel per feature, the groups along the axis of each
    Panels,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum GroupOrderArg {
    /// As in the analysis (--group order, or the order in the metadata)
    Given,
    Alphabetical,
    /// Highest median (of the first feature) first
    Median,
    /// By --box-order-field, then by name
    Field,
}

/// Options of the prevalence-abundance plot (`--plot-type
/// prevalence-abundance`).
#[derive(Args)]
pub struct PrevalenceArgs {
    /// Prevalence-abundance plot: one panel per group, every group in one
    /// panel, or each feature once at the group where it is most prevalent
    #[arg(long, value_enum, default_value_t = PrevalenceViewArg::Panels)]
    pub prevalence_view: PrevalenceViewArg,
    /// Prevalence-abundance plot, combined view: show only this group
    #[arg(long, value_name = "GROUP")]
    pub prevalence_group: Option<String>,
    /// Prevalence-abundance plot: a feature counts as present in a sample when
    /// its relative abundance is above zero and at least this percentage
    #[arg(long, default_value_t = 0.0, value_name = "PERCENT")]
    pub prevalence_min_abundance: f64,
    /// Prevalence-abundance plot: take the mean abundance over all samples of
    /// the group or only over those where the feature is present
    #[arg(long, value_enum, default_value_t = MeanOverArg::All)]
    pub prevalence_mean: MeanOverArg,
    /// Prevalence-abundance plot, specificity view: colour a feature by its
    /// group when its prevalence there exceeds its prevalence in every other
    /// group by at least this many percentage points
    #[arg(long, default_value_t = 50.0, value_name = "POINTS")]
    pub prevalence_specificity: f64,
    /// Prevalence-abundance plot: only the features that pass the filters
    /// [default: every feature of the profile]
    #[arg(long)]
    pub prevalence_only_filtered: bool,
    /// Prevalence-abundance plot: also write the prevalence and mean
    /// abundance of every feature in every group to this tab-separated file
    #[arg(long, value_name = "FILE")]
    pub prevalence_table: Option<PathBuf>,
    /// Prevalence-abundance plot: draw this feature in red and name it (may be
    /// repeated)
    #[arg(long, value_name = "FEATURE")]
    pub highlight: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum PrevalenceViewArg {
    /// One panel per group, on the same axes
    Panels,
    /// Every group (or --prevalence-group) in one panel, coloured by group
    Combined,
    /// Each feature once, at the group where it is most prevalent, coloured
    /// by that group if it is specific to it and grey if shared
    Specificity,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum MeanOverArg {
    /// Every sample of the group (absent counts as zero)
    All,
    /// Only the samples where the feature is present
    Present,
}

#[derive(Args)]
pub struct CoOccurrenceArgs {
    #[command(flatten)]
    pub profile: ProfileArgs,
    /// Metadata file describing the samples; needed for --where, --where-not,
    /// --exclude-sample and --group
    #[arg(long, short)]
    pub metadata: Option<PathBuf>,
    /// Metadata field that defines the groups, for --group
    #[arg(long, short)]
    pub field: Option<String>,
    /// Correlate only within this group of --field [default: all samples]
    #[arg(long, value_name = "GROUP")]
    pub group: Option<String>,
    /// Sample to leave out (may be repeated)
    #[arg(long = "exclude-sample", value_name = "SAMPLE")]
    pub excluded_samples: Vec<String>,
    #[command(flatten)]
    pub sample_filter: SampleFilterArgs,
    /// Measure of correlation
    #[arg(long, value_enum, default_value_t = CorrelationMethodArg::Spearman)]
    pub method: CorrelationMethodArg,
    /// Correlate centred log ratios instead of proportions, which removes the
    /// bias of proportions that must sum to 100%
    #[arg(long)]
    pub clr: bool,
    /// Number of features correlated, those ranked first by --rank-by
    #[arg(long, default_value_t = 30, value_name = "N")]
    pub top: usize,
    /// How features are ranked for --top
    #[arg(long, value_enum, default_value_t = RankingArg::MeanAbundance)]
    pub rank_by: RankingArg,
    /// Feature to include whatever its rank (may be repeated); with --top 0,
    /// only these features are correlated
    #[arg(long = "feature", value_name = "FEATURE")]
    pub features: Vec<String>,
    /// Instead of every pair of features, correlate this feature with every
    /// other feature of the profile; the plot shows the strongest correlations
    #[arg(long, value_name = "FEATURE")]
    pub with: Option<String>,
    /// With --with: plot that feature against this one, a point for each
    /// sample, with their correlation, instead of the strongest correlations
    #[arg(long, value_name = "FEATURE")]
    pub against: Option<String>,
    /// With --with: number of correlations shown in the plot
    #[arg(long, default_value_t = 30, value_name = "N")]
    pub partners: usize,
    /// Mark correlations with a Benjamini-Hochberg q-value below this level
    #[arg(long, default_value_t = 0.05, value_name = "Q")]
    pub q_threshold: f64,
    /// Mark only correlations at least this far from zero (with a large number
    /// of samples, weak correlations are significant too)
    #[arg(long, default_value_t = 0.0, value_name = "R")]
    pub mark_min_coefficient: f64,
    /// Do not mark significant correlations in the plot
    #[arg(long)]
    pub no_marks: bool,
    /// Leave out features present in fewer than this percentage of the
    /// samples (unless named with --feature or --with)
    #[arg(long, default_value_t = 0.0, value_name = "PERCENT")]
    pub min_feature_prevalence: f64,
    /// Write each correlation in its cell of the heatmap
    #[arg(long)]
    pub show_values: bool,
    /// Order of the features in the heatmap
    #[arg(long, value_enum, default_value_t = CoOrderArg::Cluster)]
    pub order: CoOrderArg,
    /// Leave out the dendrograms
    #[arg(long)]
    pub no_dendrograms: bool,
    /// Size of each cell of the heatmap (or height of each bar), in inches;
    /// the figure is sized to fit unless --fixed-size is given
    #[arg(long, default_value_t = 0.16, value_name = "INCHES")]
    pub cell_size: f64,
    /// Use --plot-width and --plot-height for the figure instead of fitting
    /// it to its cells
    #[arg(long)]
    pub fixed_size: bool,
    /// Colour scale of the cells, run from +1 (first colour) to -1
    #[arg(long, value_enum, default_value_t = ColorMapArg::RedToBlue, value_name = "MAP")]
    pub color_map: ColorMapArg,
    #[command(flatten)]
    pub plot: PlotArgs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum GroupStatisticArg {
    /// Mean relative abundance (%)
    Mean,
    /// Median relative abundance (%)
    Median,
    /// Largest relative abundance (%) in any sample of the group
    Max,
    /// Percentage of the group's samples in which the feature is present
    Prevalence,
}

impl From<GroupStatisticArg> for stamp_core::stats::group_summary::GroupStatistic {
    fn from(value: GroupStatisticArg) -> Self {
        use stamp_core::stats::group_summary::GroupStatistic;
        match value {
            GroupStatisticArg::Mean => GroupStatistic::Mean,
            GroupStatisticArg::Median => GroupStatistic::Median,
            GroupStatisticArg::Max => GroupStatistic::Maximum,
            GroupStatisticArg::Prevalence => GroupStatistic::Prevalence,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum ScaleArg {
    Linear,
    Log10,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum NumberFormatArg {
    /// A fixed number of decimals
    Decimals,
    /// A fixed number of significant digits
    Significant,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum RowOrderArg {
    /// Hierarchical clustering (see --heatmap-linkage)
    Cluster,
    /// Most abundant first
    MeanAbundance,
    Alphabetical,
    /// As in the profile
    Original,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum ColumnOrderArg {
    /// Hierarchical clustering (see --heatmap-linkage)
    Cluster,
    /// Group heatmap: by --heatmap-order-field, then by name
    Category,
    Alphabetical,
    /// As in the analysis
    Original,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum CoOrderArg {
    /// Hierarchical clustering of the correlations
    Cluster,
    /// As chosen: by rank, then the features given with --feature
    Original,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum RankingArg {
    MeanAbundance,
    /// Number of samples in which the feature is present
    Prevalence,
}

impl From<RankingArg> for stamp_core::stats::co_occurrence::FeatureRanking {
    fn from(value: RankingArg) -> Self {
        use stamp_core::stats::co_occurrence::FeatureRanking;
        match value {
            RankingArg::MeanAbundance => FeatureRanking::MeanAbundance,
            RankingArg::Prevalence => FeatureRanking::Prevalence,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum LinkageArg {
    /// Mean distance between members (UPGMA)
    Average,
    Centroid,
    /// Distance between the closest members
    Nearest,
    /// Distance between the furthest members
    Furthest,
    Ward,
}

impl From<LinkageArg> for stamp_core::stats::cluster::Linkage {
    fn from(value: LinkageArg) -> Self {
        use stamp_core::stats::cluster::Linkage;
        match value {
            LinkageArg::Average => Linkage::Average,
            LinkageArg::Centroid => Linkage::Centroid,
            LinkageArg::Nearest => Linkage::Nearest,
            LinkageArg::Furthest => Linkage::Furthest,
            LinkageArg::Ward => Linkage::Ward,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum ColorMapArg {
    Blues,
    BlueToRedToGreen,
    BlueToWhiteToRed,
    Cividis,
    CoolToWarm,
    Grayscale,
    Jet,
    Magma,
    OrangeToRed,
    Paired,
    Plasma,
    PurpleToGreen,
    Reds,
    RedToBlue,
    RedToYellowToBlue,
    Spectral,
    Viridis,
    YellowToOrangeToRed,
}

impl From<ColorMapArg> for stamp_plot::heatmap::ColorMap {
    fn from(value: ColorMapArg) -> Self {
        use stamp_plot::heatmap::ColorMap;
        match value {
            ColorMapArg::Blues => ColorMap::Blues,
            ColorMapArg::BlueToRedToGreen => ColorMap::BlueToRedToGreen,
            ColorMapArg::BlueToWhiteToRed => ColorMap::BlueToWhiteToRed,
            ColorMapArg::CoolToWarm => ColorMap::CoolToWarm,
            ColorMapArg::Grayscale => ColorMap::Grayscale,
            ColorMapArg::Jet => ColorMap::Jet,
            ColorMapArg::OrangeToRed => ColorMap::OrangeToRed,
            ColorMapArg::Paired => ColorMap::Paired,
            ColorMapArg::PurpleToGreen => ColorMap::PurpleToGreen,
            ColorMapArg::Reds => ColorMap::Reds,
            ColorMapArg::RedToBlue => ColorMap::RedToBlue,
            ColorMapArg::RedToYellowToBlue => ColorMap::RedToYellowToBlue,
            ColorMapArg::Spectral => ColorMap::Spectral,
            ColorMapArg::YellowToOrangeToRed => ColorMap::YellowToOrangeToRed,
            ColorMapArg::Viridis => ColorMap::Viridis,
            ColorMapArg::Plasma => ColorMap::Plasma,
            ColorMapArg::Magma => ColorMap::Magma,
            ColorMapArg::Cividis => ColorMap::Cividis,
        }
    }
}

// ----------------------------------------------------------------- choices

#[derive(Clone, Copy, ValueEnum)]
pub enum UnclassifiedArg {
    /// Treat unclassified reads like any other feature
    Retain,
    /// Drop unclassified reads entirely
    Remove,
    /// Drop them as features but keep them in the totals
    FrequencyOnly,
}

impl From<UnclassifiedArg> for Unclassified {
    fn from(value: UnclassifiedArg) -> Self {
        match value {
            UnclassifiedArg::Retain => Unclassified::Retain,
            UnclassifiedArg::Remove => Unclassified::Remove,
            UnclassifiedArg::FrequencyOnly => Unclassified::FrequencyOnly,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum TestTypeArg {
    OneSided,
    TwoSided,
}

impl From<TestTypeArg> for TestType {
    fn from(value: TestTypeArg) -> Self {
        match value {
            TestTypeArg::OneSided => TestType::OneSided,
            TestTypeArg::TwoSided => TestType::TwoSided,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum SampleTestArg {
    Bootstrap,
    ChiSquare,
    ChiSquareYates,
    DiffBetweenProp,
    Fisher,
    GTest,
    GTestYates,
    GTestFisher,
    Hypergeometric,
    Permutation,
}

impl From<SampleTestArg> for SampleTest {
    fn from(value: SampleTestArg) -> Self {
        match value {
            SampleTestArg::Bootstrap => SampleTest::Bootstrap,
            SampleTestArg::ChiSquare => SampleTest::ChiSquare,
            SampleTestArg::ChiSquareYates => SampleTest::ChiSquareYates,
            SampleTestArg::DiffBetweenProp => SampleTest::DiffBetweenProp,
            SampleTestArg::Fisher => SampleTest::Fisher,
            SampleTestArg::GTest => SampleTest::GTest,
            SampleTestArg::GTestYates => SampleTest::GTestYates,
            SampleTestArg::GTestFisher => SampleTest::GTestFisher,
            SampleTestArg::Hypergeometric => SampleTest::Hypergeometric,
            SampleTestArg::Permutation => SampleTest::Permutation,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum SampleCiArg {
    /// Difference between proportions, asymptotic
    DpAsymptotic,
    /// Difference between proportions, asymptotic with continuity correction
    DpAsymptoticCc,
    /// Difference between proportions, Newcombe-Wilson
    DpNewcombeWilson,
    /// Odds ratio with Haldane adjustment
    OddsRatio,
    /// Ratio of proportions, asymptotic
    RatioProportions,
}

impl From<SampleCiArg> for SampleCi {
    fn from(value: SampleCiArg) -> Self {
        match value {
            SampleCiArg::DpAsymptotic => SampleCi::DiffBetweenPropAsymptotic,
            SampleCiArg::DpAsymptoticCc => SampleCi::DiffBetweenPropAsymptoticCc,
            SampleCiArg::DpNewcombeWilson => SampleCi::NewcombeWilson,
            SampleCiArg::OddsRatio => SampleCi::OddsRatio,
            SampleCiArg::RatioProportions => SampleCi::RatioProportions,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CorrectionArg {
    None,
    BenjaminiHochberg,
    Bonferroni,
    HolmBonferroni,
    Sidak,
    Storey,
}

impl From<CorrectionArg> for Correction {
    fn from(value: CorrectionArg) -> Self {
        match value {
            CorrectionArg::None => Correction::None,
            CorrectionArg::BenjaminiHochberg => Correction::BenjaminiHochberg,
            CorrectionArg::Bonferroni => Correction::Bonferroni,
            CorrectionArg::HolmBonferroni => Correction::HolmBonferroni,
            CorrectionArg::Sidak => Correction::Sidak,
            CorrectionArg::Storey => Correction::Storey,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum TransformArg {
    /// Proportions as they are
    None,
    /// Centred log-ratio of the features of each sample
    Clr,
}

impl From<TransformArg> for Transform {
    fn from(value: TransformArg) -> Self {
        match value {
            TransformArg::None => Transform::None,
            TransformArg::Clr => Transform::CentredLogRatio,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CorrelationMethodArg {
    /// Pearson's r
    Pearson,
    /// Spearman's rank correlation
    Spearman,
}

impl From<CorrelationMethodArg> for CorrelationMethod {
    fn from(value: CorrelationMethodArg) -> Self {
        match value {
            CorrelationMethodArg::Pearson => CorrelationMethod::Pearson,
            CorrelationMethodArg::Spearman => CorrelationMethod::Spearman,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum GroupTestArg {
    /// Fisher's exact test of whether the feature is present in the same
    /// share of the samples of both groups
    FisherPresence,
    /// McNemar's exact test of presence or absence, for paired samples
    /// (needs --pair-field)
    Mcnemar,
    /// Mann-Whitney U test (Wilcoxon rank-sum test)
    MannWhitney,
    /// Paired t-test (needs --pair-field)
    PairedT,
    /// Wilcoxon signed-rank test, for paired samples (needs --pair-field)
    Wilcoxon,
    /// t-test assuming equal variances
    TTest,
    /// Welch's t-test
    Welch,
    /// White's non-parametric t-test
    White,
}

impl From<GroupTestArg> for GroupTest {
    fn from(value: GroupTestArg) -> Self {
        match value {
            GroupTestArg::FisherPresence => GroupTest::FisherPresence,
            GroupTestArg::Mcnemar => GroupTest::McNemar,
            GroupTestArg::MannWhitney => GroupTest::MannWhitney,
            GroupTestArg::PairedT => GroupTest::PairedT,
            GroupTestArg::Wilcoxon => GroupTest::WilcoxonSignedRank,
            GroupTestArg::TTest => GroupTest::TTest,
            GroupTestArg::Welch => GroupTest::Welch,
            GroupTestArg::White => GroupTest::White,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum MultiGroupTestArg {
    Anova,
    /// Chi-square test of whether the feature is present in the same share
    /// of the samples of every group
    ChiSquarePresence,
    KruskalWallis,
    /// For matched groups: the same subjects sampled once in every group
    /// (needs --pair-field)
    Friedman,
}

impl From<MultiGroupTestArg> for MultiGroupTest {
    fn from(value: MultiGroupTestArg) -> Self {
        match value {
            MultiGroupTestArg::Anova => MultiGroupTest::Anova,
            MultiGroupTestArg::KruskalWallis => MultiGroupTest::KruskalWallis,
            MultiGroupTestArg::Friedman => MultiGroupTest::Friedman,
            MultiGroupTestArg::ChiSquarePresence => MultiGroupTest::ChiSquarePresence,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum PostHocTestArg {
    /// Dunn's test with the Bonferroni adjustment
    Dunn,
    GamesHowell,
    Scheffe,
    TukeyKramer,
    /// Welch's t-test for each pair, without correction
    Welch,
    /// For matched groups: the Wilcoxon signed-rank test for each pair, with
    /// Holm's correction (needs --pair-field)
    Wilcoxon,
}

impl From<PostHocTestArg> for PostHocTest {
    fn from(value: PostHocTestArg) -> Self {
        match value {
            PostHocTestArg::Dunn => PostHocTest::Dunn,
            PostHocTestArg::GamesHowell => PostHocTest::GamesHowell,
            PostHocTestArg::Scheffe => PostHocTest::Scheffe,
            PostHocTestArg::TukeyKramer => PostHocTest::TukeyKramer,
            PostHocTestArg::Welch => PostHocTest::WelchUncorrected,
            PostHocTestArg::Wilcoxon => PostHocTest::WilcoxonHolm,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OperatorArg {
    And,
    Or,
}

impl From<OperatorArg> for FilterOperator {
    fn from(value: OperatorArg) -> Self {
        match value {
            OperatorArg::And => FilterOperator::And,
            OperatorArg::Or => FilterOperator::Or,
        }
    }
}

/// Parse `NAME=VALUE`.
fn name_and_value(text: &str) -> Result<(&str, &str), String> {
    text.split_once('=')
        .ok_or_else(|| format!("'{text}' is not of the form NAME=VALUE"))
}

fn numbers(text: &str, expected: usize) -> Result<Vec<f64>, String> {
    let values: Result<Vec<f64>, _> = text.split(',').map(|v| v.trim().parse::<f64>()).collect();
    match values {
        Ok(v) if v.len() == expected => Ok(v),
        _ => Err(format!(
            "'{text}' should be {expected} number{}",
            if expected == 1 {
                ""
            } else {
                "s separated by a comma"
            }
        )),
    }
}

/// A sequence-count filter given as `KIND=N` or `KIND=N1,N2`.
pub fn parse_count_filter(text: &str) -> Result<(String, Vec<f64>), String> {
    let (kind, value) = name_and_value(text)?;
    let expected = if kind.starts_with("independent") {
        2
    } else {
        1
    };
    Ok((kind.to_string(), numbers(value, expected)?))
}

pub fn parse_sample_effect_size(text: &str) -> Result<(SampleEffectSize, f64), String> {
    let (name, value) = name_and_value(text)?;
    let measure = match name {
        "diff-between-prop" => SampleEffectSize::DiffBetweenProp,
        "odds-ratio" => SampleEffectSize::OddsRatio,
        "ratio-proportions" => SampleEffectSize::RatioProportions,
        other => return Err(format!("unknown effect-size measure '{other}'")),
    };
    Ok((measure, numbers(value, 1)?[0]))
}

pub fn parse_group_effect_size(text: &str) -> Result<(GroupEffectSize, f64), String> {
    let (name, value) = name_and_value(text)?;
    let measure = match name {
        "diff-between-prop" => GroupEffectSize::DiffBetweenProp,
        "ratio-proportions" => GroupEffectSize::RatioProportions,
        other => return Err(format!("unknown effect-size measure '{other}'")),
    };
    Ok((measure, numbers(value, 1)?[0]))
}
