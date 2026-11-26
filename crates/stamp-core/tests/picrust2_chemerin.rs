//! Real PICRUSt2 2.6.4 outputs for the 16S data of PICRUSt2's tutorial (24
//! mouse stool samples, chemerin knockout and wild type, from two
//! facilities), in `examples/input_file_formats/PICRUSt2`. Each table must be
//! recognised, converted and loaded with the values PICRUSt2 wrote, and the
//! stratified tables must add up to the unstratified ones. See the README
//! there for how the files were made.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use stamp_core::import::profiler::{self, ProfilerFormat};
use stamp_core::import::NamedText;
use stamp_core::ProfileTree;

fn path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/input_file_formats/PICRUSt2")
        .join(relative)
}

/// A file, decompressed.
fn read(relative: &str) -> String {
    let bytes = fs::read(path(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"));
    stamp_core::text::decode(&stamp_core::text::decompress(&bytes).unwrap()).into_owned()
}

/// A table's profile: recognised as `format`, converted, with the levels
/// `levels`, and loaded.
fn profile(relative: &str, format: ProfilerFormat, levels: &[&str]) -> ProfileTree {
    let text = read(relative);
    assert_eq!(ProfilerFormat::detect(&text), Some(format), "{relative}");
    let spf = format
        .to_spf(&[NamedText {
            name: relative,
            text: &text,
        }])
        .unwrap_or_else(|e| panic!("{relative}: {e}"));
    let tree = ProfileTree::read(&spf).unwrap_or_else(|e| panic!("{relative}: {e}"));
    assert_eq!(tree.hierarchy_headings(), levels, "{relative}");
    tree
}

/// The table PICRUSt2 wrote: each feature's value in each sample.
fn table(relative: &str) -> (Vec<String>, HashMap<String, Vec<f64>>) {
    let text = read(relative);
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    let skip = if header[1] == "description" { 2 } else { 1 };
    let samples = header[skip..].iter().map(|s| s.to_string()).collect();
    let rows = lines
        .map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            let values = cells[skip..].iter().map(|c| c.parse().unwrap()).collect();
            (cells[0].to_string(), values)
        })
        .collect();
    (samples, rows)
}

/// The value of each feature at the top level in one sample.
fn top_level(tree: &ProfileTree, sample: &str) -> HashMap<String, f64> {
    let column = tree
        .sample_names()
        .iter()
        .position(|s| s == sample)
        .unwrap();
    let mut values = HashMap::new();
    for (lineage, row) in tree.rows() {
        let id = lineage[0].split(": ").next().unwrap().to_string();
        *values.entry(id).or_insert(0.0) += row[column];
    }
    values
}

/// Every value of the profile agrees with PICRUSt2's table, to the six
/// significant digits STAMP keeps.
fn agrees_with(tree: &ProfileTree, relative: &str) {
    let (samples, rows) = table(relative);
    assert_eq!(tree.sample_names(), samples, "{relative}");
    for (column, sample) in samples.iter().enumerate() {
        let values = top_level(tree, sample);
        for (feature, expected) in &rows {
            let expected = expected[column];
            let actual = values.get(feature).copied().unwrap_or(0.0);
            assert!(
                (actual - expected).abs() <= 1e-5 * expected.max(1.0),
                "{relative}: {feature} in {sample}: {actual} against {expected}"
            );
        }
    }
}

#[test]
fn predicted_functions() {
    let ec = profile(
        "chemerin/EC_metagenome_out/pred_metagenome_unstrat.tsv.gz",
        ProfilerFormat::Picrust2Functions,
        &["EC"],
    );
    assert_eq!(ec.num_samples(), 24);
    agrees_with(
        &ec,
        "chemerin/EC_metagenome_out/pred_metagenome_unstrat.tsv.gz",
    );
    let ko = profile(
        "chemerin/KO_metagenome_out/pred_metagenome_unstrat.tsv.gz",
        ProfilerFormat::Picrust2Functions,
        &["KO"],
    );
    agrees_with(
        &ko,
        "chemerin/KO_metagenome_out/pred_metagenome_unstrat.tsv.gz",
    );
    assert!(ko.rows().any(|(lineage, _)| lineage[0] == "ko:K00001"));
}

#[test]
fn tables_with_descriptions() {
    for (file, heading, name) in [
        (
            "chemerin/descriptions/EC_pred_metagenome_unstrat_descrip.tsv.gz",
            "EC",
            "1.1.1.1: Alcohol dehydrogenase",
        ),
        (
            "chemerin/descriptions/KO_pred_metagenome_unstrat_descrip.tsv.gz",
            "KO",
            "ko:K00001: alcohol dehydrogenase [EC:1.1.1.1]",
        ),
        (
            "chemerin/descriptions/path_abun_unstrat_descrip.tsv.gz",
            "Pathway",
            "1CMET2-PWY: folate transformations III (E. coli)",
        ),
    ] {
        let format = if heading == "Pathway" {
            ProfilerFormat::Picrust2PathAbundance
        } else {
            ProfilerFormat::Picrust2Functions
        };
        let tree = profile(file, format, &[heading]);
        assert!(tree.rows().any(|(lineage, _)| lineage[0] == name), "{file}");
        agrees_with(&tree, file);
    }
}

#[test]
fn pathways() {
    let abundance = profile(
        "chemerin/pathways_out/path_abun_unstrat.tsv.gz",
        ProfilerFormat::Picrust2PathAbundance,
        &["Pathway"],
    );
    agrees_with(&abundance, "chemerin/pathways_out/path_abun_unstrat.tsv.gz");
    // Coverage has the same header; its values are fractions.
    let coverage = profile(
        "chemerin/pathways_out/path_cov_unstrat.tsv.gz",
        ProfilerFormat::Picrust2PathCoverage,
        &["Pathway"],
    );
    agrees_with(&coverage, "chemerin/pathways_out/path_cov_unstrat.tsv.gz");
    assert!(coverage
        .rows()
        .all(|(_, row)| row.iter().all(|v| *v <= 1.0)));
    // --per_sequence_contrib: pathways inferred in each sequence's genome.
    let per_sequence = profile(
        "chemerin/pathways_out/path_abun_unstrat_per_seq.tsv.gz",
        ProfilerFormat::Picrust2PathAbundance,
        &["Pathway"],
    );
    agrees_with(
        &per_sequence,
        "chemerin/pathways_out/path_abun_unstrat_per_seq.tsv.gz",
    );
}

#[test]
fn wide_stratified_tables_add_up_to_the_unstratified_ones() {
    for (strat, unstrat, heading) in [
        (
            "chemerin/EC_metagenome_out/pred_metagenome_strat.tsv.gz",
            "chemerin/EC_metagenome_out/pred_metagenome_unstrat.tsv.gz",
            "EC",
        ),
        (
            "chemerin/KO_metagenome_out/pred_metagenome_strat.tsv.gz",
            "chemerin/KO_metagenome_out/pred_metagenome_unstrat.tsv.gz",
            "KO",
        ),
        // Pathways per sequence add up to the per-sequence pathway table.
        (
            "chemerin/pathways_out/path_abun_strat.tsv.gz",
            "chemerin/pathways_out/path_abun_unstrat_per_seq.tsv.gz",
            "Pathway",
        ),
    ] {
        let tree = profile(
            strat,
            ProfilerFormat::Picrust2Contributions,
            &[heading, "Sequence"],
        );
        agrees_with(&tree, unstrat);
        // The 37 sequences (ASVs) of the study. A STAMP name has one
        // parent, so each is named within its function: `<ASV> (<function>)`.
        let sequences: std::collections::HashSet<&str> = tree
            .rows()
            .map(|(lineage, _)| lineage[1].split(" (").next().unwrap())
            .collect();
        assert_eq!(sequences.len(), 37, "{strat}");
        assert!(tree
            .rows()
            .all(|(lineage, _)| lineage[1].ends_with(&format!(" ({})", lineage[0]))));
    }
}

#[test]
fn other_outputs_are_not_profiles() {
    for file in [
        "chemerin/EC_metagenome_out/seqtab_norm.tsv.gz",
        "chemerin/EC_metagenome_out/weighted_nsti.tsv.gz",
        "chemerin/pathways_out/path_abun_predictions.tsv.gz",
        "chemerin/metadata.tsv.gz",
    ] {
        assert_eq!(ProfilerFormat::detect(&read(file)), None, "{file}");
    }
}

#[test]
fn an_output_directory_holds_two_kinds_of_table() {
    let directory = path("chemerin/EC_metagenome_out");
    let mut names: Vec<String> = fs::read_dir(&directory)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let texts: Vec<String> = names
        .iter()
        .map(|n| read(&format!("chemerin/EC_metagenome_out/{n}")))
        .collect();
    let files: Vec<NamedText<'_>> = names
        .iter()
        .zip(&texts)
        .map(|(name, text)| NamedText { name, text })
        .collect();
    assert_eq!(
        profiler::formats_of(&files),
        [
            (ProfilerFormat::Picrust2Functions, 1),
            (ProfilerFormat::Picrust2Contributions, 1)
        ]
    );
    let error = profiler::directory_to_spf("EC_metagenome_out", &files, None).unwrap_err();
    assert!(error.0.contains("several kinds"), "{error}");
    let loaded = profiler::directory_to_spf(
        "EC_metagenome_out",
        &files,
        Some(ProfilerFormat::Picrust2Functions),
    )
    .unwrap();
    assert_eq!(loaded.used, 1);
    assert_eq!(
        loaded.skipped,
        [
            "pred_metagenome_strat.tsv.gz",
            "seqtab_norm.tsv.gz",
            "weighted_nsti.tsv.gz"
        ]
    );
    assert_eq!(ProfileTree::read(&loaded.spf).unwrap().num_samples(), 24);
}

#[test]
fn long_contribution_tables_add_up_to_the_unstratified_ones() {
    // PICRUSt2's default --stratified output, from a run on four samples.
    for (contrib, unstrat, heading) in [
        (
            "chemerin_4_samples/EC_metagenome_out/pred_metagenome_contrib.tsv.gz",
            "chemerin_4_samples/EC_metagenome_out/pred_metagenome_unstrat.tsv.gz",
            "EC",
        ),
        (
            "chemerin_4_samples/pathways_out/path_abun_contrib.tsv.gz",
            "chemerin_4_samples/pathways_out/path_abun_unstrat.tsv.gz",
            "Pathway",
        ),
    ] {
        let tree = profile(
            contrib,
            ProfilerFormat::Picrust2Contributions,
            &[heading, "Sequence"],
        );
        assert_eq!(
            tree.sample_names(),
            ["100CHE6KO", "101CHE6WT", "20CMK6KO", "21CMK6WT"],
            "{contrib}"
        );
        agrees_with(&tree, unstrat);
    }
}
