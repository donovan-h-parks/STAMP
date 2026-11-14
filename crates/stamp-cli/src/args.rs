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
    /// Feature shown by the bar and box plots [default: the most significant]
    #[arg(long, value_name = "FEATURE")]
    pub plot_feature: Option<String>,
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
    /// Corrected p-values
    PValueHistogram,
    /// Three views of the multiple-test correction
    MultipleComparison,
    /// Samples on their principal components
    Pca,
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
    /// Diversity within the samples of each group
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
    /// Seed of the random number generator (used by the Storey correction)
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    #[command(flatten)]
    pub plot: PlotArgs,
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
    KruskalWallis,
}

impl From<MultiGroupTestArg> for MultiGroupTest {
    fn from(value: MultiGroupTestArg) -> Self {
        match value {
            MultiGroupTestArg::Anova => MultiGroupTest::Anova,
            MultiGroupTestArg::KruskalWallis => MultiGroupTest::KruskalWallis,
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
}

impl From<PostHocTestArg> for PostHocTest {
    fn from(value: PostHocTestArg) -> Self {
        match value {
            PostHocTestArg::Dunn => PostHocTest::Dunn,
            PostHocTestArg::GamesHowell => PostHocTest::GamesHowell,
            PostHocTestArg::Scheffe => PostHocTest::Scheffe,
            PostHocTestArg::TukeyKramer => PostHocTest::TukeyKramer,
            PostHocTestArg::Welch => PostHocTest::WelchUncorrected,
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
