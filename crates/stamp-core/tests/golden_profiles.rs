//! Profiles built by the Rust port must reproduce the counts, parent counts
//! and relative frequencies in every reference table produced by the Python
//! implementation, and the reader must reject the same example files with the
//! same messages.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use stamp_core::profile::percent;
use stamp_core::{Metadata, Profile, ProfileTree, SecondGroup, Unclassified, ALL_OTHER_SAMPLES};

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
    load_errors: Vec<LoadError>,
}

#[derive(Deserialize)]
struct LoadError {
    profile: String,
    error: String,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    file: String,
    mode: String,
    key_columns: usize,
    profile: Option<String>,
    metadata: Option<String>,
    group_field: Option<String>,
    sample1: Option<String>,
    sample2: Option<String>,
    group1: Option<String>,
    group2: Option<String>,
    samples_in_group1: Option<Vec<String>>,
    samples_in_group2: Option<Vec<String>>,
    groups: Option<Vec<String>>,
    samples_in_groups: Option<Vec<Vec<String>>>,
    parent_level: Option<String>,
    profile_level: Option<String>,
    unclassified: Option<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let bytes = fs::read(repo_root().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"));
    stamp_core::text::decode(&bytes).into_owned()
}

fn manifest() -> Manifest {
    serde_json::from_str(&read("tests/golden/manifest.json")).unwrap()
}

/// A reference table: rows keyed by their hierarchy columns.
struct Table {
    rows: HashMap<Vec<String>, Vec<String>>,
}

fn table(case: &Case) -> Table {
    let text = read(&format!("tests/golden/{}", case.file));
    let mut rows = HashMap::new();
    for line in text.lines().skip(1) {
        let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
        let key = cells[..case.key_columns].to_vec();
        assert!(
            rows.insert(key, cells).is_none(),
            "{}: duplicate key",
            case.id
        );
    }
    Table { rows }
}

fn close(case: &str, what: &str, expected: &str, actual: f64) {
    let expected: f64 = expected
        .parse()
        .unwrap_or_else(|_| panic!("{case}: {what}: {expected}"));
    let tolerance = 1e-12 * expected.abs().max(1.0);
    assert!(
        (expected - actual).abs() <= tolerance,
        "{case}: {what}: expected {expected}, got {actual}"
    );
}

fn treatment(case: &Case) -> Unclassified {
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

/// Compare per-sample (count, parent count, relative frequency) triples that
/// start at `first_column` of each reference row.
fn check_sample_triples(case: &Case, profile: &Profile, table: &Table, first_column: usize) {
    assert_eq!(
        profile.num_features(),
        table.rows.len(),
        "{}: number of features",
        case.id
    );
    for feature in profile.features() {
        let row = table
            .rows
            .get(&feature.hierarchy)
            .unwrap_or_else(|| panic!("{}: no reference row for {:?}", case.id, feature.hierarchy));
        let parents = profile.parent_counts(feature);
        for (i, (&count, &parent)) in feature.counts.iter().zip(parents).enumerate() {
            let col = first_column + 3 * i;
            let what = format!("{:?} sample {}", feature.hierarchy, profile.sample_names[i]);
            close(&case.id, &what, &row[col], count);
            close(&case.id, &what, &row[col + 1], parent);
            close(&case.id, &what, &row[col + 2], percent(count, parent));
        }
    }
}

#[test]
fn two_sample_profiles_match_python() {
    let mut checked = 0;
    for case in manifest().cases.iter().filter(|c| c.mode == "two_samples") {
        let tree = ProfileTree::read(&read(case.profile.as_deref().unwrap())).unwrap();
        let profile = tree
            .sample_profile(
                case.sample1.as_deref().unwrap(),
                case.sample2.as_deref().unwrap(),
                parent_level(case),
                case.profile_level.as_deref().unwrap(),
                treatment(case),
            )
            .unwrap();
        let table = table(case);
        assert_eq!(
            profile.profile.num_features(),
            table.rows.len(),
            "{}",
            case.id
        );

        let k = case.key_columns;
        for feature in profile.profile.features() {
            let row = table
                .rows
                .get(&feature.hierarchy)
                .unwrap_or_else(|| panic!("{}: no row for {:?}", case.id, feature.hierarchy));
            let counts = profile.counts(feature);
            let (freq1, freq2) = counts.percentages();
            let what = format!("{:?}", feature.hierarchy);
            close(&case.id, &what, &row[k], counts.seq1);
            close(&case.id, &what, &row[k + 1], counts.seq2);
            close(&case.id, &what, &row[k + 2], counts.parent_seq1);
            close(&case.id, &what, &row[k + 3], counts.parent_seq2);
            close(&case.id, &what, &row[k + 4], freq1);
            close(&case.id, &what, &row[k + 5], freq2);
        }
        checked += 1;
    }
    assert!(checked >= 40, "only {checked} two-sample cases checked");
}

fn tree_and_metadata(case: &Case) -> (ProfileTree, Metadata) {
    let tree = ProfileTree::read(&read(case.profile.as_deref().unwrap())).unwrap();
    let (metadata, _) = Metadata::read(&read(case.metadata.as_deref().unwrap()), &tree).unwrap();
    (tree, metadata)
}

#[test]
fn two_group_profiles_match_python() {
    let mut checked = 0;
    for case in manifest().cases.iter().filter(|c| c.mode == "two_groups") {
        let (tree, metadata) = tree_and_metadata(case);
        let groups = metadata
            .groups(case.group_field.as_deref().unwrap())
            .unwrap();
        let group2 = match case.group2.as_deref().unwrap() {
            ALL_OTHER_SAMPLES => SecondGroup::AllOtherSamples,
            name => SecondGroup::Named(name),
        };
        let profile = tree
            .group_profile(
                &groups,
                case.group1.as_deref().unwrap(),
                group2,
                parent_level(case),
                case.profile_level.as_deref().unwrap(),
                treatment(case),
            )
            .unwrap();

        assert_eq!(
            profile.samples_in_group1(),
            case.samples_in_group1.as_deref().unwrap()
        );
        assert_eq!(
            profile.samples_in_group2(),
            case.samples_in_group2.as_deref().unwrap()
        );

        // Hierarchy columns, then ten statistics columns, then the samples.
        check_sample_triples(case, &profile.profile, &table(case), case.key_columns + 10);
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} two-group cases checked");
}

#[test]
fn multi_group_profiles_match_python() {
    let mut checked = 0;
    for case in manifest()
        .cases
        .iter()
        .filter(|c| c.mode == "multiple_groups")
    {
        let (tree, metadata) = tree_and_metadata(case);
        let groups = metadata
            .groups(case.group_field.as_deref().unwrap())
            .unwrap();
        let active: Vec<&str> = case
            .groups
            .as_ref()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect();
        let profile = tree
            .multi_group_profile(
                &groups,
                Some(&active),
                parent_level(case),
                case.profile_level.as_deref().unwrap(),
                treatment(case),
            )
            .unwrap();

        assert_eq!(
            &profile.group_names,
            case.groups.as_ref().unwrap(),
            "{}",
            case.id
        );
        let expected = case.samples_in_groups.as_ref().unwrap();
        let actual = profile.samples_in_groups();
        assert_eq!(actual.len(), expected.len(), "{}", case.id);
        for (a, e) in actual.iter().zip(expected) {
            assert_eq!(a, e, "{}", case.id);
        }

        // Hierarchy columns, four statistics columns, mean and standard
        // deviation per group, then the samples.
        let first = case.key_columns + 4 + 2 * profile.group_names.len();
        check_sample_triples(case, &profile.profile, &table(case), first);
        checked += 1;
    }
    assert!(checked >= 16, "only {checked} multiple-group cases checked");
}

#[test]
fn rejected_example_profiles_give_the_same_messages() {
    let manifest = manifest();
    assert_eq!(manifest.load_errors.len(), 4);
    for entry in manifest.load_errors {
        let error = ProfileTree::read(&read(&entry.profile)).unwrap_err();
        assert_eq!(error.message(), entry.error, "{}", entry.profile);
    }
}

#[test]
fn every_other_example_profile_loads() {
    for (path, levels, samples) in [
        ("examples/CoalSeamGas/coal.spf", 7, 44),
        ("examples/Eggerthellaceae/eggerthellaceae.spf", 6, 1289),
        (
            "examples/Eggerthellaceae/eggerthellaceae_mgr_species.spf",
            2,
            1289,
        ),
        ("examples/CowRumen-MG-RAST/CowRumen.spf", 4, 4),
        (
            "examples/EnterotypesArumugam/Enterotypes.profile.spf",
            2,
            39,
        ),
        ("examples/IronMineEdwards/EdwardsIronMine.spf", 3, 2),
        ("examples/LargeDataset/profiles.tsv", 1, 600),
        ("examples/Melainabacteria/cog_category_table.spf", 1, 97),
        (
            "examples/NineBiomes-Pfam-Profiles-CoMet/NineBiomes-Pfam.spf",
            1,
            8,
        ),
        ("examples/table_stamp.txt", 7, 62),
        ("examples/heatmap.spf", 1, 4),
        ("examples/CI_example.spf", 2, 2),
        (
            "examples/EBPR/MartinEBPR_A_phosphatis_Phosphate_Metabolism.spf",
            1,
            2,
        ),
        ("legacy/test_case.spf", 3, 6),
    ] {
        let tree = ProfileTree::read(&read(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(tree.hierarchy_headings().len(), levels, "{path}");
        assert_eq!(tree.num_samples(), samples, "{path}");
    }
}
