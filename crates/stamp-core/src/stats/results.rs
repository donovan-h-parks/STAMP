//! Running a test over every feature of a profile, correcting for multiple
//! comparisons, and filtering the results.

use std::collections::HashSet;

use super::ci::SampleCi;
use super::correction::{Corrected, Correction};
use super::effect_size::{GroupEffectSize, SampleEffectSize};
use super::multi_group::{MultiGroupEffectSize, MultiGroupTest};
use super::post_hoc::{PostHocResult, PostHocTest};
use super::two_group::{FeatureCounts, GroupTest};
use super::two_sample::SampleTest;
use super::{mean, population_std, StampRng, StatsSettings, TestType};
use crate::error::{Result, StampError};
use crate::profile::{GroupProfile, MultiGroupProfile, SampleProfile};

/// Callback reporting progress as `(done, total)`. Returning `false` cancels
/// the run.
pub type Progress<'a> = &'a mut dyn FnMut(usize, usize) -> bool;

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

/// Run a two-sample test on every feature of a profile. Returns `None` if
/// cancelled through `progress`.
pub fn run_sample_analysis(
    profile: &SampleProfile,
    analysis: SampleAnalysis,
    settings: &StatsSettings,
    rng: &mut StampRng,
    progress: Progress<'_>,
) -> Option<SampleResults> {
    let features = profile.profile.features();
    let mut rows = Vec::with_capacity(features.len());
    for (i, feature) in features.iter().enumerate() {
        if !progress(i, features.len()) {
            return None;
        }
        let c = profile.counts(feature);
        let test = analysis
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
        rows.push(SampleRow {
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
    sort_rows(&mut rows, |r| r.p_value, |r| &r.feature);

    let mut results = SampleResults {
        analysis,
        rows,
        correction: CorrectionSummary {
            method: Correction::None,
            info: Vec::new(),
            warning: None,
        },
    };
    results.apply_correction(analysis.correction, rng);
    Some(results)
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

/// Settings of a two-group analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupAnalysis {
    pub test: GroupTest,
    pub test_type: TestType,
    pub coverage: f64,
    pub correction: Correction,
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

/// Run a two-group test on every feature of a profile. Returns `None` if
/// cancelled through `progress`.
pub fn run_group_analysis(
    profile: &GroupProfile,
    analysis: GroupAnalysis,
    settings: &StatsSettings,
    rng: &mut StampRng,
    progress: Progress<'_>,
) -> Option<GroupResults> {
    let features = profile.profile.features();
    let counts: Vec<FeatureCounts<'_>> = features
        .iter()
        .map(|feature| {
            let (seq1, seq2) = profile.counts(feature);
            let (parent1, parent2) = profile.parent_counts(feature);
            FeatureCounts {
                seq1,
                seq2,
                parent1,
                parent2,
            }
        })
        .collect();
    let outcomes = analysis
        .test
        .run_all(&counts, analysis.coverage, settings, rng, progress)?;

    let mut rows: Vec<GroupRow> = features
        .iter()
        .zip(outcomes)
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
        correction: CorrectionSummary {
            method: Correction::None,
            info: Vec::new(),
            warning: None,
        },
    };
    results.apply_correction(analysis.correction, rng);
    Some(results)
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

/// Run a multiple-group test on every feature of a profile. Returns `None`
/// if cancelled through `progress`.
pub fn run_multi_group_analysis(
    profile: &MultiGroupProfile,
    analysis: MultiGroupAnalysis,
    rng: &mut StampRng,
    progress: Progress<'_>,
) -> Option<MultiGroupResults> {
    let features = profile.profile.features();
    let mut rows = Vec::with_capacity(features.len());
    for (i, feature) in features.iter().enumerate() {
        if !progress(i, features.len()) {
            return None;
        }
        let percentages = profile.percentages(feature);
        let groups: Vec<&[f64]> = percentages.iter().map(Vec::as_slice).collect();
        let (p_value, note) = analysis.test.run(&groups);
        rows.push(MultiGroupRow {
            feature: feature.name.clone(),
            p_value,
            p_value_corrected: p_value,
            effect_size: analysis.effect_size.run(&groups),
            note: note.to_string(),
            group_means: groups.iter().map(|g| mean(g)).collect(),
            group_std_devs: groups.iter().map(|g| population_std(g)).collect(),
        });
    }
    sort_rows(&mut rows, |r| r.p_value, |r| &r.feature);

    let mut results = MultiGroupResults {
        analysis,
        rows,
        correction: CorrectionSummary {
            method: Correction::None,
            info: Vec::new(),
            warning: None,
        },
    };
    results.apply_correction(analysis.correction, rng);
    Some(results)
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
}
