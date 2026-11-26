//! STAMP graphical application: entry point.

// Do not open a console window alongside the GUI on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    use eframe::egui;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("STAMP")
            .with_inner_size([1280.0, 800.0])
            .with_icon(window_icon()),
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
            app.restore_from(&cc.egui_ctx, cc.storage);
            let read = |path: &String| {
                let bytes = std::fs::read(path).ok()?;
                let name = std::path::Path::new(path)
                    .file_name()?
                    .to_string_lossy()
                    .into_owned();
                let bytes = stamp_core::text::decompress(&bytes).ok()?;
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

/// The window icon, decoded from the PNG built into the program.
fn window_icon() -> eframe::egui::IconData {
    let decoder = png::Decoder::new(std::io::Cursor::new(
        &include_bytes!("../assets/icon.png")[..],
    ));
    let decode = || -> Option<eframe::egui::IconData> {
        let mut reader = decoder.read_info().ok()?;
        let mut rgba = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut rgba).ok()?;
        (info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight).then(
            || {
                rgba.truncate(info.buffer_size());
                eframe::egui::IconData {
                    rgba,
                    width: info.width,
                    height: info.height,
                }
            },
        )
    };
    // Without an icon the window still works.
    decode().unwrap_or_default()
}
