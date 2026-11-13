//! Statistical tests, confidence intervals, effect sizes and multiple-test
//! corrections.

pub mod ci;
pub mod correction;
pub mod dist;
pub mod effect_size;
pub mod multi_group;
pub mod post_hoc;
pub mod results;
pub mod table;
pub mod two_group;
pub mod two_sample;

use rand::SeedableRng;

/// Random number generator used by the resampling methods. The same seed
/// gives the same results on every platform.
pub type StampRng = rand_chacha::ChaCha8Rng;

/// Create the random number generator for a user-visible seed.
pub fn rng_from_seed(seed: u64) -> StampRng {
    StampRng::seed_from_u64(seed)
}

/// Settings shared by the statistical methods.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsSettings {
    /// Value added to counts of zero where a ratio would be undefined.
    pub pseudocount: f64,
    /// Number of replicates for bootstrap and permutation methods.
    pub replicates: usize,
}

impl Default for StatsSettings {
    fn default() -> Self {
        StatsSettings {
            pseudocount: 0.5,
            replicates: 1000,
        }
    }
}

/// Whether a hypothesis test reports its one-sided or two-sided p-value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
    OneSided,
    TwoSided,
}

impl TestType {
    pub const ALL: [TestType; 2] = [TestType::OneSided, TestType::TwoSided];

    pub fn name(self) -> &'static str {
        match self {
            TestType::OneSided => "One-sided",
            TestType::TwoSided => "Two-sided",
        }
    }
}

/// Sample variance (n - 1 in the denominator). NaN for fewer than two values.
pub(crate) fn sample_variance(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    values.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1.0)
}

pub(crate) fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Population standard deviation (n in the denominator).
pub(crate) fn population_std(values: &[f64]) -> f64 {
    let m = mean(values);
    (values.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / values.len() as f64).sqrt()
}
