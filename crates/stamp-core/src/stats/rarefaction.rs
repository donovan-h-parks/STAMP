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

/// Probability that a feature with `count` of a sample's `total` sequences
/// is not seen when the sample is rarefied to `depth` sequences. At the
/// sample's full depth, or beyond it, this is simply whether the feature
/// is absent.
pub fn absence_probability(count: u64, total: u64, depth: u64) -> f64 {
    let total = total.max(count);
    if count == 0 {
        1.0
    } else if depth >= total {
        0.0
    } else {
        miss_probability(total, count, depth)
    }
}

/// Expected number of features found in `k` samples chosen at random from
/// a set, for every `k` from zero to the number of samples: the
/// sample-based rarefaction (species accumulation) curve.
///
/// `absent[feature][sample]` is the probability that the feature is not
/// seen in the sample: one or zero for observed absence and presence, or
/// something between when the samples are rarefied to a common depth
/// first. The result is exact, the average over every choice of `k`
/// samples, not an average of random orderings.
pub fn accumulation(absent: &[Vec<f64>], samples: usize) -> Vec<f64> {
    let mut expected = vec![0.0; samples + 1];
    // missed[k]: the probability that a random k of the samples seen so
    // far all lack the feature. Adding a sample, a random k of the larger
    // set either leaves it out or contains it.
    let mut missed = vec![0.0; samples + 1];
    for feature in absent {
        if feature.len() != samples {
            continue;
        }
        missed.fill(0.0);
        missed[0] = 1.0;
        for (seen, &absence) in feature.iter().enumerate() {
            let now = (seen + 1) as f64;
            for k in (1..=seen + 1).rev() {
                let without = if k <= seen { missed[k] } else { 0.0 };
                missed[k] =
                    without * (now - k as f64) / now + absence * missed[k - 1] * k as f64 / now;
            }
        }
        for (total, missed) in expected.iter_mut().zip(&missed) {
            *total += 1.0 - missed;
        }
    }
    expected
}

/// The Chao2 estimate of the number of features in the community a set of
/// samples was drawn from, from the features found in exactly one and
/// exactly two of them. `incidence` is the number of samples each feature
/// was found in. This is the bias-corrected form, which is also defined
/// when no feature was found in exactly two samples.
pub fn chao2(incidence: &[usize], samples: usize) -> f64 {
    let observed = incidence.iter().filter(|&&n| n > 0).count() as f64;
    if samples == 0 {
        return observed;
    }
    let uniques = incidence.iter().filter(|&&n| n == 1).count() as f64;
    let duplicates = incidence.iter().filter(|&&n| n == 2).count() as f64;
    let scale = (samples as f64 - 1.0) / samples as f64;
    observed + scale * uniques * (uniques - 1.0).max(0.0) / (2.0 * (duplicates + 1.0))
}

/// The Chao1 estimate of the number of features in the community a sample
/// was drawn from, from the features seen once and twice. This is the
/// bias-corrected form, `S + F1 (F1 - 1) / (2 (F2 + 1))`, which is the one
/// scikit-bio and QIIME report and is also defined when no feature was seen
/// exactly twice.
pub fn chao1(counts: &[u64]) -> f64 {
    let observed = counts.iter().filter(|&&c| c > 0).count() as f64;
    let singletons = counts.iter().filter(|&&c| c == 1).count() as f64;
    let doubletons = counts.iter().filter(|&&c| c == 2).count() as f64;
    observed + singletons * (singletons - 1.0).max(0.0) / (2.0 * (doubletons + 1.0))
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
        // 5 observed, 2 singletons, 1 doubleton: 5 + 2 * 1 / (2 * 2).
        close(chao1(&[1, 1, 2, 7, 9, 0]), 5.5);
        // No doubletons: 4 + 3 * 2 / 2.
        close(chao1(&[1, 1, 1, 8]), 7.0);
        // Nothing rare: nothing to add.
        close(chao1(&[5, 6]), 2.0);
        close(chao1(&[]), 0.0);
    }

    #[test]
    fn accumulation_averages_over_every_choice_of_samples() {
        // Three samples; features found in all, in two, and in one of them.
        let present = [[1, 1, 1], [1, 1, 0], [0, 0, 1], [0, 1, 0]];
        let absent: Vec<Vec<f64>> = present
            .iter()
            .map(|row| row.iter().map(|&p| 1.0 - f64::from(p)).collect())
            .collect();
        let curve = accumulation(&absent, 3);
        close(curve[0], 0.0);
        // One sample: the mean richness of the three, (2 + 3 + 2) / 3.
        close(curve[1], 7.0 / 3.0);
        // Two samples: pairs see 3, 3 and 4 of the features... the pairs
        // are {1,2}: 3, {1,3}: 3, {2,3}: 4.
        close(curve[2], 10.0 / 3.0);
        close(curve[3], 4.0);
        // The same as the hypergeometric formula on the incidences.
        let by_formula = |k: u64| -> f64 {
            [3u64, 2, 1, 1]
                .iter()
                .map(|&n| 1.0 - miss_probability(3, n, k))
                .sum()
        };
        for k in 1..=3 {
            close(curve[k as usize], by_formula(k));
        }

        // Uncertain presence: a feature seen in each of two samples with
        // probability one half is found in a random one of them half the
        // time, and in both together three times in four.
        let uncertain = accumulation(&[vec![0.5, 0.5]], 2);
        close(uncertain[1], 0.5);
        close(uncertain[2], 0.75);

        // Many samples stay finite and rise steadily.
        let many: Vec<Vec<f64>> = (0..40)
            .map(|f| {
                (0..600)
                    .map(|s| f64::from(u8::from((s * 7 + f) % (f + 2) != 0)))
                    .collect()
            })
            .collect();
        let curve = accumulation(&many, 600);
        assert!(curve.iter().all(|v| v.is_finite()));
        assert!(curve.windows(2).all(|p| p[0] <= p[1] + 1e-9));
        // Five of the forty features occur in no sample.
        close(curve[600], 35.0);
    }

    #[test]
    fn absence_and_chao2() {
        assert_eq!(absence_probability(0, 100, 10), 1.0);
        assert_eq!(absence_probability(5, 100, 100), 0.0);
        assert_eq!(absence_probability(5, 100, 500), 0.0);
        // One sequence of ten, five drawn: missed half the time.
        close(absence_probability(1, 10, 5), 0.5);

        // 4 observed, 2 in one sample only, 1 in two; 5 samples:
        // 4 + (4 / 5) * 2 * 1 / (2 * 2).
        close(chao2(&[1, 1, 2, 5, 0], 5), 4.4);
        // No duplicates: 3 + (2 / 3) * 2 * 1 / 2.
        close(chao2(&[1, 1, 3], 3), 3.0 + 2.0 / 3.0);
        close(chao2(&[2, 3], 0), 2.0);
    }

    /// Reference values from R 4.4.3 with vegan 2.7.5:
    /// - `rarefy(x, sample)`, the expected number of features in a random
    ///   subsample (Hurlbert 1971);
    /// - `estimateR(x)["S.chao1"]`, the bias-corrected Chao1;
    /// - `specaccum(m, method = "exact")$richness`, the exact
    ///   sample-based accumulation curve (Kindt's exact method);
    /// - `specpool(m)$chao`, Chao2. vegan uses the bias-corrected form only
    ///   when no feature is in exactly two samples, so it is compared there.
    ///   Otherwise this crate keeps the bias-corrected form,
    ///   S + ((m - 1) / m) Q1 (Q1 - 1) / (2 (Q2 + 1)) (Chao 1987; the
    ///   EstimateS user's guide), checked by hand in `absence_and_chao2`.
    #[test]
    fn richness_estimates_match_vegan() {
        let counts = [500, 300, 120, 50, 20, 5, 3, 1, 1];
        let total = counts.iter().sum();
        let curve = expected_richness(&counts, total, &[10, 100, 500]);
        for (got, expected) in
            curve
                .iter()
                .zip([3.3797848471086782, 5.7579321396274663, 7.8444369318917513])
        {
            close(*got, expected);
        }
        close(
            expected_richness(&[3, 2, 1], 6, &[2])[0],
            1.7333333333333329,
        );
        close(
            expected_richness(&[3, 2, 1], 6, &[4])[0],
            2.5999999999999996,
        );

        close(chao1(&[1, 1, 2, 5, 9]), 5.5);
        close(chao1(&[1, 1, 1, 3]), 7.0);

        // vegan's matrix(c(1,1,1, 1,1,0, 0,1,0, 1,0,0), nrow = 3): samples
        // by rows, four features by columns.
        let present = [[1, 1, 1], [1, 1, 0], [0, 1, 0], [1, 0, 0]];
        let absent: Vec<Vec<f64>> = present
            .iter()
            .map(|row| row.iter().map(|&p| 1.0 - f64::from(p)).collect())
            .collect();
        let curve = accumulation(&absent, 3);
        for (got, expected) in curve[1..]
            .iter()
            .zip([2.3333333333333335, 3.3333333333333335, 4.0])
        {
            close(*got, expected);
        }

        // rbind(c(1,1,0,0), c(1,0,1,0), c(1,0,0,1)): one feature in all
        // three samples and three in one each.
        close(chao2(&[3, 1, 1, 1], 3), 6.0);
    }
}
