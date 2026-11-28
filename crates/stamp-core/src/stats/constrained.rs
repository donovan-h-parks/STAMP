//! Constrained ordination: how much of the differences between samples
//! some explanatory variables account for, and the axes along which they
//! do.
//!
//! Two methods share one engine, a weighted multivariate regression of a
//! response matrix on the explanatory variables followed by an
//! eigen-analysis of the fitted values (constrained axes) and of the
//! residuals (unconstrained axes):
//!
//! - **Distance-based redundancy analysis** (db-RDA; Legendre & Anderson
//!   1999, in the form of McArdle & Anderson 2001): the response is the
//!   principal coordinates of a distance matrix. As in the PCoA, axes with
//!   negative eigenvalues (which a distance that is not Euclidean, such as
//!   Bray-Curtis, can give) are left out, as vegan's `capscale` leaves them
//!   out; shares of the variation are of the positive part.
//! - **Canonical correspondence analysis** (CCA; ter Braak 1986): the
//!   response is the table's chi-square residuals, (p_ij - r_i c_j) /
//!   sqrt(r_i c_j), and samples are weighted by their totals, as in vegan's
//!   `cca`.
//!
//! Numeric variables enter as one column each; a categorical variable as
//! one column per category but the first (which is the same model whichever
//! category is left out). Samples are placed by their weighted-average
//! scores (their own position projected onto each axis, not the position
//! the variables predict), scaled so that distances between samples are
//! kept within the plotted axes (vegan's scaling 1). Numeric variables are
//! drawn as arrows from their correlations with the constrained axes, and
//! categories as the centroids of their samples.
//!
//! Tests are by permutation of the samples (999 by default), of the whole
//! model and, with several variables, of each variable given the others
//! (vegan's `anova(..., by = "margin")`: the residuals of the model without
//! the variable are permuted, each sample taking its values of the other
//! variables, and its weight, with it). The statistic is the pseudo-F ratio
//! of the variation explained per column to the residual variation per
//! residual degree of freedom. The statistics of given permutations are
//! those of vegan 2.7.

use super::diversity::DistanceMatrix;
use super::results::{Control, Step};
use super::two_sample::random_index;
use super::StampRng;

/// The constrained ordination methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConstrainedMethod {
    /// Distance-based redundancy analysis of a distance matrix.
    DbRda,
    /// Canonical correspondence analysis of the abundances.
    Cca,
}

impl ConstrainedMethod {
    pub fn name(self) -> &'static str {
        match self {
            ConstrainedMethod::DbRda => "db-RDA",
            ConstrainedMethod::Cca => "CCA",
        }
    }

    /// The name of the constrained axes, before their number.
    pub fn constrained_axis(self) -> &'static str {
        match self {
            ConstrainedMethod::DbRda => "dbRDA",
            ConstrainedMethod::Cca => "CCA",
        }
    }

    /// The name of the unconstrained (residual) axes, before their number.
    pub fn unconstrained_axis(self) -> &'static str {
        match self {
            ConstrainedMethod::DbRda => "MDS",
            ConstrainedMethod::Cca => "CA",
        }
    }
}

/// An explanatory variable: a value for each sample, in the order of the
/// samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    pub values: Values,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Values {
    Numbers(Vec<f64>),
    Categories(Vec<String>),
}

/// A numeric variable as an arrow: its correlation with each plotted axis
/// (zero on the unconstrained axes, which are uncorrelated with every
/// variable), and the arrow's end scaled like the samples, before it is
/// stretched to fit a plot.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrow {
    pub name: String,
    pub correlations: Vec<f64>,
    pub position: Vec<f64>,
}

/// The centroid of the samples of one category of a categorical variable.
#[derive(Debug, Clone, PartialEq)]
pub struct Centroid {
    pub variable: String,
    pub category: String,
    pub position: Vec<f64>,
}

/// What a constrained ordination places and finds.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstrainedOrdination {
    pub method: ConstrainedMethod,
    /// The names of the explanatory variables, in the order given.
    pub variables: Vec<String>,
    /// Coordinates of each sample on the plotted axes:
    /// `positions[sample][axis]`, the constrained axes first.
    pub positions: Vec<Vec<f64>>,
    /// The name of each plotted axis, for example "dbRDA1" or "MDS1".
    pub axis_names: Vec<String>,
    /// The share of the total variation (inertia) of each plotted axis.
    pub axis_fractions: Vec<f64>,
    /// The number of constrained axes (the rank of the explanatory
    /// variables).
    pub constrained_axes: usize,
    /// The share of the total variation the variables account for (R²).
    pub explained: f64,
    /// R² adjusted for the number of explanatory columns (Ezekiel's
    /// formula), for db-RDA; for CCA it can only be estimated by
    /// permutation, and is not given.
    pub adjusted: Option<f64>,
    pub arrows: Vec<Arrow>,
    pub centroids: Vec<Centroid>,
}

/// A fitted constrained ordination, with what its permutation tests need.
#[derive(Debug, Clone)]
pub struct ConstrainedModel {
    pub ordination: ConstrainedOrdination,
    data: TestData,
}

/// What the permutation tests work on.
#[derive(Debug, Clone)]
struct TestData {
    /// `response[sample]`: the coordinates the variables are fitted to.
    response: Vec<Vec<f64>>,
    /// The weight of each sample, adding up to one.
    weights: Vec<f64>,
    /// Whether the weights differ (CCA), so that the explanatory columns
    /// are weighted again in every permutation.
    weighted: bool,
    terms: Vec<Term>,
}

/// An explanatory variable as columns of the design matrix.
#[derive(Debug, Clone)]
struct Term {
    name: String,
    columns: Vec<Vec<f64>>,
    /// For a categorical variable: its categories, in order, and the
    /// category of each sample.
    categories: Option<(Vec<String>, Vec<usize>)>,
}

/// The outcome of one permutation test.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstrainedTest {
    /// The variable tested, or empty for the whole model.
    pub name: String,
    /// The columns the test is about (its degrees of freedom).
    pub df: usize,
    /// The share of the total variation these columns account for (for a
    /// variable, beyond what the others account for).
    pub explained: f64,
    /// The pseudo-F ratio; `NaN` if undefined (a variable that adds
    /// nothing to the others).
    pub statistic: f64,
    /// `NaN` without permutations or a statistic.
    pub p_value: f64,
    pub permutations: usize,
}

/// The permutation tests of a constrained ordination.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstrainedTests {
    pub overall: ConstrainedTest,
    /// Each variable given the others, when there are several.
    pub variables: Vec<ConstrainedTest>,
}

/// The fewest samples a constrained ordination is fitted to.
const FEWEST_SAMPLES: usize = 3;

/// Distance-based redundancy analysis of `distances` on `variables`,
/// keeping `axes` axes for a plot.
pub fn db_rda(
    distances: &DistanceMatrix,
    variables: &[Variable],
    axes: usize,
) -> Result<ConstrainedModel, String> {
    let n = distances.size();
    if n < FEWEST_SAMPLES {
        return Err(too_few_samples("db-RDA"));
    }
    let Some(coordinates) = super::diversity::pcoa(distances) else {
        return Err("Degenerate db-RDA: the samples are all the same by this distance".to_string());
    };
    let weights = vec![1.0 / n as f64; n];
    fit(
        ConstrainedMethod::DbRda,
        coordinates.positions,
        weights,
        false,
        variables,
        axes,
    )
}

/// Canonical correspondence analysis of `table[sample][feature]` (counts
/// or other non-negative abundances) on `variables`, keeping `axes` axes
/// for a plot.
pub fn cca(
    table: &[Vec<f64>],
    variables: &[Variable],
    axes: usize,
) -> Result<ConstrainedModel, String> {
    let n = table.len();
    if n < FEWEST_SAMPLES {
        return Err(too_few_samples("CCA"));
    }
    let features = table[0].len();
    if table.iter().any(|row| row.len() != features)
        || table.iter().flatten().any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err("CCA needs abundances that are numbers of zero or more".to_string());
    }
    let totals: Vec<f64> = table.iter().map(|row| row.iter().sum()).collect();
    let grand: f64 = totals.iter().sum();
    if totals.iter().any(|&t| t <= 0.0) {
        return Err("CCA needs every sample to have some abundance".to_string());
    }
    let rows: Vec<f64> = totals.iter().map(|t| t / grand).collect();
    let mut columns = vec![0.0; features];
    for row in table {
        for (c, v) in columns.iter_mut().zip(row) {
            *c += v / grand;
        }
    }
    // The inner products of the chi-square residuals of every pair of
    // samples: sum_j p_ij p_kj / (c_j sqrt(r_i r_k)) - sqrt(r_i r_k), which
    // visits only the features the samples have.
    let present: Vec<Vec<usize>> = table
        .iter()
        .map(|row| (0..features).filter(|&f| row[f] > 0.0).collect())
        .collect();
    let mut gram = vec![0.0; n * n];
    for i in 0..n {
        for k in i..n {
            let (sparse, dense) = if present[i].len() <= present[k].len() {
                (i, k)
            } else {
                (k, i)
            };
            let sum: f64 = present[sparse]
                .iter()
                .map(|&f| table[sparse][f] * table[dense][f] / (grand * grand) / columns[f])
                .sum();
            let root = (rows[i] * rows[k]).sqrt();
            let value = sum / root - root;
            gram[i * n + k] = value;
            gram[k * n + i] = value;
        }
    }
    let Some((response, _)) = super::pca::coordinates(&gram, n, usize::MAX) else {
        return Err("Degenerate CCA: the samples are all alike in composition".to_string());
    };
    fit(
        ConstrainedMethod::Cca,
        response,
        rows,
        true,
        variables,
        axes,
    )
}

fn too_few_samples(method: &str) -> String {
    format!("{method} needs at least {FEWEST_SAMPLES} samples with a value for every variable")
}

/// The explanatory variables as columns of a design matrix.
fn terms_of(variables: &[Variable], n: usize) -> Result<Vec<Term>, String> {
    if variables.is_empty() {
        return Err("Choose at least one explanatory variable".to_string());
    }
    variables
        .iter()
        .map(|variable| match &variable.values {
            Values::Numbers(values) => {
                if values.len() != n || values.iter().any(|v| !v.is_finite()) {
                    return Err(format!("{} needs a number for every sample", variable.name));
                }
                Ok(Term {
                    name: variable.name.clone(),
                    columns: vec![values.clone()],
                    categories: None,
                })
            }
            Values::Categories(values) => {
                if values.len() != n {
                    return Err(format!("{} needs a value for every sample", variable.name));
                }
                let mut categories: Vec<String> = values.clone();
                categories.sort();
                categories.dedup();
                let of_sample: Vec<usize> = values
                    .iter()
                    .map(|v| categories.binary_search(v).unwrap_or(0))
                    .collect();
                let columns = (1..categories.len())
                    .map(|c| {
                        of_sample
                            .iter()
                            .map(|&s| if s == c { 1.0 } else { 0.0 })
                            .collect()
                    })
                    .collect();
                Ok(Term {
                    name: variable.name.clone(),
                    columns,
                    categories: Some((categories, of_sample)),
                })
            }
        })
        .collect()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    super::eigen::dot(a, b)
}

/// An orthonormal basis of the space the columns add to `start` (whose
/// vectors are orthonormal), after each column is centred on its weighted
/// mean and multiplied by the square roots of the weights: the space the
/// response is projected onto. Columns that add nothing (constant, or
/// combinations of others) are left out.
fn basis(columns: &[&[f64]], weights: &[f64], start: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut found: Vec<Vec<f64>> = Vec::new();
    for column in columns {
        let mean: f64 = column.iter().zip(weights).map(|(x, w)| x * w).sum();
        let mut v: Vec<f64> = column
            .iter()
            .zip(weights)
            .map(|(x, w)| (x - mean) * w.sqrt())
            .collect();
        let before = dot(&v, &v).sqrt();
        // A constant column leaves only rounding once centred.
        let scale: f64 = column
            .iter()
            .zip(weights)
            .map(|(x, w)| x * x * w)
            .sum::<f64>()
            .sqrt();
        if !(before > 1e-10 * scale && before.is_finite()) {
            continue;
        }
        // Twice, which is enough to keep the basis orthogonal.
        for _ in 0..2 {
            for b in start.iter().chain(&found) {
                let c = dot(&v, b);
                for (x, y) in v.iter_mut().zip(b) {
                    *x -= c * y;
                }
            }
        }
        let after = dot(&v, &v).sqrt();
        if after > 1e-8 * before {
            v.iter_mut().for_each(|x| *x /= after);
            found.push(v);
        }
    }
    found
}

/// `Q'Y`: the coordinates of the response columns in a basis, a row per
/// basis vector.
fn project(basis: &[Vec<f64>], response: &[Vec<f64>], order: Option<&[usize]>) -> Vec<Vec<f64>> {
    let r = response.first().map_or(0, Vec::len);
    basis
        .iter()
        .map(|q| {
            let mut row = vec![0.0; r];
            for (i, &qi) in q.iter().enumerate() {
                let sample = order.map_or(i, |o| o[i]);
                for (out, y) in row.iter_mut().zip(&response[sample]) {
                    *out += qi * y;
                }
            }
            row
        })
        .collect()
}

/// The sum of squares of a matrix.
fn squares(rows: &[Vec<f64>]) -> f64 {
    rows.iter().map(|row| dot(row, row)).sum()
}

/// Turn the largest coordinate positive, as the PCoA does.
fn sign_of(values: &[f64]) -> f64 {
    let extreme = values
        .iter()
        .copied()
        .fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
    if extreme < 0.0 {
        -1.0
    } else {
        1.0
    }
}

/// Fit the variables to the response and find the axes.
fn fit(
    method: ConstrainedMethod,
    response: Vec<Vec<f64>>,
    weights: Vec<f64>,
    weighted: bool,
    variables: &[Variable],
    axes: usize,
) -> Result<ConstrainedModel, String> {
    let n = response.len();
    let terms = terms_of(variables, n)?;
    let total = squares(&response);
    let columns: Vec<&[f64]> = terms
        .iter()
        .flat_map(|t| t.columns.iter().map(Vec::as_slice))
        .collect();
    let q = basis(&columns, &weights, &[]);
    let rank = q.len();
    if rank == 0 {
        return Err("The explanatory variables do not vary among these samples".to_string());
    }
    if rank + 2 > n {
        return Err(format!(
            "Too many explanatory variables: {rank} column(s) need at least {} samples",
            rank + 2
        ));
    }
    // Divide the coordinates of CCA by the square roots of the weights, to
    // turn the weighted residuals back into positions.
    let divisor: Vec<f64> = if weighted {
        weights.iter().map(|w| w.sqrt()).collect()
    } else {
        vec![1.0; n]
    };

    // The constrained axes, from the singular value decomposition of the
    // fitted values QB, through the small matrix BB'.
    let b = project(&q, &response, None);
    let mut bbt = vec![0.0; rank * rank];
    for i in 0..rank {
        for k in 0..rank {
            bbt[i * rank + k] = dot(&b[i], &b[k]);
        }
    }
    let (values, vectors) = super::eigen::full(&bbt, rank);
    let mut pairs: Vec<(f64, Vec<f64>)> = values.into_iter().zip(vectors).collect();
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
    let tolerance = total * 1e-12;
    pairs.retain(|(value, _)| *value > tolerance);
    let fitted: f64 = pairs.iter().map(|(value, _)| value).sum();
    let constrained_axes = pairs.len();

    let r = response[0].len();
    let mut positions: Vec<Vec<f64>> = vec![Vec::new(); n];
    let mut axis_names = Vec::new();
    let mut axis_fractions = Vec::new();
    let mut lc_scores: Vec<Vec<f64>> = Vec::new();
    for (k, (value, u)) in pairs.iter().enumerate().take(axes) {
        let s = value.sqrt();
        // The axis in the space of the response, and each sample's own
        // position on it.
        let v: Vec<f64> = (0..r)
            .map(|c| (0..rank).map(|m| b[m][c] * u[m]).sum::<f64>() / s)
            .collect();
        let wa: Vec<f64> = response
            .iter()
            .zip(&divisor)
            .map(|(y, d)| dot(y, &v) / d)
            .collect();
        let sign = sign_of(&wa);
        // The fitted values on the axis, weighted, for the correlations of
        // the variables.
        let lc: Vec<f64> = (0..n)
            .map(|i| sign * s * (0..rank).map(|m| q[m][i] * u[m]).sum::<f64>())
            .collect();
        for (row, x) in positions.iter_mut().zip(&wa) {
            row.push(sign * x);
        }
        lc_scores.push(lc);
        axis_names.push(format!("{}{}", method.constrained_axis(), k + 1));
        axis_fractions.push(value / total);
    }

    // The unconstrained axes, from the residuals, after the constrained
    // ones.
    let wanted = axes.saturating_sub(axis_names.len());
    if wanted > 0 {
        let mut residuals = response.clone();
        for (qm, bm) in q.iter().zip(&b) {
            for (row, &qi) in residuals.iter_mut().zip(qm) {
                for (x, y) in row.iter_mut().zip(bm) {
                    *x -= qi * y;
                }
            }
        }
        let residual_total = squares(&residuals);
        let mut gram = vec![0.0; n * n];
        for i in 0..n {
            for k in i..n {
                let value = dot(&residuals[i], &residuals[k]);
                gram[i * n + k] = value;
                gram[k * n + i] = value;
            }
        }
        if let Some((coordinates, fractions)) = super::pca::coordinates(&gram, n, wanted) {
            for (row, (extra, d)) in positions.iter_mut().zip(coordinates.iter().zip(&divisor)) {
                row.extend(extra.iter().map(|x| x / d));
            }
            for (k, fraction) in fractions.iter().enumerate().take(wanted) {
                axis_names.push(format!("{}{}", method.unconstrained_axis(), k + 1));
                axis_fractions.push(fraction * residual_total / total);
            }
        }
    }

    // Numeric variables as arrows: their weighted correlation with the
    // fitted values on each constrained axis.
    let plotted = axis_names.len();
    let mut arrows = Vec::new();
    for term in &terms {
        if term.categories.is_some() {
            continue;
        }
        let column = &term.columns[0];
        let mean: f64 = column.iter().zip(&weights).map(|(x, w)| x * w).sum();
        let centred: Vec<f64> = column
            .iter()
            .zip(&weights)
            .map(|(x, w)| (x - mean) * w.sqrt())
            .collect();
        let norm = dot(&centred, &centred).sqrt();
        if norm <= 0.0 {
            continue;
        }
        let correlations: Vec<f64> = (0..plotted)
            .map(|k| match lc_scores.get(k) {
                Some(lc) => dot(&centred, lc) / (norm * dot(lc, lc).sqrt()),
                None => 0.0,
            })
            .collect();
        let position = correlations
            .iter()
            .zip(&axis_fractions)
            .map(|(c, f)| c * (f * total).sqrt())
            .collect();
        arrows.push(Arrow {
            name: term.name.clone(),
            correlations,
            position,
        });
    }
    // Categories as the weighted centroids of their samples.
    let mut centroids = Vec::new();
    for term in &terms {
        let Some((categories, of_sample)) = &term.categories else {
            continue;
        };
        for (c, category) in categories.iter().enumerate() {
            let mut sum = vec![0.0; plotted];
            let mut weight = 0.0;
            for (i, row) in positions.iter().enumerate() {
                if of_sample[i] == c {
                    weight += weights[i];
                    for (s, x) in sum.iter_mut().zip(row) {
                        *s += weights[i] * x;
                    }
                }
            }
            if weight > 0.0 {
                centroids.push(Centroid {
                    variable: term.name.clone(),
                    category: category.clone(),
                    position: sum.iter().map(|s| s / weight).collect(),
                });
            }
        }
    }

    let explained = fitted / total;
    let adjusted = (!weighted)
        .then(|| 1.0 - (1.0 - explained) * (n as f64 - 1.0) / (n as f64 - rank as f64 - 1.0));
    Ok(ConstrainedModel {
        ordination: ConstrainedOrdination {
            method,
            variables: terms.iter().map(|t| t.name.clone()).collect(),
            positions,
            axis_names,
            axis_fractions,
            constrained_axes,
            explained,
            adjusted,
            arrows,
            centroids,
        },
        data: TestData {
            response,
            weights,
            weighted,
            terms,
        },
    })
}

impl ConstrainedModel {
    /// The permutation tests, carried out a permutation at a time.
    pub fn test_run(&self, permutations: usize) -> ConstrainedTestRun {
        ConstrainedTestRun {
            data: self.data.clone(),
            permutations,
            current: None,
            overall: None,
            variables: Vec::new(),
        }
    }

    /// The permutation tests, all at once.
    pub fn tests(&self, permutations: usize, rng: &mut StampRng) -> ConstrainedTests {
        match self
            .test_run(permutations)
            .advance(rng, &mut |_, _, _| Control::Continue)
        {
            Step::Done(tests) => tests,
            Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
        }
    }
}

/// The permutation tests of a [`ConstrainedModel`]: of the whole model,
/// then of each variable given the others, a permutation at a time. The
/// outcome does not depend on where the run pauses.
pub struct ConstrainedTestRun {
    data: TestData,
    permutations: usize,
    current: Option<OneTest>,
    overall: Option<ConstrainedTest>,
    variables: Vec<ConstrainedTest>,
}

/// One test under way: of the columns `tested` given the columns `given`.
struct OneTest {
    name: String,
    /// The residuals of the response on the given columns, which are
    /// permuted.
    residuals: Vec<Vec<f64>>,
    given: Vec<Vec<f64>>,
    tested: Vec<Vec<f64>>,
    /// Their bases with the samples' own weights.
    q_given: Vec<Vec<f64>>,
    q_tested: Vec<Vec<f64>>,
    df: usize,
    residual_df: usize,
    explained: f64,
    observed: f64,
    shuffled: Vec<usize>,
    as_extreme: usize,
    done: usize,
}

/// Square root of the machine epsilon, the tolerance vegan allows when it
/// compares permuted statistics with the observed one.
const EPS: f64 = 1.490_116_119_384_765_6e-8;

impl OneTest {
    /// The test of `tested` (indices of terms) given the other terms.
    fn new(data: &TestData, name: String, tested: &[usize]) -> OneTest {
        let n = data.response.len();
        let mut given = Vec::new();
        let mut columns = Vec::new();
        for (t, term) in data.terms.iter().enumerate() {
            let target = if tested.contains(&t) {
                &mut columns
            } else {
                &mut given
            };
            target.extend(term.columns.iter().cloned());
        }
        let (q_given, q_tested) = bases(&given, &columns, &data.weights);
        // The response less what the given columns account for.
        let b = project(&q_given, &data.response, None);
        let mut residuals = data.response.clone();
        for (qm, bm) in q_given.iter().zip(&b) {
            for (row, &qi) in residuals.iter_mut().zip(qm) {
                for (x, y) in row.iter_mut().zip(bm) {
                    *x -= qi * y;
                }
            }
        }
        let df = q_tested.len();
        let rank = q_given.len() + df;
        let residual_df = n.saturating_sub(rank + 1);
        let total = squares(&data.response);
        let identity: Vec<usize> = (0..n).collect();
        let (explained_ss, observed) =
            statistic(&residuals, &q_given, &q_tested, &identity, df, residual_df);
        OneTest {
            name,
            residuals,
            given,
            tested: columns,
            q_given,
            q_tested,
            df,
            residual_df,
            explained: explained_ss / total,
            observed,
            shuffled: identity,
            as_extreme: 0,
            done: 0,
        }
    }

    fn result(&self, permutations: usize) -> ConstrainedTest {
        let p_value = if permutations == 0 || self.observed.is_nan() {
            f64::NAN
        } else {
            (self.as_extreme + 1) as f64 / (permutations + 1) as f64
        };
        ConstrainedTest {
            name: self.name.clone(),
            df: self.df,
            explained: self.explained,
            statistic: self.observed,
            p_value,
            permutations,
        }
    }
}

/// Orthonormal bases of the given columns and of what the tested columns
/// add to them.
fn bases(
    given: &[Vec<f64>],
    tested: &[Vec<f64>],
    weights: &[f64],
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let given_refs: Vec<&[f64]> = given.iter().map(Vec::as_slice).collect();
    let tested_refs: Vec<&[f64]> = tested.iter().map(Vec::as_slice).collect();
    let q_given = basis(&given_refs, weights, &[]);
    let q_tested = basis(&tested_refs, weights, &q_given);
    (q_given, q_tested)
}

/// The variation the tested columns account for in `residuals` taken in
/// `order`, and the pseudo-F ratio: what they account for per column over
/// what is left per residual degree of freedom. The rows are first freed
/// of what the given columns account for (in the bases given, found for
/// the permutation).
fn statistic(
    residuals: &[Vec<f64>],
    q_given: &[Vec<f64>],
    q_tested: &[Vec<f64>],
    order: &[usize],
    df: usize,
    residual_df: usize,
) -> (f64, f64) {
    if df == 0 || residual_df == 0 {
        return (0.0, f64::NAN);
    }
    let all: f64 = order
        .iter()
        .map(|&i| dot(&residuals[i], &residuals[i]))
        .sum();
    let given = squares(&project(q_given, residuals, Some(order)));
    let tested = squares(&project(q_tested, residuals, Some(order)));
    let left = all - given - tested;
    let f = (tested / df as f64) / (left / residual_df as f64);
    (tested, f)
}

impl ConstrainedTestRun {
    /// The statistic of `test` with its samples in their current order.
    fn statistic_of(data: &TestData, test: &OneTest) -> f64 {
        let rebuilt;
        let (given, tested) = if data.weighted || !test.given.is_empty() {
            // The samples take their weights and their values of the given
            // variables with them (as vegan does), so the bases are found
            // anew.
            let order = &test.shuffled;
            let weights: Vec<f64> = if data.weighted {
                order.iter().map(|&i| data.weights[i]).collect()
            } else {
                data.weights.clone()
            };
            let given: Vec<Vec<f64>> = test
                .given
                .iter()
                .map(|column| order.iter().map(|&i| column[i]).collect())
                .collect();
            rebuilt = bases(&given, &test.tested, &weights);
            (&rebuilt.0, &rebuilt.1)
        } else {
            (&test.q_given, &test.q_tested)
        };
        statistic(
            &test.residuals,
            given,
            tested,
            &test.shuffled,
            test.df,
            test.residual_df,
        )
        .1
    }
    /// Make further permutations, drawing on `rng`, which must be the same
    /// on every call. `pace` is asked before each, with what is being done
    /// and how many of how many permutations are made.
    pub fn advance(
        &mut self,
        rng: &mut StampRng,
        pace: &mut dyn FnMut(&str, usize, usize) -> Control,
    ) -> Step<ConstrainedTests> {
        let terms = self.data.terms.len();
        loop {
            // The tests, in order: the whole model, then each variable if
            // there are several.
            let next = if self.overall.is_none() {
                Some(None)
            } else if terms > 1 && self.variables.len() < terms {
                Some(Some(self.variables.len()))
            } else {
                None
            };
            let Some(which) = next else {
                let Some(overall) = self.overall.clone() else {
                    unreachable!("the overall test is done first");
                };
                return Step::Done(ConstrainedTests {
                    overall,
                    variables: self.variables.clone(),
                });
            };
            let data = &self.data;
            let current = self.current.get_or_insert_with(|| match which {
                None => OneTest::new(data, String::new(), &(0..terms).collect::<Vec<_>>()),
                Some(t) => OneTest::new(data, data.terms[t].name.clone(), &[t]),
            });
            let label = match which {
                None => "Permutation test of the model".to_string(),
                Some(_) => format!("Permutation test of {}", current.name),
            };
            if !current.observed.is_nan() {
                while current.done < self.permutations {
                    match pace(&label, current.done, self.permutations) {
                        Control::Continue => {}
                        Control::Pause => return Step::Paused,
                        Control::Cancel => return Step::Cancelled,
                    }
                    for i in (1..current.shuffled.len()).rev() {
                        current.shuffled.swap(i, random_index(rng, i + 1));
                    }
                    let f = Self::statistic_of(data, current);
                    if f >= current.observed - EPS {
                        current.as_extreme += 1;
                    }
                    current.done += 1;
                }
            }
            let result = current.result(self.permutations);
            self.current = None;
            match which {
                None => self.overall = Some(result),
                Some(_) => self.variables.push(result),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::diversity::{distances, Distance};
    use crate::stats::rng_from_seed;

    // The data of the constrained-ordination section of
    // tests/reference/reference_values.R; values from vegan 2.7.5 (R 4.4).
    const COUNTS: [[f64; 6]; 10] = [
        [10.0, 0.0, 3.0, 5.0, 8.0, 1.0],
        [4.0, 2.0, 9.0, 0.0, 3.0, 7.0],
        [0.0, 5.0, 6.0, 2.0, 1.0, 9.0],
        [7.0, 7.0, 1.0, 3.0, 0.0, 2.0],
        [3.0, 1.0, 0.0, 8.0, 6.0, 4.0],
        [12.0, 3.0, 2.0, 1.0, 9.0, 0.0],
        [1.0, 9.0, 4.0, 4.0, 2.0, 6.0],
        [5.0, 0.0, 8.0, 6.0, 1.0, 3.0],
        [2.0, 6.0, 3.0, 9.0, 4.0, 1.0],
        [8.0, 4.0, 5.0, 0.0, 7.0, 5.0],
    ];

    fn table() -> Vec<Vec<f64>> {
        COUNTS.iter().map(|row| row.to_vec()).collect()
    }

    fn ph() -> Variable {
        Variable {
            name: "pH".to_string(),
            values: Values::Numbers(vec![5.1, 6.2, 7.0, 5.5, 6.8, 4.9, 7.3, 6.0, 6.5, 5.2]),
        }
    }

    fn temp() -> Variable {
        Variable {
            name: "Temp".to_string(),
            values: Values::Numbers(vec![
                12.0, 15.0, 9.0, 20.0, 11.0, 14.0, 8.0, 17.0, 10.0, 19.0,
            ]),
        }
    }

    fn site() -> Variable {
        Variable {
            name: "Site".to_string(),
            values: Values::Categories("ABCABCABCA".chars().map(|c| c.to_string()).collect()),
        }
    }

    fn bray_curtis() -> DistanceMatrix {
        distances(&table(), Distance::BrayCurtis)
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} vs {b}");
    }

    /// The two lists are proportional (the scale of vegan's scores
    /// differs, and the sign of an axis is arbitrary).
    fn proportional(a: &[f64], b: &[f64]) {
        let r = crate::stats::correlation::pearson(a, b).unwrap();
        assert!(r.abs() > 1.0 - 1e-9, "correlation {r}");
    }

    fn axis(model: &ConstrainedModel, k: usize) -> Vec<f64> {
        model.ordination.positions.iter().map(|p| p[k]).collect()
    }

    /// The sign that turns our axis `k` into vegan's.
    fn sign(model: &ConstrainedModel, k: usize, vegan: &[f64]) -> f64 {
        crate::stats::correlation::pearson(&axis(model, k), vegan)
            .unwrap()
            .signum()
    }

    #[test]
    fn db_rda_matches_vegan_capscale() {
        let model = db_rda(&bray_curtis(), &[ph(), temp(), site()], 3).unwrap();
        let o = &model.ordination;
        close(o.explained, 0.60856619514608978, 1e-10);
        close(o.adjusted.unwrap(), 0.29541915126296159, 1e-10);
        assert_eq!(o.constrained_axes, 4);
        assert_eq!(o.axis_names, ["dbRDA1", "dbRDA2", "dbRDA3"]);
        for (ours, vegan) in o.axis_fractions.iter().zip([
            0.36868310222418077,
            0.16129501822412556,
            0.052608210229626827,
        ]) {
            close(*ours, vegan, 1e-10);
        }
        let wa1 = [
            -0.47157859225765064,
            0.14100734814603477,
            0.5535986650895196,
            -0.013018765722781895,
            -0.10939000359870681,
            -0.56906696713145566,
            0.43570673101212781,
            0.055765798981523319,
            0.13804425769927456,
            -0.1610684722178852,
        ];
        let wa2 = [
            -0.0025407474677882286,
            -0.43293588876421146,
            0.045423714945681898,
            0.66067627734779499,
            -0.43989328170636721,
            0.32360284373940174,
            0.27841503704612786,
            -0.51386973283317638,
            0.065403776272631525,
            0.015718001419905276,
        ];
        proportional(&axis(&model, 0), &wa1);
        proportional(&axis(&model, 1), &wa2);
        let signs = [sign(&model, 0, &wa1), sign(&model, 1, &wa2)];
        for (name, vegan) in [
            ("pH", [0.90785983605982712, -0.27250669733362237]),
            ("Temp", [-0.41607367777958054, 0.097988208380533681]),
        ] {
            let arrow = o.arrows.iter().find(|a| a.name == name).unwrap();
            for k in 0..2 {
                close(arrow.correlations[k] * signs[k], vegan[k], 1e-9);
            }
        }

        let tests = model.tests(999, &mut rng_from_seed(1));
        close(tests.overall.statistic, 1.943387961130544, 1e-9);
        assert_eq!(tests.overall.df, 4);
        // Permutation p-values agree only roughly (vegan's 0.056).
        close(tests.overall.p_value, 0.056, 0.03);
        let vegan = [
            (3.9674630670541795, 0.003),
            (1.2326092958711532, 0.314),
            (1.3582753379272805, 0.24),
        ];
        assert_eq!(tests.variables.len(), 3);
        for (test, (f, p)) in tests.variables.iter().zip(vegan) {
            close(test.statistic, f, 1e-9);
            close(test.p_value, p, 0.05);
        }
        assert_eq!(tests.variables[2].df, 2);
    }

    #[test]
    fn one_variable_gives_one_constrained_axis_then_unconstrained_ones() {
        let model = db_rda(&bray_curtis(), &[ph()], 3).unwrap();
        let o = &model.ordination;
        close(o.explained, 0.32007162492688568, 1e-10);
        close(o.adjusted.unwrap(), 0.2350805780427464, 1e-10);
        assert_eq!(o.axis_names, ["dbRDA1", "MDS1", "MDS2"]);
        for (ours, vegan) in o.axis_fractions.iter().zip([
            0.32007162492688568,
            0.28268645815155025,
            0.21277351896435129,
        ]) {
            close(*ours, vegan, 1e-10);
        }
        proportional(
            &axis(&model, 1),
            &[
                -0.25791386974346187,
                0.4658095335556473,
                0.43017924171524702,
                0.017821796824874919,
                -0.54862246119479785,
                -0.083431507772200153,
                -0.010028537863444507,
                0.12794022307632238,
                -0.38445835233285292,
                0.24270393373466562,
            ],
        );
        // A single numeric variable lies along the constrained axis.
        close(o.arrows[0].correlations[0].abs(), 1.0, 1e-12);
        assert_eq!(&o.arrows[0].correlations[1..], [0.0, 0.0]);
        let tests = model.tests(199, &mut rng_from_seed(1));
        close(tests.overall.statistic, 3.7659451984773269, 1e-9);
        assert!(tests.variables.is_empty());
    }

    #[test]
    fn cca_matches_vegan() {
        let model = cca(&table(), &[ph(), temp(), site()], 3).unwrap();
        let o = &model.ordination;
        close(o.explained, 0.57374494426395528, 1e-10);
        // vegan estimates the adjusted R² of a CCA by permutation.
        assert_eq!(o.adjusted, None);
        assert_eq!(o.axis_names, ["CCA1", "CCA2", "CCA3"]);
        for (ours, vegan) in o.axis_fractions.iter().zip([
            0.39220753265073077,
            0.12478177815177308,
            0.05441301763514008,
        ]) {
            close(*ours, vegan, 1e-10);
        }
        let wa1 = [
            -1.427401523146778,
            0.35758831567415006,
            1.6301100869743357,
            0.28502492427108594,
            -0.13677468469129608,
            -1.7033541953704792,
            1.3737728321007907,
            0.054603759138203915,
            0.50987919778832747,
            -0.49359486048338519,
        ];
        let wa2 = [
            -0.59308000029508423,
            -0.74968741309559561,
            -0.34966788090234968,
            2.4684818162452529,
            -1.5432502827614853,
            1.2123336284218225,
            1.0085354463644802,
            -1.9429464741709306,
            -0.19260534300054682,
            0.61819135862200969,
        ];
        proportional(&axis(&model, 0), &wa1);
        proportional(&axis(&model, 1), &wa2);
        let signs = [sign(&model, 0, &wa1), sign(&model, 1, &wa2)];
        for (name, vegan) in [
            ("pH", [0.90573560403297626, -0.33500547207941445]),
            ("Temp", [-0.40266874216246656, 0.27687991273991069]),
        ] {
            let arrow = o.arrows.iter().find(|a| a.name == name).unwrap();
            for k in 0..2 {
                close(arrow.correlations[k] * signs[k], vegan[k], 1e-9);
            }
        }
        let tests = model.tests(999, &mut rng_from_seed(1));
        close(tests.overall.statistic, 1.6825165371740565, 1e-9);
        close(tests.overall.p_value, 0.116, 0.04);
        let vegan = [
            (3.9112319566800253, 0.003),
            (1.3427690806974635, 0.272),
            (0.89532757864721046, 0.555),
        ];
        for (test, (f, p)) in tests.variables.iter().zip(vegan) {
            close(test.statistic, f, 1e-9);
            close(test.p_value, p, 0.05);
        }
    }

    #[test]
    fn cca_of_one_variable_matches_vegan() {
        let model = cca(&table(), &[ph()], 2).unwrap();
        let o = &model.ordination;
        close(o.explained, 0.33933943301878, 1e-10);
        assert_eq!(o.axis_names, ["CCA1", "CA1"]);
        close(o.axis_fractions[1], 0.31063819099407997, 1e-10);
        proportional(
            &axis(&model, 1),
            &[
                -0.68926934022817643,
                1.598915438491038,
                1.2673592083083334,
                -0.029251670339621148,
                -1.7237567134751333,
                -0.31118501491026851,
                0.069327791986502976,
                0.22148487362123317,
                -1.3486786401957394,
                0.80062387098523569,
            ],
        );
        let tests = model.tests(99, &mut rng_from_seed(1));
        close(tests.overall.statistic, 4.1090926261191667, 1e-9);
    }

    #[test]
    fn with_one_grouping_and_a_euclidean_distance_the_test_is_permanova() {
        let matrix = distances(&table(), Distance::Euclidean);
        let groups: Vec<usize> = (0..10).map(|i| i % 3).collect();
        let permanova = crate::stats::community::CommunityTest::Permanova.run(
            &matrix,
            &groups,
            0,
            &mut rng_from_seed(1),
        );
        let model = db_rda(&matrix, &[site()], 2).unwrap();
        let tests = model.tests(0, &mut rng_from_seed(1));
        close(tests.overall.statistic, permanova.statistic, 1e-9);
        assert!(tests.overall.p_value.is_nan());
    }

    #[test]
    fn categories_are_drawn_at_the_centroids_of_their_samples() {
        let model = db_rda(&bray_curtis(), &[ph(), site()], 2).unwrap();
        let o = &model.ordination;
        assert_eq!(o.centroids.len(), 3);
        let a = &o.centroids[0];
        assert_eq!((a.variable.as_str(), a.category.as_str()), ("Site", "A"));
        for k in 0..2 {
            let mean = [0, 3, 6, 9].iter().map(|&i| o.positions[i][k]).sum::<f64>() / 4.0;
            close(a.position[k], mean, 1e-12);
        }
        assert_eq!(o.arrows.len(), 1);
    }

    #[test]
    fn the_tests_do_not_depend_on_where_they_pause() {
        let model = cca(&table(), &[ph(), site()], 2).unwrap();
        let whole = model.tests(50, &mut rng_from_seed(7));
        let mut run = model.test_run(50);
        let mut rng = rng_from_seed(7);
        let mut calls = 0;
        let paused = loop {
            let mut pace = |_: &str, _: usize, _: usize| {
                calls += 1;
                if calls % 13 == 0 {
                    Control::Pause
                } else {
                    Control::Continue
                }
            };
            if let Step::Done(tests) = run.advance(&mut rng, &mut pace) {
                break tests;
            }
        };
        assert_eq!(whole, paused);
    }

    #[test]
    fn unusable_variables_are_explained() {
        let constant = Variable {
            name: "Depth".to_string(),
            values: Values::Numbers(vec![1.0; 10]),
        };
        let error = db_rda(&bray_curtis(), &[constant], 2).unwrap_err();
        assert!(error.contains("do not vary"), "{error}");
        let names = Variable {
            name: "Sample".to_string(),
            values: Values::Categories((0..10).map(|i| i.to_string()).collect()),
        };
        let error = db_rda(&bray_curtis(), &[names], 2).unwrap_err();
        assert!(error.contains("Too many"), "{error}");
        let missing = Variable {
            name: "pH".to_string(),
            values: Values::Numbers(vec![f64::NAN; 10]),
        };
        assert!(cca(&table(), &[missing], 2).is_err());
        assert!(db_rda(&bray_curtis(), &[], 2).is_err());
    }

    #[test]
    fn permuted_statistics_match_vegan() {
        let variables = [ph(), temp(), site()];
        let permutations = [
            [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 0],
            [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            [2, 0, 1, 5, 3, 4, 8, 6, 7, 9],
        ];
        let cases = [
            (
                cca(&table(), &variables, 2).unwrap(),
                vec![2],
                [0.987447069898537, 0.895327578647209, 1.050286936274562],
            ),
            (
                cca(&table(), &variables, 2).unwrap(),
                vec![0],
                [0.402894741482948, 0.723568047852394, 0.452771736373422],
            ),
            (
                cca(&table(), &variables, 2).unwrap(),
                vec![0, 1, 2],
                [0.906858240189294, 0.558747396641928, 0.726927421587267],
            ),
            (
                db_rda(&bray_curtis(), &variables, 2).unwrap(),
                vec![2],
                [1.24208051086687, 1.35827533792728, 1.77062337765818],
            ),
        ];
        for (model, tested, vegan) in cases {
            let mut test = OneTest::new(&model.data, String::new(), &tested);
            for (order, f) in permutations.iter().zip(vegan) {
                test.shuffled = order.to_vec();
                let ours = ConstrainedTestRun::statistic_of(&model.data, &test);
                close(ours, f, 1e-9);
            }
        }
    }
}
