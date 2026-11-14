//! Diversity of communities: within a sample (alpha diversity) and between
//! samples (distances, also called beta diversity), and the ordination of
//! samples by their distances.

use super::pca::symmetric_eigen;
use super::rarefaction::{chao1, whole_counts};
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
}

impl Distance {
    pub const ALL: [Distance; 3] = [Distance::BrayCurtis, Distance::Jaccard, Distance::Aitchison];

    pub fn name(self) -> &'static str {
        match self {
            Distance::BrayCurtis => "Bray-Curtis",
            Distance::Jaccard => "Jaccard",
            Distance::Aitchison => "Aitchison",
        }
    }
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
    let prepared: Vec<Vec<f64>> = samples
        .iter()
        .map(|sample| match metric {
            Distance::BrayCurtis => {
                let total: f64 = sample.iter().filter(|a| **a > 0.0).sum();
                sample
                    .iter()
                    .map(|&a| if total > 0.0 { a.max(0.0) / total } else { 0.0 })
                    .collect()
            }
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
            Distance::Aitchison => pairs.map(|(a, b)| (a - b) * (a - b)).sum::<f64>().sqrt(),
        }
    })
}

/// Samples placed in space so that the distances between them are kept as
/// well as possible.
#[derive(Debug, Clone, PartialEq)]
pub struct Ordination {
    /// Coordinates of each sample: `positions[sample][axis]`.
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
        let extreme = column
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
}
