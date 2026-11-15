//! The assistant's menu, windows and status-bar controls.

use eframe::egui;

use super::describe;
use crate::StampApp;

/// The Assistant menu.
pub(crate) fn menu(app: &mut StampApp, ui: &mut egui::Ui) {
    let copy = ui.button("Copy description for AI").on_hover_text(
        "Copy a description of the data, the analysis, the plot and the top results, \
             to paste into a chat with Claude, ChatGPT or Gemini",
    );
    if copy.clicked() {
        let text = describe::everything(app);
        ui.ctx().copy_text(text);
        app.show_notice(
            "A description of the session, the plot and the top results is on the clipboard.\n\
             Paste it into a chat with an AI assistant and ask your question. To include the \
             plot itself, also use Copy plot image and paste that too.",
        );
    }
    let image = ui
        .add_enabled(
            app.session.tree.is_some(),
            egui::Button::new("Copy plot image"),
        )
        .on_hover_text("Copy the plot as an image, to paste into a chat");
    if image.clicked() {
        match plot_image(app.figure()) {
            Ok(image) => ui.ctx().copy_image(image),
            Err(message) => app.show_notice(message),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.separator();
        if ui
            .button("Open Claude Code...")
            .on_hover_text(
                "Ask Claude Code about your data and plots, and have it change them. Needs a \
                 Claude Pro, Max, Team or Enterprise plan.",
            )
            .clicked()
        {
            app.assistant.asking = true;
        }
        if ui
            .add(egui::Button::new("Assistant activity").selected(app.assistant.show_window))
            .clicked()
        {
            app.assistant.show_window = !app.assistant.show_window;
        }
        if ui
            .add_enabled(
                app.assistant.server.is_some(),
                egui::Button::new("Stop sharing with Claude Code"),
            )
            .clicked()
        {
            app.assistant.server = None;
        }
    }
    ui.separator();
    if ui
        .add_enabled(
            app.can_undo_assistant_change(),
            egui::Button::new("Undo assistant's last change"),
        )
        .clicked()
    {
        app.undo_assistant_change();
    }
}

/// An image of a figure for the clipboard, at twice the resolution of the
/// screen.
fn plot_image(figure: &stamp_plot::Figure) -> Result<egui::ColorImage, String> {
    let bytes = crate::platform::encode_plot(figure, "plot.png", 144.0)?;
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

/// The assistant's part of the status bar: that Claude Code is connected,
/// and an Undo button for its changes.
pub(crate) fn status(app: &mut StampApp, ui: &mut egui::Ui) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(server) = &app.assistant.server {
        let text = if server.clients() > 0 {
            "Claude Code connected"
        } else {
            "Waiting for Claude Code"
        };
        if ui
            .link(text)
            .on_hover_text("STAMP is sharing this session with Claude Code. Click for details.")
            .clicked()
        {
            app.assistant.show_window = true;
        }
        ui.separator();
    }
    if app.can_undo_assistant_change() {
        if ui
            .small_button("Undo assistant's change")
            .on_hover_text(
                "Put back the analysis and plot as they were before the assistant's last \
                 change",
            )
            .clicked()
        {
            app.undo_assistant_change();
        }
        ui.separator();
    }
}

/// The assistant's windows.
pub(crate) fn windows(app: &mut StampApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        asking_window(app, ctx);
        activity_window(app, ctx);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

/// What sharing with Claude Code means, said before it starts.
#[cfg(not(target_arch = "wasm32"))]
const SHARING: &str = "Claude Code will be able to read the loaded profile and metadata, the \
results and the plots, and to change the analysis and the plots in this window. What it reads \
is sent to Anthropic to answer your questions, under your Claude account. Sharing stops when \
you choose Assistant > Stop sharing with Claude Code, or close STAMP.";

#[cfg(not(target_arch = "wasm32"))]
fn asking_window(app: &mut StampApp, ctx: &egui::Context) {
    if !app.assistant.asking {
        return;
    }
    let claude = super::launch::find_claude();
    let mut open = true;
    let mut chosen = None;
    egui::Window::new("Open Claude Code")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            ui.label(SHARING);
            ui.add_space(6.0);
            match &claude {
                Some(path) => {
                    ui.label(format!(
                        "Claude Code opens in a terminal. Sign in with your Claude account the \
                         first time, then ask your questions there.\n\nClaude Code: {}",
                        path.display()
                    ));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Open Claude Code").clicked() {
                            chosen = Some(false);
                        }
                        if ui.button("Cancel").clicked() {
                            chosen = None;
                            app.assistant.asking = false;
                        }
                    });
                }
                None => {
                    ui.label(
                        "Claude Code is not installed. STAMP can open a terminal that installs \
                         it with Anthropic's installer (claude.ai/install) and then starts \
                         it. Claude Code needs a Claude Pro, Max, Team or Enterprise plan; \
                         sign in with your Claude account when it starts.",
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Install and open").clicked() {
                            chosen = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            app.assistant.asking = false;
                        }
                    });
                }
            }
        });
    if !open {
        app.assistant.asking = false;
    }
    if let Some(install) = chosen {
        app.assistant.asking = false;
        if let Err(message) = open_claude_code(app, ctx, install) {
            app.show_notice(message);
        }
    }
}

/// Start sharing (if not already), prepare Claude Code's folder and open
/// a terminal running it, or its installer first when `install` is set.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_claude_code(
    app: &mut StampApp,
    ctx: &egui::Context,
    install: bool,
) -> Result<(), String> {
    let server = start_sharing(app, ctx)?;
    let (url, token) = (server.url(), server.token().to_string());
    let folder = super::launch::working_folder()
        .ok_or("STAMP could not find a folder to run Claude Code in.")?;
    let written = if install {
        super::launch::prepare_install(&folder, &url, &token)
    } else {
        let claude = super::launch::find_claude().ok_or("Claude Code was not found.")?;
        super::launch::prepare(&folder, &url, &token, &claude)
    };
    let script = written.map_err(|e| {
        format!(
            "Could not prepare Claude Code's folder {}: {e}",
            folder.display()
        )
    })?;
    super::launch::open_terminal(&script)?;
    app.assistant.show_window = true;
    Ok(())
}

/// Start the server if it is not running, and return it.
#[cfg(not(target_arch = "wasm32"))]
pub fn start_sharing<'a>(
    app: &'a mut StampApp,
    ctx: &egui::Context,
) -> Result<&'a super::server::Server, String> {
    if app.assistant.server.is_none() {
        let waker = ctx.clone();
        let server = super::server::Server::start(move || waker.request_repaint())
            .map_err(|e| format!("Could not start sharing with Claude Code: {e}"))?;
        app.assistant.server = Some(server);
    }
    Ok(app.assistant.server.as_ref().expect("just started"))
}

#[cfg(not(target_arch = "wasm32"))]
fn activity_window(app: &mut StampApp, ctx: &egui::Context) {
    let mut open = app.assistant.show_window;
    let mut stop = false;
    let mut reopen = false;
    egui::Window::new("Assistant")
        .open(&mut open)
        .default_size([520.0, 360.0])
        .show(ctx, |ui| {
            match &app.assistant.server {
                Some(server) => {
                    ui.label(format!(
                        "Sharing this session with Claude Code at {} ({}).",
                        server.url(),
                        if server.clients() > 0 {
                            "connected"
                        } else {
                            "not connected yet"
                        }
                    ));
                    ui.horizontal(|ui| {
                        stop = ui.button("Stop sharing").clicked();
                        reopen = ui.button("Open Claude Code again").clicked();
                    });
                }
                None => {
                    ui.label("Not sharing. Use Assistant > Open Claude Code to start.");
                }
            }
            if let Some(folder) = super::launch::working_folder() {
                ui.label(format!("Claude Code's folder: {}", folder.display()));
            }
            ui.separator();
            ui.label(egui::RichText::new("What the assistant asked for, latest first").strong());
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if app.assistant.log.is_empty() {
                        ui.label("Nothing yet.");
                    }
                    for entry in app.assistant.log.iter().rev() {
                        let heading = if entry.args == "{}" || entry.args == "null" {
                            entry.tool.clone()
                        } else {
                            format!("{} {}", entry.tool, entry.args)
                        };
                        let color = if entry.is_error {
                            ui.visuals().error_fg_color
                        } else {
                            ui.visuals().strong_text_color()
                        };
                        ui.label(egui::RichText::new(heading).monospace().color(color));
                        ui.label(&entry.answer);
                        ui.add_space(4.0);
                    }
                });
        });
    app.assistant.show_window = open;
    if stop {
        app.assistant.server = None;
    }
    if reopen {
        app.assistant.asking = true;
    }
}
