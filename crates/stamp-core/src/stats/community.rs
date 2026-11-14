//! Tests on whole communities: do groups of samples differ in what they are
//! made of? All work on the distances between samples and get their
//! p-values by permuting which group each sample belongs to.

use super::correlation::CorrelationMethod;
use super::diversity::{pcoa, DistanceMatrix};
use super::two_sample::random_index;
use super::StampRng;

/// The outcome of a permutation test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PermutationTest {
    /// The test statistic of the samples as they are grouped.
    pub statistic: f64,
    /// The share of groupings, among the permutations and the real one,
    /// with a statistic at least as large. `NaN` if no permutations were
    /// asked for or the statistic is undefined.
    pub p_value: f64,
    pub permutations: usize,
}

/// Tests of whether groups of samples differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommunityTest {
    /// Permutational multivariate analysis of variance (Anderson 2001):
    /// are the groups centred in different places? The statistic is a
    /// pseudo-F ratio.
    Permanova,
    /// Analysis of similarities (Clarke 1993): are samples of the same
    /// group more alike than samples of different groups? The statistic R
    /// runs from about zero (no) to one (completely).
    Anosim,
    /// Permutational test of multivariate dispersion (Anderson 2006): do
    /// the groups differ in how spread out they are? A significant
    /// PERMANOVA can be caused by this alone. The statistic is the F ratio
    /// of the distances of samples to the centroid of their group.
    Permdisp,
}

/// A test statistic as a function of the group of each sample.
type Statistic<'a> = Box<dyn Fn(&[usize]) -> f64 + 'a>;

impl CommunityTest {
    pub const ALL: [CommunityTest; 3] = [
        CommunityTest::Permanova,
        CommunityTest::Anosim,
        CommunityTest::Permdisp,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CommunityTest::Permanova => "PERMANOVA",
            CommunityTest::Anosim => "ANOSIM",
            CommunityTest::Permdisp => "PERMDISP",
        }
    }

    /// Name of the test statistic.
    pub fn statistic_name(self) -> &'static str {
        match self {
            CommunityTest::Permanova => "pseudo-F",
            CommunityTest::Anosim => "R",
            CommunityTest::Permdisp => "F",
        }
    }

    /// Run the test. `groups[i]` is the group of sample `i`, as a number
    /// from zero; samples of every group must be present. The statistic is
    /// `NaN` if there are fewer than two groups or no more samples than
    /// groups.
    pub fn run(
        self,
        distances: &DistanceMatrix,
        groups: &[usize],
        permutations: usize,
        rng: &mut StampRng,
    ) -> PermutationTest {
        let n = distances.size();
        let group_count = groups.iter().copied().max().map_or(0, |g| g + 1);
        let undefined = PermutationTest {
            statistic: f64::NAN,
            p_value: f64::NAN,
            permutations,
        };
        if groups.len() != n || group_count < 2 || n <= group_count {
            return undefined;
        }
        // What does not change when samples are moved between groups.
        let statistic: Statistic = match self {
            CommunityTest::Permanova => {
                let total: f64 = (0..n)
                    .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
                    .map(|(i, j)| distances.get(i, j) * distances.get(i, j))
                    .sum::<f64>()
                    / n as f64;
                Box::new(move |groups: &[usize]| {
                    let mut sizes = vec![0.0; group_count];
                    for &g in groups {
                        sizes[g] += 1.0;
                    }
                    let mut within = 0.0;
                    for i in 0..n {
                        for j in i + 1..n {
                            if groups[i] == groups[j] {
                                within +=
                                    distances.get(i, j) * distances.get(i, j) / sizes[groups[i]];
                            }
                        }
                    }
                    let between = total - within;
                    (between / (group_count as f64 - 1.0)) / (within / (n - group_count) as f64)
                })
            }
            CommunityTest::Anosim => {
                // Ranks of the distances, ties sharing their average.
                let mut pairs: Vec<(f64, usize, usize)> = (0..n)
                    .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
                    .map(|(i, j)| (distances.get(i, j), i, j))
                    .collect();
                pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut ranks = vec![0.0; n * n];
                let mut start = 0;
                while start < pairs.len() {
                    let mut end = start + 1;
                    while end < pairs.len() && pairs[end].0 == pairs[start].0 {
                        end += 1;
                    }
                    let rank = (start + 1 + end) as f64 / 2.0;
                    for &(_, i, j) in &pairs[start..end] {
                        ranks[i * n + j] = rank;
                    }
                    start = end;
                }
                let pair_count = pairs.len() as f64;
                Box::new(move |groups: &[usize]| {
                    let (mut within, mut within_count) = (0.0, 0.0);
                    let (mut between, mut between_count) = (0.0, 0.0);
                    for i in 0..n {
                        for j in i + 1..n {
                            if groups[i] == groups[j] {
                                within += ranks[i * n + j];
                                within_count += 1.0;
                            } else {
                                between += ranks[i * n + j];
                                between_count += 1.0;
                            }
                        }
                    }
                    (between / between_count - within / within_count) / (pair_count / 2.0)
                })
            }
            CommunityTest::Permdisp => {
                // The samples as points, so that centroids can be taken.
                let Some(ordination) = pcoa(distances) else {
                    return undefined;
                };
                let points = ordination.positions;
                let axes = points.first().map_or(0, Vec::len);
                Box::new(move |groups: &[usize]| {
                    let mut sizes = vec![0.0; group_count];
                    let mut centroids = vec![vec![0.0; axes]; group_count];
                    for (point, &g) in points.iter().zip(groups) {
                        sizes[g] += 1.0;
                        for (sum, value) in centroids[g].iter_mut().zip(point) {
                            *sum += value;
                        }
                    }
                    for (centroid, size) in centroids.iter_mut().zip(&sizes) {
                        for value in centroid.iter_mut() {
                            *value /= size;
                        }
                    }
                    // One-way analysis of variance of the distances of the
                    // samples to the centroid of their group.
                    let spread: Vec<f64> = points
                        .iter()
                        .zip(groups)
                        .map(|(point, &g)| {
                            point
                                .iter()
                                .zip(&centroids[g])
                                .map(|(a, b)| (a - b) * (a - b))
                                .sum::<f64>()
                                .sqrt()
                        })
                        .collect();
                    let overall = spread.iter().sum::<f64>() / n as f64;
                    let mut group_means = vec![0.0; group_count];
                    for (value, &g) in spread.iter().zip(groups) {
                        group_means[g] += value / sizes[g];
                    }
                    let between: f64 = group_means
                        .iter()
                        .zip(&sizes)
                        .map(|(mean, size)| size * (mean - overall) * (mean - overall))
                        .sum();
                    let within: f64 = spread
                        .iter()
                        .zip(groups)
                        .map(|(value, &g)| (value - group_means[g]) * (value - group_means[g]))
                        .sum();
                    (between / (group_count as f64 - 1.0)) / (within / (n - group_count) as f64)
                })
            }
        };

        let observed = statistic(groups);
        if observed.is_nan() || permutations == 0 {
            return PermutationTest {
                statistic: observed,
                p_value: f64::NAN,
                permutations,
            };
        }
        // A permutation that only swaps samples within their groups gives
        // the observed statistic again, up to rounding; it must count.
        let threshold = observed - 1e-9 * observed.abs().max(1e-300);
        let mut shuffled = groups.to_vec();
        let mut as_extreme = 1;
        for _ in 0..permutations {
            for i in (1..shuffled.len()).rev() {
                shuffled.swap(i, random_index(rng, i + 1));
            }
            if statistic(&shuffled) >= threshold {
                as_extreme += 1;
            }
        }
        PermutationTest {
            statistic: observed,
            p_value: as_extreme as f64 / (permutations + 1) as f64,
            permutations,
        }
    }
}

/// The Mantel test (Mantel 1967): are two sets of distances between the
/// same samples related? The statistic is the correlation of the distances
/// between every pair of samples, by `method`; its two-sided p-value comes
/// from permuting the samples of the first matrix (rows and columns
/// together) and counting correlations at least as far from zero, as
/// scikit-bio does. The statistic is `NaN` for fewer than three samples or
/// distances that do not vary.
pub fn mantel(
    x: &DistanceMatrix,
    y: &DistanceMatrix,
    method: CorrelationMethod,
    permutations: usize,
    rng: &mut StampRng,
) -> PermutationTest {
    let n = x.size();
    let undefined = PermutationTest {
        statistic: f64::NAN,
        p_value: f64::NAN,
        permutations,
    };
    if n < 3 || y.size() != n {
        return undefined;
    }
    let pairs = |m: &DistanceMatrix, order: &[usize]| -> Vec<f64> {
        (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .map(|(i, j)| m.get(order[i], order[j]))
            .collect()
    };
    let identity: Vec<usize> = (0..n).collect();
    let fixed = pairs(y, &identity);
    let Some(observed) = method.coefficient(&pairs(x, &identity), &fixed) else {
        return undefined;
    };
    if permutations == 0 {
        return PermutationTest {
            statistic: observed,
            ..undefined
        };
    }
    let threshold = observed.abs() - 1e-9;
    let mut order = identity;
    let mut as_extreme = 1;
    for _ in 0..permutations {
        for i in (1..n).rev() {
            order.swap(i, random_index(rng, i + 1));
        }
        let r = method.coefficient(&pairs(x, &order), &fixed).unwrap_or(0.0);
        if r.abs() >= threshold {
            as_extreme += 1;
        }
    }
    PermutationTest {
        statistic: observed,
        p_value: as_extreme as f64 / (permutations + 1) as f64,
        permutations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::diversity::tests::table;
    use crate::stats::diversity::{distances, Distance};
    use crate::stats::rng_from_seed;

    const TWO: [usize; 7] = [0, 0, 0, 1, 1, 1, 1];
    const THREE: [usize; 7] = [0, 0, 1, 1, 2, 2, 2];

    fn run(test: CommunityTest, groups: &[usize]) -> PermutationTest {
        let matrix = distances(&table(), Distance::BrayCurtis);
        test.run(&matrix, groups, 999, &mut rng_from_seed(1))
    }

    /// Statistics from scikit-bio 0.7.4 on the same Bray-Curtis distances;
    /// its p-values, from other random permutations, were 0.032 and 0.267
    /// (PERMANOVA), 0.015 and 0.255 (ANOSIM), 0.147 and 0.022 (PERMDISP).
    #[test]
    fn statistics_match_scikit_bio() {
        let cases = [
            (
                CommunityTest::Permanova,
                &TWO[..],
                35.843473609153584,
                0.032,
            ),
            (
                CommunityTest::Permanova,
                &THREE[..],
                2.2050778346659254,
                0.267,
            ),
            (CommunityTest::Anosim, &TWO[..], 1.0, 0.015),
            (CommunityTest::Anosim, &THREE[..], 0.25, 0.255),
            (CommunityTest::Permdisp, &TWO[..], 2.078741925721348, 0.147),
            (
                CommunityTest::Permdisp,
                &THREE[..],
                99.72676199454071,
                0.022,
            ),
        ];
        for (test, groups, statistic, p_value) in cases {
            let result = run(test, groups);
            assert!(
                (result.statistic - statistic).abs() < 1e-8 * statistic.abs().max(1.0),
                "{}: {} against {statistic}",
                test.name(),
                result.statistic
            );
            // Seven samples allow few distinct groupings, so p-values from
            // different permutations agree closely.
            assert!(
                (result.p_value - p_value).abs() < 0.04,
                "{}: p {} against {p_value}",
                test.name(),
                result.p_value
            );
            assert_eq!(result.permutations, 999);
        }
    }

    #[test]
    fn p_values_are_repeatable_and_bounded() {
        let first = run(CommunityTest::Permanova, &TWO);
        assert_eq!(first, run(CommunityTest::Permanova, &TWO));
        // The smallest possible value is one in the permutations plus one.
        assert!(first.p_value >= 1.0 / 1000.0 && first.p_value <= 1.0);
        // Three samples against four can be split in 35 ways, and only the
        // real split separates the groups completely.
        assert!(
            (first.p_value - 1.0 / 35.0).abs() < 0.02,
            "{}",
            first.p_value
        );

        let matrix = distances(&table(), Distance::BrayCurtis);
        let none = CommunityTest::Anosim.run(&matrix, &TWO, 0, &mut rng_from_seed(1));
        assert_eq!(none.statistic, 1.0);
        assert!(none.p_value.is_nan());
    }

    #[test]
    fn undefined_cases() {
        let matrix = distances(&table(), Distance::BrayCurtis);
        let mut rng = rng_from_seed(1);
        for test in CommunityTest::ALL {
            // One group, a group per sample, or the wrong number of labels.
            assert!(test.run(&matrix, &[0; 7], 99, &mut rng).statistic.is_nan());
            assert!(test
                .run(&matrix, &[0, 1, 2, 3, 4, 5, 6], 99, &mut rng)
                .statistic
                .is_nan());
            assert!(test.run(&matrix, &[0, 1], 99, &mut rng).statistic.is_nan());
            assert!(!test.name().is_empty() && !test.statistic_name().is_empty());
        }
    }

    fn matrix(rows: [[f64; 5]; 5]) -> DistanceMatrix {
        DistanceMatrix::from_fn(5, |i, j| rows[i][j])
    }

    /// Statistics from scikit-bio 0.7.4 (`mantel`); its p-value was 0.021
    /// for both with 999 permutations, which our own permutations must
    /// come close to: only the identity and its reverse-ordered relatives
    /// are as extreme among the 120 orderings of five samples.
    #[test]
    fn mantel_matches_scikit_bio() {
        let x = matrix([
            [0.0, 1.0, 2.0, 3.0, 4.0],
            [1.0, 0.0, 1.5, 2.5, 3.5],
            [2.0, 1.5, 0.0, 1.2, 2.0],
            [3.0, 2.5, 1.2, 0.0, 0.7],
            [4.0, 3.5, 2.0, 0.7, 0.0],
        ]);
        let y = matrix([
            [0.0, 2.0, 3.0, 7.0, 9.0],
            [2.0, 0.0, 2.0, 5.0, 8.0],
            [3.0, 2.0, 0.0, 4.0, 4.0],
            [7.0, 5.0, 4.0, 0.0, 2.0],
            [9.0, 8.0, 4.0, 2.0, 0.0],
        ]);
        let mut rng = rng_from_seed(1);
        let pearson = mantel(&x, &y, CorrelationMethod::Pearson, 999, &mut rng);
        assert!((pearson.statistic - 0.9461893613372058).abs() < 1e-12);
        assert!(
            pearson.p_value > 0.005 && pearson.p_value < 0.05,
            "{}",
            pearson.p_value
        );
        let spearman = mantel(&x, &y, CorrelationMethod::Spearman, 999, &mut rng);
        assert!((spearman.statistic - 0.9167365318904004).abs() < 1e-12);
        assert!(spearman.p_value < 0.05);
        // The same seed gives the same p-value.
        let again = mantel(
            &x,
            &y,
            CorrelationMethod::Pearson,
            999,
            &mut rng_from_seed(1),
        );
        assert_eq!(again, pearson);

        // A matrix against itself, and undefined cases.
        assert!(
            (mantel(&x, &x, CorrelationMethod::Pearson, 0, &mut rng).statistic - 1.0).abs() < 1e-12
        );
        let flat = DistanceMatrix::from_fn(5, |_, _| 1.0);
        assert!(mantel(&x, &flat, CorrelationMethod::Pearson, 9, &mut rng)
            .statistic
            .is_nan());
        let small = DistanceMatrix::from_fn(2, |_, _| 1.0);
        assert!(
            mantel(&small, &small, CorrelationMethod::Pearson, 9, &mut rng)
                .statistic
                .is_nan()
        );
    }
}
