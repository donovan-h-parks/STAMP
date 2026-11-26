//! HUMAnN 3 tables: gene families (`<sample>_genefamilies.tsv`), pathway
//! abundance (`_pathabundance.tsv`) and pathway coverage
//! (`_pathcoverage.tsv`), one sample per file or joined by
//! `humann_join_tables`, and the tables HUMAnN's tools derive from them
//! (renormalised, regrouped, renamed). See `docs/PROFILER_FORMATS.md`.
//!
//! The first line names the columns: `# Gene Family` or `# Pathway`, then one
//! column per sample (`<sample>_Abundance-RPKs`, `<sample>_Abundance`,
//! `<sample>_Coverage`). A row is a feature's community total (`UniRef90_X`,
//! `PWY-101: photorespiration`, `UNMAPPED`, `UNINTEGRATED`) or its share in
//! one organism (`<feature>|g__Bacteroides.s__Bacteroides_dorei`, or
//! `<feature>|unclassified`).
//!
//! Gene families: the strata of a family sum to its community total, so a
//! table with strata gives the levels Gene family, Genus and Species (the
//! part of a total in no stratum, and the `unclassified` stratum, become
//! `unclassified <family>` at the lower levels); without strata, the
//! families alone.
//!
//! Pathways: HUMAnN computes a pathway's community abundance on its own, and
//! it is not the sum of the pathway's strata; coverage is not additive at
//! all. A pathway profile therefore holds the community values only, one
//! level. A table of strata alone (`humann_split_stratified_table`'s
//! `_stratified.tsv`) gives Pathway, Genus and Species for abundance, and is
//! refused for coverage.

use super::lineage::{own_values, parse_value, Clade, Lineages, Ranks};
use super::{check_unique, complete_lines, sample_name_of, NamedText};
use crate::error::{Result, StampError};
use crate::text;

/// The three tables HUMAnN writes for each sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    GeneFamilies,
    PathAbundance,
    PathCoverage,
}

impl TableKind {
    /// Heading of the level of features.
    fn heading(self) -> &'static str {
        match self {
            TableKind::GeneFamilies => "Gene family",
            TableKind::PathAbundance | TableKind::PathCoverage => "Pathway",
        }
    }
}

/// The first cell of the header without `#` (and, in HUMAnN 4, without
/// the version and units that follow the name).
fn first_heading(line: &str) -> &str {
    let cell = line
        .split('\t')
        .next()
        .unwrap_or("")
        .trim_start_matches('#')
        .trim();
    ["Gene Family", "Pathway"]
        .into_iter()
        .find(|name| {
            cell.strip_prefix(name)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
        })
        .unwrap_or(cell)
}

/// The kind of HUMAnN table a file starts like, or `None`.
pub fn table_kind(start: &str) -> Option<TableKind> {
    let lines = complete_lines(start);
    let header = lines.iter().find(|l| !l.trim().is_empty())?;
    if !header.starts_with('#') {
        return None;
    }
    let columns: Vec<&str> = header.split('\t').skip(1).map(str::trim).collect();
    if columns.is_empty() {
        return None;
    }
    let coverage = columns
        .iter()
        .all(|c| c.ends_with("_Coverage") || c.ends_with("-Coverage"));
    match first_heading(header) {
        "Gene Family" => Some(TableKind::GeneFamilies),
        "Pathway" if coverage => Some(TableKind::PathCoverage),
        "Pathway" => Some(TableKind::PathAbundance),
        _ => None,
    }
}

/// The sample of a column: its heading without `_Abundance...` or
/// `_Coverage...`.
fn sample_of_column(heading: &str, file_name: &str) -> String {
    let cut = ["_Abundance", "_Coverage", "-Abundance", "-Coverage"]
        .iter()
        .filter_map(|suffix| {
            heading.rfind(suffix).filter(|&i| {
                let rest = &heading[i + suffix.len()..];
                rest.is_empty() || rest.starts_with('-')
            })
        })
        .min();
    let name = match cut {
        Some(i) => &heading[..i],
        None => heading,
    };
    if name.is_empty() {
        sample_name_of(file_name)
    } else {
        name.to_string()
    }
}

/// The genus and species of a stratum (`g__Bacteroides.s__Bacteroides_dorei`;
/// `s__Bacteroides_dorei.t__SGB1862` in HUMAnN 4), either of which may be
/// missing (both for `unclassified`).
fn organism(stratum: &str) -> (Option<&str>, Option<&str>) {
    let stratum = stratum.trim();
    let stratum = stratum.find(".t__").map_or(stratum, |i| &stratum[..i]);
    let (genus, species) = match stratum.find(".s__") {
        Some(i) => (&stratum[..i], Some(&stratum[i + 4..])),
        None => match stratum.strip_prefix("s__") {
            Some(species) => ("", Some(species)),
            None => (stratum, None),
        },
    };
    let species = species.filter(|s| !s.is_empty());
    // HUMAnN 4 names only the species, `Genus_epithet` as MetaPhlAn names
    // it (`GGB1234_SGB5678` for unnamed species, GGB1234 being the genus).
    let genus = genus
        .strip_prefix("g__")
        .filter(|g| !g.is_empty())
        .or_else(|| species.and_then(|s| s.split_once('_')).map(|(g, _)| g));
    (genus, species)
}

struct Table {
    samples: Vec<String>,
    clades: Vec<Clade>,
}

fn read_table(file: &NamedText<'_>, kind: TableKind) -> Result<Table> {
    let lines = text::lines(file.text.strip_prefix('\u{feff}').unwrap_or(file.text));
    let header_at = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
    let header = lines.get(header_at).copied().unwrap_or("");
    if table_kind(header) != Some(kind) {
        return Err(StampError::new(format!(
            "{} is not a HUMAnN {} table.",
            file.name,
            match kind {
                TableKind::GeneFamilies => "gene families",
                TableKind::PathAbundance => "pathway abundance",
                TableKind::PathCoverage => "pathway coverage",
            }
        )));
    }
    let samples: Vec<String> = header
        .split('\t')
        .skip(1)
        .map(|c| sample_of_column(c.trim(), file.name))
        .collect();
    let mut clades = Vec::new();
    for (number, line) in lines.iter().enumerate().skip(header_at + 1) {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let bad =
            |what: &str| StampError::new(format!("Line {} of {}: {what}.", number + 1, file.name));
        let cells: Vec<&str> = line.split('\t').collect();
        if cells.len() != samples.len() + 1 {
            return Err(bad("the number of columns differs from the header's"));
        }
        let values = cells[1..]
            .iter()
            .map(|c| parse_value(c).ok_or_else(|| bad(&format!("'{}' is not a number", c.trim()))))
            .collect::<Result<Vec<_>>>()?;
        let path: Vec<String> = cells[0].split('|').map(|s| s.trim().to_string()).collect();
        if path.len() > 2 || path[0].is_empty() {
            return Err(bad(&format!("'{}' is not a HUMAnN feature", cells[0])));
        }
        clades.push(Clade { path, values });
    }
    if kind != TableKind::GeneFamilies {
        // Community values where the table has them.
        if clades.iter().any(|c| c.path.len() == 1) {
            clades.retain(|c| c.path.len() == 1);
        } else if kind == TableKind::PathCoverage && !clades.is_empty() {
            return Err(StampError::new(format!(
                "{} holds the coverage of pathways in single organisms only; coverage \
                 cannot be added up, so load a table with the community coverage.",
                file.name
            )));
        }
    }
    Ok(Table { samples, clades })
}

/// Combine HUMAnN tables of one kind into a STAMP profile.
pub fn to_spf(files: &[NamedText<'_>], kind: TableKind) -> Result<String> {
    let tables = files
        .iter()
        .map(|f| read_table(f, kind))
        .collect::<Result<Vec<Table>>>()?;
    check_unique(tables.iter().map(|t| t.samples.as_slice()))?;
    let stratified = tables
        .iter()
        .flat_map(|t| &t.clades)
        .any(|c| c.path.len() > 1);
    let ranks = if stratified {
        Ranks::new(&[kind.heading(), "Genus", "Species"], &["", "g__", "s__"])
    } else {
        Ranks::single(kind.heading())
    };
    let mut lineages = Lineages::with_ranks(ranks);
    for table in &tables {
        let indexes: Vec<usize> = table.samples.iter().map(|s| lineages.sample(s)).collect();
        let own = own_values(&table.clades);
        for (clade, own) in table.clades.iter().zip(&own) {
            let mut lineage = vec![(0, clade.path[0].as_str())];
            if let Some(stratum) = clade.path.get(1) {
                let (genus, species) = organism(stratum);
                lineage.extend(genus.map(|g| (1, g)));
                lineage.extend(species.map(|s| (2, s)));
            }
            for (column, &value) in own.iter().enumerate() {
                if value > 0.0 {
                    lineages.add_ranked(indexes[column], &lineage, value)?;
                }
            }
        }
    }
    lineages.to_spf()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spf(kind: TableKind, files: &[(&str, &str)]) -> Result<String> {
        let files: Vec<NamedText<'_>> = files
            .iter()
            .map(|&(name, text)| NamedText { name, text })
            .collect();
        to_spf(&files, kind)
    }

    const GENES: &str = "# Gene Family\tgut_Abundance-RPKs\n\
        UNMAPPED\t100.0000000000\n\
        UniRef90_A0A174QX49\t50.0000000000\n\
        UniRef90_A0A174QX49|g__Bacteroides.s__Bacteroides_dorei\t30.0000000000\n\
        UniRef90_A0A174QX49|unclassified\t15.0000000000\n\
        UniRef90_R5IH84\t8.0000000000\n\
        UniRef90_R5IH84|g__Roseburia.s__Roseburia_intestinalis\t8.0000000000";

    #[test]
    fn gene_families_with_strata() {
        assert_eq!(table_kind(GENES), Some(TableKind::GeneFamilies));
        let spf = spf(TableKind::GeneFamilies, &[("gut_genefamilies.tsv", GENES)]).unwrap();
        assert_eq!(
            spf,
            "Gene family\tGenus\tSpecies\tgut\n\
             UNMAPPED\tunclassified UNMAPPED\tunclassified UNMAPPED\t100.0\n\
             UniRef90_A0A174QX49\tunclassified UniRef90_A0A174QX49\t\
             unclassified UniRef90_A0A174QX49\t20.0\n\
             UniRef90_A0A174QX49\tBacteroides\tBacteroides_dorei\t30.0\n\
             UniRef90_R5IH84\tRoseburia\tRoseburia_intestinalis\t8.0\n"
        );
        // One sample is not enough for STAMP.
        crate::ProfileTree::read(&spf).unwrap_err();
    }

    #[test]
    fn joined_and_unstratified_tables() {
        // humann_join_tables, renormalised with --update-snames.
        let joined = "# Gene Family\tA_Abundance-CPM\tB_Abundance-CPM\n\
            UNMAPPED\t500000\t400000\n\
            UniRef90_X\t500000\t600000\n";
        let spf_text = spf(TableKind::GeneFamilies, &[("joined.tsv", joined)]).unwrap();
        assert_eq!(
            spf_text,
            "Gene family\tA\tB\nUNMAPPED\t500000.0\t400000.0\nUniRef90_X\t500000.0\t600000.0\n"
        );
        crate::ProfileTree::read(&spf_text).unwrap();
        // HUMAnN 4: version and units in the first cell, bare sample names,
        // strata with SGBs.
        let v4 = "# Gene Family HUMAnN v4.0.0.alpha.2 Adjusted CPMs\tS1\n\
            READS_UNMAPPED\t10\n\
            UniRef90_X\t5\n\
            UniRef90_X|s__Bacteroides_dorei.t__SGB1862\t5\n";
        assert_eq!(table_kind(v4), Some(TableKind::GeneFamilies));
        assert!(spf(TableKind::GeneFamilies, &[("x", v4)])
            .unwrap()
            .contains("UniRef90_X\tBacteroides\tBacteroides_dorei\t5.0\n"));
    }

    const PATHWAYS: &str = "# Pathway\tgut_Abundance\n\
        UNMAPPED\t4.7291449\n\
        UNINTEGRATED\t902.1316523\n\
        UNINTEGRATED|unclassified\t172.4039279\n\
        PWY0-1586: peptidoglycan maturation (meso-diaminopimelate containing)\t1.1798147\n\
        PWY0-1586: peptidoglycan maturation (meso-diaminopimelate containing)|unclassified\t\
        1.1798147\n\
        PWY-1042: glycolysis IV\t3.0\n\
        PWY-1042: glycolysis IV|g__Bacteroides.s__Bacteroides_dorei\t2.0\n\
        PWY-1042: glycolysis IV|g__Roseburia.s__Roseburia_intestinalis\t1.5\n";

    #[test]
    fn pathways_hold_community_values() {
        assert_eq!(table_kind(PATHWAYS), Some(TableKind::PathAbundance));
        let spf_text = spf(
            TableKind::PathAbundance,
            &[("gut_pathabundance.tsv", PATHWAYS)],
        )
        .unwrap();
        // Not 3.5, the sum of the strata.
        assert!(
            spf_text.ends_with("PWY-1042: glycolysis IV\t3.0\n"),
            "{spf_text}"
        );
        assert!(spf_text.starts_with("Pathway\tgut\n"));

        let coverage = PATHWAYS.replace("_Abundance", "_Coverage");
        assert_eq!(table_kind(&coverage), Some(TableKind::PathCoverage));
        assert!(spf(TableKind::PathCoverage, &[("c", &coverage)])
            .unwrap()
            .starts_with("Pathway\tgut\n"));

        // Strata alone (humann_split_stratified_table).
        let stratified: String = PATHWAYS
            .lines()
            .filter(|l| l.contains('|') || l.starts_with('#'))
            .map(|l| format!("{l}\n"))
            .collect();
        assert!(spf(TableKind::PathAbundance, &[("s", &stratified)])
            .unwrap()
            .starts_with("Pathway\tGenus\tSpecies\tgut\n"));
        let stratified = stratified.replace("_Abundance", "_Coverage");
        assert!(spf(
            TableKind::PathCoverage,
            &[("s_pathcoverage.tsv", &stratified)]
        )
        .unwrap_err()
        .0
        .contains("cannot be added up"));
    }

    #[test]
    fn problems_and_other_files() {
        let bad = "# Pathway\tA_Abundance\tB_Abundance\nPWY-1\t1\n";
        assert!(spf(TableKind::PathAbundance, &[("x", bad)])
            .unwrap_err()
            .0
            .contains("number of columns"));
        assert!(spf(TableKind::GeneFamilies, &[("x", PATHWAYS)])
            .unwrap_err()
            .0
            .contains("not a HUMAnN gene families table"));
        for other in [
            "#clade_name\tNCBI_tax_id\trelative_abundance\n",
            "Gene Family\tA\n",
            "# Reaction\tA\n",
            "",
        ] {
            assert_eq!(table_kind(other), None, "{other}");
        }
        assert_eq!(organism("unclassified"), (None, None));
        assert_eq!(
            organism("g__Bacteroides.s__Bacteroides_dorei"),
            (Some("Bacteroides"), Some("Bacteroides_dorei"))
        );
    }
}
