//! Probability distributions used by the statistical tests.
//!
//! Formulas mirror the SciPy calls of the Python implementation so that
//! results agree to within floating-point rounding.

use std::f64::consts::SQRT_2;

use statrs::function::beta::inv_beta_reg as statrs_inv_beta_reg;
use statrs::function::gamma::{gamma_lr, gamma_ur};

/// Natural logarithm of the gamma function.
pub fn ln_gamma(x: f64) -> f64 {
    libm::lgamma(x)
}

/// Regularized incomplete beta function I_x(a, b), by continued fraction
/// (modified Lentz algorithm). Accurate to a few units in the last place.
/// The inverse of the regularised incomplete beta function, or NaN when a
/// shape parameter is not a positive number (statrs panics on those, and
/// loops for ever on NaN), as happens with zero degrees of freedom.
fn inv_beta_reg(a: f64, b: f64, p: f64) -> f64 {
    if !(a > 0.0 && b > 0.0 && a.is_finite() && b.is_finite()) || p.is_nan() {
        return f64::NAN;
    }
    statrs_inv_beta_reg(a, b, p)
}

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

/// Nodes and weights of eight-point Gauss-Legendre quadrature on [-1, 1].
fn gauss_legendre() -> &'static [(f64, f64); 8] {
    static RULE: std::sync::OnceLock<[(f64, f64); 8]> = std::sync::OnceLock::new();
    RULE.get_or_init(|| {
        const N: usize = 8;
        let mut rule = [(0.0, 0.0); N];
        for (i, slot) in rule.iter_mut().enumerate() {
            // Newton's method on the Legendre polynomial of degree N, from
            // the usual starting guess for its i-th root.
            let mut x = (std::f64::consts::PI * (i as f64 + 0.75) / (N as f64 + 0.5)).cos();
            let mut derivative = 1.0;
            for _ in 0..100 {
                let (mut previous, mut current) = (1.0, x);
                for degree in 2..=N {
                    let d = degree as f64;
                    let next = ((2.0 * d - 1.0) * x * current - (d - 1.0) * previous) / d;
                    previous = current;
                    current = next;
                }
                derivative = N as f64 * (x * current - previous) / (x * x - 1.0);
                let step = current / derivative;
                x -= step;
                if step.abs() < 1e-16 {
                    break;
                }
            }
            *slot = (x, 2.0 / ((1.0 - x * x) * derivative * derivative));
        }
        rule
    })
}

/// Integral of `f` from `low` to `high`, by Gauss-Legendre quadrature on
/// `pieces` equal parts.
fn integrate(low: f64, high: f64, pieces: usize, f: impl Fn(f64) -> f64) -> f64 {
    let width = (high - low) / pieces as f64;
    let mut total = 0.0;
    for piece in 0..pieces {
        let centre = low + width * (piece as f64 + 0.5);
        for &(node, weight) in gauss_legendre() {
            total += weight * f(centre + 0.5 * width * node);
        }
    }
    0.5 * width * total
}

/// Probability that the range of `k` independent standard normal values is
/// at most `w`.
fn normal_range_cdf(w: f64, k: f64) -> f64 {
    if w <= 0.0 {
        return 0.0;
    }
    // k * integral of phi(z) * (Phi(z) - Phi(z - w))^(k - 1) over all z; the
    // density is negligible beyond 8.5 standard deviations.
    let density = |z: f64| {
        let inside = normal_cdf(z) - normal_cdf(z - w);
        (-0.5 * z * z).exp() * inside.powf(k - 1.0)
    };
    let integral = integrate(-8.5, 8.5 + w.min(8.5), 26, density);
    (k * integral / (2.0 * std::f64::consts::PI).sqrt()).clamp(0.0, 1.0)
}

/// `normal_range_cdf` for one number of groups, tabulated so that the many
/// evaluations a studentized-range probability needs are quick.
struct RangeTable {
    values: Vec<f64>,
}

impl RangeTable {
    /// Spacing of the table and the range beyond which the probability is
    /// one to within rounding.
    const STEP: f64 = 1.0 / 128.0;
    const LIMIT: f64 = 24.0;

    fn new(k: f64) -> RangeTable {
        let points = (Self::LIMIT / Self::STEP) as usize + 3;
        RangeTable {
            values: (0..points)
                .map(|i| normal_range_cdf(i as f64 * Self::STEP, k))
                .collect(),
        }
    }

    /// The probability at `w`, by cubic interpolation.
    fn at(&self, w: f64) -> f64 {
        if w <= 0.0 {
            return 0.0;
        }
        if w >= Self::LIMIT {
            return 1.0;
        }
        let position = w / Self::STEP;
        let i = position as usize;
        let t = position - i as f64;
        let v = &self.values;
        // Catmull-Rom spline through the four nearest points; the value
        // before zero mirrors the one after, as the function is even there.
        let before = if i == 0 { v[1] } else { v[i - 1] };
        let (p0, p1, p2, p3) = (before, v[i], v[i + 1], v[i + 2]);
        let value = p1
            + 0.5
                * t
                * (p2 - p0
                    + t * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3 + t * (3.0 * (p1 - p2) + p3 - p0)));
        value.clamp(0.0, 1.0)
    }
}

/// The table for `k` groups, built on first use and kept.
fn range_table(k: f64) -> std::sync::Arc<RangeTable> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static TABLES: OnceLock<Mutex<HashMap<u64, Arc<RangeTable>>>> = OnceLock::new();
    let mut tables = TABLES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Arc::clone(
        tables
            .entry(k.to_bits())
            .or_insert_with(|| Arc::new(RangeTable::new(k))),
    )
}

/// Cumulative distribution function of the studentized range: the
/// probability that the range of `k` group means, in units of their
/// estimated standard error with `df` degrees of freedom, is at most `q`.
pub fn studentized_range_cdf(q: f64, k: f64, df: f64) -> f64 {
    if q.is_nan() || k.is_nan() || df.is_nan() || k < 2.0 || df < 1.0 {
        return f64::NAN;
    }
    if q <= 0.0 {
        return 0.0;
    }
    // With very many degrees of freedom the standard error is known.
    let range = range_table(k);
    if df > 25_000.0 {
        return range.at(q);
    }
    // Average the range probability over the distribution of the estimated
    // standard deviation s, where df * s^2 is chi-square with df degrees of
    // freedom. Its density is negligible outside these limits.
    let spread = 9.0 * (2.0 * df).sqrt();
    let low = ((df - spread).max(0.0) / df).sqrt();
    let high = ((df + spread + 40.0) / df).sqrt();
    let ln_constant = std::f64::consts::LN_2 + 0.5 * df * (0.5 * df).ln() - ln_gamma(0.5 * df);
    let integrand = |s: f64| {
        if s <= 0.0 {
            return 0.0;
        }
        let ln_density = ln_constant + (df - 1.0) * s.ln() - 0.5 * df * s * s;
        ln_density.exp() * range.at(q * s)
    };
    // The range probability climbs from zero to one while q * s is below
    // about 17; that stretch is integrated finely, the rest more coarsely.
    let climb = (17.0 / q).clamp(low, high);
    let total = integrate(low, climb, 40, integrand)
        + if climb < high {
            integrate(climb, high, 16, integrand)
        } else {
            0.0
        };
    total.clamp(0.0, 1.0)
}

/// Quantile of the studentized range: the `q` below which a fraction `p` of
/// the distribution lies.
pub fn studentized_range_ppf(p: f64, k: f64, df: f64) -> f64 {
    if !(0.0..1.0).contains(&p) || k.is_nan() || df.is_nan() || k < 2.0 || df < 1.0 {
        return f64::NAN;
    }
    if p == 0.0 {
        return 0.0;
    }
    let error = |q: f64| studentized_range_cdf(q, k, df) - p;
    // Bracket the root, then close in on it by the Illinois method.
    let (mut low, mut high) = (0.0, 4.0);
    let (mut f_low, mut f_high) = (-p, error(high));
    while f_high < 0.0 {
        (low, f_low) = (high, f_high);
        high *= 2.0;
        if high > 1e6 {
            return f64::INFINITY;
        }
        f_high = error(high);
    }
    let mut side = 0;
    for _ in 0..200 {
        let middle = (low * f_high - high * f_low) / (f_high - f_low);
        let f_middle = error(middle);
        if f_middle.abs() < 1e-13 || (high - low) < 1e-11 * high {
            return middle;
        }
        if f_middle < 0.0 {
            (low, f_low) = (middle, f_middle);
            if side == -1 {
                f_high *= 0.5;
            }
            side = -1;
        } else {
            (high, f_high) = (middle, f_middle);
            if side == 1 {
                f_low *= 0.5;
            }
            side = 1;
        }
    }
    0.5 * (low + high)
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

    // Reference values from R 4.4.3: `pnorm(c(-2, -1, 0, 1, 2))` and
    // `qnorm(c(0.95, 0.975, 0.90))`, the two-sided critical values for 90%,
    // 95% and 80% coverage.
    #[test]
    fn normal_distribution() {
        close(normal_cdf(-2.0), 0.022750131948179212, 1e-14);
        close(normal_cdf(-1.0), 0.15865525393145705, 1e-14);
        close(normal_cdf(0.0), 0.5, 1e-15);
        close(normal_cdf(1.0), 0.84134474606854293, 1e-14);
        close(normal_cdf(2.0), 0.97724986805182079, 1e-14);
        close(normal_cdf(-1e-6), 1.0 - normal_cdf(1e-6), 1e-15);

        close(z_score(0.90), 1.6448536269514715, 1e-15);
        close(z_score(0.95), 1.9599639845400536, 1e-15);
        close(z_score(0.80), 1.2815515655446008, 1e-13);
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

    #[test]
    fn quadrature_rule_is_exact_for_polynomials() {
        let rule = gauss_legendre();
        close(rule.iter().map(|(_, w)| w).sum::<f64>(), 2.0, 1e-14);
        close(
            integrate(0.0, 2.0, 1, |x| x.powi(15) + 3.0 * x * x),
            4104.0,
            1e-12,
        );
        close(
            integrate(0.0, 1.0, 4, f64::exp),
            std::f64::consts::E - 1.0,
            1e-13,
        );
    }

    #[test]
    fn studentized_range_matches_known_values() {
        // The range of two normal values is |difference|, which is normal
        // with variance two.
        for w in [0.5, 1.0, 2.5, 4.0] {
            let expected = 2.0 * normal_cdf(w / SQRT_2) - 1.0;
            close(normal_range_cdf(w, 2.0), expected, 1e-12);
        }
        // With two groups, q is the square root of twice an F(1, df)
        // variable, so its distribution is known exactly.
        for (q, df) in [
            (1.0, 3.0),
            (2.95, 20.0),
            (4.0, 7.5),
            (6.0, 2.0),
            (44.69, 2.0),
            (3.0, 500.0),
        ] {
            close(
                studentized_range_cdf(q, 2.0, df),
                f_cdf(q * q / 2.0, 1.0, df),
                1e-9,
            );
        }
        // Published upper 5% and 1% points.
        for (p, k, df, q) in [
            (0.95, 3.0, 10.0, 3.877),
            (0.95, 5.0, 20.0, 4.232),
            (0.99, 3.0, 20.0, 4.639),
            (0.95, 4.0, 60.0, 3.737),
            (0.95, 10.0, 120.0, 4.560),
            (0.95, 3.0, 1e9, 3.314),
        ] {
            let got = studentized_range_ppf(p, k, df);
            assert!(
                (got - q).abs() < 1.5e-3,
                "q({p}, {k}, {df}) = {got}, not {q}"
            );
            close(studentized_range_cdf(got, k, df), p, 1e-9);
        }
        assert_eq!(studentized_range_cdf(0.0, 3.0, 10.0), 0.0);
        assert!(studentized_range_cdf(1.0, 1.0, 10.0).is_nan());
    }

    /// Reference values from R 4.4.3: `ptukey(q, k, df)`, and the upper 5%
    /// points found by solving `ptukey(q, k, df) = 0.95` with
    /// `uniroot(..., tol = 1e-14)`. (R's own `qtukey` is documented as
    /// accurate to about four decimal places, so it is not used here.) The
    /// distribution is computed here from a tabulated normal range for
    /// speed, which keeps it within 2e-7 of R.
    #[test]
    fn studentized_range_matches_r() {
        for (q, k, df, p) in [
            (3.5, 3.0, 10.0, 0.9228966891613396),
            (2.0, 5.0, 20.0, 0.37392248758856811),
            (4.2, 4.0, 7.0, 0.92211325466151806),
            (5.0, 10.0, 60.0, 0.9747157801917441),
            (3.0, 3.0, 5.3, 0.82494964917493219),
        ] {
            close(studentized_range_cdf(q, k, df), p, 2e-7);
        }
        for (k, df, q) in [
            (3.0, 5.3, 4.5100339446984119),
            (3.0, 10.0, 3.8767767500178225),
            (3.0, 60.0, 3.3986612409062702),
            (4.0, 7.0, 4.6812756715249924),
            (4.0, 20.0, 3.9582935609640937),
            (5.0, 10.0, 4.6542929979212522),
            (10.0, 5.3, 6.8198920941559864),
            (10.0, 60.0, 4.6463239637879559),
        ] {
            let got = studentized_range_ppf(0.95, k, df);
            assert!(
                (got - q).abs() < 1e-6 * q,
                "q(0.95, {k}, {df}) = {got}, not {q}"
            );
        }
        assert!(studentized_range_ppf(1.0, 3.0, 10.0).is_nan());
    }

    /// The tables of critical values that STAMP v2 looked p-values up in
    /// agree with the distribution computed here.
    #[test]
    fn studentized_range_matches_the_tables_of_stamp_v2() {
        let tables = [
            (0.001, include_str!("../../data/tukeyQ_001.txt")),
            (0.01, include_str!("../../data/tukeyQ_01.txt")),
            (0.02, include_str!("../../data/tukeyQ_02.txt")),
            (0.05, include_str!("../../data/tukeyQ_05.txt")),
            (0.1, include_str!("../../data/tukeyQ_10.txt")),
        ];
        let mut checked = 0;
        for (alpha, text) in tables {
            for (row, line) in text.lines().skip(1).enumerate() {
                let df = row as f64 + 2.0;
                // A sample of the rows and columns keeps this quick. Below
                // ten degrees of freedom the tables themselves are off, by
                // a third at two degrees of freedom and the 0.1% level:
                // their 34.71 for two groups should be 44.69, as the F
                // distribution confirms in the test above.
                if ![10.0, 17.0, 30.0, 60.0, 120.0, 200.0].contains(&df) {
                    continue;
                }
                for (column, cell) in line.split('\t').skip(1).enumerate() {
                    let k = column as f64 + 2.0;
                    if ![2.0, 3.0, 4.0, 6.0, 10.0, 20.0, 29.0].contains(&k) {
                        continue;
                    }
                    let tabled: f64 = cell.trim().parse().unwrap();
                    let p = studentized_range_cdf(tabled, k, df);
                    assert!(
                        (p - (1.0 - alpha)).abs() < 5e-7,
                        "alpha {alpha}, k {k}, df {df}: q {tabled} gives {p}"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 5 * 6 * 7);
    }

    #[test]
    fn quantiles_without_degrees_of_freedom_are_nan() {
        assert!(f_ppf(0.95, 2.0, 0.0).is_nan());
        assert!(f_ppf(0.95, 0.0, 3.0).is_nan());
        assert!(f_ppf(0.95, f64::NAN, 3.0).is_nan());
        assert!(t_isf(0.05, 0.0).is_nan());
    }
}
