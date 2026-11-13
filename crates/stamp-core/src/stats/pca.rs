//! Principal component analysis of samples.

/// Result of a principal component analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct Pca {
    /// Coordinates of each sample: `positions[sample][component]`.
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

    // Centre each feature.
    let mut centred = data.to_vec();
    for feature in 0..m {
        let mean = data.iter().map(|row| row[feature]).sum::<f64>() / n as f64;
        for row in centred.iter_mut() {
            row[feature] -= mean;
        }
    }

    // The eigenvectors of the samples' Gram matrix give the sample
    // coordinates directly, and the matrix is only as large as the number of
    // samples.
    let mut gram = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in i..n {
            let dot: f64 = centred[i].iter().zip(&centred[j]).map(|(a, b)| a * b).sum();
            gram[i][j] = dot;
            gram[j][i] = dot;
        }
    }
    let (values, vectors) = symmetric_eigen(gram);

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let total: f64 = values.iter().map(|v| v.max(0.0)).sum();
    if total <= 0.0 {
        return None;
    }

    // Keep the components that carry variance (at most n - 1 after centring).
    let tolerance = total * 1e-12;
    let kept: Vec<usize> = order
        .into_iter()
        .filter(|&k| values[k] > tolerance)
        .collect();
    let mut positions = vec![Vec::with_capacity(kept.len()); n];
    let mut variance_fractions = Vec::with_capacity(kept.len());
    for &k in &kept {
        let scale = values[k].sqrt();
        let column: Vec<f64> = (0..n).map(|i| vectors[i][k] * scale).collect();
        let extreme = column
            .iter()
            .copied()
            .fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        let sign = if extreme < 0.0 { -1.0 } else { 1.0 };
        for (row, value) in positions.iter_mut().zip(column) {
            row.push(sign * value);
        }
        variance_fractions.push(values[k] / total);
    }
    Some(Pca {
        positions,
        variance_fractions,
    })
}

/// Eigenvalues and eigenvectors (as columns) of a symmetric matrix, by the
/// cyclic Jacobi method.
#[allow(clippy::needless_range_loop)] // rows and columns are updated in step
fn symmetric_eigen(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
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
