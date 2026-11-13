//! STAMP graphical application: native entry point.

// Do not open a console window alongside the GUI on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::egui;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("STAMP")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    // Files named on the command line are opened at start-up: a profile,
    // then optionally its metadata.
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    eframe::run_native(
        "STAMP",
        options,
        Box::new(move |cc| {
            let mut app = stamp_gui::StampApp::new(&cc.egui_ctx);
            let read = |path: &String| {
                let bytes = std::fs::read(path).ok()?;
                let name = std::path::Path::new(path)
                    .file_name()?
                    .to_string_lossy()
                    .into_owned();
                Some((name, stamp_core::text::decode(&bytes).into_owned()))
            };
            if let Some((name, text)) = arguments.first().and_then(read) {
                app.load_profile(&name, &text);
            }
            if let Some((name, text)) = arguments.get(1).and_then(read) {
                app.load_metadata(&name, &text);
            }
            Ok(Box::new(app))
        }),
    )
}

// The browser entry point is added with the web build.
#[cfg(target_arch = "wasm32")]
fn main() {}
