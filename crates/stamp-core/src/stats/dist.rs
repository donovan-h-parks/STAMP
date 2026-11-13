//! Probability distributions used by the statistical tests.
//!
//! Formulas mirror the SciPy calls of the Python implementation so that
//! results agree to within floating-point rounding.

use std::f64::consts::SQRT_2;

use statrs::function::beta::inv_beta_reg;
use statrs::function::gamma::{gamma_lr, gamma_ur};

/// Natural logarithm of the gamma function.
pub fn ln_gamma(x: f64) -> f64 {
    libm::lgamma(x)
}

/// Regularized incomplete beta function I_x(a, b), by continued fraction
/// (modified Lentz algorithm). Accurate to a few units in the last place.
pub fn beta_reg(a: f64, b: f64, x: f64) -> f64 {
    if x.is_nan() || a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let ln_front = ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (-x).ln_1p();
    if x < (a + 1.0) / (a + b + 2.0) {
        ln_front.exp() * beta_continued_fraction(a, b, x) / a
    } else {
        1.0 - ln_front.exp() * beta_continued_fraction(b, a, 1.0 - x) / b
    }
}

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    const EPSILON: f64 = 1e-16;
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..10_000 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let even = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + even * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + even / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;

        let odd = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + odd * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + odd / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < EPSILON {
            break;
        }
    }
    h
}

/// Inverse error function, refined to full precision with Newton's method.
fn erf_inv(y: f64) -> f64 {
    let mut x = statrs::function::erf::erf_inv(y);
    if !x.is_finite() {
        return x;
    }
    for _ in 0..3 {
        let slope = std::f64::consts::FRAC_2_SQRT_PI * (-x * x).exp();
        if slope == 0.0 {
            break;
        }
        x -= (libm::erf(x) - y) / slope;
    }
    x
}

/// Standard normal cumulative distribution function.
pub fn normal_cdf(z: f64) -> f64 {
    0.5 * libm::erfc(-z / SQRT_2)
}

/// Two-sided critical value of the standard normal distribution: the z for
/// which the central `area` lies within ±z.
pub fn z_score(area: f64) -> f64 {
    // Common coverages are tabulated, as in the Python implementation.
    if area == 0.90 {
        1.6448536269514722
    } else if area == 0.95 {
        1.959963984540054
    } else if area == 0.98 {
        2.3263478740408408
    } else if area == 0.99 {
        2.5758293035489004
    } else {
        erf_inv(area) * SQRT_2
    }
}

/// Chi-square cumulative distribution function.
pub fn chi2_cdf(x: f64, df: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x <= 0.0 {
        0.0
    } else if x.is_infinite() {
        1.0
    } else {
        gamma_lr(df / 2.0, x / 2.0)
    }
}

/// Chi-square survival function, `1 - cdf` without cancellation.
pub fn chi2_sf(x: f64, df: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x <= 0.0 {
        1.0
    } else if x.is_infinite() {
        0.0
    } else {
        gamma_ur(df / 2.0, x / 2.0)
    }
}

/// Student's t cumulative distribution function.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    if t.is_nan() || df.is_nan() || df <= 0.0 {
        return f64::NAN;
    }
    if t.is_infinite() {
        return if t > 0.0 { 1.0 } else { 0.0 };
    }
    let tail = 0.5 * beta_reg(df / 2.0, 0.5, df / (df + t * t));
    if t > 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

/// Inverse survival function of Student's t: the t with upper-tail area `q`.
pub fn t_isf(q: f64, df: f64) -> f64 {
    if q.is_nan() || df.is_nan() || df <= 0.0 || !(0.0..=1.0).contains(&q) {
        return f64::NAN;
    }
    if q == 0.5 {
        return 0.0;
    }
    let tail = q.min(1.0 - q);
    if tail == 0.0 {
        return if q < 0.5 {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
    }
    let x = inv_beta_reg(df / 2.0, 0.5, 2.0 * tail);
    let mut t = (df * (1.0 - x) / x).sqrt();
    // Polish the starting value with Newton's method.
    let ln_norm =
        ln_gamma((df + 1.0) / 2.0) - ln_gamma(df / 2.0) - 0.5 * (df * std::f64::consts::PI).ln();
    for _ in 0..6 {
        let upper_tail = 0.5 * beta_reg(df / 2.0, 0.5, df / (df + t * t));
        let density = (ln_norm - (df + 1.0) / 2.0 * (t * t / df).ln_1p()).exp();
        let step = (upper_tail - tail) / density;
        if !step.is_finite() {
            break;
        }
        t += step;
    }
    if q < 0.5 {
        t
    } else {
        -t
    }
}

/// F distribution cumulative distribution function.
pub fn f_cdf(x: f64, dfn: f64, dfd: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x <= 0.0 {
        0.0
    } else if x.is_infinite() {
        1.0
    } else {
        beta_reg(dfn / 2.0, dfd / 2.0, dfn * x / (dfn * x + dfd))
    }
}

/// F distribution survival function.
pub fn f_sf(x: f64, dfn: f64, dfd: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x <= 0.0 {
        1.0
    } else if x.is_infinite() {
        0.0
    } else {
        beta_reg(dfd / 2.0, dfn / 2.0, dfd / (dfd + dfn * x))
    }
}

/// F distribution quantile function.
pub fn f_ppf(p: f64, dfn: f64, dfd: f64) -> f64 {
    if p.is_nan() || !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    let y = inv_beta_reg(dfn / 2.0, dfd / 2.0, p);
    let mut x = dfd * y / (dfn * (1.0 - y));
    if !x.is_finite() || x <= 0.0 {
        return x;
    }
    // Polish the starting value with Newton's method.
    let (a, b) = (dfn / 2.0, dfd / 2.0);
    let ln_beta = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);
    for _ in 0..6 {
        let ln_density =
            a * (dfn / dfd).ln() + (a - 1.0) * x.ln() - (a + b) * (dfn * x / dfd).ln_1p() - ln_beta;
        let step = (f_cdf(x, dfn, dfd) - p) / ln_density.exp();
        if !step.is_finite() || x - step <= 0.0 {
            break;
        }
        x -= step;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
            "expected {expected}, got {actual}"
        );
    }

    // Reference values from the original STAMP unit tests.
    #[test]
    fn normal_distribution() {
        close(normal_cdf(-2.0), 0.022750131948179209, 1e-14);
        close(normal_cdf(-1.0), 0.15865525393145705, 1e-14);
        close(normal_cdf(0.0), 0.5, 1e-15);
        close(normal_cdf(1.0), 0.84134474606854293, 1e-14);
        close(normal_cdf(2.0), 0.97724986805182079, 1e-14);
        close(normal_cdf(-1e-6), 1.0 - normal_cdf(1e-6), 1e-15);

        close(z_score(0.90), 1.6448536269514722, 0.0);
        close(z_score(0.95), 1.959963984540054, 0.0);
        close(z_score(0.80), 1.2815515655446004, 1e-13);
        // The tabulated values agree with the general formula.
        close(erf_inv(0.95) * SQRT_2, z_score(0.95), 1e-13);
        close(erf_inv(0.99) * SQRT_2, z_score(0.99), 1e-13);
    }

    // Reference values from SciPy 1.2.1.
    #[test]
    fn chi_square_distribution() {
        close(chi2_cdf(3.841458820694124, 1.0), 0.9500000000000001, 1e-13);
        close(chi2_cdf(1.0, 1.0), 0.6826894921370859, 1e-13);
        close(chi2_sf(10.0, 3.0), 0.01856613546304325, 1e-13);
        close(chi2_sf(50.0, 2.0) * 1e11, 1.3887943864964, 1e-12);
        assert_eq!(chi2_cdf(0.0, 1.0), 0.0);
        assert_eq!(chi2_cdf(f64::INFINITY, 1.0), 1.0);
    }

    // SciPy 1.2's quantile functions are themselves only good to about 1e-9
    // relative error, so quantiles are compared more loosely than CDFs.
    #[test]
    fn t_distribution() {
        close(t_cdf(1.0, 5.0), 0.8183912661754387, 1e-13);
        close(t_cdf(-2.5, 9.3), 0.016538905157534048, 1e-13);
        close(t_cdf(0.0, 3.0), 0.5, 1e-15);
        close(t_isf(0.025, 10.0), 2.2281388519649385, 2e-9);
        close(t_isf(0.025, 9.3), 2.2510822246423188, 2e-9);
        close(t_isf(0.005, 2.5), 7.163728134021242, 2e-9);
        close(t_isf(0.975, 10.0), -2.2281388519649385, 2e-9);
        // Quantiles invert the CDF to full precision.
        for (q, df) in [(0.025, 10.0), (0.005, 2.5), (0.05, 200.0), (0.3, 1.0)] {
            close(1.0 - t_cdf(t_isf(q, df), df), q, 1e-14);
        }
    }

    #[test]
    fn f_distribution() {
        close(f_cdf(2.0, 3.0, 10.0), 0.8219925926248246, 1e-13);
        close(f_sf(2.0, 3.0, 10.0), 0.17800740737517542, 1e-13);
        close(f_sf(25.0, 2.0, 15.0) * 1e5, 1.6743038199853825, 1e-12);
        close(f_ppf(0.95, 2.0, 15.0), 3.6823203436732412, 2e-9);
        close(f_ppf(0.95, 5.0, 114.0), 2.2939111578278037, 2e-9);
        assert_eq!(f_sf(f64::INFINITY, 2.0, 5.0), 0.0);
        for (p, dfn, dfd) in [(0.95, 2.0, 15.0), (0.99, 5.0, 114.0), (0.5, 1.0, 3.0)] {
            close(f_cdf(f_ppf(p, dfn, dfd), dfn, dfd), p, 1e-14);
        }
    }
}
