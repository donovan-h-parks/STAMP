//! Eigenvalues and eigenvectors of real symmetric matrices, for the
//! ordinations (PCA, PCoA, NMDS).
//!
//! Matrices are square, stored row by row in one slice. The full
//! decomposition and the eigenvalues alone come from nalgebra (Householder
//! reduction to tridiagonal form, then implicit QR steps); the leading
//! eigenvectors alone, which is all a plot needs, come from the Lanczos
//! method, checked against the eigenvalues and replaced by the full
//! decomposition whenever it cannot be trusted.

use nalgebra::DMatrix;

/// All eigenvalues of a symmetric matrix, in no particular order.
pub(crate) fn eigenvalues(a: &[f64], n: usize) -> Vec<f64> {
    debug_assert_eq!(a.len(), n * n);
    if n == 0 {
        return Vec::new();
    }
    DMatrix::from_row_slice(n, n, a)
        .symmetric_eigenvalues()
        .iter()
        .copied()
        .collect()
}

/// All eigenvalues of a symmetric matrix and their eigenvectors:
/// `vectors[k]` is the unit eigenvector of `values[k]`. In no particular
/// order.
pub(crate) fn full(a: &[f64], n: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    debug_assert_eq!(a.len(), n * n);
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let decomposition = DMatrix::from_row_slice(n, n, a).symmetric_eigen();
    let values = decomposition.eigenvalues.iter().copied().collect();
    let vectors = decomposition
        .eigenvectors
        .column_iter()
        .map(|column| column.iter().copied().collect())
        .collect();
    (values, vectors)
}

/// The `k` largest eigenvalues of a symmetric matrix and their unit
/// eigenvectors, largest first, given all its eigenvalues (`values`, from
/// [`eigenvalues`]) to check the result against. Uses the Lanczos method
/// with full reorthogonalisation, which needs only products of the matrix
/// with vectors; falls back on the full decomposition if the Lanczos
/// vectors do not converge, or find other eigenvalues than the largest (an
/// eigenvalue repeated among the largest, which a Krylov space holds only
/// once, or an invariant subspace found early).
pub(crate) fn leading(a: &[f64], n: usize, k: usize, values: &[f64]) -> Vec<(f64, Vec<f64>)> {
    debug_assert_eq!(a.len(), n * n);
    let k = k.min(n);
    if k == 0 {
        return Vec::new();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    // Small matrices cost nothing to decompose fully.
    if n > 48 {
        if let Some(pairs) = lanczos(a, n, k, &sorted) {
            return pairs;
        }
    }
    let (values, vectors) = full(a, n);
    let mut pairs: Vec<(f64, Vec<f64>)> = values.into_iter().zip(vectors).collect();
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
    pairs.truncate(k);
    pairs
}

/// `y = A x` for a symmetric matrix stored row by row.
fn multiply(a: &[f64], x: &[f64], y: &mut [f64]) {
    for (row, out) in a.chunks_exact(x.len()).zip(y.iter_mut()) {
        *out = dot(row, x);
    }
}

/// Dot product in four running sums, which lets the compiler use vector
/// instructions.
pub(crate) fn dot(a: &[f64], b: &[f64]) -> f64 {
    let mut sums = [0.0; 4];
    let ((a4, a_rest), (b4, b_rest)) = (a.as_chunks::<4>(), b.as_chunks::<4>());
    for (x, y) in a4.iter().zip(b4) {
        for lane in 0..4 {
            sums[lane] += x[lane] * y[lane];
        }
    }
    let mut total = (sums[0] + sums[1]) + (sums[2] + sums[3]);
    for (x, y) in a_rest.iter().zip(b_rest) {
        total += x * y;
    }
    total
}

/// `y -= c x`
fn subtract(y: &mut [f64], c: f64, x: &[f64]) {
    for (y, x) in y.iter_mut().zip(x) {
        *y -= c * x;
    }
}

fn lanczos(a: &[f64], n: usize, k: usize, sorted: &[f64]) -> Option<Vec<(f64, Vec<f64>)>> {
    // The size of the matrix, for what counts as zero.
    let scale = sorted
        .first()
        .map_or(0.0, |v| v.abs())
        .max(sorted.last().map_or(0.0, |v| v.abs()));
    if scale <= 0.0 {
        return None;
    }
    // A fixed starting vector with no special relation to the matrix, so
    // that the result is the same every time; no random numbers are drawn.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut start: Vec<f64> = (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        })
        .collect();
    let norm = dot(&start, &start).sqrt();
    start.iter_mut().for_each(|v| *v /= norm);

    let most = n.min(600.max(20 * k));
    let mut basis: Vec<Vec<f64>> = vec![start];
    let mut alphas: Vec<f64> = Vec::new();
    let mut betas: Vec<f64> = Vec::new();
    let mut w = vec![0.0; n];
    let tolerance = 1e-13 * scale;
    loop {
        let j = basis.len() - 1;
        multiply(a, &basis[j], &mut w);
        let alpha = dot(&w, &basis[j]);
        // Full reorthogonalisation, twice ("twice is enough").
        for _ in 0..2 {
            for q in &basis {
                let c = dot(&w, q);
                subtract(&mut w, c, q);
            }
        }
        alphas.push(alpha);
        let beta = dot(&w, &w).sqrt();
        let m = alphas.len();
        // Check for convergence every few steps.
        if m >= k && (m.is_multiple_of(8) || m == most || beta <= tolerance) {
            let (ritz, vectors) = tridiagonal_eigen(&alphas, &betas);
            let mut order: Vec<usize> = (0..m).collect();
            order.sort_by(|&x, &y| ritz[y].total_cmp(&ritz[x]));
            // The residual of a Ritz pair is beta times the last element
            // of its eigenvector of the tridiagonal matrix.
            let converged = order[..k]
                .iter()
                .all(|&i| (beta * vectors[i][m - 1]).abs() <= tolerance);
            if converged {
                // The largest eigenvalues must be those found.
                let agree = order[..k]
                    .iter()
                    .zip(sorted)
                    .all(|(&i, &value)| (ritz[i] - value).abs() <= 1e-9 * scale);
                if !agree {
                    return None;
                }
                let pairs = order[..k]
                    .iter()
                    .zip(sorted)
                    .map(|(&i, &value)| {
                        let mut vector = vec![0.0; n];
                        for (q, s) in basis.iter().zip(&vectors[i]) {
                            subtract(&mut vector, -s, q);
                        }
                        let norm = dot(&vector, &vector).sqrt();
                        vector.iter_mut().for_each(|v| *v /= norm);
                        (value, vector)
                    })
                    .collect();
                return Some(pairs);
            }
            if m == most || beta <= tolerance {
                return None;
            }
        }
        betas.push(beta);
        let next: Vec<f64> = w.iter().map(|v| v / beta).collect();
        basis.push(next);
    }
}

/// Eigenvalues and eigenvectors (`vectors[k]` for `values[k]`) of the
/// symmetric tridiagonal matrix with diagonal `alphas` and off-diagonal
/// `betas`.
fn tridiagonal_eigen(alphas: &[f64], betas: &[f64]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let m = alphas.len();
    let t = DMatrix::from_fn(m, m, |i, j| {
        if i == j {
            alphas[i]
        } else if i + 1 == j {
            betas[i]
        } else if j + 1 == i {
            betas[j]
        } else {
            0.0
        }
    });
    let decomposition = t.symmetric_eigen();
    (
        decomposition.eigenvalues.iter().copied().collect(),
        decomposition
            .eigenvectors
            .column_iter()
            .map(|c| c.iter().copied().collect())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::pca::symmetric_eigen;

    fn random_symmetric(n: usize, seed: u64, shift: f64) -> Vec<f64> {
        let mut state = seed.max(1);
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            for j in i..n {
                let v = next() + if i == j { shift } else { 0.0 };
                a[i * n + j] = v;
                a[j * n + i] = v;
            }
        }
        a
    }

    /// Normalise the sign of a vector as the ordinations do: its largest
    /// element positive.
    fn signed(v: &[f64]) -> Vec<f64> {
        let extreme = v
            .iter()
            .copied()
            .fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        let s = if extreme < 0.0 { -1.0 } else { 1.0 };
        v.iter().map(|x| s * x).collect()
    }

    fn jacobi(a: &[f64], n: usize) -> Vec<(f64, Vec<f64>)> {
        let rows: Vec<Vec<f64>> = a.chunks(n).map(<[f64]>::to_vec).collect();
        let (values, vectors) = symmetric_eigen(rows);
        let mut pairs: Vec<(f64, Vec<f64>)> = (0..n)
            .map(|k| (values[k], (0..n).map(|i| vectors[i][k]).collect()))
            .collect();
        pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
        pairs
    }

    /// The fast paths agree with the cyclic Jacobi method (the earlier
    /// implementation) on random matrices, indefinite ones (as PCoA of a
    /// non-Euclidean distance gives) included.
    #[test]
    fn fast_paths_match_jacobi() {
        for (n, seed, shift) in [(5, 3, 0.0), (60, 7, 0.0), (120, 11, 0.3), (200, 5, -0.2)] {
            let a = random_symmetric(n, seed, shift);
            let reference = jacobi(&a, n);
            assert!(reference.iter().any(|p| p.0 < 0.0), "indefinite");

            let mut values = eigenvalues(&a, n);
            values.sort_by(|a, b| b.total_cmp(a));
            for (v, r) in values.iter().zip(&reference) {
                assert!((v - r.0).abs() < 1e-9, "{v} vs {}", r.0);
            }

            let (full_values, full_vectors) = full(&a, n);
            let mut pairs: Vec<(f64, Vec<f64>)> =
                full_values.into_iter().zip(full_vectors).collect();
            pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
            for (p, r) in pairs.iter().zip(&reference) {
                assert!((p.0 - r.0).abs() < 1e-9);
                for (x, y) in signed(&p.1).iter().zip(signed(&r.1)) {
                    assert!((x - y).abs() < 1e-9, "n {n}: {x} vs {y}");
                }
            }

            if n > 48 {
                // Found by the Lanczos vectors, not the fallback.
                assert!(lanczos(&a, n, 4, &values).is_some(), "n {n}");
            }
            let top = leading(&a, n, 4, &values);
            assert_eq!(top.len(), 4.min(n));
            for (p, r) in top.iter().zip(&reference) {
                assert!((p.0 - r.0).abs() < 1e-9);
                for (x, y) in signed(&p.1).iter().zip(signed(&r.1)) {
                    assert!((x - y).abs() < 1e-9, "n {n}: {x} vs {y}");
                }
            }
        }
    }

    /// A Gower-centred matrix of a non-Euclidean (Bray-Curtis-like)
    /// dissimilarity, positive and negative eigenvalues, decaying spectrum.
    #[test]
    fn leading_pairs_of_a_pcoa_matrix() {
        let n = 150;
        let mut state = 99u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let samples: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..40)
                    .map(|f| {
                        let v = next();
                        // Sparse, with structure along the samples.
                        if v < 0.5 {
                            0.0
                        } else {
                            v * (1.0 + ((i * (f + 1)) % 7) as f64)
                        }
                    })
                    .collect()
            })
            .collect();
        let d = crate::stats::diversity::distances(
            &samples,
            crate::stats::diversity::Distance::BrayCurtis,
        );
        let mut b = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                b[i * n + j] = -0.5 * d.get(i, j) * d.get(i, j);
            }
        }
        let rows: Vec<f64> = (0..n)
            .map(|i| b[i * n..(i + 1) * n].iter().sum::<f64>() / n as f64)
            .collect();
        let mean = rows.iter().sum::<f64>() / n as f64;
        for i in 0..n {
            for j in 0..n {
                b[i * n + j] += mean - rows[i] - rows[j];
            }
        }
        let reference = jacobi(&b, n);
        assert!(reference.last().unwrap().0 < -1e-6, "negative eigenvalues");
        let mut values = eigenvalues(&b, n);
        values.sort_by(|a, b| b.total_cmp(a));
        assert!(lanczos(&b, n, 3, &values).is_some());
        let top = leading(&b, n, 3, &values);
        for (p, r) in top.iter().zip(&reference) {
            assert!((p.0 - r.0).abs() < 1e-9);
            for (x, y) in signed(&p.1).iter().zip(signed(&r.1)) {
                assert!((x - y).abs() < 1e-9, "{x} vs {y}");
            }
        }
    }

    /// A repeated largest eigenvalue, which the Lanczos vectors find twice
    /// only through rounding, or else leave to the full decomposition.
    #[test]
    fn repeated_eigenvalues() {
        let n = 80;
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            a[i * n + i] = if i < 2 { 5.0 } else { 1.0 / (i + 1) as f64 };
        }
        let mut values = eigenvalues(&a, n);
        values.sort_by(|a, b| b.total_cmp(a));
        let top = leading(&a, n, 3, &values);
        assert!((top[0].0 - 5.0).abs() < 1e-12 && (top[1].0 - 5.0).abs() < 1e-12);
        assert!((top[2].0 - 1.0 / 3.0).abs() < 1e-12);
        // The two vectors of the repeated value span its eigenspace.
        let overlap: f64 = top[0].1[2..]
            .iter()
            .chain(&top[1].1[2..])
            .map(|x| x * x)
            .sum();
        assert!(overlap < 1e-20);
        assert!((dot(&top[0].1, &top[1].1)).abs() < 1e-12);

        // Nothing for the Lanczos vectors to find: the full decomposition.
        let zero = vec![0.0; n * n];
        assert!(lanczos(&zero, n, 2, &vec![0.0; n]).is_none());
        let top = leading(&zero, n, 2, &vec![0.0; n]);
        assert_eq!(top.len(), 2);
        assert!(top
            .iter()
            .all(|p| p.0 == 0.0 && (dot(&p.1, &p.1) - 1.0).abs() < 1e-12));
    }
}
