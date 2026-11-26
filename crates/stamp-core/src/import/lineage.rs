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

/// Hierarchical levels of the profiles made from GTDB lineages.
pub const RANKS: [&str; 7] = [
    "Domain", "Phylum", "Class", "Order", "Family", "Genus", "Species",
];

/// GTDB rank prefixes, by rank.
pub const PREFIXES: [&str; 7] = ["d__", "p__", "c__", "o__", "f__", "g__", "s__"];

/// The ranks of a profile: the headings of its hierarchical levels and the
/// prefix written before a taxon of each rank in `unclassified <taxon>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranks {
    names: Vec<String>,
    prefixes: Vec<String>,
}

impl Ranks {
    /// Ranks with these headings and prefixes (one of each per rank).
    pub fn new(names: &[&str], prefixes: &[&str]) -> Ranks {
        assert_eq!(names.len(), prefixes.len());
        Ranks {
            names: names.iter().map(|s| s.to_string()).collect(),
            prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// The seven GTDB ranks, from domain to species.
    pub fn gtdb() -> Ranks {
        Ranks::new(&RANKS, &PREFIXES)
    }

    /// A profile of one level, such as the species of a Bracken table.
    pub fn single(name: &str) -> Ranks {
        Ranks::new(&[name], &[""])
    }

    /// These ranks with one more below them.
    pub fn with(mut self, name: &str, prefix: &str) -> Ranks {
        self.names.push(name.to_string());
        self.prefixes.push(prefix.to_string());
        self
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

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
    ranks: Ranks,
    /// Values are whole numbers (read counts), written without rounding.
    counts: bool,
    samples: Vec<String>,
    sample_index: HashMap<String, usize>,
    /// Each lineage (names from the domain down, without prefixes) with its
    /// coverage in the samples that have it.
    rows: Rows<LineageRow>,
}

impl Lineages {
    /// Lineages of the seven GTDB ranks.
    pub(crate) fn new() -> Self {
        Lineages::with_ranks(Ranks::gtdb())
    }

    pub(crate) fn with_ranks(ranks: Ranks) -> Self {
        Lineages {
            ranks,
            counts: false,
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

    /// Values are read counts: written as whole numbers, not rounded to six
    /// significant digits.
    pub(crate) fn set_counts(&mut self, counts: bool) {
        self.counts = counts;
    }

    /// Add a value to a lineage given as `(rank, name)` pairs from the top,
    /// ranks being indexes into the profile's ranks in increasing order.
    /// A rank the lineage skips (NCBI taxa often have no class, say) is
    /// filled with `<parent> (no <rank>)`, `Root` standing for the parent of
    /// the top rank.
    pub(crate) fn add_ranked(
        &mut self,
        sample: usize,
        lineage: &[(usize, &str)],
        value: f64,
    ) -> Result<()> {
        let mut names: Vec<String> = Vec::new();
        for &(rank, name) in lineage {
            if rank >= self.ranks.names.len() || rank < names.len() {
                return Err(StampError::new(format!(
                    "The lineage {} does not go down the ranks in order.",
                    lineage
                        .iter()
                        .map(|(_, n)| *n)
                        .collect::<Vec<_>>()
                        .join("; ")
                )));
            }
            while names.len() < rank {
                let parent = names.last().map_or("Root", |p| p.as_str());
                let filler = format!(
                    "{} (no {})",
                    parent.split(" (no ").next().unwrap_or(parent),
                    self.ranks.names[names.len()].to_lowercase()
                );
                names.push(filler);
            }
            names.push(name.to_string());
        }
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        self.add(sample, &names, value)
    }

    /// Add coverage to a lineage of at most as many names as there are
    /// ranks (none for coverage that was not placed in the top rank).
    pub(crate) fn add(&mut self, sample: usize, lineage: &[&str], coverage: f64) -> Result<()> {
        let ranks = &self.ranks.names;
        if lineage.len() > ranks.len() {
            return Err(StampError::new(format!(
                "The lineage {} has more than the {} ranks from {} to {}.",
                lineage.join("; "),
                ranks.len(),
                ranks[0].to_lowercase(),
                ranks[ranks.len() - 1].to_lowercase()
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
        if self.rows.iter().next().is_none() {
            // A tool that found nothing (MetaPhlAn's reads for a sample of
            // unknown organisms, HUMAnN's pathways without species).
            return Err(StampError::new(
                "Nothing in the files has a value above zero: the tool found nothing to \
                 profile, so there is no profile to load.",
            ));
        }
        let mut lineages: Vec<Vec<String>> = self
            .rows
            .iter()
            .map(|(_, (lineage, _))| lineage.clone())
            .collect();
        qualify_reused_names(&mut lineages, self.ranks.names.len());

        // Rows that are the same once padded are merged.
        let mut padded: Rows<Vec<usize>> = Rows::new();
        for (i, lineage) in lineages.iter().enumerate() {
            let row = pad(lineage, &self.ranks);
            padded.entry(&row.join("\t"), Vec::new).push(i);
        }

        let coverages: Vec<&Vec<(usize, f64)>> =
            self.rows.iter().map(|(_, (_, values))| values).collect();
        let mut out = String::with_capacity(64 * self.samples.len());
        out.push_str(&self.ranks.names.join("\t"));
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
                } else if self.counts {
                    out.push_str(&format!("{}", value.round()));
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
fn qualify_reused_names(lineages: &mut [Vec<String>], ranks: usize) {
    for rank in 1..ranks {
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

/// A lineage padded to every rank with `unclassified <prefix><taxon>`.
fn pad(lineage: &[String], ranks: &Ranks) -> Vec<String> {
    let mut row: Vec<String> = lineage.to_vec();
    let filler = match lineage.last() {
        Some(deepest) => format!(
            "unclassified {}{deepest}",
            ranks.prefixes[lineage.len() - 1]
        ),
        None => UNPLACED.to_string(),
    };
    row.resize(ranks.names.len(), filler);
    row
}

/// A number as a tool printed it, with the uncertainty its printing leaves:
/// half a unit in its last digit, none for a whole number. `None` if it is
/// not a non-negative number.
pub(crate) fn parse_value(text: &str) -> Option<(f64, f64)> {
    let text = text.trim();
    let value: f64 = text.parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(i) => (&text[..i], text[i + 1..].parse::<i32>().ok()?),
        None => (text, 0),
    };
    let decimals = mantissa.find('.').map_or(0, |i| mantissa.len() - i - 1) as i32;
    let slack = if decimals == 0 && exponent == 0 {
        0.0
    } else {
        0.5 * 10f64.powi(exponent - decimals)
    };
    Some((value, slack))
}

/// A row of a table of clades: the parts of its name from the top (each
/// clade's path starting with its ancestors' paths; empty for what was not
/// classified), and its value in each sample with that value's rounding.
#[derive(Debug, Clone)]
pub(crate) struct Clade {
    pub path: Vec<String>,
    pub values: Vec<(f64, f64)>,
}

/// The own value of each clade in each sample: its value less the values of
/// the clades nearest below it in the table. A table that lists only the
/// lowest clades is left as it is. A remainder within the rounding of the
/// numbers involved, or below zero, is zero.
pub(crate) fn own_values(clades: &[Clade]) -> Vec<Vec<f64>> {
    let index: HashMap<&[String], usize> = clades
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.path.is_empty())
        .map(|(i, c)| (c.path.as_slice(), i))
        .collect();
    let mut own: Vec<Vec<f64>> = clades
        .iter()
        .map(|c| c.values.iter().map(|v| v.0).collect())
        .collect();
    let mut slack: Vec<Vec<f64>> = clades
        .iter()
        .map(|c| c.values.iter().map(|v| v.1).collect())
        .collect();
    let mut parent_of_some = vec![false; clades.len()];
    for clade in clades {
        let parent = (1..clade.path.len())
            .rev()
            .find_map(|k| index.get(&clade.path[..k]).copied());
        if let Some(parent) = parent {
            parent_of_some[parent] = true;
            for (s, &(value, rounding)) in clade.values.iter().enumerate() {
                own[parent][s] -= value;
                slack[parent][s] += rounding;
            }
        }
    }
    for (i, clade) in clades.iter().enumerate() {
        if !parent_of_some[i] {
            continue;
        }
        for (s, value) in own[i].iter_mut().enumerate() {
            let tolerance = slack[i][s] + 1e-9 * clade.values[s].0.abs();
            if *value <= tolerance {
                *value = 0.0;
            }
        }
    }
    own
}

/// The parts of a lineage written with rank prefixes and joined with `|` or
/// `;` (`k__Bacteria|p__Firmicutes`, `d__Bacteria;p__Bacillota`).
pub(crate) fn split_lineage(text: &str) -> Vec<String> {
    text.split(['|', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// The rank letter and name of a taxon written with a rank prefix
/// (`p__Firmicutes` is `('p', "Firmicutes")`).
pub(crate) fn rank_prefix(taxon: &str) -> Option<(char, &str)> {
    let bytes = taxon.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b"__" {
        Some(((bytes[0] as char).to_ascii_lowercase(), taxon[3..].trim()))
    } else {
        None
    }
}

/// The `(rank, name)` pairs of a path of prefixed taxa. Taxa whose prefix
/// `rank_of` gives no rank are left out; an empty name (`s__`) ends the
/// lineage.
pub(crate) fn ranked<'a>(
    path: &'a [String],
    rank_of: &impl Fn(char) -> Option<usize>,
) -> Result<Vec<(usize, &'a str)>> {
    let mut lineage = Vec::new();
    for part in path {
        let Some((letter, name)) = rank_prefix(part) else {
            return Err(StampError::new(format!(
                "The taxon '{part}' has no rank prefix such as p__."
            )));
        };
        if name.is_empty() {
            break;
        }
        if let Some(rank) = rank_of(letter) {
            lineage.push((rank, name));
        }
    }
    Ok(lineage)
}

/// Add the own values of a table of clades with prefixed names to the
/// samples `samples` (one per value column).
pub(crate) fn add_clades(
    lineages: &mut Lineages,
    samples: &[usize],
    clades: &[Clade],
    rank_of: impl Fn(char) -> Option<usize>,
) -> Result<()> {
    let own = own_values(clades);
    for (clade, own) in clades.iter().zip(&own) {
        let lineage = ranked(&clade.path, &rank_of)?;
        for (column, &value) in own.iter().enumerate() {
            if value > 0.0 {
                lineages.add_ranked(samples[column], &lineage, value)?;
            }
        }
    }
    Ok(())
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
        let mut empty = Lineages::new();
        empty.sample("A");
        assert!(empty.to_spf().unwrap_err().0.contains("no profile to load"));
        let mut deep = Lineages::new();
        let s = deep.sample("A");
        assert!(deep.add(s, &["a"; 8], 1.0).is_err());
    }
}
