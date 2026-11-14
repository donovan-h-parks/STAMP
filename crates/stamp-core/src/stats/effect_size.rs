//! Effect-size measures used to filter features.

/// Effect sizes between two samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleEffectSize {
    DiffBetweenProp,
    OddsRatio,
    RatioProportions,
}

impl SampleEffectSize {
    pub const ALL: [SampleEffectSize; 3] = [
        SampleEffectSize::DiffBetweenProp,
        SampleEffectSize::OddsRatio,
        SampleEffectSize::RatioProportions,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SampleEffectSize::DiffBetweenProp => "Difference between proportions",
            SampleEffectSize::OddsRatio => "Odds ratio",
            SampleEffectSize::RatioProportions => "Ratio of proportions",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }

    /// True if the measure is a ratio, best shown on a logarithmic scale.
    pub fn is_ratio(self) -> bool {
        self != SampleEffectSize::DiffBetweenProp
    }

    /// Effect size of sample 1 relative to sample 2. The difference between
    /// proportions is in percent.
    pub fn run(self, seq1: f64, seq2: f64, total1: f64, total2: f64, pseudocount: f64) -> f64 {
        match self {
            SampleEffectSize::DiffBetweenProp => {
                (seq1 / total1.max(1.0) - seq2 / total2.max(1.0)) * 100.0
            }
            SampleEffectSize::OddsRatio => {
                let (mut a, mut b) = (seq1, seq2);
                let (mut c, mut d) = (total1 - seq1, total2 - seq2);
                if a == 0.0 || b == 0.0 || c == 0.0 || d == 0.0 {
                    a += pseudocount;
                    b += pseudocount;
                    c += pseudocount;
                    d += pseudocount;
                }
                (a * d) / (b * c)
            }
            SampleEffectSize::RatioProportions => {
                let (mut seq1, mut seq2, mut total1, mut total2) = (seq1, seq2, total1, total2);
                // As the confidence interval does: the pseudocount is added
                // to both cells of each sample (feature and the rest), so
                // each total grows by twice the pseudocount.
                if seq1 == 0.0 || seq2 == 0.0 {
                    seq1 += pseudocount;
                    seq2 += pseudocount;
                    total1 += 2.0 * pseudocount;
                    total2 += 2.0 * pseudocount;
                }
                (seq1 / total1) / (seq2 / total2)
            }
        }
    }

    /// Effect size regardless of direction: the larger of sample 1 against
    /// sample 2 and sample 2 against sample 1. This is the value that filters
    /// compare with their threshold.
    pub fn magnitude(
        self,
        seq1: f64,
        seq2: f64,
        total1: f64,
        total2: f64,
        pseudocount: f64,
    ) -> f64 {
        self.run(seq1, seq2, total1, total2, pseudocount)
            .max(self.run(seq2, seq1, total2, total1, pseudocount))
    }
}

/// Effect sizes between two groups of samples, computed from the relative
/// frequencies (%) of a feature in each sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupEffectSize {
    DiffBetweenProp,
    RatioProportions,
}

impl GroupEffectSize {
    pub const ALL: [GroupEffectSize; 2] = [
        GroupEffectSize::DiffBetweenProp,
        GroupEffectSize::RatioProportions,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GroupEffectSize::DiffBetweenProp => "Difference between proportions",
            GroupEffectSize::RatioProportions => "Ratio of proportions",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }

    pub fn is_ratio(self) -> bool {
        self == GroupEffectSize::RatioProportions
    }

    /// Effect size of group 1 relative to group 2.
    pub fn run(self, group1: &[f64], group2: &[f64], pseudocount: f64) -> f64 {
        let mean = |g: &[f64]| {
            if g.is_empty() {
                0.0
            } else {
                g.iter().sum::<f64>() / g.len() as f64
            }
        };
        let (mut mean1, mut mean2) = (mean(group1), mean(group2));
        match self {
            GroupEffectSize::DiffBetweenProp => mean1 - mean2,
            GroupEffectSize::RatioProportions => {
                if mean1 == 0.0 && mean2 == 0.0 {
                    // Absent from both groups: no difference.
                    return 1.0;
                }
                if mean1 == 0.0 || mean2 == 0.0 {
                    let adjustment = pseudocount / (mean1 + mean2);
                    mean1 += adjustment;
                    mean2 += adjustment;
                }
                mean1 / mean2
            }
        }
    }

    /// Effect size regardless of direction; see [`SampleEffectSize::magnitude`].
    pub fn magnitude(self, group1: &[f64], group2: &[f64], pseudocount: f64) -> f64 {
        self.run(group1, group2, pseudocount)
            .max(self.run(group2, group1, pseudocount))
    }
}

/// Eta-squared: the proportion of variance explained by group membership.
/// Returns -1 when it is undefined (an empty group, or no variation at all).
pub fn eta_squared(groups: &[&[f64]]) -> f64 {
    if groups.iter().any(|g| g.is_empty()) {
        return -1.0;
    }
    let n: f64 = groups.iter().map(|g| g.len() as f64).sum();
    let total_sum: f64 = groups.iter().flat_map(|g| g.iter()).sum();
    let grand_mean = total_sum / n;

    let mut total_ss = 0.0;
    let mut between_ss = 0.0;
    for group in groups {
        let mut group_sum = 0.0;
        for &x in group.iter() {
            total_ss += (x - grand_mean) * (x - grand_mean);
            group_sum += x;
        }
        between_ss += group_sum * group_sum / group.len() as f64;
    }
    between_ss -= total_sum * total_sum / n;

    if total_ss != 0.0 {
        between_ss / total_ss
    } else {
        -1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
            "expected {expected}, got {actual}"
        );
    }

    /// Reference values from R 4.4.3 with DescTools 0.99.59 for the tables
    /// (feature in sample 1, feature in sample 2, total 1, total 2):
    /// `BinomDiffCI(...)[1]` for the difference between proportions (in
    /// percent), `OddsRatio(matrix(c(s1, t1 - s1, s2, t2 - s2), 2))` and
    /// `BinomRatioCI(s1, t1, s2, t2)[1]` for the ratio of proportions.
    #[test]
    fn sample_effect_sizes() {
        use SampleEffectSize::*;
        let table1 = (10.0, 8.0, 30.0, 40.0);
        let table2 = (4000.0, 5000.0, 500000.0, 1000000.0);
        let run = |e: SampleEffectSize, t: (f64, f64, f64, f64)| e.run(t.0, t.1, t.2, t.3, 0.5);
        close(run(DiffBetweenProp, table1), 13.333333333333330);
        close(run(DiffBetweenProp, table2), 0.30000000000000001);
        close(run(OddsRatio, table1), 2.0);
        close(run(OddsRatio, table2), 1.6048387096774201);
        close(run(RatioProportions, table1), 1.6666666666666665);
        close(run(RatioProportions, table2), 1.6);
        // The magnitude is the same whichever sample is first.
        close(
            RatioProportions.magnitude(8.0, 10.0, 40.0, 30.0, 0.5),
            1.6666666666666665,
        );
        close(
            DiffBetweenProp.magnitude(8.0, 10.0, 40.0, 30.0, 0.5),
            13.333333333333330,
        );
        // A zero cell: 0.5 is added to every cell, as for the confidence
        // intervals (the Haldane-Anscombe correction), so the totals grow
        // by one: (0.5 / 11) / (5.5 / 11).
        close(RatioProportions.run(0.0, 5.0, 10.0, 10.0, 0.5), 0.5 / 5.5);
        close(
            OddsRatio.run(0.0, 5.0, 10.0, 10.0, 0.5),
            0.5 * 5.5 / (5.5 * 10.5),
        );
    }

    #[test]
    fn group_effect_sizes() {
        use GroupEffectSize::*;
        let (a, b) = ([1.0, 2.0, 3.0, 4.0, 5.0], [2.0, 4.0, 5.0, 8.0, 10.0]);
        close(DiffBetweenProp.run(&a, &b, 0.5), 15.0 / 5.0 - 29.0 / 5.0);
        close(DiffBetweenProp.run(&[1.0], &[1.0, 1.0], 0.5), 0.0);
        close(
            RatioProportions.run(&a, &b, 0.5),
            (15.0 / 5.0) / (29.0 / 5.0),
        );
        close(RatioProportions.run(&[1.0], &[1.0, 1.0], 0.5), 1.0);
    }

    #[test]
    fn ratio_of_a_feature_absent_from_both_groups_is_one() {
        let zero = [0.0, 0.0];
        assert_eq!(
            GroupEffectSize::RatioProportions.run(&zero, &zero, 0.5),
            1.0
        );
        assert_eq!(
            GroupEffectSize::RatioProportions.magnitude(&zero, &zero, 0.5),
            1.0
        );
        assert!(GroupEffectSize::RatioProportions
            .run(&zero, &[2.0, 2.0], 0.5)
            .is_finite());
    }

    /// Reference values from R 4.4.3 with DescTools 0.99.59:
    /// `EtaSq(aov(y ~ g))`, the between-group sum of squares over the total.
    #[test]
    fn eta_squared_values() {
        let g = |v: &'static [f64]| v;
        close(
            eta_squared(&[
                g(&[1.0, 2.0, 3.0, 4.0]),
                g(&[2.0, 3.0, 4.0]),
                g(&[1.0, 2.0, 3.0, 4.0]),
            ]),
            0.04347826086956532,
        );
        close(
            eta_squared(&[
                g(&[1.0, 2.0, 3.0, 4.0, 5.0]),
                g(&[10.0, 20.0, 30.0, 40.0, 50.0]),
                g(&[4.0, 5.0, 4.0]),
                g(&[5.0, 5.0, 5.0]),
            ]),
            0.69911120207903055,
        );
        close(
            eta_squared(&[
                g(&[5.0, 4.0, 6.0, 4.0, 3.0]),
                g(&[5.0, 2.0, 2.0, 5.0, 6.0, 7.0]),
                g(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]),
            ]),
            0.016383616383616389,
        );
        assert_eq!(eta_squared(&[g(&[1.0, 1.0]), g(&[1.0])]), -1.0);
        assert_eq!(eta_squared(&[g(&[1.0, 2.0]), g(&[])]), -1.0);
    }
}
