//! The importers must reproduce the STAMP profiles that ship next to their
//! source files in `examples/`, which were produced by STAMP v2.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use stamp_core::import::{biom, cog, NamedText};
use stamp_core::ProfileTree;

fn path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn read(relative: &str) -> String {
    let bytes = fs::read(path(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"));
    stamp_core::text::decode(&bytes).into_owned()
}

/// Rows keyed by their first `levels` columns, with counts as numbers.
fn numeric_rows(spf: &str, levels: usize) -> (String, HashMap<String, Vec<f64>>) {
    let mut lines = spf.lines();
    let header = lines.next().unwrap().to_string();
    let mut rows = HashMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let cells: Vec<&str> = line.split('\t').collect();
        let counts = cells[levels..].iter().map(|c| c.parse().unwrap()).collect();
        assert!(
            rows.insert(cells[..levels].join("\t"), counts).is_none(),
            "duplicate row {line}"
        );
    }
    (header, rows)
}

fn assert_same_numbers(actual: &str, expected: &str, levels: usize) {
    let (actual_header, actual_rows) = numeric_rows(actual, levels);
    let (expected_header, expected_rows) = numeric_rows(expected, levels);
    assert_eq!(actual_header, expected_header);
    assert_eq!(actual_rows.len(), expected_rows.len());
    for (key, expected_counts) in &expected_rows {
        let actual_counts = actual_rows
            .get(key)
            .unwrap_or_else(|| panic!("missing row {key}"));
        assert_eq!(actual_counts.len(), expected_counts.len(), "{key}");
        for (a, e) in actual_counts.iter().zip(expected_counts) {
            // STAMP v2 wrote counts with 12 significant digits.
            assert!(
                (a - e).abs() <= 1e-11 * e.abs().max(1.0),
                "{key}: {a} vs {e}"
            );
        }
    }
}

#[test]
fn biom_picrust_tables() {
    for (source, expected) in [
        (
            "predicted_metagenomes.L3.biom",
            "predicted_metagenomes.L3.ground_truth.spf",
        ),
        ("predicted_metagenomes.biom", "predicted_metagenomes_gt.spf"),
    ] {
        let bytes = fs::read(path(&format!("examples/BIOM-test/{source}"))).unwrap();
        let spf = biom::to_spf(&bytes, None).unwrap();
        assert_same_numbers(&spf, &read(&format!("examples/BIOM-test/{expected}")), 1);
        ProfileTree::read(&spf).unwrap();
    }
}

#[test]
fn biom_with_kegg_pathway_hierarchy() {
    let bytes = fs::read(path("examples/BIOM-test/kegg_predictions_L3.biom")).unwrap();
    let spf = biom::to_spf(&bytes, Some("KEGG_Pathways")).unwrap();
    let mut lines = spf.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    assert_eq!(
        header[..4],
        ["Level_1", "Level_2", "Level_3", "Observation Ids"]
    );
    assert_eq!(header.len(), 4 + 29);
    assert_eq!(lines.count(), 328);

    // KEGG's top-level "Unclassified" category has classified children, so
    // the profile is rejected, exactly as STAMP v2 rejects it.
    assert_eq!(
        ProfileTree::read(&spf).unwrap_err().message(),
        "Child Metabolism has an unclassified parent."
    );

    // The un-collapsed table lists several pathways per gene, which cannot
    // form a single hierarchy.
    let bytes = fs::read(path("examples/BIOM-test/predicted_metagenomes.biom")).unwrap();
    assert!(biom::to_spf(&bytes, Some("KEGG_Pathways")).is_err());
}

#[test]
fn cog_categories_example() {
    let source = read("examples/Assign_COGs_Example.tsv");
    let header = "COG classes\tCOG category names\tCOG category codes\tCOG annotations\tCOG IDs\tSample 1\tSample 2\n";

    let as_feature = cog::append_categories(&source, cog::MultiCodeTreatment::AsFeature).unwrap();
    assert_eq!(
        as_feature,
        format!(
            "{header}\
             Metabolism\tCoenzyme transport and metabolism\tH\tCould\tCOG0001\t5\t1\n\
             Metabolism\tAmino acid transport and metabolism\tE\tbe\tCOG0002\t1\t8\n\
             EH\tEH\tEH\tanything\tCOG0059\t4\t1\n"
        )
    );
    ProfileTree::read(&as_feature).unwrap();

    let each = cog::append_categories(&source, cog::MultiCodeTreatment::AssignToEachCode).unwrap();
    assert_eq!(
        each,
        format!(
            "{header}\
             Metabolism\tCoenzyme transport and metabolism\tH\tCould\tCOG0001\t5\t1\n\
             Metabolism\tAmino acid transport and metabolism\tE\tbe\tCOG0002\t1\t8\n\
             Metabolism\tAmino acid transport and metabolism\tE\tanything\tCOG0059\t4\t1\n\
             Metabolism\tCoenzyme transport and metabolism\tH\tanything\tCOG0059\t4\t1\n"
        )
    );
}

/// Rows of a profile keyed by their seven names, with values as numbers.
fn rows_by_lineage(spf: &str) -> (Vec<String>, HashMap<String, Vec<f64>>) {
    let mut lines = spf.lines();
    let header: Vec<String> = lines
        .next()
        .unwrap()
        .split('\t')
        .map(str::to_string)
        .collect();
    let rows = lines
        .filter(|l| !l.is_empty())
        .map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            let values = cells[7..].iter().map(|c| c.parse().unwrap()).collect();
            (cells[..7].join("\t"), values)
        })
        .collect();
    (header, rows)
}

fn sandpiper_files(names: &[&str]) -> (Vec<String>, Vec<String>) {
    let names: Vec<String> = names.iter().map(|n| format!("{n}.json")).collect();
    let texts = names
        .iter()
        .map(|n| read(&format!("crates/stamp-core/tests/data/singlem/{n}")))
        .collect();
    (names, texts)
}

#[test]
fn sandpiper_profiles_match_the_python_conversion() {
    use stamp_core::import::sandpiper;
    let runs = ["SRR28772463", "ERR4084076", "SRR28771658"];
    let (names, texts) = sandpiper_files(&runs);
    let files: Vec<NamedText<'_>> = names
        .iter()
        .zip(&texts)
        .map(|(name, text)| NamedText { name, text })
        .collect();
    let spf = sandpiper::to_spf(&files).unwrap();
    ProfileTree::read(&spf).unwrap();

    // The reference was written by the flatten/fill_lineage functions of a
    // Python script that the importer replaces.
    let expected = read("crates/stamp-core/tests/data/singlem/expected_from_python.spf");
    let (actual_header, actual) = rows_by_lineage(&spf);
    let (expected_header, expected) = rows_by_lineage(&expected);
    assert_eq!(actual_header, expected_header);
    assert_eq!(actual.len(), expected.len());
    for (lineage, values) in &expected {
        let got = actual
            .get(lineage)
            .unwrap_or_else(|| panic!("missing row {lineage}"));
        for (a, e) in got.iter().zip(values) {
            // The importer keeps six significant digits.
            assert!((a - e).abs() <= 1e-5 * e.abs(), "{lineage}: {a} vs {e}");
        }
    }

    // GlobDB profiles have the same form.
    let globdb = read("crates/stamp-core/tests/data/singlem/SRR28772463.globdb.json");
    let both = [
        NamedText {
            name: "a.json",
            text: &globdb,
        },
        files[1],
    ];
    ProfileTree::read(&sandpiper::to_spf(&both).unwrap()).unwrap();
}

#[test]
fn singlem_and_sandpiper_give_the_same_profile() {
    use stamp_core::import::{sandpiper, singlem};
    // The SingleM file holds the same two runs as the Sandpiper files, with
    // coverages to two decimals as SingleM writes them.
    let text = read("crates/stamp-core/tests/data/singlem/singlem_condensed.tsv");
    let from_singlem = singlem::to_spf(&[NamedText {
        name: "singlem_condensed.tsv",
        text: &text,
    }])
    .unwrap();
    let (names, texts) = sandpiper_files(&["SRR28772463", "ERR4084076"]);
    let files: Vec<NamedText<'_>> = names
        .iter()
        .zip(&texts)
        .map(|(name, text)| NamedText { name, text })
        .collect();
    let from_sandpiper = sandpiper::to_spf(&files).unwrap();
    let (header_a, a) = rows_by_lineage(&from_singlem);
    let (header_b, b) = rows_by_lineage(&from_sandpiper);
    assert_eq!(header_a, header_b);
    assert_eq!(a.len(), b.len());
    for (lineage, values) in &a {
        for (x, y) in values.iter().zip(&b[lineage]) {
            assert!((x - y).abs() < 0.006, "{lineage}: {x} vs {y}");
        }
    }
}

/// The glacier-fed stream example (SingleM coverage, built into the
/// application): the profile and metadata agree with each other, and the
/// paired test between the two reaches of each stream runs on them.
#[test]
fn glacier_fed_streams_example() {
    use stamp_core::stats::correction::Correction;
    use stamp_core::stats::results::{run_group_analysis, GroupAnalysis, Transform};
    use stamp_core::stats::rng_from_seed;
    use stamp_core::stats::two_group::GroupTest;
    use stamp_core::stats::TestType;
    use stamp_core::{Metadata, SecondGroup, Unclassified};

    let tree = ProfileTree::read(&read("examples/GlacierFedStreams/glacier_streams.spf")).unwrap();
    assert_eq!(tree.num_samples(), 158);
    assert_eq!(
        tree.hierarchy_headings(),
        ["Domain", "Phylum", "Class", "Order", "Family", "Genus", "Species"]
    );
    let (metadata, warnings) = Metadata::read(
        &read("examples/GlacierFedStreams/glacier_streams.metadata.tsv"),
        &tree,
    )
    .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(metadata.samples().len(), 158);
    assert_eq!(
        metadata.groups("Mountain_range").unwrap().names().count(),
        9
    );
    let reaches = metadata.groups("Reach").unwrap();
    assert_eq!(reaches.samples("Downstream").unwrap().len(), 81);
    assert_eq!(reaches.samples("Upstream").unwrap().len(), 77);
    // The total coverage in the metadata is that of the profile.
    for sample in ["GL100_DN", "GL140_UP_1", "GL7_UP"] {
        let expected: f64 = metadata
            .value(sample, "Total_coverage")
            .unwrap()
            .parse()
            .unwrap();
        let total = tree.sample_total(sample).unwrap();
        assert!((total - expected).abs() < 0.06, "{sample}: {total}");
    }

    // Downstream against upstream reach, the two samples of a glacier as a
    // pair: 73 glaciers have both.
    let pairing = metadata
        .paired_groups(&reaches, "Downstream", "Upstream", "Glacier")
        .unwrap();
    assert_eq!(
        pairing.summary(),
        "73 pairs; 12 samples without a partner left out"
    );
    let profile = tree
        .group_profile(
            &pairing.groups,
            "Downstream",
            SecondGroup::Named("Upstream"),
            None,
            "Family",
            Unclassified::Retain,
        )
        .unwrap();
    assert_eq!(profile.profile.num_features(), 1936);
    let analysis = GroupAnalysis {
        test: GroupTest::WilcoxonSignedRank,
        test_type: TestType::TwoSided,
        coverage: 0.95,
        correction: Correction::BenjaminiHochberg,
        transform: Transform::None,
    };
    let settings = stamp_core::stats::StatsSettings::default();
    let results = run_group_analysis(
        &profile,
        analysis,
        &settings,
        &mut rng_from_seed(1),
        &mut |_, _| true,
    )
    .unwrap();
    assert_eq!(results.rows.len(), 1936);
    let significant = results
        .rows
        .iter()
        .filter(|r| r.p_value_corrected <= 0.05)
        .count();
    assert_eq!(significant, 23);
    // JACQAP01 is more abundant downstream.
    let row = results
        .rows
        .iter()
        .find(|r| r.feature == "JACQAP01")
        .unwrap();
    assert!(row.p_value_corrected < 1e-3, "{}", row.p_value_corrected);
    assert!(row.effect_size > 0.0);
}
