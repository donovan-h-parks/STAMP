//! Hypothesis tests comparing a feature across more than two groups.

use super::dist::{chi2_sf, f_sf};

/// Statistical tests for multiple groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MultiGroupTest {
    Anova,
    KruskalWallis,
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
    pub const ALL: [MultiGroupTest; 2] = [MultiGroupTest::Anova, MultiGroupTest::KruskalWallis];

    pub fn name(self) -> &'static str {
        match self {
            MultiGroupTest::Anova => "ANOVA",
            MultiGroupTest::KruskalWallis => "Kruskal-Wallis H-test",
        }
    }

    pub fn from_name(name: &str) -> Option<MultiGroupTest> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// p-value for the null hypothesis that all groups have the same
    /// distribution of the feature, with a note for degenerate cases.
    pub fn run(self, groups: &[&[f64]]) -> (f64, &'static str) {
        match self {
            MultiGroupTest::Anova => anova(groups),
            MultiGroupTest::KruskalWallis => kruskal_wallis(groups),
        }
    }
}

/// One-way analysis of variance.
fn anova(groups: &[&[f64]]) -> (f64, &'static str) {
    if groups.iter().any(|g| g.len() < 2) {
        return (
            1.0,
            "degenerate case: at least one group contains less than 2 samples",
        );
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
    let f = (between / df_between) / (within / df_within);
    let p = f_sf(f, df_between, df_within);
    if p.is_nan() {
        (1.0, "degenerate case: failed to calculate p-value")
    } else {
        (p, "")
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
