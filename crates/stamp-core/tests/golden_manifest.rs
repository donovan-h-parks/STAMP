//! Checks that the reference outputs from the Python implementation are
//! present and consistent with their manifest. The statistical comparisons
//! against these files are added as each test is ported.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
    load_errors: Vec<LoadError>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    file: String,
    rows: usize,
    key_columns: usize,
    mode: String,
    stochastic: bool,
}

#[derive(Deserialize)]
struct LoadError {
    profile: String,
    error: String,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest() -> Manifest {
    let path = repo_root().join("tests/golden/manifest.json");
    let text = fs::read_to_string(&path).expect("tests/golden/manifest.json is readable");
    serde_json::from_str(&text).expect("manifest.json parses")
}

#[test]
fn every_case_file_matches_its_manifest_entry() {
    let manifest = manifest();
    assert!(!manifest.cases.is_empty());

    let mut ids = HashSet::new();
    for case in &manifest.cases {
        assert!(ids.insert(case.id.clone()), "duplicate case id {}", case.id);

        let path = repo_root().join("tests/golden").join(&case.file);
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", case.file));
        let mut lines = text.lines();
        let columns = lines.next().expect("header row").split('\t').count();
        assert!(
            case.key_columns >= 1 && case.key_columns < columns,
            "{}",
            case.id
        );

        let mut rows = 0;
        for line in lines {
            assert_eq!(
                line.split('\t').count(),
                columns,
                "ragged row in {}",
                case.file
            );
            rows += 1;
        }
        assert_eq!(rows, case.rows, "row count of {}", case.file);
    }
}

#[test]
fn every_analysis_mode_has_deterministic_cases() {
    let manifest = manifest();
    for mode in ["two_samples", "two_groups", "multiple_groups", "post_hoc"] {
        let n = manifest
            .cases
            .iter()
            .filter(|c| c.mode == mode && !c.stochastic)
            .count();
        assert!(n > 0, "no deterministic reference cases for mode {mode}");
    }
}

#[test]
fn rejected_example_profiles_exist() {
    for entry in manifest().load_errors {
        assert!(
            repo_root().join(&entry.profile).is_file(),
            "{}",
            entry.profile
        );
        assert!(!entry.error.is_empty());
    }
}
