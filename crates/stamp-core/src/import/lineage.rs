//! Taxonomic profiles of the seven GTDB ranks built from coverages assigned
//! to the nodes of a taxonomy, as SingleM and Sandpiper report them.
//!
//! The coverage of a node is its own, not that of its descendants: coverage
//! at a family is what could not be placed in any of its genera. Such
//! coverage becomes a row padded with `unclassified <taxon>` at every lower
//! rank, the taxon written with its rank prefix (`unclassified
//! f__Oscillospiraceae`) so that the padded names stay distinct where GTDB
//! uses one name at two ranks. Named taxa are written without their
//! prefixes. A name that has more than one parent at the same rank (a genus
//! placed in two families, say, by profiles made with different GTDB
//! releases) is written as `name (parent)` so the hierarchy stays strict.

use std::collections::{HashMap, HashSet};

use super::Rows;
use crate::error::{Result, StampError};
use crate::text;

/// Hierarchical levels of the profiles made from taxonomic lineages.
pub const RANKS: [&str; 7] = [
    "Domain", "Phylum", "Class", "Order", "Family", "Genus", "Species",
];

/// GTDB rank prefixes, by rank.
pub const PREFIXES: [&str; 7] = ["d__", "p__", "c__", "o__", "f__", "g__", "s__"];

/// Name given at every rank to coverage that was not placed in any domain.
const UNPLACED: &str = "unclassified Root";

/// A taxon name without a GTDB-style rank prefix (`g__Limivicinus` becomes
/// `Limivicinus`).
pub fn strip_rank_prefix(taxon: &str) -> &str {
    let taxon = taxon.trim();
    let bytes = taxon.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b"__" {
        taxon[3..].trim()
    } else {
        taxon
    }
}

/// A lineage with its coverage in the samples that have it.
type LineageRow = (Vec<String>, Vec<(usize, f64)>);

/// Coverages of lineages in a set of samples, collected before the profile
/// is written.
pub(crate) struct Lineages {
    samples: Vec<String>,
    sample_index: HashMap<String, usize>,
    /// Each lineage (names from the domain down, without prefixes) with its
    /// coverage in the samples that have it.
    rows: Rows<LineageRow>,
}

impl Lineages {
    pub(crate) fn new() -> Self {
        Lineages {
            samples: Vec::new(),
            sample_index: HashMap::new(),
            rows: Rows::new(),
        }
    }

    /// Index of a sample, added if it is new.
    pub(crate) fn sample(&mut self, name: &str) -> usize {
        if let Some(&i) = self.sample_index.get(name) {
            return i;
        }
        self.samples.push(name.to_string());
        self.sample_index
            .insert(name.to_string(), self.samples.len() - 1);
        self.samples.len() - 1
    }

    pub(crate) fn has_sample(&self, name: &str) -> bool {
        self.sample_index.contains_key(name)
    }

    /// Add coverage to a lineage of at most seven names (none for coverage
    /// that was not placed in a domain).
    pub(crate) fn add(&mut self, sample: usize, lineage: &[&str], coverage: f64) -> Result<()> {
        if lineage.len() > RANKS.len() {
            return Err(StampError::new(format!(
                "The lineage {} has more than the seven ranks from domain to species.",
                lineage.join("; ")
            )));
        }
        let key = lineage.join("\u{1}");
        let row = self.rows.entry(&key, || {
            (lineage.iter().map(|s| s.to_string()).collect(), Vec::new())
        });
        row.1.push((sample, coverage));
        Ok(())
    }

    /// The text of the STAMP profile.
    pub(crate) fn to_spf(&self) -> Result<String> {
        if self.samples.is_empty() {
            return Err(StampError::new("The files contain no samples."));
        }
        let mut lineages: Vec<Vec<String>> = self
            .rows
            .iter()
            .map(|(_, (lineage, _))| lineage.clone())
            .collect();
        qualify_reused_names(&mut lineages);

        // Rows that are the same once padded are merged.
        let mut padded: Rows<Vec<usize>> = Rows::new();
        for (i, lineage) in lineages.iter().enumerate() {
            let row = pad(lineage);
            padded.entry(&row.join("\t"), Vec::new).push(i);
        }

        let coverages: Vec<&Vec<(usize, f64)>> =
            self.rows.iter().map(|(_, (_, values))| values).collect();
        let mut out = String::with_capacity(64 * self.samples.len());
        out.push_str(&RANKS.join("\t"));
        for sample in &self.samples {
            out.push('\t');
            out.push_str(sample);
        }
        out.push('\n');
        let mut values = vec![0.0; self.samples.len()];
        for (names, members) in padded.iter() {
            values.iter_mut().for_each(|v| *v = 0.0);
            for &member in members {
                for &(sample, coverage) in coverages[member] {
                    values[sample] += coverage;
                }
            }
            out.push_str(names);
            for &value in &values {
                out.push('\t');
                if value == 0.0 {
                    out.push('0');
                } else {
                    out.push_str(&text::format_float(round_significant(value)));
                }
            }
            out.push('\n');
        }
        Ok(out)
    }
}

/// A value to six significant digits, which drops the noise of coverages
/// stored as single-precision numbers (0.18000000715255737 becomes 0.18).
fn round_significant(value: f64) -> f64 {
    format!("{value:.5e}").parse().unwrap_or(value)
}

/// Give every name that has more than one parent at its rank the name of its
/// parent, `name (parent)`, rank by rank from the top.
fn qualify_reused_names(lineages: &mut [Vec<String>]) {
    for rank in 1..RANKS.len() {
        let mut parents: HashMap<&str, HashSet<&str>> = HashMap::new();
        for lineage in lineages.iter() {
            if let (Some(parent), Some(name)) = (lineage.get(rank - 1), lineage.get(rank)) {
                parents.entry(name).or_default().insert(parent);
            }
        }
        let reused: HashSet<String> = parents
            .into_iter()
            .filter(|(_, parents)| parents.len() > 1)
            .map(|(name, _)| name.to_string())
            .collect();
        if reused.is_empty() {
            continue;
        }
        for lineage in lineages.iter_mut() {
            if lineage.get(rank).is_some_and(|name| reused.contains(name)) {
                lineage[rank] = format!("{} ({})", lineage[rank], lineage[rank - 1]);
            }
        }
    }
}

/// A lineage padded to seven ranks with `unclassified <prefix><taxon>`.
fn pad(lineage: &[String]) -> Vec<String> {
    let mut row: Vec<String> = lineage.to_vec();
    let filler = match lineage.last() {
        Some(deepest) => format!("unclassified {}{deepest}", PREFIXES[lineage.len() - 1]),
        None => UNPLACED.to_string(),
    };
    row.resize(RANKS.len(), filler);
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_are_removed() {
        assert_eq!(strip_rank_prefix("g__Limivicinus"), "Limivicinus");
        assert_eq!(
            strip_rank_prefix(" s__Limivicinus sp030531505"),
            "Limivicinus sp030531505"
        );
        assert_eq!(strip_rank_prefix("Bacteria"), "Bacteria");
        assert_eq!(strip_rank_prefix("s__"), "");
    }

    #[test]
    fn coverage_at_a_higher_rank_is_padded_with_unclassified_names() {
        let mut lineages = Lineages::new();
        let a = lineages.sample("A");
        let b = lineages.sample("B");
        let family = [
            "Bacteria",
            "Bacillota",
            "Clostridia",
            "Oscillospirales",
            "Oscillospiraceae",
        ];
        lineages.add(a, &family, 1.5).unwrap();
        lineages.add(b, &family, 0.25).unwrap();
        lineages.add(b, &["Bacteria"], 2.0).unwrap();
        lineages.add(a, &[], 0.5).unwrap();
        let spf = lineages.to_spf().unwrap();
        assert_eq!(
            spf,
            "Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tA\tB\n\
             Bacteria\tBacillota\tClostridia\tOscillospirales\tOscillospiraceae\t\
             unclassified f__Oscillospiraceae\tunclassified f__Oscillospiraceae\t1.5\t0.25\n\
             Bacteria\tunclassified d__Bacteria\tunclassified d__Bacteria\t\
             unclassified d__Bacteria\tunclassified d__Bacteria\tunclassified d__Bacteria\t\
             unclassified d__Bacteria\t0\t2.0\n\
             unclassified Root\tunclassified Root\tunclassified Root\tunclassified Root\t\
             unclassified Root\tunclassified Root\tunclassified Root\t0.5\t0\n"
        );
        crate::ProfileTree::read(&spf).unwrap();
    }

    #[test]
    fn names_used_at_two_ranks_stay_distinct() {
        // UBA644 is both an order and a family in GTDB.
        let mut lineages = Lineages::new();
        let a = lineages.sample("A");
        lineages.sample("B");
        lineages
            .add(a, &["Bacteria", "Bacillota", "Bacilli", "UBA644"], 1.0)
            .unwrap();
        lineages
            .add(
                a,
                &["Bacteria", "Bacillota", "Bacilli", "RFN20", "UBA644"],
                1.0,
            )
            .unwrap();
        let spf = lineages.to_spf().unwrap();
        assert!(spf.contains("\tunclassified o__UBA644\tunclassified o__UBA644\t"));
        assert!(spf.contains("\tRFN20\tUBA644\tunclassified f__UBA644\t"));
        crate::ProfileTree::read(&spf).unwrap();
    }

    #[test]
    fn a_name_with_two_parents_is_qualified_by_its_parent() {
        let mut lineages = Lineages::new();
        let a = lineages.sample("A");
        let b = lineages.sample("B");
        lineages
            .add(a, &["Bacteria", "P1", "C1", "O1", "F1", "G"], 1.0)
            .unwrap();
        lineages
            .add(b, &["Bacteria", "P1", "C1", "O1", "F2", "G", "G sp1"], 2.0)
            .unwrap();
        let spf = lineages.to_spf().unwrap();
        assert!(
            spf.contains("\tF1\tG (F1)\tunclassified g__G (F1)\t1.0\t0\n"),
            "{spf}"
        );
        assert!(spf.contains("\tF2\tG (F2)\tG sp1\t0\t2.0\n"), "{spf}");
        crate::ProfileTree::read(&spf).unwrap();
    }

    #[test]
    fn values_are_summed_and_rounded() {
        let mut lineages = Lineages::new();
        let a = lineages.sample("A");
        lineages.sample("B");
        lineages.add(a, &["Bacteria"], 0.18000000715255737).unwrap();
        lineages.add(a, &["Bacteria"], 0.1).unwrap();
        assert!(lineages.to_spf().unwrap().ends_with("\t0.28\t0\n"));
        let mut deep = Lineages::new();
        let s = deep.sample("A");
        assert!(deep.add(s, &["a"; 8], 1.0).is_err());
    }
}
