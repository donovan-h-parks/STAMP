//! The assistant's menu, windows and status-bar controls.

use eframe::egui;

use super::describe;
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
    ui.separator();
    let sharing = app.assistant.server.is_some();
    for agent in Agent::ALL {
        if ui
            .button(icons::label(
                icons::TERMINAL,
                &format!("Open {}", agent.name()),
            ))
            .on_hover_text(format!(
                "Ask {}, in the assistant panel, about your data and plots, and have it \
                 change them. Needs {}. It reads your data and results; see What is shared \
                 in the documentation.",
                agent.name(),
                agent.accounts()
            ))
            .clicked()
        {
            show_agent(app, ui.ctx(), agent);
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
    activity_window(app, ctx);
}

/// Assistant > Open Claude Code (or another assistant): its tab if it is
/// already running in the panel, otherwise [`start_agent`].
pub fn show_agent(app: &mut StampApp, ctx: &egui::Context, agent: Agent) {
    if let Some(index) = app.assistant.tab_of(agent) {
        if !app.assistant.tabs[index].terminal.has_exited() {
            app.assistant.active = index;
            app.assistant.agent = agent;
            app.assistant.show_terminal = true;
            return;
        }
    }
    start_agent(app, ctx, agent);
}

/// Run `agent` in its tab of the assistant panel (a new one, or replacing
/// what ran in its tab before), after its installer if it is not installed.
/// What is shared is described in the documentation (the assistant page).
pub fn start_agent(app: &mut StampApp, ctx: &egui::Context, agent: Agent) {
    let install = agent.find().is_none();
    if let Err(message) = open_agent(app, ctx, agent, install) {
        app.show_notice(message);
    }
}

/// Start sharing (if not already), prepare the working folder for `agent`
/// and run it in the assistant panel, or its installer first when
/// `install` is set.
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

/// Run `program` in the assistant panel, in `folder`, in the tab of `agent`
/// (ending what ran there before) or a new one, and show that tab.
pub fn run_in_panel(
    app: &mut StampApp,
    ctx: &egui::Context,
    agent: Agent,
    label: &str,
    program: &str,
    args: &[String],
    folder: &std::path::Path,
) -> std::io::Result<()> {
    let terminal =
        super::terminal::Terminal::spawn(ctx, label, program, args, folder, Default::default())?;
    let tab = super::Tab { agent, terminal };
    let assistant = &mut app.assistant;
    match assistant.tab_of(agent) {
        Some(index) => {
            assistant.tabs[index] = tab;
            assistant.active = index;
        }
        None => {
            assistant.tabs.push(tab);
            assistant.active = assistant.tabs.len() - 1;
        }
    }
    assistant.agent = agent;
    assistant.show_terminal = true;
    Ok(())
}

/// Open `agent` in the computer's own terminal instead of the panel.
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

/// The assistant panel: a tab for each assistant program opened (Claude
/// Code, OpenCode, Antigravity), on the right of the window to start with;
/// it can be moved. A program ends when it is quit in its tab (`/exit`); its
/// tab then says so, and can be closed or the program restarted.
pub(crate) fn terminal_frame(app: &mut StampApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let mut restart = None;
    let mut outside = None;
    let mut close = None;
    let mut start = false;
    egui::Frame::NONE.show(ui, |ui| {
        ui.horizontal(|ui| {
            let assistant = &mut app.assistant;
            for (index, tab) in assistant.tabs.iter().enumerate() {
                let ended = tab.terminal.has_exited();
                let text = if ended {
                    format!("{} (ended)", tab.terminal.label)
                } else {
                    tab.terminal.label.clone()
                };
                let mut text = egui::RichText::new(text);
                if index == assistant.active {
                    text = text.strong();
                }
                if ui
                    .selectable_label(index == assistant.active, text)
                    .clicked()
                {
                    assistant.active = index;
                    assistant.agent = tab.agent;
                }
                if ended
                    && ui
                        .small_button("x")
                        .on_hover_text("Close this tab")
                        .clicked()
                {
                    close = Some(index);
                }
            }
            if let Some(tab) = assistant.active_tab() {
                let title = tab.terminal.title();
                if !tab.terminal.has_exited() && !title.is_empty() && title != tab.terminal.label {
                    ui.weak(title);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("Hide")
                    .on_hover_text("Hide this panel; the assistants keep running")
                    .clicked()
                {
                    assistant.show_terminal = false;
                }
                match assistant.active_tab() {
                    Some(tab) => {
                        let name = tab.agent.name();
                        if ui
                            .small_button("Restart")
                            .on_hover_text(format!("End {name} and start it again"))
                            .clicked()
                        {
                            restart = Some(tab.agent);
                        }
                        if ui
                            .small_button("Separate terminal")
                            .on_hover_text(format!(
                                "Open {name} in the computer's own terminal instead"
                            ))
                            .clicked()
                        {
                            outside = Some(tab.agent);
                        }
                    }
                    None => {
                        start = ui
                            .small_button(format!("Start {}", assistant.agent.name()))
                            .clicked();
                    }
                }
            });
        });
        let active = app.assistant.active;
        match app.assistant.tabs.get_mut(active) {
            Some(tab) => tab.terminal.show(ui),
            None => {
                ui.label(
                    "No assistant is running. Open one from the Assistant menu; each opens in a \
                     tab of its own here.",
                );
            }
        }
    });
    if let Some(index) = close {
        app.assistant.close_tab(index);
    }
    if start {
        let agent = app.assistant.agent;
        start_agent(app, &ctx, agent);
    }
    // Its tab is replaced once it starts again.
    if let Some(agent) = restart {
        start_agent(app, &ctx, agent);
    }
    if let Some(agent) = outside {
        if let Some(index) = app.assistant.tab_of(agent) {
            app.assistant.close_tab(index);
        }
        if let Err(message) = open_agent_outside(app, &ctx, agent) {
            app.show_notice(message);
        }
    }
}

/// Start the server if it is not running, and return it.
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

fn activity_window(app: &mut StampApp, ctx: &egui::Context) {
    let mut open = app.assistant.show_window;
    let mut stop = false;
    let mut reopen = false;
    egui::Window::new("Assistant activity")
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
        show_agent(app, ctx, agent);
    }
}
