//! Sample metadata and the grouping of samples by a metadata field.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::error::{Result, StampError};
use crate::profile_tree::ProfileTree;
use crate::selection::SampleSelection;
use crate::text;

/// Metadata for the samples of a profile: one value per sample and field.
#[derive(Debug, Clone)]
pub struct Metadata {
    fields: Vec<String>,
    samples: Vec<String>,
    /// `values[sample][field]`
    values: Vec<Vec<String>>,
    sample_index: HashMap<String, usize>,
    /// Samples switched on or off one by one.
    active: Vec<bool>,
    /// The sample filter, and which samples it keeps.
    selection: SampleSelection,
    selected: Vec<bool>,
}

/// Samples grouped by the value of one metadata field. Group names and the
/// samples within each group are sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Groups {
    pub field: String,
    groups: BTreeMap<String, Vec<String>>,
}

impl Metadata {
    /// Parse a metadata file: a header row, then one row per sample with the
    /// sample name in the first column. Samples that are not in the profile
    /// are ignored. Returns the metadata and any warnings for the user.
    pub fn read(text: &str, tree: &ProfileTree) -> Result<(Metadata, Vec<String>)> {
        // Lines are split on tabs before their cells are stripped: stripping
        // the whole line first would remove the tab before an empty first
        // cell (as pandas writes a header for an unnamed index), and shift
        // every field name by one.
        let lines = text::lines(text);
        if lines.is_empty() {
            return Err(StampError::new("Metadata file is empty."));
        }

        let mut header = text::cells(lines[0]);
        // A tab at the end of the header names no field.
        while header.len() > 1 && header.last() == Some(&"") {
            header.pop();
        }
        let fields: Vec<String> = header.iter().skip(1).map(|s| s.to_string()).collect();

        let mut metadata = Metadata {
            fields,
            samples: Vec::new(),
            values: Vec::new(),
            sample_index: HashMap::new(),
            active: Vec::new(),
            selection: SampleSelection::default(),
            selected: Vec::new(),
        };

        let mut unknown = Vec::new();
        let mut duplicated = Vec::new();
        for line in &lines[1..] {
            if text::strip(line).is_empty() {
                continue;
            }
            let cells = text::cells(line);
            let sample = cells[0];
            if tree.sample_index(sample).is_none() {
                unknown.push(sample.to_string());
                continue;
            }
            if metadata.sample_index.contains_key(sample) {
                duplicated.push(sample.to_string());
                continue;
            }

            // Short rows are padded with empty values; extra cells are ignored.
            let row: Vec<String> = (1..=metadata.fields.len())
                .map(|i| cells.get(i).copied().unwrap_or("").to_string())
                .collect();
            metadata
                .sample_index
                .insert(sample.to_string(), metadata.samples.len());
            metadata.samples.push(sample.to_string());
            metadata.values.push(row);
            metadata.active.push(true);
            metadata.selected.push(true);
        }

        let mut warnings = Vec::new();
        if !unknown.is_empty() {
            warnings.push(format!(
                "Unknown sample(s) specified in metadata file: {}",
                unknown.join(", ")
            ));
        }
        if !duplicated.is_empty() {
            warnings.push(format!(
                "Sample(s) listed more than once in metadata file (first entry used): {}",
                duplicated.join(", ")
            ));
        }
        let missing: Vec<&str> = tree
            .sample_names()
            .iter()
            .filter(|s| !metadata.sample_index.contains_key(*s))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            warnings.push(format!(
                "Missing metadata for the following samples: {}",
                missing.join(", ")
            ));
        }

        Ok((metadata, warnings))
    }

    /// Metadata fields, in file order.
    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    /// Samples with metadata, in file order.
    pub fn samples(&self) -> &[String] {
        &self.samples
    }

    pub fn value(&self, sample: &str, field: &str) -> Option<&str> {
        let s = *self.sample_index.get(sample)?;
        let f = self.field_index(field)?;
        Some(&self.values[s][f])
    }

    /// Distinct values of a field, sorted.
    pub fn unique_values(&self, field: &str) -> Vec<&str> {
        let Some(f) = self.field_index(field) else {
            return Vec::new();
        };
        let mut values: Vec<&str> = self.values.iter().map(|row| row[f].as_str()).collect();
        values.sort_unstable();
        values.dedup();
        values
    }

    /// True if every value of the field is a number.
    pub fn is_numeric(&self, field: &str) -> bool {
        match self.field_index(field) {
            Some(f) => self
                .values
                .iter()
                .all(|row| text::parse_float(&row[f]).is_some()),
            None => false,
        }
    }

    /// True if a sample takes part in group analyses: it is switched on
    /// and the sample filter keeps it.
    pub fn is_active(&self, sample: &str) -> bool {
        self.sample_index
            .get(sample)
            .is_some_and(|&i| self.active[i] && self.selected[i])
    }

    /// True if a sample is switched on (ticked in the metadata table),
    /// whether or not the sample filter keeps it.
    pub fn is_switched_on(&self, sample: &str) -> bool {
        self.sample_index
            .get(sample)
            .is_some_and(|&i| self.active[i])
    }

    /// True if the sample filter keeps a sample (whether or not it is
    /// switched on).
    pub fn is_selected(&self, sample: &str) -> bool {
        self.sample_index
            .get(sample)
            .is_some_and(|&i| self.selected[i])
    }

    /// Choose the samples of the analyses by their metadata values. Samples
    /// the selection does not keep are left out of everything, and groups
    /// none of whose samples it keeps disappear. Returns how many samples
    /// it keeps. An empty selection keeps every sample.
    pub fn set_selection(&mut self, selection: SampleSelection) -> Result<usize> {
        selection.check(self)?;
        let selected: Vec<bool> = self
            .samples
            .iter()
            .map(|sample| selection.accepts(self, sample))
            .collect();
        self.selected = selected;
        self.selection = selection;
        Ok(self.num_selected())
    }

    /// The sample filter (empty if every sample is kept).
    pub fn selection(&self) -> &SampleSelection {
        &self.selection
    }

    /// Number of samples the sample filter keeps.
    pub fn num_selected(&self) -> usize {
        self.selected.iter().filter(|&&s| s).count()
    }

    /// Include or exclude a sample from group analyses.
    pub fn set_active(&mut self, sample: &str, active: bool) {
        if let Some(&i) = self.sample_index.get(sample) {
            self.active[i] = active;
        }
    }

    pub fn active_samples(&self) -> Vec<&str> {
        self.samples
            .iter()
            .enumerate()
            .filter(|(i, _)| self.active[*i] && self.selected[*i])
            .map(|(_, s)| s.as_str())
            .collect()
    }

    /// Group the active samples by the value of a field. A group whose
    /// samples are all switched off is kept, with no samples; one whose
    /// samples the sample filter leaves out is not. Samples with no value
    /// for the field belong to no group (nor to "all other samples").
    pub fn groups(&self, field: &str) -> Result<Groups> {
        let f = self
            .field_index(field)
            .ok_or_else(|| StampError::new(format!("Unknown metadata field '{field}'.")))?;

        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (i, sample) in self.samples.iter().enumerate() {
            if !self.selected[i] || self.values[i][f].is_empty() {
                continue;
            }
            let members = groups.entry(self.values[i][f].clone()).or_default();
            if self.active[i] {
                members.push(sample.clone());
            }
        }
        for members in groups.values_mut() {
            members.sort_unstable();
        }
        Ok(Groups {
            field: field.to_string(),
            groups,
        })
    }

    /// The active samples that have a number in a field, with that number,
    /// in the order of the metadata table.
    pub fn numeric_values(&self, field: &str) -> Vec<(&str, f64)> {
        let Some(f) = self.field_index(field) else {
            return Vec::new();
        };
        self.samples
            .iter()
            .enumerate()
            .filter(|(i, _)| self.active[*i] && self.selected[*i])
            .filter_map(|(i, sample)| {
                text::parse_float(&self.values[i][f])
                    .filter(|value| value.is_finite())
                    .map(|value| (sample.as_str(), value))
            })
            .collect()
    }

    /// Match the samples of two groups in pairs by the value of another
    /// field (the subject each sample came from, say), for the paired
    /// tests. A value forms a pair if exactly one sample of each group has
    /// it; every other sample of the two groups is left out. The groups
    /// returned list their samples pair by pair, in the order of the
    /// values.
    pub fn paired_groups(
        &self,
        groups: &Groups,
        group1: &str,
        group2: &str,
        pair_field: &str,
    ) -> Result<Pairing> {
        let f = self
            .field_index(pair_field)
            .ok_or_else(|| StampError::new(format!("Unknown metadata field '{pair_field}'.")))?;
        let by_value = |group: &str| -> Result<BTreeMap<&str, Vec<&String>>> {
            let samples = groups
                .samples(group)
                .ok_or_else(|| StampError::new(format!("Unknown group '{group}'.")))?;
            let mut map: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
            for sample in samples {
                if let Some(i) = self.samples.iter().position(|s| s == sample) {
                    map.entry(self.values[i][f].as_str())
                        .or_default()
                        .push(sample);
                }
            }
            Ok(map)
        };
        if group1 == group2 {
            return Err(StampError::new("A paired test needs two different groups."));
        }
        let (first, second) = (by_value(group1)?, by_value(group2)?);
        let total: usize = first.values().chain(second.values()).map(Vec::len).sum();
        let (mut samples1, mut samples2) = (Vec::new(), Vec::new());
        for (value, members) in &first {
            if let ([a], Some([b])) = (members.as_slice(), second.get(value).map(Vec::as_slice)) {
                if !value.trim().is_empty() {
                    samples1.push((*a).clone());
                    samples2.push((*b).clone());
                }
            }
        }
        let pairs = samples1.len();
        let mut paired = groups.clone();
        paired.groups.insert(group1.to_string(), samples1);
        paired.groups.insert(group2.to_string(), samples2);
        Ok(Pairing {
            groups: paired,
            pairs,
            unpaired: total - 2 * pairs,
        })
    }

    fn field_index(&self, field: &str) -> Option<usize> {
        self.fields.iter().position(|f| f == field)
    }
}

/// Two groups with their samples matched in pairs; see
/// [`Metadata::paired_groups`].
#[derive(Debug, Clone)]
pub struct Pairing {
    /// The grouping, with the samples of the two groups pair by pair.
    pub groups: Groups,
    pub pairs: usize,
    /// Samples of the two groups that had no partner, or more than one.
    pub unpaired: usize,
}

impl Pairing {
    /// A sentence saying how many pairs were found.
    pub fn summary(&self) -> String {
        let pairs = match self.pairs {
            1 => "1 pair".to_string(),
            n => format!("{n} pairs"),
        };
        match self.unpaired {
            0 => pairs,
            1 => format!("{pairs}; 1 sample without a partner left out"),
            n => format!("{pairs}; {n} samples without a partner left out"),
        }
    }
}

impl Groups {
    /// Group names, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.groups.keys().map(String::as_str)
    }

    /// Samples of a group, sorted.
    pub fn samples(&self, group: &str) -> Option<&[String]> {
        self.groups.get(group).map(Vec::as_slice)
    }

    /// Samples of every group except one, sorted.
    pub fn all_other_samples(&self, group: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut samples: Vec<String> = self
            .groups
            .iter()
            .filter(|(name, _)| name.as_str() != group)
            .flat_map(|(_, members)| members.iter().cloned())
            .filter(|s| seen.insert(s.clone()))
            .collect();
        samples.sort_unstable();
        samples
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> ProfileTree {
        ProfileTree::read("F\tS1\tS2\tS3\tS4\nx\t1\t2\t3\t4\n").unwrap()
    }

    const META: &str = "Sample Id\tGroup\tAge\n\
        S3\tB\t30\n\
        S1\tA\t10\n\
        \n\
        S2\tA\t20\n\
        S9\tC\t5\n\
        S1\tZ\t99\n";

    #[test]
    fn reads_values_and_reports_problems() {
        let (meta, warnings) = Metadata::read(META, &tree()).unwrap();
        assert_eq!(meta.fields(), ["Group", "Age"]);
        assert_eq!(meta.samples(), ["S3", "S1", "S2"]);
        assert_eq!(meta.value("S1", "Group"), Some("A"));
        assert_eq!(meta.value("S1", "Nope"), None);
        assert_eq!(
            warnings,
            [
                "Unknown sample(s) specified in metadata file: S9",
                "Sample(s) listed more than once in metadata file (first entry used): S1",
                "Missing metadata for the following samples: S4",
            ]
        );
    }

    #[test]
    fn describes_fields() {
        let (meta, _) = Metadata::read(META, &tree()).unwrap();
        assert_eq!(meta.unique_values("Group"), ["A", "B"]);
        assert!(meta.is_numeric("Age"));
        assert!(!meta.is_numeric("Group"));
    }

    #[test]
    fn groups_active_samples_in_sorted_order() {
        let (mut meta, _) = Metadata::read(META, &tree()).unwrap();
        let groups = meta.groups("Group").unwrap();
        assert_eq!(groups.names().collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!(groups.samples("A").unwrap(), ["S1", "S2"]);
        assert_eq!(groups.all_other_samples("B"), ["S1", "S2"]);

        meta.set_active("S3", false);
        let groups = meta.groups("Group").unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups.samples("B").unwrap().is_empty());
        assert_eq!(meta.active_samples(), ["S1", "S2"]);
        assert!(meta.groups("Nope").is_err());
    }

    #[test]
    fn the_sample_filter_leaves_samples_and_groups_out() {
        let (mut meta, _) = Metadata::read(META, &tree()).unwrap();
        let only_a = SampleSelection::parse(&["Group=A".to_string()], &[]).unwrap();
        assert_eq!(meta.set_selection(only_a.clone()).unwrap(), 2);
        assert_eq!(meta.selection(), &only_a);
        assert_eq!(meta.active_samples(), ["S1", "S2"]);
        assert!(!meta.is_active("S3") && !meta.is_selected("S3"));
        // The group with no kept sample disappears.
        let groups = meta.groups("Group").unwrap();
        assert_eq!(groups.names().collect::<Vec<_>>(), ["A"]);
        assert_eq!(meta.numeric_values("Age"), [("S1", 10.0), ("S2", 20.0)]);
        // Switching a sample off works within the filter.
        meta.set_active("S1", false);
        assert_eq!(meta.active_samples(), ["S2"]);
        assert!(meta.is_selected("S1"));
        // A bad selection changes nothing; an empty one keeps everything.
        let bad = SampleSelection::parse(&["Group=Q".to_string()], &[]).unwrap();
        assert!(meta.set_selection(bad).is_err());
        assert_eq!(meta.num_selected(), 2);
        assert_eq!(meta.set_selection(SampleSelection::default()).unwrap(), 3);
        assert_eq!(meta.active_samples(), ["S3", "S2"]);
    }

    #[test]
    fn pads_short_rows() {
        let (meta, _) = Metadata::read("Id\tA\tB\nS1\tx\n", &tree()).unwrap();
        assert_eq!(meta.value("S1", "B"), Some(""));
    }

    #[test]
    fn samples_without_a_value_belong_to_no_group() {
        let (meta, _) = Metadata::read("Id\tDiet\nS1\tA\nS2\t\nS3\tB\nS4\n", &tree()).unwrap();
        let groups = meta.groups("Diet").unwrap();
        assert_eq!(groups.names().collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!(groups.all_other_samples("A"), ["S3"]);
    }

    #[test]
    fn a_header_with_an_empty_first_cell_keeps_its_field_names() {
        // As pandas writes a table with an unnamed index: the header starts
        // with a tab, which used to be stripped, shifting every name by one.
        let text = "\tDiet\tpH\t\r\nS1\tA\t7.1\r\n  S2 \tB\t6.5 \r\n \t \r\n";
        let (meta, warnings) = Metadata::read(text, &tree()).unwrap();
        assert_eq!(meta.fields(), ["Diet", "pH"]);
        assert_eq!(meta.value("S1", "Diet"), Some("A"));
        assert_eq!(meta.value("S2", "pH"), Some("6.5"));
        assert_eq!(
            warnings,
            ["Missing metadata for the following samples: S3, S4"]
        );
    }
}
