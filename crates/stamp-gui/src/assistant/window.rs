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
                "Ask Claude Code, in a panel at the bottom of the window, about your data and \
                 plots, and have it change them. Needs a Claude Pro, Max, Team or Enterprise \
                 plan.",
            )
            .clicked()
        {
            open_claude_code_or_ask(app, ui.ctx());
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
                                "Claude Code opens in a panel at the bottom of this window. \
                                 The first time, sign in with your Claude account, and when \
                                 Claude Code asks whether you trust STAMP's folder, choose \
                                 \"Yes, I trust this folder\" (with the arrow keys and Enter). \
                                 Then ask your questions there.\n\nClaude Code: {}",
                                path.display()
                            ));
                            ui.add_space(6.0);
                            ui.checkbox(
                                &mut app.assistant.skip_claude_code_dialog,
                                "Don't show this again",
                            )
                            .on_hover_text(
                                "Open Claude Code straight away next time. Settings > \
                                 Preferences turns this message back on.",
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("Open Claude Code").clicked() {
                                    chosen = Some(Choice::OpenClaudeCode { install: false });
                                }
                                if ui.button("Cancel").clicked() {
                                    // Cancelling does not stop the dialog
                                    // from being shown next time.
                                    app.assistant.skip_claude_code_dialog = false;
                                    close = true;
                                }
                            });
                        }
                        None => {
                            ui.label(
                                "Claude Code is not installed. STAMP can install it with \
                                 Anthropic's installer (claude.ai/install), in a panel at the \
                                 bottom of this window, and then start it. Claude Code needs a \
                                 Claude Pro, Max, Team or Enterprise plan; sign in with your \
                                 Claude account when it starts.",
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

/// Assistant > Open Claude Code: the dialog that says what will be shared,
/// or, if the user chose not to see it again and Claude Code is installed,
/// Claude Code at once. Installing it always asks first.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_claude_code_or_ask(app: &mut StampApp, ctx: &egui::Context) {
    if app.assistant.skip_claude_code_dialog && super::launch::find_claude().is_some() {
        if let Err(message) = open_claude_code(app, ctx, false) {
            app.show_notice(message);
        }
    } else {
        app.assistant.asking = Some(super::Ask::ClaudeCode);
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
    let (program, args) = if install {
        super::launch::script_command(&script)
    } else {
        let claude = super::launch::find_claude().ok_or("Claude Code was not found.")?;
        super::launch::claude_command(&claude)
    };
    let label = if install {
        "Installing Claude Code"
    } else {
        "Claude Code"
    };
    run_in_panel(app, ctx, label, &program, &args, &folder)
        .map_err(|e| format!("Could not start Claude Code: {e}"))
}

/// Run `program` in the Claude Code panel, in `folder`, ending whatever
/// ran there before, and show the panel.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_in_panel(
    app: &mut StampApp,
    ctx: &egui::Context,
    label: &str,
    program: &str,
    args: &[String],
    folder: &std::path::Path,
) -> std::io::Result<()> {
    app.assistant.terminal = None;
    let terminal =
        super::terminal::Terminal::spawn(ctx, label, program, args, folder, Default::default())?;
    app.assistant.terminal = Some(terminal);
    app.assistant.show_terminal = true;
    Ok(())
}

/// Open Claude Code in the computer's own terminal instead of the panel.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_claude_code_outside(app: &mut StampApp, ctx: &egui::Context) -> Result<(), String> {
    let server = start_sharing(app, ctx)?;
    let (url, token) = (server.url(), server.token().to_string());
    let folder = super::launch::working_folder()
        .ok_or("STAMP could not find a folder to run Claude Code in.")?;
    let written = match super::launch::find_claude() {
        Some(claude) => super::launch::prepare(&folder, &url, &token, &claude),
        None => super::launch::prepare_install(&folder, &url, &token),
    };
    let script = written.map_err(|e| {
        format!(
            "Could not prepare Claude Code's folder {}: {e}",
            folder.display()
        )
    })?;
    super::launch::open_terminal(&script)
}

/// The Claude Code panel, at the bottom of the window like the Metadata
/// table.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn terminal_panel(app: &mut StampApp, ui: &mut egui::Ui) {
    if !app.assistant.show_terminal {
        return;
    }
    let ctx = ui.ctx().clone();
    let mut restart = false;
    let mut stop = false;
    let mut outside = false;
    egui::Panel::bottom("claude-code")
        .resizable(true)
        .default_size(320.0)
        .min_size(120.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                match &app.assistant.terminal {
                    Some(terminal) => {
                        ui.label(egui::RichText::new(&terminal.label).strong());
                        let title = terminal.title();
                        if terminal.has_exited() {
                            ui.label("has ended.");
                        } else if !title.is_empty() {
                            ui.weak(title);
                        }
                    }
                    None => {
                        ui.label(egui::RichText::new("Claude Code").strong());
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("Hide")
                        .on_hover_text("Hide this panel; Claude Code keeps running")
                        .clicked()
                    {
                        app.assistant.show_terminal = false;
                    }
                    let running = app
                        .assistant
                        .terminal
                        .as_ref()
                        .is_some_and(|t| !t.has_exited());
                    if running && ui.small_button("Stop").clicked() {
                        stop = true;
                    }
                    let again = if app.assistant.terminal.is_some() {
                        "Restart"
                    } else {
                        "Start"
                    };
                    restart = ui.small_button(again).clicked();
                    outside = ui
                        .small_button("Separate terminal")
                        .on_hover_text("Open Claude Code in the computer's own terminal instead")
                        .clicked();
                });
            });
            match &mut app.assistant.terminal {
                Some(terminal) => terminal.show(ui),
                None => {
                    ui.label(
                        "Claude Code is not running. Choose Start, or Assistant > Open Claude \
                         Code.",
                    );
                }
            }
        });
    if stop {
        app.assistant.terminal = None;
    }
    if restart {
        open_claude_code_or_ask(app, &ctx);
    }
    if outside {
        app.assistant.terminal = None;
        if let Err(message) = open_claude_code_outside(app, &ctx) {
            app.show_notice(message);
        }
    }
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
        open_claude_code_or_ask(app, ctx);
    }
}
