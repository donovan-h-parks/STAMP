//! Converters from other tools' output to STAMP profile files (`.spf`).
//!
//! Every converter takes file contents and returns the text of a STAMP
//! profile, which can be saved or passed straight to
//! [`ProfileTree::read`](crate::ProfileTree::read). Rows are written in order
//! of first appearance in the input.

pub mod biom;
pub mod cog;
pub mod humann;
pub mod kraken;
pub mod lineage;
pub mod metaphlan;
pub mod mothur;
pub mod motus;
pub mod picrust2;
pub mod profiler;
pub mod sandpiper;
pub mod singlem;

use crate::text;

/// The contents of an input file together with its file name, for converters
/// that derive sample names from file names.
#[derive(Debug, Clone, Copy)]
pub struct NamedText<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

/// Extensions removed from a file name to make a sample name.
const EXTENSIONS: [&str; 19] = [
    "txt", "tsv", "tab", "csv", "report", "kreport", "kreport2", "k2report", "kraken", "kraken2",
    "bracken", "breport", "motus", "mpa", "profile", "out", "relab", "json", "gz",
];

/// Endings of file names that profilers add to the sample name.
const SUFFIXES: [&str; 13] = [
    "_profile",
    "_metaphlan",
    "_metaphlan4",
    "_motus",
    "_kraken",
    "_kraken2",
    "_report",
    "_kreport",
    "_genefamilies",
    "_pathabundance",
    "_pathcoverage",
    "_bracken",
    ".kraken2",
];

/// The sample name of a file that holds one sample and does not name it:
/// the file name without its directory, its extensions (`.tsv`,
/// `.kreport`, ...) and a profiler's usual suffix (`_profile`,
/// `_bracken_species`, `_genefamilies`, ...). `gut1_bracken_species.kreport`
/// becomes `gut1`.
pub fn sample_name_of(file_name: &str) -> String {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let mut name = base;
    while let Some((stem, extension)) = name.rsplit_once('.') {
        if stem.is_empty() || !EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()) {
            break;
        }
        name = stem;
    }
    // Bracken names its report `<input>_bracken_<level>`.
    if let Some(i) = name.rfind("_bracken_") {
        if !name[i + 9..].contains(['_', '.']) {
            name = &name[..i];
        }
    }
    for suffix in SUFFIXES {
        if let Some(stem) = name.strip_suffix(suffix) {
            if !stem.is_empty() {
                name = stem;
                break;
            }
        }
    }
    if name.is_empty() {
        base.to_string()
    } else {
        name.to_string()
    }
}

/// The complete lines at the start of a file, which may have been cut
/// anywhere: without a last line that may be incomplete.
pub(crate) fn complete_lines(start: &str) -> Vec<&str> {
    let start = start.strip_prefix('\u{feff}').unwrap_or(start);
    let mut lines = text::lines(start);
    if !start.ends_with(['\n', '\r']) && lines.len() > 1 {
        lines.pop();
    }
    lines
}

/// An error if a sample name is used twice, within a file or across files
/// (each slice holding the samples of one file).
pub(crate) fn check_unique<'a>(files: impl Iterator<Item = &'a [String]>) -> crate::Result<()> {
    let mut seen = std::collections::HashSet::new();
    for samples in files {
        for sample in samples {
            if !seen.insert(sample.as_str()) {
                return Err(crate::StampError::new(format!(
                    "Sample '{sample}' is in more than one file, or twice in one file."
                )));
            }
        }
    }
    Ok(())
}

/// Rows of an output profile, keyed and kept in order of first appearance.
pub(crate) struct Rows<V> {
    order: Vec<String>,
    rows: std::collections::HashMap<String, V>,
}

impl<V> Rows<V> {
    pub(crate) fn new() -> Self {
        Rows {
            order: Vec::new(),
            rows: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn entry(&mut self, key: &str, create: impl FnOnce() -> V) -> &mut V {
        if !self.rows.contains_key(key) {
            self.order.push(key.to_string());
            self.rows.insert(key.to_string(), create());
        }
        self.rows.get_mut(key).expect("row was just inserted")
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.order.iter().map(|k| (k.as_str(), &self.rows[k]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_names_of_profiles() {
        assert_eq!(sample_name_of("runs/gut1_bracken_species.kreport"), "gut1");
        assert_eq!(sample_name_of("gut1.kraken2.report.txt"), "gut1");
        assert_eq!(sample_name_of("C:\\x\\S2_profile.txt"), "S2");
        assert_eq!(sample_name_of("S3_genefamilies.tsv"), "S3");
        assert_eq!(sample_name_of("marine0.1.tsv"), "marine0.1");
        assert_eq!(sample_name_of("a.motus.relab"), "a");
        assert_eq!(sample_name_of("S3_genefamilies.tsv.gz"), "S3");
        assert_eq!(sample_name_of("gut1.k2report.gz"), "gut1");
        assert_eq!(sample_name_of(".tsv"), ".tsv");
        assert_eq!(sample_name_of("_profile.txt"), "_profile");
    }

    #[test]
    fn incomplete_last_lines_are_dropped() {
        assert_eq!(complete_lines("a\nb\nc"), ["a", "b"]);
        assert_eq!(complete_lines("a\nb\n"), ["a", "b"]);
        assert_eq!(complete_lines("\u{feff}a"), ["a"]);
    }
}
