//! Post-hoc tests: pairwise comparisons of groups after a multiple-group test.

use std::sync::OnceLock;

use super::dist::{f_cdf, f_ppf};
use super::two_group::welch;
use super::{mean, sample_variance};
use crate::error::{Result, StampError};

/// Post-hoc tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostHocTest {
    GamesHowell,
    Scheffe,
    TukeyKramer,
    WelchUncorrected,
}

/// p-value of a pairwise comparison. The tests based on the studentized range
/// report a bound from tables rather than an exact value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostHocP {
    Value(f64),
    /// One of `< 0.001`, `< 0.01`, `< 0.02`, `< 0.05`, `< 0.1`, `>= 0.1`.
    Bound(&'static str),
}

impl std::fmt::Display for PostHocP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostHocP::Value(v) => write!(f, "{v:?}"),
            PostHocP::Bound(b) => f.write_str(b),
        }
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
    pub comparisons: Vec<Comparison>,
    pub note: &'static str,
}

const ZERO_WITHIN_VARIANCE: &str = "degenerate case: within group variance is zero; set to 1e-6.";
const ZERO_GROUP_VARIANCE: &str =
    "degenerate case: two groups with zero variance; variance set to 1e-6.";
const SINGLE_SAMPLE_GROUP: &str =
    "degenerate case: a group with fewer than 2 samples cannot be compared";

impl PostHocTest {
    pub const ALL: [PostHocTest; 4] = [
        PostHocTest::GamesHowell,
        PostHocTest::Scheffe,
        PostHocTest::TukeyKramer,
        PostHocTest::WelchUncorrected,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PostHocTest::GamesHowell => "Games-Howell",
            PostHocTest::Scheffe => "Scheffè",
            PostHocTest::TukeyKramer => "Tukey-Kramer",
            PostHocTest::WelchUncorrected => "Welch's (uncorrected)",
        }
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
            PostHocTest::GamesHowell => games_howell(groups, coverage)?,
            PostHocTest::Scheffe => scheffe(groups, coverage),
            PostHocTest::TukeyKramer => tukey_kramer(groups, coverage)?,
            PostHocTest::WelchUncorrected => welch_uncorrected(groups, coverage),
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
        Ok(PostHocResult { comparisons, note })
    }
}

/// (p-value, effect size, lower CI, upper CI) for each pair, in order.
type PairStats = Vec<(PostHocP, f64, f64, f64)>;

/// Critical values of the studentized range statistic.
struct QTable {
    /// One table per significance level; rows are degrees of freedom from 2
    /// to 200, columns are numbers of groups from 2 to 30.
    tables: Vec<(f64, Vec<Vec<f64>>)>,
}

fn q_table() -> &'static QTable {
    static TABLE: OnceLock<QTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let parse = |text: &str| -> Vec<Vec<f64>> {
            text.lines()
                .skip(1)
                .filter(|l| !l.trim().is_empty())
                .map(|l| {
                    l.split('\t')
                        .skip(1)
                        .map(|v| v.trim().parse().expect("number"))
                        .collect()
                })
                .collect()
        };
        QTable {
            tables: vec![
                (0.001, parse(include_str!("../../data/tukeyQ_001.txt"))),
                (0.01, parse(include_str!("../../data/tukeyQ_01.txt"))),
                (0.02, parse(include_str!("../../data/tukeyQ_02.txt"))),
                (0.05, parse(include_str!("../../data/tukeyQ_05.txt"))),
                (0.1, parse(include_str!("../../data/tukeyQ_10.txt"))),
            ],
        }
    })
}

impl QTable {
    fn table(&self, alpha: f64) -> Result<&Vec<Vec<f64>>> {
        self.tables
            .iter()
            .find(|(level, _)| (level - alpha).abs() < 1e-9)
            .map(|(_, table)| table)
            .ok_or_else(|| {
                StampError::new(
                    "This post-hoc test supports confidence levels of 90%, 95%, 98%, 99% and 99.9% only.",
                )
            })
    }

    /// Critical value for `k` groups and `df` degrees of freedom.
    fn cv(&self, alpha: f64, k: usize, df: i64) -> Result<f64> {
        let table = self.table(alpha)?;
        if k < 2 || df < 2 {
            return Ok(1000.0);
        }
        // Beyond the table, the last entries approximate infinity well.
        let k = k.min(30);
        let df = df.min(200) as usize;
        Ok(table[df - 2][k - 2])
    }

    /// Critical value for fractional degrees of freedom, by linear
    /// interpolation.
    fn cv_interpolated(&self, alpha: f64, k: usize, df: f64) -> Result<f64> {
        let (lower_df, upper_df) = (df.floor(), df.ceil());
        let lower = self.cv(alpha, k, lower_df as i64)?;
        let upper = self.cv(alpha, k, upper_df as i64)?;
        Ok(lower + (df - lower_df) * (upper - lower))
    }
}

const BOUNDS: [(f64, &str); 5] = [
    (0.001, "< 0.001"),
    (0.01, "< 0.01"),
    (0.02, "< 0.02"),
    (0.05, "< 0.05"),
    (0.1, "< 0.1"),
];
const NOT_SIGNIFICANT: &str = ">= 0.1";

fn tukey_kramer(groups: &[&[f64]], coverage: f64) -> Result<(PairStats, &'static str)> {
    let table = q_table();
    let k = groups.len();
    let n: usize = groups.iter().map(|g| g.len()).sum();
    let df = n as i64 - k as i64;

    let q_cv = table.cv(1.0 - coverage, k, df)?;
    let mut critical = [0.0; 5];
    for (slot, (alpha, _)) in critical.iter_mut().zip(BOUNDS) {
        *slot = table.cv(alpha, k, df)?;
    }

    let within_var = groups
        .iter()
        .map(|g| (g.len() as f64 - 1.0) * sample_variance(g))
        .sum::<f64>()
        / df as f64;
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
            let bound = BOUNDS
                .iter()
                .zip(critical)
                .find(|(_, cv)| q > *cv)
                .map_or(NOT_SIGNIFICANT, |((_, label), _)| *label);
            let half_width = q_cv * within_sd * scale;
            stats.push((PostHocP::Bound(bound), es, es - half_width, es + half_width));
        }
    }
    Ok((stats, note))
}

fn games_howell(groups: &[&[f64]], coverage: f64) -> Result<(PairStats, &'static str)> {
    let table = q_table();
    table.table(1.0 - coverage)?;
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
                stats.push((PostHocP::Bound(NOT_SIGNIFICANT), es, f64::NAN, f64::NAN));
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
            let mut bound = NOT_SIGNIFICANT;
            for (alpha, label) in BOUNDS {
                if q > table.cv_interpolated(alpha, k, df)? {
                    bound = label;
                    break;
                }
            }
            let half_width = table.cv_interpolated(1.0 - coverage, k, df)? * scale;
            stats.push((PostHocP::Bound(bound), es, es - half_width, es + half_width));
        }
    }
    Ok((stats, note))
}

fn scheffe(groups: &[&[f64]], coverage: f64) -> (PairStats, &'static str) {
    let k = groups.len();
    let n: usize = groups.iter().map(|g| g.len()).sum();
    let df_between = k as f64 - 1.0;
    let df_within = n as f64 - k as f64;
    let critical = df_between * f_ppf(coverage, df_between, df_within);

    let mut within_var = groups
        .iter()
        .map(|g| (g.len() as f64 - 1.0) * sample_variance(g))
        .sum::<f64>()
        / df_within;
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
            let p = 1.0 - f_cdf(f / df_between, df_between, df_within);
            let half_width = (critical * inverse_sizes).sqrt() * within_sd;
            stats.push((PostHocP::Value(p), es, es - half_width, es + half_width));
        }
    }
    (stats, note)
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
        assert_eq!(c.label, label);
        for (actual, expected) in [
            (c.effect_size, effect),
            (c.lower_ci, lower),
            (c.upper_ci, upper),
        ] {
            assert!(
                (actual - expected).abs() < 5e-8,
                "{label}: expected {expected}, got {actual}"
            );
        }
    }

    // Expected values from the original STAMP unit tests (ground truth from
    // SPSS and MATLAB; the Games-Howell limits are STAMP's own, which differ
    // slightly from SPSS because critical values are interpolated).
    #[test]
    fn games_howell_values() {
        let r = run(PostHocTest::GamesHowell);
        check(
            &r.comparisons[0],
            "1 : 2",
            -32.0,
            -56.836534205367272,
            -7.163465794632728,
        );
        check(
            &r.comparisons[1],
            "1 : 3",
            -1.0,
            -3.9627938823820417,
            1.9627938823820417,
        );
        check(
            &r.comparisons[2],
            "2 : 3",
            31.0,
            6.1693311597445302,
            55.83066884025547,
        );
        let bounds: Vec<PostHocP> = r.comparisons.iter().map(|c| c.p_value).collect();
        assert_eq!(
            bounds,
            [
                PostHocP::Bound("< 0.02"),
                PostHocP::Bound(">= 0.1"),
                PostHocP::Bound("< 0.05")
            ]
        );
    }

    #[test]
    fn tukey_kramer_values() {
        let r = run(PostHocTest::TukeyKramer);
        check(
            &r.comparisons[0],
            "1 : 2",
            -32.0,
            -49.172140035619407,
            -14.827859964380597,
        );
        check(
            &r.comparisons[1],
            "1 : 3",
            -1.0,
            -17.605245738594071,
            15.605245738594071,
        );
        check(
            &r.comparisons[2],
            "2 : 3",
            31.0,
            15.222589067602378,
            46.777410932397622,
        );
        assert_eq!(r.comparisons[0].p_value, PostHocP::Bound("< 0.001"));
        assert_eq!(r.comparisons[1].p_value, PostHocP::Bound(">= 0.1"));
        assert_eq!(r.comparisons[2].p_value, PostHocP::Bound("< 0.001"));
    }

    #[test]
    fn scheffe_values() {
        let r = run(PostHocTest::Scheffe);
        check(
            &r.comparisons[0],
            "1 : 2",
            -32.0,
            -49.941123031784372,
            -14.058876968215628,
        );
        check(
            &r.comparisons[1],
            "1 : 3",
            -1.0,
            -18.348842727299797,
            16.348842727299797,
        );
        check(
            &r.comparisons[2],
            "2 : 3",
            31.0,
            14.51606322368572,
            47.48393677631428,
        );
        let expected = [
            8.624781311637033E-4,
            0.9878500418301395,
            5.261333896968458E-4,
        ];
        for (c, e) in r.comparisons.iter().zip(expected) {
            let PostHocP::Value(p) = c.p_value else {
                panic!("expected a value")
            };
            assert!((p - e).abs() < 5e-8, "expected {e}, got {p}");
        }
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
        assert_eq!(r.comparisons[0].p_value, PostHocP::Bound(">= 0.1"));
        assert!(r.comparisons[0].lower_ci.is_nan());
        assert!(r.comparisons[1].lower_ci.is_finite());
    }

    #[test]
    fn unsupported_coverage_and_names() {
        assert!(PostHocTest::TukeyKramer
            .run(&[&A, &B], 0.97, &["a", "b"])
            .is_err());
        assert!(PostHocTest::GamesHowell
            .run(&[&A, &B], 0.97, &["a", "b"])
            .is_err());
        assert!(PostHocTest::Scheffe
            .run(&[&A, &B], 0.97, &["a", "b"])
            .is_ok());
        assert!(PostHocTest::Scheffe.run(&[&A, &B], 0.95, &["a"]).is_err());
        for test in PostHocTest::ALL {
            assert_eq!(PostHocTest::from_name(test.name()), Some(test));
        }
    }

    #[test]
    fn q_table_lookup() {
        let table = q_table();
        assert_eq!(table.cv(0.05, 2, 2).unwrap(), 6.079637419);
        assert_eq!(table.cv(0.05, 3, 3).unwrap(), 5.909663249);
        assert_eq!(table.cv(0.05, 2, 5000).unwrap(), 2.788682383);
        assert_eq!(table.cv(0.05, 1, 10).unwrap(), 1000.0);
        let mid = table.cv_interpolated(0.05, 2, 2.5).unwrap();
        assert!((mid - (6.079637419 + 4.500658726) / 2.0).abs() < 1e-12);
    }
}
