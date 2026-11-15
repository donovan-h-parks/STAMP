//! Diversity of communities: within a sample (alpha diversity) and between
//! samples (distances, also called beta diversity), and the ordination of
//! samples by their distances.

use super::pca::symmetric_eigen;
use super::rarefaction::{chao1, whole_counts};
use super::results::{Control, Pace, Step};
use super::two_sample::random_index;
use super::StampRng;
use crate::profile::Profile;

/// Measures of the diversity within one sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AlphaMetric {
    /// The number of features present.
    Observed,
    /// Shannon's index, with natural logarithms.
    Shannon,
    /// Simpson's index of diversity: the chance that two sequences drawn at
    /// random belong to different features.
    Simpson,
    /// The Chao1 estimate of the number of features in the community.
    Chao1,
}

impl AlphaMetric {
    pub const ALL: [AlphaMetric; 4] = [
        AlphaMetric::Observed,
        AlphaMetric::Shannon,
        AlphaMetric::Simpson,
        AlphaMetric::Chao1,
    ];

    pub fn name(self) -> &'static str {
        match self {
            AlphaMetric::Observed => "Observed features",
            AlphaMetric::Shannon => "Shannon index",
            AlphaMetric::Simpson => "Simpson index",
            AlphaMetric::Chao1 => "Chao1 estimate",
        }
    }

    /// True if the measure needs counts of sequences, not just proportions.
    pub fn needs_counts(self) -> bool {
        self == AlphaMetric::Chao1
    }

    /// The diversity of a sample from the abundance of each of its features
    /// (counts or proportions; only their ratios matter, except to Chao1,
    /// which needs whole counts and is `NaN` without them).
    pub fn value(self, abundances: &[f64]) -> f64 {
        let total: f64 = abundances.iter().filter(|a| **a > 0.0).sum();
        let proportions = || {
            abundances
                .iter()
                .filter(|a| **a > 0.0)
                .map(move |a| a / total)
        };
        match self {
            AlphaMetric::Observed => abundances.iter().filter(|a| **a > 0.0).count() as f64,
            AlphaMetric::Shannon => -proportions().map(|p| p * p.ln()).sum::<f64>(),
            AlphaMetric::Simpson => {
                if total > 0.0 {
                    1.0 - proportions().map(|p| p * p).sum::<f64>()
                } else {
                    0.0
                }
            }
            AlphaMetric::Chao1 => match whole_counts(abundances) {
                Some(counts) => chao1(&counts),
                None => f64::NAN,
            },
        }
    }
}

/// How samples are made comparable before their diversity is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Normalisation {
    /// Use every sequence, as proportions. Deeper samples show more rare
    /// features, which affects the measures that count features.
    Proportions,
    /// Draw the same number of sequences from every sample at random, as
    /// many as the smallest sample has. Needs counts of sequences.
    Rarefy,
}

impl Normalisation {
    pub const ALL: [Normalisation; 2] = [Normalisation::Proportions, Normalisation::Rarefy];

    pub fn name(self) -> &'static str {
        match self {
            Normalisation::Proportions => "Proportions",
            Normalisation::Rarefy => "Rarefy to the smallest sample",
        }
    }
}

/// Why a profile cannot be rarefied.
pub const NEEDS_COUNTS: &str = "Rarefying needs counts of sequences, and this profile holds \
                                fractional values. Use proportions, or a profile of raw counts.";

/// The abundance of every feature in each sample of a community, ready for
/// the diversity measures.
#[derive(Debug, Clone, PartialEq)]
pub struct CommunityTable {
    /// `abundances[sample][feature]`
    pub abundances: Vec<Vec<f64>>,
    /// The number of sequences every sample was rarefied to, if they were.
    pub depth: Option<u64>,
}

impl CommunityTable {
    /// The community table of a profile. The profile should hold every
    /// category at its level, unclassified ones included: those are not
    /// features of the community, being no taxon or function of their own,
    /// but their sequences are part of each sample, so they take part in
    /// rarefying.
    pub fn new(
        profile: &Profile,
        normalisation: Normalisation,
        rng: &mut StampRng,
    ) -> Result<CommunityTable, &'static str> {
        let samples = profile.sample_names.len();
        let counted: Vec<bool> = profile
            .features()
            .iter()
            .map(|f| !f.name.to_ascii_lowercase().contains("unclassified"))
            .collect();
        match normalisation {
            Normalisation::Proportions => Ok(CommunityTable {
                abundances: (0..samples)
                    .map(|s| {
                        profile
                            .features()
                            .iter()
                            .zip(&counted)
                            .filter(|(_, counted)| **counted)
                            .map(|(f, _)| f.counts[s].max(0.0))
                            .collect()
                    })
                    .collect(),
                depth: None,
            }),
            Normalisation::Rarefy => {
                let mut columns: Vec<Vec<u64>> = vec![Vec::new(); samples];
                for feature in profile.features() {
                    let counts = whole_counts(&feature.counts).ok_or(NEEDS_COUNTS)?;
                    for (column, count) in columns.iter_mut().zip(counts) {
                        column.push(count);
                    }
                }
                let depth = columns
                    .iter()
                    .map(|column| column.iter().sum::<u64>())
                    .min()
                    .unwrap_or(0);
                Ok(CommunityTable {
                    abundances: columns
                        .iter()
                        .map(|column| {
                            subsample(column, depth, rng)
                                .into_iter()
                                .zip(&counted)
                                .filter(|(_, counted)| **counted)
                                .map(|(count, _)| count as f64)
                                .collect()
                        })
                        .collect(),
                    depth: Some(depth),
                })
            }
        }
    }
}

/// Draw `depth` sequences at random, without replacement, from a sample
/// given as the number of sequences of each feature. Returns how many of
/// each feature were drawn. A depth of the whole sample or more returns it
/// unchanged.
pub fn subsample(counts: &[u64], depth: u64, rng: &mut StampRng) -> Vec<u64> {
    let total: u64 = counts.iter().sum();
    if depth >= total {
        return counts.to_vec();
    }
    // Go through the sequences in order, taking each with the chance that
    // fills the remaining places from the remaining sequences (selection
    // sampling). Whichever is smaller is chosen, the sequences to keep or
    // those to leave out.
    let keep = depth <= total - depth;
    let mut wanted = if keep { depth } else { total - depth };
    let mut remaining = total;
    let mut drawn = Vec::with_capacity(counts.len());
    for &count in counts {
        let mut chosen = 0;
        for _ in 0..count {
            if wanted == 0 {
                break;
            }
            if (random_index(rng, remaining as usize) as u64) < wanted {
                chosen += 1;
                wanted -= 1;
            }
            remaining -= 1;
        }
        drawn.push(if keep { chosen } else { count - chosen });
    }
    drawn
}

/// Measures of how different two samples are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Distance {
    /// Bray-Curtis dissimilarity: the share of the abundances that the two
    /// samples do not have in common.
    BrayCurtis,
    /// Jaccard distance: the share of the features found in either sample
    /// that are found in only one. Abundances are ignored.
    Jaccard,
    /// Aitchison distance: Euclidean distance after the centred log-ratio
    /// transform, which respects that proportions are compositional.
    Aitchison,
    /// Euclidean distance between the proportions of the two samples. A
    /// principal coordinates analysis of it places samples as a principal
    /// component analysis of their proportions would.
    Euclidean,
}

impl Distance {
    pub const ALL: [Distance; 4] = [
        Distance::BrayCurtis,
        Distance::Jaccard,
        Distance::Aitchison,
        Distance::Euclidean,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Distance::BrayCurtis => "Bray-Curtis",
            Distance::Jaccard => "Jaccard",
            Distance::Aitchison => "Aitchison",
            Distance::Euclidean => "Euclidean",
        }
    }
}

/// A sample as proportions of its total; all zeros if it has none.
fn as_proportions(sample: &[f64]) -> Vec<f64> {
    let total: f64 = sample.iter().filter(|a| **a > 0.0).sum();
    sample
        .iter()
        .map(|&a| if total > 0.0 { a.max(0.0) / total } else { 0.0 })
        .collect()
}

/// Distances between every pair of a set of samples.
#[derive(Debug, Clone, PartialEq)]
pub struct DistanceMatrix {
    size: usize,
    /// Row by row, the full square.
    values: Vec<f64>,
}

impl DistanceMatrix {
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.values[i * self.size + j]
    }

    /// A matrix from a function giving the distance between two items.
    pub fn from_fn(size: usize, distance: impl Fn(usize, usize) -> f64) -> DistanceMatrix {
        let mut values = vec![0.0; size * size];
        for i in 0..size {
            for j in i + 1..size {
                let d = distance(i, j);
                values[i * size + j] = d;
                values[j * size + i] = d;
            }
        }
        DistanceMatrix { size, values }
    }
}

/// The centred log-ratio transform of a sample's abundances. Zeros have no
/// logarithm, so they are first replaced by a small proportion, taken
/// evenly from the features that are present (multiplicative replacement,
/// with the proportion scikit-bio uses: one over the squared number of
/// features).
pub fn centred_log_ratio(abundances: &[f64]) -> Vec<f64> {
    let total: f64 = abundances.iter().filter(|a| **a > 0.0).sum();
    let features = abundances.len() as f64;
    if total <= 0.0 || abundances.is_empty() {
        return vec![0.0; abundances.len()];
    }
    let small = 1.0 / (features * features);
    let zeros = abundances.iter().filter(|a| **a <= 0.0).count() as f64;
    let logs: Vec<f64> = abundances
        .iter()
        .map(|&a| {
            if a > 0.0 {
                (a / total * (1.0 - zeros * small)).ln()
            } else {
                small.ln()
            }
        })
        .collect();
    let mean = logs.iter().sum::<f64>() / features;
    logs.into_iter().map(|l| l - mean).collect()
}

/// Distances between samples, each given as the abundance of every feature
/// (in the same order). Bray-Curtis is computed on proportions, so that it
/// does not depend on how deeply each sample was sequenced.
pub fn distances(samples: &[Vec<f64>], metric: Distance) -> DistanceMatrix {
    match DistanceRun::new(samples, metric).advance(&mut |_, _| Control::Continue) {
        Step::Done(matrix) => matrix,
        Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
    }
}

/// [`distances`], computed a row of the matrix at a time, so that whatever
/// runs it can show its progress, attend to other things in between, or
/// give up. The matrix does not depend on where it pauses.
#[derive(Debug, Clone)]
pub struct DistanceRun {
    metric: Distance,
    /// Each sample as the distance uses it: proportions, presence or
    /// centred log ratios.
    prepared: Vec<Vec<f64>>,
    /// The features present in each sample (Bray-Curtis and Jaccard).
    present: Vec<Vec<usize>>,
    values: Vec<f64>,
    /// The next row to compute.
    row: usize,
}

impl DistanceRun {
    pub fn new(samples: &[Vec<f64>], metric: Distance) -> DistanceRun {
        let prepared: Vec<Vec<f64>> = samples
            .iter()
            .map(|sample| match metric {
                Distance::BrayCurtis | Distance::Euclidean => as_proportions(sample),
                Distance::Jaccard => sample
                    .iter()
                    .map(|&a| if a > 0.0 { 1.0 } else { 0.0 })
                    .collect(),
                Distance::Aitchison => centred_log_ratio(sample),
            })
            .collect();
        // Profiles are mostly zeros, and a feature neither sample has adds
        // nothing to either sum of Bray-Curtis or Jaccard, so only the
        // features present in one or the other are visited, in the same
        // order as all of them would be: the result is exactly the same.
        let present = match metric {
            Distance::BrayCurtis | Distance::Jaccard => prepared
                .iter()
                .map(|sample| (0..sample.len()).filter(|&f| sample[f] != 0.0).collect())
                .collect(),
            Distance::Aitchison | Distance::Euclidean => Vec::new(),
        };
        let n = samples.len();
        DistanceRun {
            metric,
            prepared,
            present,
            values: vec![0.0; n * n],
            row: 0,
        }
    }

    /// The number of rows, each a piece of work.
    pub fn rows(&self) -> usize {
        self.prepared.len()
    }

    /// Compute further rows; `pace` is asked before each, with the number
    /// of rows done and the number of rows.
    pub fn advance(&mut self, pace: Pace<'_>) -> Step<DistanceMatrix> {
        let n = self.prepared.len();
        while self.row < n {
            match pace(self.row, n) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            let i = self.row;
            for j in i + 1..n {
                let d = self.distance(i, j);
                self.values[i * n + j] = d;
                self.values[j * n + i] = d;
            }
            self.row += 1;
        }
        Step::Done(DistanceMatrix {
            size: n,
            values: std::mem::take(&mut self.values),
        })
    }

    fn distance(&self, i: usize, j: usize) -> f64 {
        let (a, b) = (&self.prepared[i], &self.prepared[j]);
        let metric = self.metric;
        if matches!(metric, Distance::Aitchison | Distance::Euclidean) {
            return squared_difference(a, b).sqrt();
        }
        let (fa, fb) = (&self.present[i], &self.present[j]);
        let (mut p, mut q) = (0, 0);
        let (mut first, mut second) = (0.0, 0.0);
        while p < fa.len() || q < fb.len() {
            let x = fa.get(p).copied().unwrap_or(usize::MAX);
            let y = fb.get(q).copied().unwrap_or(usize::MAX);
            let f = x.min(y);
            let (u, v) = (a[f], b[f]);
            if metric == Distance::BrayCurtis {
                first += (u - v).abs();
                // In this order, as rounding depends on it.
                second = second + u + v;
            } else {
                first += f64::max(u, v);
                second += u * v;
            }
            p += usize::from(x == f);
            q += usize::from(y == f);
        }
        match metric {
            // first: difference, second: sum
            Distance::BrayCurtis if second > 0.0 => first / second,
            // first: in either, second: in both
            Distance::Jaccard if first > 0.0 => 1.0 - second / first,
            _ => 0.0,
        }
    }
}

/// The sum of squared differences of two vectors, in four running sums,
/// which lets the compiler use vector instructions.
fn squared_difference(a: &[f64], b: &[f64]) -> f64 {
    let mut sums = [0.0; 4];
    let ((a4, a_rest), (b4, b_rest)) = (a.as_chunks::<4>(), b.as_chunks::<4>());
    for (x, y) in a4.iter().zip(b4) {
        for lane in 0..4 {
            let d = x[lane] - y[lane];
            sums[lane] += d * d;
        }
    }
    let mut total = (sums[0] + sums[1]) + (sums[2] + sums[3]);
    for (x, y) in a_rest.iter().zip(b_rest) {
        total += (x - y) * (x - y);
    }
    total
}

/// Samples placed in space so that the distances between them are kept as
/// well as possible.
#[derive(Debug, Clone, PartialEq)]
pub struct Ordination {
    /// Coordinates of each sample: `positions[sample][axis]`, on every axis
    /// or, from [`pcoa_leading`], the first few.
    pub positions: Vec<Vec<f64>>,
    /// Share of the variation each axis accounts for, largest first.
    pub variance_fractions: Vec<f64>,
}

/// Principal coordinates analysis (classical multidimensional scaling) of a
/// distance matrix. Axes are those with positive eigenvalues; a distance
/// that is not Euclidean, such as Bray-Curtis, can also give negative ones,
/// which are left out, as scikit-bio leaves them out.
///
/// Returns `None` for fewer than two samples or if all distances are zero.
pub fn pcoa(distances: &DistanceMatrix) -> Option<Ordination> {
    pcoa_leading(distances, usize::MAX)
}

/// [`pcoa`] with the coordinates of only the first `axes` axes (the shares
/// of the variation of all of them are still given), which is much faster
/// for many samples: a plot needs two or three.
pub fn pcoa_leading(distances: &DistanceMatrix, axes: usize) -> Option<Ordination> {
    let n = distances.size();
    if n < 2 {
        return None;
    }
    // Gower's centring of -d^2 / 2: subtract row and column means, add the
    // overall mean.
    let mut centred: Vec<f64> = distances.values.iter().map(|d| -0.5 * d * d).collect();
    super::pca::double_centre(&mut centred, n);
    let (positions, variance_fractions) = super::pca::coordinates(&centred, n, axes)?;
    Some(Ordination {
        positions,
        variance_fractions,
    })
}

/// A non-metric multidimensional scaling of samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Nmds {
    /// Coordinates of each sample: `positions[sample][axis]`.
    pub positions: Vec<Vec<f64>>,
    /// Kruskal's stress-1 of the layout: how far the order of the distances
    /// in the layout departs from the order of the distances given, from 0
    /// (not at all) upwards. Below 0.1 is usually taken as a good layout,
    /// above 0.2 as a poor one.
    pub stress: f64,
}

/// Most iterations of the NMDS layout.
const NMDS_ITERATIONS: usize = 500;

/// Non-metric multidimensional scaling (Kruskal 1964) in `dimensions`
/// dimensions: a layout of the samples whose distances follow the order of
/// the given distances as closely as possible, whatever their size. It
/// starts from the PCoA layout, so the result is the same every time, and
/// improves it by the SMACOF method (de Leeuw 1977) with monotone
/// regression; equal distances are kept equal (Kruskal's secondary
/// approach to ties). The axes are rotated to the principal axes of the
/// layout, the first spreading the samples most.
///
/// `None` for fewer than three samples or if all distances are zero.
pub fn nmds(distances: &DistanceMatrix, dimensions: usize) -> Option<Nmds> {
    match NmdsRun::new(dimensions).advance(distances, &mut |_, _| Control::Continue) {
        Step::Done(result) => result,
        Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
    }
}

/// [`nmds`], carried out an iteration at a time, so that whatever runs it
/// can show its progress, attend to other things in between, or give up.
/// The layout does not depend on where it pauses.
#[derive(Debug, Clone)]
pub struct NmdsRun {
    dimensions: usize,
    /// Everything kept between iterations, from the first piece of work.
    state: Option<NmdsState>,
}

#[derive(Debug, Clone)]
struct NmdsState {
    n: usize,
    /// Where each pair's samples start in the layout, in the order of
    /// their given distance, in four bytes each, as the pairs are visited
    /// many times.
    pairs: Vec<[u32; 2]>,
    /// Blocks of pairs of equal given distance, each given by where it
    /// ends.
    block_ends: Vec<usize>,
    /// The layout, one sample after another.
    layout: Vec<f64>,
    /// The layout distances in pair order.
    d: Vec<f64>,
    /// The pools of the monotone regression, kept between iterations to
    /// save allocating them.
    pools: Vec<Pool>,
    moved: Vec<f64>,
    weights: Vec<f64>,
    target: f64,
    previous: f64,
    iteration: usize,
}

impl NmdsRun {
    pub fn new(dimensions: usize) -> NmdsRun {
        NmdsRun {
            dimensions,
            state: None,
        }
    }

    /// The most iterations a layout takes, each a piece of work (it
    /// usually stops sooner, once the stress no longer falls).
    pub fn iterations() -> usize {
        NMDS_ITERATIONS
    }

    /// Carry the layout further, from `distances`, which must be the same
    /// on every call. `pace` is asked before each iteration, with the
    /// number of iterations done and the most there can be; before the
    /// first, the starting layout is found by PCoA in one piece. The
    /// result is `None` for fewer than three samples or if all distances
    /// are zero.
    pub fn advance(&mut self, distances: &DistanceMatrix, pace: Pace<'_>) -> Step<Option<Nmds>> {
        let dims = self.dimensions;
        if self.state.is_none() {
            match pace(0, NMDS_ITERATIONS) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            match nmds_start(distances, dims) {
                Some(state) => self.state = Some(state),
                None => return Step::Done(None),
            }
        }
        let Some(state) = self.state.as_mut() else {
            return Step::Done(None);
        };
        let NmdsState {
            n,
            pairs,
            block_ends,
            layout,
            d,
            pools,
            moved,
            weights,
            target,
            previous,
            iteration,
        } = state;
        let n = *n;
        loop {
            match pace(*iteration, NMDS_ITERATIONS) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            // Distances in the layout, and the sum of their squares; the
            // passes over the pairs are compiled separately for two
            // dimensions, the usual case, to save the loops over the axes.
            let total = if dims == 2 {
                layout_distances(layout, pairs, d, 2)
            } else {
                layout_distances(layout, pairs, d, dims)
            };
            let pool_count = monotone_fit(d, block_ends, pools);
            let pools = &pools[..pool_count];
            // Kruskal's stress against the fit (the disparities), the sum
            // of squares of the disparities and, for the Guttman transform
            // below, X <- B X / n with B[i][j] = -f/d for i != j (zero
            // where d is zero) and each row of B summing to zero: row i of
            // B X is the sum over j of (f/d)(X_i - X_j). Taken pair by
            // pair, without forming B, and before the disparities are
            // scaled.
            let (raw, sum_sq) = if dims == 2 {
                guttman_sums(layout, pairs, d, pools, moved, weights, 2)
            } else {
                guttman_sums(layout, pairs, d, pools, moved, weights, dims)
            };
            let current = if total > 0.0 {
                (raw / total).sqrt()
            } else {
                0.0
            };
            let finished = *iteration == NMDS_ITERATIONS
                || current < 1e-12
                || *previous - current < 1e-9 * previous.min(1.0);
            // Disparities are scaled to a fixed total below, so the layout
            // cannot shrink to nothing; that needs some.
            if finished || sum_sq <= 0.0 {
                let positions = layout.chunks_exact(dims).map(<[f64]>::to_vec).collect();
                pace(NMDS_ITERATIONS, NMDS_ITERATIONS);
                return Step::Done(Some(Nmds {
                    positions: principal_axes(positions, dims),
                    stress: current,
                }));
            }
            *iteration += 1;
            *previous = current;
            let scale = (*target / sum_sq).sqrt() / n as f64;
            for ((point, shift), weight) in layout
                .chunks_exact_mut(dims)
                .zip(moved.chunks_exact(dims))
                .zip(weights.iter())
            {
                for (x, m) in point.iter_mut().zip(shift) {
                    *x = scale * (weight * *x + m);
                }
            }
        }
    }
}

/// The starting point of an NMDS layout: the PCoA layout, and the pairs of
/// samples in order of their given distance.
fn nmds_start(distances: &DistanceMatrix, dimensions: usize) -> Option<NmdsState> {
    let n = distances.size();
    if n < 3 || dimensions == 0 {
        return None;
    }
    let start = pcoa_leading(distances, dimensions)?;
    let positions: Vec<Vec<f64>> = start
        .positions
        .iter()
        .enumerate()
        .map(|(i, row)| {
            (0..dimensions)
                .map(|k| match row.get(k) {
                    Some(value) => *value,
                    // A deterministic nudge where the PCoA has no axis.
                    None => 1e-3 * (((i * 7919 + k * 104_729) % 97) as f64 / 97.0 - 0.5),
                })
                .collect()
        })
        .collect();

    // Pairs of samples in order of their given distance, in blocks of
    // equal distances (each block given by where it ends).
    let mut pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .collect();
    pairs.sort_by(|a, b| distances.get(a.0, a.1).total_cmp(&distances.get(b.0, b.1)));
    let mut block_ends: Vec<usize> = Vec::new();
    for k in 1..=pairs.len() {
        let same = k < pairs.len()
            && distances.get(pairs[k].0, pairs[k].1)
                == distances.get(pairs[k - 1].0, pairs[k - 1].1);
        if !same {
            block_ends.push(k);
        }
    }
    let pairs: Vec<[u32; 2]> = pairs
        .iter()
        .map(|&(i, j)| [(i * dimensions) as u32, (j * dimensions) as u32])
        .collect();
    let pair_count = pairs.len();
    Some(NmdsState {
        n,
        pairs,
        pools: vec![
            Pool {
                sum: 0.0,
                weight: 0.0,
                end: 0,
            };
            block_ends.len()
        ],
        block_ends,
        layout: positions.iter().flatten().copied().collect(),
        d: vec![0.0; pair_count],
        moved: vec![0.0; n * dimensions],
        weights: vec![0.0; n],
        target: (n * (n - 1) / 2) as f64,
        previous: f64::INFINITY,
        iteration: 0,
    })
}

/// The distances between the pairs of samples of a layout (`dims`
/// coordinates per sample; `pairs` holds where each pair's samples start),
/// into `d`, returning the sum of their squares.
#[inline(always)]
fn layout_distances(layout: &[f64], pairs: &[[u32; 2]], d: &mut [f64], dims: usize) -> f64 {
    let mut total = 0.0;
    for (out, &[i, j]) in d.iter_mut().zip(pairs) {
        let (a, b) = (&layout[i as usize..][..dims], &layout[j as usize..][..dims]);
        let mut squares = 0.0;
        for c in 0..dims {
            squares += (a[c] - b[c]) * (a[c] - b[c]);
        }
        let distance = f64::sqrt(squares);
        *out = distance;
        total += distance * distance;
    }
    total
}

/// Kruskal's raw stress of layout distances `d` against their monotone fit
/// (the disparities, from `pools`) and the sum of squares of the
/// disparities; and, for the Guttman transform X <- B X / n, with B[i][j] =
/// -f/d for i != j (zero where d is zero) and each row of B summing to
/// zero, so that row i of B X is the sum over j of (f/d)(X_i - X_j): the
/// sums of f/d for each sample (`weights`) and of -(f/d) X_j (`moved`).
/// Taken pair by pair, without forming B, and before the disparities are
/// scaled.
#[inline(always)]
fn guttman_sums(
    layout: &[f64],
    pairs: &[[u32; 2]],
    d: &[f64],
    pools: &[Pool],
    moved: &mut [f64],
    weights: &mut [f64],
    dims: usize,
) -> (f64, f64) {
    let (mut raw, mut sum_sq) = (0.0, 0.0);
    moved.fill(0.0);
    weights.fill(0.0);
    let mut begin = 0;
    for pool in pools {
        let f = pool.mean();
        for (&dk, &[i, j]) in d[begin..pool.end].iter().zip(&pairs[begin..pool.end]) {
            raw += (dk - f) * (dk - f);
            sum_sq += f * f;
            if dk > 0.0 {
                let w = f / dk;
                let (i, j) = (i as usize, j as usize);
                weights[i / dims] += w;
                weights[j / dims] += w;
                for c in 0..dims {
                    moved[i + c] -= w * layout[j + c];
                    moved[j + c] -= w * layout[i + c];
                }
            }
        }
        begin = pool.end;
    }
    (raw, sum_sq)
}

/// A pool of the monotone regression: blocks of pairs fitted by one value,
/// their mean.
#[derive(Debug, Clone, Copy)]
struct Pool {
    /// The sum of the layout distances of its pairs.
    sum: f64,
    /// The number of its pairs.
    weight: f64,
    /// Where its last block ends.
    end: usize,
}

impl Pool {
    fn mean(&self) -> f64 {
        self.sum / self.weight
    }
}

/// The monotone (non-decreasing) fit to the layout distances `d`, in pair
/// order, as pools of adjacent blocks of equal given distances, by pooling
/// adjacent violators. Writes the pools to the start of `pools` (as long
/// as there are blocks) and returns how many there are.
///
/// Written for speed, as it runs on every pair at every iteration: pools
/// are kept as sums and compared without dividing, the last pool is kept
/// apart in registers, and whether a block starts a new pool or joins the
/// last is worked out by arithmetic rather than a branch, which the
/// processor could not predict.
fn monotone_fit(d: &[f64], block_ends: &[usize], pools: &mut [Pool]) -> usize {
    debug_assert!(pools.len() >= block_ends.len());
    let Some(&first_end) = block_ends.first() else {
        return 0;
    };
    let block = |begin: usize, end: usize| -> (f64, f64) {
        if end - begin == 1 {
            (d[begin], 1.0)
        } else {
            (d[begin..end].iter().sum::<f64>(), (end - begin) as f64)
        }
    };
    // The last pool, and how many come before it.
    let (mut sum, mut weight) = block(0, first_end);
    let mut end = first_end;
    let mut before = 0;
    for &next_end in &block_ends[1..] {
        let (block_sum, block_weight) = block(end, next_end);
        // In order (the last pool's mean at most the block's): the block
        // starts a new pool; otherwise it joins the last one.
        let in_order = sum * block_weight <= block_sum * weight;
        pools[before] = Pool { sum, weight, end };
        before += usize::from(in_order);
        let joins = f64::from(u8::from(!in_order));
        sum = block_sum + joins * sum;
        weight = block_weight + joins * weight;
        end = next_end;
        // The last pool may now have to join those before it.
        while before > 0 {
            let previous = &pools[before - 1];
            if previous.sum * weight <= sum * previous.weight {
                break;
            }
            sum += previous.sum;
            weight += previous.weight;
            before -= 1;
        }
    }
    pools[before] = Pool { sum, weight, end };
    before + 1
}

/// A layout centred and rotated to its principal axes, largest spread
/// first, each with its largest coordinate positive.
fn principal_axes(positions: Vec<Vec<f64>>, dimensions: usize) -> Vec<Vec<f64>> {
    let n = positions.len() as f64;
    let means: Vec<f64> = (0..dimensions)
        .map(|k| positions.iter().map(|p| p[k]).sum::<f64>() / n)
        .collect();
    let centred: Vec<Vec<f64>> = positions
        .iter()
        .map(|p| p.iter().zip(&means).map(|(v, m)| v - m).collect())
        .collect();
    let covariance: Vec<Vec<f64>> = (0..dimensions)
        .map(|a| {
            (0..dimensions)
                .map(|b| centred.iter().map(|p| p[a] * p[b]).sum::<f64>())
                .collect()
        })
        .collect();
    let (values, vectors) = symmetric_eigen(covariance);
    let mut order: Vec<usize> = (0..dimensions).collect();
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let mut rotated = vec![Vec::with_capacity(dimensions); centred.len()];
    for &k in &order {
        let column: Vec<f64> = centred
            .iter()
            .map(|p| (0..dimensions).map(|a| p[a] * vectors[a][k]).sum())
            .collect();
        let extreme = column
            .iter()
            .copied()
            .fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        let sign = if extreme < 0.0 { -1.0 } else { 1.0 };
        for (row, value) in rotated.iter_mut().zip(column) {
            row.push(sign * value);
        }
    }
    rotated
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Seven samples of five features; reference values below are from
    /// scikit-bio 0.7.4 on the same table.
    pub(crate) fn table() -> Vec<Vec<f64>> {
        vec![
            vec![10.0, 20.0, 0.0, 5.0, 1.0],
            vec![12.0, 18.0, 2.0, 4.0, 0.0],
            vec![9.0, 25.0, 1.0, 6.0, 2.0],
            vec![30.0, 5.0, 10.0, 0.0, 3.0],
            vec![28.0, 7.0, 12.0, 1.0, 1.0],
            vec![35.0, 4.0, 9.0, 0.0, 6.0],
            vec![20.0, 10.0, 5.0, 3.0, 2.0],
        ]
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-10 * expected.abs().max(1.0),
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn alpha_diversity_matches_scikit_bio() {
        let expected = [
            (AlphaMetric::Observed, [4.0, 4.0, 5.0, 4.0, 5.0, 4.0, 5.0]),
            (
                AlphaMetric::Shannon,
                [
                    1.0560832172563661,
                    1.1174899594231538,
                    1.1476241158279994,
                    1.0294343690749754,
                    1.101167703707588,
                    1.0166148316012495,
                    1.2971340243560612,
                ],
            ),
            (
                AlphaMetric::Simpson,
                [
                    0.5941358024691357,
                    0.6234567901234568,
                    0.5959978366684694,
                    0.5512152777777778,
                    0.592253227821741,
                    0.5342935528120714,
                    0.6637500000000001,
                ],
            ),
            (AlphaMetric::Chao1, [4.0, 4.0, 5.0, 4.0, 6.0, 4.0, 5.0]),
        ];
        for (metric, values) in expected {
            for (sample, value) in table().iter().zip(values) {
                close(metric.value(sample), value);
            }
        }
        // Proportions give the same answers, except to Chao1.
        let proportions: Vec<f64> = table()[0].iter().map(|c| c / 36.0).collect();
        close(AlphaMetric::Shannon.value(&proportions), 1.0560832172563661);
        assert!(AlphaMetric::Chao1.value(&proportions).is_nan());
        assert!(AlphaMetric::Chao1.needs_counts() && !AlphaMetric::Shannon.needs_counts());
        assert_eq!(AlphaMetric::Simpson.value(&[0.0, 0.0]), 0.0);
        assert_eq!(AlphaMetric::Shannon.value(&[7.0]), 0.0);
    }

    #[test]
    fn distances_match_scikit_bio() {
        let expected = [
            (
                Distance::BrayCurtis,
                [
                    0.0,
                    0.1111111111111111,
                    0.06847545219638243,
                    0.5902777777777779,
                    0.5385487528344672,
                    0.6203703703703703,
                    0.3694444444444444,
                ],
            ),
            (Distance::Jaccard, [0.0, 0.4, 0.2, 0.4, 0.2, 0.4, 0.2]),
            (
                Distance::Aitchison,
                [
                    0.0,
                    0.5305317063085867,
                    0.8184689169104635,
                    2.851715715241645,
                    3.0060182938056013,
                    3.1684003133665875,
                    1.6635774589938441,
                ],
            ),
            // SciPy 1.17 `scipy.spatial.distance.pdist(p, "euclidean")` of
            // the proportions p, which scikit-bio 0.7.4's
            // `beta_diversity("euclidean", p)` matches.
            (
                Distance::Euclidean,
                [
                    0.0,
                    0.10393492741038726,
                    0.07904928717555415,
                    0.6230680015599431,
                    0.5749937151223583,
                    0.650392938105035,
                    0.4036675994728801,
                ],
            ),
        ];
        for (metric, row) in expected {
            let matrix = distances(&table(), metric);
            assert_eq!(matrix.size(), 7);
            for (j, value) in row.into_iter().enumerate() {
                close(matrix.get(0, j), value);
                close(matrix.get(j, 0), value);
            }
        }
        // Samples with nothing in them are no distance from each other.
        let empty = distances(&[vec![0.0, 0.0], vec![0.0, 0.0]], Distance::Jaccard);
        assert_eq!(empty.get(0, 1), 0.0);
        // A centred log-ratio sums to zero.
        let clr = centred_log_ratio(&[4.0, 0.0, 1.0]);
        assert!(clr.iter().sum::<f64>().abs() < 1e-12);
    }

    /// A PCoA of Euclidean distances between proportions is a PCA of the
    /// proportions: the same share of variation on each axis and the same
    /// positions, up to the sign of each axis.
    #[test]
    fn euclidean_pcoa_is_a_pca_of_the_proportions() {
        let proportions: Vec<Vec<f64>> = table().iter().map(|s| as_proportions(s)).collect();
        let pca = crate::stats::pca::pca(&proportions).unwrap();
        let pcoa = pcoa(&distances(&table(), Distance::Euclidean)).unwrap();
        for axis in 0..3 {
            assert!(
                (pca.variance_fractions[axis] - pcoa.variance_fractions[axis]).abs() < 1e-9,
                "axis {axis}"
            );
            let sign = (pca.positions[0][axis] * pcoa.positions[0][axis]).signum();
            for (a, b) in pca.positions.iter().zip(&pcoa.positions) {
                assert!((a[axis] - sign * b[axis]).abs() < 1e-9, "axis {axis}");
            }
        }
    }

    #[test]
    fn pcoa_matches_scikit_bio() {
        let ordination = pcoa(&distances(&table(), Distance::BrayCurtis)).unwrap();
        let fractions = [
            0.9577343245481587,
            0.033423021927304904,
            0.006245088344442225,
            0.0025975651800942295,
        ];
        for (actual, expected) in ordination.variance_fractions.iter().zip(fractions) {
            assert!(
                (actual - expected).abs() < 1e-9,
                "{actual} against {expected}"
            );
        }
        // Axes are defined up to their sign.
        let first = [
            0.3092992420730119,
            0.015047888722263964,
            0.009533545339388742,
        ];
        let fourth = [
            0.2789370273243144,
            0.014938517401571845,
            0.030202966806821968,
        ];
        for axis in 0..3 {
            assert!((ordination.positions[0][axis].abs() - first[axis]).abs() < 1e-9);
            assert!((ordination.positions[3][axis].abs() - fourth[axis]).abs() < 1e-9);
        }
        // Euclidean distances are reproduced exactly by all the axes.
        let aitchison = distances(&table(), Distance::Aitchison);
        let full = pcoa(&aitchison).unwrap();
        let rebuilt = |i: usize, j: usize| {
            full.positions[i]
                .iter()
                .zip(&full.positions[j])
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt()
        };
        assert!((rebuilt(0, 5) - aitchison.get(0, 5)).abs() < 1e-9);
        assert!((full.variance_fractions.iter().sum::<f64>() - 1.0).abs() < 1e-12);

        assert!(pcoa(&DistanceMatrix::from_fn(1, |_, _| 0.0)).is_none());
        assert!(pcoa(&DistanceMatrix::from_fn(3, |_, _| 0.0)).is_none());
    }

    #[test]
    fn subsampling_draws_the_depth_asked_for() {
        use crate::stats::rng_from_seed;
        let counts = [500, 300, 150, 40, 9, 1, 0];
        let mut rng = rng_from_seed(4);
        for depth in [0, 1, 10, 499, 500, 501, 990, 1000, 5000] {
            let drawn = subsample(&counts, depth, &mut rng);
            assert_eq!(drawn.iter().sum::<u64>(), depth.min(1000));
            assert!(drawn.iter().zip(&counts).all(|(d, c)| d <= c));
        }
        // Repeatable for a seed, and on average in proportion.
        let once = subsample(&counts, 100, &mut rng_from_seed(9));
        assert_eq!(once, subsample(&counts, 100, &mut rng_from_seed(9)));
        let mut rng = rng_from_seed(2);
        let mean_of_first: f64 = (0..400)
            .map(|_| subsample(&counts, 100, &mut rng)[0] as f64)
            .sum::<f64>()
            / 400.0;
        assert!((mean_of_first - 50.0).abs() < 1.0, "{mean_of_first}");
    }

    #[test]
    fn community_tables_leave_unclassified_out_and_can_be_rarefied() {
        use crate::stats::rng_from_seed;
        use crate::{ProfileTree, Unclassified};
        let spf = "Phylum\tGenus\tS1\tS2\n\
            P1\ta\t50\t5\n\
            P1\tb\t30\t5\n\
            P1\tunclassified\t40\t0\n\
            P2\tc\t20\t10\n";
        let tree = ProfileTree::read(spf).unwrap();
        let profile = tree
            .profile(&["S1", "S2"], None, "Genus", Unclassified::Retain)
            .unwrap();
        let mut rng = rng_from_seed(1);
        let table = CommunityTable::new(&profile, Normalisation::Proportions, &mut rng).unwrap();
        assert_eq!(table.abundances, [[50.0, 30.0, 20.0], [5.0, 5.0, 10.0]]);
        assert_eq!(table.depth, None);

        // Rarefied to the 20 sequences of the smaller sample; the larger
        // sample's draw includes unclassified sequences that are then left
        // out, so its three genera hold 20 or fewer.
        let rarefied = CommunityTable::new(&profile, Normalisation::Rarefy, &mut rng).unwrap();
        assert_eq!(rarefied.depth, Some(20));
        assert_eq!(rarefied.abundances[1], [5.0, 5.0, 10.0]);
        let first: f64 = rarefied.abundances[0].iter().sum();
        assert!((5.0..=20.0).contains(&first), "{first}");

        let fractional = ProfileTree::read("F\tS1\tS2\nx\t1.5\t2\ny\t3\t4\n").unwrap();
        let profile = fractional
            .profile(&["S1", "S2"], None, "F", Unclassified::Retain)
            .unwrap();
        assert!(CommunityTable::new(&profile, Normalisation::Proportions, &mut rng).is_ok());
        assert_eq!(
            CommunityTable::new(&profile, Normalisation::Rarefy, &mut rng),
            Err(NEEDS_COUNTS)
        );
    }

    fn euclidean(points: &[[f64; 2]]) -> DistanceMatrix {
        DistanceMatrix::from_fn(points.len(), |i, j| {
            let (a, b) = (points[i], points[j]);
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
        })
    }

    const POINTS: [[f64; 2]; 8] = [
        [0.0, 0.0],
        [3.0, 1.0],
        [1.0, 4.0],
        [5.0, 5.0],
        [2.0, 2.5],
        [6.0, 0.5],
        [4.0, 3.0],
        [0.5, 6.0],
    ];

    use crate::stats::pca::symmetric_eigen;

    fn pcoa_reference(distances: &DistanceMatrix) -> Option<Ordination> {
        let n = distances.size();
        if n < 2 {
            return None;
        }
        // Gower's centring of -d^2 / 2: subtract row and column means, add the
        // overall mean.
        let squared = |i: usize, j: usize| -0.5 * distances.get(i, j) * distances.get(i, j);
        let row_means: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| squared(i, j)).sum::<f64>() / n as f64)
            .collect();
        let mean = row_means.iter().sum::<f64>() / n as f64;
        let centred: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| squared(i, j) - row_means[i] - row_means[j] + mean)
                    .collect()
            })
            .collect();
        let (values, vectors) = symmetric_eigen(centred);

        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
        let total: f64 = values.iter().map(|v| v.max(0.0)).sum();
        if total <= 0.0 {
            return None;
        }
        let tolerance = total * 1e-12;
        let kept: Vec<usize> = order
            .into_iter()
            .filter(|&k| values[k] > tolerance)
            .collect();
        let mut positions = vec![Vec::with_capacity(kept.len()); n];
        let mut variance_fractions = Vec::with_capacity(kept.len());
        for &k in &kept {
            let scale = values[k].sqrt();
            let column: Vec<f64> = (0..n).map(|i| vectors[i][k] * scale).collect();
            // The sign of an axis is arbitrary; fix it so results are stable.
            let extreme =
                column
                    .iter()
                    .copied()
                    .fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
            let sign = if extreme < 0.0 { -1.0 } else { 1.0 };
            for (row, value) in positions.iter_mut().zip(column) {
                row.push(sign * value);
            }
            variance_fractions.push(values[k] / total);
        }
        Some(Ordination {
            positions,
            variance_fractions,
        })
    }

    fn nmds_reference(distances: &DistanceMatrix, dimensions: usize) -> Option<Nmds> {
        let n = distances.size();
        if n < 3 || dimensions == 0 {
            return None;
        }
        let start = pcoa_reference(distances)?;
        let mut positions: Vec<Vec<f64>> = start
            .positions
            .iter()
            .enumerate()
            .map(|(i, row)| {
                (0..dimensions)
                    .map(|k| match row.get(k) {
                        Some(value) => *value,
                        // A deterministic nudge where the PCoA has no axis.
                        None => 1e-3 * (((i * 7919 + k * 104_729) % 97) as f64 / 97.0 - 0.5),
                    })
                    .collect()
            })
            .collect();

        // Pairs of samples in order of their given distance, in blocks of
        // equal distances.
        let mut pairs: Vec<(usize, usize)> = (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .collect();
        pairs.sort_by(|a, b| distances.get(a.0, a.1).total_cmp(&distances.get(b.0, b.1)));
        let mut blocks: Vec<std::ops::Range<usize>> = Vec::new();
        let mut begin = 0;
        for k in 1..=pairs.len() {
            let same = k < pairs.len()
                && distances.get(pairs[k].0, pairs[k].1)
                    == distances.get(pairs[begin].0, pairs[begin].1);
            if !same {
                blocks.push(begin..k);
                begin = k;
            }
        }

        let layout_distances = |positions: &[Vec<f64>]| -> Vec<f64> {
            pairs
                .iter()
                .map(|&(i, j)| {
                    positions[i]
                        .iter()
                        .zip(&positions[j])
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        .sqrt()
                })
                .collect()
        };
        // The monotone (non-decreasing) fit to layout distances, in pair order.
        let disparities = |d: &[f64]| -> Vec<f64> {
            // Pool adjacent violators over the blocks, each weighted by its size.
            let mut pools: Vec<(f64, f64, usize)> = Vec::new(); // (mean, weight, blocks)
            for block in &blocks {
                let weight = block.len() as f64;
                let mut mean = d[block.clone()].iter().sum::<f64>() / weight;
                let (mut total, mut count) = (weight, 1);
                while let Some(&(last, last_weight, last_count)) = pools.last() {
                    if last <= mean {
                        break;
                    }
                    mean = (mean * total + last * last_weight) / (total + last_weight);
                    total += last_weight;
                    count += last_count;
                    pools.pop();
                }
                pools.push((mean, total, count));
            }
            let mut fitted = Vec::with_capacity(d.len());
            let mut block = 0;
            for (mean, _, count) in pools {
                for _ in 0..count {
                    fitted.extend(std::iter::repeat_n(mean, blocks[block].len()));
                    block += 1;
                }
            }
            fitted
        };
        let stress = |d: &[f64], fitted: &[f64]| -> f64 {
            let raw: f64 = d.iter().zip(fitted).map(|(a, b)| (a - b) * (a - b)).sum();
            let total: f64 = d.iter().map(|a| a * a).sum();
            if total > 0.0 {
                (raw / total).sqrt()
            } else {
                0.0
            }
        };

        let target = (n * (n - 1) / 2) as f64;
        let mut previous = f64::INFINITY;
        for _ in 0..NMDS_ITERATIONS {
            let d = layout_distances(&positions);
            let mut fitted = disparities(&d);
            let current = stress(&d, &fitted);
            if current < 1e-12 || previous - current < 1e-9 * previous.min(1.0) {
                break;
            }
            previous = current;
            // Disparities scaled to a fixed total, so the layout cannot shrink
            // to nothing.
            let sum_sq: f64 = fitted.iter().map(|f| f * f).sum();
            if sum_sq <= 0.0 {
                break;
            }
            let scale = (target / sum_sq).sqrt();
            fitted.iter_mut().for_each(|f| *f *= scale);
            // The Guttman transform.
            let mut b = vec![vec![0.0; n]; n];
            for (k, &(i, j)) in pairs.iter().enumerate() {
                if d[k] > 0.0 {
                    let value = -fitted[k] / d[k];
                    b[i][j] = value;
                    b[j][i] = value;
                }
            }
            for (i, row) in b.iter_mut().enumerate() {
                row[i] = -row.iter().sum::<f64>();
            }
            positions = (0..n)
                .map(|i| {
                    (0..dimensions)
                        .map(|k| (0..n).map(|j| b[i][j] * positions[j][k]).sum::<f64>() / n as f64)
                        .collect()
                })
                .collect();
        }
        let d = layout_distances(&positions);
        let final_stress = stress(&d, &disparities(&d));
        Some(Nmds {
            positions: principal_axes(positions, dimensions),
            stress: final_stress,
        })
    }

    /// The distances as first written: every feature of every pair.
    fn distances_reference(samples: &[Vec<f64>], metric: Distance) -> DistanceMatrix {
        let prepared: Vec<Vec<f64>> = samples
            .iter()
            .map(|sample| match metric {
                Distance::BrayCurtis | Distance::Euclidean => as_proportions(sample),
                Distance::Jaccard => sample
                    .iter()
                    .map(|&a| if a > 0.0 { 1.0 } else { 0.0 })
                    .collect(),
                Distance::Aitchison => centred_log_ratio(sample),
            })
            .collect();
        DistanceMatrix::from_fn(samples.len(), |i, j| {
            let pairs = prepared[i].iter().zip(&prepared[j]);
            match metric {
                Distance::BrayCurtis => {
                    let (difference, sum) =
                        pairs.fold((0.0, 0.0), |(d, s), (a, b)| (d + (a - b).abs(), s + a + b));
                    if sum > 0.0 {
                        difference / sum
                    } else {
                        0.0
                    }
                }
                Distance::Jaccard => {
                    let (either, both) = pairs.fold((0.0, 0.0), |(e, b), (x, y)| {
                        (e + f64::max(*x, *y), b + x * y)
                    });
                    if either > 0.0 {
                        1.0 - both / either
                    } else {
                        0.0
                    }
                }
                Distance::Aitchison | Distance::Euclidean => {
                    pairs.map(|(a, b)| (a - b) * (a - b)).sum::<f64>().sqrt()
                }
            }
        })
    }

    /// A sparse random community with some structure.
    fn random_community(samples: usize, features: usize, seed: u64) -> Vec<Vec<f64>> {
        let mut state = seed.max(1);
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..samples)
            .map(|i| {
                (0..features)
                    .map(|f| {
                        let v = next();
                        if v < 0.6 {
                            0.0
                        } else {
                            (v * 100.0 * (1.0 + ((i / 10 + f) % 5) as f64)).round()
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// The distances skip features absent from both samples, which changes
    /// nothing: Bray-Curtis and Jaccard are exactly as before, Aitchison
    /// (summed in four parts) the same but for rounding.
    #[test]
    fn distances_match_the_first_implementation() {
        let mut samples = random_community(40, 70, 17);
        samples.push(vec![0.0; 70]); // an empty sample
        samples.push(samples[3].clone()); // a duplicate
        for metric in Distance::ALL {
            let fast = distances(&samples, metric);
            let reference = distances_reference(&samples, metric);
            for i in 0..samples.len() {
                for j in 0..samples.len() {
                    let (a, b) = (fast.get(i, j), reference.get(i, j));
                    if matches!(metric, Distance::Aitchison | Distance::Euclidean) {
                        assert!((a - b).abs() < 1e-12, "{a} vs {b}");
                    } else {
                        assert_eq!(a.to_bits(), b.to_bits(), "{metric:?} {i} {j}");
                    }
                }
            }
        }
    }

    fn assert_same_ordination(a: &[Vec<f64>], b: &[Vec<f64>], axes: usize) {
        for (x, y) in a.iter().zip(b) {
            for k in 0..axes {
                assert!((x[k] - y[k]).abs() < 1e-9, "{} vs {}", x[k], y[k]);
            }
        }
    }

    /// PCoA, on every axis and on the leading ones, and NMDS agree with
    /// the first implementations (cyclic Jacobi eigenvectors, the Guttman
    /// transform through the whole matrix) on a Bray-Curtis matrix with
    /// negative eigenvalues.
    #[test]
    fn ordinations_match_the_first_implementation() {
        let samples = random_community(90, 50, 5);
        let matrix = distances(&samples, Distance::BrayCurtis);
        let reference = pcoa_reference(&matrix).unwrap();
        let all = pcoa(&matrix).unwrap();
        let leading = pcoa_leading(&matrix, 3).unwrap();
        assert_eq!(
            all.variance_fractions.len(),
            reference.variance_fractions.len()
        );
        assert!(
            all.variance_fractions.len() < 89,
            "negative eigenvalues left out"
        );
        for (a, b) in all
            .variance_fractions
            .iter()
            .zip(&reference.variance_fractions)
        {
            assert!((a - b).abs() < 1e-12);
        }
        assert_eq!(leading.variance_fractions, all.variance_fractions);
        assert!(leading.positions.iter().all(|p| p.len() == 3));
        assert_same_ordination(&all.positions, &reference.positions, 10);
        assert_same_ordination(&leading.positions, &reference.positions, 3);

        for matrix in [matrix, distances(&table(), Distance::BrayCurtis)] {
            let fast = nmds(&matrix, 2).unwrap();
            let slow = nmds_reference(&matrix, 2).unwrap();
            assert!((fast.stress - slow.stress).abs() < 1e-9);
            assert_same_ordination(&fast.positions, &slow.positions, 2);
        }
    }

    #[test]
    fn nmds_recovers_a_layout_from_the_order_of_its_distances() {
        // Squared distances keep only the order of the distances; a
        // layout in two dimensions still fits them perfectly.
        let plain = euclidean(&POINTS);
        let squared = DistanceMatrix::from_fn(8, |i, j| plain.get(i, j).powi(2));
        let result = nmds(&squared, 2).unwrap();
        assert!(result.stress < 1e-3, "{}", result.stress);
        // So the layout's distances follow the original ones in order.
        let layout = DistanceMatrix::from_fn(8, |i, j| {
            let (a, b) = (&result.positions[i], &result.positions[j]);
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
        });
        let pairs: Vec<(usize, usize)> = (0..8)
            .flat_map(|i| (i + 1..8).map(move |j| (i, j)))
            .collect();
        let rho = crate::stats::correlation::CorrelationMethod::Spearman
            .coefficient(
                &pairs
                    .iter()
                    .map(|&(i, j)| plain.get(i, j))
                    .collect::<Vec<_>>(),
                &pairs
                    .iter()
                    .map(|&(i, j)| layout.get(i, j))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        assert!(rho > 0.99, "{rho}");
        // Centred, with the first axis spreading the samples most.
        let spread = |k: usize| result.positions.iter().map(|p| p[k] * p[k]).sum::<f64>();
        assert!(spread(0) >= spread(1));
        let mean: f64 = result.positions.iter().map(|p| p[0]).sum::<f64>();
        assert!(mean.abs() < 1e-9);
    }

    #[test]
    fn nmds_improves_on_the_pcoa_layout() {
        let matrix = distances(&table(), Distance::BrayCurtis);
        let result = nmds(&matrix, 2).unwrap();
        assert!(result.positions.iter().all(|p| p.len() == 2));
        // Stress of the PCoA layout in two dimensions, measured the same way.
        let start = pcoa(&matrix).unwrap();
        let two: Vec<Vec<f64>> = start.positions.iter().map(|p| p[..2].to_vec()).collect();
        let pcoa_stress = {
            let again = nmds_stress(&matrix, &two);
            assert!(again > 0.0);
            again
        };
        assert!(
            result.stress <= pcoa_stress + 1e-12,
            "{} > {pcoa_stress}",
            result.stress
        );
        assert_eq!(nmds(&matrix, 2), Some(result));
        // Degenerate input.
        assert!(nmds(&DistanceMatrix::from_fn(2, |_, _| 1.0), 2).is_none());
        assert!(nmds(&DistanceMatrix::from_fn(5, |_, _| 0.0), 2).is_none());
    }

    /// Kruskal's stress-1 of a layout against distances, with the monotone
    /// fit computed by brute force: the best non-decreasing sequence.
    fn nmds_stress(matrix: &DistanceMatrix, positions: &[Vec<f64>]) -> f64 {
        let n = matrix.size();
        let mut pairs: Vec<(f64, f64)> = (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .map(|(i, j)| {
                let d = positions[i]
                    .iter()
                    .zip(&positions[j])
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt();
                (matrix.get(i, j), d)
            })
            .collect();
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Pool adjacent violators, ignoring ties in the given distances.
        let mut pools: Vec<(f64, f64)> = Vec::new();
        for &(_, d) in &pairs {
            let (mut mean, mut weight) = (d, 1.0);
            while let Some(&(last, w)) = pools.last() {
                if last <= mean {
                    break;
                }
                mean = (mean * weight + last * w) / (weight + w);
                weight += w;
                pools.pop();
            }
            pools.push((mean, weight));
        }
        let fitted: Vec<f64> = pools
            .iter()
            .flat_map(|&(mean, weight)| std::iter::repeat_n(mean, weight as usize))
            .collect();
        let raw: f64 = pairs
            .iter()
            .zip(&fitted)
            .map(|(p, f)| (p.1 - f).powi(2))
            .sum();
        let total: f64 = pairs.iter().map(|p| p.1 * p.1).sum();
        (raw / total).sqrt()
    }
}
