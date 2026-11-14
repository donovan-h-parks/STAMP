//! Correlation between the proportion of a feature and a number known for
//! each sample.

use super::dist::t_cdf;
use super::mean;

/// Measures of correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorrelationMethod {
    /// Pearson's r: how close the points are to a straight line.
    Pearson,
    /// Spearman's rho: Pearson's r of the ranks, so any relationship that
    /// only ever rises or only ever falls counts in full, and outlying
    /// values count for no more than their rank.
    Spearman,
}

impl CorrelationMethod {
    pub const ALL: [CorrelationMethod; 2] =
        [CorrelationMethod::Pearson, CorrelationMethod::Spearman];

    pub fn name(self) -> &'static str {
        match self {
            CorrelationMethod::Pearson => "Pearson correlation",
            CorrelationMethod::Spearman => "Spearman rank correlation",
        }
    }

    pub fn from_name(name: &str) -> Option<CorrelationMethod> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    /// Symbol of the coefficient.
    pub fn coefficient_name(self) -> &'static str {
        match self {
            CorrelationMethod::Pearson => "r",
            CorrelationMethod::Spearman => "rho",
        }
    }

    /// The coefficient for paired values, from -1 to 1. `None` if there
    /// are fewer than three pairs or either set of values is constant, when
    /// no correlation is defined.
    pub fn coefficient(self, x: &[f64], y: &[f64]) -> Option<f64> {
        match self {
            CorrelationMethod::Pearson => pearson(x, y),
            CorrelationMethod::Spearman => pearson(&ranks(x), &ranks(y)),
        }
    }
}

/// Ranks of values from 1, ties sharing the average of the ranks they span.
pub fn ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && values[order[end]] == values[order[start]] {
            end += 1;
        }
        // Ranks start + 1 to end.
        let shared = (start + end + 1) as f64 / 2.0;
        for &index in &order[start..end] {
            ranks[index] = shared;
        }
        start = end;
    }
    ranks
}

/// Pearson's correlation coefficient; see
/// [`CorrelationMethod::coefficient`].
pub fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 3 {
        return None;
    }
    let (mean_x, mean_y) = (mean(x), mean(y));
    let (mut xx, mut yy, mut xy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        let (dx, dy) = (a - mean_x, b - mean_y);
        xx += dx * dx;
        yy += dy * dy;
        xy += dx * dy;
    }
    if xx <= 0.0 || yy <= 0.0 {
        return None;
    }
    Some((xy / (xx * yy).sqrt()).clamp(-1.0, 1.0))
}

/// Two-sided p-value of a correlation coefficient from `n` pairs, against
/// no correlation: the coefficient is turned into a t statistic with n - 2
/// degrees of freedom. This is exact for Pearson's r when the values are
/// normally distributed, and the usual approximation for Spearman's rho
/// (as in SciPy).
pub fn correlation_p_value(coefficient: f64, n: usize) -> f64 {
    if n < 3 {
        return 1.0;
    }
    let dof = n as f64 - 2.0;
    let remainder = 1.0 - coefficient * coefficient;
    if remainder <= 0.0 {
        return 0.0;
    }
    let t = coefficient.abs() * (dof / remainder).sqrt();
    (2.0 * t_cdf(-t, dof)).clamp(0.0, 1.0)
}

/// The straight line through points that leaves the smallest squared
/// distances in y: its slope and its value at x = 0. `None` if there are
/// fewer than two points or x is constant.
pub fn least_squares(x: &[f64], y: &[f64]) -> Option<(f64, f64)> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }
    let (mean_x, mean_y) = (mean(x), mean(y));
    let (mut xx, mut xy) = (0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        xx += (a - mean_x) * (a - mean_x);
        xy += (a - mean_x) * (b - mean_y);
    }
    if xx <= 0.0 {
        return None;
    }
    let slope = xy / xx;
    Some((slope, mean_y - slope * mean_x))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    // Reference values are from SciPy 1.17 (`pearsonr`, `spearmanr`,
    // `linregress`).
    const X: [f64; 10] = [1.5, 2.0, 3.5, 4.0, 5.5, 6.0, 7.5, 9.0, 9.5, 12.0];
    const Y: [f64; 10] = [0.12, 0.30, 0.25, 0.41, 0.18, 0.52, 0.35, 0.48, 0.61, 0.56];

    #[test]
    fn pearson_matches_scipy() {
        let r = CorrelationMethod::Pearson.coefficient(&X, &Y).unwrap();
        close(r, 0.7792411393358121);
        close(correlation_p_value(r, X.len()), 0.007883406586610691);
    }

    #[test]
    fn spearman_matches_scipy() {
        let rho = CorrelationMethod::Spearman.coefficient(&X, &Y).unwrap();
        close(rho, 0.8303030303030302);
        close(correlation_p_value(rho, X.len()), 0.002940227023279509);
        // With tied values.
        let tied = [0.1, 0.1, 0.3, 0.3, 0.3, 0.5, 0.2, 0.2, 0.9, 0.4];
        let rho = CorrelationMethod::Spearman.coefficient(&X, &tied).unwrap();
        close(rho, 0.6235637203686687);
        close(correlation_p_value(rho, X.len()), 0.05405599414751947);
    }

    #[test]
    fn ranks_share_ties() {
        assert_eq!(ranks(&[3.0, 1.0, 3.0, 2.0]), [3.5, 1.0, 3.5, 2.0]);
        assert!(ranks(&[]).is_empty());
    }

    #[test]
    fn degenerate_cases() {
        // Too few pairs, or nothing varying.
        assert_eq!(pearson(&[1.0, 2.0], &[1.0, 2.0]), None);
        assert_eq!(pearson(&[1.0, 2.0, 3.0], &[5.0, 5.0, 5.0]), None);
        assert_eq!(pearson(&[1.0, 2.0, 3.0], &[1.0, 2.0]), None);
        // A perfect line.
        let r = pearson(&[1.0, 2.0, 3.0, 4.0], &[2.0, 4.0, 6.0, 8.0]).unwrap();
        close(r, 1.0);
        assert_eq!(correlation_p_value(r, 4), 0.0);
        assert_eq!(correlation_p_value(0.5, 2), 1.0);
        close(correlation_p_value(0.0, 10), 1.0);
    }

    #[test]
    fn least_squares_matches_scipy() {
        let (slope, intercept) = least_squares(&X, &Y).unwrap();
        close(slope, 0.0376182631207343);
        close(intercept, 0.15040950811955747);
        assert_eq!(least_squares(&[2.0, 2.0], &[1.0, 3.0]), None);
    }

    #[test]
    fn names() {
        for method in CorrelationMethod::ALL {
            assert_eq!(CorrelationMethod::from_name(method.name()), Some(method));
        }
    }
}
