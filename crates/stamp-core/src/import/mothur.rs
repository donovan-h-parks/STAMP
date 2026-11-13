//! mothur taxonomy, groups and (optional) names files.

use std::collections::{BTreeSet, HashMap};

use super::Rows;
use crate::error::{Result, StampError};
use crate::text;

fn two_columns<'a>(line: &'a str, file: &str, number: usize) -> Result<(&'a str, &'a str)> {
    let mut cells = line.split('\t');
    match (cells.next(), cells.next()) {
        (Some(a), Some(b)) => Ok((text::strip(a), text::strip(b))),
        _ => Err(StampError::new(format!(
            "Line {number} of the {file} file does not have two tab-separated columns."
        ))),
    }
}

/// Build a STAMP profile from mothur output.
///
/// * `taxonomy`: sequence id and its `;`-separated classification;
/// * `groups`: sequence id and the sample it came from;
/// * `names`: optionally, a representative sequence id and the comma-separated
///   ids of all the sequences it stands for.
pub fn to_spf(taxonomy: &str, groups: &str, names: Option<&str>) -> Result<String> {
    let mut sample_of: HashMap<&str, &str> = HashMap::new();
    let mut samples: BTreeSet<&str> = BTreeSet::new();
    for (i, line) in text::lines(groups).into_iter().enumerate() {
        if text::strip(line).is_empty() {
            continue;
        }
        let (sequence, sample) = two_columns(line, "groups", i + 1)?;
        sample_of.insert(sequence, sample);
        samples.insert(sample);
    }
    let samples: Vec<&str> = samples.into_iter().collect();

    let mut represented: Option<HashMap<&str, Vec<&str>>> = None;
    if let Some(names) = names {
        let mut map = HashMap::new();
        for (i, line) in text::lines(names).into_iter().enumerate() {
            if text::strip(line).is_empty() {
                continue;
            }
            let (sequence, members) = two_columns(line, "names", i + 1)?;
            map.insert(sequence, members.split(',').map(text::strip).collect());
        }
        represented = Some(map);
    }

    let mut deepest_rank = 0;
    let mut profile: Rows<(Vec<String>, Vec<u64>)> = Rows::new();
    for (i, line) in text::lines(taxonomy).into_iter().enumerate() {
        if text::strip(line).is_empty() {
            continue;
        }
        let (sequence, lineage) = two_columns(line, "taxonomy", i + 1)?;

        let mut classification: Vec<String> = Vec::new();
        for taxon in lineage.split(';') {
            if text::strip(taxon).is_empty() {
                continue;
            }
            // Remove a trailing confidence score such as "(100)".
            let taxon = match (taxon.ends_with(')'), taxon.rfind('(')) {
                (true, Some(open)) => &taxon[..open],
                _ => taxon,
            };
            classification.push(taxon.to_string());
        }
        deepest_rank = deepest_rank.max(classification.len());

        let members: Vec<&str> = match &represented {
            Some(map) => map
                .get(sequence)
                .ok_or_else(|| {
                    StampError::new(format!("Sequence '{sequence}' is not in the names file."))
                })?
                .clone(),
            None => vec![sequence],
        };

        let key = classification.join("$");
        let entry = profile.entry(&key, || (classification, vec![0; samples.len()]));
        for member in members {
            let sample = sample_of.get(member).ok_or_else(|| {
                StampError::new(format!("Sequence '{member}' is not in the groups file."))
            })?;
            let column = samples.binary_search(sample).expect("sample was collected");
            entry.1[column] += 1;
        }
    }

    if deepest_rank == 0 {
        return Err(StampError::new(
            "The taxonomy file does not contain any classifications.",
        ));
    }

    let mut out = String::new();
    let headings: Vec<String> = (1..=deepest_rank).map(|r| format!("Level {r}")).collect();
    out.push_str(&headings.join("\t"));
    for sample in &samples {
        out.push('\t');
        out.push_str(sample);
    }
    out.push('\n');
    for (_, (classification, counts)) in profile.iter() {
        let mut cells: Vec<String> = classification.clone();
        cells.resize(deepest_rank, "unclassified".to_string());
        cells.extend(counts.iter().map(u64::to_string));
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAXONOMY: &str = "seq1\tBacteria(100);Firmicutes(98);Bacilli(90);\n\
        seq2\tBacteria(100);Firmicutes(95);\n\
        seq3\tBacteria(100);Firmicutes(98);Bacilli(91);\n";
    const GROUPS: &str = "seq1\tgut\nseq2\tsoil\nseq3\tsoil\nseq4\tgut\nseq5\tsoil\n";

    #[test]
    fn counts_sequences_per_sample() {
        assert_eq!(
            to_spf(TAXONOMY, GROUPS, None).unwrap(),
            "Level 1\tLevel 2\tLevel 3\tgut\tsoil\n\
             Bacteria\tFirmicutes\tBacilli\t1\t1\n\
             Bacteria\tFirmicutes\tunclassified\t0\t1\n"
        );
    }

    #[test]
    fn names_file_expands_representative_sequences() {
        let names = "seq1\tseq1,seq4\nseq2\tseq2\nseq3\tseq3, seq5\n";
        assert_eq!(
            to_spf(TAXONOMY, GROUPS, Some(names)).unwrap(),
            "Level 1\tLevel 2\tLevel 3\tgut\tsoil\n\
             Bacteria\tFirmicutes\tBacilli\t2\t2\n\
             Bacteria\tFirmicutes\tunclassified\t0\t1\n"
        );
    }

    #[test]
    fn reports_missing_sequences() {
        let err = to_spf("seq9\tBacteria;\n", GROUPS, None).unwrap_err();
        assert_eq!(err.0, "Sequence 'seq9' is not in the groups file.");
        let err = to_spf(TAXONOMY, GROUPS, Some("seq1\tseq1\n")).unwrap_err();
        assert_eq!(err.0, "Sequence 'seq2' is not in the names file.");
    }
}
