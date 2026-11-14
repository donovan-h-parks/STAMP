//! Confidence intervals for the effect size between two samples.

use super::dist::z_score;
use super::wilson::wilson_interval;

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
                let std_err = (r1 * (1.0 - r1) / total1 + r2 * (1.0 - r2) / total2).sqrt();
                let mut offset = z * std_err;
                if self == SampleCi::DiffBetweenPropAsymptoticCc {
                    // The continuity correction widens the interval by
                    // (1/n1 + 1/n2) / 2 on each side (Fleiss); STAMP v2
                    // multiplied it by z as well.
                    offset += (1.0 / total1 + 1.0 / total2) / 2.0;
                }
                CiOutcome {
                    lower: (diff - offset) * 100.0,
                    upper: (diff + offset) * 100.0,
                    effect_size: diff * 100.0,
                    note,
                }
            }
            SampleCi::NewcombeWilson => {
                let (total1, total2, note) = replace_zero_totals(total1, total2, pseudocount);
                // Newcombe's hybrid score interval (his method 10) from the
                // Wilson score limits of each proportion.
                let (low1, high1, _) = wilson_interval(seq1, total1, z);
                let (low2, high2, _) = wilson_interval(seq2, total2, z);
                let (roots1, roots2) = ([low1, high1], [low2, high2]);
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

#[cfg(test)]
mod tests {
    use super::*;

    // Tables as (feature in sample 1, feature in sample 2, total of sample
    // 1, total of sample 2). Reference values are from R 4.4.3 with
    // DescTools 0.99.59, at 95% coverage:
    // - `BinomDiffCI(s1, t1, s2, t2, method = "wald")`, `"waldcc"` and
    //   `"score"` (Newcombe's hybrid score interval, his method 10), in
    //   percent;
    // - `OddsRatio(matrix(c(s1, t1 - s1, s2, t2 - s2), 2), method = "wald")`
    //   (Woolf's logit interval);
    // - `BinomRatioCI(s1, t1, s2, t2, method = "katz.log")`.
    const TABLE1: [f64; 4] = [10.0, 8.0, 30.0, 40.0];
    const TABLE2: [f64; 4] = [4000.0, 5000.0, 500000.0, 1000000.0];
    const TABLE3: [f64; 4] = [3.0, 1.0, 4.0, 4.0];

    fn check(ci: SampleCi, t: [f64; 4], effect: f64, lower: f64, upper: f64) {
        let r = ci.run(t[0], t[1], t[2], t[3], 0.95, 0.5);
        for (actual, expected) in [(r.lower, lower), (r.upper, upper), (r.effect_size, effect)] {
            assert!(
                (actual - expected).abs() <= 1e-9 * expected.abs() + 1e-12,
                "{ci:?} {t:?}: expected {expected}, got {actual}"
            );
        }
        assert_eq!(r.note, "");
    }

    #[test]
    fn difference_between_proportions_intervals() {
        use SampleCi::*;
        let percent = 100.0;
        for (ci, t, effect, lower, upper) in [
            (
                DiffBetweenPropAsymptotic,
                TABLE1,
                0.1333333333333333,
                -0.076001531909981268,
                0.34266819857664788,
            ),
            (
                DiffBetweenPropAsymptotic,
                TABLE2,
                0.003,
                0.0027170107916633388,
                0.0032829892083366613,
            ),
            (
                DiffBetweenPropAsymptoticCc,
                TABLE1,
                0.1333333333333333,
                -0.10516819857664794,
                0.37183486524331455,
            ),
            (
                DiffBetweenPropAsymptoticCc,
                TABLE2,
                0.003,
                0.0027155107916633386,
                0.0032844892083366615,
            ),
            (
                NewcombeWilson,
                TABLE1,
                0.1333333333333333,
                -0.070791151400971947,
                0.33586263792069881,
            ),
            (
                NewcombeWilson,
                TABLE2,
                0.003,
                0.002719327579401225,
                0.0032854107711739601,
            ),
            (
                NewcombeWilson,
                TABLE3,
                0.5,
                -0.13548840058295131,
                0.78908326808475571,
            ),
        ] {
            check(ci, t, effect * percent, lower * percent, upper * percent);
        }
        // The Wald intervals are not clipped to [-1, 1], unlike R's.
        let r = DiffBetweenPropAsymptotic.run(3.0, 1.0, 4.0, 4.0, 0.95, 0.5);
        assert!((r.lower - -10.011395954441349).abs() < 1e-9);
        assert!(r.upper > 100.0);
    }

    #[test]
    fn ratio_intervals() {
        use SampleCi::*;
        check(
            OddsRatio,
            TABLE1,
            2.0,
            0.67604602159559535,
            5.916756954739931,
        );
        check(
            OddsRatio,
            TABLE2,
            1.6048387096774201,
            1.5392677405941568,
            1.6732029238038484,
        );
        check(
            OddsRatio,
            TABLE3,
            9.0,
            0.36663693192554586,
            220.92700692915727,
        );
        check(
            RatioProportions,
            TABLE1,
            1.6666666666666665,
            0.74876782589755575,
            3.7097985272644776,
        );
        check(
            RatioProportions,
            TABLE2,
            1.6,
            1.535053657807854,
            1.6676941467022264,
        );
        check(
            RatioProportions,
            TABLE3,
            3.0,
            0.50128430938694102,
            17.953883318244671,
        );
    }

    /// With a zero cell, 0.5 is added to every cell (the Haldane-Anscombe
    /// correction), and the usual intervals are taken of the result: Woolf's
    /// for the odds ratio and Katz's for the ratio of proportions. Checked
    /// against the same R functions on the corrected tables:
    /// `OddsRatio(matrix(c(0.5, 10.5, 5.5, 5.5), 2), method = "wald")` and
    /// `BinomRatioCI(0.5, 11, 5.5, 11, method = "katz.log")`, computed here
    /// from their closed forms.
    #[test]
    fn zero_counts_use_the_pseudocount() {
        let r = SampleCi::OddsRatio.run(0.0, 5.0, 10.0, 10.0, 0.95, 0.5);
        assert_eq!(r.note, PSEUDOCOUNT_USED);
        let (a, b, c, d) = (0.5f64, 5.5f64, 10.5f64, 5.5f64);
        let or = a * d / (b * c);
        let se = (1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d).sqrt();
        let z = 1.959963984540054;
        assert!((r.effect_size - or).abs() < 1e-12);
        assert!((r.lower - (or.ln() - z * se).exp()).abs() < 1e-12);
        assert!((r.upper - (or.ln() + z * se).exp()).abs() < 1e-12);

        let r = SampleCi::RatioProportions.run(0.0, 5.0, 10.0, 10.0, 0.95, 0.5);
        let rr = (0.5 / 11.0) / (5.5 / 11.0);
        let se = (1.0 / 0.5 - 1.0 / 11.0 + 1.0 / 5.5 - 1.0 / 11.0f64).sqrt();
        assert!((r.effect_size - rr).abs() < 1e-12);
        assert!((r.lower - (rr.ln() - z * se).exp()).abs() < 1e-12);

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
