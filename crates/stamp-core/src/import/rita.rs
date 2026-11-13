//! RITA taxonomic classifications: one file per sample, one row per read.

use super::{sample_name_from_file_name, NamedText, Rows};
use crate::error::{Result, StampError};
use crate::text;

/// Classification groups a RITA file can contain. Only reads classified by
/// the groups passed to [`to_spf`] are counted.
pub const GROUPS: [&str; 7] = [
    "NB and D-BLASTN",
    "D-BLASTN ratio",
    "NB and BLASTN",
    "BLASTN ratio",
    "NB and BLASTX",
    "BLASTX ratio",
    "NB ratio",
];

const RANKS: [&str; 7] = [
    "DOMAIN", "PHYLUM", "CLASS", "ORDER", "FAMILY", "GENUS", "SPECIES",
];
const HEADINGS: [&str; 7] = [
    "Domain", "Phylum", "Class", "Order", "Family", "Genus", "Species",
];

/// Combine one RITA result file per sample into a STAMP profile, counting the
/// reads classified by any of `groups`.
pub fn to_spf(files: &[NamedText<'_>], groups: &[&str]) -> Result<String> {
    let mut profile: Rows<Vec<u64>> = Rows::new();
    let mut deepest_rank = 0;

    for (sample, file) in files.iter().enumerate() {
        for (i, line) in text::lines(file.text).into_iter().enumerate().skip(1) {
            let line = text::strip(line);
            if line.is_empty() {
                continue;
            }
            let invalid = || {
                StampError::new(format!(
                    "{} does not appear to be a valid RITA profile (line {}).",
                    file.name,
                    i + 1
                ))
            };
            let cells: Vec<&str> = line.split('\t').collect();
            if cells.len() < 5 {
                return Err(invalid());
            }
            if !groups.contains(&cells[2]) {
                continue;
            }
            let rank = RANKS
                .iter()
                .position(|r| *r == cells[3])
                .ok_or_else(invalid)?;
            deepest_rank = deepest_rank.max(rank);

            // The lineage is listed from the most specific rank upwards.
            let lineage = cells[4..].join(";");
            profile.entry(&lineage, || vec![0; files.len()])[sample] += 1;
        }
    }

    let mut out = HEADINGS[..=deepest_rank].join("\t");
    for file in files {
        out.push('\t');
        out.push_str(sample_name_from_file_name(file.name));
    }
    out.push('\n');
    for (lineage, counts) in profile.iter() {
        let levels: Vec<&str> = lineage.split(';').collect();
        for level in levels.iter().rev() {
            out.push_str(level);
            out.push('\t');
        }
        for _ in levels.len()..=deepest_rank {
            out.push_str("Unclassified\t");
        }
        let counts: Vec<String> = counts.iter().map(u64::to_string).collect();
        out.push_str(&counts.join("\t"));
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_reads_of_selected_groups() {
        let a = "read\tscore\tgroup\trank\tlineage\n\
            r1\t1\tNB ratio\tGENUS\tBacillus\tBacillaceae\tBacillales\tBacilli\tFirmicutes\tBacteria\n\
            r2\t1\tNB ratio\tGENUS\tBacillus\tBacillaceae\tBacillales\tBacilli\tFirmicutes\tBacteria\n\
            r3\t1\tBLASTX ratio\tPHYLUM\tFirmicutes\tBacteria\n\
            r4\t1\tBLASTN ratio\tDOMAIN\tArchaea\n";
        let b =
            "read\tscore\tgroup\trank\tlineage\nr1\t1\tNB ratio\tPHYLUM\tFirmicutes\tBacteria\n";
        let files = [
            NamedText {
                name: "rita_gutA.txt",
                text: a,
            },
            NamedText {
                name: "rita_gutB.txt",
                text: b,
            },
        ];
        assert_eq!(
            to_spf(&files, &["NB ratio", "BLASTX ratio"]).unwrap(),
            "Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tgutA\tgutB\n\
             Bacteria\tFirmicutes\tBacilli\tBacillales\tBacillaceae\tBacillus\t2\t0\n\
             Bacteria\tFirmicutes\tUnclassified\tUnclassified\tUnclassified\tUnclassified\t1\t1\n"
        );
    }

    #[test]
    fn rejects_unknown_ranks() {
        let files = [NamedText {
            name: "a.txt",
            text: "h\nr\t1\tNB ratio\tKINGDOM\tx\n",
        }];
        assert!(to_spf(&files, &GROUPS).unwrap_err().0.contains("line 2"));
    }
}
