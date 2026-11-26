//! Hypothesis tests comparing a feature across more than two groups.

use super::dist::{chi2_sf, f_sf};
use super::{mean, sample_variance};

/// Statistical tests for multiple groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MultiGroupTest {
    Anova,
    /// Welch's analysis of variance: compares the group means without
    /// assuming the groups are equally spread out.
    WelchAnova,
    KruskalWallis,
    /// For matched groups: the same subjects (or sites) sampled once in
    /// every group.
    Friedman,
    /// Presence or absence of the feature in the samples of each group.
    ChiSquarePresence,
}

/// Effect-size measures for multiple groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MultiGroupEffectSize {
    EtaSquared,
}

impl MultiGroupEffectSize {
    pub const ALL: [MultiGroupEffectSize; 1] = [MultiGroupEffectSize::EtaSquared];

    pub fn name(self) -> &'static str {
        "Eta-squared"
    }

    pub fn run(self, groups: &[&[f64]]) -> f64 {
        super::effect_size::eta_squared(groups)
    }
}

impl MultiGroupTest {
    pub const ALL: [MultiGroupTest; 5] = [
        MultiGroupTest::Anova,
        MultiGroupTest::ChiSquarePresence,
        MultiGroupTest::Friedman,
        MultiGroupTest::KruskalWallis,
        MultiGroupTest::WelchAnova,
    ];

    pub fn name(self) -> &'static str {
        match self {
            MultiGroupTest::Anova => "ANOVA",
            MultiGroupTest::WelchAnova => "Welch's ANOVA",
            MultiGroupTest::KruskalWallis => "Kruskal-Wallis H-test",
            MultiGroupTest::Friedman => "Friedman test",
            MultiGroupTest::ChiSquarePresence => "Chi-square test (presence/absence)",
        }
    }

    pub fn from_name(name: &str) -> Option<MultiGroupTest> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// True for a test of matched groups, which takes the samples of every
    /// group in the same order of subjects (sample `i` of each group came
    /// from subject `i`).
    pub fn is_matched(self) -> bool {
        self == MultiGroupTest::Friedman
    }

    /// True if the test asks only in how many samples of each group the
    /// feature is present (above zero), not how abundant it is.
    pub fn is_presence(self) -> bool {
        self == MultiGroupTest::ChiSquarePresence
    }

    /// p-value for the null hypothesis that all groups have the same
    /// distribution of the feature, with a note for degenerate cases.
    pub fn run(self, groups: &[&[f64]]) -> (f64, &'static str) {
        match self {
            MultiGroupTest::Anova => anova(groups),
            MultiGroupTest::WelchAnova => welch_anova(groups),
            MultiGroupTest::KruskalWallis => kruskal_wallis(groups),
            MultiGroupTest::Friedman => friedman(groups),
            MultiGroupTest::ChiSquarePresence => {
                let counts: Vec<(usize, usize)> = groups
                    .iter()
                    .map(|g| (g.iter().filter(|&&v| v > 0.0).count(), g.len()))
                    .collect();
                super::presence::chi_square_presence(&counts)
            }
        }
    }
}

const ZERO_WITHIN_VARIANCE: &str =
    "degenerate case: no variance within groups, so any difference between them is taken as real";
const TOO_FEW_SAMPLES: &str = "degenerate case: at least one group contains less than 2 samples";
const FAILED: &str = "degenerate case: failed to calculate p-value";

/// One-way analysis of variance.
fn anova(groups: &[&[f64]]) -> (f64, &'static str) {
    if groups.iter().any(|g| g.len() < 2) {
        return (1.0, TOO_FEW_SAMPLES);
    }
    let n: f64 = groups.iter().map(|g| g.len() as f64).sum();
    let k = groups.len() as f64;

    // Centre the data on the grand mean to reduce rounding error.
    let offset = groups.iter().flat_map(|g| g.iter()).sum::<f64>() / n;
    let mut sum = 0.0;
    let mut sum_of_squares = 0.0;
    let mut between = 0.0;
    for group in groups {
        let mut group_sum = 0.0;
        for &x in group.iter() {
            let centred = x - offset;
            group_sum += centred;
            sum_of_squares += centred * centred;
        }
        sum += group_sum;
        between += group_sum * group_sum / group.len() as f64;
    }
    let correction = sum * sum / n;
    let total = sum_of_squares - correction;
    let between = between - correction;
    let within = total - between;

    let (df_between, df_within) = (k - 1.0, n - k);
    // Groups that are constant within but differ between: the difference
    // in rounding error decides the sign of `within`, and a negative F used
    // to give p = 1. Any difference is real, as Welch's test takes it.
    if total > 0.0 && within <= 1e-12 * total {
        return (0.0, ZERO_WITHIN_VARIANCE);
    }
    let f = (between / df_between) / (within / df_within);
    let p = f_sf(f, df_between, df_within);
    if p.is_nan() {
        (1.0, FAILED)
    } else {
        (p, "")
    }
}

/// Welch's analysis of variance (Welch 1951), as R's
/// `oneway.test(var.equal = FALSE)` computes it: each group is weighted by
/// its size over its variance, so that groups need not be equally spread
/// out, and the F statistic has adjusted denominator degrees of freedom.
/// With two groups it gives the p-value of Welch's t-test.
///
/// A group whose values are all the same has no variance and so an
/// infinite weight, which R cannot use. The test is then taken at its
/// limit as the variance of such groups shrinks to zero (together, and
/// equal, if there are several): their common value becomes the weighted
/// mean and the other groups are compared with it, as Welch's t-test does
/// with one constant group. If
/// constant groups differ from each other, any difference is taken as
/// real (p = 0), as for ANOVA.
fn welch_anova(groups: &[&[f64]]) -> (f64, &'static str) {
    if groups.len() < 2 {
        return (1.0, "degenerate case: fewer than two groups");
    }
    if groups.iter().any(|g| g.len() < 2) {
        return (1.0, TOO_FEW_SAMPLES);
    }
    // (size, mean, variance); variance None for a constant group.
    let stats: Vec<(f64, f64, Option<f64>)> = groups
        .iter()
        .map(|g| {
            let n = g.len() as f64;
            if g.iter().all(|&v| v == g[0]) {
                (n, g[0], None)
            } else {
                (n, mean(g), Some(sample_variance(g)))
            }
        })
        .collect();
    let constant: Vec<(f64, f64)> = stats
        .iter()
        .filter(|s| s.2.is_none())
        .map(|s| (s.0, s.1))
        .collect();
    if let Some(&(_, first)) = constant.first() {
        if constant.iter().any(|&(_, m)| m != first) {
            return (0.0, ZERO_WITHIN_VARIANCE);
        }
        if constant.len() == stats.len() {
            // Every value is the same: nothing to compare.
            return (1.0, FAILED);
        }
    }
    let k = stats.len() as f64;
    // The weighted grand mean and the share of the total weight each group
    // carries; constant groups share all of it, in proportion to their size.
    let (grand_mean, shares): (f64, Vec<f64>) = if constant.is_empty() {
        let weights: Vec<f64> = stats.iter().map(|s| s.0 / s.2.unwrap()).collect();
        let total: f64 = weights.iter().sum();
        let grand_mean = stats
            .iter()
            .zip(&weights)
            .map(|(s, w)| w * s.1)
            .sum::<f64>()
            / total;
        (grand_mean, weights.iter().map(|w| w / total).collect())
    } else {
        let size: f64 = constant.iter().map(|c| c.0).sum();
        let shares = stats
            .iter()
            .map(|s| if s.2.is_none() { s.0 / size } else { 0.0 })
            .collect();
        (constant[0].1, shares)
    };
    let between: f64 = stats
        .iter()
        .filter_map(|&(n, m, var)| var.map(|v| n / v * (m - grand_mean) * (m - grand_mean)))
        .sum::<f64>()
        / (k - 1.0);
    let lambda: f64 = stats
        .iter()
        .zip(&shares)
        .map(|(s, share)| (1.0 - share) * (1.0 - share) / (s.0 - 1.0))
        .sum();
    let f = between / (1.0 + 2.0 * (k - 2.0) / (k * k - 1.0) * lambda);
    let df_within = (k * k - 1.0) / (3.0 * lambda);
    let p = f_sf(f, k - 1.0, df_within);
    if p.is_nan() {
        (1.0, FAILED)
    } else {
        (p, "")
    }
}

/// Brown-Forsythe test of whether the groups are equally spread out:
/// Levene's test on the distances of the values from their group's median
/// (Brown and Forsythe 1974), which is robust to skewed data. It is an
/// ANOVA of those distances, as R's `car::leveneTest` and
/// `DescTools::LeveneTest` compute it with their default `center = median`.
/// A small p-value says that the spread differs between the groups, the
/// case for Welch's ANOVA and the Games-Howell post-hoc test. The p-value
/// is NaN when the test cannot be made: fewer than two groups, or a group
/// with fewer than two samples.
pub fn brown_forsythe(groups: &[&[f64]]) -> (f64, &'static str) {
    if groups.len() < 2 {
        return (f64::NAN, "degenerate case: fewer than two groups");
    }
    if groups.iter().any(|g| g.len() < 2) {
        return (f64::NAN, TOO_FEW_SAMPLES);
    }
    let distances: Vec<Vec<f64>> = groups
        .iter()
        .map(|g| {
            let mut sorted = g.to_vec();
            sorted.sort_by(f64::total_cmp);
            let n = sorted.len();
            let median = if n == 0 {
                0.0
            } else if n % 2 == 1 {
                sorted[n / 2]
            } else {
                (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
            };
            g.iter().map(|v| (v - median).abs()).collect()
        })
        .collect();
    let distances: Vec<&[f64]> = distances.iter().map(Vec::as_slice).collect();
    match anova(&distances) {
        (_, ZERO_WITHIN_VARIANCE) => (0.0, SPREAD_CONSTANT_WITHIN),
        (_, FAILED) => (1.0, NO_SPREAD),
        outcome => outcome,
    }
}

const SPREAD_CONSTANT_WITHIN: &str = "degenerate case: in each group every value lies equally far \
    from the median, so any difference in spread between the groups is taken as real";
const NO_SPREAD: &str = "degenerate case: every group has the same spread (none or equal)";

/// Kruskal-Wallis H-test, with correction for ties, as R's `kruskal.test`
/// computes it: for groups of any size, the p-value from the chi-square
/// approximation. Groups without samples are left out. (STAMP v2 refused
/// groups of fewer than five samples and reported p = 1.)
fn kruskal_wallis(groups: &[&[f64]]) -> (f64, &'static str) {
    let groups: Vec<&[f64]> = groups.iter().copied().filter(|g| !g.is_empty()).collect();
    if groups.len() < 2 {
        return (
            1.0,
            "degenerate case: fewer than two groups contain samples",
        );
    }
    let all: Vec<f64> = groups.iter().flat_map(|g| g.iter().copied()).collect();
    let n = all.len() as f64;

    // Average ranks, and the tie correction 1 - sum(t^3 - t) / (n^3 - n).
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by(|&a, &b| all[a].total_cmp(&all[b]));
    let mut ranks = vec![0.0; all.len()];
    let mut tie_sum = 0.0;
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && all[order[end]] == all[order[start]] {
            end += 1;
        }
        let average_rank = (start + 1 + end) as f64 / 2.0;
        for &index in &order[start..end] {
            ranks[index] = average_rank;
        }
        let t = (end - start) as f64;
        tie_sum += t * t * t - t;
        start = end;
    }
    let ties = 1.0 - tie_sum / (n * n * n - n);
    if ties == 0.0 {
        // Every value is identical.
        return (1.0, "Invalid input data for Kruskal-Wallis H-test.");
    }

    let mut rank_term = 0.0;
    let mut offset = 0;
    for group in &groups {
        let rank_sum: f64 = ranks[offset..offset + group.len()].iter().sum();
        rank_term += rank_sum * rank_sum / group.len() as f64;
        offset += group.len();
    }
    let h = (12.0 / (n * (n + 1.0)) * rank_term - 3.0 * (n + 1.0)) / ties;
    (chi2_sf(h, groups.len() as f64 - 1.0), "")
}

/// Friedman test, with correction for ties, as R's `friedman.test` and
/// SciPy's `friedmanchisquare` compute it: the values of each subject are
/// ranked across the groups, and the rank sums of the groups compared; the
/// p-value comes from the chi-square approximation. The groups must hold
/// the subjects in the same order.
fn friedman(groups: &[&[f64]]) -> (f64, &'static str) {
    let k = groups.len();
    let n = groups.first().map_or(0, |g| g.len());
    if groups.iter().any(|g| g.len() != n) {
        return (
            1.0,
            "degenerate case: the Friedman test needs every subject sampled once in each group",
        );
    }
    if k < 2 || n < 2 {
        return (
            1.0,
            "degenerate case: the Friedman test needs at least 2 groups and 2 subjects",
        );
    }
    let mut rank_sums = vec![0.0; k];
    let mut tie_sum = 0.0;
    let mut order: Vec<usize> = (0..k).collect();
    // `subject` indexes every group, not `groups` itself.
    #[allow(clippy::needless_range_loop)]
    for subject in 0..n {
        order.sort_by(|&a, &b| groups[a][subject].total_cmp(&groups[b][subject]));
        let mut start = 0;
        while start < k {
            let mut end = start + 1;
            while end < k && groups[order[end]][subject] == groups[order[start]][subject] {
                end += 1;
            }
            let average_rank = (start + 1 + end) as f64 / 2.0;
            for &group in &order[start..end] {
                rank_sums[group] += average_rank;
            }
            let t = (end - start) as f64;
            tie_sum += t * t * t - t;
            start = end;
        }
    }
    let (kf, nf) = (k as f64, n as f64);
    let ties = 1.0 - tie_sum / (kf * (kf * kf - 1.0) * nf);
    if ties <= 0.0 {
        // Every subject has the same value in every group.
        return (
            1.0,
            "degenerate case: no subject differs between the groups",
        );
    }
    let squares: f64 = rank_sums.iter().map(|r| r * r).sum();
    let statistic = (12.0 / (kf * nf * (kf + 1.0)) * squares - 3.0 * nf * (kf + 1.0)) / ties;
    (chi2_sf(statistic, kf - 1.0), "")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `scipy.stats.friedmanchisquare` of SciPy 1.18.1 (statistics 6.2308
    /// and 6.75); the first three groups have a tie within a subject.
    #[test]
    fn friedman_values() {
        use MultiGroupTest::Friedman;
        let a = [1.2, 3.4, 2.2, 5.1, 0.8, 2.9, 4.4];
        let b = [2.3, 3.9, 2.2, 6.0, 1.9, 3.3, 4.1];
        let c = [0.9, 2.8, 3.1, 4.0, 0.2, 2.9, 3.0];
        let d = [1.5, 3.0, 2.6, 5.5, 1.1, 2.0, 4.9];
        close(p(Friedman, &[&a, &b, &c]), 0.044361441772906844);
        close(p(Friedman, &[&a, &b, &c, &d]), 0.08030772655502616);
        // The order of the subjects does not matter, as long as it is the
        // same in every group.
        let reversed = |g: &[f64]| g.iter().rev().copied().collect::<Vec<f64>>();
        close(
            p(Friedman, &[&reversed(&a), &reversed(&b), &reversed(&c)]),
            0.044361441772906844,
        );
        // Groups that are not matched, and subjects that never differ.
        let (value, note) = Friedman.run(&[&a, &b[..6], &c]);
        assert!(value == 1.0 && note.contains("every subject"));
        let (value, note) = Friedman.run(&[&a, &a, &a]);
        assert!(value == 1.0 && note.contains("no subject differs"));
        assert!(Friedman.is_matched() && !MultiGroupTest::Anova.is_matched());
    }

    fn p(test: MultiGroupTest, groups: &[&[f64]]) -> f64 {
        test.run(groups).0
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-9 * expected.abs() + 1e-14,
            "expected {expected}, got {actual}"
        );
    }

    // Reference values from R 4.4.3: `oneway.test(y ~ g, var.equal = TRUE)`
    // for ANOVA and `kruskal.test(y, g)` for the Kruskal-Wallis test.
    #[test]
    fn anova_values() {
        use MultiGroupTest::Anova;
        close(
            p(
                Anova,
                &[
                    &[5.0, 4.0, 6.0, 4.0, 3.0],
                    &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
                    &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
                ],
            ),
            0.88347274205030968,
        );
        close(
            p(
                Anova,
                &[
                    &[1.0, 2.0, 3.0, 4.0, 5.0],
                    &[10.0, 20.0, 30.0, 40.0, 50.0],
                    &[4.0, 5.0, 4.0],
                    &[5.0, 5.0, 5.0],
                ],
            ),
            0.001874082303100263,
        );
        close(
            p(
                Anova,
                &[
                    &[5.0, 4.0, 5.0, 4.0, 5.0],
                    &[6.0, 5.0, 6.0, 5.0, 6.0, 5.0],
                    &[700.0, 800.0, 700.0],
                ],
            ),
            3.0661478703352848e-13,
        );
        let same: &[f64] = &[1.0, 2.0, 3.0, 4.0, 5.0];
        close(p(Anova, &[same, same, same]), 1.0);
    }

    #[test]
    fn kruskal_wallis_values() {
        use MultiGroupTest::KruskalWallis;
        close(
            p(
                KruskalWallis,
                &[
                    &[5.0, 4.0, 6.0, 4.0, 3.0],
                    &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
                    &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
                ],
            ),
            0.88173680194259973,
        );
        let a: Vec<f64> = (1..=7).map(f64::from).collect();
        let b: Vec<f64> = (8..=15).map(f64::from).collect();
        let c: Vec<f64> = (16..=22).map(f64::from).collect();
        close(p(KruskalWallis, &[&a, &b, &c]), 8.8020161301173387e-05);
        let same: &[f64] = &[1.0, 2.0, 3.0, 4.0, 5.0];
        close(p(KruskalWallis, &[same, same, same]), 1.0);
    }

    /// Small groups, which STAMP v2 refused (reporting p = 1). Reference
    /// values from R 4.4.3: `kruskal.test(y, g)$p.value`.
    #[test]
    fn kruskal_wallis_small_groups_match_r() {
        use MultiGroupTest::KruskalWallis;
        close(
            p(
                KruskalWallis,
                &[&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 4.0, 5.0]],
            ),
            0.61829172084641981,
        );
        close(
            p(KruskalWallis, &[&[1.5, 2.5], &[3.0, 4.0, 5.0], &[0.5]]),
            0.11731916609425083,
        );
        close(
            p(
                KruskalWallis,
                &[&[1.0, 1.0, 2.0], &[2.0, 3.0, 3.0], &[3.0, 4.0, 4.0, 4.0]],
            ),
            0.024445959738605923,
        );
        close(p(KruskalWallis, &[&[1.0], &[2.0]]), 0.31731050786291415);
        // An empty group is left out, as R drops unused factor levels.
        close(
            p(KruskalWallis, &[&[1.5, 2.5], &[], &[3.0, 4.0, 5.0], &[0.5]]),
            0.11731916609425083,
        );
        let (_, note) = KruskalWallis.run(&[&[1.5, 2.5], &[3.0, 4.0, 5.0], &[0.5]]);
        assert_eq!(note, "");
    }

    /// Reference values from R 4.4.3: `oneway.test(y ~ g, var.equal =
    /// FALSE)$p.value` (tests/reference/reference_values.R).
    #[test]
    fn welch_anova_values() {
        use MultiGroupTest::WelchAnova;
        close(
            p(
                WelchAnova,
                &[
                    &[5.0, 4.0, 6.0, 4.0, 3.0],
                    &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
                    &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
                ],
            ),
            0.90207413662903557,
        );
        // Very different spreads: ANOVA gives 0.0019 on similar data.
        let b: Vec<f64> = (1..=6).map(|v| f64::from(v) * 10.0).collect();
        let c: Vec<f64> = (1..=7).map(f64::from).collect();
        close(
            p(WelchAnova, &[&[1.0, 2.0, 3.0, 4.0, 5.0], &b, &c]),
            0.009199536279174754,
        );
        close(
            p(
                WelchAnova,
                &[
                    &[1.2, 3.4, 2.2, 5.1, 0.8],
                    &[2.3, 3.9, 2.2, 6.0, 1.9, 3.3, 4.1],
                    &[0.9, 2.8, 3.1],
                    &[10.5, 1.5, 3.0, 2.6, 5.5, 1.1],
                ],
            ),
            0.57754712534516406,
        );
        close(
            p(
                WelchAnova,
                &[
                    &[0.0, 0.0, 0.0, 0.5, 1.2, 0.0],
                    &[0.1, 0.3, 0.2, 0.4],
                    &[2.5, 0.0, 4.1, 0.7, 0.0, 3.3, 1.9],
                ],
            ),
            0.11403024903955505,
        );
        // With two groups it is Welch's t-test (R: t.test(a, b)$p.value).
        let a = [5.0, 4.0, 6.0, 4.0, 3.0];
        let b = [5.0, 2.0, 2.0, 5.0, 6.0, 7.0];
        close(p(WelchAnova, &[&a, &b]), 0.92189964325254681);
        let welch_t = super::super::two_group::welch(&a, &b, 0.95).two_sided;
        close(p(WelchAnova, &[&a, &b]), welch_t);
        assert_eq!(MultiGroupTest::from_name("Welch's ANOVA"), Some(WelchAnova));
    }

    /// A group without variance has an infinite weight; the test takes the
    /// limit, which slightly spread values approach.
    #[test]
    fn welch_anova_constant_group_is_the_limit() {
        use MultiGroupTest::WelchAnova;
        let b = [0.1, 0.3, 0.2, 0.4];
        let c = [2.5, 0.0, 4.1, 0.7, 0.0, 3.3, 1.9];
        let exact = p(WelchAnova, &[&[0.0; 6], &b, &c]);
        assert!(exact > 0.0 && exact < 1.0, "{exact}");
        for scale in [1e-4, 1e-6] {
            let nearly = [0.0, scale, 0.0, -scale, scale, -scale];
            let approx = p(WelchAnova, &[&nearly, &b, &c]);
            assert!(
                (approx - exact).abs() < 1e3 * scale,
                "{scale}: {approx} {exact}"
            );
        }
        // Two constant groups with the same value share the infinite
        // weight in proportion to their size: the limit as their variances
        // shrink to zero while staying equal (here 4/3 s^2 both).
        let exact = p(WelchAnova, &[&[0.0; 6], &[0.0; 4], &c]);
        let s = 1e-6;
        let a = s * (5.0f64 / 3.0).sqrt();
        let approx = p(
            WelchAnova,
            &[&[a, -a, a, -a, 0.0, 0.0], &[s, -s, s, -s], &c],
        );
        assert!((approx - exact).abs() < 1e-6, "{approx} {exact}");
        // With two groups, one constant: as Welch's t-test.
        let welch_t = super::super::two_group::welch(&[0.0; 6], &c, 0.95).two_sided;
        close(p(WelchAnova, &[&[0.0; 6], &c]), welch_t);
        // Constant groups that differ: any difference is real.
        let (value, note) = WelchAnova.run(&[&[0.0; 3], &[1.0; 3], &c]);
        assert_eq!((value, note), (0.0, ZERO_WITHIN_VARIANCE));
        // Every value the same.
        let (value, note) = WelchAnova.run(&[&[2.0; 3], &[2.0; 3]]);
        assert_eq!((value, note), (1.0, FAILED));
        // Too few samples, or groups.
        let (value, note) = WelchAnova.run(&[&[1.0], &b, &c]);
        assert_eq!((value, note), (1.0, TOO_FEW_SAMPLES));
        assert_eq!(WelchAnova.run(&[&c]).0, 1.0);
    }

    /// Reference values from R 4.4.3 with DescTools 0.99.59:
    /// `LeveneTest(y, g, center = median)` (tests/reference/reference_values.R).
    #[test]
    fn brown_forsythe_values() {
        let pb = |groups: &[&[f64]]| brown_forsythe(groups).0;
        close(
            pb(&[
                &[5.0, 4.0, 6.0, 4.0, 3.0],
                &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0],
                &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            ]),
            0.40195578443559232,
        );
        let b: Vec<f64> = (1..=6).map(|v| f64::from(v) * 10.0).collect();
        let c: Vec<f64> = (1..=7).map(f64::from).collect();
        close(
            pb(&[&[1.0, 2.0, 3.0, 4.0, 5.0], &b, &c]),
            0.00046871397214314452,
        );
        close(
            pb(&[
                &[1.2, 3.4, 2.2, 5.1, 0.8],
                &[2.3, 3.9, 2.2, 6.0, 1.9, 3.3, 4.1],
                &[0.9, 2.8, 3.1],
                &[10.5, 1.5, 3.0, 2.6, 5.5, 1.1],
            ]),
            0.52081402577507796,
        );
        close(
            pb(&[&[5.0, 4.0, 6.0, 4.0, 3.0], &[5.0, 2.0, 2.0, 5.0, 6.0, 7.0]]),
            0.34857387891436054,
        );
        close(
            pb(&[
                &[0.0, 0.0, 0.0, 0.5, 1.2, 0.0],
                &[0.1, 0.3, 0.2, 0.4],
                &[2.5, 0.0, 4.1, 0.7, 0.0, 3.3, 1.9],
            ]),
            0.0070069385551000283,
        );
        // Shifting a group does not change its spread.
        let shifted: Vec<f64> = c.iter().map(|v| v + 100.0).collect();
        close(
            pb(&[&[1.0, 2.0, 3.0, 4.0, 5.0], &b, &shifted]),
            0.00046871397214314452,
        );
    }

    #[test]
    fn brown_forsythe_degenerate_cases() {
        // No spread anywhere.
        assert_eq!(brown_forsythe(&[&[1.0; 3], &[2.0; 4]]), (1.0, NO_SPREAD));
        // Groups of two always lie equally far from their median.
        assert_eq!(
            brown_forsythe(&[&[1.0, 3.0], &[1.0, 9.0], &[0.0, 0.5]]),
            (0.0, SPREAD_CONSTANT_WITHIN)
        );
        // Not possible: NaN.
        let (value, note) = brown_forsythe(&[&[1.0], &[1.0, 2.0, 4.0]]);
        assert!(value.is_nan() && note == TOO_FEW_SAMPLES);
        assert!(brown_forsythe(&[&[1.0, 2.0, 4.0]]).0.is_nan());
    }

    #[test]
    fn degenerate_cases() {
        let (p, note) = MultiGroupTest::Anova.run(&[&[1.0], &[1.0, 2.0]]);
        assert_eq!(p, 1.0);
        assert!(note.contains("less than 2 samples"));
        // No variation at all: the F statistic is undefined.
        let (p, note) = MultiGroupTest::Anova.run(&[&[1.0, 1.0], &[1.0, 1.0]]);
        assert_eq!(
            (p, note),
            (1.0, "degenerate case: failed to calculate p-value")
        );

        // Constant within each group but different between them, whatever
        // the rounding error.
        for (a, b) in [(0.1, 0.3), (0.2, 0.7), (1.0 / 3.0, 0.9), (12.5, 12.75)] {
            let (p, note) = MultiGroupTest::Anova.run(&[&[a; 3], &[b; 3]]);
            assert_eq!((p, note), (0.0, ZERO_WITHIN_VARIANCE), "{a} {b}");
        }

        // Fewer than two groups with samples.
        let (p, note) = MultiGroupTest::KruskalWallis.run(&[&[1.0, 2.0, 3.0], &[]]);
        assert_eq!(p, 1.0);
        assert!(note.contains("fewer than two groups"), "{note}");

        let constant: &[f64] = &[2.0; 5];
        let (p, note) = MultiGroupTest::KruskalWallis.run(&[constant, constant]);
        assert_eq!(
            (p, note),
            (1.0, "Invalid input data for Kruskal-Wallis H-test.")
        );
    }
}
