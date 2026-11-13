use eframe::egui;

/// Top-level application state.
#[derive(Default)]
pub struct StampApp {}

impl eframe::App for StampApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("STAMP");
            ui.label(format!("Version {}", stamp_core::VERSION));
            ui.label("The Rust port is in progress; no analyses are available yet.");
        });
    }
}
