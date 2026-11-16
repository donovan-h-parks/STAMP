//! The assistant: lets a language model see and change the session.
//!
//! - [`describe`] turns the session, results and plot into text, for "Copy
//!   description for AI" and for the tools.
//! - [`tools`] is what a model may ask of STAMP.
//! - On the desktop, `server` answers those requests over the Model Context
//!   Protocol on this computer only, and `launch` starts Claude Code,
//!   OpenCode or Google's Antigravity CLI connected to it, in the panel
//!   `terminal` draws.
//!
//! See `docs/ASSISTANT_PLAN.md`.

pub mod describe;
pub mod tools;
pub mod window;

#[cfg(not(target_arch = "wasm32"))]
pub mod launch;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
#[cfg(not(target_arch = "wasm32"))]
pub mod terminal;

use std::collections::VecDeque;

use eframe::egui;

use crate::StampApp;

/// Tool calls kept in the log.
const MAX_LOG: usize = 500;

/// The assistant's part of the window.
#[derive(Default)]
pub struct State {
    /// Requests not yet answered, oldest first, each with its answer once
    /// a change has been made and the window is catching up with it.
    pub(crate) queue: VecDeque<(Request, Option<tools::Outcome>)>,
    pub(crate) log: Vec<LogEntry>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) server: Option<server::Server>,
    /// The Assistant window is open.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) show_window: bool,
    /// The dialog being shown, if any.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) asking: Option<Ask>,
    /// The assistant program (or its installer) running in the assistant
    /// panel.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) terminal: Option<terminal::Terminal>,
    /// The program the assistant panel runs, the last one opened
    /// (remembered between runs).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) agent: launch::Agent,
    /// The assistant panel is shown.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) show_terminal: bool,
    /// Open an assistant without its dialog (remembered between runs).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) skip_claude_code_dialog: bool,
}

/// The assistant's dialogs.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ask {
    /// Asking before opening an assistant program (and sharing with it).
    Agent(launch::Agent),
}

/// Answer the requests waiting, in order, as far as the window is up to
/// date. A request that changes the session is carried out at once and
/// answered once the analyses and the plot have caught up with it.
pub fn serve(app: &mut StampApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(server) = &app.assistant.server {
        while let Some(request) = server.next_request() {
            app.assistant.queue.push_back((request, None));
        }
    }
    while !app.assistant.queue.is_empty() {
        if !app.is_settled() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }
        let Some((request, answer)) = app.assistant.queue.pop_front() else {
            return;
        };
        let outcome = match answer {
            Some(mut outcome) => {
                outcome
                    .content
                    .push(tools::Content::Text(tools::settled_summary(app)));
                outcome
            }
            None => {
                let outcome = tools::call(app, &request.tool, &request.args);
                if tools::changes(&request.tool) && !outcome.is_error {
                    app.assistant.queue.push_front((request, Some(outcome)));
                    ctx.request_repaint();
                    continue;
                }
                outcome
            }
        };
        app.assistant.log.push(LogEntry::new(&request, &outcome));
        if app.assistant.log.len() > MAX_LOG {
            app.assistant.log.remove(0);
        }
        // The server may have given up waiting.
        let _ = request.reply.send(outcome);
    }
}

/// A request from the model, waiting for the window to answer it.
pub struct Request {
    pub tool: String,
    pub args: serde_json::Value,
    /// Where the answer goes.
    pub reply: std::sync::mpsc::Sender<tools::Outcome>,
}

/// One tool call, as listed in the Assistant window.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub tool: String,
    /// The arguments, as JSON.
    pub args: String,
    /// The first line of the answer.
    pub answer: String,
    pub is_error: bool,
}

impl LogEntry {
    pub fn new(request: &Request, outcome: &tools::Outcome) -> LogEntry {
        let text = outcome.joined_text();
        let mut answer = text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .to_string();
        if outcome
            .content
            .iter()
            .any(|c| matches!(c, tools::Content::Png(_)))
        {
            answer.push_str(" (with an image of the plot)");
        }
        LogEntry {
            tool: request.tool.clone(),
            args: request.args.to_string(),
            answer,
            is_error: outcome.is_error,
        }
    }
}

/// Standard base64 with padding, for images sent to the model.
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_rfc_4648() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(super::base64(input.as_bytes()), expected);
        }
    }
}
