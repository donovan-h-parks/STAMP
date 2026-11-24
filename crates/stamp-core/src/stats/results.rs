//! Running a test over every feature of a profile, correcting for multiple
//! comparisons, and filtering the results.

use std::collections::HashSet;

use super::ci::SampleCi;
use super::correction::{Corrected, Correction};
use super::correlation::{correlation_p_value, CorrelationMethod};
use super::diversity::centred_log_ratio;
use super::effect_size::{GroupEffectSize, SampleEffectSize};
use super::multi_group::{MultiGroupEffectSize, MultiGroupTest};
use super::post_hoc::{PostHocResult, PostHocTest};
use super::two_group::{FeatureCounts, GroupOutcome, GroupTest};
use super::two_sample::SampleTest;
use super::{mean, population_std, StampRng, StatsSettings, TestType};
use crate::error::{Result, StampError};
use crate::profile::{CorrelationProfile, GroupProfile, MultiGroupProfile, Profile, SampleProfile};

/// Callback reporting progress as `(done, total)`. Returning `false` cancels
/// the run.
pub type Progress<'a> = &'a mut dyn FnMut(usize, usize) -> bool;

/// What a run should do before its next piece of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Continue,
    /// Stop for now; the run can be advanced again later.
    Pause,
    /// Give up; the run is of no further use.
    Cancel,
}

/// How far a run got when it was last advanced.
#[derive(Debug, Clone, PartialEq)]
pub enum Step<T> {
    Paused,
    Cancelled,
    Done(T),
}

/// Callback asked before each piece of work, with `(done, total)`.
pub type Pace<'a> = &'a mut dyn FnMut(usize, usize) -> Control;

/// A `Pace` that follows a progress callback: it never pauses, and cancels
/// when the callback returns `false`.
fn pace_of(progress: Progress<'_>) -> impl FnMut(usize, usize) -> Control + '_ {
    move |done, total| {
        if progress(done, total) {
            Control::Continue
        } else {
            Control::Cancel
        }
    }
}

fn no_correction() -> CorrectionSummary {
    CorrectionSummary {
        method: Correction::None,
        info: Vec::new(),
        warning: None,
    }
}

/// Summary of the multiple-test correction applied to a set of results.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrectionSummary {
    pub method: Correction,
    pub info: Vec<(&'static str, f64)>,
    pub warning: Option<&'static str>,
}

fn summarize(method: Correction, corrected: &Corrected) -> CorrectionSummary {
    CorrectionSummary {
        method,
        info: corrected.info.clone(),
        warning: corrected.warning,
    }
}

/// Order rows by p-value, then by feature name so that ties have a defined
/// order.
fn sort_rows<T>(rows: &mut [T], p: impl Fn(&T) -> f64, name: impl Fn(&T) -> &str) {
    rows.sort_by(|a, b| p(a).total_cmp(&p(b)).then_with(|| name(a).cmp(name(b))));
}

fn join_notes(first: &str, second: &str) -> String {
    match (first.is_empty(), second.is_empty()) {
        (true, _) => second.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first}; {second}"),
    }
}

/// How two effect-size filters combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    /// Remove a feature only if it falls below both thresholds.
    And,
    /// Remove a feature if it falls below either threshold.
    Or,
}

/// Whether a feature passes up to two effect-size thresholds. `first` and
/// `second` are `(effect size, minimum)` for the enabled filters.
fn passes_effect_size(
    first: Option<(f64, f64)>,
    second: Option<(f64, f64)>,
    operator: FilterOperator,
) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some((value, minimum)), None) | (None, Some((value, minimum))) => value >= minimum,
        (Some((v1, m1)), Some((v2, m2))) => match operator {
            FilterOperator::And => v1 >= m1 || v2 >= m2,
            FilterOperator::Or => v1 >= m1 && v2 >= m2,
        },
    }
}

// ------------------------------------------------------------- two samples

/// Settings of a two-sample analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleAnalysis {
    pub test: SampleTest,
    pub test_type: TestType,
    pub ci_method: SampleCi,
    /// Nominal coverage of the confidence interval, for example 0.95.
    pub coverage: f64,
    pub correction: Correction,
}

/// Statistics of one feature in a two-sample analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleRow {
    pub feature: String,
    pub seq1: f64,
    pub seq2: f64,
    pub parent_seq1: f64,
    pub parent_seq2: f64,
    /// Relative frequency in each sample, in percent.
    pub rel_freq1: f64,
    pub rel_freq2: f64,
    pub p_value: f64,
    pub p_value_corrected: f64,
    pub effect_size: f64,
    pub lower_ci: f64,
    pub upper_ci: f64,
    pub note: String,
}

/// Results of a two-sample analysis, ordered by p-value.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleResults {
    pub analysis: SampleAnalysis,
    pub rows: Vec<SampleRow>,
    pub correction: CorrectionSummary,
}

/// Sequence-count filter for two samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleCountFilter {
    /// Keep features where the larger of the two counts reaches the minimum.
    Maximum(f64),
    /// Keep features where both counts reach the minimum.
    Minimum(f64),
    /// Keep features where each sample reaches its own minimum.
    Independent(f64, f64),
}

impl SampleCountFilter {
    fn passes(self, count1: f64, count2: f64) -> bool {
        match self {
            SampleCountFilter::Maximum(minimum) => count1.max(count2) >= minimum,
            SampleCountFilter::Minimum(minimum) => count1.min(count2) >= minimum,
            SampleCountFilter::Independent(min1, min2) => count1 >= min1 && count2 >= min2,
        }
    }
}

/// Filters applied to two-sample results. A disabled filter is `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleFilter {
    /// Only these features may pass; `None` allows every feature.
    pub selected_features: Option<HashSet<String>>,
    /// Remove features with a corrected p-value above this level.
    pub significance: Option<f64>,
    pub sequences: Option<SampleCountFilter>,
    pub parent_sequences: Option<SampleCountFilter>,
    /// Remove features with an effect size below the given minimum.
    pub effect_size1: Option<(SampleEffectSize, f64)>,
    pub effect_size2: Option<(SampleEffectSize, f64)>,
    pub operator: FilterOperator,
}

impl Default for SampleFilter {
    fn default() -> Self {
        SampleFilter {
            selected_features: None,
            significance: None,
            sequences: None,
            parent_sequences: None,
            effect_size1: None,
            effect_size2: None,
            operator: FilterOperator::Or,
        }
    }
}

/// A two-sample analysis that can be carried out a few features at a time,
/// so that whatever runs it can attend to other things in between. The
/// results do not depend on where it pauses.
#[derive(Debug, Clone)]
pub struct SampleRun {
    analysis: SampleAnalysis,
    settings: StatsSettings,
    rows: Vec<SampleRow>,
    /// The random numbers as they were before the first feature, for the
    /// multiple-test correction: Storey's bootstrap then draws the same
    /// numbers whether the correction is chosen before the run or applied
    /// to its results afterwards, and whichever test ran.
    correction_rng: Option<StampRng>,
}

impl SampleRun {
    pub fn new(analysis: SampleAnalysis, settings: &StatsSettings) -> SampleRun {
        SampleRun {
            analysis,
            settings: *settings,
            rows: Vec::new(),
            correction_rng: None,
        }
    }

    /// Test further features of `profile`, which with `rng` must be the same
    /// on every call. `pace` is asked before each feature.
    pub fn advance(
        &mut self,
        profile: &SampleProfile,
        rng: &mut StampRng,
        pace: Pace<'_>,
    ) -> Step<SampleResults> {
        if self.correction_rng.is_none() {
            self.correction_rng = Some(rng.clone());
        }
        let (analysis, settings) = (self.analysis, &self.settings);
        let features = profile.profile.features();
        while self.rows.len() < features.len() {
            match pace(self.rows.len(), features.len()) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            let feature = &features[self.rows.len()];
            let c = profile.counts(feature);
            let test =
                analysis
                    .test
                    .run(c.seq1, c.seq2, c.parent_seq1, c.parent_seq2, settings, rng);
            let p_value = match analysis.test_type {
                TestType::OneSided => test.one_sided,
                TestType::TwoSided => test.two_sided,
            };
            let ci = analysis.ci_method.run(
                c.seq1,
                c.seq2,
                c.parent_seq1,
                c.parent_seq2,
                analysis.coverage,
                settings.pseudocount,
            );
            let (rel_freq1, rel_freq2) = c.percentages();
            self.rows.push(SampleRow {
                feature: feature.name.clone(),
                seq1: c.seq1,
                seq2: c.seq2,
                parent_seq1: c.parent_seq1,
                parent_seq2: c.parent_seq2,
                rel_freq1,
                rel_freq2,
                p_value,
                p_value_corrected: p_value,
                effect_size: ci.effect_size,
                lower_ci: ci.lower,
                upper_ci: ci.upper,
                note: join_notes(test.note, ci.note),
            });
        }
        let mut rows = std::mem::take(&mut self.rows);
        sort_rows(&mut rows, |r| r.p_value, |r| &r.feature);
        let mut results = SampleResults {
            analysis,
            rows,
            correction: no_correction(),
        };
        let mut correction_rng = self.correction_rng.take().unwrap_or_else(|| rng.clone());
        results.apply_correction(analysis.correction, &mut correction_rng);
        Step::Done(results)
    }
}

/// Run a two-sample test on every feature of a profile. Returns `None` if
/// cancelled through `progress`.
pub fn run_sample_analysis(
    profile: &SampleProfile,
    analysis: SampleAnalysis,
    settings: &StatsSettings,
    rng: &mut StampRng,
    progress: Progress<'_>,
) -> Option<SampleResults> {
    match SampleRun::new(analysis, settings).advance(profile, rng, &mut pace_of(progress)) {
        Step::Done(results) => Some(results),
        Step::Paused | Step::Cancelled => None,
    }
}

impl SampleResults {
    /// Significance level implied by the coverage of the confidence interval.
    pub fn alpha(&self) -> f64 {
        1.0 - self.analysis.coverage
    }

    /// Recompute the corrected p-values with another method.
    pub fn apply_correction(&mut self, method: Correction, rng: &mut StampRng) {
        let p_values: Vec<f64> = self.rows.iter().map(|r| r.p_value).collect();
        let corrected = method.correct(&p_values, self.alpha(), rng);
        for (row, value) in self.rows.iter_mut().zip(&corrected.values) {
            row.p_value_corrected = *value;
        }
        self.analysis.correction = method;
        self.correction = summarize(method, &corrected);
    }

    /// Rows that pass the filters, in order.
    pub fn filter(&self, filter: &SampleFilter, settings: &StatsSettings) -> Vec<&SampleRow> {
        let effect = |f: Option<(SampleEffectSize, f64)>, row: &SampleRow| {
            f.map(|(measure, minimum)| {
                let value = measure.magnitude(
                    row.seq1,
                    row.seq2,
                    row.parent_seq1,
                    row.parent_seq2,
                    settings.pseudocount,
                );
                (value, minimum)
            })
        };
        self.rows
            .iter()
            .filter(|row| {
                filter
                    .selected_features
                    .as_ref()
                    .is_none_or(|s| s.contains(&row.feature))
                    && filter
                        .significance
                        .is_none_or(|level| row.p_value_corrected <= level)
                    && filter
                        .sequences
                        .is_none_or(|f| f.passes(row.seq1, row.seq2))
                    && filter
                        .parent_sequences
                        .is_none_or(|f| f.passes(row.parent_seq1, row.parent_seq2))
                    && passes_effect_size(
                        effect(filter.effect_size1, row),
                        effect(filter.effect_size2, row),
                        filter.operator,
                    )
            })
            .collect()
    }
}

// -------------------------------------------------------------- two groups

/// What the values of a feature are turned into before a test between
/// groups is applied to them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Transform {
    /// The proportions as they are.
    #[default]
    None,
    /// The centred log-ratio: the logarithm of each feature relative to the
    /// geometric mean of all the features of its sample. The proportions
    /// of a sample sum to one, so that a rise in one feature lowers all
    /// the others; the transform removes that constraint.
    CentredLogRatio,
}

impl Transform {
    pub const ALL: [Transform; 2] = [Transform::None, Transform::CentredLogRatio];

    pub fn name(self) -> &'static str {
        match self {
            Transform::None => "None",
            Transform::CentredLogRatio => "Centred log-ratio",
        }
    }

    /// True if the transform changes the p-values of this two-group test.
    /// White's test works on counts and is left as it is.
    pub fn applies_to(self, test: GroupTest) -> bool {
        self == Transform::CentredLogRatio && test != GroupTest::White && !test.is_presence()
    }
}

/// The centred log-ratio of every feature of a profile in every sample,
/// by feature and then by sample. Each sample is transformed over all the
/// features of the profile; zeros are replaced as described for
/// [`centred_log_ratio`].
pub fn clr_values(profile: &Profile) -> Vec<Vec<f64>> {
    let features = profile.features();
    let samples = profile.sample_names.len();
    let mut values = vec![vec![0.0; samples]; features.len()];
    for sample in 0..samples {
        let counts: Vec<f64> = features.iter().map(|f| f.counts[sample]).collect();
        for (row, value) in values.iter_mut().zip(centred_log_ratio(&counts)) {
            row[sample] = value;
        }
    }
    values
}

/// Settings of a two-group analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupAnalysis {
    pub test: GroupTest,
    pub test_type: TestType,
    pub coverage: f64,
    pub correction: Correction,
    /// Applied to the values the p-value is computed from. The effect size
    /// and its confidence interval stay in proportions.
    pub transform: Transform,
}

/// Statistics of one feature in a two-group analysis. Means, standard
/// deviations, the effect size and its confidence interval are in percent.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRow {
    pub feature: String,
    pub mean1: f64,
    pub std_dev1: f64,
    pub mean2: f64,
    pub std_dev2: f64,
    pub p_value: f64,
    pub p_value_corrected: f64,
    /// Difference between the group means.
    pub effect_size: f64,
    pub lower_ci: f64,
    pub upper_ci: f64,
    pub note: String,
}

/// Results of a two-group analysis, ordered by p-value.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupResults {
    pub analysis: GroupAnalysis,
    pub rows: Vec<GroupRow>,
    pub correction: CorrectionSummary,
}

/// Sequence-count filter for two groups, applied to the counts of a feature
/// in the samples of each group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupCountFilter {
    /// Keep features where any sample of either group reaches the minimum.
    Maximum(f64),
    /// Keep features where every sample of both groups reaches the minimum.
    Minimum(f64),
    /// Keep features where every sample of each group reaches that group's
    /// minimum.
    IndependentMinimum(f64, f64),
    /// Keep features where some sample of each group reaches that group's
    /// minimum.
    IndependentMaximum(f64, f64),
}

impl GroupCountFilter {
    fn passes(self, group1: &[f64], group2: &[f64]) -> bool {
        let max = |g: &[f64]| g.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = |g: &[f64]| g.iter().copied().fold(f64::INFINITY, f64::min);
        match self {
            GroupCountFilter::Maximum(m) => max(group1).max(max(group2)) >= m,
            GroupCountFilter::Minimum(m) => min(group1).min(min(group2)) >= m,
            GroupCountFilter::IndependentMinimum(m1, m2) => min(group1) >= m1 && min(group2) >= m2,
            GroupCountFilter::IndependentMaximum(m1, m2) => max(group1) >= m1 && max(group2) >= m2,
        }
    }
}

/// Filters applied to two-group results. A disabled filter is `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupFilter {
    pub selected_features: Option<HashSet<String>>,
    pub significance: Option<f64>,
    pub sequences: Option<GroupCountFilter>,
    pub parent_sequences: Option<GroupCountFilter>,
    pub effect_size1: Option<(GroupEffectSize, f64)>,
    pub effect_size2: Option<(GroupEffectSize, f64)>,
    pub operator: FilterOperator,
}

impl Default for GroupFilter {
    fn default() -> Self {
        GroupFilter {
            selected_features: None,
            significance: None,
            sequences: None,
            parent_sequences: None,
            effect_size1: None,
            effect_size2: None,
            operator: FilterOperator::Or,
        }
    }
}

/// A two-group analysis that can be carried out in pieces; see
/// [`SampleRun`]. White's test works on all features at once and is done in
/// a single piece, during which it can be cancelled but not paused.
#[derive(Debug, Clone)]
pub struct GroupRun {
    analysis: GroupAnalysis,
    settings: StatsSettings,
    outcomes: Vec<GroupOutcome>,
    /// Transformed values, once worked out.
    transformed: Option<Vec<Vec<f64>>>,
    /// The random numbers as they were before the first feature, for the
    /// multiple-test correction: Storey's bootstrap then draws the same
    /// numbers whether the correction is chosen before the run or applied
    /// to its results afterwards, and whichever test ran.
    correction_rng: Option<StampRng>,
}

impl GroupRun {
    pub fn new(analysis: GroupAnalysis, settings: &StatsSettings) -> GroupRun {
        GroupRun {
            analysis,
            settings: *settings,
            outcomes: Vec::new(),
            transformed: None,
            correction_rng: None,
        }
    }

    /// Test further features of `profile`, which with `rng` must be the same
    /// on every call. `pace` is asked before each piece of work.
    pub fn advance(
        &mut self,
        profile: &GroupProfile,
        rng: &mut StampRng,
        pace: Pace<'_>,
    ) -> Step<GroupResults> {
        if self.correction_rng.is_none() {
            self.correction_rng = Some(rng.clone());
        }
        let (analysis, settings) = (self.analysis, &self.settings);
        let features = profile.profile.features();
        let counts = |feature| {
            let (seq1, seq2) = profile.counts(feature);
            let (parent1, parent2) = profile.parent_counts(feature);
            FeatureCounts {
                seq1,
                seq2,
                parent1,
                parent2,
            }
        };
        if analysis.test == GroupTest::White {
            if self.outcomes.len() < features.len() {
                let all: Vec<FeatureCounts<'_>> = features.iter().map(counts).collect();
                // Asked to pause, the test carries on: it cannot resume.
                let mut progress = |done: usize, total: usize| pace(done, total) != Control::Cancel;
                match analysis
                    .test
                    .run_all(&all, analysis.coverage, settings, rng, &mut progress)
                {
                    Some(outcomes) => self.outcomes = outcomes,
                    None => return Step::Cancelled,
                }
            }
        } else {
            // The other tests treat each feature on its own.
            if analysis.transform.applies_to(analysis.test) && self.transformed.is_none() {
                self.transformed = Some(clr_values(&profile.profile));
            }
            while self.outcomes.len() < features.len() {
                match pace(self.outcomes.len(), features.len()) {
                    Control::Continue => {}
                    Control::Pause => return Step::Paused,
                    Control::Cancel => return Step::Cancelled,
                }
                let one = [counts(&features[self.outcomes.len()])];
                let outcome = analysis
                    .test
                    .run_all(&one, analysis.coverage, settings, rng, &mut |_, _| true)
                    .and_then(|mut outcomes| outcomes.pop());
                let Some(mut outcome) = outcome else {
                    return Step::Cancelled;
                };
                // The p-values come from the transformed values; the effect
                // size and its interval stay as they are.
                if let Some(transformed) = &self.transformed {
                    let (values1, values2) = profile.split(&transformed[self.outcomes.len()]);
                    if let Some((one_sided, two_sided)) = analysis.test.p_values(values1, values2) {
                        outcome.one_sided = one_sided;
                        outcome.two_sided = two_sided;
                    }
                }
                self.outcomes.push(outcome);
            }
        }

        let mut rows: Vec<GroupRow> = features
            .iter()
            .zip(std::mem::take(&mut self.outcomes))
            .map(|(feature, outcome)| {
                let (percent1, percent2) = profile.percentages(feature);
                let p_value = match analysis.test_type {
                    TestType::OneSided => outcome.one_sided,
                    TestType::TwoSided => outcome.two_sided,
                };
                GroupRow {
                    feature: feature.name.clone(),
                    mean1: mean(&percent1),
                    std_dev1: population_std(&percent1),
                    mean2: mean(&percent2),
                    std_dev2: population_std(&percent2),
                    p_value,
                    p_value_corrected: p_value,
                    effect_size: outcome.effect_size,
                    lower_ci: outcome.lower_ci,
                    upper_ci: outcome.upper_ci,
                    note: outcome.note.to_string(),
                }
            })
            .collect();
        sort_rows(&mut rows, |r| r.p_value, |r| &r.feature);
        let mut results = GroupResults {
            analysis,
            rows,
            correction: no_correction(),
        };
        let mut correction_rng = self.correction_rng.take().unwrap_or_else(|| rng.clone());
        results.apply_correction(analysis.correction, &mut correction_rng);
        Step::Done(results)
    }
}

/// Run a two-group test on every feature of a profile. Returns `None` if
/// cancelled through `progress`.
pub fn run_group_analysis(
    profile: &GroupProfile,
    analysis: GroupAnalysis,
    settings: &StatsSettings,
    rng: &mut StampRng,
    progress: Progress<'_>,
) -> Option<GroupResults> {
    match GroupRun::new(analysis, settings).advance(profile, rng, &mut pace_of(progress)) {
        Step::Done(results) => Some(results),
        Step::Paused | Step::Cancelled => None,
    }
}

impl GroupResults {
    pub fn alpha(&self) -> f64 {
        1.0 - self.analysis.coverage
    }

    /// Recompute the corrected p-values with another method.
    pub fn apply_correction(&mut self, method: Correction, rng: &mut StampRng) {
        let p_values: Vec<f64> = self.rows.iter().map(|r| r.p_value).collect();
        let corrected = method.correct(&p_values, self.alpha(), rng);
        for (row, value) in self.rows.iter_mut().zip(&corrected.values) {
            row.p_value_corrected = *value;
        }
        self.analysis.correction = method;
        self.correction = summarize(method, &corrected);
    }

    /// Rows that pass the filters, in order. `profile` must be the profile
    /// the results were computed from.
    pub fn filter(
        &self,
        profile: &GroupProfile,
        filter: &GroupFilter,
        settings: &StatsSettings,
    ) -> Vec<&GroupRow> {
        self.rows
            .iter()
            .filter(|row| {
                if !filter
                    .selected_features
                    .as_ref()
                    .is_none_or(|s| s.contains(&row.feature))
                    || !filter
                        .significance
                        .is_none_or(|level| row.p_value_corrected <= level)
                {
                    return false;
                }
                let Some(feature) = profile.profile.feature(&row.feature) else {
                    return false;
                };
                let (seq1, seq2) = profile.counts(feature);
                let (parent1, parent2) = profile.parent_counts(feature);
                if !filter.sequences.is_none_or(|f| f.passes(seq1, seq2))
                    || !filter
                        .parent_sequences
                        .is_none_or(|f| f.passes(parent1, parent2))
                {
                    return false;
                }
                let (percent1, percent2) = profile.percentages(feature);
                let effect = |f: Option<(GroupEffectSize, f64)>| {
                    f.map(|(measure, minimum)| {
                        (
                            measure.magnitude(&percent1, &percent2, settings.pseudocount),
                            minimum,
                        )
                    })
                };
                passes_effect_size(
                    effect(filter.effect_size1),
                    effect(filter.effect_size2),
                    filter.operator,
                )
            })
            .collect()
    }
}

// --------------------------------------------------------- multiple groups

/// Significance level used to count significant features in multiple-group
/// analyses, which have no confidence-interval coverage of their own.
const MULTI_GROUP_ALPHA: f64 = 0.05;

/// Settings of a multiple-group analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultiGroupAnalysis {
    pub test: MultiGroupTest,
    pub effect_size: MultiGroupEffectSize,
    pub correction: Correction,
    /// Applied to the values the p-value is computed from. The effect
    /// size and the group means stay in proportions.
    pub transform: Transform,
}

/// Statistics of one feature in a multiple-group analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiGroupRow {
    pub feature: String,
    pub p_value: f64,
    pub p_value_corrected: f64,
    pub effect_size: f64,
    pub note: String,
    /// Mean relative frequency (%) of each group.
    pub group_means: Vec<f64>,
    /// Standard deviation of the relative frequency (%) of each group.
    pub group_std_devs: Vec<f64>,
}

/// Results of a multiple-group analysis, ordered by p-value.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiGroupResults {
    pub analysis: MultiGroupAnalysis,
    pub rows: Vec<MultiGroupRow>,
    pub correction: CorrectionSummary,
}

/// Filters applied to multiple-group results. A disabled filter is `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MultiGroupFilter {
    pub selected_features: Option<HashSet<String>>,
    pub significance: Option<f64>,
    /// Remove features with an effect size below this value.
    pub min_effect_size: Option<f64>,
}

/// A multiple-group analysis that can be carried out a few features at a
/// time; see [`SampleRun`].
#[derive(Debug, Clone)]
pub struct MultiGroupRun {
    analysis: MultiGroupAnalysis,
    rows: Vec<MultiGroupRow>,
    /// Transformed values, once worked out.
    transformed: Option<Vec<Vec<f64>>>,
    /// The random numbers as they were before the first feature, for the
    /// multiple-test correction: Storey's bootstrap then draws the same
    /// numbers whether the correction is chosen before the run or applied
    /// to its results afterwards, and whichever test ran.
    correction_rng: Option<StampRng>,
}

impl MultiGroupRun {
    pub fn new(analysis: MultiGroupAnalysis) -> MultiGroupRun {
        MultiGroupRun {
            analysis,
            rows: Vec::new(),
            transformed: None,
            correction_rng: None,
        }
    }

    /// Test further features of `profile`, which with `rng` must be the same
    /// on every call. `pace` is asked before each feature.
    pub fn advance(
        &mut self,
        profile: &MultiGroupProfile,
        rng: &mut StampRng,
        pace: Pace<'_>,
    ) -> Step<MultiGroupResults> {
        if self.correction_rng.is_none() {
            self.correction_rng = Some(rng.clone());
        }
        let analysis = self.analysis;
        let features = profile.profile.features();
        // Presence is read from the counts, which a transform would hide.
        if analysis.transform == Transform::CentredLogRatio
            && !analysis.test.is_presence()
            && self.transformed.is_none()
        {
            self.transformed = Some(clr_values(&profile.profile));
        }
        while self.rows.len() < features.len() {
            match pace(self.rows.len(), features.len()) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            let feature = &features[self.rows.len()];
            let percentages = profile.percentages(feature);
            let groups: Vec<&[f64]> = percentages.iter().map(Vec::as_slice).collect();
            let (p_value, note) = match &self.transformed {
                Some(transformed) => analysis
                    .test
                    .run(&profile.split(&transformed[self.rows.len()])),
                None => analysis.test.run(&groups),
            };
            self.rows.push(MultiGroupRow {
                feature: feature.name.clone(),
                p_value,
                p_value_corrected: p_value,
                effect_size: analysis.effect_size.run(&groups),
                note: note.to_string(),
                group_means: groups.iter().map(|g| mean(g)).collect(),
                group_std_devs: groups.iter().map(|g| population_std(g)).collect(),
            });
        }
        let mut rows = std::mem::take(&mut self.rows);
        sort_rows(&mut rows, |r| r.p_value, |r| &r.feature);
        let mut results = MultiGroupResults {
            analysis,
            rows,
            correction: no_correction(),
        };
        let mut correction_rng = self.correction_rng.take().unwrap_or_else(|| rng.clone());
        results.apply_correction(analysis.correction, &mut correction_rng);
        Step::Done(results)
    }
}

/// Run a multiple-group test on every feature of a profile. Returns `None`
/// if cancelled through `progress`.
pub fn run_multi_group_analysis(
    profile: &MultiGroupProfile,
    analysis: MultiGroupAnalysis,
    rng: &mut StampRng,
    progress: Progress<'_>,
) -> Option<MultiGroupResults> {
    match MultiGroupRun::new(analysis).advance(profile, rng, &mut pace_of(progress)) {
        Step::Done(results) => Some(results),
        Step::Paused | Step::Cancelled => None,
    }
}

impl MultiGroupResults {
    /// Recompute the corrected p-values with another method.
    pub fn apply_correction(&mut self, method: Correction, rng: &mut StampRng) {
        let p_values: Vec<f64> = self.rows.iter().map(|r| r.p_value).collect();
        let corrected = method.correct(&p_values, MULTI_GROUP_ALPHA, rng);
        for (row, value) in self.rows.iter_mut().zip(&corrected.values) {
            row.p_value_corrected = *value;
        }
        self.analysis.correction = method;
        self.correction = summarize(method, &corrected);
    }

    /// Rows that pass the filters, in order.
    pub fn filter(&self, filter: &MultiGroupFilter) -> Vec<&MultiGroupRow> {
        self.rows
            .iter()
            .filter(|row| {
                filter
                    .selected_features
                    .as_ref()
                    .is_none_or(|s| s.contains(&row.feature))
                    && filter
                        .significance
                        .is_none_or(|level| row.p_value_corrected <= level)
                    && filter
                        .min_effect_size
                        .is_none_or(|minimum| row.effect_size >= minimum)
            })
            .collect()
    }
}

/// Run a post-hoc test on one feature of a multiple-group profile.
pub fn run_post_hoc(
    profile: &MultiGroupProfile,
    feature: &str,
    test: PostHocTest,
    coverage: f64,
) -> Result<PostHocResult> {
    let feature = profile
        .profile
        .feature(feature)
        .ok_or_else(|| StampError::new(format!("Unknown feature '{feature}'.")))?;
    let percentages = profile.percentages(feature);
    let groups: Vec<&[f64]> = percentages.iter().map(Vec::as_slice).collect();
    let names: Vec<&str> = profile.group_names.iter().map(String::as_str).collect();
    test.run(&groups, coverage, &names)
}

/// Format a p-value for display: three decimals, or scientific notation
/// below 0.01, with values under `10^min_exponent` shown as a bound.
pub fn format_p_value(p: f64, min_exponent: i32) -> String {
    if p.is_nan() {
        return "nan".to_string();
    }
    if p >= 0.01 {
        return format!("{p:.3}");
    }
    if p < 10f64.powi(min_exponent) {
        return format!("< 1e{min_exponent}");
    }
    // Mantissa with two decimals and an exponent without padding: 1.23e-5.
    format!("{p:.2e}")
}

// -------------------------------------------------------------- correlation

/// Significance level used to count significant features in correlation
/// analyses, which have no confidence-interval coverage of their own.
const CORRELATION_ALPHA: f64 = 0.05;

const TOO_FEW_VALUES: &str = "degenerate case: a correlation needs at least 3 samples";
const CONSTANT_FEATURE: &str =
    "degenerate case: the feature has the same proportion in every sample";
const CONSTANT_FIELD: &str = "degenerate case: the field has the same value in every sample";

/// Settings of a correlation analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelationAnalysis {
    pub method: CorrelationMethod,
    pub correction: Correction,
}

/// Correlation of one feature with a numeric metadata field.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrelationRow {
    pub feature: String,
    /// The correlation coefficient, from -1 to 1.
    pub coefficient: f64,
    /// Two-sided p-value against no correlation.
    pub p_value: f64,
    pub p_value_corrected: f64,
    /// Mean relative frequency (%) of the feature over the samples.
    pub mean: f64,
    /// Standard deviation of the relative frequency (%).
    pub std_dev: f64,
    pub note: String,
}

/// Results of a correlation analysis, ordered by p-value.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrelationResults {
    pub analysis: CorrelationAnalysis,
    pub rows: Vec<CorrelationRow>,
    pub correction: CorrectionSummary,
}

/// Filters applied to correlation results. A disabled filter is `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CorrelationFilter {
    pub selected_features: Option<HashSet<String>>,
    pub significance: Option<f64>,
    /// Remove features whose coefficient is closer to zero than this.
    pub min_coefficient: Option<f64>,
}

/// A correlation analysis that can be carried out a few features at a
/// time; see [`SampleRun`].
#[derive(Debug, Clone)]
pub struct CorrelationRun {
    analysis: CorrelationAnalysis,
    rows: Vec<CorrelationRow>,
    /// The random numbers as they were before the first feature, for the
    /// multiple-test correction: Storey's bootstrap then draws the same
    /// numbers whether the correction is chosen before the run or applied
    /// to its results afterwards, and whichever test ran.
    correction_rng: Option<StampRng>,
}

impl CorrelationRun {
    pub fn new(analysis: CorrelationAnalysis) -> CorrelationRun {
        CorrelationRun {
            analysis,
            rows: Vec::new(),
            correction_rng: None,
        }
    }

    /// Correlate further features of `profile`, which with `rng` must be
    /// the same on every call. `pace` is asked before each feature.
    pub fn advance(
        &mut self,
        profile: &CorrelationProfile,
        rng: &mut StampRng,
        pace: Pace<'_>,
    ) -> Step<CorrelationResults> {
        if self.correction_rng.is_none() {
            self.correction_rng = Some(rng.clone());
        }
        let analysis = self.analysis;
        let features = profile.profile.features();
        let samples = profile.values.len();
        let field_varies = profile.values.iter().any(|v| *v != profile.values[0]);
        while self.rows.len() < features.len() {
            match pace(self.rows.len(), features.len()) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            let feature = &features[self.rows.len()];
            let percentages = profile.profile.percentages(feature);
            let (coefficient, p_value, note) =
                match analysis.method.coefficient(&percentages, &profile.values) {
                    Some(r) => (r, correlation_p_value(r, samples), ""),
                    None if samples < 3 => (0.0, 1.0, TOO_FEW_VALUES),
                    None if !field_varies => (0.0, 1.0, CONSTANT_FIELD),
                    None => (0.0, 1.0, CONSTANT_FEATURE),
                };
            self.rows.push(CorrelationRow {
                feature: feature.name.clone(),
                coefficient,
                p_value,
                p_value_corrected: p_value,
                mean: mean(&percentages),
                std_dev: population_std(&percentages),
                note: note.to_string(),
            });
        }
        let mut rows = std::mem::take(&mut self.rows);
        sort_rows(&mut rows, |r| r.p_value, |r| &r.feature);
        let mut results = CorrelationResults {
            analysis,
            rows,
            correction: no_correction(),
        };
        let mut correction_rng = self.correction_rng.take().unwrap_or_else(|| rng.clone());
        results.apply_correction(analysis.correction, &mut correction_rng);
        Step::Done(results)
    }
}

/// Correlate every feature of a profile with the field. Returns `None` if
/// cancelled through `progress`.
pub fn run_correlation_analysis(
    profile: &CorrelationProfile,
    analysis: CorrelationAnalysis,
    rng: &mut StampRng,
    progress: Progress<'_>,
) -> Option<CorrelationResults> {
    match CorrelationRun::new(analysis).advance(profile, rng, &mut pace_of(progress)) {
        Step::Done(results) => Some(results),
        Step::Paused | Step::Cancelled => None,
    }
}

impl CorrelationResults {
    pub fn alpha(&self) -> f64 {
        CORRELATION_ALPHA
    }

    /// Recompute the corrected p-values with another method.
    pub fn apply_correction(&mut self, method: Correction, rng: &mut StampRng) {
        let p_values: Vec<f64> = self.rows.iter().map(|r| r.p_value).collect();
        let corrected = method.correct(&p_values, self.alpha(), rng);
        for (row, value) in self.rows.iter_mut().zip(&corrected.values) {
            row.p_value_corrected = *value;
        }
        self.analysis.correction = method;
        self.correction = summarize(method, &corrected);
    }

    /// Rows that pass the filters, in order.
    pub fn filter(&self, filter: &CorrelationFilter) -> Vec<&CorrelationRow> {
        self.rows
            .iter()
            .filter(|row| {
                filter
                    .selected_features
                    .as_ref()
                    .is_none_or(|s| s.contains(&row.feature))
                    && filter
                        .significance
                        .is_none_or(|level| row.p_value_corrected <= level)
                    && filter
                        .min_coefficient
                        .is_none_or(|minimum| row.coefficient.abs() >= minimum)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::Metadata;
    use crate::profile::{SecondGroup, Unclassified};
    use crate::stats::rng_from_seed;
    use crate::ProfileTree;

    const SPF: &str = "Feature\tA1\tA2\tA3\tB1\tB2\tB3\n\
        x\t10\t12\t11\t30\t33\t31\n\
        y\t50\t48\t52\t50\t49\t51\n\
        z\t40\t40\t37\t20\t18\t18\n\
        rare\t0\t0\t0\t0\t0\t0\n";
    const META: &str = "Id\tGroup\nA1\tA\nA2\tA\nA3\tA\nB1\tB\nB2\tB\nB3\tB\n";

    fn settings() -> StatsSettings {
        StatsSettings::default()
    }

    fn sample_results(correction: Correction) -> SampleResults {
        let tree = ProfileTree::read(SPF).unwrap();
        let profile = tree
            .sample_profile("A1", "B1", None, "Feature", Unclassified::Retain)
            .unwrap();
        let analysis = SampleAnalysis {
            test: SampleTest::Fisher,
            test_type: TestType::TwoSided,
            ci_method: SampleCi::DiffBetweenPropAsymptotic,
            coverage: 0.95,
            correction,
        };
        run_sample_analysis(
            &profile,
            analysis,
            &settings(),
            &mut rng_from_seed(1),
            &mut |_, _| true,
        )
        .unwrap()
    }

    fn group_setup() -> (GroupProfile, GroupResults) {
        let tree = ProfileTree::read(SPF).unwrap();
        let (meta, _) = Metadata::read(META, &tree).unwrap();
        let groups = meta.groups("Group").unwrap();
        let profile = tree
            .group_profile(
                &groups,
                "A",
                SecondGroup::Named("B"),
                None,
                "Feature",
                Unclassified::Retain,
            )
            .unwrap();
        let analysis = GroupAnalysis {
            test: GroupTest::Welch,
            test_type: TestType::TwoSided,
            coverage: 0.95,
            correction: Correction::None,
            transform: Transform::None,
        };
        let results = run_group_analysis(
            &profile,
            analysis,
            &settings(),
            &mut rng_from_seed(1),
            &mut |_, _| true,
        )
        .unwrap();
        (profile, results)
    }

    fn names<T>(rows: &[&T], name: impl Fn(&T) -> &str) -> Vec<String> {
        rows.iter().map(|r| name(r).to_string()).collect()
    }

    #[test]
    fn sample_rows_are_sorted_and_corrected() {
        let r = sample_results(Correction::Bonferroni);
        assert_eq!(r.rows.len(), 4);
        for pair in r.rows.windows(2) {
            assert!(pair[0].p_value <= pair[1].p_value);
        }
        assert_eq!(r.rows[0].feature, "x");
        for row in &r.rows {
            assert!((row.p_value_corrected - (row.p_value * 4.0).min(1.0)).abs() < 1e-15);
        }
        assert_eq!(r.correction.method, Correction::Bonferroni);
        assert!((r.alpha() - 0.05).abs() < 1e-12);
        let rare = r.rows.iter().find(|row| row.feature == "rare").unwrap();
        assert_eq!((rare.p_value, rare.rel_freq1), (1.0, 0.0));
    }

    #[test]
    fn sample_filters() {
        let r = sample_results(Correction::None);
        let s = settings();
        let all = SampleFilter::default();
        assert_eq!(r.filter(&all, &s).len(), 4);

        let significant = SampleFilter {
            significance: Some(0.05),
            ..all.clone()
        };
        assert_eq!(
            names(&r.filter(&significant, &s), |row| &row.feature),
            ["x", "z"]
        );

        let counts = SampleFilter {
            sequences: Some(SampleCountFilter::Minimum(25.0)),
            ..all.clone()
        };
        assert_eq!(names(&r.filter(&counts, &s), |row| &row.feature).len(), 1);
        let counts = SampleFilter {
            sequences: Some(SampleCountFilter::Maximum(25.0)),
            ..all.clone()
        };
        assert_eq!(r.filter(&counts, &s).len(), 3);
        let counts = SampleFilter {
            sequences: Some(SampleCountFilter::Independent(5.0, 25.0)),
            ..all.clone()
        };
        assert_eq!(r.filter(&counts, &s).len(), 2);

        let selected = SampleFilter {
            selected_features: Some(["y".to_string(), "rare".to_string()].into()),
            ..all.clone()
        };
        assert_eq!(r.filter(&selected, &s).len(), 2);

        // x: difference 20 points, ratio 3. z: difference 20, ratio 2. y: none.
        let dp = Some((SampleEffectSize::DiffBetweenProp, 15.0));
        let rp = Some((SampleEffectSize::RatioProportions, 2.5));
        let one = SampleFilter {
            effect_size1: dp,
            ..all.clone()
        };
        assert_eq!(r.filter(&one, &s).len(), 2);
        let either_fails = SampleFilter {
            effect_size1: dp,
            effect_size2: rp,
            operator: FilterOperator::Or,
            ..all.clone()
        };
        assert_eq!(
            names(&r.filter(&either_fails, &s), |row| &row.feature),
            ["x"]
        );
        let both_fail = SampleFilter {
            effect_size1: dp,
            effect_size2: rp,
            operator: FilterOperator::And,
            ..all
        };
        assert_eq!(r.filter(&both_fail, &s).len(), 2);
    }

    #[test]
    fn group_rows_and_filters() {
        let (profile, r) = group_setup();
        let s = settings();
        let x = r.rows.iter().find(|row| row.feature == "x").unwrap();
        assert!((x.mean1 - 11.0).abs() < 1e-12);
        assert!((x.effect_size - (x.mean1 - x.mean2)).abs() < 1e-9);
        assert!(x.p_value < 0.001);

        let all = GroupFilter::default();
        assert_eq!(r.filter(&profile, &all, &s).len(), 4);
        let significant = GroupFilter {
            significance: Some(0.05),
            ..all.clone()
        };
        assert_eq!(r.filter(&profile, &significant, &s).len(), 2);

        let f = GroupFilter {
            sequences: Some(GroupCountFilter::Minimum(18.0)),
            ..all.clone()
        };
        assert_eq!(
            names(&r.filter(&profile, &f, &s), |row| &row.feature),
            ["z", "y"]
        );
        let f = GroupFilter {
            sequences: Some(GroupCountFilter::Maximum(52.0)),
            ..all.clone()
        };
        assert_eq!(
            names(&r.filter(&profile, &f, &s), |row| &row.feature),
            ["y"]
        );
        let f = GroupFilter {
            sequences: Some(GroupCountFilter::IndependentMinimum(10.0, 30.0)),
            ..all.clone()
        };
        assert_eq!(r.filter(&profile, &f, &s).len(), 2);
        let f = GroupFilter {
            sequences: Some(GroupCountFilter::IndependentMaximum(40.0, 20.0)),
            ..all.clone()
        };
        assert_eq!(r.filter(&profile, &f, &s).len(), 2);

        // The feature absent from both groups has a ratio of one, not a crash.
        let f = GroupFilter {
            effect_size1: Some((GroupEffectSize::RatioProportions, 1.5)),
            ..all
        };
        assert_eq!(
            names(&r.filter(&profile, &f, &s), |row| &row.feature),
            ["x", "z"]
        );
    }

    #[test]
    fn multi_group_analysis_and_post_hoc() {
        let tree = ProfileTree::read(SPF).unwrap();
        let (meta, _) = Metadata::read(META, &tree).unwrap();
        let groups = meta.groups("Group").unwrap();
        let profile = tree
            .multi_group_profile(&groups, None, None, "Feature", Unclassified::Retain)
            .unwrap();
        let analysis = MultiGroupAnalysis {
            test: MultiGroupTest::Anova,
            effect_size: MultiGroupEffectSize::EtaSquared,
            correction: Correction::BenjaminiHochberg,
            transform: Transform::None,
        };
        let r =
            run_multi_group_analysis(&profile, analysis, &mut rng_from_seed(1), &mut |_, _| true)
                .unwrap();
        assert_eq!(r.rows.len(), 4);
        assert_eq!(r.rows[0].group_means.len(), 2);
        assert!(r.rows[0].p_value < 0.001);

        let f = MultiGroupFilter {
            significance: Some(0.05),
            ..Default::default()
        };
        assert_eq!(r.filter(&f).len(), 2);
        let f = MultiGroupFilter {
            min_effect_size: Some(0.9),
            ..Default::default()
        };
        assert_eq!(r.filter(&f).len(), 2);

        let post_hoc = run_post_hoc(&profile, "x", PostHocTest::TukeyKramer, 0.95).unwrap();
        assert_eq!(post_hoc.comparisons[0].label, "A : B");
        assert!(run_post_hoc(&profile, "nope", PostHocTest::TukeyKramer, 0.95).is_err());
    }

    #[test]
    fn analyses_can_be_cancelled() {
        let tree = ProfileTree::read(SPF).unwrap();
        let profile = tree
            .sample_profile("A1", "B1", None, "Feature", Unclassified::Retain)
            .unwrap();
        let analysis = SampleAnalysis {
            test: SampleTest::Fisher,
            test_type: TestType::TwoSided,
            ci_method: SampleCi::DiffBetweenPropAsymptotic,
            coverage: 0.95,
            correction: Correction::None,
        };
        let cancelled = run_sample_analysis(
            &profile,
            analysis,
            &settings(),
            &mut rng_from_seed(1),
            &mut |done, _| done < 2,
        );
        assert!(cancelled.is_none());
    }

    #[test]
    fn p_value_formatting() {
        assert_eq!(format_p_value(0.5, -15), "0.500");
        assert_eq!(format_p_value(0.0123, -15), "0.012");
        assert_eq!(format_p_value(0.00123, -15), "1.23e-3");
        assert_eq!(format_p_value(1.5e-12, -15), "1.50e-12");
        assert_eq!(format_p_value(1e-20, -15), "< 1e-15");
        assert_eq!(format_p_value(0.0, -15), "< 1e-15");
    }

    /// Advancing a run a feature at a time gives exactly what running it in
    /// one go does, random numbers included.
    #[test]
    fn runs_can_pause_between_features() {
        use crate::stats::rng_from_seed;
        use crate::stats::two_sample::SampleTest;
        use crate::{ProfileTree, SecondGroup, Unclassified};

        let mut spf = String::from("Feature\tA1\tA2\tA3\tB1\tB2\tB3\tC1\tC2\tC3\n");
        for row in 0..12 {
            spf.push_str(&format!("f{row}"));
            for column in 0..9 {
                spf.push_str(&format!(
                    "\t{}",
                    5 + (row * 7 + column * 13 + row * column) % 41
                ));
            }
            spf.push('\n');
        }
        let tree = ProfileTree::read(&spf).unwrap();
        let metadata = "Id\tGroup\nA1\tA\nA2\tA\nA3\tA\nB1\tB\nB2\tB\nB3\tB\nC1\tC\nC2\tC\nC3\tC\n";
        let (metadata, _) = crate::Metadata::read(metadata, &tree).unwrap();
        let groups = metadata.groups("Group").unwrap();
        let settings = StatsSettings {
            replicates: 50,
            ..StatsSettings::default()
        };
        // Pause before every second piece of work, cancel never.
        let stepped = |advance: &mut dyn FnMut(Pace<'_>) -> bool| {
            let mut pauses = 0;
            loop {
                let mut calls = 0;
                let done = advance(&mut |_, _| {
                    calls += 1;
                    if calls % 2 == 0 {
                        Control::Pause
                    } else {
                        Control::Continue
                    }
                });
                if done {
                    return pauses;
                }
                pauses += 1;
                assert!(pauses < 1000, "the run does not finish");
            }
        };

        // Two samples, with a test that draws random numbers.
        let profile = tree
            .sample_profile("A1", "B2", None, "Feature", Unclassified::Retain)
            .unwrap();
        let analysis = SampleAnalysis {
            test: SampleTest::Bootstrap,
            test_type: TestType::TwoSided,
            ci_method: crate::stats::ci::SampleCi::NewcombeWilson,
            coverage: 0.95,
            correction: Correction::BenjaminiHochberg,
        };
        let whole = run_sample_analysis(
            &profile,
            analysis,
            &settings,
            &mut rng_from_seed(3),
            &mut |_, _| true,
        )
        .unwrap();
        let (mut run, mut rng, mut result) =
            (SampleRun::new(analysis, &settings), rng_from_seed(3), None);
        let pauses = stepped(&mut |pace| match run.advance(&profile, &mut rng, pace) {
            Step::Done(results) => {
                result = Some(results);
                true
            }
            Step::Paused => false,
            Step::Cancelled => panic!("cancelled"),
        });
        assert!(pauses >= 5);
        assert_eq!(result.unwrap(), whole);

        // Two groups.
        let profile = tree
            .group_profile(
                &groups,
                "A",
                SecondGroup::Named("B"),
                None,
                "Feature",
                Unclassified::Retain,
            )
            .unwrap();
        for test in [GroupTest::Welch, GroupTest::White] {
            let analysis = GroupAnalysis {
                test,
                test_type: TestType::TwoSided,
                coverage: 0.95,
                correction: Correction::None,
                transform: Transform::None,
            };
            let whole = run_group_analysis(
                &profile,
                analysis,
                &settings,
                &mut rng_from_seed(3),
                &mut |_, _| true,
            )
            .unwrap();
            let (mut run, mut rng, mut result) =
                (GroupRun::new(analysis, &settings), rng_from_seed(3), None);
            stepped(&mut |pace| match run.advance(&profile, &mut rng, pace) {
                Step::Done(results) => {
                    result = Some(results);
                    true
                }
                Step::Paused => false,
                Step::Cancelled => panic!("cancelled"),
            });
            assert_eq!(result.unwrap(), whole, "{test:?}");

            // Storey's bootstrap draws the same random numbers whether it is
            // chosen before the run or applied afterwards, even after a
            // test that drew random numbers of its own (White's).
            let storey = GroupAnalysis {
                correction: Correction::Storey,
                ..analysis
            };
            let before = run_group_analysis(
                &profile,
                storey,
                &settings,
                &mut rng_from_seed(3),
                &mut |_, _| true,
            )
            .unwrap();
            let mut after = whole.clone();
            after.apply_correction(Correction::Storey, &mut rng_from_seed(3));
            assert_eq!(before, after, "{test:?}");
        }

        // Multiple groups, and cancelling.
        let profile = tree
            .multi_group_profile(&groups, None, None, "Feature", Unclassified::Retain)
            .unwrap();
        let analysis = MultiGroupAnalysis {
            test: crate::stats::multi_group::MultiGroupTest::Anova,
            effect_size: crate::stats::multi_group::MultiGroupEffectSize::EtaSquared,
            correction: Correction::None,
            transform: Transform::None,
        };
        let whole =
            run_multi_group_analysis(&profile, analysis, &mut rng_from_seed(3), &mut |_, _| true)
                .unwrap();
        let (mut run, mut rng, mut result) = (MultiGroupRun::new(analysis), rng_from_seed(3), None);
        stepped(&mut |pace| match run.advance(&profile, &mut rng, pace) {
            Step::Done(results) => {
                result = Some(results);
                true
            }
            Step::Paused => false,
            Step::Cancelled => panic!("cancelled"),
        });
        assert_eq!(result.unwrap(), whole);
        let mut run = MultiGroupRun::new(analysis);
        assert_eq!(
            run.advance(&profile, &mut rng, &mut |done, _| if done == 3 {
                Control::Cancel
            } else {
                Control::Continue
            }),
            Step::Cancelled
        );
    }

    const PAIR_META: &str = "Id\tGroup\tSubject\tDepth\n\
        A1\tA\ts1\t1.0\nA2\tA\ts2\t2.5\nA3\tA\ts3\t4.0\n\
        B1\tB\ts3\t5.0\nB2\tB\ts1\t7.5\nB3\tB\ts2\tunknown\n";

    fn group_analysis(test: GroupTest, transform: Transform) -> GroupAnalysis {
        GroupAnalysis {
            test,
            test_type: TestType::TwoSided,
            coverage: 0.95,
            correction: Correction::None,
            transform,
        }
    }

    fn group_run(profile: &GroupProfile, analysis: GroupAnalysis) -> GroupResults {
        run_group_analysis(
            profile,
            analysis,
            &settings(),
            &mut rng_from_seed(1),
            &mut |_, _| true,
        )
        .unwrap()
    }

    #[test]
    fn paired_tests_use_the_pairs_of_the_metadata() {
        let tree = ProfileTree::read(SPF).unwrap();
        let (metadata, _) = Metadata::read(PAIR_META, &tree).unwrap();
        let groups = metadata.groups("Group").unwrap();
        let pairing = metadata
            .paired_groups(&groups, "A", "B", "Subject")
            .unwrap();
        assert_eq!((pairing.pairs, pairing.unpaired), (3, 0));
        assert_eq!(pairing.summary(), "3 pairs");
        // Pair by pair, in the order of the subjects.
        assert_eq!(pairing.groups.samples("A").unwrap(), ["A1", "A2", "A3"]);
        assert_eq!(pairing.groups.samples("B").unwrap(), ["B2", "B3", "B1"]);

        let profile = tree
            .group_profile(
                &pairing.groups,
                "A",
                SecondGroup::Named("B"),
                None,
                "Feature",
                Unclassified::Retain,
            )
            .unwrap();
        assert_eq!(profile.samples_in_group2(), ["B2", "B3", "B1"]);
        let results = group_run(
            &profile,
            group_analysis(GroupTest::PairedT, Transform::None),
        );
        let x = results.rows.iter().find(|r| r.feature == "x").unwrap();
        // The differences in x within the pairs, as proportions of 100:
        // 10-33, 12-31, 11-30.
        let direct =
            crate::stats::two_group::paired_t(&[0.10, 0.12, 0.11], &[0.33, 0.31, 0.30], 0.95);
        assert!((x.p_value - direct.two_sided).abs() < 1e-12);
        assert!((x.effect_size - direct.effect_size).abs() < 1e-9);
        assert!(x.note.is_empty());

        // Samples without exactly one partner are left out.
        let mut fewer = metadata.clone();
        fewer.set_active("B2", false);
        let groups = fewer.groups("Group").unwrap();
        let pairing = fewer.paired_groups(&groups, "A", "B", "Subject").unwrap();
        assert_eq!((pairing.pairs, pairing.unpaired), (2, 1));
        assert_eq!(
            pairing.summary(),
            "2 pairs; 1 sample without a partner left out"
        );
        assert_eq!(pairing.groups.samples("A").unwrap(), ["A2", "A3"]);
        // Pairing by the group field itself pairs nothing.
        let none = fewer.paired_groups(&groups, "A", "B", "Group").unwrap();
        assert_eq!((none.pairs, none.unpaired), (0, 5));
        assert!(fewer.paired_groups(&groups, "A", "A", "Subject").is_err());
        assert!(fewer.paired_groups(&groups, "A", "B", "Nothing").is_err());
    }

    #[test]
    fn centred_log_ratio_changes_only_the_p_values() {
        let (profile, plain) = group_setup();
        let analysis = group_analysis(plain.analysis.test, Transform::CentredLogRatio);
        let analysis = GroupAnalysis {
            test_type: plain.analysis.test_type,
            coverage: plain.analysis.coverage,
            correction: plain.analysis.correction,
            ..analysis
        };
        let transformed = group_run(&profile, analysis);
        let values = clr_values(&profile.profile);
        // Every sample's values sum to zero over the features.
        for sample in 0..profile.profile.sample_names.len() {
            let sum: f64 = values.iter().map(|feature| feature[sample]).sum();
            assert!(sum.abs() < 1e-9, "{sum}");
        }
        for row in &transformed.rows {
            let before = plain
                .rows
                .iter()
                .find(|r| r.feature == row.feature)
                .unwrap();
            assert_eq!(
                (row.effect_size, row.lower_ci, row.upper_ci, row.mean1),
                (
                    before.effect_size,
                    before.lower_ci,
                    before.upper_ci,
                    before.mean1
                )
            );
            let index = profile
                .profile
                .features()
                .iter()
                .position(|f| f.name == row.feature)
                .unwrap();
            let (v1, v2) = profile.split(&values[index]);
            let (_, two_sided) = analysis.test.p_values(v1, v2).unwrap();
            assert!((row.p_value - two_sided).abs() < 1e-12);
        }
        // y is the same share of every sample, but not relative to the
        // other features: its p-value changes.
        let y = |results: &GroupResults| {
            results
                .rows
                .iter()
                .find(|r| r.feature == "y")
                .unwrap()
                .p_value
        };
        assert!(y(&transformed) < y(&plain));

        // White's test works on counts and is not transformed.
        assert!(!Transform::CentredLogRatio.applies_to(GroupTest::White));
        assert!(Transform::CentredLogRatio.applies_to(GroupTest::Welch));
        assert!(!Transform::None.applies_to(GroupTest::Welch));

        // Multiple groups: the test is run on the transformed values.
        let tree = ProfileTree::read(SPF).unwrap();
        let (metadata, _) = Metadata::read(META, &tree).unwrap();
        let groups = metadata.groups("Group").unwrap();
        let multi = tree
            .multi_group_profile(&groups, None, None, "Feature", Unclassified::Retain)
            .unwrap();
        let run = |transform| {
            run_multi_group_analysis(
                &multi,
                MultiGroupAnalysis {
                    test: MultiGroupTest::Anova,
                    effect_size: MultiGroupEffectSize::EtaSquared,
                    correction: Correction::None,
                    transform,
                },
                &mut rng_from_seed(1),
                &mut |_, _| true,
            )
            .unwrap()
        };
        let (plain, transformed) = (run(Transform::None), run(Transform::CentredLogRatio));
        let values = clr_values(&multi.profile);
        for row in &transformed.rows {
            let before = plain
                .rows
                .iter()
                .find(|r| r.feature == row.feature)
                .unwrap();
            assert_eq!(row.effect_size, before.effect_size);
            assert_eq!(row.group_means, before.group_means);
            let index = multi
                .profile
                .features()
                .iter()
                .position(|f| f.name == row.feature)
                .unwrap();
            let (p, _) = MultiGroupTest::Anova.run(&multi.split(&values[index]));
            assert!((row.p_value - p).abs() < 1e-12);
        }
    }

    #[test]
    fn correlation_with_a_numeric_field() {
        let tree = ProfileTree::read(SPF).unwrap();
        let (metadata, _) = Metadata::read(PAIR_META, &tree).unwrap();
        let profile = tree
            .correlation_profile(&metadata, "Depth", None, "Feature", Unclassified::Retain)
            .unwrap();
        // B3 has no number and is left out.
        assert_eq!(profile.profile.sample_names, ["A1", "A2", "A3", "B1", "B2"]);
        assert_eq!(profile.values, [1.0, 2.5, 4.0, 5.0, 7.5]);
        assert_eq!(profile.without_value, 1);
        assert!(tree
            .correlation_profile(&metadata, "Nothing", None, "Feature", Unclassified::Retain)
            .is_err());

        let analysis = CorrelationAnalysis {
            method: CorrelationMethod::Spearman,
            correction: Correction::Bonferroni,
        };
        let results =
            run_correlation_analysis(&profile, analysis, &mut rng_from_seed(1), &mut |_, _| true)
                .unwrap();
        assert_eq!(results.rows.len(), 4);
        let row = |name: &str| results.rows.iter().find(|r| r.feature == name).unwrap();
        // x: 10, 12, 11, 30, 33 of 100 against 1, 2.5, 4, 5, 7.5: ranks
        // 1 3 2 4 5 against 1 2 3 4 5, so rho = 1 - 6 * 2 / 120 = 0.9.
        assert!((row("x").coefficient - 0.9).abs() < 1e-12);
        assert!((row("x").p_value - correlation_p_value(0.9, 5)).abs() < 1e-12);
        assert!((row("x").p_value_corrected - (4.0 * row("x").p_value).min(1.0)).abs() < 1e-12);
        assert!((row("x").mean - 19.2).abs() < 1e-9);
        assert!(row("z").coefficient < 0.0);
        // A feature that never varies has no correlation.
        assert_eq!((row("rare").coefficient, row("rare").p_value), (0.0, 1.0));
        assert_eq!(row("rare").note, CONSTANT_FEATURE);
        // Ordered by p-value.
        assert!(results
            .rows
            .windows(2)
            .all(|w| w[0].p_value <= w[1].p_value));

        let all = results.filter(&CorrelationFilter::default());
        assert_eq!(all.len(), 4);
        let strong = results.filter(&CorrelationFilter {
            min_coefficient: Some(0.85),
            ..Default::default()
        });
        assert!(strong.iter().any(|r| r.feature == "x"));
        assert!(strong.iter().all(|r| r.coefficient.abs() >= 0.85));
        assert!(!strong.iter().any(|r| r.feature == "rare"));
        let chosen = results.filter(&CorrelationFilter {
            selected_features: Some(["y".to_string()].into_iter().collect()),
            ..Default::default()
        });
        assert_eq!(chosen.len(), 1);

        // Pearson gives another coefficient for the same data.
        let pearson = run_correlation_analysis(
            &profile,
            CorrelationAnalysis {
                method: CorrelationMethod::Pearson,
                correction: Correction::None,
            },
            &mut rng_from_seed(1),
            &mut |_, _| true,
        )
        .unwrap();
        let x = pearson.rows.iter().find(|r| r.feature == "x").unwrap();
        assert!(x.coefficient > 0.8 && (x.coefficient - 0.9).abs() > 1e-6);

        // Fewer than three samples with a number: nothing can be said.
        let mut few = metadata.clone();
        for sample in ["A1", "A2", "A3"] {
            few.set_active(sample, false);
        }
        let profile = tree
            .correlation_profile(&few, "Depth", None, "Feature", Unclassified::Retain)
            .unwrap();
        let results =
            run_correlation_analysis(&profile, analysis, &mut rng_from_seed(1), &mut |_, _| true)
                .unwrap();
        assert!(results.rows.iter().all(|r| r.note == TOO_FEW_VALUES));
    }
}
