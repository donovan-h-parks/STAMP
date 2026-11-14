//! Correlations between features across samples (co-occurrence): which
//! features rise and fall together.
//!
//! Proportions of one sample sum to 100%, so a rise in one feature lowers
//! the others and correlations between proportions are biased towards
//! negative values (compositional data). The centred log-ratio transform
//! (see [`super::results::clr_values`]) removes that constraint and is the
//! usual remedy.

use super::correction::Correction;
use super::correlation::{correlation_p_value, ranks, CorrelationMethod};
use super::rng_from_seed;

/// How features are ranked when only the first few are correlated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FeatureRanking {
    /// Highest mean relative abundance across the samples.
    MeanAbundance,
    /// Present in the most samples.
    Prevalence,
}

impl FeatureRanking {
    pub const ALL: [FeatureRanking; 2] =
        [FeatureRanking::MeanAbundance, FeatureRanking::Prevalence];

    pub fn name(self) -> &'static str {
        match self {
            FeatureRanking::MeanAbundance => "Mean abundance",
            FeatureRanking::Prevalence => "Prevalence",
        }
    }
}

/// The features to correlate: the `top` features by `ranking` of the
/// relative abundances `proportions` (by feature, then sample), followed by
/// those of `forced` (indices) not already among them. Ties are broken by
/// the order of the features.
pub fn top_features(
    proportions: &[Vec<f64>],
    ranking: FeatureRanking,
    top: usize,
    forced: &[usize],
) -> Vec<usize> {
    let score = |row: &[f64]| match ranking {
        FeatureRanking::MeanAbundance => row.iter().sum::<f64>() / row.len().max(1) as f64,
        FeatureRanking::Prevalence => row.iter().filter(|&&v| v > 0.0).count() as f64,
    };
    let scores: Vec<f64> = proportions.iter().map(|r| score(r)).collect();
    let mut order: Vec<usize> = (0..proportions.len()).collect();
    order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]).then(a.cmp(&b)));
    order.truncate(top);
    for &index in forced {
        if index < proportions.len() && !order.contains(&index) {
            order.push(index);
        }
    }
    order
}

/// Values of a feature prepared so that the correlation of two features is
/// the sum of the products of their prepared values: ranked for Spearman,
/// centred, and scaled to unit length. `None` if the values are constant.
fn prepare(values: &[f64], method: CorrelationMethod) -> Option<Vec<f64>> {
    let values = match method {
        CorrelationMethod::Pearson => values.to_vec(),
        CorrelationMethod::Spearman => ranks(values),
    };
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let centred: Vec<f64> = values.iter().map(|v| v - mean).collect();
    let length = centred.iter().map(|v| v * v).sum::<f64>().sqrt();
    // Constant values: allow for rounding in the mean.
    let scale = values.iter().map(|v| v.abs()).fold(0.0, f64::max);
    (length > 1e-12 * scale.max(1e-300) && length > 0.0)
        .then(|| centred.iter().map(|v| v / length).collect())
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| x * y)
        .sum::<f64>()
        .clamp(-1.0, 1.0)
}

/// Benjamini-Hochberg q-values of p-values, NaN kept where a p-value is NaN.
fn benjamini_hochberg(p_values: &[f64]) -> Vec<f64> {
    let defined: Vec<usize> = (0..p_values.len())
        .filter(|&i| p_values[i].is_finite())
        .collect();
    let values: Vec<f64> = defined.iter().map(|&i| p_values[i]).collect();
    // The correction does not draw random numbers; the generator is only
    // part of its signature.
    let corrected = Correction::BenjaminiHochberg
        .correct(&values, 0.05, &mut rng_from_seed(0))
        .values;
    let mut q = vec![f64::NAN; p_values.len()];
    for (&i, value) in defined.iter().zip(corrected) {
        q[i] = value;
    }
    q
}

/// Correlations between every pair of features. Matrices are symmetric,
/// indexed `[feature][feature]`; a coefficient (and its p- and q-values)
/// is NaN where a feature does not vary, and the diagonal holds 1.
#[derive(Debug, Clone, PartialEq)]
pub struct CoOccurrence {
    pub features: Vec<String>,
    pub method: CorrelationMethod,
    /// Number of samples correlated.
    pub samples: usize,
    pub coefficients: Vec<Vec<f64>>,
    /// Two-sided p-values against no correlation.
    pub p_values: Vec<Vec<f64>>,
    /// Benjamini-Hochberg q-values over the n(n - 1)/2 pairs.
    pub q_values: Vec<Vec<f64>>,
}

/// Correlate every pair of features. `values` holds each feature's value
/// (a proportion, or its centred log ratio) in every sample.
pub fn co_occurrence(
    features: Vec<String>,
    values: &[Vec<f64>],
    method: CorrelationMethod,
) -> CoOccurrence {
    let n = values.len();
    let samples = values.first().map_or(0, Vec::len);
    let prepared: Vec<Option<Vec<f64>>> = values
        .iter()
        .map(|v| (samples >= 3).then(|| prepare(v, method)).flatten())
        .collect();
    let mut coefficients = vec![vec![f64::NAN; n]; n];
    let mut p_values = vec![vec![f64::NAN; n]; n];
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    let mut pair_p: Vec<f64> = Vec::new();
    for i in 0..n {
        coefficients[i][i] = 1.0;
        for j in i + 1..n {
            if let (Some(a), Some(b)) = (&prepared[i], &prepared[j]) {
                let r = dot(a, b);
                let p = correlation_p_value(r, samples);
                coefficients[i][j] = r;
                coefficients[j][i] = r;
                p_values[i][j] = p;
                p_values[j][i] = p;
                pairs.push((i, j));
                pair_p.push(p);
            }
        }
    }
    let mut q_values = vec![vec![f64::NAN; n]; n];
    for (&(i, j), q) in pairs.iter().zip(benjamini_hochberg(&pair_p)) {
        q_values[i][j] = q;
        q_values[j][i] = q;
    }
    CoOccurrence {
        features,
        method,
        samples,
        coefficients,
        p_values,
        q_values,
    }
}

/// The correlation of one feature with another.
#[derive(Debug, Clone, PartialEq)]
pub struct Partner {
    pub feature: String,
    pub coefficient: f64,
    pub p_value: f64,
    /// Benjamini-Hochberg q-value over the correlations of the feature with
    /// every other.
    pub q_value: f64,
}

/// Correlations of `target` with each of `others` (name and values),
/// strongest first (by distance from zero); features that do not vary are
/// left out.
pub fn partners(
    target: &[f64],
    others: &[(String, &[f64])],
    method: CorrelationMethod,
) -> Vec<Partner> {
    let samples = target.len();
    let Some(base) = (samples >= 3).then(|| prepare(target, method)).flatten() else {
        return Vec::new();
    };
    let mut found: Vec<Partner> = others
        .iter()
        .filter_map(|(name, values)| {
            let other = prepare(values, method)?;
            let r = dot(&base, &other);
            Some(Partner {
                feature: name.clone(),
                coefficient: r,
                p_value: correlation_p_value(r, samples),
                q_value: f64::NAN,
            })
        })
        .collect();
    let p: Vec<f64> = found.iter().map(|f| f.p_value).collect();
    for (partner, q) in found.iter_mut().zip(benjamini_hochberg(&p)) {
        partner.q_value = q;
    }
    found.sort_by(|a, b| {
        b.coefficient
            .abs()
            .total_cmp(&a.coefficient.abs())
            .then_with(|| a.feature.cmp(&b.feature))
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual}"
        );
    }

    // Four features in twelve samples, with zeros and ties.
    const VALUES: [[f64; 12]; 4] = [
        [0.0, 1.2, 3.4, 0.0, 5.1, 2.2, 0.0, 7.5, 4.4, 3.3, 0.0, 6.0],
        [0.5, 1.0, 2.9, 0.1, 4.0, 2.5, 0.3, 6.1, 3.0, 3.9, 0.0, 4.4],
        [9.0, 7.7, 5.5, 8.8, 2.0, 6.0, 9.9, 1.0, 4.0, 6.0, 8.0, 3.0],
        [2.0, 0.0, 2.0, 1.0, 0.0, 3.0, 2.0, 1.0, 0.0, 2.0, 1.0, 0.0],
    ];

    fn rows() -> Vec<Vec<f64>> {
        VALUES.iter().map(|r| r.to_vec()).collect()
    }

    fn names() -> Vec<String> {
        ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect()
    }

    // Reference values from SciPy 1.16.0 (`scipy.stats.spearmanr`,
    // `scipy.stats.pearsonr` for each pair, and
    // `scipy.stats.false_discovery_control(p, method="bh")` over the six
    // pairs), in the order (a,b), (a,c), (a,d), (b,c), (b,d), (c,d).
    const SPEARMAN: [(f64, f64, f64); 6] = [
        (
            0.9610062376230345,
            6.649518363963842e-07,
            1.9948555091891526e-06,
        ),
        (
            -0.9733873008872699,
            1.0053888079851838e-07,
            6.032332847911103e-07,
        ),
        (-0.4127791877216318, 0.18234033237225566, 0.2188083988467068),
        (
            -0.900176511814775,
            6.588021000587215e-05,
            0.0001317604200117443,
        ),
        (-0.3032106351448338, 0.338043317789007, 0.338043317789007),
        (0.4482940895579113, 0.14384109775099532, 0.215761646626493),
    ];
    const PEARSON: [(f64, f64, f64); 6] = [
        (
            0.9731428786059714,
            1.0519812557774126e-07,
            3.249112695069103e-07,
        ),
        (
            -0.9729847058357964,
            1.0830375650230344e-07,
            3.249112695069103e-07,
        ),
        (
            -0.3756888030033614,
            0.22878637345355146,
            0.27454364814426174,
        ),
        (
            -0.9338828212157224,
            8.899248364237102e-06,
            1.7798496728474205e-05,
        ),
        (-0.20195136520564042, 0.5290632870063338, 0.5290632870063338),
        (0.42127994488783027, 0.1725979921535433, 0.25889698823031493),
    ];

    #[test]
    fn matches_scipy() {
        for (method, expected) in [
            (CorrelationMethod::Spearman, SPEARMAN),
            (CorrelationMethod::Pearson, PEARSON),
        ] {
            let result = co_occurrence(names(), &rows(), method);
            assert_eq!(result.samples, 12);
            let pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
            for (&(i, j), &(r, p, q)) in pairs.iter().zip(&expected) {
                close(result.coefficients[i][j], r, 1e-12);
                close(result.coefficients[j][i], r, 1e-12);
                // p-values via the t distribution, as in correlation.rs.
                close(result.p_values[i][j], p, 1e-12 + 1e-9 * p);
                close(result.q_values[j][i], q, 1e-12 + 1e-9 * q);
            }
            for i in 0..4 {
                assert_eq!(result.coefficients[i][i], 1.0);
            }
            // The same as the correlation of a feature with a field.
            for &(i, j) in &pairs {
                let r = method.coefficient(&VALUES[i], &VALUES[j]).unwrap();
                close(result.coefficients[i][j], r, 1e-12);
            }
        }
    }

    #[test]
    fn constant_features_have_no_correlation() {
        let mut rows = rows();
        rows[3] = vec![0.0; 12];
        let result = co_occurrence(names(), &rows, CorrelationMethod::Spearman);
        assert!(result.coefficients[0][3].is_nan());
        assert!(result.q_values[3][1].is_nan());
        assert_eq!(result.coefficients[3][3], 1.0);
        // BH now runs over the three defined pairs.
        let p = result.p_values[0][1];
        let smallest = [(0, 1), (0, 2), (1, 2)]
            .iter()
            .map(|&(i, j)| result.p_values[i][j])
            .fold(f64::INFINITY, f64::min);
        assert!(result.q_values[0][1] >= p);
        assert!(smallest <= p);
        // Too few samples.
        let short: Vec<Vec<f64>> = rows.iter().map(|r| r[..2].to_vec()).collect();
        let result = co_occurrence(names(), &short, CorrelationMethod::Pearson);
        assert!(result.coefficients[0][1].is_nan());
    }

    #[test]
    fn partners_are_ranked_by_strength() {
        let rows = rows();
        let others: Vec<(String, &[f64])> = names()
            .into_iter()
            .zip(&rows)
            .skip(1)
            .map(|(n, r)| (n, r.as_slice()))
            .collect();
        let found = partners(&rows[0], &others, CorrelationMethod::Spearman);
        let order: Vec<&str> = found.iter().map(|p| p.feature.as_str()).collect();
        assert_eq!(order, ["c", "b", "d"]);
        close(found[0].coefficient, SPEARMAN[1].0, 1e-12);
        close(found[1].p_value, SPEARMAN[0].1, 1e-12);
        // BH over three tests: q = p * 3 / rank, made monotone.
        close(found[2].q_value, SPEARMAN[2].1, 1e-12);
        close(found[1].q_value, SPEARMAN[0].1 * 3.0 / 2.0, 1e-15);
    }

    #[test]
    fn top_features_by_abundance_or_prevalence() {
        let rows = rows();
        // Means: a 2.76, b 2.39, c 5.91, d 1.17; prevalence a 8, b 11, c 12, d 8.
        assert_eq!(
            top_features(&rows, FeatureRanking::MeanAbundance, 2, &[]),
            [2, 0]
        );
        assert_eq!(
            top_features(&rows, FeatureRanking::Prevalence, 2, &[]),
            [2, 1]
        );
        // Forced features are added once, after the top ones.
        assert_eq!(
            top_features(&rows, FeatureRanking::Prevalence, 2, &[3, 1, 3]),
            [2, 1, 3]
        );
        assert_eq!(
            top_features(&rows, FeatureRanking::Prevalence, 0, &[3]),
            [3]
        );
        // Ties keep the order of the features.
        assert_eq!(
            top_features(&rows, FeatureRanking::Prevalence, 4, &[]),
            [2, 1, 0, 3]
        );
    }
}
