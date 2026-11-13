//! Confidence intervals for the effect size between two samples.

use super::dist::z_score;

/// A confidence interval with its point estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct CiOutcome {
    pub lower: f64,
    pub upper: f64,
    pub effect_size: f64,
    pub note: &'static str,
}

/// Confidence-interval methods for two samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleCi {
    DiffBetweenPropAsymptotic,
    DiffBetweenPropAsymptoticCc,
    NewcombeWilson,
    OddsRatio,
    RatioProportions,
}

const PSEUDOCOUNT_USED: &str = "degenerate case: CI calculation used pseudocount";

impl SampleCi {
    pub const ALL: [SampleCi; 5] = [
        SampleCi::DiffBetweenPropAsymptotic,
        SampleCi::DiffBetweenPropAsymptoticCc,
        SampleCi::NewcombeWilson,
        SampleCi::OddsRatio,
        SampleCi::RatioProportions,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SampleCi::DiffBetweenPropAsymptotic => "DP: Asymptotic",
            SampleCi::DiffBetweenPropAsymptoticCc => "DP: Asymptotic-CC",
            SampleCi::NewcombeWilson => "DP: Newcombe-Wilson",
            SampleCi::OddsRatio => "OR: Haldane adjustment",
            SampleCi::RatioProportions => "RP: Asymptotic",
        }
    }

    pub fn from_name(name: &str) -> Option<SampleCi> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }

    /// Axis label for plots of this effect size.
    pub fn plot_label(self) -> &'static str {
        match self {
            SampleCi::OddsRatio => "Odds ratio",
            SampleCi::RatioProportions => "Ratio of proportions",
            _ => "Difference between proportions (%)",
        }
    }

    /// True if the effect size is a ratio (no effect at 1) rather than a
    /// difference (no effect at 0).
    pub fn is_ratio(self) -> bool {
        matches!(self, SampleCi::OddsRatio | SampleCi::RatioProportions)
    }

    /// Confidence interval with the given coverage (for example 0.95).
    /// Differences between proportions are reported in percent.
    pub fn run(
        self,
        seq1: f64,
        seq2: f64,
        total1: f64,
        total2: f64,
        coverage: f64,
        pseudocount: f64,
    ) -> CiOutcome {
        let z = z_score(coverage);
        match self {
            SampleCi::DiffBetweenPropAsymptotic | SampleCi::DiffBetweenPropAsymptoticCc => {
                let (total1, total2, note) = replace_zero_totals(total1, total2, pseudocount);
                let (r1, r2) = (seq1 / total1, seq2 / total2);
                let diff = r1 - r2;
                let mut std_err = (r1 * (1.0 - r1) / total1 + r2 * (1.0 - r2) / total2).sqrt();
                if self == SampleCi::DiffBetweenPropAsymptoticCc {
                    std_err += (1.0 / total1 + 1.0 / total2) / 2.0;
                }
                let offset = z * std_err;
                CiOutcome {
                    lower: (diff - offset) * 100.0,
                    upper: (diff + offset) * 100.0,
                    effect_size: diff * 100.0,
                    note,
                }
            }
            SampleCi::NewcombeWilson => {
                let (total1, total2, note) = replace_zero_totals(total1, total2, pseudocount);
                let roots1 = newcombe_wilson_roots(seq1, total1, z);
                let roots2 = newcombe_wilson_roots(seq2, total2, z);
                let diff = seq1 / total1 - seq2 / total2;
                let lower = z
                    * (roots1[0] * (1.0 - roots1[0]) / total1
                        + roots2[1] * (1.0 - roots2[1]) / total2)
                        .sqrt();
                let upper = z
                    * (roots1[1] * (1.0 - roots1[1]) / total1
                        + roots2[0] * (1.0 - roots2[0]) / total2)
                        .sqrt();
                CiOutcome {
                    lower: (diff - lower) * 100.0,
                    upper: (diff + upper) * 100.0,
                    effect_size: diff * 100.0,
                    note,
                }
            }
            SampleCi::OddsRatio => {
                let (mut a, mut b) = (seq1, seq2);
                let (mut c, mut d) = (total1 - seq1, total2 - seq2);
                let mut note = "";
                if a == 0.0 || b == 0.0 || c == 0.0 || d == 0.0 {
                    a += pseudocount;
                    b += pseudocount;
                    c += pseudocount;
                    d += pseudocount;
                    note = PSEUDOCOUNT_USED;
                }
                let effect_size = (a * d) / (b * c);
                let log_se = (1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d).sqrt();
                log_interval(effect_size, z * log_se, note)
            }
            SampleCi::RatioProportions => {
                let (mut seq1, mut seq2, mut total1, mut total2) = (seq1, seq2, total1, total2);
                let mut note = "";
                if seq1 == 0.0 || seq2 == 0.0 {
                    seq1 += pseudocount;
                    seq2 += pseudocount;
                    total1 += 2.0 * pseudocount;
                    total2 += 2.0 * pseudocount;
                    note = PSEUDOCOUNT_USED;
                }
                let effect_size = (seq1 / total1) / (seq2 / total2);
                let log_se = (1.0 / seq1 - 1.0 / total1 + 1.0 / seq2 - 1.0 / total2).sqrt();
                log_interval(effect_size, z * log_se, note)
            }
        }
    }
}

fn replace_zero_totals(total1: f64, total2: f64, pseudocount: f64) -> (f64, f64, &'static str) {
    let mut note = "";
    let mut fix = |total: f64| {
        if total == 0.0 {
            note = PSEUDOCOUNT_USED;
            pseudocount
        } else {
            total
        }
    };
    let (t1, t2) = (fix(total1), fix(total2));
    (t1, t2, note)
}

/// Interval that is symmetric on the log scale.
fn log_interval(effect_size: f64, log_offset: f64, note: &'static str) -> CiOutcome {
    let log_effect = effect_size.ln();
    CiOutcome {
        lower: (log_effect - log_offset).exp(),
        upper: (log_effect + log_offset).exp(),
        effect_size,
        note,
    }
}

/// Lower and upper limits of the Wilson score interval for one proportion,
/// found by scanning for sign changes as the Python implementation does.
fn newcombe_wilson_roots(seq: f64, total: f64, z: f64) -> [f64; 2] {
    let estimate = seq / total;
    let f = |value: f64| z * (value * (1.0 - value) / total).sqrt() - (value - estimate).abs();

    let step = 1.0 / total.max(1000.0);
    let steps = (1.0 / step) as i64;
    let mut value: f64 = 0.0;
    let mut previous_p = f(value);
    let mut previous_value = value;
    let mut roots = Vec::with_capacity(2);
    for _ in 0..steps {
        let p = f(value);
        if p * previous_p < 0.0 || (p == 0.0 && value == 0.0) || (p == 0.0 && value == 1.0) {
            let root = if p.abs() + previous_p.abs() != 0.0 {
                previous_value + step * (1.0 - p.abs() / (p.abs() + previous_p.abs()))
            } else {
                previous_value
            };
            roots.push(root);
            if roots.len() == 2 {
                break;
            }
        }
        previous_p = p;
        previous_value = value;
        value += step;
    }
    match roots.len() {
        0 => [estimate, estimate],
        1 => [roots[0], roots[0]],
        _ => [roots[0], roots[1]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values from the original STAMP unit tests.
    const TABLE1: [f64; 4] = [10.0, 8.0, 30.0, 40.0];
    const TABLE2: [f64; 4] = [4000.0, 5000.0, 500000.0, 1000000.0];

    fn check(ci: SampleCi, t: [f64; 4], lower: f64, upper: f64, effect: f64) {
        let r = ci.run(t[0], t[1], t[2], t[3], 0.95, 0.5);
        for (actual, expected) in [(r.lower, lower), (r.upper, upper), (r.effect_size, effect)] {
            assert!(
                (actual - expected).abs() < 5e-8,
                "{ci:?}: expected {expected}, got {actual}"
            );
        }
        assert_eq!(r.note, "");
    }

    #[test]
    fn difference_between_proportions_intervals() {
        use SampleCi::*;
        check(
            DiffBetweenPropAsymptotic,
            TABLE1,
            -7.60015319099813,
            34.2668198576648,
            13.333333333,
        );
        check(
            DiffBetweenPropAsymptotic,
            TABLE2,
            0.271701079166334,
            0.328298920833666,
            0.3,
        );
        check(
            DiffBetweenPropAsymptoticCc,
            TABLE1,
            -13.3167148125733,
            39.98338147924,
            13.333333333,
        );
        check(
            DiffBetweenPropAsymptoticCc,
            TABLE2,
            0.271407084568653,
            0.328592915431347,
            0.3,
        );
        check(
            NewcombeWilson,
            TABLE1,
            -7.07911677674112,
            33.5862638376494,
            13.333333333,
        );
        check(
            NewcombeWilson,
            TABLE2,
            0.271932757939523,
            0.328541077116921,
            0.3,
        );
    }

    #[test]
    fn ratio_intervals() {
        use SampleCi::*;
        check(OddsRatio, TABLE1, 0.676046021596, 5.91675695474, 2.0);
        check(
            OddsRatio,
            TABLE2,
            1.53926774059,
            1.6732029238,
            1.60483870968,
        );
        check(
            RatioProportions,
            TABLE1,
            0.748767825898,
            3.70979852726,
            1.66666666666666,
        );
        check(RatioProportions, TABLE2, 1.53505365781, 1.6676941467, 1.6);
    }

    #[test]
    fn zero_counts_use_the_pseudocount() {
        for ci in SampleCi::ALL {
            let r = ci.run(0.0, 5.0, 0.0, 10.0, 0.95, 0.5);
            assert_eq!(r.note, PSEUDOCOUNT_USED, "{ci:?}");
            assert!(r.lower.is_finite() && r.upper.is_finite(), "{ci:?}");
            assert!(
                r.lower <= r.effect_size && r.effect_size <= r.upper,
                "{ci:?}"
            );
            assert_eq!(SampleCi::from_name(ci.name()), Some(ci));
        }
    }
}
