//! Tests on whole communities: do groups of samples differ in what they are
//! made of? All work on the distances between samples and get their
//! p-values by permuting which group each sample belongs to.

use super::correlation::{pearson, ranks, CorrelationMethod};
use super::diversity::{pcoa, DistanceMatrix, Ordination};
use super::results::{Control, Pace, Step};
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
type Statistic = Box<dyn Fn(&[usize]) -> f64 + Send>;

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
        self.run_with(distances, groups, permutations, rng, None)
    }

    /// [`run`](Self::run), given the principal coordinates of the
    /// distances on every axis ([`pcoa`]) if they have already been found,
    /// for PERMDISP, so that they are not found again.
    pub fn run_with(
        self,
        distances: &DistanceMatrix,
        groups: &[usize],
        permutations: usize,
        rng: &mut StampRng,
        ordination: Option<&Ordination>,
    ) -> PermutationTest {
        let mut run = self.start(distances, groups, permutations, ordination);
        match run.advance(rng, &mut |_, _| Control::Continue) {
            Step::Done(result) => result,
            Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
        }
    }

    /// The test as a [`PermutationRun`], to be carried out a permutation at
    /// a time with [`PermutationRun::advance`]; what does not change from
    /// one permutation to the next (and the observed statistic) is worked
    /// out here. Arguments as for [`run_with`](Self::run_with).
    pub fn start(
        self,
        distances: &DistanceMatrix,
        groups: &[usize],
        permutations: usize,
        ordination: Option<&Ordination>,
    ) -> PermutationRun {
        let n = distances.size();
        let group_count = groups.iter().copied().max().map_or(0, |g| g + 1);
        let undefined = PermutationTest {
            statistic: f64::NAN,
            p_value: f64::NAN,
            permutations,
        };
        if groups.len() != n || group_count < 2 || n <= group_count {
            return PermutationRun::finished(undefined);
        }
        // The principal coordinates found here for PERMDISP, if not given.
        let found: Vec<Vec<f64>>;
        // What does not change when samples are moved between groups.
        let statistic: Statistic = match self {
            CommunityTest::Permanova => {
                // Squared distances of the pairs, in the order visited.
                let squared: Vec<f64> = (0..n)
                    .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
                    .map(|(i, j)| distances.get(i, j) * distances.get(i, j))
                    .collect();
                let total: f64 = squared.iter().sum::<f64>() / n as f64;
                // Permutations keep the size of every group.
                let mut sizes = vec![0.0; group_count];
                for &g in groups {
                    sizes[g] += 1.0;
                }
                Box::new(move |groups: &[usize]| {
                    let mut within = 0.0;
                    let mut pairs = squared.iter();
                    for i in 0..n {
                        let (gi, size) = (groups[i], sizes[groups[i]]);
                        for (&gj, &d2) in groups[i + 1..].iter().zip(pairs.by_ref()) {
                            if gi == gj {
                                within += d2 / size;
                            }
                        }
                    }
                    let between = total - within;
                    (between / (group_count as f64 - 1.0)) / (within / (n - group_count) as f64)
                })
            }
            CommunityTest::Anosim => {
                // Ranks of the distances, ties sharing their average, in the
                // order the pairs are visited.
                let mut pairs: Vec<(f64, usize)> = (0..n)
                    .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
                    .enumerate()
                    .map(|(k, (i, j))| (distances.get(i, j), k))
                    .collect();
                pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut ranks = vec![0.0; pairs.len()];
                let mut start = 0;
                while start < pairs.len() {
                    let mut end = start + 1;
                    while end < pairs.len() && pairs[end].0 == pairs[start].0 {
                        end += 1;
                    }
                    let rank = (start + 1 + end) as f64 / 2.0;
                    for &(_, k) in &pairs[start..end] {
                        ranks[k] = rank;
                    }
                    start = end;
                }
                let pair_count = pairs.len() as f64;
                // Ranks are whole or half numbers, so their sums are exact
                // in any order, and the sum between groups is the total less
                // the sum within. Permutations keep the size of every group,
                // and with it the number of pairs within groups.
                let rank_total: f64 = ranks.iter().sum();
                let mut sizes = vec![0usize; group_count];
                for &g in groups {
                    sizes[g] += 1;
                }
                let within_count = sizes
                    .iter()
                    .map(|&s| (s * s.saturating_sub(1) / 2) as f64)
                    .sum::<f64>();
                let between_count = pair_count - within_count;
                Box::new(move |groups: &[usize]| {
                    let mut within = 0.0;
                    let mut pairs = ranks.iter();
                    for i in 0..n {
                        let gi = groups[i];
                        for (&gj, &rank) in groups[i + 1..].iter().zip(pairs.by_ref()) {
                            if gi == gj {
                                within += rank;
                            }
                        }
                    }
                    let between = rank_total - within;
                    (between / between_count - within / within_count) / (pair_count / 2.0)
                })
            }
            CommunityTest::Permdisp => {
                // The samples as points, so that centroids can be taken.
                let points: &[Vec<f64>] = match ordination {
                    Some(ordination) => &ordination.positions,
                    None => {
                        let Some(ordination) = pcoa(distances) else {
                            return PermutationRun::finished(undefined);
                        };
                        found = ordination.positions;
                        &found
                    }
                };
                let axes = points.first().map_or(0, Vec::len);
                // The points one after another, for speed.
                let flat: Vec<f64> = points.iter().flatten().copied().collect();
                let mut sizes = vec![0.0; group_count];
                for &g in groups {
                    sizes[g] += 1.0;
                }
                Box::new(move |groups: &[usize]| {
                    let mut centroids = vec![0.0; axes * group_count];
                    for (point, &g) in flat.chunks_exact(axes.max(1)).zip(groups) {
                        let centroid = &mut centroids[g * axes..(g + 1) * axes];
                        for (sum, value) in centroid.iter_mut().zip(point) {
                            *sum += value;
                        }
                    }
                    for (centroid, size) in centroids.chunks_exact_mut(axes.max(1)).zip(&sizes) {
                        for value in centroid.iter_mut() {
                            *value /= size;
                        }
                    }
                    // One-way analysis of variance of the distances of the
                    // samples to the centroid of their group.
                    let spread: Vec<f64> = flat
                        .chunks_exact(axes.max(1))
                        .zip(groups)
                        .map(|(point, &g)| {
                            point
                                .iter()
                                .zip(&centroids[g * axes..(g + 1) * axes])
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
            return PermutationRun::finished(PermutationTest {
                statistic: observed,
                p_value: f64::NAN,
                permutations,
            });
        }
        PermutationRun {
            statistic: Some(statistic),
            observed,
            // A permutation that only swaps samples within their groups
            // gives the observed statistic again, up to rounding; it must
            // count.
            threshold: observed - 1e-9 * observed.abs().max(1e-300),
            shuffled: groups.to_vec(),
            as_extreme: 1,
            done: 0,
            permutations,
            result: None,
        }
    }
}

/// A community test carried out a permutation at a time (from
/// [`CommunityTest::start`]), so that whatever runs it can show its
/// progress, attend to other things in between, or give up. The outcome
/// does not depend on where it pauses: the permutations draw the same
/// random numbers in the same order.
pub struct PermutationRun {
    /// The statistic of a grouping; `None` once the outcome is known.
    statistic: Option<Statistic>,
    observed: f64,
    threshold: f64,
    /// The grouping, shuffled once per permutation.
    shuffled: Vec<usize>,
    /// Groupings with a statistic at least as large, the real one included.
    as_extreme: usize,
    done: usize,
    permutations: usize,
    /// The outcome, when known without permutations.
    result: Option<PermutationTest>,
}

impl PermutationRun {
    fn finished(result: PermutationTest) -> PermutationRun {
        PermutationRun {
            statistic: None,
            observed: result.statistic,
            threshold: 0.0,
            shuffled: Vec::new(),
            as_extreme: 0,
            done: 0,
            permutations: result.permutations,
            result: Some(result),
        }
    }

    /// Make further permutations, drawing on `rng`, which must be the same
    /// on every call. `pace` is asked before each, with the number made
    /// and the number to make.
    pub fn advance(&mut self, rng: &mut StampRng, pace: Pace<'_>) -> Step<PermutationTest> {
        if let Some(result) = self.result {
            return Step::Done(result);
        }
        let Some(statistic) = &self.statistic else {
            unreachable!("a run without an outcome has a statistic");
        };
        while self.done < self.permutations {
            match pace(self.done, self.permutations) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            for i in (1..self.shuffled.len()).rev() {
                self.shuffled.swap(i, random_index(rng, i + 1));
            }
            if statistic(&self.shuffled) >= self.threshold {
                self.as_extreme += 1;
            }
            self.done += 1;
        }
        let result = PermutationTest {
            statistic: self.observed,
            p_value: self.as_extreme as f64 / (self.permutations + 1) as f64,
            permutations: self.permutations,
        };
        self.result = Some(result);
        Step::Done(result)
    }
}

/// PERMANOVA between two of the groups, one of the pairs of a
/// [`PairwiseRun`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairwiseTest {
    /// The two groups, as numbered in the grouping given.
    pub first: usize,
    pub second: usize,
    /// PERMANOVA on the samples of the two groups alone. Undefined (`NaN`)
    /// if they have only two samples between them.
    pub test: PermutationTest,
    /// The p-value corrected for the number of pairs by Holm's method.
    pub p_value_corrected: f64,
}

/// Pairwise PERMANOVA: which groups differ, once PERMANOVA on all of them
/// says that some do? Each pair of groups is tested by PERMANOVA on the
/// distances between their samples alone, as the pairwiseAdonis package for
/// R does, and the p-values are corrected by Holm's method. Carried out a
/// permutation at a time, like [`PermutationRun`]; the outcome does not
/// depend on where it pauses.
pub struct PairwiseRun {
    /// The samples of each group.
    members: Vec<Vec<usize>>,
    /// The pairs of groups, in order.
    pairs: Vec<(usize, usize)>,
    permutations: usize,
    finished: Vec<PermutationTest>,
    current: Option<PermutationRun>,
}

impl PairwiseRun {
    /// `groups[i]` is the group of sample `i`, as a number from zero.
    pub fn new(groups: &[usize], permutations: usize) -> PairwiseRun {
        let count = groups.iter().copied().max().map_or(0, |g| g + 1);
        let mut members = vec![Vec::new(); count];
        for (sample, &group) in groups.iter().enumerate() {
            members[group].push(sample);
        }
        let pairs = (0..count)
            .flat_map(|a| (a + 1..count).map(move |b| (a, b)))
            .collect();
        PairwiseRun {
            members,
            pairs,
            permutations,
            finished: Vec::new(),
            current: None,
        }
    }

    /// The number of pairs of groups.
    pub fn pairs(&self) -> usize {
        self.pairs.len()
    }

    /// Carry the tests further, drawing on `rng`, which must be the same
    /// on every call, as must `distances`. `pace` is asked before each
    /// permutation, with the number made and to make over all the pairs.
    pub fn advance(
        &mut self,
        distances: &DistanceMatrix,
        rng: &mut StampRng,
        pace: Pace<'_>,
    ) -> Step<Vec<PairwiseTest>> {
        let total = self.pairs.len() * self.permutations;
        while self.finished.len() < self.pairs.len() {
            let (a, b) = self.pairs[self.finished.len()];
            let before = self.finished.len() * self.permutations;
            let members = &self.members;
            let permutations = self.permutations;
            let run = self.current.get_or_insert_with(|| {
                let samples: Vec<usize> = members[a].iter().chain(&members[b]).copied().collect();
                let subset = DistanceMatrix::from_fn(samples.len(), |i, j| {
                    distances.get(samples[i], samples[j])
                });
                let groups: Vec<usize> = (0..samples.len())
                    .map(|i| usize::from(i >= members[a].len()))
                    .collect();
                CommunityTest::Permanova.start(&subset, &groups, permutations, None)
            });
            match run.advance(rng, &mut |done, _| pace(before + done, total)) {
                Step::Done(result) => {
                    self.finished.push(result);
                    self.current = None;
                }
                Step::Paused => return Step::Paused,
                Step::Cancelled => return Step::Cancelled,
            }
        }
        let p_values: Vec<f64> = self.finished.iter().map(|t| t.p_value).collect();
        let corrected = holm(&p_values);
        Step::Done(
            self.pairs
                .iter()
                .zip(&self.finished)
                .zip(corrected)
                .map(
                    |((&(first, second), &test), p_value_corrected)| PairwiseTest {
                        first,
                        second,
                        test,
                        p_value_corrected,
                    },
                )
                .collect(),
        )
    }
}

/// [`PairwiseRun`] carried out in one piece.
pub fn pairwise_permanova(
    distances: &DistanceMatrix,
    groups: &[usize],
    permutations: usize,
    rng: &mut StampRng,
) -> Vec<PairwiseTest> {
    let mut run = PairwiseRun::new(groups, permutations);
    match run.advance(distances, rng, &mut |_, _| Control::Continue) {
        Step::Done(tests) => tests,
        Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
    }
}

/// Holm's correction of the p-values that are numbers; the others stay as
/// they are and are not counted among the tests.
fn holm(p_values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..p_values.len())
        .filter(|&i| !p_values[i].is_nan())
        .collect();
    order.sort_by(|&a, &b| p_values[a].total_cmp(&p_values[b]));
    let mut corrected = p_values.to_vec();
    let mut largest: f64 = 0.0;
    for (rank, &i) in order.iter().enumerate() {
        largest = largest.max(((order.len() - rank) as f64 * p_values[i]).min(1.0));
        corrected[i] = largest;
    }
    corrected
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
    match MantelRun::new(x, y, method, permutations).advance(rng, &mut |_, _| Control::Continue) {
        Step::Done(result) => result,
        Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
    }
}

/// The [`mantel`] test carried out a permutation at a time, so that whatever
/// runs it can show its progress, attend to other things in between, or
/// give up. The outcome does not depend on where it pauses.
#[derive(Debug, Clone)]
pub struct MantelRun {
    n: usize,
    /// The first matrix, or for Spearman's rho the ranks of its distances:
    /// permuting the samples only reorders the pairs, and with them their
    /// ranks, so the distances are ranked once rather than once per
    /// permutation (the coefficient is exactly the same).
    x: DistanceMatrix,
    /// The pairs of the second matrix, or their ranks.
    fixed: Vec<f64>,
    observed: f64,
    threshold: f64,
    order: Vec<usize>,
    /// The pairs of the permuted first matrix, kept to save allocating.
    buffer: Vec<f64>,
    as_extreme: usize,
    done: usize,
    permutations: usize,
    result: Option<PermutationTest>,
}

impl MantelRun {
    /// Arguments as for [`mantel`]; the observed statistic is found here.
    pub fn new(
        x: &DistanceMatrix,
        y: &DistanceMatrix,
        method: CorrelationMethod,
        permutations: usize,
    ) -> MantelRun {
        let n = x.size();
        let undefined = PermutationTest {
            statistic: f64::NAN,
            p_value: f64::NAN,
            permutations,
        };
        let finished = |result: PermutationTest| MantelRun {
            n,
            x: DistanceMatrix::from_fn(0, |_, _| 0.0),
            fixed: Vec::new(),
            observed: result.statistic,
            threshold: 0.0,
            order: Vec::new(),
            buffer: Vec::new(),
            as_extreme: 0,
            done: 0,
            permutations,
            result: Some(result),
        };
        if n < 3 || y.size() != n {
            return finished(undefined);
        }
        let identity: Vec<usize> = (0..n).collect();
        let fixed = pairs_of(y, &identity);
        let first = pairs_of(x, &identity);
        let Some(observed) = method.coefficient(&first, &fixed) else {
            return finished(undefined);
        };
        if permutations == 0 {
            return finished(PermutationTest {
                statistic: observed,
                ..undefined
            });
        }
        let (x, fixed) = match method {
            CorrelationMethod::Pearson => (x.clone(), fixed),
            CorrelationMethod::Spearman => {
                let ranked = ranks(&first);
                // Where pair (i, j), i < j, is in the order of the pairs.
                let index = |i: usize, j: usize| i * n - i * (i + 1) / 2 + (j - i - 1);
                (
                    DistanceMatrix::from_fn(n, |i, j| ranked[index(i, j)]),
                    ranks(&fixed),
                )
            }
        };
        MantelRun {
            n,
            x,
            fixed,
            observed,
            threshold: observed.abs() - 1e-9,
            order: identity,
            buffer: Vec::with_capacity(first.len()),
            as_extreme: 1,
            done: 0,
            permutations,
            result: None,
        }
    }

    /// Make further permutations, drawing on `rng`, which must be the same
    /// on every call. `pace` is asked before each, with the number made
    /// and the number to make.
    pub fn advance(&mut self, rng: &mut StampRng, pace: Pace<'_>) -> Step<PermutationTest> {
        if let Some(result) = self.result {
            return Step::Done(result);
        }
        let n = self.n;
        while self.done < self.permutations {
            match pace(self.done, self.permutations) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            for i in (1..n).rev() {
                self.order.swap(i, random_index(rng, i + 1));
            }
            self.buffer.clear();
            for i in 0..n {
                for j in i + 1..n {
                    self.buffer.push(self.x.get(self.order[i], self.order[j]));
                }
            }
            let r = pearson(&self.buffer, &self.fixed).unwrap_or(0.0);
            if r.abs() >= self.threshold {
                self.as_extreme += 1;
            }
            self.done += 1;
        }
        let result = PermutationTest {
            statistic: self.observed,
            p_value: self.as_extreme as f64 / (self.permutations + 1) as f64,
            permutations: self.permutations,
        };
        self.result = Some(result);
        Step::Done(result)
    }
}

/// The distances between every pair of samples, `order[i]` taking the place
/// of sample `i`.
fn pairs_of(m: &DistanceMatrix, order: &[usize]) -> Vec<f64> {
    let n = m.size();
    (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .map(|(i, j)| m.get(order[i], order[j]))
        .collect()
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

    /// Each pair is PERMANOVA on the samples of the two groups alone (whose
    /// statistic is checked against scikit-bio above), and the p-values
    /// are Holm's: R's `p.adjust(c(0.01, 0.04, 0.03), "holm")` gives 0.03,
    /// 0.06, 0.06.
    #[test]
    fn pairwise_permanova_tests_each_pair_alone() {
        let matrix = distances(&table(), Distance::BrayCurtis);
        let tests = pairwise_permanova(&matrix, &THREE, 999, &mut rng_from_seed(1));
        let pairs: Vec<(usize, usize)> = tests.iter().map(|t| (t.first, t.second)).collect();
        assert_eq!(pairs, [(0, 1), (0, 2), (1, 2)]);
        // Groups 0 and 2: samples 0, 1 and 4, 5, 6.
        let samples = [0, 1, 4, 5, 6];
        let subset = DistanceMatrix::from_fn(5, |i, j| matrix.get(samples[i], samples[j]));
        let alone =
            CommunityTest::Permanova.run(&subset, &[0, 0, 1, 1, 1], 999, &mut rng_from_seed(2));
        assert!((tests[1].test.statistic - alone.statistic).abs() < 1e-12);
        // Two groups of two samples allow only three groupings: p >= 1/3.
        assert!(tests[0].test.p_value > 0.3);
        for t in &tests {
            assert!(t.p_value_corrected >= t.test.p_value);
            assert!(t.p_value_corrected <= 1.0);
        }
        let corrected = holm(&[0.01, 0.04, 0.03, f64::NAN]);
        for (value, expected) in corrected.iter().zip([0.03, 0.06, 0.06]) {
            assert!((value - expected).abs() < 1e-12);
        }
        assert!(corrected[3].is_nan());

        // The same wherever it pauses.
        let mut run = PairwiseRun::new(&THREE, 999);
        let mut rng = rng_from_seed(1);
        let mut calls = 0;
        let paused = loop {
            let step = run.advance(&matrix, &mut rng, &mut |_, _| {
                calls += 1;
                if calls % 7 == 0 {
                    Control::Pause
                } else {
                    Control::Continue
                }
            });
            if let Step::Done(tests) = step {
                break tests;
            }
        };
        assert_eq!(paused, tests);
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

    /// Spearman's Mantel test ranks the distances once; ranking them again
    /// for every permutation, as it first did, gives exactly the same.
    #[test]
    fn mantel_ranks_once_without_changing_the_result() {
        let x = distances(&table(), Distance::BrayCurtis);
        let y = distances(&table(), Distance::Aitchison);
        let n = x.size();
        let naive = |rng: &mut StampRng| {
            let identity: Vec<usize> = (0..n).collect();
            let fixed = pairs_of(&y, &identity);
            let method = CorrelationMethod::Spearman;
            let observed = method
                .coefficient(&pairs_of(&x, &identity), &fixed)
                .unwrap();
            let mut order = identity;
            let mut as_extreme = 1;
            for _ in 0..199 {
                for i in (1..n).rev() {
                    order.swap(i, random_index(rng, i + 1));
                }
                let r = method
                    .coefficient(&pairs_of(&x, &order), &fixed)
                    .unwrap_or(0.0);
                if r.abs() >= observed.abs() - 1e-9 {
                    as_extreme += 1;
                }
            }
            (observed, as_extreme as f64 / 200.0)
        };
        let (statistic, p_value) = naive(&mut rng_from_seed(4));
        let test = mantel(
            &x,
            &y,
            CorrelationMethod::Spearman,
            199,
            &mut rng_from_seed(4),
        );
        assert_eq!((test.statistic, test.p_value), (statistic, p_value));
    }
}
