//! Statistics computed by the Rust port must reproduce the reference tables
//! produced by the Python implementation.
//!
//! Deliberate differences (see docs/CHANGES_FROM_PYTHON.md) are checked
//! explicitly rather than skipped:
//!   * Benjamini-Hochberg values are the running minimum of Python's values;
//!   * Bonferroni values are capped at one;
//!   * the equal-variance t-test handles zero variance like Welch's test;
//!   * the group ratio of proportions is 1 where Python divided by zero;
//!   * Games-Howell does not crash on single-sample groups;
//!   * Welch's post-hoc test reports percentages (Python was 10,000x larger).
//!
//! Resampling methods are compared within a statistical tolerance.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::effect_size::{GroupEffectSize, SampleEffectSize};
use stamp_core::stats::multi_group::{MultiGroupEffectSize, MultiGroupTest};
use stamp_core::stats::post_hoc::{PostHocP, PostHocTest};
use stamp_core::stats::results::{
    run_group_analysis, run_multi_group_analysis, run_post_hoc, run_sample_analysis, GroupAnalysis,
    MultiGroupAnalysis, SampleAnalysis,
};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::{rng_from_seed, StatsSettings, TestType};
use stamp_core::{
    GroupProfile, Metadata, MultiGroupProfile, ProfileTree, SampleProfile, SecondGroup,
    Unclassified, ALL_OTHER_SAMPLES,
};

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

#[derive(Deserialize, Clone)]
struct Case {
    id: String,
    file: String,
    mode: String,
    key_columns: usize,
    stochastic: bool,
    profile: Option<String>,
    metadata: Option<String>,
    group_field: Option<String>,
    sample1: Option<String>,
    sample2: Option<String>,
    group1: Option<String>,
    group2: Option<String>,
    groups: Option<Vec<String>>,
    parent_level: Option<String>,
    profile_level: Option<String>,
    unclassified: Option<String>,
    test: Option<String>,
    test_type: Option<String>,
    ci_method: Option<String>,
    coverage: Option<f64>,
    source_case: Option<String>,
    estimated_pi0: Option<f64>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let bytes = fs::read(repo_root().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"));
    stamp_core::text::decode(&bytes).into_owned()
}

fn cases(mode: &str) -> Vec<Case> {
    let manifest: Manifest = serde_json::from_str(&read("tests/golden/manifest.json")).unwrap();
    manifest
        .cases
        .into_iter()
        .filter(|c| c.mode == mode)
        .collect()
}

/// A reference table with rows keyed by their first `key_columns` cells.
struct Table {
    id: String,
    columns: HashMap<String, usize>,
    headings: Vec<String>,
    rows: HashMap<Vec<String>, Vec<String>>,
}

impl Table {
    fn load(case: &Case) -> Table {
        let text = read(&format!("tests/golden/{}", case.file));
        let mut lines = text.split('\n').filter(|l| !l.is_empty());
        let headings: Vec<String> = lines
            .next()
            .unwrap()
            .split('\t')
            .map(str::to_string)
            .collect();
        let columns = headings
            .iter()
            .enumerate()
            .map(|(i, h)| (h.clone(), i))
            .collect();
        let mut rows = HashMap::new();
        for line in lines {
            let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
            assert!(rows
                .insert(cells[..case.key_columns].to_vec(), cells)
                .is_none());
        }
        Table {
            id: case.id.clone(),
            columns,
            headings,
            rows,
        }
    }

    fn column(&self, name: &str) -> usize {
        *self
            .columns
            .get(name)
            .unwrap_or_else(|| panic!("{}: no column {name}", self.id))
    }

    /// The column whose heading ends with `suffix`.
    fn column_ending(&self, suffix: &str) -> usize {
        self.headings
            .iter()
            .position(|h| h.ends_with(suffix))
            .unwrap_or_else(|| panic!("{}: no column ending {suffix}", self.id))
    }

    fn row(&self, key: &[String]) -> &Vec<String> {
        self.rows
            .get(key)
            .unwrap_or_else(|| panic!("{}: no reference row for {key:?}", self.id))
    }
}

fn number(cell: &str) -> f64 {
    cell.parse()
        .unwrap_or_else(|_| panic!("not a number: {cell:?}"))
}

/// Relative agreement, with an absolute floor for values near zero. NaN
/// matches NaN and infinities must match exactly.
fn agree(expected: f64, actual: f64, relative: f64, absolute: f64) -> bool {
    if expected.is_nan() || actual.is_nan() {
        return expected.is_nan() && actual.is_nan();
    }
    if expected.is_infinite() || actual.is_infinite() {
        return expected == actual;
    }
    (expected - actual).abs() <= absolute + relative * expected.abs()
}

#[track_caller]
fn check(context: &str, what: &str, expected: f64, actual: f64, relative: f64, absolute: f64) {
    assert!(
        agree(expected, actual, relative, absolute),
        "{context}: {what}: expected {expected:e}, got {actual:e}"
    );
}

/// p-values: SciPy and this port round differently in the last digits, and
/// several tests compute `1 - cdf`, which loses relative precision near zero.
const P_REL: f64 = 1e-8;
const P_ABS: f64 = 1e-12;

/// Tolerances for a two-sample test's p-values. The exact tests exponentiate
/// differences of log-factorials of size ~1e6, which limits both
/// implementations to about seven significant digits, and the hypergeometric
/// test then takes `1 - cdf` of a sum of thousands of such terms.
fn p_tolerance(test: SampleTest) -> (f64, f64) {
    match test {
        SampleTest::Fisher | SampleTest::GTestFisher => (1e-6, 1e-12),
        SampleTest::Hypergeometric => (1e-6, 1e-8),
        _ => (P_REL, P_ABS),
    }
}
/// Confidence limits depend on quantile functions that SciPy 1.2 evaluates to
/// about nine significant digits.
const CI_REL: f64 = 5e-8;
const CI_ABS: f64 = 1e-9;
/// Compare confidence limits, allowing an error proportional to the width of
/// the interval: a limit close to zero is the difference of two larger numbers.
#[track_caller]
fn check_ci(context: &str, expected: (f64, f64), actual: (f64, f64), scale: f64) {
    let (lower, upper) = (expected.0 * scale, expected.1 * scale);
    let slack = CI_ABS + CI_REL * (upper - lower).abs();
    let slack = if slack.is_finite() { slack } else { CI_ABS };
    check(context, "lower CI", lower, actual.0, CI_REL, slack);
    check(context, "upper CI", upper, actual.1, CI_REL, slack);
}

/// Plain arithmetic.
const EXACT_REL: f64 = 1e-11;
const EXACT_ABS: f64 = 1e-12;

fn unclassified(case: &Case) -> Unclassified {
    match case.unclassified.as_deref().unwrap() {
        "Retain unclassified reads" => Unclassified::Retain,
        "Remove unclassified reads" => Unclassified::Remove,
        "Use only for calculating frequency profiles" => Unclassified::FrequencyOnly,
        other => panic!("unknown treatment {other}"),
    }
}

fn parent_level(case: &Case) -> Option<&str> {
    match case.parent_level.as_deref().unwrap() {
        "Entire sample" => None,
        level => Some(level),
    }
}

fn test_type(case: &Case) -> TestType {
    match case.test_type.as_deref().unwrap() {
        "One-sided" => TestType::OneSided,
        "Two-sided" => TestType::TwoSided,
        other => panic!("unknown test type {other}"),
    }
}

fn sample_profile(case: &Case) -> SampleProfile {
    let tree = ProfileTree::read(&read(case.profile.as_deref().unwrap())).unwrap();
    tree.sample_profile(
        case.sample1.as_deref().unwrap(),
        case.sample2.as_deref().unwrap(),
        parent_level(case),
        case.profile_level.as_deref().unwrap(),
        unclassified(case),
    )
    .unwrap()
}

fn tree_and_groups(case: &Case) -> (ProfileTree, stamp_core::Groups) {
    let tree = ProfileTree::read(&read(case.profile.as_deref().unwrap())).unwrap();
    let (metadata, _) = Metadata::read(&read(case.metadata.as_deref().unwrap()), &tree).unwrap();
    let groups = metadata
        .groups(case.group_field.as_deref().unwrap())
        .unwrap();
    (tree, groups)
}

fn group_profile(case: &Case) -> GroupProfile {
    let (tree, groups) = tree_and_groups(case);
    let group2 = match case.group2.as_deref().unwrap() {
        ALL_OTHER_SAMPLES => SecondGroup::AllOtherSamples,
        name => SecondGroup::Named(name),
    };
    tree.group_profile(
        &groups,
        case.group1.as_deref().unwrap(),
        group2,
        parent_level(case),
        case.profile_level.as_deref().unwrap(),
        unclassified(case),
    )
    .unwrap()
}

fn multi_group_profile(case: &Case) -> MultiGroupProfile {
    let (tree, groups) = tree_and_groups(case);
    let active: Vec<&str> = case
        .groups
        .as_ref()
        .unwrap()
        .iter()
        .map(String::as_str)
        .collect();
    tree.multi_group_profile(
        &groups,
        Some(&active),
        parent_level(case),
        case.profile_level.as_deref().unwrap(),
        unclassified(case),
    )
    .unwrap()
}

/// Check the deterministic corrections of a set of p-values against the
/// reference columns. `rows` pairs each reference row with its p-value.
fn check_corrections(
    table: &Table,
    rows: &[(&Vec<String>, f64)],
    alpha: f64,
    (relative, absolute): (f64, f64),
) {
    let p_values: Vec<f64> = rows.iter().map(|(_, p)| *p).collect();
    // Ties are identified from Python's p-values: values that are exactly
    // tied there can differ in the last digit here.
    let c_p = table.column("p-values");
    let python_p: Vec<f64> = rows.iter().map(|(row, _)| number(&row[c_p])).collect();
    let mut rng = rng_from_seed(0);
    let column = |name: &str| table.column(&format!("corr:{name}"));

    let bonferroni = Correction::Bonferroni
        .correct(&p_values, alpha, &mut rng)
        .values;
    let sidak = Correction::Sidak.correct(&p_values, alpha, &mut rng).values;
    let bh = Correction::BenjaminiHochberg
        .correct(&p_values, alpha, &mut rng)
        .values;
    let (c_bonferroni, c_sidak, c_bh) = (
        column("Bonferroni"),
        column("Sidak"),
        column("Benjamini-Hochberg FDR"),
    );

    for (i, (row, _)) in rows.iter().enumerate() {
        // Python did not cap Bonferroni-corrected values at one.
        let expected = number(&row[c_bonferroni]).min(1.0);
        let n = rows.len() as f64;
        check(
            &table.id,
            "Bonferroni",
            expected,
            bonferroni[i],
            relative,
            absolute * n,
        );
        check(
            &table.id,
            "Sidak",
            number(&row[c_sidak]),
            sidak[i],
            relative,
            absolute * n + 1e-11,
        );
    }

    // Python's Benjamini-Hochberg values are p * n / rank without the running
    // minimum; applying it (from the largest p-value down, over groups of
    // tied p-values) must give this port's values.
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| python_p[b].total_cmp(&python_p[a]));
    let mut running_min: f64 = 1.0;
    let mut start = 0;
    while start < order.len() {
        let mut end = start;
        while end < order.len() && python_p[order[end]] == python_p[order[start]] {
            end += 1;
        }
        for &i in &order[start..end] {
            let python = number(&rows[i].0[c_bh]);
            if !python.is_nan() {
                running_min = running_min.min(python);
            }
        }
        for &i in &order[start..end] {
            check(
                &table.id,
                "Benjamini-Hochberg",
                running_min,
                bh[i],
                relative,
                absolute * rows.len() as f64,
            );
        }
        start = end;
    }
}

#[test]
fn two_sample_statistics_match_python() {
    let settings = StatsSettings::default();
    let mut checked = 0;
    for case in cases("two_samples") {
        let profile = sample_profile(&case);
        let table = Table::load(&case);
        let test = SampleTest::from_name(case.test.as_deref().unwrap()).unwrap();
        let ci_method = SampleCi::from_name(case.ci_method.as_deref().unwrap()).unwrap();
        let coverage = case.coverage.unwrap();
        assert_eq!(test.is_stochastic(), case.stochastic, "{}", case.id);

        let analysis = SampleAnalysis {
            test,
            test_type: test_type(&case),
            ci_method,
            coverage,
            correction: Correction::None,
        };
        let results = run_sample_analysis(
            &profile,
            analysis,
            &settings,
            &mut rng_from_seed(42),
            &mut |_, _| true,
        )
        .unwrap();
        assert_eq!(results.rows.len(), table.rows.len(), "{}", case.id);

        let (c_p, c_es, c_note) = (
            table.column("p-values"),
            table.column("Effect size"),
            table.column("Note"),
        );
        let (c_lower, c_upper) = (
            table.column_ending("% lower CI"),
            table.column_ending("% upper CI"),
        );

        let mut correction_rows = Vec::new();
        for row in &results.rows {
            let feature = profile.profile.feature(&row.feature).unwrap();
            let expected = table.row(&feature.hierarchy);
            let context = format!("{} {:?}", case.id, feature.hierarchy);

            if case.stochastic {
                // 1000 replicates: the standard error of a p-value is at most 0.016.
                check(
                    &context,
                    "p-value",
                    number(&expected[c_p]),
                    row.p_value,
                    0.0,
                    0.07,
                );
            } else {
                let (relative, absolute) = p_tolerance(test);
                check(
                    &context,
                    "p-value",
                    number(&expected[c_p]),
                    row.p_value,
                    relative,
                    absolute,
                );
                assert_eq!(row.p_value_corrected, row.p_value);
                assert_eq!(row.note, expected[c_note], "{context}");
                correction_rows.push((expected, row.p_value));
            }
            check(
                &context,
                "effect size",
                number(&expected[c_es]),
                row.effect_size,
                CI_REL,
                CI_ABS,
            );
            let python_ci = (number(&expected[c_lower]), number(&expected[c_upper]));
            check_ci(&context, python_ci, (row.lower_ci, row.upper_ci), 1.0);

            for measure in SampleEffectSize::ALL {
                let value = measure.run(
                    row.seq1,
                    row.seq2,
                    row.parent_seq1,
                    row.parent_seq2,
                    settings.pseudocount,
                );
                let column = table.column(&format!("es:{}", measure.name()));
                check(
                    &context,
                    measure.name(),
                    number(&expected[column]),
                    value,
                    EXACT_REL,
                    EXACT_ABS,
                );
            }
        }
        if !case.stochastic {
            check_corrections(&table, &correction_rows, 1.0 - coverage, p_tolerance(test));
        }
        checked += 1;
    }
    assert!(checked >= 43, "only {checked} two-sample cases checked");
}

#[test]
fn two_group_statistics_match_python() {
    let settings = StatsSettings::default();
    let mut checked = 0;
    let mut zero_variance_rows = 0;
    let mut ratio_fixes = 0;
    for case in cases("two_groups") {
        let profile = group_profile(&case);
        let table = Table::load(&case);
        let test = GroupTest::from_name(case.test.as_deref().unwrap()).unwrap();
        let coverage = case.coverage.unwrap();
        assert_eq!(test.is_stochastic(), case.stochastic, "{}", case.id);

        let analysis = GroupAnalysis {
            test,
            test_type: test_type(&case),
            coverage,
            correction: Correction::None,
            transform: stamp_core::stats::results::Transform::None,
        };
        let results = run_group_analysis(
            &profile,
            analysis,
            &settings,
            &mut rng_from_seed(42),
            &mut |_, _| true,
        )
        .unwrap();
        assert_eq!(results.rows.len(), table.rows.len(), "{}", case.id);

        let k = case.key_columns;
        let (c_p, c_es, c_note) = (
            table.column("p-values"),
            table.column("Difference between means"),
            table.column("Note"),
        );
        let (c_lower, c_upper) = (
            table.column_ending("% lower CI"),
            table.column_ending("% upper CI"),
        );

        let mut correction_rows = Vec::new();
        for row in &results.rows {
            let feature = profile.profile.feature(&row.feature).unwrap();
            let expected = table.row(&feature.hierarchy);
            let context = format!("{} {:?}", case.id, feature.hierarchy);

            for (offset, value) in [row.mean1, row.std_dev1, row.mean2, row.std_dev2]
                .into_iter()
                .enumerate()
            {
                check(
                    &context,
                    "mean/sd",
                    number(&expected[k + offset]),
                    value,
                    1e-9,
                    1e-12,
                );
            }

            let python_note = expected[c_note].as_str();
            if python_note == "float division by zero" {
                // Python's t-test fell into the wrong error handler when both
                // groups had zero variance, and reported no difference at all.
                assert_eq!(
                    row.note, "degenerate case: variance of both groups is zero",
                    "{context}"
                );
                check(
                    &context,
                    "effect size",
                    row.mean1 - row.mean2,
                    row.effect_size,
                    1e-9,
                    1e-12,
                );
                assert_eq!(
                    (row.lower_ci, row.upper_ci),
                    (row.effect_size, row.effect_size)
                );
                zero_variance_rows += 1;
            } else if case.stochastic
                && python_note != "heuristic: statistics calculated with Fisher's test"
            {
                check(
                    &context,
                    "effect size",
                    number(&expected[c_es]),
                    row.effect_size,
                    1e-9,
                    1e-12,
                );
                check(
                    &context,
                    "p-value",
                    number(&expected[c_p]),
                    row.p_value,
                    0.0,
                    0.05,
                );
                // Bootstrap limits vary between runs by a fraction of the interval.
                let width = number(&expected[c_upper]) - number(&expected[c_lower]);
                let slack = 0.3 * width.abs() + 1e-9;
                check(
                    &context,
                    "lower CI",
                    number(&expected[c_lower]),
                    row.lower_ci,
                    0.0,
                    slack,
                );
                check(
                    &context,
                    "upper CI",
                    number(&expected[c_upper]),
                    row.upper_ci,
                    0.0,
                    slack,
                );
            } else {
                assert_eq!(row.note, python_note, "{context}");
                check(
                    &context,
                    "p-value",
                    number(&expected[c_p]),
                    row.p_value,
                    P_REL,
                    P_ABS,
                );
                check(
                    &context,
                    "effect size",
                    number(&expected[c_es]),
                    row.effect_size,
                    1e-9,
                    1e-12,
                );
                let python_ci = (number(&expected[c_lower]), number(&expected[c_upper]));
                check_ci(&context, python_ci, (row.lower_ci, row.upper_ci), 1.0);
                if !case.stochastic {
                    correction_rows.push((expected, row.p_value));
                }
            }

            let (percent1, percent2) = profile.percentages(feature);
            for measure in GroupEffectSize::ALL {
                let value = measure.magnitude(&percent1, &percent2, settings.pseudocount);
                let cell = &expected[table.column(&format!("es:{}", measure.name()))];
                if cell == "error: ZeroDivisionError" {
                    assert_eq!(value, 1.0, "{context}");
                    ratio_fixes += 1;
                } else {
                    check(&context, measure.name(), number(cell), value, 1e-9, 1e-12);
                }
            }
        }

        // Corrections are only comparable when every p-value matches Python's.
        if !case.stochastic && correction_rows.len() == results.rows.len() {
            check_corrections(&table, &correction_rows, 1.0 - coverage, (1e-7, P_ABS));
        }
        checked += 1;
    }
    assert!(checked >= 26, "only {checked} two-group cases checked");
    assert!(
        zero_variance_rows > 0,
        "no zero-variance t-test rows were exercised"
    );
    assert!(
        ratio_fixes > 0,
        "no ratio-of-proportions fixes were exercised"
    );
}

#[test]
fn multi_group_statistics_match_python() {
    let mut checked = 0;
    for case in cases("multiple_groups") {
        let profile = multi_group_profile(&case);
        let table = Table::load(&case);
        let analysis = MultiGroupAnalysis {
            test: MultiGroupTest::from_name(case.test.as_deref().unwrap()).unwrap(),
            effect_size: MultiGroupEffectSize::EtaSquared,
            correction: Correction::None,
            transform: stamp_core::stats::results::Transform::None,
        };
        let results =
            run_multi_group_analysis(&profile, analysis, &mut rng_from_seed(42), &mut |_, _| true)
                .unwrap();
        assert_eq!(results.rows.len(), table.rows.len(), "{}", case.id);

        let k = case.key_columns;
        let (c_p, c_es, c_note) = (
            table.column("p-values"),
            table.column("Effect size"),
            table.column("Note"),
        );

        let mut correction_rows = Vec::new();
        for row in &results.rows {
            let feature = profile.profile.feature(&row.feature).unwrap();
            let expected = table.row(&feature.hierarchy);
            let context = format!("{} {:?}", case.id, feature.hierarchy);

            check(
                &context,
                "p-value",
                number(&expected[c_p]),
                row.p_value,
                P_REL,
                P_ABS,
            );
            check(
                &context,
                "eta-squared",
                number(&expected[c_es]),
                row.effect_size,
                1e-8,
                1e-11,
            );
            assert_eq!(row.note, expected[c_note], "{context}");
            for g in 0..profile.group_names.len() {
                let (mean, sd) = (
                    number(&expected[k + 4 + 2 * g]),
                    number(&expected[k + 5 + 2 * g]),
                );
                check(
                    &context,
                    "group mean",
                    mean,
                    row.group_means[g],
                    1e-9,
                    1e-12,
                );
                check(&context, "group sd", sd, row.group_std_devs[g], 1e-9, 1e-12);
            }
            correction_rows.push((expected, row.p_value));
        }
        check_corrections(&table, &correction_rows, 0.05, (1e-7, P_ABS));
        checked += 1;
    }
    assert!(checked >= 18, "only {checked} multiple-group cases checked");
}

#[test]
fn post_hoc_statistics_match_python() {
    let parents: HashMap<String, Case> = cases("multiple_groups")
        .into_iter()
        .map(|c| (c.id.clone(), c))
        .collect();
    let mut checked = 0;
    let (mut crashes_fixed, mut welch_rows, mut bounds) = (0, 0, 0);
    let mut pooled_fixed = 0;
    /// How far a confidence limit may be from Python's table-based one.
    const STUDENTIZED_CI_REL: f64 = 5e-3;
    for case in cases("post_hoc") {
        let parent = &parents[case.id.strip_suffix("__posthoc").unwrap()];
        let profile = multi_group_profile(parent);
        let table = Table::load(&case);
        let coverage = case.coverage.unwrap();

        // Group the reference rows by feature and test.
        let mut expected: HashMap<(String, String), Vec<&Vec<String>>> = HashMap::new();
        for row in table.rows.values() {
            expected
                .entry((row[0].clone(), row[1].clone()))
                .or_default()
                .push(row);
        }

        for ((feature, test_name), rows) in &expected {
            let test = PostHocTest::from_name(test_name).unwrap();
            let result = run_post_hoc(&profile, feature, test, coverage).unwrap();
            let context = format!("{} {feature} {test_name}", case.id);

            if rows.len() == 1 && rows[0][2].starts_with("error:") {
                // Python crashed (Games-Howell with a single-sample group).
                assert_eq!(test, PostHocTest::GamesHowell);
                assert!(!result.note.is_empty(), "{context}");
                assert!(result.comparisons.iter().any(|c| c.lower_ci.is_nan()));
                crashes_fixed += 1;
                continue;
            }

            assert_eq!(result.comparisons.len(), rows.len(), "{context}");
            // Python reported Welch's effect sizes and limits 10,000 times too large.
            let scale = if test == PostHocTest::WelchUncorrected {
                1e-4
            } else {
                1.0
            };
            for comparison in &result.comparisons {
                let row = rows
                    .iter()
                    .find(|r| r[2] == comparison.label)
                    .unwrap_or_else(|| panic!("{context}: no row for {}", comparison.label));
                let context = format!("{context} {}", comparison.label);

                let PostHocP::Value(p) = comparison.p_value;
                // A group of one sample made Python's pooled variance, and
                // with it the whole comparison, undefined.
                if number(&row[5]).is_nan()
                    && matches!(test, PostHocTest::TukeyKramer | PostHocTest::Scheffe)
                {
                    assert!(
                        p.is_finite() && comparison.lower_ci.is_finite(),
                        "{context}: {comparison:?}"
                    );
                    pooled_fixed += 1;
                    continue;
                }
                let studentized =
                    matches!(test, PostHocTest::TukeyKramer | PostHocTest::GamesHowell);
                if studentized {
                    // Python read a bound from tables; the computed value
                    // must fall within it. The slack covers the tables'
                    // inaccuracy at few degrees of freedom and Python's
                    // interpolation in them for Games-Howell.
                    let levels = [0.0, 0.001, 0.01, 0.02, 0.05, 0.1, 1.0];
                    let (low, high) = match row[3].as_str() {
                        ">= 0.1" => (0.1, 1.0),
                        bound => {
                            let level: f64 = bound.trim_start_matches("< ").parse().unwrap();
                            let at = levels.iter().position(|l| *l == level).unwrap();
                            (levels[at - 1], level)
                        }
                    };
                    assert!(
                        p >= low * 0.8 - 1e-12 && p <= high * 1.25,
                        "{context}: p-value {p} outside Python's {}",
                        row[3]
                    );
                    bounds += 1;
                } else {
                    check(&context, "p-value", number(&row[3]), p, P_REL, 1e-11);
                }
                check(
                    &context,
                    "effect size",
                    number(&row[4]) * scale,
                    comparison.effect_size,
                    1e-8,
                    1e-11,
                );
                let python_ci = (number(&row[5]), number(&row[6]));
                if studentized {
                    // Python's limits came from its tables of critical
                    // values, which it interpolated for Games-Howell; the
                    // computed limits are close to them, not equal.
                    let python_half = (python_ci.1 - python_ci.0) / 2.0;
                    let half = (comparison.upper_ci - comparison.lower_ci) / 2.0;
                    let difference = (half - python_half).abs() / python_half;
                    assert!(
                        difference < STUDENTIZED_CI_REL,
                        "{context}: half-width {half} against Python's {python_half}"
                    );
                    let centre = (comparison.upper_ci + comparison.lower_ci) / 2.0;
                    check(
                        &context,
                        "CI centre",
                        comparison.effect_size,
                        centre,
                        1e-8,
                        1e-9,
                    );
                } else {
                    check_ci(
                        &context,
                        python_ci,
                        (comparison.lower_ci, comparison.upper_ci),
                        scale,
                    );
                }
                assert_eq!(result.note, row[7], "{context}");
                if test == PostHocTest::WelchUncorrected {
                    welch_rows += 1;
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 18, "only {checked} post-hoc cases checked");
    assert!(crashes_fixed > 0 && welch_rows > 0 && bounds > 0);
    assert!(pooled_fixed > 0);
}

#[test]
fn storey_q_values_are_consistent_with_python() {
    let all: HashMap<String, Case> = ["two_samples", "two_groups", "multiple_groups"]
        .into_iter()
        .flat_map(cases)
        .map(|c| (c.id.clone(), c))
        .collect();
    let mut checked = 0;
    for case in cases("storey_fdr") {
        assert!(all.contains_key(case.source_case.as_deref().unwrap()));
        let table = Table::load(&case);
        let rows: Vec<&Vec<String>> = table.rows.values().collect();
        let (c_p, c_q) = (table.column("p-values"), table.column("q-values"));
        let p_values: Vec<f64> = rows.iter().map(|r| number(&r[c_p])).collect();

        let corrected = Correction::Storey.correct(&p_values, 0.05, &mut rng_from_seed(42));
        let pi0 = corrected
            .info
            .iter()
            .find(|(name, _)| *name == "Estimated pi_0")
            .unwrap()
            .1;
        let python_pi0 = case.estimated_pi0.unwrap();

        // The estimate of pi_0 depends on a bootstrap choice of lambda, so it
        // varies between runs; q-values are proportional to it.
        assert!(
            (pi0 - python_pi0).abs() <= 0.25,
            "{}: pi0 {pi0} vs {python_pi0}",
            case.id
        );
        for (row, q) in rows.iter().zip(&corrected.values) {
            let python_q = number(&row[c_q]);
            if python_pi0 > 0.0 && pi0 > 0.0 {
                check(
                    &case.id,
                    "q-value",
                    python_q / python_pi0,
                    q / pi0,
                    1e-9,
                    1e-13,
                );
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 3);
}
