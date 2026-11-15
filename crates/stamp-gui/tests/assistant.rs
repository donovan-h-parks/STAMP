//! The assistant's tools, driven through the window as the server drives
//! them: requests are queued, the window answers them between frames, and
//! changes are answered once the analyses and the plot have caught up.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

use serde_json::{json, Value};
use stamp_gui::assistant::tools::{Content, Outcome};
use stamp_gui::assistant::Request;
use stamp_gui::headless::Renderer;
use stamp_gui::plots::PlotKind;
use stamp_gui::session::Mode;
use stamp_gui::StampApp;

fn example(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(relative);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    stamp_core::text::decode(&bytes).into_owned()
}

struct Harness {
    renderer: Renderer,
    app: StampApp,
}

impl Harness {
    fn enterotypes() -> Harness {
        let renderer = Renderer::new(1280.0, 800.0, 1.0);
        let mut app = StampApp::new(&renderer.ctx);
        app.load_profile(
            "Enterotypes.profile.spf",
            &example("EnterotypesArumugam/Enterotypes.profile.spf"),
        );
        app.load_metadata(
            "Enterotypes.metadata.tsv",
            &example("EnterotypesArumugam/Enterotypes.metadata.tsv"),
        );
        let mut harness = Harness { renderer, app };
        harness.settle();
        harness
    }

    fn frames(&mut self, n: usize) {
        let app = &mut self.app;
        self.renderer.run(n, |ui| app.draw(ui));
    }

    fn settle(&mut self) {
        for _ in 0..2000 {
            self.frames(3);
            if self.app.is_settled() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the window did not settle");
    }

    /// Queue a tool call as the server does.
    fn send(&mut self, tool: &str, args: Value) -> Receiver<Outcome> {
        let (reply, answer) = channel();
        self.app.assistant_request(Request {
            tool: tool.to_string(),
            args,
            reply,
        });
        answer
    }

    /// Call a tool and run frames until it is answered.
    fn call(&mut self, tool: &str, args: Value) -> Outcome {
        let answer = self.send(tool, args);
        for _ in 0..4000 {
            self.frames(1);
            if let Ok(outcome) = answer.try_recv() {
                return outcome;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("{tool} was not answered");
    }
}

fn text(outcome: &Outcome) -> String {
    outcome.joined_text()
}

#[test]
fn read_tools_describe_what_the_window_shows() {
    let mut h = Harness::enterotypes();
    assert_eq!(h.app.mode, Mode::TwoGroups);

    let session = h.call("get_session", json!({}));
    assert!(!session.is_error);
    let session = text(&session);
    assert!(session.contains("Enterotypes.profile.spf"), "{session}");
    assert!(session.contains("## Analysis: Two groups"), "{session}");
    assert!(session.contains("Group field: Enterotype"), "{session}");
    assert!(session.contains("Welch's t-test"), "{session}");

    let plot = text(&h.call("get_current_plot", json!({})));
    assert!(
        plot.contains("Extended error bar (id `ExtendedErrorBar`)"),
        "{plot}"
    );
    assert!(plot.contains("## Items in the plot"), "{plot}");
    assert!(plot.contains("\"error_bar\""), "{plot}");

    let png = h.call("get_current_plot", json!({"format": "png"}));
    let image = png.content.iter().find_map(|c| match c {
        Content::Png(bytes) => Some(bytes.clone()),
        Content::Text(_) => None,
    });
    assert!(image.unwrap().starts_with(b"\x89PNG"));

    let svg = text(&h.call("get_current_plot", json!({"format": "svg"})));
    assert!(svg.contains("<svg"));

    let results = text(&h.call("get_results", json!({"rows": 3})));
    let table: Vec<&str> = results
        .split("```tsv\n")
        .nth(1)
        .unwrap()
        .lines()
        .take_while(|l| !l.starts_with("```"))
        .collect();
    assert_eq!(table.len(), 4, "heading and three rows: {results}");
    assert!(
        table[0].starts_with("Genus") || table[0].contains("p-values"),
        "{results}"
    );

    let samples = text(&h.call("get_samples", json!({})));
    assert!(samples.starts_with("Sample\tSwitched on\tTotal count\tEnterotype"));
    assert!(samples.contains("AM-AD-1\tyes"));

    let plots = text(&h.call("list_plots", json!({})));
    assert!(plots.contains("- Volcano: Volcano plot"), "{plots}");

    let options = text(&h.call("get_plot_options", json!({"plot": "volcano"})));
    assert!(options.contains("\"volcano\""), "{options}");

    // Reading changes nothing and leaves nothing to undo.
    assert!(!h.app.can_undo_assistant_change());
    let log: Vec<&str> = h
        .app
        .assistant_log()
        .iter()
        .map(|e| e.tool.as_str())
        .collect();
    assert_eq!(log[0], "get_session");
    assert_eq!(log.len(), 8);
}

#[test]
fn show_plot_changes_the_plot_and_can_be_undone() {
    let mut h = Harness::enterotypes();
    let before = h.app.plot_settings().clone();
    let outcome = h.call(
        "show_plot",
        json!({"plot": "Volcano", "options": {"min_p_exponent": -8}}),
    );
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert!(text(&outcome).contains("Now showing: Two groups analysis, Volcano plot"));
    assert_eq!(h.app.plot_kind(), PlotKind::Volcano);
    assert_eq!(h.app.plot_settings().min_p_exponent, -8);
    assert!(
        stamp_gui::assistant::describe::figure_message(h.app.figure()).is_none(),
        "the volcano plot was drawn"
    );

    // A feature for the single-feature plots.
    let feature = h.app.feature_table().last().unwrap().feature.clone();
    let outcome = h.call("show_plot", json!({"plot": "Box", "feature": feature}));
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert_eq!(
        h.app.session.selected_feature.as_deref(),
        Some(feature.as_str())
    );
    assert!(text(&outcome).contains(&format!("Feature: {feature}")));

    let outcome = h.call("undo", json!({}));
    assert!(!outcome.is_error);
    h.settle();
    assert_eq!(h.app.plot_kind(), PlotKind::Volcano);
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(h.app.plot_kind(), PlotKind::ExtendedErrorBar);
    assert_eq!(*h.app.plot_settings(), before);
    assert!(!h.app.can_undo_assistant_change());
}

#[test]
fn mistakes_are_explained_and_change_nothing() {
    let mut h = Harness::enterotypes();
    for (tool, args, expected) in [
        (
            "show_plot",
            json!({"plot": "Pie chart"}),
            "no plot Pie chart",
        ),
        (
            "show_plot",
            json!({"plot": "PostHoc"}),
            "not offered in the Two groups",
        ),
        (
            "show_plot",
            json!({"options": {"volcano": {"sparkle": true}}}),
            "volcano.sparkle",
        ),
        (
            "show_plot",
            json!({"feature": "Unicornia"}),
            "No feature named Unicornia",
        ),
        ("set_analysis", json!({"test": "magic"}), "Choices"),
        (
            "set_analysis",
            json!({"group1": "Martians"}),
            "Unknown group",
        ),
        (
            "set_analysis",
            json!({"sample1": "AM-AD-1"}),
            "belongs to the Two samples",
        ),
        ("set_analysis", json!({"coverage": 7}), "between 0.5 and 1"),
        (
            "set_analysis",
            json!({"flavour": "mint"}),
            "no setting flavour",
        ),
        ("no_such_tool", json!({}), "no tool named"),
    ] {
        let outcome = h.call(tool, args);
        assert!(outcome.is_error, "{tool}: {}", text(&outcome));
        assert!(
            text(&outcome).contains(expected),
            "{tool}: {}",
            text(&outcome)
        );
    }
    assert!(!h.app.can_undo_assistant_change());
    assert_eq!(h.app.plot_kind(), PlotKind::ExtendedErrorBar);
}

#[test]
fn set_analysis_runs_the_analysis_again_before_answering() {
    let mut h = Harness::enterotypes();
    let outcome = h.call(
        "set_analysis",
        json!({"test": "mann whitney u test", "correction": "Bonferroni"}),
    );
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert_eq!(
        h.app.session.groups.analysis.test,
        stamp_core::stats::two_group::GroupTest::MannWhitney
    );
    let session = text(&h.call("get_session", json!({})));
    assert!(session.contains("Mann-Whitney"), "{session}");
    assert!(session.contains("Bonferroni"), "{session}");
    assert!(h.app.session.groups.output.is_some());

    let outcome = h.call(
        "set_analysis",
        json!({"mode": "Multiple groups", "groups": ["enterotype 1", "Enterotype 2", "Enterotype 3"]}),
    );
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert_eq!(h.app.mode, Mode::MultipleGroups);
    assert_eq!(
        h.app.session.multi.active_groups,
        ["Enterotype 1", "Enterotype 2", "Enterotype 3"]
    );
    assert!(text(&outcome).contains("Multiple groups analysis"));

    // Both changes are undone, in turn.
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(h.app.mode, Mode::TwoGroups);
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(
        h.app.session.groups.analysis.test,
        stamp_core::stats::two_group::GroupTest::Welch
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn claude_code_reaches_the_tools_over_http() {
    use std::io::{Read, Write};

    let mut h = Harness::enterotypes();
    let ctx = h.renderer.ctx.clone();
    let server = stamp_gui::assistant::window::start_sharing(&mut h.app, &ctx).unwrap();
    let (port, token) = (server.port(), server.token().to_string());

    let client = std::thread::spawn(move || {
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "show_plot", "arguments": {"plot": "Pca"}}
        })
        .to_string();
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(
            stream,
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    });
    // The window answers between frames.
    for _ in 0..4000 {
        h.frames(1);
        if client.is_finished() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let response = client.join().unwrap();
    let body: Value = serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["result"]["isError"], false, "{body}");
    let said = body["result"]["content"][1]["text"].as_str().unwrap();
    assert!(said.contains("PCA plot"), "{said}");
    assert_eq!(h.app.plot_kind(), PlotKind::Pca);
}

/// Not a test: serves the Enterotypes example to a real Claude Code for
/// `STAMP_ASSISTANT_SECONDS` seconds, with its working folder written to
/// `STAMP_ASSISTANT_FOLDER`. Run with `--ignored --nocapture`, then run
/// the folder's start script (or `claude -p ... --mcp-config
/// stamp-mcp.json` there).
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore]
fn serve_for_claude_code() {
    let seconds: u64 = std::env::var("STAMP_ASSISTANT_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);
    let folder = PathBuf::from(std::env::var("STAMP_ASSISTANT_FOLDER").expect("a folder"));
    let mut h = Harness::enterotypes();
    let ctx = h.renderer.ctx.clone();
    let server = stamp_gui::assistant::window::start_sharing(&mut h.app, &ctx).unwrap();
    let claude = stamp_gui::assistant::launch::find_claude().expect("Claude Code is installed");
    let script =
        stamp_gui::assistant::launch::prepare(&folder, &server.url(), server.token(), &claude)
            .unwrap();
    println!("serving at {}; script {}", server.url(), script.display());
    let end = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while std::time::Instant::now() < end {
        h.frames(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for entry in h.app.assistant_log() {
        println!("{} {} -> {}", entry.tool, entry.args, entry.answer);
    }
}
