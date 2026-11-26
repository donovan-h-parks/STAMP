//! PICRUSt2 tables: the predicted functions of a 16S community
//! (`EC_metagenome_out/pred_metagenome_unstrat.tsv.gz` and the KO, COG,
//! Pfam and TIGRFAM equivalents), MetaCyc pathway abundance and coverage
//! (`pathways_out/path_abun_unstrat.tsv.gz`, `path_cov_unstrat.tsv.gz`), and
//! the contributions of each sequence (ASV) to them (`--stratified`). See
//! `docs/PROFILER_FORMATS.md`.
//!
//! * Unstratified tables: `function` or `pathway`, optionally `description`
//!   (added by `add_descriptions.py`), then one column per sample; a row per
//!   function. One level, named after the kind of function (`EC`, `KO`,
//!   ...); with descriptions the feature is `<id>: <description>`, as HUMAnN
//!   names its pathways.
//! * Pathway coverage has the same header as pathway abundance; it is told
//!   by its values, which are fractions from 0 to 1.
//! * Contributions, the long table `--stratified` writes
//!   (`pred_metagenome_contrib.tsv.gz`, `path_abun_contrib.tsv.gz`: `sample`,
//!   `function`, `taxon`, ..., `taxon_function_abun`, ...), or the wide one
//!   `--wide_table` writes (`pred_metagenome_strat.tsv.gz`,
//!   `path_abun_strat.tsv.gz`: `function` or `pathway`, `sequence`, then the
//!   samples). Two levels, the function and the sequence; each sequence's
//!   value is its `taxon_function_abun`, and in PICRUSt2 2.6 a function's
//!   contributions add up to its unstratified value, so the function level
//!   holds the same numbers as the unstratified table.

use super::lineage::{parse_value, Lineages, Ranks};
use super::{check_unique, complete_lines, NamedText};
use crate::error::{Result, StampError};
use crate::text;

/// The kinds of PICRUSt2 table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    /// Predicted functions (EC, KO, ...), one row per function.
    Functions,
    /// MetaCyc pathway abundance, one row per pathway.
    PathAbundance,
    /// MetaCyc pathway coverage (`--coverage`), one row per pathway.
    PathCoverage,
    /// The contributions of each sequence to functions or pathways.
    Contributions,
}

/// The columns of the long contribution table.
const CONTRIB_START: [&str; 3] = ["sample", "function", "taxon"];
const CONTRIB_VALUE: &str = "taxon_function_abun";
/// The column only the long table of gene families' contributions has, which
/// tells it from the pathways' one.
const CONTRIB_GENES_ONLY: &str = "norm_taxon_function_contrib";

fn cells(line: &str) -> Vec<&str> {
    line.split('\t').map(str::trim).collect()
}

/// The kind of PICRUSt2 table a file starts like, or `None`.
pub fn table_kind(start: &str) -> Option<TableKind> {
    let lines = complete_lines(start.strip_prefix('\u{feff}').unwrap_or(start));
    let mut rows = lines.iter().filter(|l| !l.trim().is_empty());
    let header = cells(rows.next()?);
    if header.starts_with(&CONTRIB_START) && header.contains(&CONTRIB_VALUE) {
        return Some(TableKind::Contributions);
    }
    let first = header.first().copied()?;
    if first != "function" && first != "pathway" {
        return None;
    }
    let skip = match header.get(1).copied() {
        Some("sequence") => return (header.len() > 2).then_some(TableKind::Contributions),
        Some("description") => 2,
        _ => 1,
    };
    if header.len() <= skip {
        return None;
    }
    if first == "function" {
        return Some(TableKind::Functions);
    }
    // Coverage is a fraction; abundance is not, in any real table.
    let mut values = rows
        .flat_map(|row| cells(row).into_iter().skip(skip))
        .filter_map(|cell| cell.parse::<f64>().ok())
        .peekable();
    values.peek()?;
    if values.all(|v| (0.0..=1.0).contains(&v)) {
        Some(TableKind::PathCoverage)
    } else {
        Some(TableKind::PathAbundance)
    }
}

/// The heading of the level of functions, from the identifiers PICRUSt2
/// gives them (`1.1.1.1`, `ko:K00001`, `COG0001`, `PF00001`, `TIGR00001`).
fn function_heading<'a>(ids: impl Iterator<Item = &'a str>) -> &'static str {
    let mut heading = None;
    for id in ids {
        let id = id.trim();
        let this = if id.starts_with("ko:") || is_kegg_ortholog(id) {
            "KO"
        } else if id.starts_with("COG") {
            "COG"
        } else if id.starts_with("PF") {
            "Pfam"
        } else if id.starts_with("TIGR") {
            "TIGRFAM"
        } else if id.starts_with("EC:") || is_ec_number(id) {
            "EC"
        } else {
            return "Function";
        };
        match heading {
            Some(h) if h != this => return "Function",
            _ => heading = Some(this),
        }
    }
    heading.unwrap_or("Function")
}

fn is_kegg_ortholog(id: &str) -> bool {
    id.len() == 6 && id.starts_with('K') && id[1..].bytes().all(|b| b.is_ascii_digit())
}

fn is_ec_number(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

/// A table read from a file.
struct Table {
    /// Pathways rather than gene functions (for contributions).
    pathways: bool,
    samples: Vec<String>,
    /// Function (with its description), sequence (contributions only),
    /// sample index and value.
    values: Vec<(String, Option<String>, usize, f64)>,
    /// The identifiers of the functions, without descriptions.
    ids: Vec<String>,
}

fn not_a(file: &NamedText<'_>, kind: TableKind) -> StampError {
    StampError::new(format!(
        "{} is not a PICRUSt2 {} table.",
        file.name,
        kind_name(kind)
    ))
}

fn kind_name(kind: TableKind) -> &'static str {
    match kind {
        TableKind::Functions => "function",
        TableKind::PathAbundance => "pathway abundance",
        TableKind::PathCoverage => "pathway coverage",
        TableKind::Contributions => "contribution",
    }
}

fn read_table(file: &NamedText<'_>, kind: TableKind) -> Result<Table> {
    let content = file.text.strip_prefix('\u{feff}').unwrap_or(file.text);
    let lines = text::lines(content);
    let header_at = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
    let header = cells(lines.get(header_at).copied().unwrap_or(""));
    let found = table_kind(&format!(
        "{}\n{}\n",
        lines.get(header_at).copied().unwrap_or(""),
        lines.get(header_at + 1).copied().unwrap_or("")
    ));
    // Coverage and abundance are told apart by the whole table below.
    let compatible = match (found, kind) {
        (Some(TableKind::PathAbundance | TableKind::PathCoverage), TableKind::PathAbundance)
        | (Some(TableKind::PathAbundance | TableKind::PathCoverage), TableKind::PathCoverage) => {
            true
        }
        (Some(found), kind) => found == kind,
        (None, _) => false,
    };
    if !compatible {
        return Err(not_a(file, kind));
    }
    let bad = |number: usize, what: &str| {
        StampError::new(format!("Line {} of {}: {what}.", number + 1, file.name))
    };
    let number = |text: &str, line: usize| -> Result<f64> {
        parse_value(text)
            .map(|(v, _)| v)
            .ok_or_else(|| bad(line, &format!("'{text}' is not a number")))
    };
    let mut table = Table {
        pathways: header[0] == "pathway",
        samples: Vec::new(),
        values: Vec::new(),
        ids: Vec::new(),
    };
    let rows = lines
        .iter()
        .enumerate()
        .skip(header_at + 1)
        .filter(|(_, l)| !l.trim().is_empty());
    if header.starts_with(&CONTRIB_START) {
        // The long table: one row per sample, function and sequence.
        table.pathways = !header.contains(&CONTRIB_GENES_ONLY);
        let column = header.iter().position(|h| *h == CONTRIB_VALUE).unwrap();
        for (line, row) in rows {
            let row = cells(row);
            if row.len() != header.len() {
                return Err(bad(line, "the number of columns differs from the header's"));
            }
            let sample = match table.samples.iter().position(|s| s == row[0]) {
                Some(i) => i,
                None => {
                    table.samples.push(row[0].to_string());
                    table.samples.len() - 1
                }
            };
            table.ids.push(row[1].to_string());
            table.values.push((
                row[1].to_string(),
                Some(row[2].to_string()),
                sample,
                number(row[column], line)?,
            ));
        }
        return Ok(table);
    }
    let stratified = header.get(1) == Some(&"sequence");
    let described = header.get(1) == Some(&"description");
    let first_sample = if stratified || described { 2 } else { 1 };
    table.samples = header[first_sample..]
        .iter()
        .map(|s| s.to_string())
        .collect();
    for (line, row) in rows {
        let row = cells(row);
        if row.len() != header.len() {
            return Err(bad(line, "the number of columns differs from the header's"));
        }
        if row[0].is_empty() {
            return Err(bad(line, "the function has no name"));
        }
        let name = match (described, row[1]) {
            (true, description) if !description.is_empty() => {
                format!("{}: {description}", row[0])
            }
            _ => row[0].to_string(),
        };
        table.ids.push(row[0].to_string());
        let sequence = stratified.then(|| row[1].to_string());
        for (sample, cell) in row[first_sample..].iter().enumerate() {
            table
                .values
                .push((name.clone(), sequence.clone(), sample, number(cell, line)?));
        }
    }
    Ok(table)
}

/// Combine PICRUSt2 tables of one kind into a STAMP profile.
pub fn to_spf(files: &[NamedText<'_>], kind: TableKind) -> Result<String> {
    let tables = files
        .iter()
        .map(|f| read_table(f, kind))
        .collect::<Result<Vec<Table>>>()?;
    let pathways = tables.iter().any(|t| t.pathways);
    if tables.iter().any(|t| t.pathways != pathways) {
        return Err(StampError::new(
            "The files mix contributions to pathways with contributions to gene functions; \
             load one kind at a time.",
        ));
    }
    check_unique(tables.iter().map(|t| t.samples.as_slice()))?;
    let heading = match kind {
        TableKind::PathAbundance | TableKind::PathCoverage => "Pathway",
        TableKind::Contributions if pathways => "Pathway",
        TableKind::Functions | TableKind::Contributions => {
            let headings: Vec<&'static str> = tables
                .iter()
                .map(|t| function_heading(t.ids.iter().map(String::as_str)))
                .collect();
            match headings.as_slice() {
                [first, rest @ ..] if rest.iter().all(|h| h == first) => *first,
                _ => {
                    return Err(StampError::new(
                        "The files hold different kinds of function (EC numbers and KEGG \
                         orthologs, say); load one kind at a time.",
                    ))
                }
            }
        }
    };
    let ranks = if kind == TableKind::Contributions {
        Ranks::new(&[heading, "Sequence"], &["", ""])
    } else {
        Ranks::single(heading)
    };
    let mut lineages = Lineages::with_ranks(ranks);
    for table in &tables {
        let indexes: Vec<usize> = table.samples.iter().map(|s| lineages.sample(s)).collect();
        for (function, sequence, sample, value) in &table.values {
            if *value <= 0.0 {
                continue;
            }
            let mut lineage = vec![function.as_str()];
            lineage.extend(sequence.as_deref());
            lineages.add(indexes[*sample], &lineage, *value)?;
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

    const EC: &str = "function\tA\tB\n\
        1.1.1.1\t127.0\t555.6700000000001\n\
        1.1.1.100\t3956.0\t0.0\n";

    #[test]
    fn functions_become_one_level_named_after_their_kind() {
        assert_eq!(table_kind(EC), Some(TableKind::Functions));
        assert_eq!(
            spf(TableKind::Functions, &[("pred_metagenome_unstrat.tsv", EC)]).unwrap(),
            "EC\tA\tB\n1.1.1.1\t127.0\t555.67\n1.1.1.100\t3956.0\t0\n"
        );
        let ko = "function\tA\tB\nko:K00001\t1\t2\nko:K00003\t3\t4\n";
        assert!(spf(TableKind::Functions, &[("ko.tsv", ko)])
            .unwrap()
            .starts_with("KO\tA\tB\n"));
        // Tables of different kinds of function are not combined.
        let other = "function\tC\nko:K00001\t1\n";
        let error = spf(TableKind::Functions, &[("ec.tsv", EC), ("ko.tsv", other)]).unwrap_err();
        assert!(error.0.contains("different kinds of function"), "{error}");
        // Several tables of one kind are combined, sample by sample.
        let more = "function\tC\n1.1.1.1\t2\n";
        let both = spf(TableKind::Functions, &[("a.tsv", EC), ("b.tsv", more)]).unwrap();
        assert!(both.starts_with("EC\tA\tB\tC\n"), "{both}");
        crate::ProfileTree::read(&both).unwrap();
    }

    #[test]
    fn descriptions_are_kept_in_the_names() {
        let described = "pathway\tdescription\tA\tB\n\
            1CMET2-PWY\tfolate transformations III (E. coli)\t3493.3\t4170.9\n\
            ARO-PWY\tchorismate biosynthesis I\t12.5\t0\n";
        assert_eq!(table_kind(described), Some(TableKind::PathAbundance));
        // The kind of function is judged from the identifiers alone.
        let ec = "function\tdescription\tA\n1.1.1.1\tAlcohol dehydrogenase\t127.0\n";
        assert_eq!(
            spf(TableKind::Functions, &[("ec.tsv", ec)]).unwrap(),
            "EC\tA\n1.1.1.1: Alcohol dehydrogenase\t127.0\n"
        );
        assert_eq!(
            spf(
                TableKind::PathAbundance,
                &[("path_abun_unstrat_descrip.tsv", described)]
            )
            .unwrap(),
            "Pathway\tA\tB\n\
             1CMET2-PWY: folate transformations III (E. coli)\t3493.3\t4170.9\n\
             ARO-PWY: chorismate biosynthesis I\t12.5\t0\n"
        );
    }

    #[test]
    fn coverage_is_told_from_abundance_by_its_values() {
        let coverage = "pathway\tA\tB\n1CMET2-PWY\t0.376\t0.265\nARO-PWY\t1.0\t0.0\n";
        assert_eq!(table_kind(coverage), Some(TableKind::PathCoverage));
        let abundance = "pathway\tA\tB\n1CMET2-PWY\t0.376\t3.5\n";
        assert_eq!(table_kind(abundance), Some(TableKind::PathAbundance));
        // A pathway table with no rows yet is neither.
        assert_eq!(table_kind("pathway\tA\tB\n"), None);
        assert!(spf(
            TableKind::PathCoverage,
            &[("path_cov_unstrat.tsv", coverage)]
        )
        .unwrap()
        .starts_with("Pathway\tA\tB\n"));
    }

    #[test]
    fn long_contribution_tables_give_function_and_sequence() {
        let genes = "sample\tfunction\ttaxon\ttaxon_abun\ttaxon_rel_abun\tgenome_function_count\t\
            taxon_function_abun\ttaxon_rel_function_abun\tnorm_taxon_function_contrib\n\
            A\t2.1.1.190\tasv1\t26.0\t0.75\t2\t52.0\t1.5\t0.0135\n\
            A\t2.1.1.190\tasv2\t108.0\t3.1\t1\t108.0\t3.1\t0.028\n\
            B\t2.1.1.190\tasv1\t10.0\t0.5\t2\t20.0\t1.0\t0.01\n";
        assert_eq!(table_kind(genes), Some(TableKind::Contributions));
        assert_eq!(
            spf(
                TableKind::Contributions,
                &[("pred_metagenome_contrib.tsv", genes)]
            )
            .unwrap(),
            "EC\tSequence\tA\tB\n\
             2.1.1.190\tasv1\t52.0\t20.0\n\
             2.1.1.190\tasv2\t108.0\t0\n"
        );
        // Pathways' contributions have no norm_taxon_function_contrib.
        let pathways = "sample\tfunction\ttaxon\ttaxon_abun\ttaxon_rel_abun\t\
            genome_function_count\ttaxon_function_abun\ttaxon_rel_function_abun\n\
            A\t1CMET2-PWY\tasv1\t26.0\t0.75\t0.99\t25.87\t0.74\n\
            B\t1CMET2-PWY\tasv1\t10.0\t0.5\t0.99\t9.95\t0.49\n";
        let text = spf(
            TableKind::Contributions,
            &[("path_abun_contrib.tsv", pathways)],
        )
        .unwrap();
        assert!(text.starts_with("Pathway\tSequence\tA\tB\n"), "{text}");
        crate::ProfileTree::read(&text).unwrap();
        let error = spf(
            TableKind::Contributions,
            &[("g.tsv", genes), ("p.tsv", pathways)],
        )
        .unwrap_err();
        assert!(error.0.contains("mix contributions"), "{error}");
    }

    #[test]
    fn wide_contribution_tables_give_function_and_sequence() {
        let wide = "pathway\tsequence\tA\tB\n\
            1CMET2-PWY\tasv1\t25.87\t9.95\n\
            1CMET2-PWY\tasv2\t0\t4.5\n";
        assert_eq!(table_kind(wide), Some(TableKind::Contributions));
        assert_eq!(
            spf(TableKind::Contributions, &[("path_abun_strat.tsv", wide)]).unwrap(),
            "Pathway\tSequence\tA\tB\n\
             1CMET2-PWY\tasv1\t25.87\t9.95\n\
             1CMET2-PWY\tasv2\t0\t4.5\n"
        );
    }

    #[test]
    fn other_tables_are_not_picrust2() {
        // PICRUSt2's other outputs are not profiles of functions.
        for text in [
            "normalized\tA\tB\nasv1\t26.0\t0.0\n",
            "sample\tweighted_NSTI\nA\t0.05\n",
            "sequence\t16S_rRNA_Count\tmetadata_NSTI\nasv1\t1\t0.0014\n",
            "sequence\t1.1.1.1\t1.1.1.100\nasv1\t0\t2\n",
            "Function\tA\tB\nx\t1\t2\n",
            "function\n",
        ] {
            assert_eq!(table_kind(text), None, "{text}");
        }
        let error = spf(TableKind::Functions, &[("x.tsv", "pathway\tA\nPWY\t2\n")]).unwrap_err();
        assert!(
            error.0.contains("is not a PICRUSt2 function table"),
            "{error}"
        );
        let error = spf(
            TableKind::Functions,
            &[("x.tsv", "function\tA\n1.1.1.1\tmany\n")],
        )
        .unwrap_err();
        assert!(error.0.contains("'many' is not a number"), "{error}");
    }
}
