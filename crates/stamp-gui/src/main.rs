//! STAMP graphical application.
//!
//! The same code is built as a native window on Windows, Linux and macOS and,
//! later, as WebAssembly for the browser.

// Do not open a console window alongside the GUI on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("STAMP")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "STAMP",
        options,
        Box::new(|_cc| Ok(Box::new(app::StampApp::default()))),
    )
}

// The browser entry point is added with the web build.
#[cfg(target_arch = "wasm32")]
fn main() {}
