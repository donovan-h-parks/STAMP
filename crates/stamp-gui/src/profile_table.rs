//! The Profile table: the values of the loaded profile, one row per feature
//! at the profile level and one column per sample. It shows what the tests
//! and plots are given (the same profile level, parent level and treatment
//! of unclassified features), so the numbers behind a result can be read.
//!
//! This module holds the table and its logic; `app.rs` draws it.

use stamp_core::profile::Unclassified;
use stamp_core::profile_tree::ProfileTree;

/// What the cells of the table show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CellValues {
    /// The feature as a percentage of its parent category in the sample:
    /// the relative frequency the tests of groups compare.
    #[default]
    Percent,
    /// The numbers of the profile file (counts, or coverage), summed over
    /// the rows of the file that belong to the feature.
    Loaded,
}

impl CellValues {
    pub const ALL: [CellValues; 2] = [CellValues::Percent, CellValues::Loaded];

    pub fn name(self) -> &'static str {
        match self {
            CellValues::Percent => "Proportion (%)",
            CellValues::Loaded => "Values in the file",
        }
    }
}

/// One feature of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Names from the top hierarchical level down to the profile level; the
    /// last is the feature.
    pub hierarchy: Vec<String>,
    /// The value in each sample, in the order of [`ProfileTable::samples`].
    pub values: Vec<f64>,
}

impl Row {
    pub fn feature(&self) -> &str {
        self.hierarchy.last().map(String::as_str).unwrap_or("")
    }
}

/// The profile of a set of samples at one hierarchical level.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileTable {
    /// Headings of the hierarchical levels down to the profile level.
    pub levels: Vec<String>,
    pub samples: Vec<String>,
    /// Features in the order of the profile file.
    pub rows: Vec<Row>,
    pub values: CellValues,
}

impl ProfileTable {
    /// The table of `samples` at `profile_level`, with percentages taken
    /// against `parent_level` (`None` for the entire sample).
    pub fn build(
        tree: &ProfileTree,
        samples: &[&str],
        parent_level: Option<&str>,
        profile_level: &str,
        unclassified: Unclassified,
        values: CellValues,
    ) -> Result<ProfileTable, String> {
        let profile = tree
            .profile(samples, parent_level, profile_level, unclassified)
            .map_err(|e| e.to_string())?;
        let rows = profile
            .features()
            .iter()
            .map(|feature| Row {
                hierarchy: feature.hierarchy.clone(),
                values: match values {
                    CellValues::Percent => profile.percentages(feature),
                    CellValues::Loaded => feature.counts.clone(),
                },
            })
            .collect();
        Ok(ProfileTable {
            levels: profile.hierarchy_headings.clone(),
            samples: profile.sample_names.clone(),
            rows,
            values,
        })
    }

    /// The number of columns that hold names, before the samples: every
    /// level of the hierarchy, or the feature alone.
    pub fn name_columns(&self, higher_levels: bool) -> usize {
        if higher_levels {
            self.levels.len().max(1)
        } else {
            1
        }
    }

    /// The headings of the columns: the levels shown, then the samples.
    pub fn headings(&self, higher_levels: bool) -> Vec<&str> {
        let names = self.name_columns(higher_levels);
        let skipped = self.levels.len().saturating_sub(names);
        let mut headings: Vec<&str> = self.levels[skipped..].iter().map(String::as_str).collect();
        if headings.is_empty() {
            headings.push("Feature");
        }
        headings.extend(self.samples.iter().map(String::as_str));
        headings
    }

    /// The name in a name column of a row.
    pub fn name(&self, row: usize, column: usize, higher_levels: bool) -> &str {
        let hierarchy = &self.rows[row].hierarchy;
        let skipped = hierarchy
            .len()
            .saturating_sub(self.name_columns(higher_levels));
        hierarchy
            .get(skipped + column)
            .map(String::as_str)
            .unwrap_or("")
    }

    /// The rows to show, in order: those with `search` in one of their
    /// names (any level, whatever the case of the letters), sorted by a
    /// column (its number among [`ProfileTable::headings`], and whether
    /// descending). Names sort alphabetically and samples by value; rows
    /// that compare equal keep the order of the file.
    pub fn order(
        &self,
        sort: Option<(usize, bool)>,
        higher_levels: bool,
        search: &str,
    ) -> Vec<usize> {
        let search = search.trim().to_lowercase();
        let mut order: Vec<usize> = (0..self.rows.len())
            .filter(|&row| {
                search.is_empty()
                    || self.rows[row]
                        .hierarchy
                        .iter()
                        .any(|name| name.to_lowercase().contains(&search))
            })
            .collect();
        let Some((column, descending)) = sort else {
            return order;
        };
        let names = self.name_columns(higher_levels);
        if column < names {
            let keys: Vec<String> = (0..self.rows.len())
                .map(|row| self.name(row, column, higher_levels).to_lowercase())
                .collect();
            order.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
        } else if column - names < self.samples.len() {
            let sample = column - names;
            order.sort_by(|&a, &b| {
                self.rows[a].values[sample].total_cmp(&self.rows[b].values[sample])
            });
        }
        if descending {
            order.reverse();
        }
        order
    }

    /// The table as tab-separated text, with every level of the hierarchy
    /// and the values in full: the rows of `order`.
    pub fn to_tsv(&self, order: &[usize]) -> String {
        let mut text = self.headings(true).join("\t");
        text.push('\n');
        for &row in order {
            let row = &self.rows[row];
            let mut cells: Vec<String> = row.hierarchy.clone();
            cells.extend(row.values.iter().map(|value| value.to_string()));
            text.push_str(&cells.join("\t"));
            text.push('\n');
        }
        text
    }
}

/// A value as a cell shows it: whole numbers in full, others to four
/// decimal places (or in scientific notation when smaller than that shows).
pub fn cell_text(value: f64) -> String {
    let magnitude = value.abs();
    if value == 0.0 {
        "0".to_string()
    } else if !value.is_finite() {
        value.to_string()
    } else if value.fract() == 0.0 && magnitude < 1e15 {
        format!("{value:.0}")
    } else if magnitude < 1e-4 {
        format!("{value:.2e}")
    } else {
        let text = format!("{value:.4}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = "Phylum\tFamily\tS1\tS2\tS3\n\
        Firmicutes\tLachnospiraceae\t30\t10\t0\n\
        Firmicutes\tRuminococcaceae\t10\t10\t5\n\
        Bacteroidota\tBacteroidaceae\t60\t20\t15\n";

    fn table(level: &str, parent: Option<&str>, values: CellValues) -> ProfileTable {
        let tree = ProfileTree::read(PROFILE).unwrap();
        ProfileTable::build(
            &tree,
            &["S1", "S2", "S3"],
            parent,
            level,
            Unclassified::Retain,
            values,
        )
        .unwrap()
    }

    #[test]
    fn rows_are_the_features_of_the_level_with_their_values() {
        let loaded = table("Family", None, CellValues::Loaded);
        assert_eq!(loaded.levels, ["Phylum", "Family"]);
        assert_eq!(loaded.samples, ["S1", "S2", "S3"]);
        assert_eq!(loaded.rows.len(), 3);
        assert_eq!(loaded.rows[0].feature(), "Lachnospiraceae");
        assert_eq!(loaded.rows[0].values, [30.0, 10.0, 0.0]);

        // Percentages of the entire sample, and of the parent level.
        let percent = table("Family", None, CellValues::Percent);
        assert_eq!(percent.rows[0].values, [30.0, 25.0, 0.0]);
        assert_eq!(percent.rows[2].values, [60.0, 50.0, 75.0]);
        let of_phylum = table("Family", Some("Phylum"), CellValues::Percent);
        assert_eq!(of_phylum.rows[0].values, [75.0, 50.0, 0.0]);
        assert_eq!(of_phylum.rows[2].values, [100.0, 100.0, 100.0]);

        // A higher level sums the rows of the file beneath each feature.
        let phyla = table("Phylum", None, CellValues::Loaded);
        assert_eq!(phyla.levels, ["Phylum"]);
        assert_eq!(phyla.rows[0].feature(), "Firmicutes");
        assert_eq!(phyla.rows[0].values, [40.0, 20.0, 5.0]);
    }

    #[test]
    fn columns_are_the_names_then_the_samples() {
        let table = table("Family", None, CellValues::Loaded);
        assert_eq!(table.headings(false), ["Family", "S1", "S2", "S3"]);
        assert_eq!(table.headings(true), ["Phylum", "Family", "S1", "S2", "S3"]);
        assert_eq!(table.name(0, 0, false), "Lachnospiraceae");
        assert_eq!(table.name(0, 0, true), "Firmicutes");
        assert_eq!(table.name(0, 1, true), "Lachnospiraceae");
    }

    #[test]
    fn rows_are_sorted_and_searched() {
        let table = table("Family", None, CellValues::Loaded);
        let features = |order: Vec<usize>| -> Vec<&str> {
            order.iter().map(|&row| table.rows[row].feature()).collect()
        };
        // Unsorted: the order of the file.
        assert_eq!(table.order(None, false, ""), [0, 1, 2]);
        // By name, and by a sample's values (column 1 is S1; with the
        // higher levels shown it is the Family column).
        assert_eq!(
            features(table.order(Some((0, false)), false, "")),
            ["Bacteroidaceae", "Lachnospiraceae", "Ruminococcaceae"]
        );
        assert_eq!(
            features(table.order(Some((1, true)), false, "")),
            ["Bacteroidaceae", "Lachnospiraceae", "Ruminococcaceae"]
        );
        assert_eq!(
            features(table.order(Some((3, false)), false, "")),
            ["Lachnospiraceae", "Ruminococcaceae", "Bacteroidaceae"]
        );
        assert_eq!(
            features(table.order(Some((1, true)), true, "")),
            ["Ruminococcaceae", "Lachnospiraceae", "Bacteroidaceae"]
        );
        // The search looks at every level, whatever the case.
        assert_eq!(
            features(table.order(None, false, "firmi")),
            ["Lachnospiraceae", "Ruminococcaceae"]
        );
        assert_eq!(
            features(table.order(None, false, " COCC ")),
            ["Ruminococcaceae"]
        );
        assert!(table.order(None, false, "absent").is_empty());
    }

    #[test]
    fn the_copied_text_has_every_level_and_full_values() {
        let table = table("Family", None, CellValues::Percent);
        let text = table.to_tsv(&[2, 0]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "Phylum\tFamily\tS1\tS2\tS3");
        assert_eq!(lines[1], "Bacteroidota\tBacteroidaceae\t60\t50\t75");
        assert_eq!(lines[2], "Firmicutes\tLachnospiraceae\t30\t25\t0");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn values_are_written_briefly() {
        assert_eq!(cell_text(0.0), "0");
        assert_eq!(cell_text(1234.0), "1234");
        assert_eq!(cell_text(12.5), "12.5");
        assert_eq!(cell_text(0.123456), "0.1235");
        assert_eq!(cell_text(0.00001234), "1.23e-5");
    }
}
