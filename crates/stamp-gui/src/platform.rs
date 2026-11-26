//! File access: opening, converting and saving files.
//!
//! The interface asks for something with [`Platform::request`] and later
//! receives the outcome from [`Platform::poll`]. On the desktop the request
//! is answered at once with native dialogs. In the browser, files come from
//! the browser's file picker and are saved as downloads, both of which
//! finish some time after they are asked for.
//!
//! PNG images of plots are drawn with resvg on the desktop. The browser
//! build leaves resvg out and has the browser draw the plot's SVG into a
//! canvas, which also finishes later, so PNG images are asked for in the
//! same way.

use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui;
use stamp_core::import::profiler::{self, ProfilerFormat};
use stamp_core::import::{biom, cog, mothur, sandpiper, singlem, NamedText};
use stamp_plot::export::{export, Format};
use stamp_plot::Figure;

/// A file chosen by the user.
pub struct OpenedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl OpenedFile {
    /// A file as read, decompressed if it is gzip-compressed, so that every
    /// file can be given compressed or not.
    pub fn new(name: String, bytes: Vec<u8>) -> Result<Self, String> {
        let bytes = match stamp_core::text::decompress(&bytes) {
            Ok(std::borrow::Cow::Owned(decompressed)) => decompressed,
            Ok(std::borrow::Cow::Borrowed(_)) => bytes,
            Err(error) => return Err(format!("Could not read {name}: {error}.")),
        };
        Ok(Self { name, bytes })
    }

    pub fn text(&self) -> String {
        stamp_core::text::decode(&self.bytes).into_owned()
    }

    /// True if the file is named as a mothur taxonomy file
    /// (`*.taxonomy`, or `*.taxonomy.gz`).
    pub fn is_mothur_taxonomy(&self) -> bool {
        let name = self.name.to_lowercase();
        let name = name.strip_suffix(".gz").unwrap_or(&name);
        name.ends_with(".taxonomy")
    }
}

/// The files of a directory that may be profiles written by profilers
/// (SingleM, mOTUs, MetaPhlAn, Kraken, HUMAnN, ...).
pub struct ProfileDirectory {
    /// The directory's name, without its parent directories.
    pub name: String,
    /// Files that start like a profiler's output, in order of name.
    pub files: Vec<OpenedFile>,
    /// Names of the other files, which are not read in full.
    pub skipped: Vec<String>,
}

/// How much of a file is read to tell whether it is a profiler's output.
const DETECTION_BYTES: usize = 65536;

/// True if a file starting with these bytes (compressed or not) is a
/// profiler's output.
fn starts_like_profile(start: &[u8]) -> bool {
    let start = stamp_core::text::decompressed_start(start, DETECTION_BYTES);
    ProfilerFormat::detect(&String::from_utf8_lossy(&start)).is_some()
}

/// File formats that can be converted to a STAMP profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Biom,
    Mothur,
    SingleM,
    Sandpiper,
    ImgCog,
}

impl Source {
    pub const ALL: [Source; 5] = [
        Source::Biom,
        Source::Mothur,
        Source::SingleM,
        Source::Sandpiper,
        Source::ImgCog,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Source::Biom => "BIOM table...",
            Source::Mothur => "mothur files...",
            Source::SingleM => "SingleM condensed profiles...",
            Source::Sandpiper => "Sandpiper profiles...",
            Source::ImgCog => "IMG/M COG profile (add COG categories)...",
        }
    }

    /// The files the user is asked for, in order: a title, the file
    /// extensions offered, whether several files may be chosen, and whether
    /// the step may be skipped.
    fn steps(self) -> &'static [(&'static str, &'static [&'static str], bool, bool)] {
        match self {
            Source::Biom => &[("BIOM table", &["biom", "gz"], false, false)],
            Source::ImgCog => &[("IMG/M COG profile", &["xls", "tsv", "gz"], false, false)],
            Source::SingleM => &[(
                "SingleM condensed profiles",
                &["tsv", "txt", "csv", "gz"],
                true,
                false,
            )],
            Source::Sandpiper => &[(
                "Sandpiper profiles, one per sample",
                &["json", "gz"],
                true,
                false,
            )],
            Source::Mothur => &[
                ("mothur taxonomy file", &["taxonomy", "gz"], false, false),
                ("mothur groups file", &["groups", "gz"], false, false),
                (
                    "mothur names file (optional)",
                    &["names", "gz"],
                    false,
                    true,
                ),
            ],
        }
    }
}

/// A further file that a profile needs before it can be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auxiliary {
    MothurGroups,
    MothurNames,
}

/// Something the interface wants done with files.
pub enum Request {
    OpenProfile,
    /// Choose a directory of profiles written by a profiler.
    OpenProfileDirectory,
    OpenMetadata,
    /// Ask for a further file of a profile that is being opened.
    OpenAuxiliary(Auxiliary),
    /// Convert another tool's files to a STAMP profile, save it and open it.
    Convert(Source),
    /// Save data under a suggested name.
    Save {
        name: String,
        bytes: Vec<u8>,
    },
    /// Save a figure as a PNG image of the given resolution.
    SavePng {
        figure: Box<Figure>,
        dpi: f64,
    },
    /// Draw a figure at the given resolution, for the clipboard.
    PlotImage {
        figure: Box<Figure>,
        dpi: f64,
    },
}

/// The outcome of a request.
pub enum Event {
    /// Files chosen with Load profile file: one, or several profiles of one
    /// profiler.
    Profile(Vec<OpenedFile>),
    /// The profiles in a directory chosen with Load profile directory.
    ProfileDirectory(ProfileDirectory),
    Metadata(OpenedFile),
    Auxiliary(Auxiliary, OpenedFile),
    /// A converted profile, ready to be loaded.
    Converted {
        name: String,
        spf: String,
    },
    /// An image of a figure, for the clipboard.
    PlotImage(egui::ColorImage),
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
        Source::SingleM => singlem::to_spf(&named),
        Source::Sandpiper => sandpiper::to_spf(&named),
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

/// Resolution of saved PNG images unless another is chosen.
pub const DEFAULT_DPI: f64 = 300.0;

/// Encode a figure in the format implied by a file name. `dpi` sets the
/// resolution of a PNG image. The browser build cannot make PNG images this
/// way: ask for them with [`Request::SavePng`] or [`Request::PlotImage`].
pub fn encode_plot(figure: &Figure, file_name: &str, dpi: f64) -> Result<Vec<u8>, String> {
    let format = Format::from_file_name(file_name)
        .ok_or("Use a file name ending in .png, .svg or .pdf to choose the image format.")?;
    export(figure, format, dpi).map_err(|e| e.to_string())
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

/// STAMP profiles, BIOM tables, mothur taxonomy files, and the outputs of
/// SingleM, Sandpiper (JSON), mOTUs, MetaPhlAn, Kraken, Bracken and HUMAnN.
const PROFILE_EXTENSIONS: [&str; 15] = [
    "spf", "tsv", "txt", "biom", "gz", "taxonomy", "json", "motus", "relab", "report", "kreport",
    "k2report", "kreport2", "bracken", "mpa",
];
const METADATA_EXTENSIONS: [&str; 4] = ["tsv", "txt", "csv", "gz"];

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
        Request::OpenProfile => {
            // Several files can be chosen: profilers usually write one file
            // per sample.
            let files = imp::open("Load profile file", &PROFILE_EXTENSIONS, true).await?;
            Ok((!files.is_empty()).then_some(Event::Profile(files)))
        }
        Request::OpenProfileDirectory => Ok(imp::open_directory("Load profile directory")
            .await?
            .map(Event::ProfileDirectory)),
        Request::OpenMetadata => Ok(imp::open("Load metadata", &METADATA_EXTENSIONS, false)
            .await?
            .pop()
            .map(Event::Metadata)),
        Request::OpenAuxiliary(which) => {
            let (title, extensions): (&str, &[&str]) = match which {
                Auxiliary::MothurGroups => ("mothur groups file", &["groups", "gz"]),
                Auxiliary::MothurNames => ("mothur names file", &["names", "gz"]),
            };
            Ok(imp::open(title, extensions, false)
                .await?
                .pop()
                .map(|file| Event::Auxiliary(which, file)))
        }
        Request::Save { name, bytes } => imp::save(&name, bytes).await.map(|_| None),
        Request::SavePng { figure, dpi } => {
            let bytes = imp::png(&figure, dpi).await?;
            imp::save("plot.png", bytes).await.map(|_| None)
        }
        Request::PlotImage { figure, dpi } => {
            Ok(Some(Event::PlotImage(imp::image(&figure, dpi).await?)))
        }
        Request::Convert(source) => {
            let mut files = Vec::new();
            for (title, extensions, several, optional) in source.steps() {
                let chosen = imp::open(title, extensions, *several).await?;
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

/// Read the profiles of a directory as Load profile directory does.
#[cfg(not(target_arch = "wasm32"))]
pub use imp::read_directory;

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

    fn read(path: &std::path::Path) -> Result<OpenedFile, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        OpenedFile::new(name, bytes)
    }

    pub async fn open(
        title: &str,
        extensions: &[&str],
        several: bool,
    ) -> Result<Vec<OpenedFile>, String> {
        let dialog = rfd::FileDialog::new()
            .set_title(title)
            .add_filter(title, extensions)
            .add_filter("All files", &["*"]);
        let paths = if several {
            dialog.pick_files().unwrap_or_default()
        } else {
            dialog.pick_file().into_iter().collect()
        };
        paths.iter().map(|p| read(p)).collect()
    }

    pub async fn open_directory(title: &str) -> Result<Option<ProfileDirectory>, String> {
        match rfd::FileDialog::new().set_title(title).pick_folder() {
            Some(path) => read_directory(&path).map(Some),
            None => Ok(None),
        }
    }

    /// The files directly in a directory (not in its subdirectories) that
    /// may be profilers' outputs. Only the start of other files
    /// is read, so that large files beside the profiles, such as reads,
    /// cost little. Hidden files are left out.
    pub fn read_directory(path: &std::path::Path) -> Result<ProfileDirectory, String> {
        use std::io::Read;
        let failed = |e: std::io::Error| format!("Could not read {}: {e}", path.display());
        let mut entries: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(path)
            .map_err(failed)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = entry.path();
                // Follows links, unlike the entry's own file type.
                let is_file = std::fs::metadata(&path).is_ok_and(|m| m.is_file());
                (is_file && profiler::is_candidate(&name)).then_some((name, path))
            })
            .collect();
        entries.sort();
        let mut directory = ProfileDirectory {
            name: path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            ),
            files: Vec::new(),
            skipped: Vec::new(),
        };
        for (name, path) in entries {
            let mut start = Vec::new();
            let opened = std::fs::File::open(&path)
                .and_then(|file| file.take(DETECTION_BYTES as u64).read_to_end(&mut start));
            if opened.is_ok() && starts_like_profile(&start) {
                directory.files.push(read(&path)?);
            } else {
                directory.skipped.push(name);
            }
        }
        Ok(directory)
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

    /// A PNG image of a figure, drawn with resvg.
    pub async fn png(figure: &Figure, dpi: f64) -> Result<Vec<u8>, String> {
        encode_plot(figure, "plot.png", dpi)
    }

    /// An image of a figure, drawn with resvg.
    pub async fn image(figure: &Figure, dpi: f64) -> Result<egui::ColorImage, String> {
        let bytes = png(figure, dpi).await?;
        let failed = |_| "The plot could not be turned into an image.".to_string();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .map_err(failed)?;
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).map_err(failed)?;
        let size = [info.width as usize, info.height as usize];
        match (info.color_type, info.bit_depth) {
            (png::ColorType::Rgba, png::BitDepth::Eight) => Ok(
                egui::ColorImage::from_rgba_unmultiplied(size, &buffer[..info.buffer_size()]),
            ),
            (png::ColorType::Rgb, png::BitDepth::Eight) => Ok(egui::ColorImage::from_rgb(
                size,
                &buffer[..info.buffer_size()],
            )),
            _ => Err("The plot could not be turned into an image.".to_string()),
        }
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

    pub async fn open(
        title: &str,
        extensions: &[&str],
        several: bool,
    ) -> Result<Vec<OpenedFile>, String> {
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
            files.push(OpenedFile::new(handle.file_name(), handle.read().await)?);
        }
        Ok(files)
    }

    /// The contents of a blob, such as a file or part of one.
    async fn blob_bytes(blob: &web_sys::Blob) -> Result<Vec<u8>, String> {
        let buffer = wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
            .await
            .map_err(|_| "The browser could not read a file.".to_string())?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }

    /// Ask for a directory with the browser's folder picker (an input
    /// element with `webkitdirectory`, which rfd does not offer), and read
    /// the files directly in it that may be profilers' outputs.
    /// The browser asks the user to allow the page to read the folder.
    pub async fn open_directory(_title: &str) -> Result<Option<ProfileDirectory>, String> {
        let failed = |_| "The browser could not open a folder picker.".to_string();
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("No page to open a folder from.")?;
        let input: web_sys::HtmlInputElement = document
            .create_element("input")
            .map_err(failed)?
            .dyn_into()
            .map_err(|_| "No input element.".to_string())?;
        input.set_type("file");
        input.set_attribute("webkitdirectory", "").map_err(failed)?;
        input
            .set_attribute("style", "display: none")
            .map_err(failed)?;
        let body = document.body().ok_or("No page to open a folder from.")?;
        body.append_child(&input).map_err(failed)?;
        // Settled when files are chosen or the picker is cancelled. A
        // browser that refuses to show the picker settles nothing, which
        // leaves this request waiting harmlessly.
        let chosen = js_sys::Promise::new(&mut |resolve, _| {
            input.set_onchange(Some(&resolve));
            let _ = input.add_event_listener_with_callback("cancel", &resolve);
        });
        input.click();
        let _ = wasm_bindgen_futures::JsFuture::from(chosen).await;
        input.set_onchange(None);
        let _ = body.remove_child(&input);

        let Some(list) = input.files().filter(|list| list.length() > 0) else {
            return Ok(None);
        };
        let mut name = String::new();
        let mut entries = Vec::new();
        for index in 0..list.length() {
            let Some(file) = list.get(index) else {
                continue;
            };
            // `folder/file`; files in subdirectories have more parts.
            // web-sys has no binding for this property.
            let path = js_sys::Reflect::get(&file, &"webkitRelativePath".into())
                .ok()
                .and_then(|path| path.as_string())
                .unwrap_or_default();
            let parts: Vec<&str> = path.split('/').collect();
            if let [folder, file_name] = parts.as_slice() {
                name = folder.to_string();
                if profiler::is_candidate(file_name) {
                    entries.push((file_name.to_string(), file));
                }
            }
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut directory = ProfileDirectory {
            name,
            files: Vec::new(),
            skipped: Vec::new(),
        };
        for (name, file) in entries {
            let start = file
                .slice_with_f64_and_f64(0.0, DETECTION_BYTES as f64)
                .map_err(|_| "The browser could not read a file.".to_string())?;
            if starts_like_profile(&blob_bytes(&start).await?) {
                let bytes = blob_bytes(&file).await?;
                directory.files.push(OpenedFile::new(name, bytes)?);
            } else {
                directory.skipped.push(name);
            }
        }
        Ok(Some(directory))
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

    /// The largest canvas, in pixels, that every common browser can draw
    /// (Chrome's and Safari's limit).
    const MAX_PIXELS: f64 = 268_435_456.0;

    fn drawing_failed(_: wasm_bindgen::JsValue) -> String {
        "The browser could not draw the plot.".to_string()
    }

    /// Have the browser draw a figure into a new canvas: its SVG, carrying
    /// the figure's font, loaded as an image and drawn at the size the
    /// resolution asks for. Returns the canvas's drawing context.
    async fn draw(figure: &Figure, dpi: f64) -> Result<web_sys::CanvasRenderingContext2d, String> {
        if !(dpi.is_finite() && dpi > 0.0) {
            return Err(format!("Invalid resolution: {dpi} dpi."));
        }
        let scale = dpi / 72.0;
        let width = (figure.width * scale).round().max(1.0);
        let height = (figure.height * scale).round().max(1.0);
        if width * height > MAX_PIXELS {
            return Err(format!(
                "The image would be {width} by {height} pixels, which is more than the browser can draw. Use a lower resolution or a vector format."
            ));
        }
        let (width, height) = (width as u32, height as u32);

        let svg = stamp_plot::svg::to_svg_with_font(figure, width, height);
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("image/svg+xml");
        let parts = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(&svg));
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
            .map_err(drawing_failed)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(drawing_failed)?;
        let image = web_sys::HtmlImageElement::new().map_err(drawing_failed)?;
        let loaded = js_sys::Promise::new(&mut |resolve, reject| {
            image.set_onload(Some(&resolve));
            image.set_onerror(Some(&reject));
        });
        image.set_src(&url);
        let outcome = wasm_bindgen_futures::JsFuture::from(loaded).await;
        if outcome.is_ok() {
            // Wait until the image, with its font, is ready to be drawn;
            // a browser that cannot say so draws it as it is.
            let _ = wasm_bindgen_futures::JsFuture::from(image.decode()).await;
        }
        image.set_onload(None);
        image.set_onerror(None);
        let _ = web_sys::Url::revoke_object_url(&url);
        outcome.map_err(drawing_failed)?;

        let canvas: web_sys::HtmlCanvasElement = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("No page to draw on.")?
            .create_element("canvas")
            .map_err(drawing_failed)?
            .dyn_into()
            .map_err(|_| "No canvas element.".to_string())?;
        canvas.set_width(width);
        canvas.set_height(height);
        let context: web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")
            .map_err(drawing_failed)?
            .ok_or("The browser could not draw the plot.")?
            .dyn_into()
            .map_err(|_| "No canvas drawing context.".to_string())?;
        context
            .draw_image_with_html_image_element_and_dw_and_dh(
                &image,
                0.0,
                0.0,
                f64::from(width),
                f64::from(height),
            )
            .map_err(drawing_failed)?;
        Ok(context)
    }

    /// A PNG image of a figure, drawn by the browser.
    pub async fn png(figure: &Figure, dpi: f64) -> Result<Vec<u8>, String> {
        let context = draw(figure, dpi).await?;
        let canvas = context.canvas().ok_or("No canvas.")?;
        let encoded = js_sys::Promise::new(&mut |resolve, reject| {
            if let Err(error) = canvas.to_blob_with_type(&resolve, "image/png") {
                let _ = reject.call1(&wasm_bindgen::JsValue::NULL, &error);
            }
        });
        let blob: web_sys::Blob = wasm_bindgen_futures::JsFuture::from(encoded)
            .await
            .map_err(drawing_failed)?
            .dyn_into()
            // No image (null) when the canvas is too large to encode.
            .map_err(|_| "The browser could not make the image.".to_string())?;
        let buffer = wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
            .await
            .map_err(drawing_failed)?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }

    /// An image of a figure, drawn by the browser.
    pub async fn image(figure: &Figure, dpi: f64) -> Result<egui::ColorImage, String> {
        let context = draw(figure, dpi).await?;
        let canvas = context.canvas().ok_or("No canvas.")?;
        let (width, height) = (canvas.width(), canvas.height());
        let pixels = context
            .get_image_data(0.0, 0.0, f64::from(width), f64::from(height))
            .map_err(drawing_failed)?
            .data();
        Ok(egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            &pixels,
        ))
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
        let singlem = [file(
            "p.tsv",
            "sample\tcoverage\ttaxonomy\nA\t1\tRoot; d__Bacteria\nB\t2\tRoot; d__Archaea\n",
        )];
        assert!(convert_files(Source::SingleM, &singlem)
            .unwrap()
            .starts_with("Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tA\tB\n"));
        assert!(convert_files(Source::Sandpiper, &singlem)
            .unwrap_err()
            .contains("Sandpiper"));

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
        assert!(encode_plot(&figure, "a.svg", DEFAULT_DPI)
            .unwrap()
            .starts_with(b"<svg"));
        assert!(encode_plot(&figure, "a.png", DEFAULT_DPI)
            .unwrap()
            .starts_with(b"\x89PNG"));
        assert!(encode_plot(&figure, "a.pdf", DEFAULT_DPI)
            .unwrap()
            .starts_with(b"%PDF"));
        assert!(encode_plot(&figure, "a.eps", DEFAULT_DPI)
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
