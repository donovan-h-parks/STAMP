//! Profiles of coverage written by SingleM and Sandpiper, recognised by
//! their contents and loaded without first being converted: one file, several
//! files chosen together, or the files of a directory.

use super::{sandpiper, singlem, NamedText};
use crate::error::{Result, StampError};

/// The two formats of SingleM profiles that are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageFormat {
    /// A SingleM condensed taxonomic profile (`singlem pipe -p`), one or more
    /// samples per file.
    SingleM,
    /// A Sandpiper condensed profile (JSON), one sample per file.
    Sandpiper,
}

impl CoverageFormat {
    /// The format of a file, judged from its start (the first 64 KiB are
    /// enough), or `None` if it is neither.
    pub fn detect(start: &str) -> Option<CoverageFormat> {
        // Sandpiper first: it is JSON, which a SingleM header never is.
        if sandpiper::looks_like_sandpiper(start.as_bytes()) {
            Some(CoverageFormat::Sandpiper)
        } else if singlem::looks_like_singlem(start) {
            Some(CoverageFormat::SingleM)
        } else {
            None
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            CoverageFormat::SingleM => "SingleM",
            CoverageFormat::Sandpiper => "Sandpiper",
        }
    }

    /// Combine files of this format into the text of a STAMP profile.
    pub fn to_spf(self, files: &[NamedText<'_>]) -> Result<String> {
        match self {
            CoverageFormat::SingleM => singlem::to_spf(files),
            CoverageFormat::Sandpiper => sandpiper::to_spf(files),
        }
    }
}

/// Combine files that must all be SingleM profiles or all Sandpiper
/// profiles. `Ok(None)` if any file is neither, so that the caller can try
/// other formats or report that only these can be combined.
pub fn combine(files: &[NamedText<'_>]) -> Result<Option<(CoverageFormat, String)>> {
    let mut format = None;
    for file in files {
        let Some(this) = CoverageFormat::detect(file.text) else {
            return Ok(None);
        };
        match format {
            Some(first) if first != this => return Err(mixed()),
            _ => format = Some(this),
        }
    }
    let Some(format) = format else {
        return Ok(None);
    };
    Ok(Some((format, format.to_spf(files)?)))
}

fn mixed() -> StampError {
    StampError::new(
        "The files are a mixture of SingleM and Sandpiper profiles; load profiles of one \
         kind at a time.",
    )
}

/// A profile made from the files of a directory.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryProfile {
    pub format: CoverageFormat,
    pub spf: String,
    /// Number of files the profile was made from.
    pub used: usize,
    /// Names of the files that are not SingleM or Sandpiper profiles.
    pub skipped: Vec<String>,
}

/// Combine the SingleM or Sandpiper profiles among the files of a
/// directory, skipping files of any other kind. `directory` names the
/// directory in messages.
pub fn directory_to_spf(directory: &str, files: &[NamedText<'_>]) -> Result<DirectoryProfile> {
    let mut profiles = Vec::new();
    let mut skipped = Vec::new();
    let mut format = None;
    for file in files {
        match CoverageFormat::detect(file.text) {
            Some(this) => {
                if format.is_some_and(|first| first != this) {
                    return Err(mixed());
                }
                format = Some(this);
                profiles.push(*file);
            }
            None => skipped.push(file.name.to_string()),
        }
    }
    let Some(format) = format else {
        return Err(StampError::new(format!(
            "No SingleM or Sandpiper profiles were found in {directory}. A directory can be \
             loaded if it holds SingleM condensed profiles (singlem pipe -p) or Sandpiper \
             profiles (JSON); other files in it are skipped."
        )));
    };
    Ok(DirectoryProfile {
        format,
        spf: format.to_spf(&profiles)?,
        used: profiles.len(),
        skipped,
    })
}

/// True if a file in a directory should be looked at: not hidden (a name
/// starting with `.`, such as `.DS_Store`).
pub fn is_candidate(file_name: &str) -> bool {
    !file_name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named<'a>(name: &'a str, text: &'a str) -> NamedText<'a> {
        NamedText { name, text }
    }

    const SINGLEM_A: &str = "sample\tcoverage\ttaxonomy\nA\t1\tRoot; d__Bacteria\n";
    const SINGLEM_B: &str = "sample\tcoverage\ttaxonomy\nB\t2\tRoot; d__Archaea\n";
    const SANDPIPER: &str = r#"{"condensed": {"name": "", "size": 0, "children": [
        {"name": "d__Bacteria", "size": 3, "children": []}]}, "sample_name": "C"}"#;

    #[test]
    fn formats_are_told_apart() {
        assert_eq!(
            CoverageFormat::detect(SINGLEM_A),
            Some(CoverageFormat::SingleM)
        );
        assert_eq!(
            CoverageFormat::detect(SANDPIPER),
            Some(CoverageFormat::Sandpiper)
        );
        assert_eq!(CoverageFormat::detect("Phylum\tA\tB\nx\t1\t2\n"), None);
        assert_eq!(CoverageFormat::detect(""), None);
    }

    #[test]
    fn several_singlem_files_are_combined() {
        let files = [named("a.tsv", SINGLEM_A), named("b.tsv", SINGLEM_B)];
        let (format, spf) = combine(&files).unwrap().unwrap();
        assert_eq!(format, CoverageFormat::SingleM);
        assert!(spf.starts_with("Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tA\tB\n"));

        let other = [
            named("a.tsv", SINGLEM_A),
            named("x.spf", "Phylum\tA\nx\t1\n"),
        ];
        assert_eq!(combine(&other).unwrap(), None);
        let mixed = [named("a.tsv", SINGLEM_A), named("c.json", SANDPIPER)];
        assert!(combine(&mixed).unwrap_err().0.contains("mixture"));
        // The same sample in two files is refused.
        let twice = [named("a.tsv", SINGLEM_A), named("a2.tsv", SINGLEM_A)];
        assert!(combine(&twice)
            .unwrap_err()
            .0
            .contains("more than one file"));
    }

    #[test]
    fn a_directory_skips_other_files() {
        let files = [
            named("a.tsv", SINGLEM_A),
            named("notes.txt", "reads were trimmed\n"),
            named("b.tsv", SINGLEM_B),
        ];
        let profile = directory_to_spf("runs", &files).unwrap();
        assert_eq!(profile.format, CoverageFormat::SingleM);
        assert_eq!(profile.used, 2);
        assert_eq!(profile.skipped, ["notes.txt"]);
        assert!(profile.spf.lines().next().unwrap().ends_with("\tA\tB"));

        let none = directory_to_spf("runs", &[named("notes.txt", "x\n")]).unwrap_err();
        assert!(none
            .0
            .contains("No SingleM or Sandpiper profiles were found in runs"));
        let mixed = [named("a.tsv", SINGLEM_A), named("c.json", SANDPIPER)];
        assert!(directory_to_spf("runs", &mixed)
            .unwrap_err()
            .0
            .contains("mixture"));
        assert!(is_candidate("a.tsv"));
        assert!(!is_candidate(".DS_Store"));
    }
}
