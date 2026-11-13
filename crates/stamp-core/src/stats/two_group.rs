//! Hypothesis tests comparing the proportion of a feature in two groups of
//! samples.

use super::ci::SampleCi;
use super::dist::{t_cdf, t_isf};
use super::two_sample::{random_index, SampleTest};
use super::{sample_variance, StampRng, StatsSettings};

/// Result of a two-group test on one feature. The effect size is the
/// difference between the mean proportions of the groups, in percent, and the
/// confidence interval is for that difference.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupOutcome {
    pub one_sided: f64,
    pub two_sided: f64,
    pub lower_ci: f64,
    pub upper_ci: f64,
    pub effect_size: f64,
    pub note: &'static str,
}

/// Statistical tests for two groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupTest {
    TTest,
    Welch,
    White,
}

const TOO_FEW_SAMPLES: &str = "degenerate case: both groups must contain at least 2 samples";
const ZERO_PARENT: &str = "degenerate case: parent group had a count of zero";
const ZERO_VARIANCE: &str = "degenerate case: variance of both groups is zero";
const WHITE_ZERO_VARIANCE: &str =
    "degenerate case: zero variance for both groups; variance set to 1e-6.";
const WHITE_FISHER: &str = "heuristic: statistics calculated with Fisher's test";

impl GroupTest {
    pub const ALL: [GroupTest; 3] = [GroupTest::TTest, GroupTest::Welch, GroupTest::White];

    pub fn name(self) -> &'static str {
        match self {
            GroupTest::TTest => "t-test (equal variance)",
            GroupTest::Welch => "Welch's t-test",
            GroupTest::White => "White's non-parametric t-test",
        }
    }

    pub fn from_name(name: &str) -> Option<GroupTest> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Name of the confidence-interval method that goes with the test.
    pub fn ci_method(self) -> &'static str {
        match self {
            GroupTest::TTest => "DP: t-test inverted",
            GroupTest::Welch => "DP: Welch's inverted",
            GroupTest::White => "DP: bootstrap",
        }
    }

    /// True if the result depends on the random seed.
    pub fn is_stochastic(self) -> bool {
        self == GroupTest::White
    }

    /// Run the test on every feature. For each feature, `seq1` and `parent1`
    /// hold the feature and parent counts of the samples in group 1, and
    /// likewise for group 2.
    ///
    /// `progress(done, total)` is called as work proceeds; returning `false`
    /// cancels the run, in which case `None` is returned.
    pub fn run_all(
        self,
        features: &[FeatureCounts<'_>],
        coverage: f64,
        settings: &StatsSettings,
        rng: &mut StampRng,
        progress: &mut dyn FnMut(usize, usize) -> bool,
    ) -> Option<Vec<GroupOutcome>> {
        match self {
            GroupTest::TTest | GroupTest::Welch => {
                let mut out = Vec::with_capacity(features.len());
                for (i, f) in features.iter().enumerate() {
                    if !progress(i, features.len()) {
                        return None;
                    }
                    let (prop1, zero1) = proportions(f.seq1, f.parent1);
                    let (prop2, zero2) = proportions(f.seq2, f.parent2);
                    let mut outcome = if self == GroupTest::TTest {
                        t_test(&prop1, &prop2, coverage)
                    } else {
                        welch(&prop1, &prop2, coverage)
                    };
                    if (zero1 || zero2) && outcome.note.is_empty() {
                        outcome.note = ZERO_PARENT;
                    }
                    out.push(outcome);
                }
                Some(out)
            }
            GroupTest::White => white(features, coverage, settings, rng, progress),
        }
    }
}

/// Counts of one feature, and of its parent category, in the samples of two
/// groups.
#[derive(Debug, Clone, Copy)]
pub struct FeatureCounts<'a> {
    pub seq1: &'a [f64],
    pub seq2: &'a [f64],
    pub parent1: &'a [f64],
    pub parent2: &'a [f64],
}

/// Proportions of a feature in each sample, and whether any parent was empty.
fn proportions(seq: &[f64], parent: &[f64]) -> (Vec<f64>, bool) {
    let mut any_zero = false;
    let values = seq
        .iter()
        .zip(parent)
        .map(|(&s, &p)| {
            if p > 0.0 {
                s / p
            } else {
                any_zero = true;
                0.0
            }
        })
        .collect();
    (values, any_zero)
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn too_few_samples() -> GroupOutcome {
    GroupOutcome {
        one_sided: 0.5,
        two_sided: 1.0,
        lower_ci: 0.0,
        upper_ci: 0.0,
        effect_size: 0.0,
        note: TOO_FEW_SAMPLES,
    }
}

/// Outcome from the lower-tail probability of the t statistic.
fn outcome(p_lower: f64, dp: f64, half_width: f64, note: &'static str) -> GroupOutcome {
    GroupOutcome {
        one_sided: 1.0 - p_lower,
        two_sided: 2.0 * p_lower.min(1.0 - p_lower),
        lower_ci: (dp - half_width) * 100.0,
        upper_ci: (dp + half_width) * 100.0,
        effect_size: dp * 100.0,
        note,
    }
}

/// With no variance in either group, any difference in means is taken as real.
fn zero_variance(mean1: f64, mean2: f64) -> GroupOutcome {
    let p_lower = if mean1 != mean2 { 0.0 } else { 0.5 };
    outcome(p_lower, mean1 - mean2, 0.0, ZERO_VARIANCE)
}

/// Student's t-test assuming equal variances, on proportions.
pub fn t_test(prop1: &[f64], prop2: &[f64], coverage: f64) -> GroupOutcome {
    let (n1, n2) = (prop1.len() as f64, prop2.len() as f64);
    if prop1.len() < 2 || prop2.len() < 2 {
        return too_few_samples();
    }
    let (mean1, mean2) = (mean(prop1), mean(prop2));
    let dof = n1 + n2 - 2.0;
    let pooled_var =
        ((n1 - 1.0) * sample_variance(prop1) + (n2 - 1.0) * sample_variance(prop2)) / dof;
    let denom = pooled_var.sqrt() * (1.0 / n1 + 1.0 / n2).sqrt();
    if denom == 0.0 {
        return zero_variance(mean1, mean2);
    }
    let dp = mean1 - mean2;
    let p_lower = t_cdf(dp / denom, dof);
    let t_critical = t_isf(0.5 * (1.0 - coverage), dof);
    outcome(p_lower, dp, t_critical * denom, "")
}

/// Welch's t-test (unequal variances), on proportions.
pub fn welch(prop1: &[f64], prop2: &[f64], coverage: f64) -> GroupOutcome {
    let (n1, n2) = (prop1.len() as f64, prop2.len() as f64);
    if prop1.len() < 2 || prop2.len() < 2 {
        return too_few_samples();
    }
    let (mean1, mean2) = (mean(prop1), mean(prop2));
    let norm_var1 = sample_variance(prop1) / n1;
    let norm_var2 = sample_variance(prop2) / n2;
    let unpooled_var = norm_var1 + norm_var2;
    if unpooled_var == 0.0 {
        return zero_variance(mean1, mean2);
    }
    let std_err = unpooled_var.sqrt();
    let dp = mean1 - mean2;
    let dof = unpooled_var * unpooled_var
        / (norm_var1 * norm_var1 / (n1 - 1.0) + norm_var2 * norm_var2 / (n2 - 1.0));
    let p_lower = t_cdf(dp / std_err, dof);
    let t_critical = t_isf(0.5 * (1.0 - coverage), dof);
    outcome(p_lower, dp, t_critical * std_err, "")
}

/// t statistic with unpooled variance for one feature.
fn white_statistic(prop1: &[f64], prop2: &[f64]) -> (f64, f64, &'static str) {
    let dp = mean(prop1) - mean(prop2);
    let denom = (sample_variance(prop1) / prop1.len() as f64
        + sample_variance(prop2) / prop2.len() as f64)
        .sqrt();
    if denom == 0.0 {
        (dp / 1e-6, dp, WHITE_ZERO_VARIANCE)
    } else {
        (dp / denom, dp, "")
    }
}

/// Bootstrap confidence interval for the difference in mean proportion.
fn bootstrap_ci(
    prop1: &[f64],
    prop2: &[f64],
    coverage: f64,
    replicates: usize,
    rng: &mut StampRng,
) -> (f64, f64) {
    let resampled_mean = |values: &[f64], rng: &mut StampRng| {
        (0..values.len())
            .map(|_| values[random_index(rng, values.len())])
            .sum::<f64>()
            / values.len() as f64
    };
    let mut distribution: Vec<f64> = (0..replicates)
        .map(|_| resampled_mean(prop1, rng) - resampled_mean(prop2, rng))
        .collect();
    distribution.sort_by(f64::total_cmp);

    let n = distribution.len();
    let tail = 0.5 * (1.0 - coverage);
    let lower = (tail * n as f64).floor() as usize;
    let upper = (((coverage + tail) * n as f64).ceil() as usize).min(n - 1);
    (distribution[lower], distribution[upper])
}

/// White's non-parametric t-test (White et al., 2009): a permutation test on
/// the t statistic, with Fisher's exact test for sparse features and a
/// bootstrap confidence interval.
fn white(
    features: &[FeatureCounts<'_>],
    coverage: f64,
    settings: &StatsSettings,
    rng: &mut StampRng,
    progress: &mut dyn FnMut(usize, usize) -> bool,
) -> Option<Vec<GroupOutcome>> {
    let Some(first) = features.first() else {
        return Some(Vec::new());
    };
    let (n1, n2) = (first.seq1.len(), first.seq2.len());
    let replicates = settings.replicates.max(1);
    let total_work = replicates + features.len();

    // Each feature's proportions, group 1 followed by group 2.
    let rows: Vec<Vec<f64>> = features
        .iter()
        .map(|f| {
            let mut row = proportions(f.seq1, f.parent1).0;
            row.extend(proportions(f.seq2, f.parent2).0);
            row
        })
        .collect();

    let observed: Vec<(f64, f64, &'static str)> = rows
        .iter()
        .map(|row| white_statistic(&row[..n1], &row[n1..]))
        .collect();

    // Null distribution of the t statistics under permutation of the samples.
    let mut permuted: Vec<Vec<f64>> = Vec::with_capacity(replicates);
    let mut order: Vec<usize> = (0..n1 + n2).collect();
    let mut shuffled = vec![0.0; n1 + n2];
    for b in 0..replicates {
        if !progress(b, total_work) {
            return None;
        }
        for i in (1..order.len()).rev() {
            order.swap(i, random_index(rng, i + 1));
        }
        let statistics = rows
            .iter()
            .map(|row| {
                for (slot, &source) in shuffled.iter_mut().zip(&order) {
                    *slot = row[source];
                }
                white_statistic(&shuffled[..n1], &shuffled[n1..]).0
            })
            .collect();
        permuted.push(statistics);
    }

    // A permutation that reproduces the observed split of values gives the
    // observed statistic again, up to rounding. Such ties must not count as
    // more extreme, so comparisons allow for a few units of rounding error.
    const TIE_TOLERANCE: f64 = 1e-9;
    let above = |t: f64| t + TIE_TOLERANCE * t.abs();

    let sum = |values: &[f64]| values.iter().sum::<f64>();
    let mut one_sided = vec![0.0; features.len()];
    let mut two_sided = vec![0.0; features.len()];
    if n1 < 8 || n2 < 8 {
        // Few samples: pool the null statistics of all frequently observed
        // features to get enough resolution.
        let frequent: Vec<usize> = (0..features.len())
            .filter(|&r| sum(features[r].seq1) >= n1 as f64 || sum(features[r].seq2) >= n2 as f64)
            .collect();
        let mut pooled: Vec<f64> = Vec::with_capacity(replicates * frequent.len());
        for statistics in &permuted {
            pooled.extend(frequent.iter().map(|&r| statistics[r]));
        }
        let mut pooled_abs: Vec<f64> = pooled.iter().map(|t| t.abs()).collect();
        pooled.sort_by(f64::total_cmp);
        pooled_abs.sort_by(f64::total_cmp);
        let greater =
            |sorted: &[f64], value: f64| sorted.len() - sorted.partition_point(|t| *t <= value);

        let pool_size = pooled.len() as f64;
        for &r in &frequent {
            let t = observed[r].0;
            one_sided[r] = greater(&pooled, above(t)) as f64 / pool_size;
            two_sided[r] = greater(&pooled_abs, above(t.abs())) as f64 / pool_size;
        }
    } else {
        for r in 0..features.len() {
            let t = observed[r].0;
            let one = permuted.iter().filter(|s| s[r] > above(t)).count();
            let two = permuted
                .iter()
                .filter(|s| s[r].abs() > above(t.abs()))
                .count();
            one_sided[r] = (one + 1) as f64 / (replicates + 1) as f64;
            two_sided[r] = (two + 1) as f64 / (replicates + 1) as f64;
        }
    }

    let mut out = Vec::with_capacity(features.len());
    for (r, feature) in features.iter().enumerate() {
        if !progress(replicates + r, total_work) {
            return None;
        }
        let (lower, upper) =
            bootstrap_ci(&rows[r][..n1], &rows[r][n1..], coverage, replicates, rng);
        let mut outcome = GroupOutcome {
            one_sided: one_sided[r],
            two_sided: two_sided[r],
            lower_ci: lower * 100.0,
            upper_ci: upper * 100.0,
            effect_size: observed[r].1 * 100.0,
            note: observed[r].2,
        };

        // Sparse features are tested on their pooled counts instead.
        let (seq1, seq2) = (sum(feature.seq1), sum(feature.seq2));
        if seq1 < n1 as f64 && seq2 < n2 as f64 {
            let (parent1, parent2) = (sum(feature.parent1), sum(feature.parent2));
            let fisher = SampleTest::Fisher.run(seq1, seq2, parent1, parent2, settings, rng);
            let ci = SampleCi::DiffBetweenPropAsymptoticCc.run(
                seq1,
                seq2,
                parent1,
                parent2,
                coverage,
                settings.pseudocount,
            );
            outcome = GroupOutcome {
                one_sided: fisher.one_sided,
                two_sided: fisher.two_sided,
                lower_ci: ci.lower,
                upper_ci: ci.upper,
                effect_size: ci.effect_size,
                note: WHITE_FISHER,
            };
        }
        out.push(outcome);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rng_from_seed;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 5e-8,
            "expected {expected}, got {actual}"
        );
    }

    fn run(test: GroupTest, seq1: &[f64], seq2: &[f64], parent: f64) -> GroupOutcome {
        let (parent1, parent2) = (vec![parent; seq1.len()], vec![parent; seq2.len()]);
        let features = [FeatureCounts {
            seq1,
            seq2,
            parent1: &parent1,
            parent2: &parent2,
        }];
        test.run_all(
            &features,
            0.95,
            &StatsSettings::default(),
            &mut rng_from_seed(5),
            &mut |_, _| true,
        )
        .unwrap()
        .remove(0)
    }

    // Expected values from the original STAMP unit tests.
    #[test]
    fn equal_variance_t_test() {
        let r = run(
            GroupTest::TTest,
            &[5.0, 4.0, 6.0, 4.0, 3.0],
            &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
            1.0,
        );
        close(r.one_sided, 0.537141726);
        close(r.two_sided, 0.925716547365);
        close(r.lower_ci, -245.935268272);
        close(r.upper_ci, 225.935268272);
        close(r.effect_size, -10.0);
    }

    #[test]
    fn welch_t_test() {
        let r = run(
            GroupTest::Welch,
            &[5.0, 4.0, 6.0, 4.0, 3.0],
            &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
            1.0,
        );
        close(r.one_sided, 0.5390501783);
        close(r.two_sided, 0.9218996432);
        close(r.lower_ci, -238.023177152);
        close(r.upper_ci, 218.023177152);
        close(r.effect_size, -10.0);

        let r = run(
            GroupTest::Welch,
            &[3.4, 6.3, 5.3, 1.4, 6.3, 6.3],
            &[3.5, 6.4, 5.2, 1.3, 6.4, 6.2],
            1.0,
        );
        close(r.one_sided, 0.5);
        close(r.two_sided, 1.0);
        close(r.lower_ci, -262.6606201199);
        close(r.upper_ci, 262.6606201199);
        close(r.effect_size, 0.0);

        let a: Vec<f64> = (1..=10).map(f64::from).collect();
        let b: Vec<f64> = (1..=10).map(|i| f64::from(i) * 10.0).collect();
        let r = run(GroupTest::Welch, &a, &b, 1.0);
        close(r.one_sided, 0.9997146330);
        close(r.two_sided, 0.0005707338);
        close(r.lower_ci, -7120.16500998);
        close(r.upper_ci, -2779.83499002);
        close(r.effect_size, -4950.0);
    }

    #[test]
    fn degenerate_groups() {
        for test in [GroupTest::TTest, GroupTest::Welch] {
            let r = run(test, &[1.0], &[2.0, 3.0], 10.0);
            assert_eq!(
                (r.two_sided, r.effect_size, r.note),
                (1.0, 0.0, TOO_FEW_SAMPLES)
            );

            // No variance but different means: taken as a real difference.
            let r = run(test, &[5.0, 5.0], &[6.0, 6.0, 6.0], 10.0);
            assert_eq!(r.note, ZERO_VARIANCE, "{test:?}");
            assert_eq!(r.two_sided, 0.0);
            close(r.effect_size, -10.0);
            close(r.lower_ci, -10.0);
            close(r.upper_ci, -10.0);

            let r = run(test, &[5.0, 5.0], &[5.0, 5.0], 10.0);
            assert_eq!((r.two_sided, r.effect_size), (1.0, 0.0));

            let r = run(test, &[1.0, 2.0], &[2.0, 5.0], 0.0);
            assert_eq!(r.note, ZERO_VARIANCE);
        }
        let features = [FeatureCounts {
            seq1: &[1.0, 2.0],
            seq2: &[2.0, 5.0],
            parent1: &[0.0, 10.0],
            parent2: &[10.0, 10.0],
        }];
        let r = GroupTest::Welch
            .run_all(
                &features,
                0.95,
                &StatsSettings::default(),
                &mut rng_from_seed(1),
                &mut |_, _| true,
            )
            .unwrap();
        assert_eq!(r[0].note, ZERO_PARENT);
    }

    #[test]
    fn white_test_constant_groups() {
        let r = run(GroupTest::White, &[5.0; 5], &[6.0; 8], 10.0);
        close(r.two_sided, 0.0);
        close(r.lower_ci, -10.0);
        close(r.upper_ci, -10.0);
        close(r.effect_size, -10.0);
    }

    #[test]
    fn white_test_agrees_roughly_with_welch_and_is_reproducible() {
        let seq1 = [30.0, 35.0, 28.0, 33.0, 31.0, 36.0, 29.0, 34.0, 32.0, 30.0];
        let seq2 = [20.0, 24.0, 19.0, 25.0, 22.0, 21.0, 23.0, 18.0, 26.0, 22.0];
        let white = run(GroupTest::White, &seq1, &seq2, 100.0);
        let welch = run(GroupTest::Welch, &seq1, &seq2, 100.0);
        close(white.effect_size, welch.effect_size);
        assert!(white.two_sided < 0.01, "{}", white.two_sided);
        assert!((white.lower_ci - welch.lower_ci).abs() < 1.5);
        assert!((white.upper_ci - welch.upper_ci).abs() < 1.5);
        assert_eq!(white, run(GroupTest::White, &seq1, &seq2, 100.0));
    }

    #[test]
    fn white_test_uses_fisher_for_sparse_features_and_can_be_cancelled() {
        let r = run(
            GroupTest::White,
            &[0.0, 1.0, 0.0, 0.0, 1.0],
            &[0.0, 0.0, 1.0, 0.0, 0.0],
            100.0,
        );
        assert_eq!(r.note, WHITE_FISHER);
        assert!(r.two_sided > 0.5);

        let features = [FeatureCounts {
            seq1: &[1.0, 2.0],
            seq2: &[2.0, 5.0],
            parent1: &[10.0, 10.0],
            parent2: &[10.0, 10.0],
        }];
        let cancelled = GroupTest::White.run_all(
            &features,
            0.95,
            &StatsSettings::default(),
            &mut rng_from_seed(1),
            &mut |done, _| done < 10,
        );
        assert!(cancelled.is_none());
    }
}
