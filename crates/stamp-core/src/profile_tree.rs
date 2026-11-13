//! Reading STAMP profile files (`.spf`) into a hierarchy of features.

use std::collections::HashMap;

use crate::error::{Result, StampError};
use crate::text;

/// Name of the implicit root of every hierarchy.
pub const ENTIRE_SAMPLE: &str = "Entire sample";

/// A profile file: a hierarchy of features with one count per sample at each
/// leaf.
#[derive(Debug, Clone)]
pub struct ProfileTree {
    hierarchy_headings: Vec<String>,
    sample_names: Vec<String>,
    sample_index: HashMap<String, usize>,
    /// Node names; node 0 is the root.
    node_names: Vec<String>,
    /// Leaves in depth-first order, children in order of first appearance.
    leaves: Vec<Leaf>,
    sample_totals: Vec<f64>,
}

#[derive(Debug, Clone)]
pub(crate) struct Leaf {
    /// Node ids from the first hierarchical level down to the leaf.
    pub(crate) path: Vec<usize>,
    pub(crate) counts: Vec<f64>,
}

/// True if a value marks an unclassified feature: `unclassified` in any case,
/// or the GreenGenes form `x__unclassified`.
pub fn is_unclassified(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower == "unclassified" || lower.get(1..) == Some("__unclassified")
}

impl ProfileTree {
    /// Parse the contents of a STAMP profile file.
    pub fn read(text: &str) -> Result<ProfileTree> {
        let lines: Vec<&str> = text::lines(text).into_iter().map(text::strip).collect();
        if lines.len() < 2 {
            return Err(StampError::new(
                "Profile file must contain a header row and at least one row of data.",
            ));
        }

        // The first numeric entry of the first data row marks the first sample column.
        let num_levels = lines[1]
            .split('\t')
            .position(text::is_number)
            .unwrap_or_else(|| lines[1].split('\t').count());

        let headings = text::cells(lines[0]);
        let split = num_levels.min(headings.len());
        let hierarchy_headings: Vec<String> =
            headings[..split].iter().map(|s| s.to_string()).collect();
        let sample_names: Vec<String> = headings[split..].iter().map(|s| s.to_string()).collect();

        if sample_names.len() < 2 {
            return Err(StampError::new(
                "Profile file must contain at least two samples.",
            ));
        }
        if hierarchy_headings.is_empty() {
            return Err(StampError::new(
                "Profile file must contain a column indicating feature names.",
            ));
        }

        let mut sample_index = HashMap::new();
        for (i, name) in sample_names.iter().enumerate() {
            if sample_index.insert(name.clone(), i).is_some() {
                return Err(StampError::new(format!(
                    "Profile file contains more than one sample named '{name}'."
                )));
            }
        }

        check_hierarchy(&lines, num_levels)?;

        let num_samples = sample_names.len();
        let mut builder = Builder::new();
        let mut sample_totals = vec![0.0; num_samples];

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.is_empty() {
                continue;
            }
            let parse_error =
                || StampError::new(format!("Failed to correctly parse line: {}", i + 1));

            let cells = text::cells(line);
            if cells.len() != num_levels + num_samples {
                return Err(parse_error());
            }
            let mut counts = Vec::with_capacity(num_samples);
            for cell in &cells[num_levels..] {
                counts.push(text::parse_float(cell).ok_or_else(parse_error)?);
            }

            // Unclassified entries are named after the last classified level above them.
            let mut last_classified = "";
            let mut node = 0;
            for category in &cells[..num_levels] {
                let name = if is_unclassified(category) {
                    format!("Unclassified {last_classified}")
                        .trim_end()
                        .to_string()
                } else {
                    last_classified = category;
                    category.to_string()
                };
                node = builder.child(node, name);
            }

            for (total, count) in sample_totals.iter_mut().zip(&counts) {
                *total += count;
            }
            builder.add_counts(node, counts);
        }

        let (node_names, leaves) = builder.finish(num_levels);
        Ok(ProfileTree {
            hierarchy_headings,
            sample_names,
            sample_index,
            node_names,
            leaves,
            sample_totals,
        })
    }

    /// Headings of the hierarchical levels, from the top level down.
    pub fn hierarchy_headings(&self) -> &[String] {
        &self.hierarchy_headings
    }

    pub fn sample_names(&self) -> &[String] {
        &self.sample_names
    }

    pub fn num_samples(&self) -> usize {
        self.sample_names.len()
    }

    /// Column index of a sample.
    pub fn sample_index(&self, name: &str) -> Option<usize> {
        self.sample_index.get(name).copied()
    }

    /// Total number of sequences in a sample.
    pub fn sample_total(&self, name: &str) -> Option<f64> {
        self.sample_index(name).map(|i| self.sample_totals[i])
    }

    /// Number of leaf features (unique full lineages).
    pub fn num_leaves(&self) -> usize {
        self.leaves.len()
    }

    /// Every leaf as its lineage (one name per hierarchical level) and its
    /// counts per sample, in depth-first order.
    pub fn rows(&self) -> impl Iterator<Item = (Vec<&str>, &[f64])> + '_ {
        self.leaves.iter().map(|leaf| {
            let lineage = leaf
                .path
                .iter()
                .map(|&n| self.node_names[n].as_str())
                .collect();
            (lineage, leaf.counts.as_slice())
        })
    }

    /// Depth of a hierarchical level: 0 for the entire sample (`None`), 1 for
    /// the first heading, and so on.
    pub fn level_depth(&self, heading: Option<&str>) -> Result<usize> {
        match heading {
            None => Ok(0),
            Some(ENTIRE_SAMPLE) => Ok(0),
            Some(h) => self
                .hierarchy_headings
                .iter()
                .position(|x| x == h)
                .map(|i| i + 1)
                .ok_or_else(|| StampError::new(format!("Unknown hierarchical level '{h}'."))),
        }
    }

    pub(crate) fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }

    pub(crate) fn node_name(&self, node: usize) -> &str {
        &self.node_names[node]
    }
}

/// Verify that every feature below the top level has a single parent.
fn check_hierarchy(lines: &[&str], num_levels: usize) -> Result<()> {
    let mut parents: Vec<HashMap<&str, &str>> = vec![HashMap::new(); num_levels];
    // The header row is included, as in the Python implementation; it is harmless.
    for line in lines {
        let cells = text::cells(line);
        let categories = &cells[..num_levels.min(cells.len())];
        for r in 1..categories.len() {
            let value = categories[r];
            if is_unclassified(value) {
                continue;
            }
            let parent = categories[r - 1];
            if is_unclassified(parent) {
                return Err(StampError::new(format!(
                    "Child {value} has an unclassified parent."
                )));
            }
            match parents[r].get(value) {
                None => {
                    parents[r].insert(value, parent);
                }
                Some(&known) if known != parent => {
                    return Err(StampError::new(format!(
                        "Data does not form a strict hierarchy. Child {value} has multiple parents (e.g., {known}, {parent})."
                    )));
                }
                Some(_) => {}
            }
        }
    }
    Ok(())
}

/// Incrementally builds the tree while rows are read.
struct Builder {
    names: Vec<String>,
    parents: Vec<usize>,
    children: Vec<Vec<usize>>,
    lookup: HashMap<(usize, String), usize>,
    counts: HashMap<usize, Vec<f64>>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            names: vec![ENTIRE_SAMPLE.to_string()],
            parents: vec![0],
            children: vec![Vec::new()],
            lookup: HashMap::new(),
            counts: HashMap::new(),
        }
    }

    fn child(&mut self, parent: usize, name: String) -> usize {
        if let Some(&id) = self.lookup.get(&(parent, name.clone())) {
            return id;
        }
        let id = self.names.len();
        self.names.push(name.clone());
        self.parents.push(parent);
        self.children.push(Vec::new());
        self.children[parent].push(id);
        self.lookup.insert((parent, name), id);
        id
    }

    fn add_counts(&mut self, leaf: usize, counts: Vec<f64>) {
        match self.counts.get_mut(&leaf) {
            Some(existing) => {
                for (a, b) in existing.iter_mut().zip(&counts) {
                    *a += b;
                }
            }
            None => {
                self.counts.insert(leaf, counts);
            }
        }
    }

    fn finish(mut self, num_levels: usize) -> (Vec<String>, Vec<Leaf>) {
        let mut leaves = Vec::with_capacity(self.counts.len());
        // Depth-first traversal, visiting children in order of first appearance.
        let mut stack: Vec<usize> = self.children[0].iter().rev().copied().collect();
        while let Some(node) = stack.pop() {
            if let Some(counts) = self.counts.remove(&node) {
                let mut path = Vec::with_capacity(num_levels);
                let mut current = node;
                while current != 0 {
                    path.push(current);
                    current = self.parents[current];
                }
                path.reverse();
                leaves.push(Leaf { path, counts });
            }
            stack.extend(self.children[node].iter().rev());
        }
        (self.names, leaves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = "Phylum\tGenus\tS1\tS2\tS3\n\
        Firmicutes\tBacillus\t1\t2\t3\n\
        Proteobacteria\tEscherichia\t4\t5\t6\n\
        \n\
        Firmicutes\tunclassified\t7\t8\t9\n\
        Firmicutes\tBacillus\t10\t20\t30\n\
        unclassified\tg__unclassified\t1\t1\t1\n";

    #[test]
    fn reads_headings_samples_and_totals() {
        let tree = ProfileTree::read(SMALL).unwrap();
        assert_eq!(tree.hierarchy_headings(), ["Phylum", "Genus"]);
        assert_eq!(tree.sample_names(), ["S1", "S2", "S3"]);
        assert_eq!(tree.sample_total("S1"), Some(23.0));
        assert_eq!(tree.sample_total("S3"), Some(49.0));
        assert_eq!(tree.sample_total("missing"), None);
    }

    #[test]
    fn merges_duplicate_rows_and_names_unclassified_after_parent() {
        let tree = ProfileTree::read(SMALL).unwrap();
        let rows: Vec<_> = tree
            .rows()
            .map(|(l, c)| (l.join("|"), c.to_vec()))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("Firmicutes|Bacillus".to_string(), vec![11.0, 22.0, 33.0]),
                (
                    "Firmicutes|Unclassified Firmicutes".to_string(),
                    vec![7.0, 8.0, 9.0]
                ),
                (
                    "Proteobacteria|Escherichia".to_string(),
                    vec![4.0, 5.0, 6.0]
                ),
                ("Unclassified|Unclassified".to_string(), vec![1.0, 1.0, 1.0]),
            ]
        );
    }

    #[test]
    fn accepts_old_mac_and_windows_line_endings() {
        let unix = ProfileTree::read(SMALL).unwrap();
        for ending in ["\r", "\r\n"] {
            let tree = ProfileTree::read(&SMALL.replace('\n', ending)).unwrap();
            assert_eq!(tree.num_leaves(), unix.num_leaves());
            assert_eq!(tree.sample_total("S2"), unix.sample_total("S2"));
        }
    }

    fn error(text: &str) -> String {
        ProfileTree::read(text).unwrap_err().0
    }

    #[test]
    fn reports_structural_problems() {
        assert_eq!(
            error("A\tS1\nx\t1\n"),
            "Profile file must contain at least two samples."
        );
        assert_eq!(
            error("S1\tS2\n1\t2\n"),
            "Profile file must contain a column indicating feature names."
        );
        assert!(error("A\tS1\tS2\n").contains("header row"));
        assert!(error("A\tS1\tS1\nx\t1\t2\n").contains("more than one sample named 'S1'"));
    }

    #[test]
    fn reports_unparsable_rows_by_line_number() {
        assert_eq!(
            error("A\tS1\tS2\nx\t1\t2\ny\t1\tz\n"),
            "Failed to correctly parse line: 3"
        );
        assert_eq!(
            error("A\tS1\tS2\nx\t1\t2\ny\t1\n"),
            "Failed to correctly parse line: 3"
        );
        assert_eq!(
            error("A\tS1\tS2\nx\t1\t2\n\ny\t1\t2\t3\n"),
            "Failed to correctly parse line: 4"
        );
    }

    #[test]
    fn rejects_non_strict_hierarchies() {
        assert_eq!(
            error("A\tB\tS1\tS2\np1\tc\t1\t2\np2\tc\t1\t2\n"),
            "Data does not form a strict hierarchy. Child c has multiple parents (e.g., p1, p2)."
        );
        assert_eq!(
            error("A\tB\tS1\tS2\nunclassified\tc\t1\t2\n"),
            "Child c has an unclassified parent."
        );
    }

    #[test]
    fn recognises_unclassified_markers() {
        assert!(is_unclassified("Unclassified"));
        assert!(is_unclassified("g__unclassified"));
        assert!(!is_unclassified("Unclassified Firmicutes"));
        assert!(!is_unclassified("gg__unclassified"));
    }
}
