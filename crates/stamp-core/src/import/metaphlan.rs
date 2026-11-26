//! MetaPhlAn profiles (3.x and 4.x): `-t rel_ab` and `-t
//! rel_ab_w_read_stats` profiles, tables merged by
//! `merge_metaphlan_tables.py`, and GTDB profiles from
//! `sgb_to_gtdb_profile.py`. See `docs/PROFILER_FORMATS.md`.
//!
//! Every clade is listed with its clade (cumulative) value, from `k__` down
//! to `s__` and, in MetaPhlAn 4, the species-level genome bin `t__SGB...`;
//! own values are found with [`own_values`](super::lineage::own_values).
//! The `UNCLASSIFIED` (`UNKNOWN` in MetaPhlAn 3) row becomes `unclassified
//! Root`. Estimated read counts are used when a profile gives them.

use super::lineage::{add_clades, parse_value, rank_prefix, split_lineage, Clade, Lineages, Ranks};
use super::{check_unique, complete_lines, sample_name_of, NamedText};
use crate::error::{Result, StampError};
use crate::text;

/// Column of the estimated reads in `rel_ab_w_read_stats` profiles.
const READS: &str = "estimated_number_of_reads_from_the_clade";

/// Columns that are not samples.
const NOT_SAMPLES: [&str; 6] = [
    "NCBI_tax_id",
    "clade_taxid",
    "additional_species",
    "relative_abundance",
    "coverage",
    READS,
];

/// True if the start of a file looks like a MetaPhlAn profile or merged
/// table: a `#mpa_v...` database line, or a `clade_name` column header,
/// or (as MetaPhlAn 2 wrote) no header and lineages starting at `k__`.
pub fn looks_like_metaphlan(start: &str) -> bool {
    let lines = complete_lines(start);
    let mut lines = lines.iter().filter(|l| !l.trim().is_empty());
    let mut first_data = None;
    for line in lines.by_ref() {
        if line.starts_with("#mpa_v") || is_header(line) {
            return true;
        }
        if !line.starts_with('#') {
            first_data = Some(*line);
            break;
        }
    }
    // A table without a header: every row a k__ lineage with numbers.
    let Some(first) = first_data else {
        return false;
    };
    std::iter::once(first)
        .chain(lines.take(20).copied())
        .all(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            cells.len() >= 2
                && cells[0].starts_with("k__")
                && cells[1..].iter().all(|c| parse_value(c).is_some())
        })
}

/// The line naming the columns: `#clade_name\t...` (profiles) or
/// `clade_name\t...` (merged tables).
fn is_header(line: &str) -> bool {
    let first = line.split('\t').next().unwrap_or("").trim();
    matches!(first, "#clade_name" | "clade_name")
}

/// What one file holds.
struct Table {
    samples: Vec<String>,
    clades: Vec<Clade>,
    /// Values are estimated read counts.
    reads: bool,
}

fn read_table(file: &NamedText<'_>) -> Result<Table> {
    let lines = text::lines(file.text.strip_prefix('\u{feff}').unwrap_or(file.text));
    let header_at = lines.iter().position(|l| is_header(l));
    let sample_id = lines
        .iter()
        .take_while(|l| l.starts_with('#'))
        .find_map(|l| l.strip_prefix("#SampleID\t"))
        .map(str::trim)
        .filter(|id| !id.is_empty() && *id != "Metaphlan_Analysis");
    let own_name = || sample_id.map_or_else(|| sample_name_of(file.name), str::to_string);

    // The columns that hold values, and the sample of each.
    let (value_columns, samples, reads, data_from) = match header_at {
        Some(at) => {
            let cells: Vec<&str> = lines[at]
                .trim_start_matches('#')
                .split('\t')
                .map(str::trim)
                .collect();
            if let Some(i) = cells.iter().position(|c| *c == READS) {
                (vec![i], vec![own_name()], true, at + 1)
            } else if let Some(i) = cells.iter().position(|c| *c == "relative_abundance") {
                (vec![i], vec![own_name()], false, at + 1)
            } else {
                let columns: Vec<usize> = (1..cells.len())
                    .filter(|&i| !NOT_SAMPLES.contains(&cells[i]) && !cells[i].is_empty())
                    .collect();
                if columns.is_empty() {
                    return Err(StampError::new(format!(
                        "{} does not appear to be a MetaPhlAn profile: its header names no \
                         samples.",
                        file.name
                    )));
                }
                let samples = columns.iter().map(|&i| cells[i].to_string()).collect();
                (columns, samples, false, at + 1)
            }
        }
        // MetaPhlAn 2: clade and relative abundance.
        None => (vec![1], vec![own_name()], false, 0),
    };

    let mut clades = Vec::new();
    for (number, line) in lines.iter().enumerate().skip(data_from) {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let bad =
            |what: &str| StampError::new(format!("Line {} of {}: {what}.", number + 1, file.name));
        let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
        let mut values = Vec::with_capacity(value_columns.len());
        for &column in &value_columns {
            let cell = cells.get(column).ok_or_else(|| bad("too few columns"))?;
            values
                .push(parse_value(cell).ok_or_else(|| bad(&format!("'{cell}' is not a number")))?);
        }
        let name = cells[0];
        let path = if matches!(
            name.to_ascii_lowercase().as_str(),
            "unclassified" | "unknown" | "-1"
        ) {
            Vec::new()
        } else {
            let path = split_lineage(name);
            if path.first().and_then(|p| rank_prefix(p)).is_none() {
                return Err(bad(&format!("'{name}' is not a MetaPhlAn clade name")));
            }
            path
        };
        clades.push(Clade { path, values });
    }
    Ok(Table {
        samples,
        clades,
        reads,
    })
}

/// The rank of a MetaPhlAn prefix: `k__` (or `d__` in GTDB profiles) to
/// `s__`, and `t__` for the SGB.
fn rank_of(letter: char) -> Option<usize> {
    "kpcofgst"
        .find(letter)
        .or_else(|| (letter == 'd').then_some(0))
}

/// Combine MetaPhlAn profiles and merged tables into a STAMP profile.
pub fn to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let tables = files
        .iter()
        .map(read_table)
        .collect::<Result<Vec<Table>>>()?;
    if tables.iter().any(|t| t.reads) && !tables.iter().all(|t| t.reads) {
        return Err(StampError::new(
            "Some MetaPhlAn profiles give estimated read counts (-t rel_ab_w_read_stats) \
             and others relative abundances; load profiles of one kind.",
        ));
    }
    check_unique(tables.iter().map(|t| t.samples.as_slice()))?;

    let (mut ncbi, mut gtdb, mut sgb) = (false, false, false);
    for part in tables.iter().flat_map(|t| &t.clades).flat_map(|c| &c.path) {
        match rank_prefix(part).map(|(letter, _)| letter) {
            Some('k') => ncbi = true,
            Some('d') => gtdb = true,
            Some('t') => sgb = true,
            _ => {}
        }
    }
    if ncbi && gtdb {
        return Err(StampError::new(
            "Some MetaPhlAn profiles use MetaPhlAn's own taxonomy (k__) and others GTDB's \
             (d__); load profiles of one kind.",
        ));
    }
    let top = if gtdb {
        ("Domain", "d__")
    } else {
        ("Kingdom", "k__")
    };
    let mut ranks = Ranks::new(
        &[
            top.0, "Phylum", "Class", "Order", "Family", "Genus", "Species",
        ],
        &[top.1, "p__", "c__", "o__", "f__", "g__", "s__"],
    );
    if sgb {
        ranks = ranks.with("SGB", "t__");
    }
    let mut lineages = Lineages::with_ranks(ranks);
    lineages.set_counts(tables.iter().all(|t| t.reads));
    for table in &tables {
        let indexes: Vec<usize> = table.samples.iter().map(|s| lineages.sample(s)).collect();
        add_clades(&mut lineages, &indexes, &table.clades, rank_of)?;
    }
    lineages.to_spf()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spf(files: &[(&str, &str)]) -> Result<String> {
        let files: Vec<NamedText<'_>> = files
            .iter()
            .map(|&(name, text)| NamedText { name, text })
            .collect();
        to_spf(&files)
    }

    const PROFILE: &str = "#mpa_vJan21_CHOCOPhlAnSGB_202103\n\
        #/usr/bin/metaphlan gut.fq --input_type fastq -o gut_profile.txt\n\
        #19048 reads processed\n\
        #SampleID\tMetaphlan_Analysis\n\
        #clade_name\tNCBI_tax_id\trelative_abundance\tadditional_species\n\
        UNCLASSIFIED\t-1\t20.0\t\n\
        k__Bacteria\t2\t80.0\t\n\
        k__Bacteria|p__Firmicutes\t2|1239\t80.0\t\n\
        k__Bacteria|p__Firmicutes|c__Clostridia\t2|1239|186801\t80.0\t\n\
        k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales\t2|1239|186801|186802\t80.0\t\n\
        k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales|f__Lachnospiraceae\
        \t2|1239|186801|186802|186803\t80.0\t\n\
        k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales|f__Lachnospiraceae|\
        g__Roseburia\t2|1239|186801|186802|186803|841\t80.0\t\n\
        k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales|f__Lachnospiraceae|\
        g__Roseburia|s__Roseburia_intestinalis\t2|1239|186801|186802|186803|841|166486\t\
        33.33333\t\n\
        k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales|f__Lachnospiraceae|\
        g__Roseburia|s__Roseburia_hominis\t2|1239|186801|186802|186803|841|301301\t\
        46.66667\tk__Bacteria|...\n\
        k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales|f__Lachnospiraceae|\
        g__Roseburia|s__Roseburia_intestinalis|t__SGB4951\t2|1239|186801|186802|186803|841|\
        166486|\t33.33333\t\n\
        k__Bacteria|p__Firmicutes|c__Clostridia|o__Eubacteriales|f__Lachnospiraceae|\
        g__Roseburia|s__Roseburia_hominis|t__SGB4936\t2|1239|186801|186802|186803|841|\
        301301|\t46.66667\t\n";

    #[test]
    fn clade_values_become_own_values() {
        assert!(looks_like_metaphlan(PROFILE));
        let spf = spf(&[("runs/gut_profile.txt", PROFILE)]).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert_eq!(
            lines[0],
            "Kingdom\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tSGB\tgut"
        );
        // 33.33333 + 46.66667 = 80: the rounding leaves nothing at the genus.
        assert_eq!(lines[1], ["unclassified Root"; 8].join("\t") + "\t20.0");
        assert!(lines[2].ends_with("\tRoseburia_intestinalis\tSGB4951\t33.3333"));
        assert!(lines[3].ends_with("\tRoseburia_hominis\tSGB4936\t46.6667"));
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn abundance_left_at_a_genus_makes_a_row() {
        let text = "#mpa_v30_CHOCOPhlAn_201901\n#metaphlan x\n#SampleID\tS3\n\
            #clade_name\tNCBI_tax_id\trelative_abundance\tadditional_species\n\
            UNKNOWN\t-1\t10.0\t\n\
            k__Bacteria\t2\t90.0\t\n\
            k__Bacteria|p__Firmicutes\t2|1239\t90.0\t\n\
            k__Bacteria|p__Firmicutes|c__Bacilli\t2|1239|91061\t60.0\t\n\
            k__Bacteria|p__Firmicutes|c__Bacilli|o__Lactobacillales\t2|1239|91061|186826\t60.0\t\n";
        let spf = spf(&[("x.txt", text)]).unwrap();
        // MetaPhlAn 3 has no SGBs; the sample name comes from #SampleID.
        assert!(spf.starts_with("Kingdom\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tS3\n"));
        assert!(spf.contains("\tFirmicutes\tunclassified p__Firmicutes\t"));
        assert!(spf.ends_with("\tunclassified o__Lactobacillales\t60.0\n"));
        assert!(spf.contains("unclassified Root\t10.0\n"));
    }

    #[test]
    fn read_counts_are_used_when_given() {
        let text = "#mpa_vJun23_CHOCOPhlAnSGB_202307\n#metaphlan x -t rel_ab_w_read_stats\n\
            #1000 reads processed\n#SampleID\tMetaphlan_Analysis\n\
            #Estimated reads mapped to known clades: 900\n#Average read length 150\n\
            #clade_name\tclade_taxid\trelative_abundance\tcoverage\t\
            estimated_number_of_reads_from_the_clade\n\
            UNCLASSIFIED\t-1\t10.0\t-\t100\n\
            k__Bacteria\t2\t90.0\t1.5\t900\n\
            k__Bacteria|p__Firmicutes\t2|1239\t60.0\t1.0\t600\n";
        let spf = spf(&[("s1_profile.tsv", text)]).unwrap();
        assert!(spf.contains("\ts1\n"));
        assert!(spf.contains("Bacteria\tunclassified k__Bacteria\t"));
        assert!(spf.contains("\t300\n"));
        assert!(spf.contains("\t600\n"));
        assert!(spf.contains("unclassified Root\t100\n"));
        let mixed = spf_err(&[("a", text), ("b", PROFILE)]);
        assert!(mixed.contains("estimated read counts"), "{mixed}");
    }

    fn spf_err(files: &[(&str, &str)]) -> String {
        spf(files).unwrap_err().0
    }

    #[test]
    fn a_profile_with_nothing_above_zero_is_refused() {
        // What MetaPhlAn 4.1 writes with -t rel_ab_w_read_stats when it
        // detects no clade: the one line, with no reads estimated.
        let text = "#mpa_vJun23_CHOCOPhlAnSGB_202403\n#metaphlan x -t rel_ab_w_read_stats\n\
            #200000 reads processed\n#SampleID\tMetaphlan_Analysis\n\
            #estimated_reads_mapped_to_known_clades:0\n\
            #clade_name\tclade_taxid\trelative_abundance\tcoverage\t\
            estimated_number_of_reads_from_the_clade\n\
            UNCLASSIFIED\t-1\t100.0\t-\t0\n";
        let message = spf_err(&[("s1_profile.txt", text)]);
        assert!(message.contains("no profile to load"), "{message}");
    }

    #[test]
    fn merged_tables_and_other_variants() {
        // merge_metaphlan_tables.py of MetaPhlAn 4: no taxid column.
        let merged = "#mpa_vJan21_CHOCOPhlAnSGB_202103\n\
            clade_name\tA\tB\n\
            UNCLASSIFIED\t10.0\t0.0\n\
            k__Bacteria\t90.0\t100.0\n\
            k__Bacteria|p__Firmicutes\t90.0\t0.0\n\
            k__Bacteria|p__Bacteroidetes\t0.0\t100.0\n";
        assert!(looks_like_metaphlan(merged));
        let spf_text = spf(&[("merged.txt", merged)]).unwrap();
        assert!(spf_text.contains("\tA\tB\n"));
        assert!(spf_text.contains("Bacteria\tFirmicutes\t"));
        assert!(spf_text.contains("\t90.0\t0\n"));
        assert!(spf_text.contains("\t0\t100.0\n"));
        // MetaPhlAn 3 merges keep the taxid column.
        let v3 = "#mpa_v30_CHOCOPhlAn_201901\nclade_name\tNCBI_tax_id\tA_profile\n\
            k__Bacteria\t2\t100.0\n";
        assert!(spf(&[("m", v3)]).unwrap().contains("\tA_profile\n"));
        // GTDB profiles from sgb_to_gtdb_profile.py.
        let gtdb = "#mpa_vJan25_CHOCOPhlAnSGB_202503\tr220\n#clade_name\trelative_abundance\n\
            UNCLASSIFIED\t5.0\n\
            d__Bacteria\t95.0\n\
            d__Bacteria;p__Bacillota\t95.0\n";
        let gtdb_spf = spf(&[("g_profile.txt", gtdb)]).unwrap();
        assert!(gtdb_spf.starts_with("Domain\tPhylum\t"));
        assert!(gtdb_spf.contains("\tg\n"));
        assert!(spf_err(&[("a", gtdb), ("b", PROFILE)]).contains("GTDB"));
        // MetaPhlAn 2 wrote no header at all.
        let legacy = "k__Bacteria\t100.0\nk__Bacteria|p__Firmicutes\t100.0\n";
        assert!(looks_like_metaphlan(legacy));
        assert!(spf(&[("old.txt", legacy)]).unwrap().contains("\told\n"));
    }

    #[test]
    fn other_files_are_not_metaphlan() {
        for other in [
            "sample\tcoverage\ttaxonomy\n",
            "Domain\tPhylum\tA\tB\nBacteria\tFirmicutes\t1\t2\n",
            "d__Bacteria\t100\nd__Bacteria|p__Bacillota\t50\n",
            "",
        ] {
            assert!(!looks_like_metaphlan(other), "{other}");
        }
        let bad = "#mpa_v30_x\n#clade_name\tNCBI_tax_id\trelative_abundance\nBacteria\t2\t1\n";
        assert!(spf_err(&[("x", bad)]).contains("is not a MetaPhlAn clade name"));
    }
}
