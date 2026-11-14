//! SingleM condensed taxonomic profiles, as written by `singlem pipe -p` and
//! `singlem summarise`: tab-separated columns `sample`, `coverage` and
//! `taxonomy` (`Root; d__Bacteria; p__Bacillota; ...`), any number of
//! samples in one file. The variant written with
//! `--output-taxonomic-profile-with-extras` has further columns, which are
//! ignored. Coverage is a node's own, not including its descendants; see
//! [`lineage`](super::lineage) for how it becomes a profile.

use super::lineage::{strip_rank_prefix, Lineages};
use super::NamedText;
use crate::error::{Result, StampError};
use crate::text;

/// Positions of the sample, coverage and taxonomy columns in a header line.
fn columns(header: &str) -> Option<(usize, usize, usize)> {
    let cells: Vec<&str> = header.split('\t').map(str::trim).collect();
    let find = |name: &str| cells.iter().position(|c| *c == name);
    Some((find("sample")?, find("coverage")?, find("taxonomy")?))
}

/// True if a text looks like a SingleM condensed profile: its first line
/// has the columns `sample`, `coverage` and `taxonomy`.
pub fn looks_like_singlem(text: &str) -> bool {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.lines()
        .find(|l| !l.trim().is_empty())
        .is_some_and(|header| columns(header).is_some())
}

/// Combine SingleM condensed profiles (each with one or more samples) into a
/// STAMP profile of the seven ranks from domain to species.
pub fn to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let mut lineages = Lineages::new();
    for file in files {
        let lines = text::lines(file.text);
        let mut rows = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());
        let header = rows.next().map(|(_, l)| *l).unwrap_or("");
        let Some((sample_col, coverage_col, taxonomy_col)) = columns(header) else {
            return Err(StampError::new(format!(
                "{} does not appear to be a SingleM condensed profile: its first line \
                 must name the columns sample, coverage and taxonomy.",
                file.name
            )));
        };
        // Samples are those of the file; a name that another file already
        // used is an error rather than being silently merged.
        let mut own: Vec<String> = Vec::new();
        for (number, line) in rows {
            let cells: Vec<&str> = line.split('\t').collect();
            let bad = |what: &str| {
                StampError::new(format!("Line {} of {}: {what}.", number + 1, file.name))
            };
            let cell = |i: usize| cells.get(i).map(|c| c.trim());
            let (Some(sample), Some(coverage), Some(taxonomy)) =
                (cell(sample_col), cell(coverage_col), cell(taxonomy_col))
            else {
                return Err(bad("too few columns"));
            };
            if sample.is_empty() {
                return Err(bad("no sample name"));
            }
            let coverage = text::parse_float(coverage)
                .filter(|c| c.is_finite() && *c >= 0.0)
                .ok_or_else(|| bad("the coverage is not a number"))?;
            if !own.iter().any(|s| s == sample) {
                if lineages.has_sample(sample) {
                    return Err(StampError::new(format!(
                        "Sample '{sample}' is in more than one file."
                    )));
                }
                own.push(sample.to_string());
            }
            let index = lineages.sample(sample);

            let mut taxa: Vec<&str> = taxonomy.split(';').map(str::trim).collect();
            if taxa.first() == Some(&"Root") {
                taxa.remove(0);
            }
            // An empty name (`s__`) ends the lineage.
            let lineage: Vec<&str> = taxa
                .into_iter()
                .map(strip_rank_prefix)
                .take_while(|t| !t.is_empty())
                .collect();
            if coverage > 0.0 {
                lineages
                    .add(index, &lineage, coverage)
                    .map_err(|e| bad(e.0.trim_end_matches('.')))?;
            }
        }
    }
    lineages.to_spf()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = "sample\tcoverage\ttaxonomy\n\
        gut\t0.5\tRoot\n\
        gut\t3.64\tRoot; d__Bacteria\n\
        gut\t1.25\tRoot; d__Bacteria; p__Bacillota; c__Clostridia; o__Oscillospirales; \
        f__Oscillospiraceae; g__Limivicinus; s__Limivicinus sp030531505\n\
        soil\t0\tRoot; d__Archaea\n\
        soil\t2\tRoot; d__Bacteria; p__Bacillota; c__Clostridia; o__Oscillospirales; \
        f__Oscillospiraceae\n";

    fn one(name: &'static str, text: &'static str) -> [NamedText<'static>; 1] {
        [NamedText { name, text }]
    }

    #[test]
    fn builds_seven_ranks_from_a_condensed_profile() {
        assert!(looks_like_singlem(PROFILE));
        let spf = to_spf(&one("p.tsv", PROFILE)).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert_eq!(
            lines[0],
            "Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tgut\tsoil"
        );
        assert_eq!(lines[1], ["unclassified Root"; 7].join("\t") + "\t0.5\t0");
        assert_eq!(
            lines[2],
            format!(
                "Bacteria{}\t3.64\t0",
                "\tunclassified d__Bacteria".repeat(6)
            )
        );
        assert_eq!(
            lines[3],
            "Bacteria\tBacillota\tClostridia\tOscillospirales\tOscillospiraceae\t\
             Limivicinus\tLimivicinus sp030531505\t1.25\t0"
        );
        assert_eq!(
            lines[4],
            "Bacteria\tBacillota\tClostridia\tOscillospirales\tOscillospiraceae\t\
             unclassified f__Oscillospiraceae\tunclassified f__Oscillospiraceae\t0\t2.0"
        );
        // A taxon with no coverage of its own makes no row.
        assert_eq!(lines.len(), 5);
        let tree = crate::ProfileTree::read(&spf).unwrap();
        assert_eq!(tree.sample_total("gut"), Some(0.5 + 3.64 + 1.25));
    }

    #[test]
    fn reads_the_variant_with_extras() {
        let text = "sample\tcoverage\tfull_coverage\trelative_abundance\tlevel\ttaxonomy\n\
            a\t0.02\t2.99\t41.59\tdomain\tRoot; d__Bacteria\n\
            b\t1.5\t1.5\t100\tphylum\tRoot; d__Bacteria; p__Bacillota\n";
        let spf = to_spf(&one("x", text)).unwrap();
        assert!(spf.contains("\tunclassified d__Bacteria\t0.02\t0\n"));
        assert!(spf.contains("\tunclassified p__Bacillota\t0\t1.5\n"));
    }

    #[test]
    fn combines_files_and_reports_problems() {
        let a = NamedText {
            name: "a.tsv",
            text: "sample\tcoverage\ttaxonomy\nA\t1\tRoot; d__Bacteria\n",
        };
        let b = NamedText {
            name: "b.tsv",
            text: "sample\tcoverage\ttaxonomy\nB\t2\tRoot; d__Archaea\n",
        };
        let spf = to_spf(&[a, b]).unwrap();
        assert!(spf.starts_with("Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tA\tB\n"));
        assert!(to_spf(&[a, a])
            .unwrap_err()
            .0
            .contains("more than one file"));

        assert!(!looks_like_singlem("Level 1\tS1\tS2\n"));
        let err = to_spf(&one("x.tsv", "Level 1\tS1\nA\t1\n")).unwrap_err();
        assert!(err.0.contains("SingleM condensed profile"), "{}", err.0);
        let err = to_spf(&one("x.tsv", "sample\tcoverage\ttaxonomy\nA\tlots\tRoot\n")).unwrap_err();
        assert_eq!(err.0, "Line 2 of x.tsv: the coverage is not a number.");
    }
}
