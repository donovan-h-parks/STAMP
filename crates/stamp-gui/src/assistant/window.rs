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
        let sharing = app.assistant.server.is_some();
        let share = ui
            .add_enabled(!sharing, egui::Button::new("Share with Claude Desktop..."))
            .on_hover_text(
                "Let Claude, in the Claude Desktop app, see this session and change the analysis \
                 and plots. Needs STAMP's extension in Claude Desktop (Install in Claude \
                 Desktop).",
            );
        if share.clicked() {
            app.assistant.asking = Some(super::Ask::ClaudeDesktop);
        }
        if ui
            .button("Install in Claude Desktop...")
            .on_hover_text("Add STAMP's extension to the Claude Desktop app (once)")
            .clicked()
        {
            match install_in_claude_desktop() {
                Ok(ask) => app.assistant.asking = Some(ask),
                Err(message) => app.show_notice(message),
            }
        }
        if ui
            .button("Open Claude Code...")
            .on_hover_text(
                "Ask Claude Code, in a terminal, about your data and plots, and have it change \
                 them. Needs a Claude Pro, Max, Team or Enterprise plan.",
            )
            .clicked()
        {
            app.assistant.asking = Some(super::Ask::ClaudeCode);
        }
        if ui
            .add_enabled(sharing, egui::Button::new("Stop sharing with Claude"))
            .clicked()
        {
            app.assistant.server = None;
        }
        if ui
            .add(egui::Button::new("Assistant activity").selected(app.assistant.show_window))
            .clicked()
        {
            app.assistant.show_window = !app.assistant.show_window;
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

/// The assistant's part of the status bar: that Claude is connected,
/// and an Undo button for its changes.
pub(crate) fn status(app: &mut StampApp, ui: &mut egui::Ui) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(server) = &app.assistant.server {
        let text = if server.clients() > 0 {
            "Claude connected"
        } else {
            "Sharing with Claude"
        };
        if ui
            .link(text)
            .on_hover_text("STAMP is sharing this session with Claude. Click for details.")
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

/// What sharing means, said before it starts. `who` is "Claude" or
/// "Claude Code".
#[cfg(not(target_arch = "wasm32"))]
fn sharing(who: &str) -> String {
    format!(
        "{who} will be able to read the loaded profile and metadata, the results and the plots, \
         and to change the analysis and the plots in this window. What it reads is sent to \
         Anthropic to answer your questions, under your Claude account. Sharing stops when you \
         choose Assistant > Stop sharing with Claude, or close STAMP."
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn asking_window(app: &mut StampApp, ctx: &egui::Context) {
    let Some(ask) = app.assistant.asking.clone() else {
        return;
    };
    let title = match ask {
        super::Ask::ClaudeDesktop => "Share with Claude Desktop",
        super::Ask::ClaudeCode => "Open Claude Code",
        super::Ask::Installed { .. } => "Install in Claude Desktop",
    };
    let claude = match ask {
        super::Ask::ClaudeCode => super::launch::find_claude(),
        _ => None,
    };
    let mut open = true;
    let mut close = false;
    /// What the user chose.
    enum Choice {
        Share,
        ShareAndInstall,
        OpenClaudeCode { install: bool },
    }
    let mut chosen = None;
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            match &ask {
                super::Ask::ClaudeDesktop => {
                    ui.label(sharing("Claude"));
                    ui.add_space(6.0);
                    ui.label(
                        "Then ask Claude about your data in the Claude Desktop app. It reaches \
                         STAMP through STAMP's extension: if you have not added it to Claude \
                         Desktop yet, choose Share and install.",
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Share").clicked() {
                            chosen = Some(Choice::Share);
                        }
                        if ui.button("Share and install").clicked() {
                            chosen = Some(Choice::ShareAndInstall);
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                super::Ask::ClaudeCode => {
                    ui.label(sharing("Claude Code"));
                    ui.add_space(6.0);
                    match &claude {
                        Some(path) => {
                            ui.label(format!(
                                "Claude Code opens in a terminal. Sign in with your Claude \
                                 account the first time, then ask your questions there.\n\n\
                                 Claude Code: {}",
                                path.display()
                            ));
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("Open Claude Code").clicked() {
                                    chosen = Some(Choice::OpenClaudeCode { install: false });
                                }
                                close = ui.button("Cancel").clicked();
                            });
                        }
                        None => {
                            ui.label(
                                "Claude Code is not installed. STAMP can open a terminal that \
                                 installs it with Anthropic's installer (claude.ai/install) and \
                                 then starts it. Claude Code needs a Claude Pro, Max, Team or \
                                 Enterprise plan; sign in with your Claude account when it \
                                 starts.",
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("Install and open").clicked() {
                                    chosen = Some(Choice::OpenClaudeCode { install: true });
                                }
                                close = ui.button("Cancel").clicked();
                            });
                        }
                    }
                }
                super::Ask::Installed { bundle, opened } => {
                    let path = bundle.display().to_string();
                    if *opened {
                        ui.label(
                            "Claude Desktop should now ask whether to install STAMP's \
                             extension: choose Install. If it does not, open Claude Desktop's \
                             settings, choose Extensions, then Install Extension, and pick this \
                             file:",
                        );
                    } else {
                        ui.label(
                            "In Claude Desktop, open the settings, choose Extensions, then \
                             Install Extension, and pick this file:",
                        );
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.monospace(&path);
                        if ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(path.clone());
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(
                        "This is needed once. Afterwards, choose Assistant > Share with Claude \
                         Desktop whenever you want to ask Claude about a session.",
                    );
                    ui.add_space(6.0);
                    close = ui.button("OK").clicked();
                }
            }
        });
    if !open || close {
        app.assistant.asking = None;
    }
    let Some(choice) = chosen else {
        return;
    };
    app.assistant.asking = None;
    let outcome = match choice {
        Choice::Share => start_sharing(app, ctx).map(|_| ()),
        Choice::ShareAndInstall => start_sharing(app, ctx)
            .map(|_| ())
            .and_then(|()| install_in_claude_desktop())
            .map(|installed| app.assistant.asking = Some(installed)),
        Choice::OpenClaudeCode { install } => open_claude_code(app, ctx, install),
    };
    if outcome.is_ok() {
        app.assistant.show_window = true;
    }
    if let Err(message) = outcome {
        app.show_notice(message);
    }
}

/// Write STAMP's Claude Desktop extension, holding the bridge installed
/// beside STAMP, and hand it to Claude Desktop.
#[cfg(not(target_arch = "wasm32"))]
fn install_in_claude_desktop() -> Result<super::Ask, String> {
    let bridge = super::bundle::find_bridge().ok_or_else(|| {
        let beside = std::env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .map(|dir| dir.join(super::bundle::bridge_name()))
        });
        format!(
            "STAMP's extension for Claude Desktop needs the program {}, which is part of STAMP's \
             packages, beside STAMP ({}). It was not found.",
            super::bundle::bridge_name(),
            beside.map_or_else(String::new, |p| p.display().to_string())
        )
    })?;
    let folder = super::launch::working_folder()
        .ok_or("STAMP could not find a folder to write the extension to.")?;
    let bundle = folder.join("stamp.mcpb");
    super::bundle::write(&bundle, &bridge)
        .map_err(|e| format!("Could not write {}: {e}", bundle.display()))?;
    let opened = super::bundle::open(&bundle)?;
    Ok(super::Ask::Installed { bundle, opened })
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

/// Start the server if it is not running, and return it. It writes the
/// connection file for Claude Desktop's bridge in STAMP's data folder.
#[cfg(not(target_arch = "wasm32"))]
pub fn start_sharing<'a>(
    app: &'a mut StampApp,
    ctx: &egui::Context,
) -> Result<&'a super::server::Server, String> {
    start_sharing_with(app, ctx, super::bridge::connection_file().as_deref())
}

/// [`start_sharing`], with the connection file at `connection` (or none).
#[cfg(not(target_arch = "wasm32"))]
pub fn start_sharing_with<'a>(
    app: &'a mut StampApp,
    ctx: &egui::Context,
    connection: Option<&std::path::Path>,
) -> Result<&'a super::server::Server, String> {
    if app.assistant.server.is_none() {
        let waker = ctx.clone();
        let mut server = super::server::Server::start(move || waker.request_repaint())
            .map_err(|e| format!("Could not start sharing: {e}"))?;
        // For Claude Desktop's bridge. Claude Code is told directly, so
        // sharing goes on without the file if it cannot be written.
        if let Some(path) = connection {
            if let Err(error) = server.publish(path) {
                eprintln!("could not write {}: {error}", path.display());
            }
        }
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
                        "Sharing this session with Claude at {} ({}).",
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
                    ui.label(
                        "Not sharing. Use Assistant > Share with Claude Desktop, or Open Claude \
                         Code, to start.",
                    );
                }
            }
            if let Some(folder) = super::launch::working_folder() {
                ui.label(format!("STAMP's assistant folder: {}", folder.display()));
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
        app.assistant.asking = Some(super::Ask::ClaudeCode);
    }
}
