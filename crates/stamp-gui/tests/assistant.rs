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

    /// The coal seam gas data, with the company of each sample.
    fn coal() -> Harness {
        let renderer = Renderer::new(1280.0, 800.0, 1.0);
        let mut app = StampApp::new(&renderer.ctx);
        app.load_profile("coal.spf", &example("CoalSeamGas/coal.spf"));
        app.load_metadata(
            "coal.metadata.tsv",
            &example("CoalSeamGas/coal.metadata.tsv"),
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
fn samples_are_switched_off_by_metadata_and_undone() {
    // The coal seam gas data, whose metadata names the company of each
    // sample.
    let mut h = Harness::coal();
    let on = |h: &Harness| {
        h.app
            .session
            .metadata
            .as_ref()
            .unwrap()
            .active_samples()
            .len()
    };
    assert_eq!(on(&h), 44);

    // "Remove samples from the QS company": the value is matched however
    // it is written.
    let outcome = h.call(
        "switch_samples",
        json!({"where": {"field": "company", "relation": "=", "value": "qs"}}),
    );
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert!(
        text(&outcome).contains("Switched off 4 sample(s); 40 of 44"),
        "{}",
        text(&outcome)
    );
    assert_eq!(on(&h), 40);
    let samples = text(&h.call("get_samples", json!({})));
    let qs_off = samples
        .lines()
        .filter(|l| l.split('\t').nth(6) == Some("QS"))
        .all(|l| l.split('\t').nth(1) == Some("no"));
    assert!(qs_off, "{samples}");

    // By name, and all back on.
    let first = samples
        .lines()
        .nth(1)
        .unwrap()
        .split('\t')
        .next()
        .unwrap()
        .to_string();
    let outcome = h.call("switch_samples", json!({"samples": [first]}));
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert_eq!(on(&h), 39);
    let outcome = h.call("switch_samples", json!({"all": true, "active": true}));
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert_eq!(on(&h), 44);

    for (args, expected) in [
        (
            json!({"where": {"field": "Company", "value": "Acme"}}),
            "Unknown value of Company",
        ),
        (
            json!({"where": {"field": "Planet", "value": "Mars"}}),
            "Unknown metadata field",
        ),
        (json!({"samples": ["no-such-sample"]}), "Unknown sample"),
        (json!({"all": true, "samples": []}), "exactly one of"),
    ] {
        let outcome = h.call("switch_samples", args);
        assert!(
            outcome.is_error && text(&outcome).contains(expected),
            "{}",
            text(&outcome)
        );
    }

    // Undone in turn: all on, one off, then the four of QS off, then none.
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(on(&h), 39);
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(on(&h), 40);
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(on(&h), 44);
}

#[test]
fn plot_fonts_are_changed_and_undone() {
    let mut h = Harness::enterotypes();
    let before = h.app.plot_settings().fonts;
    let outcome = h.call(
        "set_plot_fonts",
        json!({"tick_labels": 12, "axis_labels": 14, "legend": 10, "font": "times new roman"}),
    );
    assert!(!outcome.is_error, "{}", text(&outcome));
    let fonts = h.app.plot_settings().fonts;
    assert_eq!(
        (fonts.tick, fonts.axis_label, fonts.legend),
        (12.0, 14.0, 10.0)
    );
    assert_eq!(fonts.family, stamp_plot::text::PlotFont::TimesNewRoman);
    // The figure is drawn in it.
    assert_eq!(
        h.app.figure().font,
        stamp_plot::text::PlotFont::TimesNewRoman
    );
    for (args, expected) in [
        (json!({"tick_labels": 100}), "between 4 and 36"),
        (json!({"font": "Comic Sans"}), "Unknown font"),
        (json!({}), "at least one"),
    ] {
        let outcome = h.call("set_plot_fonts", args);
        assert!(
            outcome.is_error && text(&outcome).contains(expected),
            "{}",
            text(&outcome)
        );
    }
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(h.app.plot_settings().fonts, before);
}

#[test]
fn group_colours_can_be_changed_and_undone() {
    let mut h = Harness::enterotypes();
    let before = h.app.session.group_color("Enterotype 3");
    let outcome = h.call(
        "set_group_colors",
        json!({"colors": {"enterotype 3": "red", "Enterotype 1": "#123456"}}),
    );
    assert!(!outcome.is_error, "{}", text(&outcome));
    assert_eq!(
        h.app.session.group_color("Enterotype 3"),
        stamp_plot::Color::from_hex("#e41a1c").unwrap()
    );
    assert_eq!(
        h.app.session.group_color("Enterotype 1"),
        stamp_plot::Color::rgb(0x12, 0x34, 0x56)
    );
    let session = text(&h.call("get_session", json!({})));
    assert!(
        session.contains("Enterotype 3 (18 samples, colour #e41a1c)"),
        "{session}"
    );
    for (args, expected) in [
        (json!({"colors": {"Martians": "red"}}), "Unknown group"),
        (
            json!({"colors": {"Enterotype 2": "sparkly"}}),
            "not a colour",
        ),
    ] {
        let outcome = h.call("set_group_colors", args);
        assert!(
            outcome.is_error && text(&outcome).contains(expected),
            "{}",
            text(&outcome)
        );
    }
    h.app.undo_assistant_change();
    h.settle();
    assert_eq!(h.app.session.group_color("Enterotype 3"), before);
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
    let server = stamp_gui::assistant::window::start_sharing_with(&mut h.app, &ctx, None).unwrap();
    let (port, token) = (server.port(), server.token().to_string());

    let client = std::thread::spawn(move || {
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "show_plot", "arguments": {"plot": "Pcoa"}}
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
    assert!(said.contains("PCoA plot"), "{said}");
    assert_eq!(h.app.plot_kind(), PlotKind::Pcoa);
}

/// Claude Desktop's side: it starts `stamp-mcp` and talks to it over
/// standard input and output; the bridge answers by itself until STAMP
/// shares, and then passes tool calls on to the window.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn claude_desktop_reaches_the_tools_through_the_bridge() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    let folder = std::env::temp_dir().join(format!("stamp-bridge-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let connection = folder.join("connection.json");
    let mut bridge = Command::new(env!("CARGO_BIN_EXE_stamp-mcp"))
        .env("STAMP_ASSISTANT_CONNECTION", &connection)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = bridge.stdin.take().unwrap();
    let mut output = BufReader::new(bridge.stdout.take().unwrap());
    // Each message is answered on a line of its own, on a thread so that
    // the window can run frames meanwhile.
    let (to_bridge, messages) = channel::<String>();
    let (from_bridge, answers) = channel::<Value>();
    let talker = std::thread::spawn(move || {
        for message in messages {
            writeln!(input, "{message}").unwrap();
            input.flush().unwrap();
            let mut line = String::new();
            output.read_line(&mut line).unwrap();
            from_bridge
                .send(serde_json::from_str(&line).unwrap())
                .unwrap();
        }
    });

    let mut h = Harness::enterotypes();
    let ask = |h: &mut Harness, message: Value| -> Value {
        to_bridge.send(message.to_string()).unwrap();
        for _ in 0..4000 {
            h.frames(1);
            if let Ok(answer) = answers.try_recv() {
                return answer;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("the bridge did not answer {message}");
    };
    let call = |id: u32, name: &str, arguments: Value| {
        json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}})
    };

    let reply = ask(
        &mut h,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                          "clientInfo": {"name": "test", "version": "1"}}}),
    );
    assert_eq!(reply["result"]["serverInfo"]["name"], "stamp");

    // Not sharing yet: Claude is told what to ask the user.
    let reply = ask(&mut h, call(2, "get_session", json!({})));
    assert_eq!(reply["result"]["isError"], true);
    let said = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(said.contains("Share with Claude Desktop"), "{said}");

    // Sharing: the connection file appears, and calls reach the window.
    let ctx = h.renderer.ctx.clone();
    stamp_gui::assistant::window::start_sharing_with(&mut h.app, &ctx, Some(&connection)).unwrap();
    assert!(connection.is_file());
    let reply = ask(&mut h, call(3, "show_plot", json!({"plot": "Volcano"})));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(h.app.plot_kind(), PlotKind::Volcano);
    let reply = ask(
        &mut h,
        call(4, "get_current_plot", json!({"format": "png"})),
    );
    assert_eq!(reply["result"]["content"][1]["type"], "image", "{reply}");

    // Stopping removes the file, and the bridge says so again.
    h.app.assistant_stop_sharing();
    assert!(!connection.exists());
    let reply = ask(&mut h, call(5, "get_session", json!({})));
    assert_eq!(reply["result"]["isError"], true);

    drop(to_bridge);
    talker.join().unwrap();
    assert!(bridge.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&folder);
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
    // The bridge finds this session through STAMP_ASSISTANT_CONNECTION, if set.
    let connection = std::env::var_os("STAMP_ASSISTANT_CONNECTION").map(PathBuf::from);
    let server =
        stamp_gui::assistant::window::start_sharing_with(&mut h.app, &ctx, connection.as_deref())
            .unwrap();
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

/// The Claude Code panel is a real terminal: a shell started in it shows
/// its output, takes typed commands, and is interrupted by Ctrl+C.
#[cfg(unix)]
#[test]
fn the_claude_code_panel_runs_a_terminal() {
    use eframe::egui;

    let renderer = Renderer::new(1280.0, 800.0, 1.0);
    let app = StampApp::new(&renderer.ctx);
    let mut h = Harness { renderer, app };
    let folder = std::env::temp_dir();
    let ctx = h.renderer.ctx.clone();
    // STAMP_TEST_SHELL tries another shell (CI has dash on Ubuntu).
    let shell = std::env::var("STAMP_TEST_SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    stamp_gui::assistant::window::run_in_panel(&mut h.app, &ctx, "Shell", &shell, &[], &folder)
        .unwrap();
    // Wait (up to half a minute: CI machines can be slow) until the screen
    // shows `wanted`. Commands print a computed value, such as
    // `$((6*7))`, so that what is waited for is their output, never the
    // command as typed and echoed, whatever the shell (dash on Ubuntu,
    // bash on macOS).
    let wait_for = |h: &mut Harness, wanted: &str| {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while std::time::Instant::now() < end {
            h.frames(1);
            if h.app.assistant_terminal_text().unwrap().contains(wanted) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!(
            "the terminal never showed {wanted}:\n{}",
            h.app.assistant_terminal_text().unwrap()
        );
    };
    let typed = |h: &mut Harness, events: Vec<egui::Event>| {
        let app = &mut h.app;
        h.renderer.send_events(events, |ui| app.draw(ui));
    };
    let key = |key: egui::Key, modifiers: egui::Modifiers| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    h.frames(3);
    // The panel is at the bottom of the window; clicking it gives it the
    // keyboard.
    let app = &mut h.app;
    h.renderer
        .click(egui::pos2(640.0, 700.0), |ui| app.draw(ui));
    assert!(h.app.assistant_terminal_has_focus());

    typed(
        &mut h,
        vec![
            // A known prompt, to wait for after an interruption.
            egui::Event::Text("PS1='ready> '; echo stamp-$((6*7))".to_string()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "stamp-42");

    // Ctrl+C (which egui reports as Copy with nothing selected) interrupts
    // a command that would otherwise run for a minute.
    typed(
        &mut h,
        vec![
            egui::Event::Text("echo sleeping-$((2+2)); sleep 60; echo too-$((2+3))".to_string()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "sleeping-4");
    typed(&mut h, vec![egui::Event::Copy]);
    // Some shells (zsh) drop what was typed while a command is
    // interrupted: wait for the prompt to come back (within 30 s, half the
    // time `sleep 60` would take).
    let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !h
        .app
        .assistant_terminal_text()
        .unwrap()
        .trim_end()
        .ends_with("ready>")
    {
        assert!(
            std::time::Instant::now() < end,
            "no prompt after Ctrl+C:\n{}",
            h.app.assistant_terminal_text().unwrap()
        );
        h.frames(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    typed(
        &mut h,
        vec![
            egui::Event::Text("echo after-$((1+1))".to_string()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "after-2");
    // The prompt came back well within the minute `sleep` would have taken:
    // Ctrl+C interrupted it. (Whether the rest of the line then runs
    // depends on the shell: macOS's bash 3.2 runs it, dash does not.)

    // A paste arrives as typed text.
    typed(
        &mut h,
        vec![egui::Event::Paste("echo pasted-$((1+2))\n".to_string())],
    );
    wait_for(&mut h, "pasted-3");

    // Keys send what a terminal sends: `cat -v` shows them.
    typed(
        &mut h,
        vec![
            egui::Event::Text("echo cat-$((1+1)); cat -v".to_string()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "cat-2");
    typed(
        &mut h,
        vec![
            key(egui::Key::ArrowDown, egui::Modifiers::NONE),
            egui::Event::Text("x".to_string()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "^[[Bx");
    typed(&mut h, vec![key(egui::Key::D, egui::Modifiers::CTRL)]);

    // Claude Code's symbols: ⏵ ⏺ ✻ ⎿ and box drawing.
    typed(
        &mut h,
        vec![
            egui::Event::Text(
                "printf 'A\\342\\217\\265\\342\\217\\272\\342\\234\\273\\342\\216\\277\\342\\224\\200Z\\n'"
                    .to_string(),
            ),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "A\u{23f5}\u{23fa}\u{273b}\u{23bf}\u{2500}Z");

    // The mouse wheel scrolls back through the output.
    typed(
        &mut h,
        vec![
            egui::Event::Text("seq 1 200".to_string()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "\n200\n");
    let wheel = |lines: f32| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, lines),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    };
    h.renderer.move_pointer(egui::pos2(640.0, 650.0));
    // Ten notches, three lines each.
    typed(&mut h, vec![wheel(10.0)]);
    h.frames(5);
    let screen = h.app.assistant_terminal_text().unwrap();
    assert!(
        !screen.contains("\n200\n") && screen.contains("\n170\n"),
        "scrolled up 30 lines:\n{screen}"
    );
    // Typing returns to the bottom.
    typed(&mut h, vec![egui::Event::Text(" ".to_string())]);
    h.frames(5);
    assert!(h.app.assistant_terminal_text().unwrap().contains("\n200\n"));
    typed(
        &mut h,
        vec![key(egui::Key::Backspace, egui::Modifiers::NONE)],
    );

    // A program that asks for mouse reports (Claude Code's full-screen
    // view) gets the wheel as SGR mouse buttons 64 (up) and 65 (down).
    typed(
        &mut h,
        vec![
            egui::Event::Text(
                "clear; printf '\\033[?1000h\\033[?1006h'; echo mouse-$((2*3)); cat -v".to_string(),
            ),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    // Mouse reports are on once the marker after them is shown.
    wait_for(&mut h, "mouse-6");
    h.renderer.move_pointer(egui::pos2(640.0, 650.0));
    typed(&mut h, vec![wheel(1.0)]);
    typed(&mut h, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
    wait_for(&mut h, "^[[<64;");
    let screen = h.app.assistant_terminal_text().unwrap();
    // One notch, three lines (the line appears twice: as typed, and as
    // `cat` prints it).
    let line = screen.lines().find(|l| l.contains("^[[<64;")).unwrap();
    assert_eq!(line.matches("^[[<64;").count(), 3, "{screen}");
    typed(&mut h, vec![key(egui::Key::D, egui::Modifiers::CTRL)]);
    typed(
        &mut h,
        vec![
            egui::Event::Text("printf '\\033[?1000l\\033[?1006l'; seq 1 200".to_string()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    wait_for(&mut h, "\n200\n");
    h.renderer.move_pointer(egui::pos2(640.0, 650.0));
    typed(&mut h, vec![wheel(10.0)]);
    h.frames(3);

    if let Ok(directory) = std::env::var("STAMP_SCREENSHOTS") {
        let app = &mut h.app;
        let image = h.renderer.render(|ui| app.draw(ui));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            PathBuf::from(directory).join("terminal.png"),
            image.to_png(),
        )
        .unwrap();
    }
}

/// Not a test: runs the real Claude Code in the panel against the
/// Enterotypes example, asks it `STAMP_CLAUDE_PROMPT`, and keeps
/// screenshots in `STAMP_SCREENSHOTS`. Needs Claude Code installed and
/// signed in; uses the signed-in account. Run with `--ignored --nocapture`.
#[cfg(unix)]
#[test]
#[ignore]
fn claude_code_in_the_panel() {
    use eframe::egui;

    let folder = PathBuf::from(std::env::var("STAMP_ASSISTANT_FOLDER").expect("a folder"));
    let prompt = std::env::var("STAMP_CLAUDE_PROMPT")
        .unwrap_or_else(|_| "What does the plot in STAMP show? One sentence.".to_string());
    let shots = std::env::var("STAMP_SCREENSHOTS").ok().map(PathBuf::from);
    // STAMP_CLAUDE_DATA=coal uses the coal seam gas data instead.
    let mut h = match std::env::var("STAMP_CLAUDE_DATA").as_deref() {
        Ok("coal") => Harness::coal(),
        _ => Harness::enterotypes(),
    };
    let ctx = h.renderer.ctx.clone();
    let server = stamp_gui::assistant::window::start_sharing_with(&mut h.app, &ctx, None).unwrap();
    let claude = stamp_gui::assistant::launch::find_claude().expect("Claude Code is installed");
    stamp_gui::assistant::launch::prepare(&folder, &server.url(), server.token(), &claude).unwrap();
    let (mut program, mut args) = stamp_gui::assistant::launch::claude_command(&claude);
    // With STAMP_CLAUDE_LOG set, `script` records what passes each way.
    if let Ok(log) = std::env::var("STAMP_CLAUDE_LOG") {
        let command = format!("{program} {}", args.join(" "));
        args = ["-q", "-f", "-c", &command, &log]
            .iter()
            .map(|a| a.to_string())
            .collect();
        program = "script".to_string();
    }
    stamp_gui::assistant::window::run_in_panel(
        &mut h.app,
        &ctx,
        "Claude Code",
        &program,
        &args,
        &folder,
    )
    .unwrap();
    let shoot = |h: &mut Harness, name: &str| {
        if let Some(directory) = &shots {
            let app = &mut h.app;
            let image = h.renderer.render(|ui| app.draw(ui));
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(directory.join(format!("{name}.png")), image.to_png()).unwrap();
        }
    };
    let run_for = |h: &mut Harness, seconds: u64, until: &dyn Fn(&str) -> bool| {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
        while std::time::Instant::now() < end {
            h.frames(1);
            if until(&h.app.assistant_terminal_text().unwrap_or_default()) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    };
    let enter = egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    // The panel is drawn before it can be clicked.
    h.frames(3);
    let app = &mut h.app;
    h.renderer
        .click(egui::pos2(640.0, 700.0), |ui| app.draw(ui));
    assert!(h.app.assistant_terminal_has_focus());
    // Claude Code's first question in a new folder is whether to trust it;
    // the user answers it (Down, then Enter, for "Yes").
    let ready = |text: &str| text.contains("trust this folder") || text.contains("for shortcuts");
    run_for(&mut h, 30, &ready);
    shoot(&mut h, "claude-start");
    if h.app
        .assistant_terminal_text()
        .unwrap_or_default()
        .contains("trust this folder")
    {
        let down = egui::Event::Key {
            key: egui::Key::ArrowDown,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        // As a person would, read the question before answering it:
        // Claude Code may not take keys the moment it first draws.
        run_for(&mut h, 2, &|_| false);
        let app = &mut h.app;
        h.renderer.send_events(vec![down], |ui| app.draw(ui));
        run_for(&mut h, 1, &|_| false);
        let app = &mut h.app;
        h.renderer
            .send_events(vec![enter.clone()], |ui| app.draw(ui));
        run_for(&mut h, 30, &|text| text.contains("for shortcuts"));
    }
    run_for(&mut h, 5, &|_| false);
    shoot(&mut h, "claude-ready");
    let app = &mut h.app;
    h.renderer
        .send_events(vec![egui::Event::Text(prompt)], |ui| app.draw(ui));
    let app = &mut h.app;
    h.renderer.send_events(vec![enter], |ui| app.draw(ui));
    // The answer is in once Claude Code has used a STAMP tool and is idle
    // again.
    let wait: u64 = std::env::var("STAMP_CLAUDE_WAIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);
    run_for(&mut h, wait, &|_| false);
    shoot(&mut h, "claude-answer");
    let before = h.app.assistant_terminal_text().unwrap_or_default();
    println!("{before}");
    // The mouse wheel over the panel scrolls what Claude Code shows.
    h.renderer.move_pointer(egui::pos2(640.0, 600.0));
    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 5.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    };
    let app = &mut h.app;
    h.renderer.send_events(vec![wheel], |ui| app.draw(ui));
    run_for(&mut h, 2, &|_| false);
    shoot(&mut h, "claude-scrolled");
    let after = h.app.assistant_terminal_text().unwrap_or_default();
    println!("wheel changed the screen: {}", before != after);
    for entry in h.app.assistant_log() {
        println!("tool: {} {}", entry.tool, entry.args);
    }
}
