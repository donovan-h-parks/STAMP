//! Analysis of composition of microbiomes (ANCOM; Mandal et al. 2015): which
//! features differ in abundance between groups, judged by the ratios of
//! each feature to every other feature rather than by proportions.
//!
//! Proportions sum to one, so a feature that rises makes every other feature
//! seem to fall. The ratio of two features does not depend on the rest of
//! the sample, so ANCOM tests, for every pair of features, whether the
//! logarithm of their ratio differs between the groups. A feature whose
//! abundance really changes differs from most other features; its count of
//! such differences, W, is high.

use super::correction::Correction;
use super::dist::f_sf;
use super::diversity::centred_log_ratio;
use super::effect_size::eta_squared;
use super::multi_group::MultiGroupTest;
use super::results::{Control, Pace, Step};
use super::rng_from_seed;

/// Lower bound, as a share of the features, on the largest W for any
/// feature to be called different (scikit-bio's `theta`).
const THETA: f64 = 0.1;
/// Tolerance used in choosing the cut-off (scikit-bio's `tau`).
const TAU: f64 = 0.02;

/// The outcome of ANCOM for one feature.
#[derive(Debug, Clone, PartialEq)]
pub struct AncomRow {
    /// Number of other features whose log ratio with this one differs
    /// significantly between the groups.
    pub w: usize,
    /// Whether the feature is called different: its W reaches the cut-off.
    pub reject: bool,
    /// How much the feature differs, on the centred log-ratio scale: the
    /// difference between the means of the first and second group for two
    /// groups, and eta-squared for more.
    pub clr_effect: f64,
}

/// The outcome of ANCOM for every feature.
#[derive(Debug, Clone, PartialEq)]
pub struct AncomResult {
    /// One row per feature, in the order given.
    pub rows: Vec<AncomRow>,
    /// The W a feature must reach to be called different, if any feature
    /// can be.
    pub cutoff: Option<f64>,
    /// The test applied to each log ratio.
    pub test: MultiGroupTest,
    pub alpha: f64,
}

impl AncomResult {
    /// Number of features called different.
    pub fn rejected(&self) -> usize {
        self.rows.iter().filter(|r| r.reject).count()
    }

    /// Name of the effect on the centred log-ratio scale.
    pub fn effect_name(groups: usize) -> &'static str {
        if groups == 2 {
            "Difference in mean CLR"
        } else {
            "Eta-squared of CLR"
        }
    }
}

/// Run ANCOM. `samples[s][f]` is the abundance (count or proportion) of
/// feature `f` in sample `s`, and `groups[s]` the group of sample `s`,
/// numbered from zero. Zeros are replaced as for the centred log-ratio
/// transform ([`centred_log_ratio`]). Each log ratio is compared between
/// the groups with `test`, the p-values of each feature are adjusted with
/// the Holm-Bonferroni method, and W counts those below `alpha`; the
/// cut-off on W is chosen as scikit-bio chooses it.
///
/// `None` if there are fewer than two groups with samples, fewer than two
/// features, or no group has more than one sample.
pub fn ancom(
    samples: &[Vec<f64>],
    groups: &[usize],
    test: MultiGroupTest,
    alpha: f64,
) -> Option<AncomResult> {
    let mut run = AncomRun::new(samples, groups, test, alpha)?;
    match run.advance(&mut |_, _| Control::Continue) {
        Step::Done(result) => Some(result),
        Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
    }
}

/// [`ancom`] carried out a feature at a time (each feature's log ratios with
/// the others), so that whatever runs it can show its progress, attend to
/// other things in between, or give up. The outcome does not depend on
/// where it pauses.
#[derive(Debug, Clone)]
pub struct AncomRun {
    features: usize,
    /// Centred log ratios, `clr[sample][feature]`.
    clr: Vec<Vec<f64>>,
    /// The samples of each group that has any.
    members: Vec<Vec<usize>>,
    test: MultiGroupTest,
    alpha: f64,
    method: Method,
    /// The next feature to work on.
    next: usize,
}

#[derive(Debug, Clone)]
enum Method {
    /// ANOVA on groups of two or more samples, which has a shortcut; W of
    /// the features done so far.
    Anova(AnovaSums, Vec<usize>),
    /// Any other test goes through the p-value of every pair: `p[i][j]`,
    /// filled for `j > i` (and mirrored) row by row.
    Test(Vec<Vec<f64>>),
}

impl AncomRun {
    /// Arguments as for [`ancom`]; `None` in the same cases.
    pub fn new(
        samples: &[Vec<f64>],
        groups: &[usize],
        test: MultiGroupTest,
        alpha: f64,
    ) -> Option<AncomRun> {
        let features = samples.first().map_or(0, Vec::len);
        let group_count = groups.iter().copied().max().map_or(0, |g| g + 1);
        let used = (0..group_count).filter(|g| groups.contains(g)).count();
        if features < 2 || used < 2 || used == samples.len() || samples.len() != groups.len() {
            return None;
        }
        // The log ratio of two features is the difference of their centred
        // log ratios: the sample's mean logarithm cancels.
        let clr: Vec<Vec<f64>> = samples.iter().map(|s| centred_log_ratio(s)).collect();
        let members: Vec<Vec<usize>> = (0..group_count)
            .map(|g| (0..samples.len()).filter(|&s| groups[s] == g).collect())
            .filter(|m: &Vec<usize>| !m.is_empty())
            .collect();
        let method = if test == MultiGroupTest::Anova && members.iter().all(|m| m.len() >= 2) {
            Method::Anova(
                AnovaSums::new(&clr, &members, features, alpha),
                Vec::with_capacity(features),
            )
        } else {
            Method::Test(vec![vec![0.0; features]; features])
        };
        Some(AncomRun {
            features,
            clr,
            members,
            test,
            alpha,
            method,
            next: 0,
        })
    }

    /// The number of features, each a piece of work.
    pub fn features(&self) -> usize {
        self.features
    }

    /// Work on further features; `pace` is asked before each, with the
    /// number done and the number of features.
    #[allow(clippy::needless_range_loop)] // p[i][j] and p[j][i] are set together
    pub fn advance(&mut self, pace: Pace<'_>) -> Step<AncomResult> {
        let features = self.features;
        let mut buffers: Vec<Vec<f64>> = vec![Vec::new(); self.members.len()];
        let mut f_values = Vec::new();
        while self.next < features {
            match pace(self.next, features) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            let i = self.next;
            match &mut self.method {
                Method::Anova(sums, w) => w.push(sums.w(i, &mut f_values)),
                Method::Test(p) => {
                    for j in i + 1..features {
                        for (buffer, member) in buffers.iter_mut().zip(&self.members) {
                            buffer.clear();
                            buffer.extend(member.iter().map(|&s| self.clr[s][i] - self.clr[s][j]));
                        }
                        let slices: Vec<&[f64]> = buffers.iter().map(Vec::as_slice).collect();
                        let (value, _) = self.test.run(&slices);
                        p[i][j] = value;
                        p[j][i] = value;
                    }
                }
            }
            self.next += 1;
        }
        let w = match &mut self.method {
            Method::Anova(_, w) => std::mem::take(w),
            Method::Test(p) => w_of_p_values(p, self.alpha),
        };
        Step::Done(self.finish(w))
    }

    /// The outcome from W of every feature.
    fn finish(&self, w: Vec<usize>) -> AncomResult {
        let (features, alpha) = (self.features, self.alpha);
        let (clr, members) = (&self.clr, &self.members);
        let n = features as f64;
        let start = w.iter().copied().max().unwrap_or(0) as f64 / n;
        let cutoff = (start >= THETA).then(|| {
            let cuts: Vec<f64> = (0..5).map(|k| start - (0.05 + 0.05 * k as f64)).collect();
            let shares: Vec<f64> = cuts
                .iter()
                .map(|cut| w.iter().filter(|&&x| x as f64 > n * cut).count() as f64 / n)
                .collect();
            let mut changes: Vec<f64> = (0..5)
                .map(|k| (shares[k] - shares[(k + 1) % 5]).abs())
                .collect();
            changes[4] = 0.0;
            let nu = if changes[0] < TAU && changes[1] < TAU && changes[2] < TAU {
                cuts[1]
            } else if changes[0] >= TAU && changes[1] < TAU && changes[2] < TAU {
                cuts[2]
            } else if changes[1] >= TAU && changes[2] < TAU && changes[3] < TAU {
                cuts[3]
            } else {
                cuts[4]
            };
            nu * n
        });

        let rows = (0..features)
            .map(|f| {
                let values: Vec<Vec<f64>> = members
                    .iter()
                    .map(|m| m.iter().map(|&s| clr[s][f]).collect())
                    .collect();
                let clr_effect = if values.len() == 2 {
                    mean(&values[0]) - mean(&values[1])
                } else {
                    let slices: Vec<&[f64]> = values.iter().map(Vec::as_slice).collect();
                    eta_squared(&slices)
                };
                AncomRow {
                    w: w[f],
                    reject: cutoff.is_some_and(|c| w[f] as f64 >= c),
                    clr_effect,
                }
            })
            .collect();
        AncomResult {
            rows,
            cutoff,
            test: self.test,
            alpha,
        }
    }
}

/// W of every feature from the p-values of every pair of features: the
/// number of other features whose log ratio with it differs between the
/// groups, after the Holm-Bonferroni adjustment of the feature's p-values.
/// Each feature's p-values are adjusted together with its comparison with
/// itself (zero), as scikit-bio does; that one never counts.
fn w_of_p_values(p: &[Vec<f64>], alpha: f64) -> Vec<usize> {
    let mut rng = rng_from_seed(0);
    p.iter()
        .enumerate()
        .map(|(i, row)| {
            let adjusted = Correction::HolmBonferroni
                .correct(row, alpha, &mut rng)
                .values;
            adjusted
                .iter()
                .enumerate()
                .filter(|&(j, value)| j != i && *value < alpha)
                .count()
        })
        .collect()
}

/// W by one-way ANOVA, as the p-values of every pair give it, but faster.
/// Every pair has the same degrees of freedom, so a p-value passes the Holm
/// threshold for its rank exactly when the F statistic exceeds a critical
/// value that can be found once for each rank; the F statistics come from
/// sums kept for each feature and group, without the p-values themselves.
/// Every group must have at least two samples.
#[derive(Debug, Clone)]
struct AnovaSums {
    sizes: Vec<f64>,
    total: f64,
    dfn: f64,
    dfd: f64,
    /// Each feature's values, group after group.
    values: Vec<Vec<f64>>,
    ranges: Vec<std::ops::Range<usize>>,
    /// Each feature's sum and sum of squares in each group.
    sums: Vec<Vec<f64>>,
    squares: Vec<Vec<f64>>,
    /// The F statistic above which a p-value passes the Holm threshold, by
    /// rank.
    critical: Vec<f64>,
}

impl AnovaSums {
    fn new(clr: &[Vec<f64>], members: &[Vec<usize>], features: usize, alpha: f64) -> AnovaSums {
        let sizes: Vec<f64> = members.iter().map(|m| m.len() as f64).collect();
        let total: f64 = sizes.iter().sum();
        let (dfn, dfd) = (sizes.len() as f64 - 1.0, total - sizes.len() as f64);
        let order: Vec<usize> = members.iter().flatten().copied().collect();
        let values: Vec<Vec<f64>> = (0..features)
            .map(|f| order.iter().map(|&s| clr[s][f]).collect())
            .collect();
        let mut ranges = Vec::with_capacity(members.len());
        let mut begin = 0;
        for m in members {
            ranges.push(begin..begin + m.len());
            begin += m.len();
        }
        let sums: Vec<Vec<f64>> = values
            .iter()
            .map(|v| ranges.iter().map(|r| v[r.clone()].iter().sum()).collect())
            .collect();
        let squares: Vec<Vec<f64>> = values
            .iter()
            .map(|v| {
                ranges
                    .iter()
                    .map(|r| v[r.clone()].iter().map(|x| x * x).sum())
                    .collect()
            })
            .collect();
        // The Holm threshold at rank r of the m p-values of a feature (its
        // own, zero, first) is alpha / (m - r + 1); the F statistic above
        // which the p-value is smaller, found by bisection on the F
        // distribution.
        let m = features;
        let critical: Vec<f64> = (1..=m)
            .map(|rank| f_above(alpha / (m - rank + 1) as f64, dfn, dfd))
            .collect();
        AnovaSums {
            sizes,
            total,
            dfn,
            dfd,
            values,
            ranges,
            sums,
            squares,
            critical,
        }
    }

    /// W of feature `i`; `f_values` is room to work in.
    fn w(&self, i: usize, f_values: &mut Vec<f64>) -> usize {
        let AnovaSums {
            sizes,
            total,
            dfn,
            dfd,
            values,
            ranges,
            sums,
            squares,
            critical,
        } = self;
        let (total, dfn, dfd) = (*total, *dfn, *dfd);
        f_values.clear();
        for j in (0..values.len()).filter(|&j| j != i) {
            let (mut sum, mut square, mut between) = (0.0, 0.0, 0.0);
            for (g, range) in ranges.iter().enumerate() {
                let dot: f64 = values[i][range.clone()]
                    .iter()
                    .zip(&values[j][range.clone()])
                    .map(|(a, b)| a * b)
                    .sum();
                let group_sum = sums[i][g] - sums[j][g];
                sum += group_sum;
                square += squares[i][g] + squares[j][g] - 2.0 * dot;
                between += group_sum * group_sum / sizes[g];
            }
            let correction = sum * sum / total;
            let spread = square - correction;
            let between = between - correction;
            let within = spread - between;
            // A log ratio that is the same in every sample tells nothing
            // (ANOVA gives no p-value there).
            let f = if spread <= 1e-12 * square.max(f64::MIN_POSITIVE) {
                f64::NAN
            } else if within <= 0.0 {
                f64::INFINITY
            } else {
                (between / dfn) / (within / dfd)
            };
            f_values.push(f);
        }
        // Largest F (smallest p-value) first; no value at all last.
        f_values.sort_by(|a, b| match (a.is_nan(), b.is_nan()) {
            (false, false) => b.total_cmp(a),
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
        });
        // The step-down procedure rejects a run of the smallest p-values;
        // the feature's own comparison takes rank one.
        f_values
            .iter()
            .zip(&critical[1..])
            .take_while(|(f, c)| **f > **c)
            .count()
    }
}

/// The value of an F(dfn, dfd) statistic above which the upper-tail
/// probability is below `p`.
fn f_above(p: f64, dfn: f64, dfd: f64) -> f64 {
    if p >= 1.0 {
        return 0.0;
    }
    let mut high = 1.0;
    while f_sf(high, dfn, dfd) >= p {
        high *= 2.0;
        if high > 1e300 {
            return f64::INFINITY;
        }
    }
    let mut low = 0.0;
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        if middle <= low || middle >= high {
            break;
        }
        if f_sf(middle, dfn, dfd) >= p {
            low = middle;
        } else {
            high = middle;
        }
    }
    // The smallest F whose p-value is below `p`; anything above `low`
    // passes.
    low
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values are from scikit-bio 0.7.4 (`ancom` with its
    // defaults: alpha 0.05, Holm-Bonferroni, one-way ANOVA).
    fn table() -> Vec<Vec<f64>> {
        vec![
            vec![57.0, 45.0, 47.0, 55.0, 43.0, 51.0, 53.0, 29.0],
            vec![22.0, 32.0, 31.0, 54.0, 56.0, 20.0, 39.0, 52.0],
            vec![25.0, 51.0, 24.0, 38.0, 52.0, 32.0, 33.0, 31.0],
            vec![48.0, 30.0, 59.0, 37.0, 39.0, 40.0, 43.0, 42.0],
            vec![160.0, 59.0, 52.0, 51.0, 48.0, 44.0, 33.0, 59.0],
            vec![152.0, 28.0, 53.0, 26.0, 54.0, 44.0, 24.0, 21.0],
            vec![148.0, 21.0, 25.0, 40.0, 58.0, 38.0, 52.0, 56.0],
            vec![208.0, 45.0, 37.0, 40.0, 30.0, 39.0, 35.0, 29.0],
            vec![472.0, 4.0, 23.0, 27.0, 58.0, 47.0, 55.0, 28.0],
            vec![384.0, 6.8, 39.0, 20.0, 44.0, 53.0, 46.0, 26.0],
            vec![328.0, 6.0, 58.0, 55.0, 27.0, 40.0, 57.0, 53.0],
            vec![384.0, 9.0, 21.0, 49.0, 38.0, 23.0, 29.0, 41.0],
        ]
    }

    fn groups() -> Vec<usize> {
        vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]
    }

    fn ws(result: &AncomResult) -> Vec<usize> {
        result.rows.iter().map(|r| r.w).collect()
    }

    fn rejected(result: &AncomResult) -> Vec<bool> {
        result.rows.iter().map(|r| r.reject).collect()
    }

    #[test]
    fn three_groups_match_scikit_bio() {
        let result = ancom(&table(), &groups(), MultiGroupTest::Anova, 0.05).unwrap();
        assert_eq!(ws(&result), [7, 7, 2, 2, 2, 2, 2, 2]);
        assert_eq!(
            rejected(&result),
            [true, true, false, false, false, false, false, false]
        );
        assert_eq!(result.rejected(), 2);
        // Eta-squared lies between zero and one; the first feature differs
        // most.
        assert!(result
            .rows
            .iter()
            .all(|r| (0.0..=1.0).contains(&r.clr_effect)));
        assert!(result.rows[0].clr_effect > 0.8);

        // With four samples a group, the Kruskal-Wallis p-values do not
        // survive the Holm adjustment over eight features, so no W is above
        // zero.
        let ranks = ancom(&table(), &groups(), MultiGroupTest::KruskalWallis, 0.05).unwrap();
        assert_eq!(ws(&ranks), [0; 8]);
        assert_eq!(ranks.cutoff, None);
        assert_eq!(ranks.rejected(), 0);
    }

    #[test]
    fn two_groups_match_scikit_bio() {
        let table = table()[..8].to_vec();
        let result = ancom(&table, &groups()[..8], MultiGroupTest::Anova, 0.05).unwrap();
        assert_eq!(ws(&result), [7, 1, 1, 1, 1, 1, 1, 0]);
        assert_eq!(
            rejected(&result),
            [true, false, false, false, false, false, false, false]
        );
        // The first feature is more abundant in the second group.
        assert!(result.rows[0].clr_effect < -1.0);
    }

    #[test]
    fn zeros_and_degenerate_input() {
        let mut table = table();
        table[0][3] = 0.0;
        table[5][6] = 0.0;
        let result = ancom(&table, &groups(), MultiGroupTest::Anova, 0.05).unwrap();
        assert!(result.rows[0].reject);
        assert!(result.rows.iter().all(|r| r.clr_effect.is_finite()));

        // One group, one feature, or a sample in every group of its own.
        assert!(ancom(&table, &[0; 12], MultiGroupTest::Anova, 0.05).is_none());
        let single: Vec<Vec<f64>> = table.iter().map(|s| vec![s[0]]).collect();
        assert!(ancom(&single, &groups(), MultiGroupTest::Anova, 0.05).is_none());
        let own: Vec<usize> = (0..12).collect();
        assert!(ancom(&table, &own, MultiGroupTest::Anova, 0.05).is_none());
        assert!(ancom(&table, &groups()[..5], MultiGroupTest::Anova, 0.05).is_none());
    }

    /// The ANOVA shortcut gives the W of the general path, on data with
    /// zeros, ties and a feature constant in every sample.
    fn w_by_anova(
        clr: &[Vec<f64>],
        members: &[Vec<usize>],
        features: usize,
        alpha: f64,
    ) -> Vec<usize> {
        let sums = AnovaSums::new(clr, members, features, alpha);
        let mut room = Vec::new();
        (0..features).map(|i| sums.w(i, &mut room)).collect()
    }

    /// W through the p-value of every pair, whatever the test.
    fn w_by_test(
        clr: &[Vec<f64>],
        members: &[Vec<usize>],
        features: usize,
        test: MultiGroupTest,
        alpha: f64,
    ) -> Vec<usize> {
        let mut run = AncomRun {
            features,
            clr: clr.to_vec(),
            members: members.to_vec(),
            test,
            alpha,
            method: Method::Test(vec![vec![0.0; features]; features]),
            next: 0,
        };
        match run.advance(&mut |_, _| Control::Continue) {
            Step::Done(result) => result.rows.iter().map(|r| r.w).collect(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn the_anova_shortcut_agrees_with_the_general_path() {
        let mut state: u64 = 7;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as f64 / (1u64 << 31) as f64
        };
        let samples: Vec<Vec<f64>> = (0..30)
            .map(|s| {
                (0..60)
                    .map(|f| {
                        let shift = if f % 5 == 0 && s % 3 == 1 { 6.0 } else { 1.0 };
                        let value = (next() * 40.0 * shift).floor();
                        // Some zeros, and one feature the same everywhere.
                        if f == 7 {
                            10.0
                        } else if value < 3.0 {
                            0.0
                        } else {
                            value
                        }
                    })
                    .collect()
            })
            .collect();
        let groups: Vec<usize> = (0..30).map(|s| s % 3).collect();
        let clr: Vec<Vec<f64>> = samples.iter().map(|s| centred_log_ratio(s)).collect();
        let members: Vec<Vec<usize>> = (0..3)
            .map(|g| (0..30).filter(|&s| groups[s] == g).collect())
            .collect();
        for alpha in [0.05, 0.01, 0.2] {
            let fast = w_by_anova(&clr, &members, 60, alpha);
            let general = w_by_test(&clr, &members, 60, MultiGroupTest::Anova, alpha);
            assert_eq!(fast, general, "alpha {alpha}");
            assert!(fast.iter().any(|&w| w > 0));
        }
        assert!(f_above(1.0, 2.0, 10.0) == 0.0);
        let c = f_above(0.05, 2.0, 27.0);
        assert!((f_sf(c, 2.0, 27.0) - 0.05).abs() < 1e-12);
    }
}
