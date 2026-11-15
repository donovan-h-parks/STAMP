//! Hypothesis tests comparing the proportion of a feature in two samples.

use rand::RngExt;
use rand_distr::{Binomial, Distribution, Hypergeometric};

use super::dist::{chi2_sf, ln_gamma, normal_cdf};
use super::{StampRng, StatsSettings};

/// Result of a hypothesis test on one feature.
#[derive(Debug, Clone, PartialEq)]
pub struct TestOutcome {
    /// One-sided p-value; infinite for tests that are inherently two-sided.
    pub one_sided: f64,
    pub two_sided: f64,
    /// Explanation when the result is a degenerate case; otherwise empty.
    pub note: &'static str,
}

impl TestOutcome {
    fn two_sided_only(two_sided: f64, note: &'static str) -> Self {
        TestOutcome {
            one_sided: f64::INFINITY,
            two_sided,
            note,
        }
    }
}

/// Statistical tests for two samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleTest {
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

const SUSPECT: &str = "degenerate case: suspect p-value";
const ALL_ZERO: &str = "degenerate case: all values are zero";
const ZERO_PARENT: &str = "degenerate case: parent has a count of zero";

impl SampleTest {
    /// Every test, in alphabetical order of name.
    pub const ALL: [SampleTest; 10] = [
        SampleTest::Bootstrap,
        SampleTest::ChiSquare,
        SampleTest::ChiSquareYates,
        SampleTest::DiffBetweenProp,
        SampleTest::Fisher,
        SampleTest::GTest,
        SampleTest::GTestYates,
        SampleTest::GTestFisher,
        SampleTest::Hypergeometric,
        SampleTest::Permutation,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SampleTest::Bootstrap => "Bootstrap",
            SampleTest::ChiSquare => "Chi-square test",
            SampleTest::ChiSquareYates => "Chi-square test (w/ Yates')",
            SampleTest::DiffBetweenProp => "Difference between proportions",
            SampleTest::Fisher => "Fisher's exact test",
            SampleTest::GTest => "G-test",
            SampleTest::GTestYates => "G-test (w/ Yates')",
            SampleTest::GTestFisher => "G-test (w/ Yates') + Fisher's",
            SampleTest::Hypergeometric => "Hypergeometric",
            SampleTest::Permutation => "Permutation",
        }
    }

    pub fn from_name(name: &str) -> Option<SampleTest> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// True if the test uses random resampling, so that its p-values depend
    /// on the random seed.
    pub fn is_stochastic(self) -> bool {
        matches!(self, SampleTest::Bootstrap | SampleTest::Permutation)
    }

    /// True if the test has a meaningful one-sided p-value.
    pub fn has_one_sided(self) -> bool {
        !matches!(
            self,
            SampleTest::ChiSquare
                | SampleTest::ChiSquareYates
                | SampleTest::GTest
                | SampleTest::GTestYates
        )
    }

    /// Test whether a feature with `seq1` of `total1` sequences in the first
    /// sample and `seq2` of `total2` in the second differs in proportion.
    pub fn run(
        self,
        seq1: f64,
        seq2: f64,
        total1: f64,
        total2: f64,
        settings: &StatsSettings,
        rng: &mut StampRng,
    ) -> TestOutcome {
        match self {
            SampleTest::Bootstrap => {
                bootstrap(seq1, seq2, total1, total2, settings.replicates, rng)
            }
            SampleTest::ChiSquare => chi_square(seq1, seq2, total1, total2, 0.0),
            SampleTest::ChiSquareYates => chi_square(seq1, seq2, total1, total2, 0.5),
            SampleTest::DiffBetweenProp => diff_between_prop(seq1, seq2, total1, total2),
            SampleTest::Fisher => fisher(seq1, seq2, total1, total2),
            SampleTest::GTest => g_test(seq1, seq2, total1, total2),
            SampleTest::GTestYates => g_test_yates(seq1, seq2, total1, total2),
            SampleTest::GTestFisher => {
                let (a, b, c, d) = (seq1, seq2, total1 - seq1, total2 - seq2);
                if a < 20.0 || b < 20.0 || c < 20.0 || d < 20.0 {
                    fisher(seq1, seq2, total1, total2)
                } else {
                    g_test_yates(seq1, seq2, total1, total2)
                }
            }
            SampleTest::Hypergeometric => hypergeometric(seq1, seq2, total1, total2),
            SampleTest::Permutation => {
                permutation(seq1, seq2, total1, total2, settings.replicates, rng)
            }
        }
    }
}

/// Expected counts of the 2x2 table under independence.
fn expected(x1: f64, x2: f64, y1: f64, y2: f64) -> [f64; 4] {
    let n = x1 + x2 + y1 + y2;
    [
        (x1 + x2) * (x1 + y1) / n,
        (x1 + x2) * (x2 + y2) / n,
        (y1 + y2) * (x1 + y1) / n,
        (y1 + y2) * (x2 + y2) / n,
    ]
}

fn chi_square(seq1: f64, seq2: f64, total1: f64, total2: f64, continuity: f64) -> TestOutcome {
    let (x1, x2) = (seq1, seq2);
    let (y1, y2) = (total1 - x1, total2 - x2);
    if (x1 == 0.0 && x2 == 0.0) || (x1 == total1 || x2 == total2) {
        return TestOutcome::two_sided_only(1.0, SUSPECT);
    }
    let e = expected(x1, x2, y1, y2);
    let statistic: f64 = [x1, x2, y1, y2]
        .iter()
        .zip(&e)
        .map(|(observed, expected)| ((observed - expected).abs() - continuity).powi(2) / expected)
        .sum();
    // The upper tail directly, not one minus the CDF, so that p-values far
    // below 1e-16 keep their value.
    TestOutcome::two_sided_only(chi2_sf(statistic, 1.0), "")
}

fn diff_between_prop(seq1: f64, seq2: f64, total1: f64, total2: f64) -> TestOutcome {
    let mut note = "";
    let d = if (seq1 == 0.0 && seq2 == 0.0) || (seq1 == total1 && seq2 == total2) {
        note = SUSPECT;
        0.0
    } else if total1 == 0.0 || total2 == 0.0 {
        note = ZERO_PARENT;
        0.0
    } else {
        let diff = seq1 / total1 - seq2 / total2;
        let p = (seq1 + seq2) / (total1 + total2);
        let q = 1.0 - p;
        diff / (p * q * (1.0 / total1 + 1.0 / total2)).sqrt()
    };

    // The upper tail of |d|, computed directly so that it stays accurate
    // far into the tail.
    let one_sided = normal_cdf(-d.abs());
    TestOutcome {
        one_sided,
        two_sided: 2.0 * one_sided,
        note,
    }
}

fn ln_choose(n: f64, k: f64) -> f64 {
    ln_gamma(n + 1.0) - (ln_gamma(n - k + 1.0) + ln_gamma(k + 1.0))
}

/// Probability of the 2x2 table [[a, b], [c, d]] with fixed margins.
fn table_probability(a: f64, b: f64, c: f64, d: f64) -> f64 {
    if a < 0.0 || b < 0.0 || c < 0.0 || d < 0.0 {
        return 0.0;
    }
    (ln_choose(a + b, a) + ln_choose(c + d, c) - ln_choose(a + b + c + d, a + c)).exp()
}

fn fisher(seq1: f64, seq2: f64, total1: f64, total2: f64) -> TestOutcome {
    let a = (seq1 + 0.5).floor();
    let b = (seq2 + 0.5).floor();
    let c = (total1 - seq1 + 0.5).floor();
    let d = (total2 - seq2 + 0.5).floor();

    let (r1, c1, c2) = (a + b, a + c, b + d);
    let cutoff = table_probability(a, b, c, d);

    // "Minimum likelihood" two-sided p-value: the total probability of the
    // tables that are no more likely than the observed one.
    let (mut two_sided, mut right, mut left) = (0.0, 0.0, 0.0);
    let most = r1.min(c1) as i64;
    for i in 0..=most {
        let ai = i as f64;
        let bi = r1 - ai;
        let ci = c1 - ai;
        let di = c2 - bi;
        if bi >= 0.0 && ci >= 0.0 && di >= 0.0 {
            let p = table_probability(ai, bi, ci, di);
            // Tables as likely as the observed one, allowing for rounding
            // in their computed probabilities: the relative tolerance R's
            // fisher.test and SciPy use. Without it, a table exactly as
            // likely could be missed.
            if p <= cutoff * (1.0 + 1e-7) {
                two_sided += p;
            }
            // Compared with the rounded count, the table actually tested:
            // with the unrounded one a count that rounds up left the
            // observed table out of the left tail.
            if ai >= a {
                right += p;
            }
            if ai <= a {
                left += p;
            }
        }
    }
    // Sums of rounded probabilities can pass one slightly; R clamps them.
    TestOutcome {
        one_sided: left.min(right).min(1.0),
        two_sided: two_sided.min(1.0),
        note: "",
    }
}

fn g_test(seq1: f64, seq2: f64, total1: f64, total2: f64) -> TestOutcome {
    let (x1, x2) = (seq1, seq2);
    let (y1, y2) = (total1 - x1, total2 - x2);
    if x1 + x2 + y1 + y2 <= 0.0 {
        return TestOutcome::two_sided_only(1.0, ALL_ZERO);
    }
    let e = expected(x1, x2, y1, y2);
    let g: f64 = [x1, x2, y1, y2]
        .iter()
        .zip(&e)
        .filter(|(observed, _)| **observed != 0.0)
        .map(|(observed, expected)| observed * (observed / expected).ln())
        .sum();
    TestOutcome::two_sided_only(chi2_sf(2.0 * g, 1.0), "")
}

fn g_test_yates(seq1: f64, seq2: f64, total1: f64, total2: f64) -> TestOutcome {
    let (mut x1, mut x2) = (seq1, seq2);
    let (mut y1, mut y2) = (total1 - x1, total2 - x2);
    // Move each cell half a count towards its expected value.
    let shift = if x1 * y2 - x2 * y1 > 0.0 { -0.5 } else { 0.5 };
    x1 += shift;
    y2 += shift;
    x2 -= shift;
    y1 -= shift;

    if x1 + x2 + y1 + y2 <= 0.0 {
        return TestOutcome::two_sided_only(1.0, ALL_ZERO);
    }
    let e = expected(x1, x2, y1, y2);
    let g: f64 = [x1, x2, y1, y2]
        .iter()
        .zip(&e)
        .filter(|(observed, expected)| **observed > 0.0 && **expected > 0.0)
        .map(|(observed, expected)| observed * (observed / expected).ln())
        .sum();
    TestOutcome::two_sided_only(chi2_sf(2.0 * g, 1.0), "")
}

/// The hypergeometric test: the smaller tail of the hypergeometric
/// distribution of the first sample's count given the margins (the
/// one-sided Fisher's exact test), doubled for the two-sided p-value and
/// capped at one.
fn hypergeometric(seq1: f64, seq2: f64, total1: f64, total2: f64) -> TestOutcome {
    let (a, b) = (seq1, seq2);
    let (c, d) = (total1 - seq1, total2 - seq2);

    // P(A <= a) and P(A >= a), each summed over its own tables so that a
    // tiny tail is not lost by subtracting from one.
    let (r1, c1) = (a + b, a + c);
    let probability = |i: f64| table_probability(i, r1 - i, c1 - i, d - a + i);
    let (mut left, mut right) = (0.0, 0.0);
    for i in 0..=(r1.min(c1) as i64) {
        let i = i as f64;
        let p = probability(i);
        if i <= a {
            left += p;
        }
        if i >= a {
            right += p;
        }
    }
    let one_sided = left.min(right).min(1.0);
    TestOutcome {
        one_sided,
        two_sided: (2.0 * one_sided).min(1.0),
        note: "",
    }
}

/// Turn the simulated differences in proportion under the null hypothesis
/// into p-values.
fn resampled_p_values(differences: &[f64], observed: f64) -> (f64, f64) {
    let (mut left, mut right, mut two_sided) = (0usize, 0usize, 0usize);
    for &value in differences {
        if value <= observed {
            left += 1;
        }
        if value >= observed {
            right += 1;
        }
        if value.abs() >= observed.abs() {
            two_sided += 1;
        }
    }
    let n = differences.len() as f64;
    (left.min(right) as f64 / n, two_sided as f64 / n)
}

fn bootstrap(
    seq1: f64,
    seq2: f64,
    total1: f64,
    total2: f64,
    replicates: usize,
    rng: &mut StampRng,
) -> TestOutcome {
    if total1 == 0.0 || total2 == 0.0 || replicates == 0 {
        return TestOutcome {
            one_sided: 1.0,
            two_sided: 1.0,
            note: ZERO_PARENT,
        };
    }
    let pooled = ((seq1 + seq2) / (total1 + total2)).clamp(0.0, 1.0);
    let draw1 = Binomial::new(total1 as u64, pooled).expect("valid binomial parameters");
    let draw2 = Binomial::new(total2 as u64, pooled).expect("valid binomial parameters");
    let differences: Vec<f64> = (0..replicates)
        .map(|_| draw1.sample(rng) as f64 / total1 - draw2.sample(rng) as f64 / total2)
        .collect();

    let (one_sided, two_sided) = resampled_p_values(&differences, seq1 / total1 - seq2 / total2);
    TestOutcome {
        one_sided,
        two_sided,
        note: "",
    }
}

fn permutation(
    seq1: f64,
    seq2: f64,
    total1: f64,
    total2: f64,
    replicates: usize,
    rng: &mut StampRng,
) -> TestOutcome {
    if total1 == 0.0 || total2 == 0.0 || replicates == 0 {
        return TestOutcome {
            one_sided: 1.0,
            two_sided: 1.0,
            note: ZERO_PARENT,
        };
    }
    // Randomly reassign the pooled sequences to the two samples.
    let positive = (seq1 + seq2) as u64;
    let everything = (total1 + total2) as u64;
    let draw = Hypergeometric::new(everything, positive, total1 as u64)
        .expect("valid hypergeometric parameters");
    let differences: Vec<f64> = (0..replicates)
        .map(|_| {
            let c1 = draw.sample(rng) as f64;
            c1 / total1 - (positive as f64 - c1) / total2
        })
        .collect();

    let (one_sided, two_sided) = resampled_p_values(&differences, seq1 / total1 - seq2 / total2);
    TestOutcome {
        one_sided,
        two_sided,
        note: "",
    }
}

/// Uniform random index in `0..n`.
pub(crate) fn random_index(rng: &mut StampRng, n: usize) -> usize {
    rng.random_range(0..n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rng_from_seed;

    // Tables as (feature in sample 1, feature in sample 2, total of sample
    // 1, total of sample 2), the R table `matrix(c(s1, t1 - s1, s2, t2 - s2),
    // 2)`. Reference values are from R 4.4.3: `chisq.test`, `fisher.test`,
    // `prop.test(correct = FALSE)` and `phyper` (stats), and `GTest` from
    // DescTools 0.99.59.
    const TABLE1: [f64; 4] = [10.0, 8.0, 30.0, 40.0];
    const TABLE2: [f64; 4] = [4000.0, 5000.0, 500000.0, 1000000.0];
    /// Its two most extreme tables are exactly as likely as each other.
    const SYMMETRIC: [f64; 4] = [3.0, 1.0, 4.0, 4.0];
    /// Nearly equal proportions: both tails exceed one half.
    const ALIKE: [f64; 4] = [10.0, 13.0, 30.0, 40.0];

    fn run(test: SampleTest, t: [f64; 4]) -> TestOutcome {
        test.run(
            t[0],
            t[1],
            t[2],
            t[3],
            &StatsSettings::default(),
            &mut rng_from_seed(1),
        )
    }

    /// Equal to a relative precision of 1e-9 (or 1e-12 absolute).
    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-9 * expected.abs() + 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn chi_square_tests() {
        // chisq.test(m, correct = FALSE)$p.value and correct = TRUE.
        let r = run(SampleTest::ChiSquare, TABLE1);
        assert_eq!(r.one_sided, f64::INFINITY);
        close(r.two_sided, 0.20655040125167884);
        close(
            run(SampleTest::ChiSquare, TABLE2).two_sided,
            2.0955338526444138e-111,
        );
        close(
            run(SampleTest::ChiSquare, SYMMETRIC).two_sided,
            0.15729920705028447,
        );
        close(
            run(SampleTest::ChiSquareYates, TABLE1).two_sided,
            0.32373919646607668,
        );
        close(
            run(SampleTest::ChiSquareYates, TABLE2).two_sided,
            2.6959554684832328e-111,
        );
        close(
            run(SampleTest::ChiSquareYates, SYMMETRIC).two_sided,
            0.47950012218695348,
        );
    }

    #[test]
    fn difference_between_proportions() {
        // prop.test(c(s1, s2), c(t1, t2), correct = FALSE): the two-sided
        // p-value, and the alternative in the observed direction.
        let r = run(SampleTest::DiffBetweenProp, TABLE1);
        close(r.one_sided, 0.10327520062583943);
        close(r.two_sided, 0.20655040125167881);
        let r = run(SampleTest::DiffBetweenProp, TABLE2);
        close(r.one_sided, 1.0477669263221812e-111);
        close(r.two_sided, 2.0955338526444138e-111);
        let r = run(SampleTest::DiffBetweenProp, ALIKE);
        close(r.one_sided, 0.47072039836691043);
        close(r.two_sided, 0.94144079673382097);
    }

    #[test]
    fn fishers_exact_test() {
        // fisher.test(m)$p.value; the one-sided value is the smaller of
        // alternative = "less" and "greater".
        let r = run(SampleTest::Fisher, TABLE1);
        close(r.one_sided, 0.16187126209690728);
        close(r.two_sided, 0.27155433277891844);
        let r = run(SampleTest::Fisher, TABLE2);
        close(r.one_sided, 7.0436418998473035e-107);
        close(r.two_sided, 1.117976846912833e-106);
        // Tables as likely as the observed one count, despite rounding.
        let r = run(SampleTest::Fisher, SYMMETRIC);
        close(r.one_sided, 0.24285714285714288);
        close(r.two_sided, 0.48571428571428582);
        let r = run(SampleTest::Fisher, ALIKE);
        close(r.one_sided, 0.57106820504812972);
        close(r.two_sided, 1.0);
        let r = run(SampleTest::Fisher, [0.0, 0.0, 920852.999591, 953828.994346]);
        close(r.one_sided, 1.0);
        close(r.two_sided, 1.0);
        // A fractional count is rounded, and both tails are those of the
        // rounded table (STAMP v2 left it out of the left tail).
        let rounded = run(SampleTest::Fisher, [3.0, 10.0, 30.0, 40.0]);
        let fractional = run(SampleTest::Fisher, [2.6, 10.0, 30.0, 40.0]);
        close(fractional.one_sided, rounded.one_sided);
        close(fractional.two_sided, rounded.two_sided);
    }

    #[test]
    fn g_tests() {
        // DescTools::GTest(m, correct = "none") and correct = "yates".
        let r = run(SampleTest::GTest, TABLE1);
        assert_eq!(r.one_sided, f64::INFINITY);
        close(r.two_sided, 0.20824866445758117);
        close(
            run(SampleTest::GTest, SYMMETRIC).two_sided,
            0.14797595938502583,
        );
        close(run(SampleTest::GTest, ALIKE).two_sided, 0.94145820258000512);
        close(
            run(SampleTest::GTestYates, TABLE1).two_sided,
            0.32550224000959427,
        );
        close(
            run(SampleTest::GTestYates, SYMMETRIC).two_sided,
            0.47716178085961258,
        );
        close(
            run(SampleTest::GTestYates, ALIKE).two_sided,
            0.85416910636196564,
        );
        // R reports 0 for the large table, below the smallest double; the
        // statistic is large enough that the p-value is far below 1e-100.
        assert!(run(SampleTest::GTest, TABLE2).two_sided < 1e-100);
        assert!(run(SampleTest::GTestYates, TABLE2).two_sided < 1e-100);
        // Small counts fall back to Fisher's exact test.
        close(
            run(SampleTest::GTestFisher, TABLE1).two_sided,
            0.27155433277891844,
        );
        assert_eq!(
            run(SampleTest::GTestFisher, TABLE2),
            run(SampleTest::GTestYates, TABLE2)
        );
    }

    #[test]
    fn hypergeometric_test() {
        // The smaller of phyper(s1, t1, t2, s1 + s2) and
        // phyper(s1 - 1, t1, t2, s1 + s2, lower.tail = FALSE), doubled for
        // the two-sided p-value and capped at one.
        let r = run(SampleTest::Hypergeometric, TABLE1);
        close(r.one_sided, 0.16187126209690728);
        close(r.two_sided, 2.0 * 0.16187126209690728);
        let r = run(SampleTest::Hypergeometric, TABLE2);
        close(r.one_sided, 7.0436418998473035e-107);
        close(r.two_sided, 2.0 * 7.0436418998473035e-107);
        let r = run(SampleTest::Hypergeometric, ALIKE);
        close(r.one_sided, 0.57106820504812972);
        assert_eq!(r.two_sided, 1.0);
        // The same whichever sample is first.
        let swapped = run(SampleTest::Hypergeometric, [8.0, 10.0, 40.0, 30.0]);
        close(swapped.one_sided, 0.16187126209690728);
    }

    #[test]
    fn degenerate_tables() {
        assert_eq!(
            run(SampleTest::ChiSquare, [0.0, 0.0, 10.0, 10.0]).note,
            SUSPECT
        );
        assert_eq!(run(SampleTest::GTest, [0.0, 0.0, 0.0, 0.0]).note, ALL_ZERO);
        let r = run(SampleTest::DiffBetweenProp, [0.0, 5.0, 0.0, 10.0]);
        assert_eq!((r.two_sided, r.note), (1.0, ZERO_PARENT));
        let r = run(SampleTest::Bootstrap, [0.0, 5.0, 0.0, 10.0]);
        assert_eq!((r.two_sided, r.note), (1.0, ZERO_PARENT));
    }

    /// The exact p-values the resampling tests estimate, computed in R 4.4.3
    /// for TABLE1 by enumerating every outcome:
    /// - permutation: `dhyper(x, 30, 40, 18)` over the counts x of the
    ///   first sample, keeping those whose difference in proportion is at
    ///   least as large as observed;
    /// - bootstrap: `outer(dbinom(0:30, 30, p0), dbinom(0:40, 40, p0))` with
    ///   the pooled proportion p0 = 18 / 70, likewise.
    ///
    /// With 20,000 replicates the estimates have a standard error below
    /// 0.0035; they must lie within four of them.
    #[test]
    fn resampling_tests_estimate_the_exact_p_values() {
        let settings = StatsSettings {
            replicates: 20_000,
            ..StatsSettings::default()
        };
        let cases: [(SampleTest, f64, f64); 2] = [
            (
                SampleTest::Permutation,
                0.16187126209690744,
                0.27155433277891827,
            ),
            (
                SampleTest::Bootstrap,
                0.11131369037238556,
                0.2214118181470407,
            ),
        ];
        for (test, one_sided, two_sided) in cases {
            assert!(test.is_stochastic());
            let [s1, s2, t1, t2] = TABLE1;
            let r = test.run(s1, s2, t1, t2, &settings, &mut rng_from_seed(1));
            let again = test.run(s1, s2, t1, t2, &settings, &mut rng_from_seed(1));
            assert_eq!(r, again, "same seed, same result");
            for (estimate, exact) in [(r.one_sided, one_sided), (r.two_sided, two_sided)] {
                let se = (exact * (1.0 - exact) / 20_000.0f64).sqrt();
                assert!(
                    (estimate - exact).abs() < 4.0 * se,
                    "{test:?}: {estimate} against the exact {exact}"
                );
            }
            assert!(run(test, TABLE2).two_sided < 0.002);
        }
    }

    #[test]
    fn names_round_trip() {
        for test in SampleTest::ALL {
            assert_eq!(SampleTest::from_name(test.name()), Some(test));
        }
        assert_eq!(SampleTest::from_name("nope"), None);
    }
}
