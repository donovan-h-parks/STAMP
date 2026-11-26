//! Kraken 2 (and Kraken 1) reports, Kraken's MPA-style reports, and
//! Bracken's reports and abundance tables. See `docs/PROFILER_FORMATS.md`.
//!
//! A report (`kraken2 --report`) has no header and one line per taxon:
//! percentage, clade reads, direct reads, (with `--report-minimizer-data`
//! two minimizer counts,) rank code, NCBI taxid and the name, indented two
//! spaces per depth. The direct reads are the values; ranks D, P, C, O, F,
//! G and S are levels, and S1 (Strain) when reads were assigned below a
//! species. Bracken writes reports of the same layout (`-w`), which are told
//! apart so that a directory holding both kinds is not counted twice.

use super::lineage::{add_clades, parse_value, rank_prefix, split_lineage, Clade, Lineages, Ranks};
use super::{check_unique, complete_lines, sample_name_of, NamedText};
use crate::error::{Result, StampError};
use crate::text;

/// Which tool wrote a report in Kraken's layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Kraken,
    Bracken,
}

/// One line of a report.
struct Line<'a> {
    padded: bool,
    clade: f64,
    direct: f64,
    rank: &'a str,
    depth: usize,
    name: &'a str,
}

fn parse_line(line: &str) -> Option<Line<'_>> {
    let fields: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
    let n = fields.len();
    if !(6..=8).contains(&n) {
        return None;
    }
    fields[0].trim().parse::<f64>().ok()?;
    let whole = |s: &str| parse_value(s).filter(|v| v.1 == 0.0).map(|v| v.0);
    let clade = whole(fields[1])?;
    let direct = whole(fields[2])?;
    let rank = fields[n - 3].trim();
    let mut chars = rank.chars();
    let rank_ok = rank == "-"
        || (chars.next().is_some_and(|c| "URDKPCOFGS".contains(c))
            && chars.all(|c| c.is_ascii_digit()));
    if !rank_ok || !fields[n - 2].trim().chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let raw = fields[n - 1];
    let name = raw.trim();
    if name.is_empty() {
        return None;
    }
    Some(Line {
        padded: fields[0].starts_with(' '),
        clade,
        direct,
        rank,
        depth: (raw.len() - raw.trim_start_matches(' ').len()) / 2,
        name,
    })
}

/// Which tool wrote a file in the layout of a Kraken report, judged from its
/// start, or `None` if it is not one. Kraken pads its percentages (`%6.2f`)
/// and gives reads to taxa that have taxa below them; Bracken does neither.
pub fn report_kind(start: &str) -> Option<ReportKind> {
    let lines: Vec<Line<'_>> = complete_lines(start)
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .take(500)
        .map(parse_line)
        .collect::<Option<Vec<_>>>()?;
    if lines.is_empty() {
        return None;
    }
    let padded = lines.iter().any(|l| l.padded);
    let internal_reads = lines
        .windows(2)
        .any(|w| w[1].depth > w[0].depth && w[0].direct > 0.0 && w[0].rank != "U");
    Some(if padded || internal_reads {
        ReportKind::Kraken
    } else {
        ReportKind::Bracken
    })
}

/// The ranks of Kraken profiles, with Strain when it is used.
fn kraken_ranks(strain: bool) -> Ranks {
    let ranks = Ranks::gtdb();
    if strain {
        ranks.with("Strain", "t__")
    } else {
        ranks
    }
}

/// The level of a standard rank code.
fn standard_rank(code: &str) -> Option<usize> {
    match code {
        "D" => Some(0),
        "P" => Some(1),
        "C" => Some(2),
        "O" => Some(3),
        "F" => Some(4),
        "G" => Some(5),
        "S" => Some(6),
        _ => None,
    }
}

/// A lineage as `(level, name)` pairs from the top.
type Lineage = Vec<(usize, String)>;

/// The direct reads of one report, each with its lineage.
type Assigned = Vec<(Vec<(usize, String)>, f64)>;

/// The reads of each taxon of one report, with its lineage. In a Kraken
/// report they are the direct reads. In a report of Bracken they are the
/// clade reads less those of the taxa printed below: Bracken drops the taxa
/// below the level it estimated but, in some versions, leaves the taxa of
/// that level with no direct reads (a genus-level report of Bracken 3.1
/// gives every genus that had species 0).
fn read_report(file: &NamedText<'_>) -> Result<Assigned> {
    let bracken = report_kind(file.text) == Some(ReportKind::Bracken);
    // Each line with its lineage and the line it is below.
    let mut taxa: Vec<(Line<'_>, Lineage, Option<usize>)> = Vec::new();
    // The taxa above the current line: depth, rank code, name and index.
    let mut stack: Vec<(usize, &str, &str, usize)> = Vec::new();
    for (number, line) in text::lines(file.text).into_iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let line = parse_line(line).ok_or_else(|| {
            StampError::new(format!(
                "Line {} of {} is not a line of a Kraken report.",
                number + 1,
                file.name
            ))
        })?;
        if line.rank == "U" {
            taxa.push((line, Vec::new(), None));
            continue;
        }
        while stack
            .last()
            .is_some_and(|&(depth, _, _, _)| depth >= line.depth)
        {
            stack.pop();
        }
        let parent = stack.last().map(|&(_, _, _, index)| index);
        stack.push((line.depth, line.rank, line.name, taxa.len()));
        let path: Vec<(usize, &str, &str)> = stack.iter().map(|&(d, r, n, _)| (d, r, n)).collect();
        taxa.push((line, lineage_of(&path), parent));
    }
    let mut reads: Vec<f64> = taxa
        .iter()
        .map(|(line, _, _)| if bracken { line.clade } else { line.direct })
        .collect();
    if bracken {
        for (line, _, parent) in &taxa {
            if let Some(parent) = parent {
                reads[*parent] -= line.clade;
            }
        }
    }
    Ok(taxa
        .into_iter()
        .zip(reads)
        .filter(|(_, reads)| *reads > 0.0)
        .map(|((_, lineage, _), reads)| (lineage, reads))
        .collect())
}

/// The lineage of the last taxon of `stack`: its taxa of standard ranks,
/// the first taxon below a species as its strain, and a domain taken from
/// the top unranked taxon when it has none (viruses under `Viruses`).
fn lineage_of(stack: &[(usize, &str, &str)]) -> Vec<(usize, String)> {
    let mut lineage: Vec<(usize, String)> = Vec::new();
    for &(_, rank, name) in stack {
        if let Some(level) = standard_rank(rank) {
            lineage.push((level, name.to_string()));
        } else if (rank.starts_with('S') || rank == "-")
            && lineage.last().is_some_and(|&(level, _)| level == 6)
        {
            lineage.push((7, name.to_string()));
        }
    }
    let has_domain = lineage.first().is_some_and(|&(level, _)| level == 0);
    // Below root (depth 0) and not "cellular organisms".
    let top = stack
        .iter()
        .find(|&&(depth, _, name)| depth > 0 && name != "cellular organisms")
        .filter(|&&(_, rank, _)| standard_rank(rank).is_none());
    if !has_domain {
        if let Some(&(_, _, name)) = top {
            lineage.insert(0, (0, name.to_string()));
        }
    }
    lineage
}

/// Combine Kraken (or Bracken) reports, one sample per file, into a STAMP
/// profile of direct read counts.
pub fn reports_to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let samples: Vec<String> = files.iter().map(|f| sample_name_of(f.name)).collect();
    check_unique(std::iter::once(samples.as_slice()))?;
    let reports = files
        .iter()
        .map(read_report)
        .collect::<Result<Vec<Assigned>>>()?;
    let strain = reports
        .iter()
        .flatten()
        .any(|(lineage, _)| lineage.iter().any(|&(level, _)| level == 7));
    let mut lineages = Lineages::with_ranks(kraken_ranks(strain));
    lineages.set_counts(true);
    for (sample, report) in samples.iter().zip(&reports) {
        let index = lineages.sample(sample);
        for (lineage, reads) in report {
            let lineage: Vec<(usize, &str)> =
                lineage.iter().map(|(l, n)| (*l, n.as_str())).collect();
            lineages.add_ranked(index, &lineage, *reads)?;
        }
    }
    lineages.to_spf()
}

/// True if the start of a file looks like an MPA-style report of Kraken
/// (`--use-mpa-style`, KrakenTools `kreport2mpa.py` or `combine_mpa.py`):
/// rows of `d__...|p__...` lineages and read counts.
pub fn looks_like_mpa(start: &str) -> bool {
    let lines = complete_lines(start);
    let rows: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .take(50)
        .collect();
    let Some(first) = rows.first() else {
        return false;
    };
    first.starts_with("d__")
        && rows.iter().all(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            cells.len() >= 2
                && cells[0].split('|').all(|part| rank_prefix(part).is_some())
                && cells[1..]
                    .iter()
                    .all(|c| parse_value(c).is_some_and(|v| v.1 == 0.0))
        })
}

/// Combine MPA-style reports of Kraken into a STAMP profile.
pub fn mpa_to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let mut tables = Vec::new();
    for file in files {
        let lines = text::lines(file.text);
        // `#Classification\t<sample>...` names the samples.
        let header = lines
            .iter()
            .find(|l| l.starts_with("#Classification"))
            .map(|l| {
                l.split('\t')
                    .skip(1)
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>()
            });
        let mut clades = Vec::new();
        let mut columns = header.as_ref().map_or(0, Vec::len);
        for (number, line) in lines.iter().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let bad = |what: &str| {
                StampError::new(format!("Line {} of {}: {what}.", number + 1, file.name))
            };
            let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
            if columns == 0 {
                columns = cells.len() - 1;
            }
            if cells.len() != columns + 1 {
                return Err(bad("the number of columns differs from the other lines"));
            }
            let values = cells[1..]
                .iter()
                .map(|c| parse_value(c).ok_or_else(|| bad(&format!("'{c}' is not a number"))))
                .collect::<Result<Vec<_>>>()?;
            clades.push(Clade {
                path: split_lineage(cells[0]),
                values,
            });
        }
        let samples = match header {
            Some(names) if names.len() == columns && names.iter().all(|n| !n.is_empty()) => {
                names.iter().map(|n| sample_name_of(n)).collect()
            }
            _ if columns == 1 => vec![sample_name_of(file.name)],
            _ => {
                return Err(StampError::new(format!(
                    "{} has {columns} columns of counts but no #Classification line naming \
                     their samples.",
                    file.name
                )))
            }
        };
        tables.push((samples, clades));
    }
    check_unique(tables.iter().map(|(s, _)| s.as_slice()))?;
    let mut lineages = Lineages::with_ranks(kraken_ranks(false));
    lineages.set_counts(true);
    for (samples, clades) in &tables {
        let indexes: Vec<usize> = samples.iter().map(|s| lineages.sample(s)).collect();
        add_clades(&mut lineages, &indexes, clades, |letter| {
            "dpcofgs".find(letter)
        })?;
    }
    lineages.to_spf()
}

/// The first columns of a Bracken abundance table.
const BRACKEN_COLUMNS: &str = "name\ttaxonomy_id\ttaxonomy_lvl\t";

/// True if the start of a file looks like a Bracken abundance table
/// (`.bracken`, or `combine_bracken_outputs.py` output).
pub fn looks_like_bracken_table(start: &str) -> bool {
    complete_lines(start)
        .iter()
        .find(|l| !l.trim().is_empty())
        .is_some_and(|l| l.starts_with(BRACKEN_COLUMNS))
}

/// The heading of a Bracken level code.
fn level_heading(code: &str) -> String {
    match code {
        "D" => "Domain".into(),
        "K" => "Kingdom".into(),
        "P" => "Phylum".into(),
        "C" => "Class".into(),
        "O" => "Order".into(),
        "F" => "Family".into(),
        "G" => "Genus".into(),
        "S" => "Species".into(),
        "S1" => "Strain".into(),
        other => format!("Rank {other}"),
    }
}

/// Combine Bracken abundance tables into a STAMP profile of one level, the
/// level Bracken estimated, with the estimated reads (`new_est_reads`).
pub fn bracken_tables_to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let mut level: Option<String> = None;
    let mut tables = Vec::new();
    for file in files {
        let lines = text::lines(file.text.strip_prefix('\u{feff}').unwrap_or(file.text));
        let header: Vec<&str> = lines
            .first()
            .map_or(Vec::new(), |l| l.split('\t').map(str::trim).collect());
        if !lines
            .first()
            .is_some_and(|l| l.starts_with(BRACKEN_COLUMNS))
        {
            return Err(StampError::new(format!(
                "{} does not appear to be a Bracken abundance table.",
                file.name
            )));
        }
        let (columns, samples): (Vec<usize>, Vec<String>) =
            match header.iter().position(|c| *c == "new_est_reads") {
                Some(i) => (vec![i], vec![sample_name_of(file.name)]),
                None => header
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.strip_suffix("_num").map(|s| (i, sample_name_of(s))))
                    .unzip(),
            };
        if columns.is_empty() {
            return Err(StampError::new(format!(
                "{} has no new_est_reads column, nor <sample>_num columns.",
                file.name
            )));
        }
        let mut rows = Vec::new();
        for (number, line) in lines.iter().enumerate().skip(1) {
            if line.trim().is_empty() {
                continue;
            }
            let bad = |what: &str| {
                StampError::new(format!("Line {} of {}: {what}.", number + 1, file.name))
            };
            let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
            if cells.len() < header.len() {
                return Err(bad("too few columns"));
            }
            match &level {
                Some(level) if level != cells[2] => {
                    return Err(bad(&format!(
                        "taxa of level {} and of level {level} cannot be combined; Bracken \
                         estimates one level at a time",
                        cells[2]
                    )))
                }
                _ => level = Some(cells[2].to_string()),
            }
            let values = columns
                .iter()
                .map(|&i| {
                    parse_value(cells[i])
                        .map(|v| v.0)
                        .ok_or_else(|| bad(&format!("'{}' is not a number", cells[i])))
                })
                .collect::<Result<Vec<f64>>>()?;
            rows.push((cells[0].to_string(), values));
        }
        tables.push((samples, rows));
    }
    check_unique(tables.iter().map(|(s, _)| s.as_slice()))?;
    let heading = level_heading(level.as_deref().unwrap_or("S"));
    let mut lineages = Lineages::with_ranks(Ranks::single(&heading));
    lineages.set_counts(true);
    for (samples, rows) in &tables {
        let indexes: Vec<usize> = samples.iter().map(|s| lineages.sample(s)).collect();
        for (name, values) in rows {
            for (column, &value) in values.iter().enumerate() {
                if value > 0.0 {
                    lineages.add(indexes[column], &[name], value)?;
                }
            }
        }
    }
    lineages.to_spf()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named<'a>(files: &[(&'a str, &'a str)]) -> Vec<NamedText<'a>> {
        files
            .iter()
            .map(|&(name, text)| NamedText { name, text })
            .collect()
    }

    /// A Kraken 2 report: unclassified reads, reads at root, a strain, a
    /// genus with no family above it, and viruses with no domain.
    const REPORT: &str = " 10.00\t100\t100\tU\t0\tunclassified\n\
         90.00\t900\t5\tR\t1\troot\n\
         85.00\t850\t10\tR1\t131567\t  cellular organisms\n\
         85.00\t850\t20\tD\t2\t    Bacteria\n\
         60.00\t600\t0\tP\t1224\t      Pseudomonadota\n\
         60.00\t600\t0\tC\t1236\t        Gammaproteobacteria\n\
         60.00\t600\t0\tO\t91347\t          Enterobacterales\n\
         60.00\t600\t30\tF\t543\t            Enterobacteriaceae\n\
         57.00\t570\t70\tG\t561\t              Escherichia\n\
         50.00\t500\t400\tS\t562\t                Escherichia coli\n\
         10.00\t100\t60\tS1\t83333\t                  Escherichia coli K-12\n\
          4.00\t40\t40\tS2\t511145\t                    Escherichia coli str. K-12 substr. MG1655\n\
          23.00\t230\t0\tP\t1239\t      Bacillota\n\
          23.00\t230\t0\tC\t91061\t        Bacilli\n\
          23.00\t230\t30\tO\t1385\t          Bacillales\n\
          20.00\t200\t200\tG\t1386\t            Bacillus\n\
          4.50\t45\t0\tR1\t10239\t  Viruses\n\
          4.50\t45\t45\tS\t10710\t    Escherichia phage Lambda\n";

    #[test]
    fn reports_give_direct_reads_by_rank() {
        assert_eq!(report_kind(REPORT), Some(ReportKind::Kraken));
        let other = REPORT.replace("\t400\tS\t", "\t300\tS\t");
        let spf = reports_to_spf(&named(&[("a.k2report", REPORT), ("b.kreport", &other)])).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert_eq!(
            lines[0],
            "Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tStrain\ta\tb"
        );
        // Unclassified reads and reads at root and cellular organisms.
        assert_eq!(lines[1], ["unclassified Root"; 8].join("\t") + "\t115\t115");
        let row = |end: &str| {
            lines
                .iter()
                .find(|l| l.contains(end))
                .unwrap_or_else(|| panic!("no row with {end} in\n{spf}"))
                .to_string()
        };
        assert!(row("\tEscherichia coli\tunclassified s__").ends_with("\t400\t300"));
        // S2 reads are counted with their S1 strain.
        assert!(row("\tEscherichia coli K-12\t").ends_with("\t100\t100"));
        // Bacillus has no family in this taxonomy.
        assert!(row("\tBacillales\tBacillales (no family)\tBacillus\t").ends_with("\t200\t200"));
        // Viruses with no domain above them take the top unranked taxon.
        assert!(row("Viruses\tViruses (no phylum)\t").contains("\tEscherichia phage Lambda\t"));
        crate::ProfileTree::read(&spf).unwrap();
    }

    #[test]
    fn minimizer_columns_and_kraken_1_reports() {
        // --report-minimizer-data: two more columns before the rank.
        let report = " 20.00\t20\t20\tU\t0\tunclassified\n\
             80.00\t80\t0\tR\t1\troot\n\
             80.00\t80\t80\t1688\t18\tD\t2\t  Bacteria\n";
        assert_eq!(report_kind(report), Some(ReportKind::Kraken));
        let spf = reports_to_spf(&named(&[("x.txt", report), ("y.txt", report)])).unwrap();
        assert!(spf.contains("Bacteria\tunclassified d__Bacteria"));
        // Kraken 1 writes - for unranked taxa.
        let kraken1 = "  1.60\t16\t16\tU\t0\tunclassified\n\
             98.40\t984\t0\t-\t1\troot\n\
             98.40\t984\t18\t-\t131567\t  cellular organisms\n\
             96.60\t966\t6\tD\t2\t    Bacteria\n\
             96.00\t960\t900\tS\t562\t      Escherichia coli\n\
              6.00\t60\t60\t-\t83333\t        Escherichia coli K-12\n";
        let spf = reports_to_spf(&named(&[("a", kraken1), ("b", kraken1)])).unwrap();
        assert!(spf.contains("unclassified Root\t34\t34\n"));
        assert!(spf.contains("\tEscherichia coli\tEscherichia coli K-12\t60\t60\n"));
        assert!(spf.contains("Bacteria (no phylum)"));
    }

    #[test]
    fn bracken_reports_are_told_apart() {
        let bracken = "100.00\t1000\t0\tR\t1\troot\n\
            100.00\t1000\t0\tR1\t131567\t  cellular organisms\n\
            100.00\t1000\t0\tD\t2\t    Bacteria\n\
            60.00\t600\t600\tS\t562\t      Escherichia coli\n\
            40.00\t400\t400\tS\t1351\t      Enterococcus faecalis\n";
        assert_eq!(report_kind(bracken), Some(ReportKind::Bracken));
        let spf = reports_to_spf(&named(&[
            ("gut_bracken_species.kreport", bracken),
            ("x", bracken),
        ]))
        .unwrap();
        assert!(spf.starts_with("Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tgut\tx\n"));
    }

    #[test]
    fn bracken_reports_are_read_by_clade() {
        // Bracken 3.1 at genus level: genera that had species keep no
        // direct reads, though the species are dropped.
        let genus = "100.00\t300\t0\tR\t1\troot\n\
            100.00\t300\t0\tD\t2\t  Bacteria\n\
            100.00\t300\t0\tF\t80864\t    Comamonadaceae\n\
            50.00\t150\t0\tG\t34072\t      Variovorax\n\
            40.00\t120\t0\tG\t52972\t      Polaromonas\n\
            3.33\t10\t10\tG\t665874\t      Limnohabitans\n";
        assert_eq!(report_kind(genus), Some(ReportKind::Bracken));
        let spf = reports_to_spf(&named(&[("a", genus), ("b", genus)])).unwrap();
        assert!(
            spf.contains("\tVariovorax\tunclassified g__Variovorax\t150\t150\n"),
            "{spf}"
        );
        assert!(spf.contains("\tLimnohabitans\tunclassified g__Limnohabitans\t10\t10\n"));
        // The 20 reads Bracken left at the family.
        assert!(
            spf.contains("\tunclassified f__Comamonadaceae\t20\t20\n"),
            "{spf}"
        );
    }

    #[test]
    fn mpa_style_reports() {
        let mpa = "#Classification\tA.k2report\tB\n\
            d__Bacteria\t900\t500\n\
            d__Bacteria|p__Pseudomonadota\t600\t500\n\
            d__Bacteria|p__Pseudomonadota|c__Gammaproteobacteria\t600\t500\n\
            d__Eukaryota\t10\t0\n\
            d__Eukaryota|k__Metazoa\t10\t0\n\
            d__Eukaryota|k__Metazoa|p__Chordata\t10\t0\n";
        assert!(looks_like_mpa(mpa));
        let spf = mpa_to_spf(&named(&[("combined.txt", mpa)])).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert!(lines[0].ends_with("\tSpecies\tA\tB"));
        assert!(lines[1].starts_with("Bacteria\tunclassified d__Bacteria\t"));
        assert!(lines[1].ends_with("\t300\t0"));
        // Kingdoms are not a level.
        assert!(spf.contains("Eukaryota\tChordata\tunclassified p__Chordata"));
        // Without a header, one column of counts per file.
        let single = "d__Bacteria\t10\nd__Bacteria|p__Bacillota\t10\n";
        assert!(
            mpa_to_spf(&named(&[("s1.mpa", single), ("s2.mpa", single)]))
                .unwrap()
                .contains("\ts1\ts2\n")
        );
        assert!(!looks_like_mpa("k__Bacteria\t10\n"));
    }

    #[test]
    fn bracken_tables() {
        let table = "name\ttaxonomy_id\ttaxonomy_lvl\tkraken_assigned_reads\tadded_reads\t\
            new_est_reads\tfraction_total_reads\n\
            Escherichia coli\t562\tS\t500\t50\t550\t0.55000\n\
            Bacillus subtilis\t1423\tS\t400\t50\t450\t0.45000\n";
        assert!(looks_like_bracken_table(table));
        let combined = "name\ttaxonomy_id\ttaxonomy_lvl\tc.bracken_num\tc.bracken_frac\t\
            d_num\td_frac\n\
            Escherichia coli\t562\tS\t10\t1.00000\t0\t0.00000\n";
        let spf =
            bracken_tables_to_spf(&named(&[("a.bracken", table), ("all.txt", combined)])).unwrap();
        assert_eq!(
            spf,
            "Species\ta\tc\td\nEscherichia coli\t550\t10\t0\nBacillus subtilis\t450\t0\t0\n"
        );
        let genus = table.replace("\tS\t", "\tG\t");
        let error = bracken_tables_to_spf(&named(&[("a", table), ("b", &genus)])).unwrap_err();
        assert!(
            error.0.contains("Bracken estimates one level at a time"),
            "{error}"
        );
    }

    #[test]
    fn other_files_are_not_reports() {
        for other in [
            "sample\tcoverage\ttaxonomy\nA\t1\tRoot\n",
            "Domain\tPhylum\tA\tB\nBacteria\tFirmicutes\t1\t2\n",
            "a\tb\tc\td\te\tf\n",
            "",
        ] {
            assert_eq!(report_kind(other), None, "{other}");
            assert!(!looks_like_mpa(other));
            assert!(!looks_like_bracken_table(other));
        }
        // The start of a large report may end in the middle of a line.
        assert_eq!(
            report_kind(&REPORT[..REPORT.len() - 9]),
            Some(ReportKind::Kraken)
        );
    }
}
