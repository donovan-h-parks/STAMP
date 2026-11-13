//! Wilson score interval for a single proportion.

/// Wilson score interval for `positive` out of `total`, for the normal
/// critical value `z` (1.96 for 95% coverage). Returns the lower limit, the
/// upper limit and the observed proportion. A total below one is treated as
/// one.
pub fn wilson_interval(positive: f64, total: f64, z: f64) -> (f64, f64, f64) {
    let total = total.max(1.0);
    let p = positive / total;
    let z2 = z * z;
    let centre = p + z2 / (2.0 * total);
    let offset = z * (p * (1.0 - p) / total + z2 / (4.0 * total * total)).sqrt();
    let denominator = 1.0 + z2 / total;
    (
        (centre - offset) / denominator,
        (centre + offset) / denominator,
        p,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_textbook_example() {
        // 10 of 30 at 95%: binom::confint(10, 30, methods = "wilson") in R.
        let (low, high, p) = wilson_interval(10.0, 30.0, 1.959963984540054);
        assert!((p - 1.0 / 3.0).abs() < 1e-15);
        assert!((low - 0.1923050).abs() < 1e-6, "{low}");
        assert!((high - 0.5121995).abs() < 1e-6, "{high}");
        let (low, high, p) = wilson_interval(0.0, 0.0, 1.96);
        assert!(p == 0.0 && low.abs() < 1e-15 && high > 0.0 && high < 1.0);
    }
}
