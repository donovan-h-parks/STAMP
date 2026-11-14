//! Principal component analysis of samples.

/// Result of a principal component analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct Pca {
    /// Coordinates of each sample: `positions[sample][component]`, for
    /// every component or, from [`pca_leading`], the first few.
    pub positions: Vec<Vec<f64>>,
    /// Fraction of the total variance captured by each component, largest
    /// first.
    pub variance_fractions: Vec<f64>,
}

/// Principal components of `data[sample][feature]`. Features are centred but
/// not scaled. Returns `None` if there are fewer than two samples, the rows
/// differ in length, or the samples do not vary.
///
/// The sign of each component is arbitrary; it is fixed so that the sample
/// furthest from the origin has a positive coordinate.
pub fn pca(data: &[Vec<f64>]) -> Option<Pca> {
    pca_leading(data, usize::MAX)
}

/// [`pca`] with the coordinates of only the first `components` components
/// (all of their shares of the variance are still given), which is much
/// faster for many samples: a plot needs two or three.
pub fn pca_leading(data: &[Vec<f64>], components: usize) -> Option<Pca> {
    let n = data.len();
    let m = data.first()?.len();
    if n < 2
        || m == 0
        || data
            .iter()
            .any(|row| row.len() != m || row.iter().any(|v| !v.is_finite()))
    {
        return None;
    }

    // The eigenvectors of the samples' Gram matrix give the sample
    // coordinates directly, and the matrix is only as large as the number of
    // samples. It is the Gram matrix of the raw values, centred on both
    // sides (the same as that of the centred features), so that features a
    // pair of samples lacks cost nothing: profiles are mostly zeros.
    let present: Vec<Vec<usize>> = data
        .iter()
        .map(|row| (0..m).filter(|&f| row[f] != 0.0).collect())
        .collect();
    let mut gram = vec![0.0; n * n];
    for i in 0..n {
        for j in i..n {
            let (sparse, dense) = if present[i].len() <= present[j].len() {
                (i, j)
            } else {
                (j, i)
            };
            let dot: f64 = present[sparse]
                .iter()
                .map(|&f| data[sparse][f] * data[dense][f])
                .sum();
            gram[i * n + j] = dot;
            gram[j * n + i] = dot;
        }
    }
    double_centre(&mut gram, n);
    let (positions, variance_fractions) = coordinates(&gram, n, components)?;
    Some(Pca {
        positions,
        variance_fractions,
    })
}

/// Subtract the mean of each row and of each column of a symmetric matrix
/// and add back the overall mean.
pub(crate) fn double_centre(a: &mut [f64], n: usize) {
    let rows: Vec<f64> = a
        .chunks_exact(n)
        .map(|row| row.iter().sum::<f64>() / n as f64)
        .collect();
    let mean = rows.iter().sum::<f64>() / n as f64;
    for (row, &row_mean) in a.chunks_exact_mut(n).zip(&rows) {
        for (value, &column_mean) in row.iter_mut().zip(&rows) {
            *value = *value - row_mean - column_mean + mean;
        }
    }
}

/// The coordinates of samples on the axes of a centred inner-product
/// matrix (`b`, `n` by `n`, row by row): the axes with positive eigenvalues
/// (above a tolerance for rounding), largest first, with their shares of
/// the sum of the positive eigenvalues. Coordinates are given for the first
/// `axes` axes (`usize::MAX` for all), each scaled by the square root of its
/// eigenvalue and with its largest coordinate positive. `None` if no
/// eigenvalue is positive.
pub(crate) fn coordinates(b: &[f64], n: usize, axes: usize) -> Option<(Vec<Vec<f64>>, Vec<f64>)> {
    let (mut values, pairs) = if axes >= n {
        let (values, vectors) = super::eigen::full(b, n);
        let pairs: Vec<(f64, Vec<f64>)> = values.iter().copied().zip(vectors).collect();
        (values, pairs)
    } else {
        let values = super::eigen::eigenvalues(b, n);
        let pairs = super::eigen::leading(b, n, axes, &values);
        (values, pairs)
    };
    values.sort_by(|a, b| b.total_cmp(a));
    let total: f64 = values.iter().map(|v| v.max(0.0)).sum();
    if total <= 0.0 {
        return None;
    }
    // Keep the axes that carry variation (at most n - 1 after centring).
    let tolerance = total * 1e-12;
    let variance_fractions: Vec<f64> = values
        .iter()
        .take_while(|&&v| v > tolerance)
        .map(|v| v / total)
        .collect();
    let mut pairs = pairs;
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut positions = vec![Vec::new(); n];
    for (value, vector) in pairs.iter().take(variance_fractions.len().min(axes)) {
        let scale = value.sqrt();
        let extreme = vector
            .iter()
            .copied()
            .fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        let sign = if extreme < 0.0 { -scale } else { scale };
        for (row, v) in positions.iter_mut().zip(vector) {
            row.push(sign * v);
        }
    }
    Some((positions, variance_fractions))
}

/// Eigenvalues and eigenvectors (as columns) of a symmetric matrix, by the
/// cyclic Jacobi method.
#[allow(clippy::needless_range_loop)] // rows and columns are updated in step
pub(crate) fn symmetric_eigen(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut v = vec![vec![0.0; n]; n];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let scale: f64 = a
        .iter()
        .flatten()
        .map(|x| x * x)
        .sum::<f64>()
        .sqrt()
        .max(f64::MIN_POSITIVE);

    for _sweep in 0..100 {
        let off: f64 = (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .map(|(i, j)| a[i][j] * a[i][j])
            .sum();
        if off.sqrt() <= 1e-14 * scale {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q].abs() <= 1e-300 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for row in v.iter_mut() {
                    let (vp, vq) = (row[p], row[q]);
                    row[p] = c * vp - s * vq;
                    row[q] = s * vp + c * vq;
                }
            }
        }
    }
    ((0..n).map(|i| a[i][i]).collect(), v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_on_a_line_have_one_component() {
        let data: Vec<Vec<f64>> = (0..5)
            .map(|i| vec![i as f64, 2.0 * i as f64, 5.0])
            .collect();
        let result = pca(&data).unwrap();
        assert_eq!(result.variance_fractions.len(), 1);
        assert!((result.variance_fractions[0] - 1.0).abs() < 1e-12);
        // Spacing along the line is sqrt(1 + 4) per step, centred on the middle sample.
        let expected = [-2.0, -1.0, 0.0, 1.0, 2.0].map(|k: f64| k * 5f64.sqrt());
        for (row, e) in result.positions.iter().zip(expected) {
            assert!((row[0].abs() - e.abs()).abs() < 1e-9, "{} vs {e}", row[0]);
        }
    }

    // numpy: u, s, vt = svd(x - x.mean(0)); positions = u * s; s**2 / sum(s**2)
    #[test]
    fn matches_numpy_svd() {
        let data = vec![
            vec![2.0, 0.0, 1.0],
            vec![0.0, 1.0, 3.0],
            vec![4.0, 2.0, 0.0],
            vec![1.0, 5.0, 2.0],
        ];
        let result = pca(&data).unwrap();
        let expected_variance = [
            0.5489473709411196,
            0.44719241913411534,
            0.0038602099247650725,
        ];
        for (a, e) in result.variance_fractions.iter().zip(expected_variance) {
            assert!((a - e).abs() < 1e-10, "{a} vs {e}");
        }
        let expected_pc1 = [
            -1.8349400411729055,
            0.7687798095766246,
            -1.7806736927978661,
            2.8468339243941467,
        ];
        for (row, e) in result.positions.iter().zip(expected_pc1) {
            assert!((row[0] - e).abs() < 1e-9, "{} vs {e}", row[0]);
        }
        // Components are uncorrelated and centred.
        for k in 0..3 {
            assert!(result.positions.iter().map(|r| r[k]).sum::<f64>().abs() < 1e-9);
        }
        let dot: f64 = result.positions.iter().map(|r| r[0] * r[1]).sum();
        assert!(dot.abs() < 1e-9);
    }

    #[test]
    fn degenerate_inputs() {
        assert!(pca(&[]).is_none());
        assert!(pca(&[vec![1.0, 2.0]]).is_none());
        assert!(pca(&[vec![1.0, 2.0], vec![1.0, 2.0]]).is_none());
        assert!(pca(&[vec![1.0, 2.0], vec![1.0]]).is_none());
        assert!(pca(&[vec![1.0, f64::NAN], vec![1.0, 2.0]]).is_none());
    }

    #[test]
    fn sign_is_deterministic() {
        let data = vec![vec![0.0, 0.0], vec![1.0, 1.0], vec![10.0, 9.0]];
        let result = pca(&data).unwrap();
        assert!(result.positions[2][0] > 0.0);
    }
}
