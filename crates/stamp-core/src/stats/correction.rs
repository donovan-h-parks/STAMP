//! Corrections of p-values for multiple comparisons.

use super::two_sample::random_index;
use super::StampRng;

/// Multiple-comparison correction methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Correction {
    BenjaminiHochberg,
    Bonferroni,
    HolmBonferroni,
    None,
    Sidak,
    Storey,
}

/// Corrected p-values with summary information about the correction.
#[derive(Debug, Clone, PartialEq)]
pub struct Corrected {
    /// Corrected value for each input p-value, in input order.
    pub values: Vec<f64>,
    /// Labelled summary values, such as the number of significant features.
    pub info: Vec<(&'static str, f64)>,
    /// A caution for the user about the suitability of the method.
    pub warning: Option<&'static str>,
}

const SIGNIFICANT: &str = "Number of significant features";

impl Correction {
    pub const ALL: [Correction; 6] = [
        Correction::BenjaminiHochberg,
        Correction::Bonferroni,
        Correction::HolmBonferroni,
        Correction::None,
        Correction::Sidak,
        Correction::Storey,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Correction::BenjaminiHochberg => "Benjamini-Hochberg FDR",
            Correction::Bonferroni => "Bonferroni",
            Correction::HolmBonferroni => "Holm-Bonferroni",
            Correction::None => "No correction",
            Correction::Sidak => "Sidak",
            Correction::Storey => "Storey FDR",
        }
    }

    pub fn from_name(name: &str) -> Option<Correction> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }

    /// True if the corrected values are q-values controlling the false
    /// discovery rate, rather than p-values controlling the family-wise
    /// error rate.
    pub fn is_false_discovery_rate(self) -> bool {
        matches!(self, Correction::BenjaminiHochberg | Correction::Storey)
    }

    /// True if the result depends on the random seed.
    pub fn is_stochastic(self) -> bool {
        self == Correction::Storey
    }

    /// Correct a set of p-values. `alpha` is the significance level used to
    /// count significant features.
    pub fn correct(self, p_values: &[f64], alpha: f64, rng: &mut StampRng) -> Corrected {
        let n = p_values.len() as f64;
        let mut warning = None;
        let mut extra: Vec<(&'static str, f64)> = Vec::new();

        let values = match self {
            Correction::None => p_values.to_vec(),
            Correction::Bonferroni => {
                extra = vec![("Corrected error rate", alpha / n)];
                p_values.iter().map(|p| (p * n).min(1.0)).collect()
            }
            Correction::Sidak => {
                extra = vec![("Corrected error rate", 1.0 - (1.0 - alpha).powf(1.0 / n))];
                // 1 - (1 - p)^n, without losing the digits of a small p.
                p_values
                    .iter()
                    .map(|&p| {
                        if p.is_finite() {
                            -(n * (-p.clamp(0.0, 1.0)).ln_1p()).exp_m1()
                        } else {
                            // The infinite "p-value" of a test with no
                            // one-sided form goes through as in v2.
                            1.0 - (1.0 - p).powf(n)
                        }
                    })
                    .collect()
            }
            Correction::HolmBonferroni => {
                // Step-down: the i-th smallest p-value is multiplied by
                // (n - i + 1), and adjusted values never decrease.
                let order = ascending_order(p_values);
                let mut values = vec![0.0; p_values.len()];
                let mut running_max: f64 = 0.0;
                for (rank, &index) in order.iter().enumerate() {
                    let adjusted = (p_values[index] * (n - rank as f64)).min(1.0);
                    running_max = running_max.max(adjusted);
                    values[index] = running_max;
                }
                values
            }
            Correction::BenjaminiHochberg => {
                // Step-up: the i-th smallest p-value is multiplied by n / i,
                // and adjusted values never increase going down the ranks.
                let order = ascending_order(p_values);
                let mut values = vec![0.0; p_values.len()];
                let mut running_min: f64 = 1.0;
                for (rank, &index) in order.iter().enumerate().rev() {
                    let adjusted = p_values[index] * n / (rank as f64 + 1.0);
                    running_min = running_min.min(adjusted);
                    values[index] = running_min;
                }
                values
            }
            Correction::Storey => {
                let (values, estimate) = storey(p_values, rng);
                extra = vec![
                    ("Estimated pi_0", estimate.pi0),
                    ("pi_0 used", estimate.used()),
                    ("Lambda", estimate.lambda),
                    ("P-values above lambda", estimate.above as f64),
                    ("Standard error of pi_0", estimate.standard_error),
                ];
                if !estimate.reliable {
                    warning = Some(
                        "Storey's estimate of pi_0 is too uncertain to use: too few p-values \
                         lie above lambda, or none reaches 0.95. pi_0 = 1 was used instead, so \
                         the q-values are those of Benjamini-Hochberg.",
                    );
                } else if estimate.pi0 < 0.01 {
                    warning = Some(
                        "P-values do not appear to be uniformly distributed. Consider using \
                         the Benjamini-Hochberg FDR approach.",
                    );
                }
                values
            }
        };

        let significant = values.iter().filter(|v| **v <= alpha).count() as f64;
        let mut info = vec![(SIGNIFICANT, significant)];
        info.extend(extra);
        Corrected {
            values,
            info,
            warning,
        }
    }
}

/// Indices of the p-values from smallest to largest.
fn ascending_order(p_values: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..p_values.len()).collect();
    order.sort_by(|&a, &b| p_values[a].total_cmp(&p_values[b]).then(a.cmp(&b)));
    order
}

/// Storey's estimate of the proportion of true null hypotheses, pi_0, and
/// how far it can be trusted.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pi0Estimate {
    /// The estimate, at most one.
    pi0: f64,
    /// The lambda it was taken at: pi_0 is the share of p-values above
    /// lambda, divided by 1 - lambda.
    lambda: f64,
    /// How many p-values lie above lambda.
    above: usize,
    /// Standard error of the estimate (binomial, for the number above).
    standard_error: f64,
    /// Whether the estimate is sound enough to use (see [`storey`]).
    reliable: bool,
}

impl Pi0Estimate {
    /// The pi_0 the q-values are computed with: one when the estimate
    /// cannot be trusted, which gives the Benjamini-Hochberg q-values.
    fn used(&self) -> f64 {
        if self.reliable {
            self.pi0
        } else {
            1.0
        }
    }
}

/// The largest standard error of pi_0, relative to the estimate, at which
/// it is used. With few p-values above lambda the relative error is about
/// one over the square root of their number, so this asks for roughly 16 or
/// more of them.
const MAX_RELATIVE_ERROR_OF_PI0: f64 = 0.25;

/// Storey's q-values (Storey and Tibshirani, 2003), with the proportion of
/// true null hypotheses (pi_0) estimated by the bootstrap procedure of
/// Storey, Taylor and Siegmund (2004).
///
/// An estimate of pi_0 below one makes every q-value smaller than
/// Benjamini-Hochberg's, so it is used only when it can be trusted;
/// otherwise pi_0 = 1. It cannot be trusted when no p-value reaches 0.95
/// (R's qvalue refuses to estimate pi_0 then), when no p-value lies above
/// the chosen lambda (an estimate of 0, which made every q-value 0), or
/// when its standard error is more than [`MAX_RELATIVE_ERROR_OF_PI0`] of
/// it: with few features the estimate rests on a handful of p-values (2 of
/// 41 in one data set, which put 34 of the 41 features below q = 0.05
/// where Benjamini-Hochberg found none).
fn storey(p_values: &[f64], rng: &mut StampRng) -> (Vec<f64>, Pi0Estimate) {
    let count = p_values.len();
    if count == 0 {
        let none = Pi0Estimate {
            pi0: 1.0,
            lambda: 0.0,
            above: 0,
            standard_error: 0.0,
            reliable: true,
        };
        return (Vec::new(), none);
    }
    let n = count as f64;
    // 0, 0.05, ..., 0.95, as exactly as they can be written.
    let lambdas: Vec<f64> = (0..20).map(|i| i as f64 / 20.0).collect();
    let above = |values: &[f64], lambda: f64| values.iter().filter(|p| **p > lambda).count();
    let pi0_at = |values: &[f64], lambda: f64| above(values, lambda) as f64 / (n * (1.0 - lambda));

    let min_pi0 = lambdas
        .iter()
        .map(|&l| pi0_at(p_values, l))
        .fold(1.0, f64::min);

    // Choose the lambda whose bootstrap estimates of pi_0 are closest, in
    // mean squared error, to the smallest estimate.
    const BOOTSTRAPS: usize = 100;
    let mut best_mse = f64::INFINITY;
    let mut best_lambda = lambdas[0];
    let mut resample = vec![0.0; count];
    for &lambda in &lambdas {
        let mut mse = 0.0;
        for _ in 0..BOOTSTRAPS {
            for slot in resample.iter_mut() {
                *slot = p_values[random_index(rng, count)];
            }
            let difference = pi0_at(&resample, lambda) - min_pi0;
            mse += difference * difference;
        }
        mse /= BOOTSTRAPS as f64;
        if mse < best_mse {
            best_mse = mse;
            best_lambda = lambda;
        }
    }
    let lambda = best_lambda;
    let raw = pi0_at(p_values, lambda);
    let w = above(p_values, lambda);
    let share = w as f64 / n;
    let standard_error = (share * (1.0 - share) / n).sqrt() / (1.0 - lambda);
    let highest = p_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // An estimate of one or more gives the Benjamini-Hochberg q-values
    // anyway, so only one below needs to be sound.
    let reliable = raw >= 1.0
        || (w > 0 && highest >= 0.95 && standard_error <= MAX_RELATIVE_ERROR_OF_PI0 * raw);
    let estimate = Pi0Estimate {
        // A proportion cannot exceed one; R's qvalue caps it the same way.
        pi0: raw.min(1.0),
        lambda,
        above: w,
        standard_error,
        reliable,
    };
    (q_values(p_values, estimate.used()), estimate)
}

/// Storey's q-values for a given proportion of true null hypotheses:
/// `pi0 * m * p / rank`, made non-decreasing in p from the largest down and
/// capped at one, as R's qvalue computes them.
fn q_values(p_values: &[f64], pi0: f64) -> Vec<f64> {
    let count = p_values.len();
    let n = count as f64;
    let mut order = ascending_order(p_values);
    order.reverse();
    let mut q_values = vec![0.0; count];
    let mut running: f64 = 1.0;
    for (i, &index) in order.iter().enumerate() {
        let candidate = pi0 * n * p_values[index] / (n - i as f64);
        running = running.min(candidate);
        q_values[index] = running;
    }
    q_values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rng_from_seed;

    const P: [f64; 6] = [1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1];

    fn correct(method: Correction, p: &[f64]) -> Vec<f64> {
        method.correct(p, 0.05, &mut rng_from_seed(7)).values
    }

    fn close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a - e).abs() <= 1e-12 * e.abs().max(1e-300) + 1e-15,
                "{actual:?} vs {expected:?}"
            );
        }
    }

    /// statsmodels 0.15.0:
    /// `multipletests([0.01, 0.04, 0.03, 0.04, 0.9, 0.02], method = "sidak")`.
    #[test]
    fn sidak_matches_statsmodels() {
        close(
            &correct(Correction::Sidak, &[0.01, 0.04, 0.03, 0.04, 0.9, 0.02]),
            &[
                0.058519850599,
                0.217242210304,
                0.16702799507100002,
                0.217242210304,
                0.999999,
                0.114157619136,
            ],
        );
    }

    /// R 4.4.3 with qvalue 2.38.0 (Bioconductor):
    /// `qvalue(p, pi0 = 0.6)$qvalues` and `qvalue(p, pi0 = 1)$qvalues`, the
    /// q-values for a given proportion of true null hypotheses, capped at
    /// one; and `pi0est(p, lambda = 0.5)$pi0`, the estimate at one lambda,
    /// which R caps at one.
    #[test]
    fn storey_q_values_match_qvalue() {
        let p = [1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1, 0.3, 0.5, 0.7, 0.95];
        close(
            &q_values(&p, 0.6),
            &[
                5.9999999999999993e-06,
                3.0000000000000001e-05,
                0.00019999999999999998,
                0.0015,
                0.012,
                0.099999999999999992,
                0.25714285714285712,
                0.375,
                0.46666666666666667,
                0.56999999999999995,
            ],
        );
        close(
            &q_values(&[0.2, 0.9, 0.99], 1.0),
            &[
                0.60000000000000009,
                0.98999999999999988,
                0.98999999999999988,
            ],
        );
        // Uniform p-values estimate pi_0 at one, not above it: by itself,
        // #(p > 0.5) / (m (1 - 0.5)) is 1.0 here, and over one for other
        // lambdas; the estimate is capped as R caps it.
        let uniform: Vec<f64> = (1..=200).map(|i| i as f64 / 200.0).collect();
        let r = Correction::Storey.correct(&uniform, 0.05, &mut rng_from_seed(3));
        assert!(r.info[1].1 <= 1.0);
        assert!(r.values.iter().all(|q| *q <= 1.0));
    }

    #[test]
    fn simple_corrections() {
        close(&correct(Correction::None, &P), &P);
        close(
            &correct(Correction::Bonferroni, &P),
            &[6e-6, 6e-5, 6e-4, 6e-3, 6e-2, 6e-1],
        );
        let sidak: Vec<f64> = P.iter().map(|p| 1.0 - (1.0 - p).powi(6)).collect();
        for (a, e) in correct(Correction::Sidak, &P).iter().zip(&sidak) {
            assert!((a - e).abs() < 1e-10 * e, "{a} against {e}");
        }
        // Bonferroni never reports a corrected p-value above one.
        close(&correct(Correction::Bonferroni, &[0.5, 0.01]), &[1.0, 0.02]);
    }

    #[test]
    fn benjamini_hochberg_matches_r() {
        // Already monotonic: p * n / rank, as in the original unit test.
        let expected: Vec<f64> = P
            .iter()
            .enumerate()
            .map(|(i, p)| p * 6.0 / (i as f64 + 1.0))
            .collect();
        close(&correct(Correction::BenjaminiHochberg, &P), &expected);

        // p.adjust(c(0.01, 0.04, 0.03, 0.04, 0.9, 0.02), method = "BH")
        close(
            &correct(
                Correction::BenjaminiHochberg,
                &[0.01, 0.04, 0.03, 0.04, 0.9, 0.02],
            ),
            &[0.048, 0.048, 0.048, 0.048, 0.9, 0.048],
        );
        // p.adjust(c(0.5, 0.6, 0.7), method = "BH")
        close(
            &correct(Correction::BenjaminiHochberg, &[0.5, 0.6, 0.7]),
            &[0.7, 0.7, 0.7],
        );
    }

    #[test]
    fn holm_bonferroni_matches_r() {
        // p.adjust(c(0.01, 0.04, 0.03, 0.04, 0.9, 0.02), method = "holm")
        close(
            &correct(
                Correction::HolmBonferroni,
                &[0.01, 0.04, 0.03, 0.04, 0.9, 0.02],
            ),
            &[0.06, 0.12, 0.12, 0.12, 0.9, 0.1],
        );
        close(
            &correct(Correction::HolmBonferroni, &[0.6, 0.5]),
            &[1.0, 1.0],
        );
    }

    #[test]
    fn summary_information() {
        let r = Correction::Bonferroni.correct(&P, 0.05, &mut rng_from_seed(1));
        assert_eq!(r.info[0], (SIGNIFICANT, 4.0));
        assert_eq!(r.info[1].0, "Corrected error rate");
        assert!((r.info[1].1 - 0.05 / 6.0).abs() < 1e-15);
        assert!(Correction::Storey.is_false_discovery_rate());
        assert!(!Correction::Sidak.is_false_discovery_rate());
        for method in Correction::ALL {
            assert_eq!(Correction::from_name(method.name()), Some(method));
            assert!(correct(method, &[]).is_empty());
        }
    }

    #[test]
    fn storey_q_values() {
        // Uniform p-values: pi_0 is near one and q-values are near BH's.
        let uniform: Vec<f64> = (1..=200).map(|i| i as f64 / 200.0).collect();
        let r = Correction::Storey.correct(&uniform, 0.05, &mut rng_from_seed(3));
        let pi0 = r.info[1].1;
        assert_eq!(r.info[1].0, "Estimated pi_0");
        assert!((0.8..=1.05).contains(&pi0), "pi0 = {pi0}");
        assert!(r.warning.is_none());
        assert_eq!(info(&r, "pi_0 used"), pi0);

        // q-values are monotonic in the p-values and scale with pi_0.
        for pair in r.values.windows(2) {
            assert!(pair[0] <= pair[1] + 1e-15);
        }
        let bh = correct(Correction::BenjaminiHochberg, &uniform);
        assert!((r.values[0] - pi0 * bh[0]).abs() < 1e-12);

        // Same seed, same answer; all-tiny p-values trigger the caution.
        let again = Correction::Storey.correct(&uniform, 0.05, &mut rng_from_seed(3));
        assert_eq!(r, again);
        let tiny = vec![1e-8; 50];
        assert!(Correction::Storey
            .correct(&tiny, 0.05, &mut rng_from_seed(3))
            .warning
            .is_some());
    }

    #[test]
    fn storey_never_estimates_that_no_hypothesis_is_null() {
        // No p-value above 0.95 used to give pi_0 = 0 and every q-value 0,
        // so that every feature was called significant.
        let spread: Vec<f64> = (1..=200).map(|i| i as f64 * 0.9 / 200.0).collect();
        for p_values in [vec![0.01, 0.2, 0.3, 0.4, 0.5], spread] {
            let r = Correction::Storey.correct(&p_values, 0.05, &mut rng_from_seed(3));
            assert_eq!(info(&r, "pi_0 used"), 1.0);
            assert_eq!(r.values, correct(Correction::BenjaminiHochberg, &p_values));
            assert!(r.warning.unwrap().contains("pi_0 = 1"));
        }
    }

    fn info(r: &Corrected, name: &str) -> f64 {
        r.info.iter().find(|(n, _)| *n == name).unwrap().1
    }

    #[test]
    fn an_uncertain_estimate_of_pi_0_is_not_used() {
        // Welch's test of 41 phyla, Core against all other coal seam gas
        // samples: pi_0 was estimated at 0.12 from the 2 p-values above
        // lambda = 0.6, which put 34 phyla below q = 0.05 where
        // Benjamini-Hochberg finds none (R's qvalue bootstrap gives the
        // same estimate; its smoother gives 0.28).
        let coal = COAL_P;
        let r = Correction::Storey.correct(&coal, 0.05, &mut rng_from_seed(1));
        assert!(info(&r, "Estimated pi_0") < 0.3);
        assert_eq!(info(&r, "pi_0 used"), 1.0);
        assert!(info(&r, "Standard error of pi_0") > 0.25 * info(&r, "Estimated pi_0"));
        assert_eq!(r.values, correct(Correction::BenjaminiHochberg, &coal));
        assert_eq!(info(&r, "Number of significant features"), 0.0);
        assert!(r.warning.unwrap().contains("too uncertain"));

        // Many features, a fifth of them truly different: a sound estimate,
        // used, and smaller q-values than Benjamini-Hochberg's.
        let nulls = (0..1600).map(|i| (i as f64 + 0.5) / 1600.0);
        let different = (0..400).map(|i| 1e-4 * (i as f64 + 1.0) / 400.0);
        let many: Vec<f64> = nulls.chain(different).collect();
        let r = Correction::Storey.correct(&many, 0.05, &mut rng_from_seed(1));
        let pi0 = info(&r, "pi_0 used");
        assert!((0.7..=0.9).contains(&pi0), "{pi0}");
        assert_eq!(pi0, info(&r, "Estimated pi_0"));
        assert!(r.warning.is_none());
        let bh = correct(Correction::BenjaminiHochberg, &many);
        assert!(r.values.iter().zip(&bh).all(|(q, b)| q <= b));
    }

    /// The coal seam gas p-values of the test above.
    const COAL_P: [f64; 41] = [
        0.00444526066756679,
        0.010935203872333616,
        0.014735557046800055,
        0.027634552999519935,
        0.029351990796787835,
        0.04151476112595319,
        0.0447927975651355,
        0.06112020363238413,
        0.06584704480206283,
        0.07095636672610536,
        0.07329164817284192,
        0.0774062135375917,
        0.08025387864838035,
        0.09490444987070784,
        0.10993282588118247,
        0.11316239972130138,
        0.12603324423546658,
        0.12937481802189787,
        0.1525215355561158,
        0.15309701458228742,
        0.16906822869642446,
        0.1727585400219187,
        0.1759126187497151,
        0.19112630907174522,
        0.19793982561998635,
        0.19882875403298395,
        0.2322928770619993,
        0.2478083761519806,
        0.2771764980837538,
        0.28260135844250656,
        0.3241743423860389,
        0.3241743423860389,
        0.3241743423860389,
        0.3241743423860389,
        0.35061666282020787,
        0.42034517161675555,
        0.49619719184531397,
        0.5782501344694972,
        0.5851019302733544,
        0.8046407476444865,
        0.9942598100562496,
    ];
}
