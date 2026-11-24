//! Hypothesis tests comparing the proportion of a feature in two groups of
//! samples.

use super::ci::SampleCi;
use super::dist::{normal_cdf, t_cdf, t_isf};
use super::presence::{fisher_presence, mcnemar};
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
    /// Presence or absence of the feature in the samples of each group.
    FisherPresence,
    MannWhitney,
    /// Presence or absence, for samples in pairs.
    McNemar,
    PairedT,
    TTest,
    Welch,
    White,
    WilcoxonSignedRank,
}

const TOO_FEW_SAMPLES: &str = "degenerate case: both groups must contain at least 2 samples";
const NOT_PAIRED: &str =
    "degenerate case: a paired test needs the samples of the two groups in matching pairs";
const TOO_FEW_PAIRS: &str = "degenerate case: a paired test needs at least 2 pairs";
const ZERO_PARENT: &str = "degenerate case: parent group had a count of zero";
const ZERO_VARIANCE: &str = "degenerate case: variance of both groups is zero";
const WHITE_ZERO_VARIANCE: &str =
    "degenerate case: zero variance for both groups; variance set to 1e-6.";
const WHITE_FISHER: &str = "heuristic: statistics calculated with Fisher's test";

impl GroupTest {
    pub const ALL: [GroupTest; 8] = [
        GroupTest::FisherPresence,
        GroupTest::MannWhitney,
        GroupTest::McNemar,
        GroupTest::PairedT,
        GroupTest::TTest,
        GroupTest::Welch,
        GroupTest::White,
        GroupTest::WilcoxonSignedRank,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GroupTest::FisherPresence => "Fisher's exact test (presence/absence)",
            GroupTest::McNemar => "McNemar's test (presence/absence)",
            GroupTest::MannWhitney => "Mann-Whitney U test",
            GroupTest::PairedT => "Paired t-test",
            GroupTest::WilcoxonSignedRank => "Wilcoxon signed-rank test",
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
            GroupTest::FisherPresence | GroupTest::McNemar => "DP: bootstrap",
            GroupTest::MannWhitney => "DP: bootstrap",
            GroupTest::PairedT => "DP: paired t-test inverted",
            GroupTest::WilcoxonSignedRank => "DP: bootstrap",
            GroupTest::TTest => "DP: t-test inverted",
            GroupTest::Welch => "DP: Welch's inverted",
            GroupTest::White => "DP: bootstrap",
        }
    }

    /// True if the result depends on the random seed.
    pub fn is_stochastic(self) -> bool {
        // The Mann-Whitney p-values are not random; its bootstrap
        // confidence intervals are.
        matches!(
            self,
            GroupTest::White
                | GroupTest::MannWhitney
                | GroupTest::WilcoxonSignedRank
                | GroupTest::FisherPresence
                | GroupTest::McNemar
        )
    }

    /// True if the test asks only in how many samples of each group the
    /// feature is present (above zero), not how abundant it is.
    pub fn is_presence(self) -> bool {
        matches!(self, GroupTest::FisherPresence | GroupTest::McNemar)
    }

    /// True if the test compares each sample of the first group with its
    /// partner in the second: the samples of the two groups must then be
    /// given in the same order, pair by pair.
    pub fn is_paired(self) -> bool {
        matches!(
            self,
            GroupTest::PairedT | GroupTest::WilcoxonSignedRank | GroupTest::McNemar
        )
    }

    /// The one-sided and two-sided p-values of the test on values that are
    /// not proportions (transformed values, say), for which the effect
    /// size and confidence interval of [`GroupTest::run_all`] would mean
    /// nothing. `None` for White's test, which works on all features at
    /// once and on counts.
    pub fn p_values(self, values1: &[f64], values2: &[f64]) -> Option<(f64, f64)> {
        let from_tails =
            |(lower, upper): (f64, f64)| (upper.min(1.0), (2.0 * lower.min(upper)).min(1.0));
        let of = |outcome: GroupOutcome| (outcome.one_sided, outcome.two_sided);
        match self {
            GroupTest::TTest => Some(of(t_test(values1, values2, 0.95))),
            GroupTest::Welch => Some(of(welch(values1, values2, 0.95))),
            GroupTest::PairedT => Some(of(paired_t(values1, values2, 0.95))),
            GroupTest::MannWhitney => {
                if values1.is_empty() || values2.is_empty() {
                    return Some(of(too_few_samples()));
                }
                Some(from_tails(mann_whitney_tails(values1, values2)))
            }
            GroupTest::WilcoxonSignedRank => {
                if values1.len() != values2.len() || values1.len() < 2 {
                    return Some(of(too_few_samples()));
                }
                Some(from_tails(signed_rank_tails(values1, values2)))
            }
            // Counts, and presence, are not to be read from other values.
            GroupTest::White | GroupTest::FisherPresence | GroupTest::McNemar => None,
        }
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
            GroupTest::MannWhitney | GroupTest::PairedT | GroupTest::WilcoxonSignedRank => {
                let mut out = Vec::with_capacity(features.len());
                for (i, f) in features.iter().enumerate() {
                    if !progress(i, features.len()) {
                        return None;
                    }
                    let (prop1, zero1) = proportions(f.seq1, f.parent1);
                    let (prop2, zero2) = proportions(f.seq2, f.parent2);
                    let mut outcome = match self {
                        GroupTest::PairedT => paired_t(&prop1, &prop2, coverage),
                        GroupTest::WilcoxonSignedRank => {
                            wilcoxon_signed_rank(&prop1, &prop2, coverage, settings.replicates, rng)
                        }
                        _ => mann_whitney(&prop1, &prop2, coverage, settings.replicates, rng),
                    };
                    if (zero1 || zero2) && outcome.note.is_empty() {
                        outcome.note = ZERO_PARENT;
                    }
                    out.push(outcome);
                }
                Some(out)
            }
            GroupTest::FisherPresence | GroupTest::McNemar => {
                let mut out = Vec::with_capacity(features.len());
                for (i, f) in features.iter().enumerate() {
                    if !progress(i, features.len()) {
                        return None;
                    }
                    let (prop1, zero1) = proportions(f.seq1, f.parent1);
                    let (prop2, zero2) = proportions(f.seq2, f.parent2);
                    // The test says nothing about the size of a difference:
                    // the effect size and its interval are those of the
                    // rank-based test of the same design.
                    let mut outcome = if self == GroupTest::McNemar {
                        wilcoxon_signed_rank(&prop1, &prop2, coverage, settings.replicates, rng)
                    } else {
                        mann_whitney(&prop1, &prop2, coverage, settings.replicates, rng)
                    };
                    if outcome.note.is_empty() {
                        let present = |seq: &[f64]| seq.iter().filter(|&&v| v > 0.0).count();
                        let (one_sided, two_sided) = if self == GroupTest::McNemar {
                            let only = |a: &[f64], b: &[f64]| {
                                a.iter()
                                    .zip(b)
                                    .filter(|(x, y)| **x > 0.0 && **y <= 0.0)
                                    .count()
                            };
                            mcnemar(only(f.seq1, f.seq2), only(f.seq2, f.seq1))
                        } else {
                            fisher_presence(
                                present(f.seq1),
                                f.seq1.len(),
                                present(f.seq2),
                                f.seq2.len(),
                            )
                        };
                        outcome.one_sided = one_sided;
                        outcome.two_sided = two_sided;
                        if zero1 || zero2 {
                            outcome.note = ZERO_PARENT;
                        }
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

/// Outcome from the lower-tail probability of the statistic.
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

/// Outcome of a t statistic with `df` degrees of freedom. The tails are
/// computed directly rather than as one minus the CDF, so that p-values far
/// below 1e-16 keep their value.
fn t_outcome(t: f64, df: f64, dp: f64, half_width: f64, note: &'static str) -> GroupOutcome {
    GroupOutcome {
        one_sided: t_cdf(-t, df),
        two_sided: (2.0 * t_cdf(-t.abs(), df)).min(1.0),
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
    let t = dp / denom;
    let t_critical = t_isf(0.5 * (1.0 - coverage), dof);
    t_outcome(t, dof, dp, t_critical * denom, "")
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
    let t = dp / std_err;
    let t_critical = t_isf(0.5 * (1.0 - coverage), dof);
    t_outcome(t, dof, dp, t_critical * std_err, "")
}

/// Ranks of the values of two groups taken together, ties sharing the
/// average of the ranks they span. Returns the sum of the ranks of the
/// first group and the sum of `t^3 - t` over the sizes `t` of the sets of
/// tied values.
fn rank_sum(first: &[f64], second: &[f64]) -> (f64, f64) {
    let mut values: Vec<(f64, bool)> = first
        .iter()
        .map(|&v| (v, true))
        .chain(second.iter().map(|&v| (v, false)))
        .collect();
    values.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut sum, mut ties) = (0.0, 0.0);
    let mut start = 0;
    while start < values.len() {
        let mut end = start + 1;
        while end < values.len() && values[end].0 == values[start].0 {
            end += 1;
        }
        // Ranks start + 1 ..= end share their average.
        let rank = (start + 1 + end) as f64 / 2.0;
        let tied = (end - start) as f64;
        ties += tied * tied * tied - tied;
        sum += rank * values[start..end].iter().filter(|v| v.1).count() as f64;
        start = end;
    }
    (sum, ties)
}

/// Exact lower and upper tail probabilities of the Mann-Whitney statistic
/// `u` for groups of `n1` and `n2` values without ties: `P(U <= u)` and
/// `P(U >= u)`.
fn mann_whitney_exact(u: f64, n1: usize, n2: usize) -> (f64, f64) {
    let most = n1 * n2;
    let at = (u.round().max(0.0) as usize).min(most);
    // The distribution is that of the smaller group against the larger,
    // reflected when the first group is the larger: U counts the pairs in
    // which the first group's value is the larger.
    let (small, large) = (n1.min(n2), n1.max(n2));
    let counts = mann_whitney_counts(small, large);
    let count = |k: usize| {
        if n1 <= n2 {
            counts[k]
        } else {
            counts[most - k]
        }
    };
    let total: f64 = counts.iter().sum();
    (
        (0..=at).map(count).sum::<f64>() / total,
        (at..=most).map(count).sum::<f64>() / total,
    )
}

/// The number of arrangements of `m` values of one group and `n` of
/// another, without ties, for each value of the Mann-Whitney statistic
/// (0 to m * n). Every feature of an analysis has groups of the same sizes,
/// so the last counts are kept for the next feature.
fn mann_whitney_counts(m: usize, n: usize) -> std::rc::Rc<Vec<f64>> {
    /// The group sizes and their counts.
    type Counts = ((usize, usize), std::rc::Rc<Vec<f64>>);
    thread_local! {
        static LAST: std::cell::RefCell<Option<Counts>> = const { std::cell::RefCell::new(None) };
    }
    if let Some(counts) = LAST.with(|last| {
        last.borrow()
            .as_ref()
            .filter(|(key, _)| *key == (m, n))
            .map(|(_, counts)| std::rc::Rc::clone(counts))
    }) {
        return counts;
    }
    // ways[i][k], for j values of the second group so far: the number of
    // arrangements of i values of the first group and those j with
    // statistic k. The largest value is either from the first group, which
    // then exceeds all j of the second, or from the second, which adds
    // nothing: ways(i, j, k) = ways(i - 1, j, k - j) + ways(i, j - 1, k).
    // Only the rows of the current j are kept, (m + 1) * (m * n + 1)
    // numbers, where a table over i, j and k needed (n + 1) times as many.
    let most = m * n;
    let mut ways = vec![vec![0.0f64; most + 1]; m + 1];
    for row in ways.iter_mut() {
        row[0] = 1.0;
    }
    for j in 1..=n {
        for i in 1..=m {
            let (before, rest) = ways.split_at_mut(i);
            let (previous, row) = (&before[i - 1], &mut rest[0]);
            for k in j..=i * j {
                row[k] += previous[k - j];
            }
        }
    }
    let counts = std::rc::Rc::new(ways.swap_remove(m));
    LAST.with(|last| *last.borrow_mut() = Some(((m, n), std::rc::Rc::clone(&counts))));
    counts
}

/// Probabilities of a Mann-Whitney statistic at most, and at least, the
/// one observed. Neither group may be empty.
fn mann_whitney_tails(prop1: &[f64], prop2: &[f64]) -> (f64, f64) {
    let (n1, n2) = (prop1.len() as f64, prop2.len() as f64);
    let (rank_sum1, ties) = rank_sum(prop1, prop2);
    let u = rank_sum1 - n1 * (n1 + 1.0) / 2.0;
    let centre = n1 * n2 / 2.0;

    if ties == 0.0 && (prop1.len() <= 8 || prop2.len() <= 8) {
        mann_whitney_exact(u, prop1.len(), prop2.len())
    } else {
        let n = n1 + n2;
        let variance = n1 * n2 / 12.0 * ((n + 1.0) - ties / (n * (n - 1.0)));
        if variance <= 0.0 {
            // Every value is the same.
            (1.0, 1.0)
        } else {
            let sd = variance.sqrt();
            (
                normal_cdf((u - centre + 0.5) / sd),
                normal_cdf(-(u - centre - 0.5) / sd),
            )
        }
    }
}

/// The Mann-Whitney U test (Wilcoxon rank-sum test), on proportions: do
/// the values of one group tend to be larger than those of the other?
///
/// The p-value is exact when there are no ties and either group has eight
/// values or fewer, and otherwise from the normal approximation with
/// corrections for ties and for continuity, as in SciPy. The test itself
/// gives no interval for the difference between the mean proportions, so
/// that comes from the bootstrap, as for White's test.
pub fn mann_whitney(
    prop1: &[f64],
    prop2: &[f64],
    coverage: f64,
    replicates: usize,
    rng: &mut StampRng,
) -> GroupOutcome {
    if prop1.is_empty() || prop2.is_empty() {
        return too_few_samples();
    }
    let (lower, upper) = mann_whitney_tails(prop1, prop2);
    let (low, high) = bootstrap_ci(prop1, prop2, coverage, replicates.max(1), rng);
    GroupOutcome {
        one_sided: upper.min(1.0),
        two_sided: (2.0 * lower.min(upper)).min(1.0),
        lower_ci: low * 100.0,
        upper_ci: high * 100.0,
        effect_size: (mean(prop1) - mean(prop2)) * 100.0,
        note: "",
    }
}

/// Paired t-test on proportions: the samples of the two groups are taken
/// in pairs, in the order given, and the differences within the pairs are
/// tested against zero.
pub fn paired_t(prop1: &[f64], prop2: &[f64], coverage: f64) -> GroupOutcome {
    if prop1.len() != prop2.len() {
        return GroupOutcome {
            note: NOT_PAIRED,
            ..too_few_samples()
        };
    }
    if prop1.len() < 2 {
        return GroupOutcome {
            note: TOO_FEW_PAIRS,
            ..too_few_samples()
        };
    }
    let differences: Vec<f64> = prop1.iter().zip(prop2).map(|(a, b)| a - b).collect();
    let n = differences.len() as f64;
    let dp = mean(&differences);
    let std_err = (sample_variance(&differences) / n).sqrt();
    if std_err == 0.0 {
        return zero_variance(dp, 0.0);
    }
    let dof = n - 1.0;
    let t = dp / std_err;
    let t_critical = t_isf(0.5 * (1.0 - coverage), dof);
    t_outcome(t, dof, dp, t_critical * std_err, "")
}

/// Most pairs for which the distribution of the signed-rank statistic is
/// worked out exactly, as in SciPy.
const SIGNED_RANK_EXACT_PAIRS: usize = 50;

/// Probabilities of a Wilcoxon signed-rank statistic (the sum of the ranks
/// of the pairs in which the first value is the larger) at most, and at
/// least, the one observed. Pairs with no difference are left out. With 50
/// pairs or fewer the distribution is exact, ties included; otherwise it
/// is the normal approximation with the correction for ties.
pub(super) fn signed_rank_tails(prop1: &[f64], prop2: &[f64]) -> (f64, f64) {
    let mut differences: Vec<f64> = prop1
        .iter()
        .zip(prop2)
        .map(|(a, b)| a - b)
        .filter(|d| *d != 0.0)
        .collect();
    if differences.is_empty() {
        return (1.0, 1.0);
    }
    differences.sort_by(|a, b| a.abs().total_cmp(&b.abs()));
    let n = differences.len();
    // Twice the rank of each difference by size, so that the average rank
    // shared by tied differences is a whole number.
    let mut doubled = vec![0usize; n];
    let mut ties = 0.0;
    let mut start = 0;
    while start < n {
        let mut end = start + 1;
        while end < n && differences[end].abs() == differences[start].abs() {
            end += 1;
        }
        let tied = (end - start) as f64;
        ties += tied * tied * tied - tied;
        for rank in &mut doubled[start..end] {
            // Ranks start + 1 to end: their average, doubled.
            *rank = start + end + 1;
        }
        start = end;
    }
    let observed: usize = differences
        .iter()
        .zip(&doubled)
        .filter(|(d, _)| **d > 0.0)
        .map(|(_, rank)| rank)
        .sum();

    if n <= SIGNED_RANK_EXACT_PAIRS {
        // Probability of each (doubled) sum when every pair is as likely
        // to fall one way as the other.
        let total: usize = doubled.iter().sum();
        let mut probability = vec![0.0f64; total + 1];
        probability[0] = 1.0;
        let mut reached = 0;
        for &rank in &doubled {
            for sum in (0..=reached).rev() {
                let half = 0.5 * probability[sum];
                probability[sum] = half;
                probability[sum + rank] += half;
            }
            // Sums are filled from the top down, so each pair is counted
            // once.
            reached += rank;
        }
        let lower: f64 = probability[..=observed].iter().sum();
        let upper: f64 = probability[observed..].iter().sum();
        (lower.min(1.0), upper.min(1.0))
    } else {
        let n = n as f64;
        let centre = n * (n + 1.0) / 4.0;
        let variance = n * (n + 1.0) * (2.0 * n + 1.0) / 24.0 - ties / 48.0;
        let z = (observed as f64 / 2.0 - centre) / variance.sqrt();
        (normal_cdf(z), normal_cdf(-z))
    }
}

/// Wilcoxon signed-rank test on proportions: the rank-based test for
/// samples in pairs, taken in the order given.
///
/// Pairs with no difference are left out; the p-value is exact for 50
/// pairs or fewer and otherwise from the normal approximation, as in
/// SciPy. The interval for the mean difference comes from the bootstrap,
/// resampling the pairs.
pub fn wilcoxon_signed_rank(
    prop1: &[f64],
    prop2: &[f64],
    coverage: f64,
    replicates: usize,
    rng: &mut StampRng,
) -> GroupOutcome {
    if prop1.len() != prop2.len() {
        return GroupOutcome {
            note: NOT_PAIRED,
            ..too_few_samples()
        };
    }
    if prop1.len() < 2 {
        return GroupOutcome {
            note: TOO_FEW_PAIRS,
            ..too_few_samples()
        };
    }
    let (lower, upper) = signed_rank_tails(prop1, prop2);
    let differences: Vec<f64> = prop1.iter().zip(prop2).map(|(a, b)| a - b).collect();
    let n = differences.len();
    let mut distribution: Vec<f64> = (0..replicates.max(1))
        .map(|_| {
            (0..n)
                .map(|_| differences[random_index(rng, n)])
                .sum::<f64>()
                / n as f64
        })
        .collect();
    distribution.sort_by(f64::total_cmp);
    let count = distribution.len();
    let tail = 0.5 * (1.0 - coverage);
    let low = (tail * count as f64).floor() as usize;
    let high = (((coverage + tail) * count as f64).ceil() as usize).min(count - 1);
    GroupOutcome {
        one_sided: upper.min(1.0),
        two_sided: (2.0 * lower.min(upper)).min(1.0),
        lower_ci: distribution[low] * 100.0,
        upper_ci: distribution[high] * 100.0,
        effect_size: mean(&differences) * 100.0,
        note: "",
    }
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

    // A permutation that reproduces the observed split of values gives the
    // observed statistic again, up to rounding. Such ties must not count as
    // more extreme, so comparisons allow for a few units of rounding error.
    const TIE_TOLERANCE: f64 = 1e-9;
    let above = |t: f64| t + TIE_TOLERANCE * t.abs();
    let sum = |values: &[f64]| values.iter().sum::<f64>();

    // With few samples the null statistics of all frequently observed
    // features are pooled to get enough resolution, and only theirs are
    // kept; otherwise each feature is compared with its own null
    // statistics as they are made, and none are kept. (Every statistic of
    // every permutation used to be kept: replicates x features numbers.)
    let pooling = n1 < 8 || n2 < 8;
    let frequent: Vec<usize> = if pooling {
        (0..features.len())
            .filter(|&r| sum(features[r].seq1) >= n1 as f64 || sum(features[r].seq2) >= n2 as f64)
            .collect()
    } else {
        Vec::new()
    };
    let mut pooled: Vec<f64> = Vec::with_capacity(replicates * frequent.len());
    let mut more_extreme = vec![(0usize, 0usize); if pooling { 0 } else { features.len() }];

    // Null distribution of the t statistics under permutation of the samples.
    let mut order: Vec<usize> = (0..n1 + n2).collect();
    let mut shuffled = vec![0.0; n1 + n2];
    let mut permuted_statistic = |row: &[f64], order: &[usize]| {
        for (slot, &source) in shuffled.iter_mut().zip(order) {
            *slot = row[source];
        }
        white_statistic(&shuffled[..n1], &shuffled[n1..]).0
    };
    for b in 0..replicates {
        if !progress(b, total_work) {
            return None;
        }
        for i in (1..order.len()).rev() {
            order.swap(i, random_index(rng, i + 1));
        }
        if pooling {
            for &r in &frequent {
                pooled.push(permuted_statistic(&rows[r], &order));
            }
        } else {
            for (r, row) in rows.iter().enumerate() {
                let statistic = permuted_statistic(row, &order);
                let t = observed[r].0;
                let (one, two) = &mut more_extreme[r];
                *one += usize::from(statistic > above(t));
                *two += usize::from(statistic.abs() > above(t.abs()));
            }
        }
    }

    let mut one_sided = vec![0.0; features.len()];
    let mut two_sided = vec![0.0; features.len()];
    if pooling {
        let greater =
            |sorted: &[f64], value: f64| sorted.len() - sorted.partition_point(|t| *t <= value);
        let pool_size = pooled.len() as f64;
        pooled.sort_by(f64::total_cmp);
        for &r in &frequent {
            one_sided[r] = greater(&pooled, above(observed[r].0)) as f64 / pool_size;
        }
        // The same numbers, as magnitudes, for the two-sided test.
        for t in pooled.iter_mut() {
            *t = t.abs();
        }
        pooled.sort_by(f64::total_cmp);
        for &r in &frequent {
            two_sided[r] = greater(&pooled, above(observed[r].0.abs())) as f64 / pool_size;
        }
    } else {
        for (r, &(one, two)) in more_extreme.iter().enumerate() {
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
            (actual - expected).abs() <= 1e-9 * expected.abs() + 1e-12,
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

    // Reference values from R 4.4.3: `t.test(a, b, var.equal = TRUE)` and
    // `var.equal = FALSE` (Welch), with `alternative = "greater"` for the
    // one-sided p-value. The parent count is one, so the proportions are
    // the values themselves; effect sizes and intervals are in percent.
    #[test]
    fn equal_variance_t_test() {
        let r = run(
            GroupTest::TTest,
            &[5.0, 4.0, 6.0, 4.0, 3.0],
            &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
            1.0,
        );
        close(r.one_sided, 0.53714172631726331);
        close(r.two_sided, 0.92571654736547337);
        close(r.lower_ci, -245.93526827844681);
        close(r.upper_ci, 225.93526827844683);
        close(r.effect_size, -10.0);
    }

    #[test]
    fn exact_mann_whitney_needs_little_memory_and_matches_the_full_table() {
        // The full table over i, j and k, as it was computed before.
        fn full(u: f64, n1: usize, n2: usize) -> (f64, f64) {
            let most = n1 * n2;
            let mut ways = vec![vec![vec![0.0f64; most + 1]; n2 + 1]; n1 + 1];
            for (i, by_second) in ways.iter_mut().enumerate() {
                for (j, counts) in by_second.iter_mut().enumerate() {
                    if i == 0 || j == 0 {
                        counts[0] = 1.0;
                    }
                }
            }
            for i in 1..=n1 {
                for j in 1..=n2 {
                    for k in 0..=i * j {
                        let with_first = if k >= j { ways[i - 1][j][k - j] } else { 0.0 };
                        ways[i][j][k] = with_first + ways[i][j - 1][k];
                    }
                }
            }
            let counts = &ways[n1][n2];
            let total: f64 = counts.iter().sum();
            let at = (u.round() as usize).min(most);
            (
                counts[..=at].iter().sum::<f64>() / total,
                counts[at..].iter().sum::<f64>() / total,
            )
        }
        for (n1, n2) in [(1, 1), (3, 5), (5, 3), (8, 8), (2, 9), (9, 2), (7, 12)] {
            for u in 0..=n1 * n2 {
                let (a, b) = mann_whitney_exact(u as f64, n1, n2);
                let (c, d) = full(u as f64, n1, n2);
                assert!(
                    (a - c).abs() < 1e-15 && (b - d).abs() < 1e-15,
                    "{n1} {n2} {u}"
                );
            }
        }
        // 8 against 1,500 samples needed 1.3 GB per feature.
        let (lower, upper) = mann_whitney_exact(0.0, 8, 1500);
        let all = (1..=8)
            .map(|i| i as f64 / (1500 + i) as f64)
            .product::<f64>();
        assert!((lower / all - 1.0).abs() < 1e-9, "{lower} {all}");
        assert_eq!(upper, 1.0);
    }

    #[test]
    fn welch_t_test() {
        let r = run(
            GroupTest::Welch,
            &[5.0, 4.0, 6.0, 4.0, 3.0],
            &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
            1.0,
        );
        close(r.one_sided, 0.5390501783737266);
        close(r.two_sided, 0.92189964325254681);
        close(r.lower_ci, -238.02317716945791);
        close(r.upper_ci, 218.02317716945794);
        close(r.effect_size, -10.0);

        let r = run(
            GroupTest::Welch,
            &[3.4, 6.3, 5.3, 1.4, 6.3, 6.3],
            &[3.5, 6.4, 5.2, 1.3, 6.4, 6.2],
            1.0,
        );
        close(r.two_sided, 0.99999999999999933);
        close(r.lower_ci, -262.66062012243707);
        close(r.upper_ci, 262.66062012243685);
        assert!(r.effect_size.abs() < 1e-12);

        let a: Vec<f64> = (1..=10).map(f64::from).collect();
        let b: Vec<f64> = (1..=10).map(|i| f64::from(i) * 10.0).collect();
        let r = run(GroupTest::Welch, &a, &b, 1.0);
        close(r.one_sided, 0.99971463308159636);
        close(r.two_sided, 0.00057073383680729482);
        close(r.lower_ci, -7120.165010028073);
        close(r.upper_ci, -2779.8349899719277);
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

    // White's non-parametric t-test (White, Nagarajan and Pop 2009) has no
    // independent implementation to compare with: the original is an R
    // script inside Metastats, and its permutation p-values are random.
    // The tests below check its parts instead: the statistic is Welch's t
    // (so its p-values come close to Welch's), it is repeatable for a seed,
    // and sparse features fall back to Fisher's exact test, whose values
    // are checked against R in stats/two_sample.rs.
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

    /// Reference values from SciPy 1.17 (`scipy.stats.mannwhitneyu`, with
    /// its default method choice and continuity correction).
    #[test]
    fn mann_whitney_matches_scipy() {
        let run = |a: &[f64], b: &[f64]| {
            mann_whitney(a, b, 0.95, 200, &mut crate::stats::rng_from_seed(1))
        };
        let check = |a: &[f64], b: &[f64], two_sided: f64, greater: f64| {
            let outcome = run(a, b);
            assert!(
                (outcome.two_sided - two_sided).abs() < 1e-12,
                "two-sided {} against {two_sided}",
                outcome.two_sided
            );
            assert!(
                (outcome.one_sided - greater).abs() < 1e-12,
                "one-sided {} against {greater}",
                outcome.one_sided
            );
        };
        // No ties and a small group: exact.
        check(
            &[1.1, 2.2, 3.3, 4.4, 9.9],
            &[5.5, 6.6, 7.7, 8.8, 10.1, 12.0],
            0.05194805194805195,
            0.9848484848484849,
        );
        check(
            &[1.0, 2.0, 3.0],
            &[4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
            0.00909090909090909,
            1.0,
        );
        // Ties: the normal approximation, corrected for them.
        check(
            &[1.0, 2.0, 2.0, 3.0, 5.0, 5.0, 7.0],
            &[2.0, 4.0, 5.0, 6.0, 8.0, 8.0, 9.0, 10.0],
            0.054224898519599225,
            0.9794095727612359,
        );
        // Both groups larger than eight: the normal approximation.
        check(
            &[
                0.12, 0.5, 0.33, 0.9, 0.41, 0.77, 0.05, 0.62, 0.38, 0.29, 0.81,
            ],
            &[
                0.55, 0.95, 0.61, 1.2, 0.72, 0.99, 0.48, 1.5, 0.67, 0.84, 1.1, 0.58,
            ],
            0.010644786560634337,
            0.9955476863951409,
        );
        // Nothing to tell apart.
        check(&[3.0, 3.0, 3.0, 3.0], &[3.0, 3.0, 3.0], 1.0, 1.0);

        // The effect is the difference in mean proportion, in percent, and
        // its bootstrap interval contains it.
        let outcome = run(&[0.10, 0.12, 0.14, 0.11], &[0.30, 0.33, 0.29, 0.31]);
        assert!((outcome.effect_size - (11.75 - 30.75)).abs() < 1e-9);
        assert!(outcome.lower_ci < outcome.effect_size && outcome.effect_size < outcome.upper_ci);
        assert!(outcome.upper_ci < 0.0);
        assert_eq!(
            GroupTest::from_name("Mann-Whitney U test"),
            Some(GroupTest::MannWhitney)
        );
        assert!(GroupTest::MannWhitney.is_stochastic());
    }

    // Reference values for the paired tests are from SciPy 1.17
    // (`ttest_rel`, `wilcoxon`).
    const PAIRED1: [f64; 10] = [0.12, 0.30, 0.25, 0.41, 0.18, 0.22, 0.35, 0.28, 0.31, 0.16];
    const PAIRED2: [f64; 10] = [0.10, 0.22, 0.27, 0.30, 0.11, 0.20, 0.25, 0.21, 0.33, 0.09];

    #[test]
    fn paired_t_matches_scipy() {
        let outcome = paired_t(&PAIRED1, &PAIRED2, 0.95);
        close(outcome.two_sided, 0.00824389416835759);
        close(outcome.one_sided, 0.004121947084178795);
        close(outcome.effect_size, 5.0);
        close(outcome.lower_ci, 1.6446786943366667);
        close(outcome.upper_ci, 8.355321305663332);
        // Swapping the groups swaps the direction.
        let swapped = paired_t(&PAIRED2, &PAIRED1, 0.95);
        close(swapped.one_sided, 1.0 - 0.004121947084178795);
        close(swapped.two_sided, outcome.two_sided);
    }

    #[test]
    fn wilcoxon_signed_rank_matches_scipy() {
        let mut rng = rng_from_seed(1);
        let outcome = wilcoxon_signed_rank(&PAIRED1, &PAIRED2, 0.95, 1000, &mut rng);
        close(outcome.two_sided, 0.03515625);
        close(outcome.one_sided, 0.017578125);
        close(outcome.effect_size, 5.0);
        assert!(outcome.lower_ci < 5.0 && outcome.upper_ci > 5.0);
        assert!(outcome.lower_ci > 0.0, "{}", outcome.lower_ci);
        let swapped = wilcoxon_signed_rank(&PAIRED2, &PAIRED1, 0.95, 1000, &mut rng);
        close(swapped.one_sided, 0.984375);

        // Pairs without a difference are dropped and tied differences share
        // a rank; the distribution is still exact.
        let first = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let second = [1.0, 1.0, 5.0, 2.0, 5.0, 3.0, 9.0, 6.0, 9.0, 7.0, 14.0, 9.0];
        let (one, two) = GroupTest::WilcoxonSignedRank
            .p_values(&first, &second)
            .unwrap();
        close(two, 0.37109375);
        close(one, 0.185546875);

        // No pair differs: no evidence either way.
        let same = wilcoxon_signed_rank(&PAIRED1, &PAIRED1, 0.95, 100, &mut rng);
        assert_eq!((same.one_sided, same.two_sided), (1.0, 1.0));
    }

    #[test]
    fn signed_rank_normal_approximation_for_many_pairs() {
        // 60 pairs, so past the exact distribution. With differences
        // 1, -2, 3, -4 ... the positive ranks are the odd ones: W+ = 900,
        // against a centre of 915 and a variance of 18452.5.
        let first: Vec<f64> = (1..=60)
            .map(|i| if i % 2 == 1 { i as f64 } else { 0.0 })
            .collect();
        let second: Vec<f64> = (1..=60)
            .map(|i| if i % 2 == 1 { 0.0 } else { i as f64 })
            .collect();
        let (lower, upper) = signed_rank_tails(&first, &second);
        let z = (900.0 - 915.0) / 18452.5f64.sqrt();
        close(lower, normal_cdf(z));
        close(upper, 1.0 - normal_cdf(z));
    }

    #[test]
    fn paired_tests_need_pairs() {
        let mut rng = rng_from_seed(1);
        assert!(GroupTest::PairedT.is_paired() && GroupTest::WilcoxonSignedRank.is_paired());
        assert!(!GroupTest::Welch.is_paired());
        let unequal = paired_t(&PAIRED1, &PAIRED2[..9], 0.95);
        assert_eq!(unequal.note, NOT_PAIRED);
        assert_eq!(unequal.two_sided, 1.0);
        let unequal = wilcoxon_signed_rank(&PAIRED1, &PAIRED2[..9], 0.95, 10, &mut rng);
        assert_eq!(unequal.note, NOT_PAIRED);
        let one = paired_t(&PAIRED1[..1], &PAIRED2[..1], 0.95);
        assert_eq!(one.note, TOO_FEW_PAIRS);
        // Through the interface used by the analyses.
        let outcome = run(
            GroupTest::PairedT,
            &[12.0, 30.0, 25.0],
            &[10.0, 22.0, 27.0],
            100.0,
        );
        assert!(outcome.note.is_empty() && outcome.two_sided < 1.0);
        // The same p-values are available for values that are not
        // proportions; White's test has no such form.
        let (one, two) = GroupTest::PairedT.p_values(&PAIRED1, &PAIRED2).unwrap();
        close(two, 0.00824389416835759);
        close(one, 0.004121947084178795);
        assert!(GroupTest::White.p_values(&PAIRED1, &PAIRED2).is_none());
        let direct = mann_whitney(&PAIRED1, &PAIRED2, 0.95, 10, &mut rng);
        let (one, two) = GroupTest::MannWhitney.p_values(&PAIRED1, &PAIRED2).unwrap();
        assert_eq!((one, two), (direct.one_sided, direct.two_sided));
    }
}
