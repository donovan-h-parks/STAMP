//! The importers must reproduce the STAMP profiles that ship next to their
//! source files in `examples/`, which were produced by STAMP v2.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use stamp_core::import::{biom, cog, comet, mgrast, NamedText};
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

/// Header line and data rows, with the rows sorted (row order is not defined
/// in files written by STAMP v2).
fn header_and_sorted_rows(spf: &str) -> (String, Vec<String>) {
    let mut lines = spf.lines().map(str::to_string);
    let header = lines.next().unwrap();
    let mut rows: Vec<String> = lines.filter(|l| !l.is_empty()).collect();
    rows.sort();
    (header, rows)
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
fn mgrast_cow_rumen() {
    let source = read("examples/CowRumen-MG-RAST/CowRumen-MG-RAST.tsv");
    let spf = mgrast::MgRastProfile::parse(&source)
        .unwrap()
        .to_spf()
        .unwrap();
    assert_eq!(
        header_and_sorted_rows(&spf),
        header_and_sorted_rows(&read("examples/CowRumen-MG-RAST/CowRumen.spf"))
    );
    ProfileTree::read(&spf).unwrap();
}

#[test]
fn comet_nine_biomes_pfam() {
    let dir = "examples/NineBiomes-Pfam-Profiles-CoMet";
    let biomes = [
        "Coral",
        "Fish",
        "Freshwater",
        "Hypersaline",
        "Marine",
        "Microbialites",
        "Subterranean",
        "TerrestrialAnimals",
    ];
    let names: Vec<String> = biomes
        .iter()
        .map(|b| format!("profilePfam_{b}.fa.txt"))
        .collect();
    let texts: Vec<String> = names.iter().map(|n| read(&format!("{dir}/{n}"))).collect();
    let files: Vec<NamedText> = names
        .iter()
        .zip(&texts)
        .map(|(name, text)| NamedText { name, text })
        .collect();

    let spf = comet::to_spf(&files).unwrap();
    assert_same_numbers(&spf, &read(&format!("{dir}/NineBiomes-Pfam.spf")), 1);
    ProfileTree::read(&spf).unwrap();
}

#[test]
fn comet_go_profiles_convert_and_load() {
    let dir = path("examples/NineBiomes-GO-Profiles-CoMet");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.starts_with("profileGO_"))
        .collect();
    names.sort();
    assert_eq!(names.len(), 8);
    let texts: Vec<String> = names
        .iter()
        .map(|n| read(&format!("examples/NineBiomes-GO-Profiles-CoMet/{n}")))
        .collect();
    let files: Vec<NamedText> = names
        .iter()
        .zip(&texts)
        .map(|(name, text)| NamedText { name, text })
        .collect();

    let tree = ProfileTree::read(&comet::to_spf(&files).unwrap()).unwrap();
    assert_eq!(tree.num_samples(), 8);
    assert_eq!(tree.sample_names()[0], "Coral");
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
