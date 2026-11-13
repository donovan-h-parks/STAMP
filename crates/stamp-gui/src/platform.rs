//! File dialogs and file access. Kept apart from the interface because the
//! browser build needs a different implementation.

use stamp_core::import::{biom, cog, comet, mgrast, mothur, rita, NamedText};
use stamp_plot::export::{export, Format};
use stamp_plot::Figure;

/// A file chosen by the user.
pub struct OpenedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl OpenedFile {
    pub fn text(&self) -> String {
        stamp_core::text::decode(&self.bytes).into_owned()
    }
}

/// File formats that can be converted to a STAMP profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Biom,
    CoMet,
    MgRast,
    Mothur,
    Rita,
    ImgCog,
}

impl Source {
    pub const ALL: [Source; 6] = [
        Source::Biom,
        Source::CoMet,
        Source::MgRast,
        Source::Mothur,
        Source::Rita,
        Source::ImgCog,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Source::Biom => "BIOM table...",
            Source::CoMet => "CoMet profiles...",
            Source::MgRast => "MG-RAST profile...",
            Source::Mothur => "mothur files...",
            Source::Rita => "RITA profiles...",
            Source::ImgCog => "IMG/M COG profile (add COG categories)...",
        }
    }
}

/// Convert already-chosen input files to the text of a STAMP profile.
/// `files` are the main inputs; for mothur they are the taxonomy file, the
/// groups file and, optionally, the names file, in that order.
pub fn convert_files(source: Source, files: &[OpenedFile]) -> Result<String, String> {
    let first = files.first().ok_or("No file was chosen.")?;
    let texts: Vec<String> = files.iter().map(OpenedFile::text).collect();
    let named: Vec<NamedText<'_>> = files
        .iter()
        .zip(&texts)
        .map(|(file, text)| NamedText {
            name: &file.name,
            text,
        })
        .collect();
    let result = match source {
        Source::Biom => biom::to_spf(&first.bytes, None),
        Source::CoMet => comet::to_spf(&named),
        Source::MgRast => mgrast::MgRastProfile::parse(&texts[0]).and_then(|p| p.to_spf()),
        Source::Rita => rita::to_spf(&named, &rita::GROUPS),
        Source::ImgCog => cog::append_categories(&texts[0], cog::MultiCodeTreatment::AsFeature),
        Source::Mothur => {
            let groups = texts
                .get(1)
                .ok_or("mothur conversion needs a taxonomy file and a groups file.")?;
            mothur::to_spf(&texts[0], groups, texts.get(2).map(String::as_str))
        }
    };
    result.map_err(|e| e.to_string())
}

/// Encode a figure in the format implied by a file name.
pub fn encode_plot(figure: &Figure, file_name: &str) -> Result<Vec<u8>, String> {
    let format = Format::from_file_name(file_name)
        .ok_or("Use a file name ending in .png, .svg or .pdf to choose the image format.")?;
    export(figure, format, 300.0).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::path::Path;

    fn read(path: &Path) -> Option<OpenedFile> {
        let bytes = std::fs::read(path).ok()?;
        let name = path.file_name()?.to_string_lossy().into_owned();
        Some(OpenedFile { name, bytes })
    }

    fn dialog(title: &str, filters: &[(&str, &[&str])]) -> rfd::FileDialog {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }
        dialog.add_filter("All files", &["*"])
    }

    /// Ask the user for one file to open.
    pub fn open_file(title: &str, filters: &[(&str, &[&str])]) -> Option<OpenedFile> {
        read(&dialog(title, filters).pick_file()?)
    }

    /// Ask the user for several files to open.
    pub fn open_files(title: &str, filters: &[(&str, &[&str])]) -> Vec<OpenedFile> {
        let paths = dialog(title, filters).pick_files().unwrap_or_default();
        paths.iter().filter_map(|p| read(p)).collect()
    }

    fn save(
        title: &str,
        default_name: &str,
        filters: &[(&str, &[&str])],
        bytes: impl FnOnce(&str) -> Result<Vec<u8>, String>,
    ) -> Result<Option<String>, String> {
        let mut dialog = rfd::FileDialog::new()
            .set_title(title)
            .set_file_name(default_name);
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }
        let Some(path) = dialog.save_file() else {
            return Ok(None);
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        std::fs::write(&path, bytes(&name)?)
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(Some(name))
    }

    /// Ask where to save some text, and save it.
    pub fn save_text(default_name: &str, text: &str) -> Result<(), String> {
        save(
            "Save",
            default_name,
            &[("Tab-separated values", &["tsv"])],
            |_| Ok(text.as_bytes().to_vec()),
        )
        .map(|_| ())
    }

    /// Ask where to save the plot, and save it in the format of the chosen
    /// file name.
    pub fn save_plot(figure: &Figure) -> Result<(), String> {
        let filters: [(&str, &[&str]); 3] = [
            ("PNG image", &["png"]),
            ("SVG image", &["svg"]),
            ("PDF document", &["pdf"]),
        ];
        save("Save plot", "plot.png", &filters, |name| {
            encode_plot(figure, name)
        })
        .map(|_| ())
    }

    /// Ask for the files of another tool, convert them, and save the STAMP
    /// profile. Returns the saved file's name and contents, or `None` if the
    /// user cancelled.
    pub fn convert(source: Source) -> Result<Option<(String, String)>, String> {
        let files = match source {
            Source::Biom => open_file("BIOM table", &[("BIOM table", &["biom", "gz"])])
                .into_iter()
                .collect(),
            Source::MgRast => open_file("MG-RAST profile", &[("MG-RAST profile", &["tsv"])])
                .into_iter()
                .collect(),
            Source::ImgCog => open_file("IMG/M COG profile", &[("IMG/M profile", &["xls", "tsv"])])
                .into_iter()
                .collect(),
            Source::CoMet => open_files(
                "CoMet profiles, one per sample",
                &[("CoMet profile", &["txt"])],
            ),
            Source::Rita => open_files(
                "RITA profiles, one per sample",
                &[("RITA profile", &["txt"])],
            ),
            Source::Mothur => {
                let Some(taxonomy) =
                    open_file("mothur taxonomy file", &[("Taxonomy file", &["taxonomy"])])
                else {
                    return Ok(None);
                };
                let Some(groups) = open_file("mothur groups file", &[("Groups file", &["groups"])])
                else {
                    return Ok(None);
                };
                let mut files = vec![taxonomy, groups];
                // The names file is optional; cancelling skips it.
                files.extend(open_file(
                    "mothur names file (optional)",
                    &[("Names file", &["names"])],
                ));
                files
            }
        };
        if files.is_empty() {
            return Ok(None);
        }
        let spf = convert_files(source, &files)?;
        let saved = save(
            "Save STAMP profile",
            "profile.spf",
            &[("STAMP profile", &["spf"])],
            |_| Ok(spf.clone().into_bytes()),
        )?;
        Ok(saved.map(|name| (name, spf)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

/// The browser build gets its own file handling; until then these do nothing.
#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;

    const UNAVAILABLE: &str = "File access is not available in the browser build yet.";

    pub fn open_file(_title: &str, _filters: &[(&str, &[&str])]) -> Option<OpenedFile> {
        None
    }

    pub fn save_text(_default_name: &str, _text: &str) -> Result<(), String> {
        Err(UNAVAILABLE.to_string())
    }

    pub fn save_plot(_figure: &Figure) -> Result<(), String> {
        Err(UNAVAILABLE.to_string())
    }

    pub fn convert(_source: Source) -> Result<Option<(String, String)>, String> {
        Err(UNAVAILABLE.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, text: &str) -> OpenedFile {
        OpenedFile {
            name: name.to_string(),
            bytes: text.as_bytes().to_vec(),
        }
    }

    #[test]
    fn converts_chosen_files() {
        let comet = [
            file("profilePfam_A.fa.txt", "PF1 (Kinase): 5\n"),
            file("profilePfam_B.fa.txt", "PF1 (Kinase): 2\n"),
        ];
        assert_eq!(
            convert_files(Source::CoMet, &comet).unwrap(),
            "Category\tA\tB\nKinase\t5.0\t2.0\n"
        );

        let mothur = [
            file("x.taxonomy", "s1\tBacteria;Firmicutes;\n"),
            file("x.groups", "s1\tgut\ns2\tsoil\n"),
        ];
        assert!(convert_files(Source::Mothur, &mothur)
            .unwrap()
            .starts_with("Level 1\tLevel 2\tgut\tsoil\n"));
        assert!(convert_files(Source::Mothur, &mothur[..1])
            .unwrap_err()
            .contains("groups file"));

        assert!(
            convert_files(Source::MgRast, &[file("x.tsv", "not mg-rast\n")])
                .unwrap_err()
                .contains("MG-RAST")
        );
        assert_eq!(
            convert_files(Source::Biom, &[]).unwrap_err(),
            "No file was chosen."
        );
        for source in Source::ALL {
            assert!(source.name().ends_with("..."));
        }
    }

    #[test]
    fn plot_format_follows_the_file_name() {
        let figure = Figure::message("test");
        assert!(encode_plot(&figure, "a.svg").unwrap().starts_with(b"<svg"));
        assert!(encode_plot(&figure, "a.png")
            .unwrap()
            .starts_with(b"\x89PNG"));
        assert!(encode_plot(&figure, "a.pdf").unwrap().starts_with(b"%PDF"));
        assert!(encode_plot(&figure, "a.eps")
            .unwrap_err()
            .contains(".png, .svg or .pdf"));
    }
}
