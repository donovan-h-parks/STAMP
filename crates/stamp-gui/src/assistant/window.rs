//! The assistant's menu, windows and status-bar controls.

use eframe::egui;

use super::describe;
#[cfg(not(target_arch = "wasm32"))]
use super::launch::Agent;
use crate::icons;
use crate::StampApp;

/// The Assistant menu.
pub(crate) fn menu(app: &mut StampApp, ui: &mut egui::Ui) {
    let copy = ui
        .button(icons::label(
            icons::CLIPBOARD_TEXT,
            "Copy description for AI",
        ))
        .on_hover_text(
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
            egui::Button::new(icons::label(icons::IMAGE, "Copy plot image")),
        )
        .on_hover_text("Copy the plot as an image, to paste into a chat");
    if image.clicked() {
        app.copy_plot_image(ui.ctx());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.separator();
        let sharing = app.assistant.server.is_some();
        for agent in Agent::ALL {
            if ui
                .button(icons::label(
                    icons::TERMINAL,
                    &format!("Open {}...", agent.name()),
                ))
                .on_hover_text(format!(
                    "Ask {}, in a panel at the bottom of the window, about your data and plots, \
                     and have it change them. Needs {}.",
                    agent.name(),
                    agent.accounts()
                ))
                .clicked()
            {
                open_agent_or_ask(app, ui.ctx(), agent);
            }
        }
        if ui
            .add_enabled(
                sharing,
                egui::Button::new(icons::label(icons::PLUGS, "Stop sharing")),
            )
            .on_hover_text(
                "Stop letting the assistant reach this session. The program keeps running, but \
                 can no longer read or change anything in STAMP.",
            )
            .clicked()
        {
            app.assistant.server = None;
        }
        if ui
            .add(
                egui::Button::new(icons::label(icons::PULSE, "Assistant activity"))
                    .selected(app.assistant.show_window),
            )
            .clicked()
        {
            app.assistant.show_window = !app.assistant.show_window;
        }
    }
    ui.separator();
    if ui
        .add_enabled(
            app.can_undo_assistant_change(),
            egui::Button::new(icons::label(
                icons::ARROW_COUNTER_CLOCKWISE,
                "Undo assistant's last change",
            )),
        )
        .clicked()
    {
        app.undo_assistant_change();
    }
}

/// The assistant's part of the status bar: that an assistant is connected,
/// and an Undo button for its changes.
pub(crate) fn status(app: &mut StampApp, ui: &mut egui::Ui) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(server) = &app.assistant.server {
        let text = if server.clients() > 0 {
            "Assistant connected"
        } else {
            "Sharing with assistant"
        };
        if ui
            .link(text)
            .on_hover_text("STAMP is sharing this session with an AI assistant. Click for details.")
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

/// What sharing means, said before it starts, with `agent`.
#[cfg(not(target_arch = "wasm32"))]
fn sharing(agent: Agent) -> String {
    let who = agent.name();
    let sent_to = match agent {
        Agent::ClaudeCode => "Anthropic to answer your questions, under your Claude account",
        Agent::OpenCode => {
            "OpenCode's own service while you use its free models, or the AI provider you sign \
             in to in OpenCode (such as OpenAI or GitHub Copilot), to answer your questions"
        }
        Agent::Antigravity => "Google to answer your questions, under your Google account",
    };
    format!(
        "{who} will be able to read the loaded profile and metadata, the results and the plots, \
         and to change the analysis and the plots in this window. What it reads is sent to \
         {sent_to}. Sharing stops when you choose Assistant > Stop sharing, or close STAMP."
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn asking_window(app: &mut StampApp, ctx: &egui::Context) {
    let Some(ask) = app.assistant.asking.clone() else {
        return;
    };
    let super::Ask::Agent(agent) = ask;
    let title = format!("Open {}", agent.name());
    let found = agent.find();
    let mut open = true;
    let mut close = false;
    // What the user chose: to open the program, after installing it if
    // `Some(true)`.
    let mut chosen: Option<bool> = None;
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            let name = agent.name();
            ui.label(sharing(agent));
            ui.add_space(6.0);
            match &found {
                Some(path) => {
                    let first_time = match agent {
                        Agent::ClaudeCode => {
                            "The first time, sign in with your Claude account, and when \
                             Claude Code asks whether you trust STAMP's folder, choose \
                             \"Yes, I trust this folder\" (with the arrow keys and \
                             Enter)."
                        }
                        Agent::OpenCode => {
                            "It starts with one of OpenCode's free models. To use your \
                             own account, type /connect and choose how to sign in (for \
                             example OpenAI, then \"Sign in with ChatGPT\", or GitHub \
                             Copilot), then /models to choose a model."
                        }
                        Agent::Antigravity => {
                            "The first time, sign in with your Google account, then choose a \
                             colour scheme, accept Google's terms and confirm that you trust \
                             STAMP's folder (\"Yes, I trust this folder\"), with the arrow \
                             keys and Enter."
                        }
                    };
                    ui.label(format!(
                        "{name} opens in a panel at the bottom of this window. \
                         {first_time} Then ask your questions there.\n\n{name}: {}",
                        path.display()
                    ));
                    ui.add_space(6.0);
                    ui.checkbox(
                        &mut app.assistant.skip_claude_code_dialog,
                        "Don't show this again",
                    )
                    .on_hover_text(
                        "Open the assistant straight away next time. \
                         Settings > Preferences turns this message back on.",
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("Open {name}")).clicked() {
                            chosen = Some(false);
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
                    ui.label(format!(
                        "{name} is not installed. STAMP can install it with {}, in a \
                         panel at the bottom of this window, and then start it. {name} \
                         needs {}; sign in when it starts.",
                        agent.installer(),
                        agent.accounts()
                    ));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Install and open").clicked() {
                            chosen = Some(true);
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
            }
        });
    if !open || close {
        app.assistant.asking = None;
    }
    let Some(install) = chosen else {
        return;
    };
    app.assistant.asking = None;
    let outcome = open_agent(app, ctx, agent, install);
    if outcome.is_ok() {
        app.assistant.show_window = true;
    }
    if let Err(message) = outcome {
        app.show_notice(message);
    }
}

/// Assistant > Open Claude Code (or another assistant): the dialog that says
/// what will be shared, or, if the user chose not to see it again and the
/// program is installed, the program at once. Installing it always asks
/// first.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_agent_or_ask(app: &mut StampApp, ctx: &egui::Context, agent: Agent) {
    if app.assistant.skip_claude_code_dialog && agent.find().is_some() {
        if let Err(message) = open_agent(app, ctx, agent, false) {
            app.show_notice(message);
        }
    } else {
        app.assistant.asking = Some(super::Ask::Agent(agent));
    }
}

/// Start sharing (if not already), prepare the working folder for `agent`
/// and run it in the assistant panel, or its installer first when
/// `install` is set.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_agent(
    app: &mut StampApp,
    ctx: &egui::Context,
    agent: Agent,
    install: bool,
) -> Result<(), String> {
    let name = agent.name();
    let server = start_sharing(app, ctx)?;
    let (url, token) = (server.url(), server.token().to_string());
    let folder = super::launch::working_folder()
        .ok_or_else(|| format!("STAMP could not find a folder to run {name} in."))?;
    let found = if install {
        None
    } else {
        Some(
            agent
                .find()
                .ok_or_else(|| format!("{name} was not found."))?,
        )
    };
    let written = match &found {
        Some(program) => agent.prepare(&folder, &url, &token, program),
        None => agent.prepare_install(&folder, &url, &token),
    };
    let script = written.map_err(|e| {
        format!(
            "Could not prepare {name}'s folder {}: {e}",
            folder.display()
        )
    })?;
    let (program, args) = match &found {
        Some(program) => agent.command(program),
        None => super::launch::script_command(&script),
    };
    let label = if install {
        format!("Installing {name}")
    } else {
        name.to_string()
    };
    run_in_panel(app, ctx, agent, &label, &program, &args, &folder)
        .map_err(|e| format!("Could not start {name}: {e}"))
}

/// Run `program` in the assistant panel, in `folder`, ending whatever ran
/// there before, and show the panel, named after `agent`.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_in_panel(
    app: &mut StampApp,
    ctx: &egui::Context,
    agent: Agent,
    label: &str,
    program: &str,
    args: &[String],
    folder: &std::path::Path,
) -> std::io::Result<()> {
    app.assistant.terminal = None;
    let terminal =
        super::terminal::Terminal::spawn(ctx, label, program, args, folder, Default::default())?;
    app.assistant.terminal = Some(terminal);
    app.assistant.agent = agent;
    app.assistant.show_terminal = true;
    Ok(())
}

/// Open `agent` in the computer's own terminal instead of the panel.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_agent_outside(
    app: &mut StampApp,
    ctx: &egui::Context,
    agent: Agent,
) -> Result<(), String> {
    let name = agent.name();
    let server = start_sharing(app, ctx)?;
    let (url, token) = (server.url(), server.token().to_string());
    let folder = super::launch::working_folder()
        .ok_or_else(|| format!("STAMP could not find a folder to run {name} in."))?;
    let written = match agent.find() {
        Some(program) => agent.prepare(&folder, &url, &token, &program),
        None => agent.prepare_install(&folder, &url, &token),
    };
    let script = written.map_err(|e| {
        format!(
            "Could not prepare {name}'s folder {}: {e}",
            folder.display()
        )
    })?;
    super::launch::open_terminal(&script)
}

/// The assistant panel, which runs Claude Code, OpenCode or Antigravity (at the bottom
/// of the window to start with, like the Metadata table; it can be moved).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn terminal_frame(app: &mut StampApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let agent = app.assistant.agent;
    let name = agent.name();
    let mut restart = false;
    let mut outside = false;
    egui::Frame::NONE.show(ui, |ui| {
        ui.horizontal(|ui| {
            match &app.assistant.terminal {
                Some(terminal) => {
                    ui.label(egui::RichText::new(&terminal.label).strong());
                    let title = terminal.title();
                    if terminal.has_exited() {
                        ui.label("has ended.");
                    } else if !title.is_empty() && title != terminal.label {
                        ui.weak(title);
                    }
                }
                None => {
                    ui.label(egui::RichText::new(name).strong());
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("Hide")
                    .on_hover_text(format!("Hide this panel; {name} keeps running"))
                    .clicked()
                {
                    app.assistant.show_terminal = false;
                }
                let again = if app.assistant.terminal.is_some() {
                    "Restart"
                } else {
                    "Start"
                };
                restart = ui.small_button(again).clicked();
                outside = ui
                    .small_button("Separate terminal")
                    .on_hover_text(format!(
                        "Open {name} in the computer's own terminal instead"
                    ))
                    .clicked();
            });
        });
        match &mut app.assistant.terminal {
            Some(terminal) => terminal.show(ui),
            None => {
                ui.label(format!(
                    "{name} is not running. Choose Start, or open an assistant from the \
                     Assistant menu."
                ));
            }
        }
    });
    if restart {
        open_agent_or_ask(app, &ctx, agent);
    }
    if outside {
        app.assistant.terminal = None;
        if let Err(message) = open_agent_outside(app, &ctx, agent) {
            app.show_notice(message);
        }
    }
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
            .map_err(|e| format!("Could not start sharing: {e}"))?;
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
                        "Sharing this session at {} ({}).",
                        server.url(),
                        if server.clients() > 0 {
                            "connected"
                        } else {
                            "not connected yet"
                        }
                    ));
                    ui.horizontal(|ui| {
                        stop = ui.button("Stop sharing").clicked();
                        reopen = ui
                            .button(format!("Open {} again", app.assistant.agent.name()))
                            .clicked();
                    });
                }
                None => {
                    ui.label("Not sharing. Open an assistant from the Assistant menu to start.");
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
        let agent = app.assistant.agent;
        open_agent_or_ask(app, ctx, agent);
    }
}
