//! File access: opening, converting and saving files.
//!
//! The interface asks for something with [`Platform::request`] and later
//! receives the outcome from [`Platform::poll`]. On the desktop the request
//! is answered at once with native dialogs. In the browser, files come from
//! the browser's file picker and are saved as downloads, both of which
//! finish some time after they are asked for.

use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui;
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

    /// The files the user is asked for, in order: a title, the file
    /// extensions offered, whether several files may be chosen, and whether
    /// the step may be skipped.
    fn steps(self) -> &'static [(&'static str, &'static [&'static str], bool, bool)] {
        match self {
            Source::Biom => &[("BIOM table", &["biom", "gz"], false, false)],
            Source::MgRast => &[("MG-RAST profile", &["tsv"], false, false)],
            Source::ImgCog => &[("IMG/M COG profile", &["xls", "tsv"], false, false)],
            Source::CoMet => &[("CoMet profiles, one per sample", &["txt"], true, false)],
            Source::Rita => &[("RITA profiles, one per sample", &["txt"], true, false)],
            Source::Mothur => &[
                ("mothur taxonomy file", &["taxonomy"], false, false),
                ("mothur groups file", &["groups"], false, false),
                ("mothur names file (optional)", &["names"], false, true),
            ],
        }
    }
}

/// Something the interface wants done with files.
pub enum Request {
    OpenProfile,
    OpenMetadata,
    /// Convert another tool's files to a STAMP profile, save it and open it.
    Convert(Source),
    /// Save data under a suggested name.
    Save {
        name: String,
        bytes: Vec<u8>,
    },
}

/// The outcome of a request.
pub enum Event {
    Profile(OpenedFile),
    Metadata(OpenedFile),
    /// A converted profile, ready to be loaded.
    Converted {
        name: String,
        spf: String,
    },
    Failed(String),
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

/// The application's connection to the file system or the browser.
pub struct Platform {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
}

impl Default for Platform {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Platform { sender, receiver }
    }
}

const PROFILE_EXTENSIONS: [&str; 3] = ["spf", "tsv", "txt"];
const METADATA_EXTENSIONS: [&str; 3] = ["tsv", "txt", "csv"];

impl Platform {
    /// Outcomes that have arrived since the last call.
    pub fn poll(&self) -> Vec<Event> {
        self.receiver.try_iter().collect()
    }

    /// Deliver an outcome directly, as if a request had produced it.
    pub fn push(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    /// Ask for something to be done. The outcome, if there is one, arrives
    /// through [`Platform::poll`]; `ctx` is asked to repaint when it does.
    pub fn request(&self, request: Request, ctx: &egui::Context) {
        imp::request(request, self.sender.clone(), ctx.clone());
    }
}

/// Carry out a request once its dialogs have been answered. Shared by the
/// desktop and browser implementations, which differ only in how they ask.
async fn perform(request: Request) -> Result<Option<Event>, String> {
    match request {
        Request::OpenProfile => Ok(imp::open("Load profile", &PROFILE_EXTENSIONS, false)
            .await
            .pop()
            .map(Event::Profile)),
        Request::OpenMetadata => Ok(imp::open("Load metadata", &METADATA_EXTENSIONS, false)
            .await
            .pop()
            .map(Event::Metadata)),
        Request::Save { name, bytes } => imp::save(&name, bytes).await.map(|_| None),
        Request::Convert(source) => {
            let mut files = Vec::new();
            for (title, extensions, several, optional) in source.steps() {
                let chosen = imp::open(title, extensions, *several).await;
                if chosen.is_empty() && !optional {
                    return Ok(None); // cancelled
                }
                files.extend(chosen);
            }
            let spf = convert_files(source, &files)?;
            let name = imp::save("profile.spf", spf.clone().into_bytes()).await?;
            Ok(Some(Event::Converted {
                name: name.unwrap_or_else(|| "profile.spf".to_string()),
                spf,
            }))
        }
    }
}

fn deliver(outcome: Result<Option<Event>, String>, sender: &Sender<Event>, ctx: &egui::Context) {
    let event = match outcome {
        Ok(Some(event)) => event,
        Ok(None) => return,
        Err(message) => Event::Failed(message),
    };
    let _ = sender.send(event);
    ctx.request_repaint();
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;

    /// Run a future that never actually waits: on the desktop every dialog
    /// answers before its function returns.
    fn run_now<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => unreachable!("desktop file dialogs complete immediately"),
        }
    }

    pub fn request(request: Request, sender: Sender<Event>, ctx: egui::Context) {
        deliver(run_now(perform(request)), &sender, &ctx);
    }

    fn read(path: &std::path::Path) -> Option<OpenedFile> {
        let bytes = std::fs::read(path).ok()?;
        let name = path.file_name()?.to_string_lossy().into_owned();
        Some(OpenedFile { name, bytes })
    }

    pub async fn open(title: &str, extensions: &[&str], several: bool) -> Vec<OpenedFile> {
        let dialog = rfd::FileDialog::new()
            .set_title(title)
            .add_filter(title, extensions)
            .add_filter("All files", &["*"]);
        let paths = if several {
            dialog.pick_files().unwrap_or_default()
        } else {
            dialog.pick_file().into_iter().collect()
        };
        paths.iter().filter_map(|p| read(p)).collect()
    }

    /// Ask where to save, and save. Returns the chosen file name, or `None`
    /// if the user cancelled.
    pub async fn save(default_name: &str, bytes: Vec<u8>) -> Result<Option<String>, String> {
        let extension = default_name.rsplit_once('.').map_or("", |(_, e)| e);
        let Some(path) = rfd::FileDialog::new()
            .set_title("Save")
            .set_file_name(default_name)
            .add_filter(extension, &[extension])
            .save_file()
        else {
            return Ok(None);
        };
        std::fs::write(&path, bytes)
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(path.file_name().map(|n| n.to_string_lossy().into_owned()))
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use wasm_bindgen::JsCast;

    pub fn request(request: Request, sender: Sender<Event>, ctx: egui::Context) {
        wasm_bindgen_futures::spawn_local(async move {
            deliver(perform(request).await, &sender, &ctx);
        });
    }

    pub async fn open(title: &str, extensions: &[&str], several: bool) -> Vec<OpenedFile> {
        let dialog = rfd::AsyncFileDialog::new()
            .set_title(title)
            .add_filter(title, extensions);
        let handles = if several {
            dialog.pick_files().await.unwrap_or_default()
        } else {
            dialog.pick_file().await.into_iter().collect()
        };
        let mut files = Vec::new();
        for handle in handles {
            files.push(OpenedFile {
                name: handle.file_name(),
                bytes: handle.read().await,
            });
        }
        files
    }

    /// Hand the data to the browser as a download.
    pub async fn save(default_name: &str, bytes: Vec<u8>) -> Result<Option<String>, String> {
        let failed = |_| "The browser refused the download.".to_string();
        let parts = js_sys::Array::new();
        parts.push(&js_sys::Uint8Array::from(bytes.as_slice()).buffer());
        let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(failed)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(failed)?;

        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("No page to download from.")?;
        let anchor: web_sys::HtmlAnchorElement = document
            .create_element("a")
            .map_err(failed)?
            .dyn_into()
            .map_err(|_| "No link element.".to_string())?;
        anchor.set_href(&url);
        anchor.set_download(default_name);
        anchor.click();
        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(Some(default_name.to_string()))
    }
}

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
            assert!(!source.steps().is_empty());
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

    #[test]
    fn events_are_queued_until_polled() {
        let platform = Platform::default();
        assert!(platform.poll().is_empty());
        platform.push(Event::Failed("first".to_string()));
        platform.push(Event::Converted {
            name: "a.spf".to_string(),
            spf: String::new(),
        });
        let events = platform.poll();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Event::Failed(m) if m == "first"));
        assert!(platform.poll().is_empty());
    }
}
