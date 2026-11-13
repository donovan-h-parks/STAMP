//! Rarefaction: how many features a sample would show if fewer of its
//! sequences had been read.

use super::dist::ln_gamma;

/// The counts as whole numbers, or `None` if any is negative, fractional or
/// not finite. Rarefaction draws individual sequences, so it needs counts of
/// them; proportions and normalised values cannot be rarefied.
pub fn whole_counts(values: &[f64]) -> Option<Vec<u64>> {
    values
        .iter()
        .map(|&value| {
            (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value < 1e15)
                .then_some(value as u64)
        })
        .collect()
}

/// Natural logarithm of the number of ways to choose `k` of `n`.
fn ln_choose(n: u64, k: u64) -> f64 {
    ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0)
}

/// Probability that a subsample of `depth` of `total` sequences contains
/// none of the `count` that belong to one feature:
/// `C(total - count, depth) / C(total, depth)`.
fn miss_probability(total: u64, count: u64, depth: u64) -> f64 {
    if total - count < depth {
        // Too few other sequences to fill the subsample.
        return 0.0;
    }
    // The ratio is symmetric in `count` and `depth`, and equals a product
    // with as many factors as the smaller of them, which is exact enough
    // where differences of large log-factorials are not.
    let (few, many) = (count.min(depth), count.max(depth));
    if few <= 64 {
        (0..few)
            .map(|j| ((total - many - j) as f64 / (total - j) as f64).ln())
            .sum::<f64>()
            .exp()
    } else {
        (ln_choose(total - count, depth) - ln_choose(total, depth)).exp()
    }
}

/// Expected number of features seen in a random subsample of a sample,
/// drawn without replacement, at each of `depths` sequences.
///
/// `counts` are the sequences assigned to each feature that is to be
/// counted. `total` is the number of sequences in the whole sample, which
/// may exceed their sum: sequences that belong to no counted feature (those
/// in unclassified categories, say) are still drawn, and take the place of
/// ones that would have revealed a feature. A depth beyond `total` gives the
/// number of features observed.
///
/// This is the exact expectation (Hurlbert 1971): a feature with `c`
/// sequences is missed by a subsample of `n` with probability
/// `C(total - c, n) / C(total, n)`.
pub fn expected_richness(counts: &[u64], total: u64, depths: &[u64]) -> Vec<f64> {
    let total = total.max(counts.iter().sum());
    depths
        .iter()
        .map(|&depth| {
            let depth = depth.min(total);
            if depth == 0 {
                return 0.0;
            }
            counts
                .iter()
                .filter(|&&count| count > 0)
                .map(|&count| 1.0 - miss_probability(total, count, depth))
                .sum()
        })
        .collect()
}

/// The Chao1 estimate of the number of features in the community a sample
/// was drawn from, from the features seen once and twice. The
/// bias-corrected form is used when no feature was seen exactly twice.
pub fn chao1(counts: &[u64]) -> f64 {
    let observed = counts.iter().filter(|&&c| c > 0).count() as f64;
    let singletons = counts.iter().filter(|&&c| c == 1).count() as f64;
    let doubletons = counts.iter().filter(|&&c| c == 2).count() as f64;
    if doubletons > 0.0 {
        observed + singletons * singletons / (2.0 * doubletons)
    } else {
        observed + singletons * (singletons - 1.0).max(0.0) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9 * expected.abs().max(1.0),
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn whole_numbers_only() {
        assert_eq!(whole_counts(&[0.0, 3.0, 12.0]), Some(vec![0, 3, 12]));
        assert_eq!(whole_counts(&[1.0, 2.5]), None);
        assert_eq!(whole_counts(&[-1.0]), None);
        assert_eq!(whole_counts(&[f64::NAN]), None);
        assert_eq!(whole_counts(&[]), Some(Vec::new()));
    }

    #[test]
    fn richness_follows_the_hypergeometric_expectation() {
        // Two features with one sequence each: one draw sees one of them.
        close(expected_richness(&[1, 1], 2, &[1])[0], 1.0);
        // Features of 2 and 1 sequences. A subsample of two misses the
        // singleton in one of three cases and the pair in none... the pair
        // is missed only if both draws avoid it, which cannot happen.
        let curve = expected_richness(&[2, 1], 3, &[0, 1, 2, 3, 99]);
        close(curve[0], 0.0);
        close(curve[1], 1.0);
        close(curve[2], 1.0 + 2.0 / 3.0);
        close(curve[3], 2.0);
        close(curve[4], 2.0);

        // By enumeration for a larger case: counts 3, 2, 1 and depth 2.
        // P(miss) = C(N - c, 2) / C(6, 2) = 3/15, 6/15, 10/15.
        close(
            expected_richness(&[3, 2, 1], 6, &[2])[0],
            3.0 - (3.0 + 6.0 + 10.0) / 15.0,
        );
        // The curve never falls and ends at the number observed.
        let counts = [50, 30, 10, 5, 3, 1, 1, 0];
        let depths: Vec<u64> = (0..=100).collect();
        let curve = expected_richness(&counts, 100, &depths);
        assert!(curve.windows(2).all(|pair| pair[0] <= pair[1] + 1e-12));
        close(curve[100], 7.0);
        // Large samples keep their precision.
        let big = expected_richness(&[4_000_000, 3_000_000, 1], 7_000_001, &[3_500_000]);
        close(big[0], 2.0 + 3_500_000.0 / 7_000_001.0);
    }

    #[test]
    fn uncounted_sequences_slow_the_curve() {
        // Half the sample belongs to no counted feature.
        let alone = expected_richness(&[5, 5], 10, &[2])[0];
        let diluted = expected_richness(&[5, 5], 20, &[2])[0];
        assert!(diluted < alone);
        // P(miss a feature) = C(15, 2) / C(20, 2) = 105 / 190.
        close(diluted, 2.0 * (1.0 - 105.0 / 190.0));
        // At full depth everything is seen either way.
        close(expected_richness(&[5, 5], 20, &[20])[0], 2.0);
    }

    #[test]
    fn chao1_from_singletons_and_doubletons() {
        // 5 observed, 2 singletons, 1 doubleton: 5 + 4 / 2.
        close(chao1(&[1, 1, 2, 7, 9, 0]), 7.0);
        // No doubletons: 4 + 3 * 2 / 2.
        close(chao1(&[1, 1, 1, 8]), 7.0);
        // Nothing rare: nothing to add.
        close(chao1(&[5, 6]), 2.0);
        close(chao1(&[]), 0.0);
    }
}
