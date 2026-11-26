//! mOTUs profiles: `motus profile` and `motus merge` output of mOTUs 2 and
//! 3, and the different files of mOTUs 4. See `docs/PROFILER_FORMATS.md`.
//!
//! mOTUs 2 and 3 write three `#` lines, the last naming the columns
//! (`#consensus_taxonomy\t<sample>...`, with `mOTU` and `NCBI_tax_id`
//! columns for `-u`, `-q` and `-p`, or `#mOTUs2_clade\t<sample>` for `-A`).
//! Rows are mOTUs (`Kandleria vitulina [ref_mOTU_v2_0001]`), taxa of the
//! rank chosen with `-k`, or with `-q`/`-A` lineages (`k__Bacteria|...|s__X
//! [ref_mOTU_v3_00095]`). mOTUs 4 writes `#tool_version=...` and then
//! `mOTU\tTaxonomy\t<sample>...`, with `;`-joined GTDB lineages. The
//! `unassigned` row (`-1` in old versions) becomes `unclassified Root`.

use super::lineage::{add_clades, parse_value, rank_prefix, split_lineage, Clade, Lineages, Ranks};
use super::{check_unique, complete_lines, sample_name_of, NamedText};
use crate::error::{Result, StampError};
use crate::text;

/// True if the start of a file looks like a mOTUs profile.
pub fn looks_like_motus(start: &str) -> bool {
    let lines = complete_lines(start);
    let mut lines = lines.iter().map(|l| l.trim_end()).filter(|l| !l.is_empty());
    let Some(first) = lines.next() else {
        return false;
    };
    if first.starts_with("#tool_version=") {
        return first.contains("value_type=")
            && lines.next().is_some_and(|l| l.starts_with("mOTU\t"));
    }
    std::iter::once(first)
        .chain(lines)
        .take_while(|l| l.starts_with('#'))
        .take(4)
        .any(|l| {
            l.starts_with("# motus version")
                || (l.starts_with("# git tag version") && l.contains("motus"))
                || is_column_header(l)
        })
}

/// The `#` line naming the columns of a mOTUs 2 or 3 profile.
fn is_column_header(line: &str) -> bool {
    let first = line.split('\t').next().unwrap_or("");
    matches!(first, "#consensus_taxonomy" | "#mOTU")
        || is_clade_column(&first[1.min(first.len())..])
}

/// `mOTUs2_clade`, `mOTUs3_clade`, ...: the column of `-A` profiles.
fn is_clade_column(cell: &str) -> bool {
    cell.strip_prefix("mOTUs")
        .and_then(|rest| rest.strip_suffix("_clade"))
        .is_some_and(|version| version.chars().all(|c| c.is_ascii_digit()))
}

/// What a mOTUs file holds once read.
struct Table {
    samples: Vec<String>,
    /// Rows with lineages (`-q`, `-A`, mOTUs 4), or names at one rank.
    lineages: bool,
    /// The rank of the names when there are no lineages (`mOTU`, `genus`).
    level: String,
    clades: Vec<Clade>,
    counts: bool,
}

/// Read one mOTUs file.
fn read_table(file: &NamedText<'_>) -> Result<Table> {
    let lines = text::lines(file.text.strip_prefix('\u{feff}').unwrap_or(file.text));
    let invalid = |reason: &str| {
        StampError::new(format!(
            "{} does not appear to be a mOTUs profile: {reason}.",
            file.name
        ))
    };
    let first = lines.first().map_or("", |l| l.trim_end());
    let version4 = first.starts_with("#tool_version=");

    // The line naming the columns, and the lines of data after it.
    let (header_at, header) = if version4 {
        let at = lines
            .iter()
            .position(|l| l.starts_with("mOTU\t"))
            .ok_or_else(|| invalid("it has no line naming the columns mOTU and Taxonomy"))?;
        (at, lines[at].trim_end())
    } else {
        let at = lines
            .iter()
            .take_while(|l| l.starts_with('#'))
            .count()
            .checked_sub(1)
            .ok_or_else(|| invalid("it has no header lines starting with #"))?;
        (at, lines[at].trim_end().trim_start_matches('#'))
    };
    let cells: Vec<&str> = header.split('\t').map(str::trim).collect();
    let known = |c: &str| {
        matches!(
            c,
            "mOTU" | "consensus_taxonomy" | "NCBI_tax_id" | "Taxonomy"
        ) || is_clade_column(c)
    };
    let first_sample = cells.iter().position(|c| !known(c)).unwrap_or(cells.len());
    if first_sample == cells.len() {
        return Err(invalid("its header names no samples"));
    }
    let name_col = cells
        .iter()
        .position(|c| matches!(*c, "consensus_taxonomy" | "Taxonomy") || is_clade_column(c))
        .ok_or_else(|| invalid("its header has no consensus_taxonomy or Taxonomy column"))?;
    let id_col = cells.iter().position(|c| *c == "mOTU");
    let samples: Vec<String> = cells[first_sample..]
        .iter()
        .map(|c| {
            if c.is_empty() || *c == "unnamed sample" {
                sample_name_of(file.name)
            } else {
                c.to_string()
            }
        })
        .collect();
    let level = if version4 {
        "mOTU".to_string()
    } else {
        // `calc_motu 3.1.0 -k genus` in the first line; mOTUs by default.
        first
            .split(" -k ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or("mOTU")
            .to_string()
    };

    let mut clades = Vec::new();
    let mut counts = true;
    let mut lineages = None;
    for (number, line) in lines.iter().enumerate().skip(header_at + 1) {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let bad =
            |what: &str| StampError::new(format!("Line {} of {}: {what}.", number + 1, file.name));
        let row: Vec<&str> = line.split('\t').map(str::trim).collect();
        if row.len() < cells.len() {
            return Err(bad("too few columns"));
        }
        let mut values = Vec::with_capacity(samples.len());
        for cell in &row[first_sample..cells.len()] {
            let value =
                parse_value(cell).ok_or_else(|| bad(&format!("'{cell}' is not a number")))?;
            counts &= value.1 == 0.0;
            values.push(value);
        }
        let name = row[name_col];
        let id = id_col.map(|i| row[i]).filter(|id| !id.is_empty());
        let unassigned = matches!(name, "unassigned" | "-1" | "")
            || id.is_some_and(|id| id.contains("unassigned"));
        let path = if unassigned {
            Vec::new()
        } else if rank_prefix(name).is_some() {
            if lineages == Some(false) {
                return Err(bad("some rows are lineages and others are not"));
            }
            lineages = Some(true);
            let mut path = split_lineage(name);
            // The species of a mOTU carries the mOTU's id, so that each
            // mOTU stays a species of its own.
            if let (Some(id), Some(last)) = (id, path.last_mut()) {
                if last.starts_with("s__") && !last.contains(id) {
                    last.push_str(&format!(" [{id}]"));
                }
            }
            path
        } else {
            if lineages == Some(true) {
                return Err(bad("some rows are lineages and others are not"));
            }
            lineages = Some(false);
            match id {
                Some(id) if !name.contains(id) => vec![format!("{name} [{id}]")],
                _ => vec![name.to_string()],
            }
        };
        clades.push(Clade { path, values });
    }
    Ok(Table {
        samples,
        lineages: lineages.unwrap_or(false),
        level,
        clades,
        counts,
    })
}

/// Combine mOTUs profiles and merged tables into a STAMP profile.
pub fn to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let tables = files
        .iter()
        .map(read_table)
        .collect::<Result<Vec<Table>>>()?;
    let Some(first) = tables.first() else {
        return Err(StampError::new("No mOTUs profiles were given."));
    };
    for (table, file) in tables.iter().zip(files) {
        if table.lineages != first.lineages {
            return Err(StampError::new(format!(
                "{} and {} are different kinds of mOTUs profile: one gives the full \
                 taxonomy of each mOTU (motus profile -q) and the other does not.",
                files[0].name, file.name
            )));
        }
        if !table.lineages && table.level != first.level {
            return Err(StampError::new(format!(
                "{} is a profile of {} and {} one of {}; load profiles of one rank \
                 (motus profile -k).",
                files[0].name, first.level, file.name, table.level
            )));
        }
    }

    let ranks = if first.lineages {
        lineage_ranks(&tables)?
    } else {
        let level = &first.level;
        let mut heading = level.to_string();
        if level != "mOTU" {
            heading[..1].make_ascii_uppercase();
        }
        Ranks::single(&heading)
    };
    check_unique(tables.iter().map(|t| t.samples.as_slice()))?;
    let mut lineages = Lineages::with_ranks(ranks);
    lineages.set_counts(tables.iter().all(|t| t.counts));
    for table in &tables {
        let indexes: Vec<usize> = table.samples.iter().map(|s| lineages.sample(s)).collect();
        if first.lineages {
            add_clades(&mut lineages, &indexes, &table.clades, rank_of)?;
        } else {
            for clade in &table.clades {
                let names: Vec<&str> = clade.path.iter().map(String::as_str).collect();
                for (column, &(value, _)) in clade.values.iter().enumerate() {
                    if value > 0.0 {
                        lineages.add(indexes[column], &names, value)?;
                    }
                }
            }
        }
    }
    lineages.to_spf()
}

/// Ranks of mOTUs lineages: k__ (NCBI, mOTUs 2 and 3) or d__ (GTDB, mOTUs
/// 4) at the top, then p__ to s__.
fn rank_of(letter: char) -> Option<usize> {
    "kpcofgs"
        .find(letter)
        .or_else(|| (letter == 'd').then_some(0))
}

/// The ranks of tables of lineages, down to the lowest rank any reaches.
fn lineage_ranks(tables: &[Table]) -> Result<Ranks> {
    let (mut ncbi, mut gtdb, mut deepest) = (false, false, 0);
    for clade in tables.iter().flat_map(|t| &t.clades) {
        for part in &clade.path {
            if let Some((letter, _)) = rank_prefix(part) {
                ncbi |= letter == 'k';
                gtdb |= letter == 'd';
                deepest = deepest.max(rank_of(letter).unwrap_or(0));
            }
        }
    }
    if ncbi && gtdb {
        return Err(StampError::new(
            "The mOTUs profiles mix NCBI lineages (k__, mOTUs 2 and 3) and GTDB \
             lineages (d__, mOTUs 4); load profiles made with one version.",
        ));
    }
    let top = if gtdb {
        ("Domain", "d__")
    } else {
        ("Kingdom", "k__")
    };
    let mut ranks = Ranks::new(&[top.0], &[top.1]);
    for (name, prefix) in [
        ("Phylum", "p__"),
        ("Class", "c__"),
        ("Order", "o__"),
        ("Family", "f__"),
        ("Genus", "g__"),
        ("Species", "s__"),
    ]
    .into_iter()
    .take(deepest)
    {
        ranks = ranks.with(name, prefix);
    }
    Ok(ranks)
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

    const V3: &str = "# git tag version 3.1.0 |  motus version 3.1.0 | map_tax 3.1.0 | gene \
        database: nr3.1.0 | calc_mgc 3.1.0 -y insert.scaled_counts -l 75 | calc_motu 3.1.0 -k \
        mOTU -C no_CAMI -g 3 | taxonomy: ref_mOTU_3.1.0 meta_mOTU_3.1.0\n\
        # call: python motus profile -s reads.fq -n gut1\n\
        #consensus_taxonomy\tgut1\n\
        Kandleria vitulina [ref_mOTU_v31_00001]\t0.0688211617\n\
        Escherichia coli [ref_mOTU_v31_00095]\t0.7000000000\n\
        unknown Clostridiales [meta_mOTU_v31_05308]\t0.0000000000\n\
        unassigned\t0.2311788383\n";

    #[test]
    fn default_profiles_have_one_level_of_motus() {
        assert!(looks_like_motus(V3));
        let other = V3.replace("gut1", "gut2");
        let spf = spf(&[("a.motus", V3), ("b.motus", &other)]).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert_eq!(lines[0], "mOTU\tgut1\tgut2");
        assert_eq!(
            lines[1],
            "Kandleria vitulina [ref_mOTU_v31_00001]\t0.0688212\t0.0688212"
        );
        // A mOTU found in no sample makes no row; unassigned is unclassified.
        assert_eq!(lines[3], "unclassified Root\t0.231179\t0.231179");
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn options_change_the_columns() {
        // -k genus, -p (NCBI ids), unnamed sample, -1 for unassigned.
        let text = "# git tag version 2.5.1 | motus version 2.5.1 | calc_motu 2.5.1 -k genus -c\n\
            # call: python motus profile -s x.fq -k genus -p -c\n\
            #consensus_taxonomy\tNCBI_tax_id\tunnamed sample\n\
            Escherichia\t561\t12\n\
            -1\tNA\t3\n";
        let genus = spf(&[("runs/S7.motus", text)]).unwrap();
        assert_eq!(genus, "Genus\tS7\nEscherichia\t12\nunclassified Root\t3\n");

        // -u: the mOTU id in a column of its own goes with the name.
        let text = "# motus version 3.0.3 | merge 3.0.3\n# call: motus merge\n\
            #mOTU\tconsensus_taxonomy\tA\tB\n\
            ref_mOTU_v3_00035\tEscherichia sp. [C KTE11]\t1\t2\n\
            unassigned\tunassigned\t3\t4\n";
        assert!(spf(&[("m.tsv", text)])
            .unwrap()
            .contains("Escherichia sp. [C KTE11] [ref_mOTU_v3_00035]\t1\t2\n"));
    }

    #[test]
    fn full_taxonomy_gives_ranks_to_species() {
        let text = "# git tag version 3.1.0 | calc_motu 3.1.0 -k mOTU -c -q\n# call: x\n\
            #mOTU\tconsensus_taxonomy\tgut\n\
            ref_mOTU_v31_00095\tk__Bacteria|p__Proteobacteria|c__Gammaproteobacteria|\
            o__Enterobacterales|f__Enterobacteriaceae|g__Escherichia|\
            s__Escherichia coli [ref_mOTU_v31_00095]\t40\n\
            ref_mOTU_v31_00096\tk__Bacteria|p__Proteobacteria|c__Gammaproteobacteria|\
            o__Enterobacterales|f__Enterobacteriaceae|g__Escherichia|\
            s__Escherichia albertii [ref_mOTU_v31_00096]\t0\n\
            unassigned\tunassigned\t7\n";
        let other = text.replace("\tgut\n", "\tsoil\n");
        let spf = spf(&[("a", text), ("b", &other)]).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert_eq!(
            lines[0],
            "Kingdom\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tgut\tsoil"
        );
        assert!(lines[1].ends_with("\tEscherichia coli [ref_mOTU_v31_00095]\t40\t40"));
        assert!(lines[2].starts_with("unclassified Root\t"));
        assert!(lines[2].ends_with("\t7\t7"));
    }

    #[test]
    fn all_ranks_are_clade_values() {
        // -A lists every rank with the values of whole clades.
        let text = "#mOTUs2_clade\tgut\n\
            k__Bacteria\t0.8000000000\n\
            k__Bacteria|p__Firmicutes\t0.5000000000\n\
            k__Bacteria|p__Firmicutes|c__Clostridia\t0.4500000000\n\
            unassigned\t0.2000000000\n";
        assert!(looks_like_motus(text));
        let spf = spf(&[("a", text), ("b", &text.replace("\tgut", "\tsoil"))]).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert_eq!(lines[0], "Kingdom\tPhylum\tClass\tgut\tsoil");
        assert_eq!(
            lines[1],
            "Bacteria\tunclassified k__Bacteria\tunclassified k__Bacteria\t0.3\t0.3"
        );
        assert_eq!(
            lines[2],
            "Bacteria\tFirmicutes\tunclassified p__Firmicutes\t0.05\t0.05"
        );
        assert_eq!(lines[3], "Bacteria\tFirmicutes\tClostridia\t0.45\t0.45");
    }

    #[test]
    fn motus_4_profiles() {
        let text = "#tool_version=4.1.0\tdatabase_version=4.1\tmin_alignment_length=75\t\
            min_mgcs=3\tcount_mode=INSERT_SCALED\tvalue_type=counts\n\
            mOTU\tTaxonomy\tERR1\tERR2\n\
            mOTUv4.0_000255\td__Bacteria;p__Bacillota;c__Clostridia;o__Oscillospirales;\
            f__Oscillospiraceae;g__Flavonifractor;s__Flavonifractor plautii\t48\t43\n\
            mOTUv4.0_000611\td__Bacteria;p__Bacillota;c__Bacilli;o__Lactobacillales;\
            f__Lactobacillaceae;g__Limosilactobacillus;\
            s__Unknown Limosilactobacillus mOTUv4.0_000611\t125\t0\n\
            mOTUv4.0_unassigned\td__;p___A;c__;o__;f__;g__;s__\t9\t11\n";
        assert!(looks_like_motus(text));
        let spf = spf(&[("merged.tsv", text)]).unwrap();
        assert!(
            spf.starts_with("Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tERR1\tERR2\n")
        );
        assert!(spf.contains("\tFlavonifractor plautii [mOTUv4.0_000255]\t48\t43\n"));
        assert!(spf.contains("\tUnknown Limosilactobacillus mOTUv4.0_000611\t125\t0\n"));
        assert!(spf.contains("unclassified Root\t9\t11\n"));
        crate::ProfileTree::read(&spf).unwrap();
    }

    #[test]
    fn problems_are_reported() {
        let flat = V3;
        let lineages = "#mOTUs2_clade\tgut2\nk__Bacteria\t1.0\n";
        assert!(spf(&[("a", flat), ("b", lineages)])
            .unwrap_err()
            .0
            .contains("different kinds of mOTUs profile"));
        assert!(spf(&[("a", V3), ("b", V3)])
            .unwrap_err()
            .0
            .contains("Sample 'gut1' is in more than one file"));
        let bad = V3.replace("0.7000000000", "lots");
        assert_eq!(
            spf(&[("x", &bad)]).unwrap_err().0,
            "Line 5 of x: 'lots' is not a number."
        );
        for other in [
            "sample\tcoverage\ttaxonomy\n",
            "#mpa_vJan21_CHOCOPhlAnSGB_202103\n",
            "# Gene Family\tS_Abundance-RPKs\n",
            "Level\tA\tB\nx\t1\t2\n",
            "",
        ] {
            assert!(!looks_like_motus(other), "{other}");
        }
    }
}
