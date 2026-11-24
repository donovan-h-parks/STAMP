//! Tests of presence and absence: is a feature found in more of the samples
//! of one group than of another, whatever its abundance where it is found?
//! A feature is present in a sample where its count is above zero.

use super::dist::{chi2_sf, ln_gamma};

fn ln_choose(n: f64, k: f64) -> f64 {
    ln_gamma(n + 1.0) - (ln_gamma(n - k + 1.0) + ln_gamma(k + 1.0))
}

/// Fisher's exact test on the table of present and absent against two
/// groups: `present1` of `total1` samples of the first group hold the
/// feature, and `present2` of `total2` of the second. Returns the
/// one-sided p-value (the probability of the feature being present in at
/// least as many samples of the first group) and the two-sided p-value, by
/// the "minimum likelihood" method of R's `fisher.test` and SciPy's
/// `fisher_exact`: the total probability of the tables no more likely than
/// the one observed.
pub fn fisher_presence(
    present1: usize,
    total1: usize,
    present2: usize,
    total2: usize,
) -> (f64, f64) {
    let (n1, n2) = (total1 as f64, total2 as f64);
    let present = present1 + present2;
    if total1 == 0 || total2 == 0 {
        return (1.0, 1.0);
    }
    // The first group's count given the margins is hypergeometric.
    let ln_all = ln_choose(n1 + n2, present as f64);
    let probability = |a: usize| -> f64 {
        let b = present - a;
        (ln_choose(n1, a as f64) + ln_choose(n2, b as f64) - ln_all).exp()
    };
    let lowest = present.saturating_sub(total2);
    let highest = present.min(total1);
    let cutoff = probability(present1);
    let (mut upper, mut two_sided) = (0.0, 0.0);
    for a in lowest..=highest {
        let p = probability(a);
        if a >= present1 {
            upper += p;
        }
        // Tables as likely as the observed one are counted, allowing for
        // rounding in their probabilities (the tolerance R and SciPy use).
        if p <= cutoff * (1.0 + 1e-7) {
            two_sided += p;
        }
    }
    (upper.min(1.0), two_sided.min(1.0))
}

/// McNemar's exact test for samples in pairs: of the pairs in which the
/// feature is present in one sample only, `first_only` have it in the
/// sample of the first group and `second_only` in that of the second. Each
/// such pair is as likely to fall one way as the other if presence does not
/// differ, so the count is binomial. Returns the one-sided p-value (at
/// least as many pairs with the feature in the first group only) and the
/// two-sided p-value (twice the smaller tail, at most one), as R's
/// `binom.test` and SciPy's `binomtest` give. Pairs that agree say nothing.
pub fn mcnemar(first_only: usize, second_only: usize) -> (f64, f64) {
    let n = first_only + second_only;
    if n == 0 {
        return (1.0, 1.0);
    }
    let nf = n as f64;
    let probability = |k: usize| (ln_choose(nf, k as f64) - nf * std::f64::consts::LN_2).exp();
    let upper: f64 = (first_only..=n).map(probability).sum();
    let lower: f64 = (0..=first_only).map(probability).sum();
    (upper.min(1.0), (2.0 * lower.min(upper)).min(1.0))
}

pub const PRESENT_EVERYWHERE: &str =
    "degenerate case: the feature is present in every sample, or in none";
pub const SMALL_EXPECTED: &str =
    "heuristic: expected counts below 5; the chi-square approximation is rough";

/// Pearson's chi-square test of independence on the table of present and
/// absent against any number of groups, without a continuity correction,
/// as R's `chisq.test(correct = FALSE)` and SciPy's `chi2_contingency`
/// compute it. Each group is `(samples with the feature, samples)`; groups
/// without samples are left out. Returns the p-value and a note.
pub fn chi_square_presence(groups: &[(usize, usize)]) -> (f64, &'static str) {
    let groups: Vec<(f64, f64)> = groups
        .iter()
        .filter(|(_, total)| *total > 0)
        .map(|&(present, total)| (present as f64, total as f64))
        .collect();
    if groups.len() < 2 {
        return (
            1.0,
            "degenerate case: fewer than two groups contain samples",
        );
    }
    let samples: f64 = groups.iter().map(|g| g.1).sum();
    let present: f64 = groups.iter().map(|g| g.0).sum();
    let absent = samples - present;
    if present == 0.0 || absent == 0.0 {
        return (1.0, PRESENT_EVERYWHERE);
    }
    let mut statistic = 0.0;
    let mut smallest = f64::INFINITY;
    for (observed, total) in &groups {
        for (count, margin) in [(*observed, present), (total - observed, absent)] {
            let expected = total * margin / samples;
            statistic += (count - expected) * (count - expected) / expected;
            smallest = smallest.min(expected);
        }
    }
    let note = if smallest < 5.0 { SMALL_EXPECTED } else { "" };
    (chi2_sf(statistic, groups.len() as f64 - 1.0), note)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-9 * expected.abs() + 1e-14,
            "expected {expected}, got {actual}"
        );
    }

    /// `scipy.stats.fisher_exact([[8, 2], [3, 9]])` of SciPy 1.18.1.
    #[test]
    fn fisher_matches_scipy() {
        let (upper, two_sided) = fisher_presence(8, 10, 3, 12);
        close(two_sided, 0.029973122852379817);
        // The other way round, the feature is rarely more present.
        let (other, same) = fisher_presence(3, 12, 8, 10);
        close(same, two_sided);
        assert!(upper < 0.05 && other > 0.95);
        // Present everywhere, or nowhere: nothing to tell apart.
        assert_eq!(fisher_presence(5, 5, 7, 7), (1.0, 1.0));
        assert_eq!(fisher_presence(0, 5, 0, 7), (1.0, 1.0));
        assert_eq!(fisher_presence(0, 0, 3, 7), (1.0, 1.0));
    }

    /// `scipy.stats.binomtest(2, 9, 0.5)` and `binomtest(0, 5, 0.5)`.
    #[test]
    fn mcnemar_matches_scipy() {
        close(mcnemar(2, 7).1, 0.1796875);
        close(mcnemar(7, 2).1, 0.1796875);
        close(mcnemar(0, 5).1, 0.0625);
        close(mcnemar(5, 0).0, 0.03125);
        close(mcnemar(0, 5).0, 1.0);
        assert_eq!(mcnemar(0, 0), (1.0, 1.0));
    }

    /// `scipy.stats.chi2_contingency([[8, 2], [3, 9], [5, 5]], correction=False)`
    /// (statistic 6.6).
    #[test]
    fn chi_square_matches_scipy() {
        let (p, note) = chi_square_presence(&[(8, 10), (3, 12), (5, 10)]);
        close(p, 0.03688316740124002);
        assert_eq!(note, "");
        // An empty group takes no part.
        close(
            chi_square_presence(&[(8, 10), (0, 0), (3, 12), (5, 10)]).0,
            0.03688316740124002,
        );
        assert_eq!(chi_square_presence(&[(3, 4), (1, 4)]).1, SMALL_EXPECTED);
        assert_eq!(
            chi_square_presence(&[(4, 4), (6, 6)]),
            (1.0, PRESENT_EVERYWHERE)
        );
    }
}
