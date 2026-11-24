//! Post-hoc tests: pairwise comparisons of groups after a multiple-group test.

use super::dist::{f_ppf, f_sf, normal_cdf, studentized_range_cdf, studentized_range_ppf};
use super::two_group::{paired_t, signed_rank_tails, welch};
use super::{mean, sample_variance};
use crate::error::{Result, StampError};

/// Post-hoc tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostHocTest {
    Dunn,
    GamesHowell,
    Scheffe,
    TukeyKramer,
    WelchUncorrected,
    /// For matched groups (after the Friedman test).
    WilcoxonHolm,
}

/// p-value of a pairwise comparison. It is `NaN` where the comparison could
/// not be made, such as for a group of one sample under Games-Howell.
///
/// STAMP v2 reported the tests based on the studentized range as a bound
/// read from tables (`< 0.05`); they are now computed, like the others.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostHocP {
    Value(f64),
}

impl std::fmt::Display for PostHocP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PostHocP::Value(v) = self;
        write!(f, "{v:?}")
    }
}

/// One pairwise comparison. The effect size is the difference between the
/// group means, with its confidence interval.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// `"<group i> : <group j>"`
    pub label: String,
    pub p_value: PostHocP,
    pub effect_size: f64,
    pub lower_ci: f64,
    pub upper_ci: f64,
}

/// All pairwise comparisons for one feature.
#[derive(Debug, Clone, PartialEq)]
pub struct PostHocResult {
    /// The test that made the comparisons.
    pub test: PostHocTest,
    pub comparisons: Vec<Comparison>,
    pub note: &'static str,
}

const ZERO_WITHIN_VARIANCE: &str = "degenerate case: within group variance is zero; set to 1e-6.";
const ZERO_GROUP_VARIANCE: &str =
    "degenerate case: two groups with zero variance; variance set to 1e-6.";
const SINGLE_SAMPLE_GROUP: &str =
    "degenerate case: a group with fewer than 2 samples cannot be compared";

impl PostHocTest {
    pub const ALL: [PostHocTest; 6] = [
        PostHocTest::Dunn,
        PostHocTest::GamesHowell,
        PostHocTest::Scheffe,
        PostHocTest::TukeyKramer,
        PostHocTest::WelchUncorrected,
        PostHocTest::WilcoxonHolm,
    ];

    /// True if the p-values allow for the number of pairs compared; false
    /// only for Welch's test, which leaves each pair uncorrected.
    pub fn corrects_for_pairs(self) -> bool {
        self != PostHocTest::WelchUncorrected
    }

    pub fn name(self) -> &'static str {
        match self {
            PostHocTest::Dunn => "Dunn's test (Bonferroni)",
            PostHocTest::GamesHowell => "Games-Howell",
            PostHocTest::Scheffe => "Scheffè",
            PostHocTest::TukeyKramer => "Tukey-Kramer",
            PostHocTest::WelchUncorrected => "Welch's (uncorrected)",
            PostHocTest::WilcoxonHolm => "Wilcoxon signed-rank (Holm)",
        }
    }

    /// True for a test of matched groups, which takes the samples of every
    /// group in the same order of subjects.
    pub fn is_matched(self) -> bool {
        self == PostHocTest::WilcoxonHolm
    }

    pub fn from_name(name: &str) -> Option<PostHocTest> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Compare every pair of groups. `groups` holds the relative frequencies
    /// (%) of the feature in the samples of each group.
    ///
    /// The studentized-range tests support coverages of 0.9, 0.95, 0.98,
    /// 0.99 and 0.999 only.
    pub fn run(
        self,
        groups: &[&[f64]],
        coverage: f64,
        group_names: &[&str],
    ) -> Result<PostHocResult> {
        if groups.len() != group_names.len() {
            return Err(StampError::new("One name is required for each group."));
        }
        let (stats, note) = match self {
            PostHocTest::Dunn => dunn(groups, coverage)?,
            PostHocTest::GamesHowell => games_howell(groups, coverage)?,
            PostHocTest::Scheffe => scheffe(groups, coverage),
            PostHocTest::TukeyKramer => tukey_kramer(groups, coverage)?,
            PostHocTest::WelchUncorrected => welch_uncorrected(groups, coverage),
            PostHocTest::WilcoxonHolm => wilcoxon_holm(groups, coverage)?,
        };

        let mut comparisons = Vec::with_capacity(stats.len());
        let mut stats = stats.into_iter();
        for i in 0..groups.len() {
            for j in i + 1..groups.len() {
                let (p_value, effect_size, lower_ci, upper_ci) =
                    stats.next().expect("one result per pair");
                comparisons.push(Comparison {
                    label: format!("{} : {}", group_names[i], group_names[j]),
                    p_value,
                    effect_size,
                    lower_ci,
                    upper_ci,
                });
            }
        }
        Ok(PostHocResult {
            test: self,
            comparisons,
            note,
        })
    }
}

/// (p-value, effect size, lower CI, upper CI) for each pair, in order.
type PairStats = Vec<(PostHocP, f64, f64, f64)>;

/// p-value and critical value of the studentized range for `k` groups and
/// `df` degrees of freedom: the probability of a statistic above `q`, and
/// the statistic exceeded with probability `1 - coverage`.
///
/// With fewer than one degree of freedom nothing can be concluded: the
/// p-value is one and the critical value effectively infinite, as in STAMP
/// v2.
fn studentized_range(q: f64, k: usize, df: f64, coverage: f64) -> (f64, f64) {
    if k < 2 || df.is_nan() || df < 1.0 {
        return (1.0, 1000.0);
    }
    let k = k as f64;
    (
        (1.0 - studentized_range_cdf(q, k, df)).clamp(0.0, 1.0),
        studentized_range_ppf(coverage, k, df),
    )
}

fn tukey_kramer(groups: &[&[f64]], coverage: f64) -> Result<(PairStats, &'static str)> {
    check_coverage(coverage)?;
    let k = groups.len();
    let n: usize = groups.iter().map(|g| g.len()).sum();
    let df = n as f64 - k as f64;
    let (_, q_cv) = studentized_range(0.0, k, df, coverage);

    let within_var = pooled_sum_of_squares(groups) / df;
    let mut within_sd = within_var.sqrt();
    let mut note = "";
    if within_sd == 0.0 {
        note = ZERO_WITHIN_VARIANCE;
        within_sd = 1e-6;
    }

    let mut stats = Vec::new();
    for i in 0..k {
        for j in i + 1..k {
            let scale =
                ((1.0 / groups[i].len() as f64 + 1.0 / groups[j].len() as f64) / 2.0).sqrt();
            let es = mean(groups[i]) - mean(groups[j]);
            let q = es.abs() / (within_sd * scale);
            let p = if df < 1.0 {
                1.0
            } else {
                (1.0 - studentized_range_cdf(q, k as f64, df)).clamp(0.0, 1.0)
            };
            let half_width = q_cv * within_sd * scale;
            stats.push((PostHocP::Value(p), es, es - half_width, es + half_width));
        }
    }
    Ok((stats, note))
}

fn games_howell(groups: &[&[f64]], coverage: f64) -> Result<(PairStats, &'static str)> {
    check_coverage(coverage)?;
    let k = groups.len();
    let mut note = "";

    let mut stats = Vec::new();
    for i in 0..k {
        for j in i + 1..k {
            let (ni, nj) = (groups[i].len() as f64, groups[j].len() as f64);
            let es = mean(groups[i]) - mean(groups[j]);
            if groups[i].len() < 2 || groups[j].len() < 2 {
                // The variance of a single sample is undefined.
                note = SINGLE_SAMPLE_GROUP;
                stats.push((PostHocP::Value(f64::NAN), es, f64::NAN, f64::NAN));
                continue;
            }

            let mut vn1 = sample_variance(groups[i]) / ni;
            let mut vn2 = sample_variance(groups[j]) / nj;
            if vn1 == 0.0 {
                note = ZERO_GROUP_VARIANCE;
                vn1 = 1e-6;
            }
            if vn2 == 0.0 {
                note = ZERO_GROUP_VARIANCE;
                vn2 = 1e-6;
            }
            let df = (vn1 + vn2) * (vn1 + vn2) / (vn1 * vn1 / (ni - 1.0) + vn2 * vn2 / (nj - 1.0));

            let scale = ((vn1 + vn2) / 2.0).sqrt();
            let q = es.abs() / scale;
            let (p, q_cv) = studentized_range(q, k, df, coverage);
            let half_width = q_cv * scale;
            stats.push((PostHocP::Value(p), es, es - half_width, es + half_width));
        }
    }
    Ok((stats, note))
}

/// Sum over the groups of the squared deviations from the group mean. A
/// group of a single sample has none to add; STAMP v2 took its variance,
/// which is undefined, and so lost the whole test.
fn pooled_sum_of_squares(groups: &[&[f64]]) -> f64 {
    groups
        .iter()
        .filter(|g| g.len() > 1)
        .map(|g| (g.len() as f64 - 1.0) * sample_variance(g))
        .sum()
}

fn check_coverage(coverage: f64) -> Result<()> {
    if coverage > 0.0 && coverage < 1.0 {
        Ok(())
    } else {
        Err(StampError::new(
            "The coverage of a confidence interval must lie between 0 and 1.",
        ))
    }
}

fn scheffe(groups: &[&[f64]], coverage: f64) -> (PairStats, &'static str) {
    let k = groups.len();
    let n: usize = groups.iter().map(|g| g.len()).sum();
    let df_between = k as f64 - 1.0;
    let df_within = n as f64 - k as f64;
    let critical = df_between * f_ppf(coverage, df_between, df_within);

    let mut within_var = pooled_sum_of_squares(groups) / df_within;
    let within_sd = within_var.sqrt();
    let mut note = "";
    if within_var == 0.0 {
        note = ZERO_WITHIN_VARIANCE;
        within_var = 1e-6;
    }

    let mut stats = Vec::new();
    for i in 0..k {
        for j in i + 1..k {
            let es = mean(groups[i]) - mean(groups[j]);
            let inverse_sizes = 1.0 / groups[i].len() as f64 + 1.0 / groups[j].len() as f64;
            let f = es * es / (within_var * inverse_sizes);
            // Without degrees of freedom within the groups (every group a
            // single sample) there is no test, as for Tukey-Kramer.
            let p = if df_within < 1.0 {
                1.0
            } else {
                f_sf(f / df_between, df_between, df_within)
            };
            let half_width = (critical * inverse_sizes).sqrt() * within_sd;
            stats.push((PostHocP::Value(p), es, es - half_width, es + half_width));
        }
    }
    (stats, note)
}

/// Dunn's test: the rank-based post-hoc test that goes with the
/// Kruskal-Wallis test. All values are ranked together, and each pair of
/// groups is compared by the difference between their mean ranks, with a
/// correction for ties. p-values are multiplied by the number of pairs
/// (the Bonferroni adjustment), which is how the test is usually reported.
///
/// The test says nothing about the size of a difference, so the effect
/// size is the difference between the mean proportions, as for the other
/// post-hoc tests, and its interval is Welch's at the confidence level
/// that the same adjustment gives each pair.
fn dunn(groups: &[&[f64]], coverage: f64) -> Result<(PairStats, &'static str)> {
    check_coverage(coverage)?;
    let k = groups.len();
    let pairs = (k * k.saturating_sub(1) / 2).max(1) as f64;
    let n: usize = groups.iter().map(|g| g.len()).sum();

    // Rank every value, ties sharing the average of their ranks.
    let mut values: Vec<(f64, usize)> = groups
        .iter()
        .enumerate()
        .flat_map(|(group, values)| values.iter().map(move |&v| (v, group)))
        .collect();
    values.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut rank_sums = vec![0.0; k];
    let mut ties = 0.0;
    let mut start = 0;
    while start < values.len() {
        let mut end = start + 1;
        while end < values.len() && values[end].0 == values[start].0 {
            end += 1;
        }
        let rank = (start + 1 + end) as f64 / 2.0;
        let tied = (end - start) as f64;
        ties += tied * tied * tied - tied;
        for &(_, group) in &values[start..end] {
            rank_sums[group] += rank;
        }
        start = end;
    }
    let n = n as f64;
    let spread = n * (n + 1.0) / 12.0 - ties / (12.0 * (n - 1.0));

    let pair_coverage = 1.0 - (1.0 - coverage) / pairs;
    let as_proportions = |g: &[f64]| g.iter().map(|x| x / 100.0).collect::<Vec<f64>>();
    let mut note = "";
    let mut stats = Vec::new();
    for i in 0..k {
        for j in i + 1..k {
            let (ni, nj) = (groups[i].len() as f64, groups[j].len() as f64);
            let es = mean(groups[i]) - mean(groups[j]);
            let standard_error = (spread * (1.0 / ni + 1.0 / nj)).sqrt();
            let p = if groups[i].is_empty() || groups[j].is_empty() {
                f64::NAN
            } else if standard_error > 0.0 {
                let z = (rank_sums[i] / ni - rank_sums[j] / nj) / standard_error;
                (2.0 * normal_cdf(-z.abs()) * pairs).min(1.0)
            } else {
                // Every value is the same.
                1.0
            };
            let interval = welch(
                &as_proportions(groups[i]),
                &as_proportions(groups[j]),
                pair_coverage,
            );
            let (lower, upper) = if groups[i].len() < 2 || groups[j].len() < 2 {
                // Welch's interval needs a variance for both groups.
                note = SINGLE_SAMPLE_GROUP;
                (f64::NAN, f64::NAN)
            } else {
                (interval.lower_ci, interval.upper_ci)
            };
            stats.push((PostHocP::Value(p), es, lower, upper));
        }
    }
    Ok((stats, note))
}

/// For matched groups: the Wilcoxon signed-rank test on each pair of groups
/// (exact for 50 subjects or fewer), with the p-values corrected for the
/// number of pairs by Holm's method, as R's `pairwise.wilcox.test(paired =
/// TRUE)` does by default.
///
/// The effect size is the mean difference within the subjects, and its
/// interval that of the paired t-test at the confidence level a Bonferroni
/// adjustment gives each pair.
fn wilcoxon_holm(groups: &[&[f64]], coverage: f64) -> Result<(PairStats, &'static str)> {
    check_coverage(coverage)?;
    let k = groups.len();
    let n = groups.first().map_or(0, |g| g.len());
    if groups.iter().any(|g| g.len() != n) {
        return Err(StampError::new(
            "The Wilcoxon signed-rank post-hoc test needs matched groups: every subject sampled \
             once in each group.",
        ));
    }
    let pairs = (k * k.saturating_sub(1) / 2).max(1) as f64;
    let pair_coverage = 1.0 - (1.0 - coverage) / pairs;
    let as_proportions = |g: &[f64]| g.iter().map(|x| x / 100.0).collect::<Vec<f64>>();
    let mut raw = Vec::new();
    for i in 0..k {
        for j in i + 1..k {
            let p = if n < 2 {
                f64::NAN
            } else {
                let (lower, upper) = signed_rank_tails(groups[i], groups[j]);
                (2.0 * lower.min(upper)).min(1.0)
            };
            let interval = paired_t(
                &as_proportions(groups[i]),
                &as_proportions(groups[j]),
                pair_coverage,
            );
            raw.push((p, mean(groups[i]) - mean(groups[j]), interval));
        }
    }
    // Holm: the smallest p-value times the number of pairs, the next by
    // one fewer, and so on, none below the one before.
    let mut order: Vec<usize> = (0..raw.len()).filter(|&i| !raw[i].0.is_nan()).collect();
    order.sort_by(|&a, &b| raw[a].0.total_cmp(&raw[b].0));
    let mut corrected: Vec<f64> = raw.iter().map(|r| r.0).collect();
    let mut largest: f64 = 0.0;
    for (rank, &i) in order.iter().enumerate() {
        largest = largest.max(((order.len() - rank) as f64 * raw[i].0).min(1.0));
        corrected[i] = largest;
    }
    let note = if n < 2 { SINGLE_SAMPLE_GROUP } else { "" };
    Ok((
        raw.into_iter()
            .zip(corrected)
            .map(|((_, effect, interval), p)| {
                let (lower, upper) = if n < 2 {
                    (f64::NAN, f64::NAN)
                } else {
                    (interval.lower_ci, interval.upper_ci)
                };
                (PostHocP::Value(p), effect, lower, upper)
            })
            .collect(),
        note,
    ))
}

fn welch_uncorrected(groups: &[&[f64]], coverage: f64) -> (PairStats, &'static str) {
    let mut note = "";
    let mut stats = Vec::new();
    for i in 0..groups.len() {
        for j in i + 1..groups.len() {
            // Welch's test works on proportions and reports percentages.
            let as_proportions = |g: &[f64]| g.iter().map(|x| x / 100.0).collect::<Vec<f64>>();
            let r = welch(
                &as_proportions(groups[i]),
                &as_proportions(groups[j]),
                coverage,
            );
            if !r.note.is_empty() {
                note = r.note;
            }
            stats.push((
                PostHocP::Value(r.two_sided),
                r.effect_size,
                r.lower_ci,
                r.upper_ci,
            ));
        }
    }
    (stats, note)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: [f64; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
    const B: [f64; 6] = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
    const C: [f64; 7] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];

    fn run(test: PostHocTest) -> PostHocResult {
        test.run(&[&A, &B, &C], 0.95, &["1", "2", "3"]).unwrap()
    }

    fn check(c: &Comparison, label: &str, effect: f64, lower: f64, upper: f64) {
        check_within(c, label, effect, lower, upper, 1e-9);
    }

    /// As `check`, allowing the confidence limits to differ by `tolerance`.
    fn check_within(
        c: &Comparison,
        label: &str,
        effect: f64,
        lower: f64,
        upper: f64,
        tolerance: f64,
    ) {
        assert_eq!(c.label, label);
        for (actual, expected) in [
            (c.effect_size, effect),
            (c.lower_ci, lower),
            (c.upper_ci, upper),
        ] {
            assert!(
                (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
                "{label}: expected {expected}, got {actual}"
            );
        }
    }

    fn p_values(r: &PostHocResult) -> Vec<f64> {
        r.comparisons
            .iter()
            .map(|c| {
                let PostHocP::Value(p) = c.p_value;
                p
            })
            .collect()
    }

    fn close_all(actual: &[f64], expected: &[f64], tolerance: f64) {
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a - e).abs() <= tolerance * e.abs().max(1e-3),
                "expected {e}, got {a}"
            );
        }
    }

    // The groups A, B and C above, as `y <- c(A, B, C)` and
    // `g <- factor(rep(1:3, c(5, 6, 7)))` in R 4.4.3. R reports group j
    // minus group i, so its signs are the reverse of ours.

    /// `PMCMRplus::gamesHowellTest(y, g)` (1.9.12) for the p-values. Its
    /// intervals are not reported, so they are computed in R from the
    /// definition: the difference of the means plus or minus q times
    /// `sqrt((v1 / n1 + v2 / n2) / 2)`, with Welch's degrees of freedom and
    /// q solving `ptukey(q, 3, df) = 0.95` (`uniroot`, as `qtukey` is only
    /// accurate to about four decimals). The studentized range here agrees
    /// with R's to about 2e-7, hence the tolerance.
    #[test]
    fn games_howell_values() {
        let r = run(PostHocTest::GamesHowell);
        check_within(
            &r.comparisons[0],
            "1 : 2",
            -32.0,
            -56.809019695967692,
            -7.1909803040323119,
            1e-6,
        );
        check_within(
            &r.comparisons[1],
            "1 : 3",
            -1.0,
            -3.9625910350677422,
            1.9625910350677422,
            1e-6,
        );
        check_within(
            &r.comparisons[2],
            "2 : 3",
            31.0,
            6.2047365669622856,
            55.795263433037718,
            1e-6,
        );
        close_all(
            &p_values(&r),
            &[
                0.019165263527079923,
                0.63722232278324431,
                0.021640721055594891,
            ],
            1e-5,
        );
    }

    /// `TukeyHSD(aov(y ~ g))`.
    #[test]
    fn tukey_kramer_values() {
        let r = run(PostHocTest::TukeyKramer);
        check_within(
            &r.comparisons[0],
            "1 : 2",
            -32.0,
            -49.172140035499972,
            -14.827859964500043,
            1e-7,
        );
        check_within(
            &r.comparisons[1],
            "1 : 3",
            -1.0,
            -17.605245738478576,
            15.605245738478576,
            1e-7,
        );
        check_within(
            &r.comparisons[2],
            "2 : 3",
            31.0,
            15.22258906771212,
            46.777410932287893,
            1e-7,
        );
        close_all(
            &p_values(&r),
            &[
                0.00059605831888509542,
                0.98661302841403109,
                0.000359362522298734,
            ],
            1e-5,
        );
    }

    /// `DescTools::ScheffeTest(aov(y ~ g))` (DescTools 0.99.59).
    #[test]
    fn scheffe_values() {
        let r = run(PostHocTest::Scheffe);
        check(
            &r.comparisons[0],
            "1 : 2",
            -32.0,
            -49.941123031784372,
            -14.058876968215639,
        );
        check(
            &r.comparisons[1],
            "1 : 3",
            -1.0,
            -18.348842727299793,
            16.348842727299793,
        );
        check(
            &r.comparisons[2],
            "2 : 3",
            31.0,
            14.516063223685727,
            47.483936776314287,
        );
        close_all(
            &p_values(&r),
            &[
                0.00086247813116370063,
                0.98785004183013969,
                0.00052613338969684361,
            ],
            1e-9,
        );
    }

    /// `t.test(x1 / 100, x2 / 100)` for each pair, with the interval in
    /// percent.
    #[test]
    fn welch_uncorrected_values() {
        let r = run(PostHocTest::WelchUncorrected);
        check(
            &r.comparisons[0],
            "1 : 2",
            -32.0,
            -51.617685407569802,
            -12.382314592430184,
        );
        check(
            &r.comparisons[1],
            "1 : 3",
            -1.0,
            -3.4077750760636958,
            1.4077750760636962,
        );
        check(
            &r.comparisons[2],
            "2 : 3",
            31.0,
            11.387071067812595,
            50.612928932187394,
        );
        close_all(
            &p_values(&r),
            &[
                0.0084087318194191202,
                0.37641401606175107,
                0.0095108938981320907,
            ],
            1e-9,
        );
    }

    #[test]
    fn welch_uncorrected_reports_percentages_like_the_other_tests() {
        let r = run(PostHocTest::WelchUncorrected);
        let other = run(PostHocTest::TukeyKramer);
        for (a, b) in r.comparisons.iter().zip(&other.comparisons) {
            assert!((a.effect_size - b.effect_size).abs() < 1e-9);
            assert!(a.lower_ci < a.effect_size && a.effect_size < a.upper_ci);
        }
    }

    #[test]
    fn single_sample_groups_do_not_crash() {
        let single: [f64; 1] = [3.0];
        let r = PostHocTest::GamesHowell
            .run(&[&A, &single, &C], 0.95, &["a", "b", "c"])
            .unwrap();
        assert_eq!(r.note, SINGLE_SAMPLE_GROUP);
        let PostHocP::Value(p) = r.comparisons[0].p_value;
        assert!(p.is_nan());
        assert!(r.comparisons[0].lower_ci.is_nan());
        assert!(r.comparisons[1].lower_ci.is_finite());
    }

    #[test]
    fn every_group_a_single_sample_does_not_crash() {
        // No degrees of freedom within the groups: Scheffe's F quantile used
        // to panic in statrs.
        let groups: [&[f64]; 3] = [&[1.0], &[2.0], &[3.0]];
        for test in PostHocTest::ALL {
            let r = test.run(&groups, 0.95, &["a", "b", "c"]).unwrap();
            assert_eq!(r.comparisons.len(), 3, "{}", test.name());
        }
        let r = PostHocTest::Scheffe
            .run(&groups, 0.95, &["a", "b", "c"])
            .unwrap();
        let PostHocP::Value(p) = r.comparisons[0].p_value;
        assert_eq!(p, 1.0);
    }

    #[test]
    fn any_coverage_is_supported_and_names_are_checked() {
        // STAMP v2 had tables for five coverages only.
        for test in [PostHocTest::TukeyKramer, PostHocTest::GamesHowell] {
            let narrow = test.run(&[&A, &B, &C], 0.90, &["a", "b", "c"]).unwrap();
            let odd = test.run(&[&A, &B, &C], 0.97, &["a", "b", "c"]).unwrap();
            let wide = test.run(&[&A, &B, &C], 0.99, &["a", "b", "c"]).unwrap();
            let width = |r: &PostHocResult| r.comparisons[0].upper_ci - r.comparisons[0].lower_ci;
            assert!(width(&narrow) < width(&odd) && width(&odd) < width(&wide));
            assert!(test.run(&[&A, &B], 1.0, &["a", "b"]).is_err());
        }
        assert!(PostHocTest::Scheffe.run(&[&A, &B], 0.95, &["a"]).is_err());
        for test in PostHocTest::ALL {
            assert_eq!(PostHocTest::from_name(test.name()), Some(test));
        }
    }

    #[test]
    fn two_groups_agree_with_the_t_test() {
        // With two groups Tukey-Kramer is the equal-variance t-test.
        let r = PostHocTest::TukeyKramer
            .run(&[&A, &C], 0.95, &["a", "c"])
            .unwrap();
        let PostHocP::Value(p) = r.comparisons[0].p_value;
        let (na, nc) = (A.len() as f64, C.len() as f64);
        let pooled =
            ((na - 1.0) * sample_variance(&A) + (nc - 1.0) * sample_variance(&C)) / (na + nc - 2.0);
        let t = (mean(&A) - mean(&C)) / (pooled * (1.0 / na + 1.0 / nc)).sqrt();
        let expected = 2.0 * (1.0 - super::super::dist::t_cdf(t.abs(), na + nc - 2.0));
        assert!((p - expected).abs() < 1e-9, "{p} against {expected}");
    }

    #[test]
    fn a_single_sample_group_does_not_spoil_the_pooled_variance() {
        let single: [f64; 1] = [3.0];
        for test in [PostHocTest::TukeyKramer, PostHocTest::Scheffe] {
            let r = test
                .run(&[&A, &single, &C], 0.95, &["a", "b", "c"])
                .unwrap();
            for c in &r.comparisons {
                let PostHocP::Value(p) = c.p_value;
                assert!((0.0..=1.0).contains(&p), "{test:?} {}: {p}", c.label);
                assert!(c.lower_ci < c.effect_size && c.effect_size < c.upper_ci);
            }
        }
    }

    /// `scipy.stats.wilcoxon` of SciPy 1.18.1 gives 0.0625, 0.21875 and
    /// 0.046875 for the three pairs; Holm's method (R's `p.adjust`) makes
    /// them 0.140625, 0.21875 and 0.140625.
    #[test]
    fn wilcoxon_holm_values() {
        let a = [1.2, 3.4, 2.2, 5.1, 0.8, 2.9, 4.4];
        let b = [2.3, 3.9, 2.2, 6.0, 1.9, 3.3, 4.1];
        let c = [0.9, 2.8, 3.1, 4.0, 0.2, 2.9, 3.0];
        let r = PostHocTest::WilcoxonHolm
            .run(&[&a, &b, &c], 0.95, &["a", "b", "c"])
            .unwrap();
        for (comparison, expected) in r.comparisons.iter().zip([0.140625, 0.21875, 0.140625]) {
            let PostHocP::Value(p) = comparison.p_value;
            assert!((p - expected).abs() < 1e-12, "{}: {p}", comparison.label);
            assert!(comparison.lower_ci < comparison.effect_size);
            assert!(comparison.effect_size < comparison.upper_ci);
        }
        let mean = |g: &[f64]| g.iter().sum::<f64>() / g.len() as f64;
        assert!((r.comparisons[0].effect_size - (mean(&a) - mean(&b))).abs() < 1e-12);
        // Groups of different sizes cannot be matched.
        assert!(PostHocTest::WilcoxonHolm
            .run(&[&a, &b[..5]], 0.95, &["a", "b"])
            .is_err());
        assert!(PostHocTest::WilcoxonHolm.is_matched() && !PostHocTest::Dunn.is_matched());
    }

    /// `dunn.test(y, g, method = "bonferroni", altp = TRUE)` from the R
    /// package dunn.test 1.4.1 (z = -3.1015, -0.5498 and 2.7969), which
    /// agrees with a computation from the definition in SciPy 1.17.
    #[test]
    fn dunn_values() {
        let r = run(PostHocTest::Dunn);
        let expected = [0.0057772001154175268, 1.0, 0.015476513491731396];
        for (c, e) in r.comparisons.iter().zip(expected) {
            let PostHocP::Value(p) = c.p_value;
            assert!((p - e).abs() < 1e-12, "{}: expected {e}, got {p}", c.label);
            // The interval is Welch's at the adjusted level, around the
            // difference between the means.
            assert!(c.lower_ci < c.effect_size && c.effect_size < c.upper_ci);
        }
        assert_eq!(r.comparisons[0].effect_size, -32.0);
        // Wider than an unadjusted Welch interval for the same pair.
        let welch = run(PostHocTest::WelchUncorrected);
        let width = |c: &Comparison| c.upper_ci - c.lower_ci;
        assert!(width(&r.comparisons[0]) > width(&welch.comparisons[0]));

        // Ties are corrected for, and identical groups do not differ.
        let tied: [f64; 4] = [2.0, 2.0, 3.0, 3.0];
        let same = PostHocTest::Dunn
            .run(&[&tied, &tied, &tied], 0.95, &["a", "b", "c"])
            .unwrap();
        for c in &same.comparisons {
            assert_eq!(c.p_value, PostHocP::Value(1.0));
        }
        // A group of one sample can be ranked, but has no interval.
        let single: [f64; 1] = [3.0];
        let r = PostHocTest::Dunn
            .run(&[&A, &single, &B], 0.95, &["a", "b", "c"])
            .unwrap();
        let PostHocP::Value(p) = r.comparisons[0].p_value;
        assert!((0.0..=1.0).contains(&p));
        assert!(r.comparisons[0].lower_ci.is_nan());
        assert_eq!(r.note, SINGLE_SAMPLE_GROUP);
    }
}
