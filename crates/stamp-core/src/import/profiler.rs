//! Profiles written by taxonomic and functional profilers (SingleM,
//! Sandpiper, mOTUs, MetaPhlAn, Kraken, Bracken and HUMAnN), recognised by
//! their contents and loaded without first being converted: one file,
//! several files chosen together, or the files of a directory. See
//! `docs/PROFILER_FORMATS.md`.

use super::{humann, kraken, metaphlan, motus, sandpiper, singlem, NamedText};
use crate::error::{Result, StampError};

/// How much of the start of a file is looked at to tell its format.
pub const DETECTION_BYTES: usize = 65536;

/// The formats that are recognised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProfilerFormat {
    /// A SingleM condensed taxonomic profile (`singlem pipe -p`), one or more
    /// samples per file.
    SingleM,
    /// A Sandpiper condensed profile (JSON), one sample per file.
    Sandpiper,
    /// A mOTUs profile or merged table.
    Motus,
    /// A MetaPhlAn profile or merged table.
    MetaPhlAn,
    /// A Kraken 2 (or Kraken 1) report.
    Kraken,
    /// A Kraken report in MetaPhlAn's style (`--use-mpa-style`).
    KrakenMpa,
    /// A report written by Bracken (`-w`), in Kraken's layout.
    BrackenReport,
    /// A Bracken abundance table (`.bracken`) or combined tables.
    Bracken,
    /// HUMAnN gene families (`_genefamilies.tsv`).
    HumannGeneFamilies,
    /// HUMAnN pathway abundance (`_pathabundance.tsv`).
    HumannPathAbundance,
    /// HUMAnN pathway coverage (`_pathcoverage.tsv`).
    HumannPathCoverage,
}

impl ProfilerFormat {
    pub const ALL: [ProfilerFormat; 11] = [
        ProfilerFormat::SingleM,
        ProfilerFormat::Sandpiper,
        ProfilerFormat::Motus,
        ProfilerFormat::MetaPhlAn,
        ProfilerFormat::Kraken,
        ProfilerFormat::KrakenMpa,
        ProfilerFormat::BrackenReport,
        ProfilerFormat::Bracken,
        ProfilerFormat::HumannGeneFamilies,
        ProfilerFormat::HumannPathAbundance,
        ProfilerFormat::HumannPathCoverage,
    ];

    /// The format of a file, judged from its start (the first 64 KiB are
    /// enough), or `None` if it is none of these.
    pub fn detect(start: &str) -> Option<ProfilerFormat> {
        let mut end = start.len().min(DETECTION_BYTES);
        while !start.is_char_boundary(end) {
            end -= 1;
        }
        let start = &start[..end];
        // Sandpiper first: it is JSON, which no other header is.
        if sandpiper::looks_like_sandpiper(start.as_bytes()) {
            Some(ProfilerFormat::Sandpiper)
        } else if singlem::looks_like_singlem(start) {
            Some(ProfilerFormat::SingleM)
        } else if let Some(kind) = humann::table_kind(start) {
            Some(match kind {
                humann::TableKind::GeneFamilies => ProfilerFormat::HumannGeneFamilies,
                humann::TableKind::PathAbundance => ProfilerFormat::HumannPathAbundance,
                humann::TableKind::PathCoverage => ProfilerFormat::HumannPathCoverage,
            })
        } else if motus::looks_like_motus(start) {
            Some(ProfilerFormat::Motus)
        } else if metaphlan::looks_like_metaphlan(start) {
            Some(ProfilerFormat::MetaPhlAn)
        } else if kraken::looks_like_bracken_table(start) {
            Some(ProfilerFormat::Bracken)
        } else if let Some(kind) = kraken::report_kind(start) {
            Some(match kind {
                kraken::ReportKind::Kraken => ProfilerFormat::Kraken,
                kraken::ReportKind::Bracken => ProfilerFormat::BrackenReport,
            })
        } else if kraken::looks_like_mpa(start) {
            Some(ProfilerFormat::KrakenMpa)
        } else {
            None
        }
    }

    /// The name of the format, as in "3 Kraken reports".
    pub fn name(self) -> &'static str {
        match self {
            ProfilerFormat::SingleM => "SingleM",
            ProfilerFormat::Sandpiper => "Sandpiper",
            ProfilerFormat::Motus => "mOTUs",
            ProfilerFormat::MetaPhlAn => "MetaPhlAn",
            ProfilerFormat::Kraken => "Kraken",
            ProfilerFormat::KrakenMpa => "Kraken (MPA-style)",
            ProfilerFormat::BrackenReport => "Bracken report",
            ProfilerFormat::Bracken => "Bracken",
            ProfilerFormat::HumannGeneFamilies => "HUMAnN gene families",
            ProfilerFormat::HumannPathAbundance => "HUMAnN pathway abundance",
            ProfilerFormat::HumannPathCoverage => "HUMAnN pathway coverage",
        }
    }

    /// The name used on the command line (`--profile-format`).
    pub fn key(self) -> &'static str {
        match self {
            ProfilerFormat::SingleM => "singlem",
            ProfilerFormat::Sandpiper => "sandpiper",
            ProfilerFormat::Motus => "motus",
            ProfilerFormat::MetaPhlAn => "metaphlan",
            ProfilerFormat::Kraken => "kraken",
            ProfilerFormat::KrakenMpa => "kraken-mpa",
            ProfilerFormat::BrackenReport => "bracken-report",
            ProfilerFormat::Bracken => "bracken",
            ProfilerFormat::HumannGeneFamilies => "humann-genefamilies",
            ProfilerFormat::HumannPathAbundance => "humann-pathabundance",
            ProfilerFormat::HumannPathCoverage => "humann-pathcoverage",
        }
    }

    /// The format of a `--profile-format` name.
    pub fn from_key(key: &str) -> Option<ProfilerFormat> {
        ProfilerFormat::ALL.into_iter().find(|f| f.key() == key)
    }

    /// Combine files of this format into the text of a STAMP profile.
    pub fn to_spf(self, files: &[NamedText<'_>]) -> Result<String> {
        match self {
            ProfilerFormat::SingleM => singlem::to_spf(files),
            ProfilerFormat::Sandpiper => sandpiper::to_spf(files),
            ProfilerFormat::Motus => motus::to_spf(files),
            ProfilerFormat::MetaPhlAn => metaphlan::to_spf(files),
            ProfilerFormat::Kraken | ProfilerFormat::BrackenReport => kraken::reports_to_spf(files),
            ProfilerFormat::KrakenMpa => kraken::mpa_to_spf(files),
            ProfilerFormat::Bracken => kraken::bracken_tables_to_spf(files),
            ProfilerFormat::HumannGeneFamilies => {
                humann::to_spf(files, humann::TableKind::GeneFamilies)
            }
            ProfilerFormat::HumannPathAbundance => {
                humann::to_spf(files, humann::TableKind::PathAbundance)
            }
            ProfilerFormat::HumannPathCoverage => {
                humann::to_spf(files, humann::TableKind::PathCoverage)
            }
        }
    }
}

/// Combine files that must all be of one of these formats. `Ok(None)` if
/// any file is of none of them, so that the caller can try other formats or
/// report that only these can be combined.
pub fn combine(files: &[NamedText<'_>]) -> Result<Option<(ProfilerFormat, String)>> {
    let mut format = None;
    for file in files {
        let Some(this) = ProfilerFormat::detect(file.text) else {
            return Ok(None);
        };
        match format {
            Some(first) if first != this => return Err(mixed(&[first, this])),
            _ => format = Some(this),
        }
    }
    let Some(format) = format else {
        return Ok(None);
    };
    Ok(Some((format, format.to_spf(files)?)))
}

/// The error for files of several formats.
fn mixed(formats: &[ProfilerFormat]) -> StampError {
    let names: Vec<&str> = formats.iter().map(|f| f.name()).collect();
    let list = match names.as_slice() {
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => String::new(),
    };
    StampError::new(format!(
        "The files are a mixture of {list} profiles; load profiles of one kind at a time."
    ))
}

/// The formats of the files of a directory that are profiles, each with
/// the number of its files, in the order of [`ProfilerFormat::ALL`].
pub fn formats_of(files: &[NamedText<'_>]) -> Vec<(ProfilerFormat, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for file in files {
        if let Some(format) = ProfilerFormat::detect(file.text) {
            *counts.entry(format).or_insert(0) += 1;
        }
    }
    counts.into_iter().collect()
}

/// A profile made from the files of a directory.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryProfile {
    pub format: ProfilerFormat,
    pub spf: String,
    /// Number of files the profile was made from.
    pub used: usize,
    /// Names of the files that were not used: not profiles, or profiles of
    /// another format than the one chosen.
    pub skipped: Vec<String>,
}

/// Combine the profiles among the files of a directory, skipping files of
/// any other kind. With `only`, files of other formats are skipped too;
/// without it, a directory holding profiles of several formats is refused.
/// `directory` names the directory in messages.
pub fn directory_to_spf(
    directory: &str,
    files: &[NamedText<'_>],
    only: Option<ProfilerFormat>,
) -> Result<DirectoryProfile> {
    let formats = formats_of(files);
    let format = match (only, formats.as_slice()) {
        (Some(format), _) => format,
        (None, [(format, _)]) => *format,
        (None, []) => {
            return Err(StampError::new(format!(
                "No profiles were found in {directory}. A directory can be loaded if it \
                 holds profiles written by SingleM, Sandpiper, mOTUs, MetaPhlAn, Kraken, \
                 Bracken or HUMAnN; other files in it are skipped."
            )))
        }
        (None, several) => {
            let kinds: Vec<String> = several
                .iter()
                .map(|(f, n)| format!("{n} {}", f.name()))
                .collect();
            return Err(StampError::new(format!(
                "{directory} holds profiles of several kinds ({}); choose the kind to \
                 load.",
                kinds.join(", ")
            )));
        }
    };
    let mut profiles = Vec::new();
    let mut skipped = Vec::new();
    for file in files {
        if ProfilerFormat::detect(file.text) == Some(format) {
            profiles.push(*file);
        } else {
            skipped.push(file.name.to_string());
        }
    }
    if profiles.is_empty() {
        return Err(StampError::new(format!(
            "No {} profiles were found in {directory}.",
            format.name()
        )));
    }
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
    const KRAKEN: &str = " 10.00\t10\t10\tU\t0\tunclassified\n 90.00\t90\t90\tR\t1\troot\n";
    const HUMANN: &str = "# Gene Family\tS_Abundance-RPKs\nUNMAPPED\t1.0\nUniRef90_X\t2.0\n";

    #[test]
    fn formats_are_told_apart() {
        let cases = [
            (SINGLEM_A, ProfilerFormat::SingleM),
            (SANDPIPER, ProfilerFormat::Sandpiper),
            (
                "#mOTUs2_clade\tA\nk__Bacteria\t1.0\n",
                ProfilerFormat::Motus,
            ),
            (
                "#mpa_vJan21_CHOCOPhlAnSGB_202103\n",
                ProfilerFormat::MetaPhlAn,
            ),
            (KRAKEN, ProfilerFormat::Kraken),
            ("100.00\t9\t9\tR\t1\troot\n", ProfilerFormat::BrackenReport),
            ("d__Bacteria\t9\n", ProfilerFormat::KrakenMpa),
            (
                "name\ttaxonomy_id\ttaxonomy_lvl\tkraken_assigned_reads\n",
                ProfilerFormat::Bracken,
            ),
            (HUMANN, ProfilerFormat::HumannGeneFamilies),
            (
                "# Pathway\tS_Abundance\n",
                ProfilerFormat::HumannPathAbundance,
            ),
            (
                "# Pathway\tS_Coverage\n",
                ProfilerFormat::HumannPathCoverage,
            ),
        ];
        for (text, format) in cases {
            assert_eq!(ProfilerFormat::detect(text), Some(format), "{text}");
            assert_eq!(ProfilerFormat::from_key(format.key()), Some(format));
        }
        assert_eq!(cases.len(), ProfilerFormat::ALL.len());
        for other in [
            "Phylum\tA\tB\nx\t1\t2\n",
            "",
            "# just a comment\n",
            "{\"id\": 1}",
        ] {
            assert_eq!(ProfilerFormat::detect(other), None, "{other}");
        }
    }

    #[test]
    fn files_of_one_format_are_combined() {
        let files = [named("a.tsv", SINGLEM_A), named("b.tsv", SINGLEM_B)];
        let (format, spf) = combine(&files).unwrap().unwrap();
        assert_eq!(format, ProfilerFormat::SingleM);
        assert!(spf.starts_with("Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tA\tB\n"));

        let other = [
            named("a.tsv", SINGLEM_A),
            named("x.spf", "Phylum\tA\nx\t1\n"),
        ];
        assert_eq!(combine(&other).unwrap(), None);
        let mixed = [named("a.tsv", SINGLEM_A), named("k.k2report", KRAKEN)];
        assert_eq!(
            combine(&mixed).unwrap_err().0,
            "The files are a mixture of SingleM and Kraken profiles; load profiles of one kind \
             at a time."
        );
        let twice = [named("a.tsv", SINGLEM_A), named("a2.tsv", SINGLEM_A)];
        assert!(combine(&twice)
            .unwrap_err()
            .0
            .contains("more than one file"));
    }

    #[test]
    fn a_directory_skips_other_files_and_may_need_a_choice() {
        let files = [
            named("a.tsv", SINGLEM_A),
            named("notes.txt", "reads were trimmed\n"),
            named("b.tsv", SINGLEM_B),
        ];
        let profile = directory_to_spf("runs", &files, None).unwrap();
        assert_eq!(profile.format, ProfilerFormat::SingleM);
        assert_eq!(profile.used, 2);
        assert_eq!(profile.skipped, ["notes.txt"]);
        assert!(profile.spf.lines().next().unwrap().ends_with("\tA\tB"));

        let none = directory_to_spf("runs", &[named("notes.txt", "x\n")], None).unwrap_err();
        assert!(none.0.contains("No profiles were found in runs"));

        let several = [
            named("a.tsv", SINGLEM_A),
            named("b.tsv", SINGLEM_B),
            named("k.k2report", KRAKEN),
        ];
        assert_eq!(
            formats_of(&several),
            [(ProfilerFormat::SingleM, 2), (ProfilerFormat::Kraken, 1)]
        );
        assert_eq!(
            directory_to_spf("runs", &several, None).unwrap_err().0,
            "runs holds profiles of several kinds (2 SingleM, 1 Kraken); choose the kind to load."
        );
        let chosen = directory_to_spf("runs", &several, Some(ProfilerFormat::SingleM)).unwrap();
        assert_eq!(
            (chosen.used, chosen.skipped.as_slice()),
            (2, &["k.k2report".to_string()][..])
        );
        let missing = directory_to_spf("runs", &several, Some(ProfilerFormat::Motus)).unwrap_err();
        assert_eq!(missing.0, "No mOTUs profiles were found in runs.");

        assert!(is_candidate("a.tsv"));
        assert!(!is_candidate(".DS_Store"));
    }

    #[test]
    fn only_the_start_of_a_file_is_looked_at() {
        // A character of two bytes across the end of what is looked at.
        let mut long = String::from(HUMANN);
        while long.len() < DETECTION_BYTES - 1 {
            long.push('x');
        }
        long.truncate(DETECTION_BYTES - 1);
        long.push('ä');
        long.push_str("\t1.0\n");
        assert_eq!(
            ProfilerFormat::detect(&long),
            Some(ProfilerFormat::HumannGeneFamilies)
        );
    }
}
