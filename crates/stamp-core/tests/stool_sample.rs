//! Real outputs of every profiler STAMP reads, for one human stool
//! metagenome (SRR17283923, subsampled to 1,000,000 read pairs), in
//! `examples/input_file_formats/<tool>/stool_SRR17283923`. Each must be
//! recognised as its format, converted, and give a profile STAMP can load,
//! with the values the tool reported. The files are kept gzip-compressed.
//! See the README there for how the files were made.

use std::fs;
use std::path::PathBuf;

use stamp_core::import::profiler::ProfilerFormat;
use stamp_core::import::NamedText;
use stamp_core::ProfileTree;

const RUN: &str = "SRR17283923";

/// A file of the sample, which is kept gzip-compressed as `<file>.gz`.
fn read(tool: &str, file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/input_file_formats")
        .join(tool)
        .join("stool_SRR17283923")
        .join(format!("{file}.gz"));
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    stamp_core::text::decode(&stamp_core::text::decompress(&bytes).unwrap()).into_owned()
}

/// A file's profile: it is recognised as `format`, converted, and has the
/// one sample `sample` and the levels `levels`. STAMP needs two samples to
/// load a profile, so the profile is loaded with a copy of its sample beside
/// it, which checks that its hierarchy is valid. Returns the profile and the
/// sample's total.
fn profile(
    tool: &str,
    file: &str,
    format: ProfilerFormat,
    sample: &str,
    levels: &[&str],
) -> (ProfileTree, f64) {
    let text = read(tool, file);
    assert_eq!(ProfilerFormat::detect(&text), Some(format), "{file}");
    // The name the file has on disk, ".gz" and all.
    let name = format!("{file}.gz");
    let spf = format
        .to_spf(&[NamedText {
            name: &name,
            text: &text,
        }])
        .unwrap_or_else(|e| panic!("{file}: {e}"));
    let mut lines = spf.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    assert_eq!(header[..header.len() - 1], *levels, "{file}");
    assert_eq!(header[header.len() - 1], sample, "{file}");

    // The same sample again, as a second column.
    let mut doubled = String::new();
    for (i, line) in spf.lines().enumerate() {
        let last = line.rsplit('\t').next().unwrap();
        let copy = if i == 0 { "copy" } else { last };
        doubled.push_str(&format!("{line}\t{copy}\n"));
    }
    let tree = ProfileTree::read(&doubled).unwrap_or_else(|e| panic!("{file}: {e}"));
    assert_eq!(tree.sample_names(), [sample, "copy"], "{file}");
    let total = tree.sample_total(sample).unwrap();
    assert!(total > 0.0, "{file}");
    (tree, total)
}

/// The value of the rows whose names at `level` equal `name`.
fn value(tree: &ProfileTree, level: &str, name: &str) -> f64 {
    let depth = tree
        .hierarchy_headings()
        .iter()
        .position(|h| h == level)
        .unwrap_or_else(|| panic!("no level {level}"));
    tree.rows()
        .filter(|(lineage, _)| lineage[depth] == name)
        .map(|(_, values)| values[0])
        .sum()
}

/// The number of distinct names at `level`, unclassified ones left out.
fn count(tree: &ProfileTree, level: &str) -> usize {
    let depth = tree
        .hierarchy_headings()
        .iter()
        .position(|h| h == level)
        .unwrap();
    let mut names: Vec<&str> = tree
        .rows()
        .map(|(lineage, _)| lineage[depth])
        .filter(|n| !n.starts_with("unclassified"))
        .collect();
    names.sort_unstable();
    names.dedup();
    names.len()
}

/// The sum of one column of a tab-separated table, after its header.
fn column_sum(text: &str, column: usize) -> f64 {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .skip(1)
        .map(|l| l.split('\t').nth(column).unwrap().parse::<f64>().unwrap())
        .sum()
}

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

const GTDB: [&str; 7] = [
    "Domain", "Phylum", "Class", "Order", "Family", "Genus", "Species",
];
const NCBI: [&str; 7] = [
    "Kingdom", "Phylum", "Class", "Order", "Family", "Genus", "Species",
];

#[test]
fn kraken_reports() {
    let mut strain = GTDB.to_vec();
    strain.push("Strain");
    for (file, sample) in [
        ("SRR17283923.k2report", RUN),
        ("SRR17283923_minimizers.k2report", "SRR17283923_minimizers"),
    ] {
        let (tree, total) = profile("Kraken2", file, ProfilerFormat::Kraken, sample, &strain);
        // Every read of the 1,000,000 pairs: 365,889 unclassified, and the
        // 89 + 337 reads Kraken left at root and "cellular organisms".
        assert_eq!(total, 1_000_000.0, "{file}");
        assert_eq!(
            value(&tree, "Domain", "unclassified Root"),
            365_889.0 + 89.0 + 337.0
        );
        // A species's value is its clade reads (its own and its strains').
        assert_eq!(value(&tree, "Species", "Segatella copri"), 169_834.0);
        assert_eq!(value(&tree, "Species", "Blautia wexlerae"), 39_064.0);
        assert_eq!(value(&tree, "Domain", "Bacteria"), 630_685.0);
        // Kingdoms of the 2025 NCBI taxonomy (Bacillati) are no level.
        assert!(value(&tree, "Phylum", "Bacillota") > value(&tree, "Phylum", "Bacteroidota"));
        assert!(count(&tree, "Species") > 1_000, "{file}");
    }
    // --use-mpa-style: clade counts of the ranked taxa only, so the reads
    // left at root and "cellular organisms" are not there.
    let (tree, total) = profile(
        "Kraken2",
        "SRR17283923_mpa.txt",
        ProfilerFormat::KrakenMpa,
        "SRR17283923_mpa",
        &GTDB,
    );
    assert_eq!(total, 634_111.0 - 89.0 - 337.0);
    assert_eq!(value(&tree, "Domain", "unclassified Root"), 0.0);
    assert_eq!(value(&tree, "Species", "Segatella copri"), 169_834.0);
}

#[test]
fn bracken_tables_and_reports() {
    let text = read("Bracken", "SRR17283923.bracken");
    let (tree, total) = profile(
        "Bracken",
        "SRR17283923.bracken",
        ProfilerFormat::Bracken,
        RUN,
        &["Species"],
    );
    assert_eq!(total, column_sum(&text, 5));
    assert!(value(&tree, "Species", "Segatella copri") > 160_000.0);
    assert_eq!(count(&tree, "Species"), 281);

    let text = read("Bracken", "SRR17283923_genus.bracken");
    let (_, genera) = profile(
        "Bracken",
        "SRR17283923_genus.bracken",
        ProfilerFormat::Bracken,
        "SRR17283923_genus",
        &["Genus"],
    );
    assert_eq!(genera, column_sum(&text, 5));

    // The reports hold Bracken's whole total (its root), the reads it could
    // not take down to the level included.
    for (file, root) in [
        ("SRR17283923_bracken_species.kreport", 631_323.0),
        ("SRR17283923_bracken_genus.kreport", 632_687.0),
    ] {
        let (tree, total) = profile("Bracken", file, ProfilerFormat::BrackenReport, RUN, &GTDB);
        assert_eq!(total, root, "{file}");
        // Bracken 3.1 gives the taxa of its level no direct reads in the
        // genus report; reading by clade reads keeps them.
        assert!(value(&tree, "Genus", "Segatella") > 160_000.0, "{file}");
        assert!(
            value(&tree, "Genus", "Faecalibacterium") > 10_000.0,
            "{file}"
        );
    }
}

#[test]
fn singlem_profiles() {
    for file in [
        "SRR17283923.profile.tsv",
        "SRR17283923.profile_with_extras.tsv",
    ] {
        let text = read("SingleM", file);
        let (tree, total) = profile("SingleM", file, ProfilerFormat::SingleM, RUN, &GTDB);
        assert!(close(total, column_sum(&text, 1), 1e-6), "{file}");
        assert!(close(total, 88.35, 1e-9), "{file}: {total}");
        assert!(value(&tree, "Domain", "Bacteria") > 80.0, "{file}");
        assert!(value(&tree, "Genus", "Faecalibacterium") > 0.0, "{file}");
    }
}

#[test]
fn sandpiper_profile() {
    // Sandpiper's profile is of the whole run, not of the subsample.
    let (tree, total) = profile(
        "Sandpiper",
        "SRR17283923.json",
        ProfilerFormat::Sandpiper,
        RUN,
        &GTDB,
    );
    assert!(close(total, 388.35, 1e-9), "{total}");
    assert!(value(&tree, "Domain", "Bacteria") > value(&tree, "Domain", "Archaea"));
    assert!(value(&tree, "Genus", "Faecalibacterium") > 0.0);
}

#[test]
fn motus_3_profiles() {
    // The default output: one level, the mOTUs, adding up to 1 with
    // unassigned. The table lists every mOTU of the database (2.3 MB before
    // compression); those at zero are left out.
    let (tree, total) = profile(
        "mOTUs",
        "SRR17283923.motus",
        ProfilerFormat::Motus,
        RUN,
        &["mOTU"],
    );
    assert!(close(total, 1.0, 1e-5), "{total}");
    assert_eq!(count(&tree, "mOTU"), 120);
    assert!(value(&tree, "mOTU", "Escherichia coli [ref_mOTU_v31_00095]") > 0.002);
    // -c -q: insert counts with the full taxonomy of each mOTU.
    let (tree, total) = profile(
        "mOTUs",
        "SRR17283923_counts_taxonomy.motus",
        ProfilerFormat::Motus,
        RUN,
        &NCBI,
    );
    assert_eq!(total, 581.0);
    assert_eq!(value(&tree, "Kingdom", "unclassified Root"), 8.0);
    assert_eq!(count(&tree, "Species"), 119);

    // -A: every rank, with clade values; mOTUs leaves out what it could not
    // assign, so the sample adds up to less than 1.
    let (tree, total) = profile(
        "mOTUs",
        "SRR17283923_all_ranks.motus",
        ProfilerFormat::Motus,
        RUN,
        &NCBI,
    );
    assert!(close(total, 0.802253, 1e-5), "{total}");
    assert!(value(&tree, "Phylum", "Firmicutes") > value(&tree, "Phylum", "Proteobacteria"));
    assert!(count(&tree, "Species") > 100);
    // -k genus: one level, adding up to 1 with unassigned. The table lists
    // every genus of the database; those at zero are left out.
    let (tree, total) = profile(
        "mOTUs",
        "SRR17283923_genus.motus",
        ProfilerFormat::Motus,
        RUN,
        &["Genus"],
    );
    assert!(close(total, 1.0, 1e-5), "{total}");
    // Values are converted to six significant digits.
    assert!(close(
        value(&tree, "Genus", "unclassified Root"),
        0.01315477,
        1e-7
    ));
    assert_eq!(count(&tree, "Genus"), 42);
}

#[test]
fn motus_4_profiles() {
    for (file, counts) in [
        ("motus4_SRR17283923.motus", true),
        ("motus4_SRR17283923.motus.relab", false),
    ] {
        let (tree, total) = profile("mOTUs", file, ProfilerFormat::Motus, RUN, &GTDB);
        if counts {
            assert_eq!(total, 605.0, "{file}");
            // mOTUs 4 writes its unassigned share with an empty lineage
            // ("d__;p___A;c__;...").
            assert_eq!(value(&tree, "Domain", "unclassified Root"), 21.0);
            assert_eq!(
                value(&tree, "Species", "Unknown Prevotella mOTUv4.0_000001"),
                86.0
            );
        } else {
            assert!(close(total, 1.0, 1e-4), "{file}: {total}");
            assert!(value(&tree, "Domain", "unclassified Root") > 0.0, "{file}");
        }
        assert!(value(&tree, "Genus", "Faecalibacterium") > 0.0, "{file}");
        assert_eq!(count(&tree, "Species"), 97, "{file}");
    }
}

#[test]
fn metaphlan_profiles() {
    // Relative abundances, with SGBs below the species.
    let mut levels = NCBI.to_vec();
    levels.push("SGB");
    let (tree, total) = profile(
        "MetaPhlAn",
        "SRR17283923_profile.txt",
        ProfilerFormat::MetaPhlAn,
        RUN,
        &levels,
    );
    assert!(close(total, 100.0, 1e-4), "{total}");
    assert!(close(
        value(&tree, "Kingdom", "unclassified Root"),
        6.56712,
        1e-9
    ));
    assert!(close(
        value(&tree, "Species", "Segatella_copri"),
        16.523,
        1e-3
    ));
    assert!(value(&tree, "SGB", "SGB1626") > 0.0);

    // Estimated reads: both mates of the 1,000,000 pairs.
    let (tree, total) = profile(
        "MetaPhlAn",
        "SRR17283923_read_stats_profile.txt",
        ProfilerFormat::MetaPhlAn,
        "SRR17283923_read_stats",
        &levels,
    );
    assert!(close(total, 2_000_000.0, 1.0), "{total}");
    assert_eq!(value(&tree, "Kingdom", "unclassified Root"), 623_489.0);
    assert_eq!(value(&tree, "Species", "Segatella_copri"), 284_938.0);

    // sgb_to_gtdb_profile.py rescales the classified part to 100 but keeps
    // the UNCLASSIFIED line, so its file adds up to more than 100; STAMP
    // reads the values as they are written.
    let (tree, total) = profile(
        "MetaPhlAn",
        "SRR17283923_gtdb_profile.txt",
        ProfilerFormat::MetaPhlAn,
        "SRR17283923_gtdb",
        &GTDB,
    );
    assert!(close(total, 106.56712, 1e-4), "{total}");
    assert!(close(value(&tree, "Domain", "Bacteria"), 100.0, 1e-4));
    assert!(value(&tree, "Phylum", "Firmicutes_A") > value(&tree, "Phylum", "Bacteroidota"));
}

#[test]
fn humann_tables() {
    // Gene families: each family split by the species HUMAnN assigned its
    // reads to; the strata add up to the family's community value.
    let file = "SRR17283923_genefamilies.tsv";
    let text = read("HUMAnN", file);
    let community: f64 = text
        .lines()
        .skip(1)
        .filter(|l| !l.contains('|'))
        .map(|l| l.split('\t').nth(1).unwrap().parse::<f64>().unwrap())
        .sum();
    let (tree, total) = profile(
        "HUMAnN",
        file,
        ProfilerFormat::HumannGeneFamilies,
        RUN,
        &["Gene family", "Genus", "Species"],
    );
    assert!(
        close(total, community, 1e-6 * community),
        "{total} {community}"
    );
    assert_eq!(count(&tree, "Gene family"), 78_424);
    assert_eq!(value(&tree, "Gene family", "UNMAPPED"), 855_917.0);
    assert_eq!(value(&tree, "Gene family", "UniRef90_A0A174PA28"), 22_000.0);
    // A stratum's genus and species are named within their family.
    assert_eq!(
        value(
            &tree,
            "Species",
            "Prevotella_copri (Prevotella (UniRef90_A0A174PA28))"
        ),
        22_000.0
    );

    // Pathways: community values only (the strata need not add up to them).
    let (tree, total) = profile(
        "HUMAnN",
        "SRR17283923_pathabundance.tsv",
        ProfilerFormat::HumannPathAbundance,
        RUN,
        &["Pathway"],
    );
    assert_eq!(count(&tree, "Pathway"), 206);
    assert!(close(total, 1_049_430.0, 10.0), "{total}");
    assert!(close(
        value(&tree, "Pathway", "UNINTEGRATED"),
        706_465.6,
        1.0
    ));
    let (tree, total) = profile(
        "HUMAnN",
        "SRR17283923_pathcoverage.tsv",
        ProfilerFormat::HumannPathCoverage,
        RUN,
        &["Pathway"],
    );
    // Coverage of 0 for 120 of the 206 pathways: those rows are left out.
    assert_eq!(count(&tree, "Pathway"), 86);
    assert!(close(total, 55.6001, 1e-3), "{total}");
}
