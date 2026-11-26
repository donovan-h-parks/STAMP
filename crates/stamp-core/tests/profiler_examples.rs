//! The example outputs of profilers in `examples/input_file_formats` load as
//! directories, as File > Load profile directory and `--profile <dir>` read
//! them, and give the profile the community they were drawn from implies.
//! The files are gzip-compressed, and are read as STAMP reads every file:
//! decompressed if compressed.

use std::fs;
use std::path::PathBuf;

use stamp_core::import::profiler::{self, ProfilerFormat};
use stamp_core::import::{biom, cog, mothur, NamedText};
use stamp_core::ProfileTree;

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/input_file_formats")
}

/// A file of the examples, decompressed.
fn bytes(path: &PathBuf) -> Vec<u8> {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    stamp_core::text::decompress(&bytes).unwrap().into_owned()
}

/// The files directly in a directory of the examples, in order of name.
fn directory(name: &str) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(examples().join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .map(|p| {
            let text = stamp_core::text::decode(&bytes(&p)).into_owned();
            (p.file_name().unwrap().to_string_lossy().into_owned(), text)
        })
        .collect();
    files.sort();
    files
}

fn named(files: &[(String, String)]) -> Vec<NamedText<'_>> {
    files
        .iter()
        .map(|(name, text)| NamedText { name, text })
        .collect()
}

const SAMPLES: [&str; 10] = [
    "H01", "H02", "H03", "H04", "H05", "I01", "I02", "I03", "I04", "I05",
];

/// Load a directory of the examples, holding profiles of one format.
fn load(name: &str, only: Option<ProfilerFormat>) -> (ProfilerFormat, ProfileTree, usize) {
    let files = directory(name);
    let profile = profiler::directory_to_spf(name, &named(&files), only)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    let tree = ProfileTree::read(&profile.spf).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(tree.sample_names(), SAMPLES, "{name}");
    (profile.format, tree, profile.skipped.len())
}

/// The names at a level, in order of first appearance.
fn features_at(tree: &ProfileTree, level: &str) -> Vec<String> {
    let depth = tree
        .hierarchy_headings()
        .iter()
        .position(|h| h == level)
        .unwrap();
    let mut names: Vec<String> = Vec::new();
    for (lineage, _) in tree.rows() {
        if !names.iter().any(|n| n == lineage[depth]) {
            names.push(lineage[depth].to_string());
        }
    }
    names
}

/// The value of a feature at a level in a sample: the sum of its rows.
fn value(tree: &ProfileTree, level: &str, name: &str, sample: &str) -> Option<f64> {
    let depth = tree
        .hierarchy_headings()
        .iter()
        .position(|h| h == level)
        .unwrap();
    let column = tree.sample_index(sample)?;
    let rows: Vec<f64> = tree
        .rows()
        .filter(|(lineage, _)| lineage[depth] == name)
        .map(|(_, values)| values[column])
        .collect();
    (!rows.is_empty()).then(|| rows.iter().sum())
}

/// The headings of a profile's levels.
fn levels(tree: &ProfileTree) -> Vec<String> {
    tree.hierarchy_headings().to_vec()
}

/// Mean proportion of the features at `level` whose name contains `name`,
/// in the healthy and the IBD samples.
fn means(tree: &ProfileTree, level: &str, name: &str) -> (f64, f64) {
    let mean = |samples: &[&str]| {
        samples
            .iter()
            .map(|s| {
                let total = tree.sample_total(s).unwrap();
                features_at(tree, level)
                    .iter()
                    .filter(|f| f.contains(name))
                    .map(|f| value(tree, level, f, s).unwrap_or(0.0))
                    .sum::<f64>()
                    / total
            })
            .sum::<f64>()
            / samples.len() as f64
    };
    (mean(&SAMPLES[..5]), mean(&SAMPLES[5..]))
}

/// E. coli is more common in IBD and F. prausnitzii less, in every
/// taxonomic example.
fn assert_ibd_signal(tree: &ProfileTree, level: &str, coli: &str, prausnitzii: &str) {
    let (healthy, ibd) = means(tree, level, coli);
    assert!(ibd > 3.0 * healthy, "{coli}: {healthy} vs {ibd}");
    let (healthy, ibd) = means(tree, level, prausnitzii);
    assert!(healthy > 2.0 * ibd, "{prausnitzii}: {healthy} vs {ibd}");
}

const RANKS: [&str; 7] = [
    "Domain", "Phylum", "Class", "Order", "Family", "Genus", "Species",
];

#[test]
fn kraken_reports() {
    let (format, tree, skipped) = load("Kraken2", None);
    assert_eq!((format, skipped), (ProfilerFormat::Kraken, 0));
    let mut expected: Vec<&str> = RANKS.to_vec();
    expected.push("Strain");
    assert_eq!(levels(&tree), expected);
    assert_ibd_signal(
        &tree,
        "Species",
        "Escherichia coli",
        "Faecalibacterium prausnitzii",
    );
    // Unclassified reads and reads left at root are one row.
    assert!(features_at(&tree, "Species").contains(&"unclassified Root".to_string()));
    assert!(features_at(&tree, "Strain").contains(&"Escherichia coli K-12".to_string()));
}

#[test]
fn bracken_tables_and_reports() {
    let (format, tree, skipped) = load("Bracken", None);
    assert_eq!((format, skipped), (ProfilerFormat::Bracken, 0));
    assert_eq!(levels(&tree), ["Species"]);
    assert_ibd_signal(
        &tree,
        "Species",
        "Escherichia coli",
        "Faecalibacterium prausnitzii",
    );

    let (format, tree, _) = load("Bracken/reports", None);
    assert_eq!(format, ProfilerFormat::BrackenReport);
    assert_eq!(levels(&tree), RANKS);
    // The report and the table give each species the same reads.
    let tables = load("Bracken", None).1;
    for sample in SAMPLES {
        for species in features_at(&tables, "Species") {
            assert_eq!(
                value(&tables, "Species", &species, sample),
                value(&tree, "Species", &species, sample),
                "{species} in {sample}"
            );
        }
    }
}

#[test]
fn metaphlan_profiles_and_merged_table() {
    let (format, tree, skipped) = load("MetaPhlAn", None);
    assert_eq!((format, skipped), (ProfilerFormat::MetaPhlAn, 0));
    assert_eq!(
        levels(&tree),
        ["Kingdom", "Phylum", "Class", "Order", "Family", "Genus", "Species", "SGB"]
    );
    assert_ibd_signal(
        &tree,
        "Species",
        "Escherichia_coli",
        "Faecalibacterium_prausnitzii",
    );
    // Each sample adds up to 100 %, unclassified included, and nothing is
    // left above the SGBs but the unclassified share.
    for sample in SAMPLES {
        assert!((tree.sample_total(sample).unwrap() - 100.0).abs() < 1e-3);
    }
    assert!(features_at(&tree, "Species")
        .iter()
        .all(|f| !f.starts_with("unclassified") || f == "unclassified Root"));

    let (_, merged, _) = load("MetaPhlAn/merged", None);
    for sample in SAMPLES {
        for sgb in features_at(&tree, "SGB") {
            let (a, b) = (
                value(&tree, "SGB", &sgb, sample).unwrap_or(0.0),
                value(&merged, "SGB", &sgb, sample).unwrap_or(0.0),
            );
            assert!((a - b).abs() < 1e-9, "{sgb} in {sample}: {a} vs {b}");
        }
    }
}

#[test]
fn motus_profiles_and_merged_table() {
    let (format, tree, skipped) = load("mOTUs", None);
    assert_eq!((format, skipped), (ProfilerFormat::Motus, 0));
    let mut expected = RANKS.to_vec();
    expected[0] = "Kingdom";
    assert_eq!(levels(&tree), expected);
    assert_ibd_signal(
        &tree,
        "Species",
        "Escherichia coli",
        "Faecalibacterium prausnitzii",
    );
    assert!(features_at(&tree, "Species")
        .contains(&"Escherichia coli [ref_mOTU_v31_00095]".to_string()));

    let (_, merged, _) = load("mOTUs/merged", None);
    for sample in SAMPLES {
        let total = merged.sample_total(sample).unwrap();
        assert!((total - 1.0).abs() < 1e-5, "{sample}: {total}");
    }
    assert_ibd_signal(&merged, "Genus", "Escherichia", "Faecalibacterium");
}

#[test]
fn singlem_and_sandpiper_profiles() {
    let (format, singlem, _) = load("SingleM", None);
    assert_eq!(format, ProfilerFormat::SingleM);
    assert_eq!(levels(&singlem), RANKS);
    assert_ibd_signal(
        &singlem,
        "Species",
        "Escherichia coli",
        "Faecalibacterium prausnitzii",
    );
    let (format, sandpiper, _) = load("Sandpiper", None);
    assert_eq!(format, ProfilerFormat::Sandpiper);
    for sample in SAMPLES {
        let (a, b) = (
            singlem.sample_total(sample).unwrap(),
            sandpiper.sample_total(sample).unwrap(),
        );
        // SingleM rounds to two decimals.
        assert!((a - b).abs() < 0.2, "{sample}: {a} vs {b}");
    }
}

#[test]
fn humann_tables() {
    // The directory holds three kinds of table, so the kind must be chosen.
    let files = directory("HUMAnN");
    let error = profiler::directory_to_spf("HUMAnN", &named(&files), None).unwrap_err();
    assert!(
        error.0.contains(
            "10 HUMAnN gene families, 10 HUMAnN pathway abundance, 10 HUMAnN \
                          pathway coverage"
        ),
        "{error}"
    );
    assert_eq!(
        profiler::formats_of(&named(&files)),
        [
            (ProfilerFormat::HumannGeneFamilies, 10),
            (ProfilerFormat::HumannPathAbundance, 10),
            (ProfilerFormat::HumannPathCoverage, 10)
        ]
    );

    let (_, genes, skipped) = load("HUMAnN", Some(ProfilerFormat::HumannGeneFamilies));
    assert_eq!(skipped, 20);
    assert_eq!(levels(&genes), ["Gene family", "Genus", "Species"]);
    assert!(features_at(&genes, "Gene family").contains(&"UNMAPPED".to_string()));

    let (_, pathways, _) = load("HUMAnN", Some(ProfilerFormat::HumannPathAbundance));
    assert_eq!(levels(&pathways), ["Pathway"]);
    let (healthy, ibd) = means(&pathways, "Pathway", "ENTBACSYN-PWY");
    assert!(ibd > 3.0 * healthy, "{healthy} vs {ibd}");

    let (_, coverage, _) = load("HUMAnN", Some(ProfilerFormat::HumannPathCoverage));
    assert_eq!(levels(&coverage), ["Pathway"]);
    for sample in SAMPLES {
        assert_eq!(value(&coverage, "Pathway", "UNMAPPED", sample), Some(1.0));
    }
}

#[test]
fn other_formats_load() {
    let base = examples();
    for path in ["BIOM/otu_table.biom.gz", "BIOM/otu_table_hdf5.biom.gz"] {
        let bytes = bytes(&base.join(path));
        let tree = ProfileTree::read(&biom::to_spf(&bytes, Some("taxonomy")).unwrap())
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(tree.sample_names(), SAMPLES);
    }
    let read =
        |p: &str| stamp_core::text::decode(&bytes(&base.join(format!("{p}.gz")))).into_owned();
    let spf = mothur::to_spf(
        &read("mothur/stool.unique.rdp.wang.taxonomy"),
        &read("mothur/stool.groups"),
        Some(&read("mothur/stool.names")),
    )
    .unwrap();
    let tree = ProfileTree::read(&spf).unwrap();
    assert_eq!(tree.sample_names(), SAMPLES);
    let spf = cog::append_categories(
        &read("IMG-M_COG/cog_profile.tsv"),
        cog::MultiCodeTreatment::AsFeature,
    )
    .unwrap();
    assert_eq!(ProfileTree::read(&spf).unwrap().sample_names(), SAMPLES);
    // None of these is taken for a profiler's output.
    for path in [
        "mothur/stool.groups",
        "mothur/stool.names",
        "mothur/stool.unique.rdp.wang.taxonomy",
        "IMG-M_COG/cog_profile.tsv",
        "metadata.tsv",
    ] {
        assert_eq!(ProfilerFormat::detect(&read(path)), None, "{path}");
    }
}
