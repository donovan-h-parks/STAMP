# Plan: a STAMP assistant built on Claude Code

Status: planned, not started (2026-11-15).

## Goal

Claude subscribers can ask Claude Code about their data and plots, and have
it change or create plots in the STAMP window. Users of the browser version,
or of ChatGPT or Gemini, get a "Copy for AI" option instead. STAMP never
stores a login or an API key.

```
STAMP window ── local MCP server (127.0.0.1, token) ◄── Claude Code (user's own Claude login)
     ▲                                                        │
     └──────── plot or analysis changes shown live ◄──────────┘
```

All new code goes under `crates/stamp-gui/src/assistant/`. The parts that
can't run in a browser are compiled only for the desktop
(`cfg(not(target_arch = "wasm32"))`), so the browser build stays clean.

## Why this design

- STAMP's users are mostly not developers. They have monthly or yearly
  subscriptions to Claude, ChatGPT or Gemini rather than API keys.
- A chat panel inside STAMP that calls an LLM itself can only use API keys.
  Anthropic's terms (clarified in February 2026) allow Free, Pro and Max
  logins only in Anthropic's own products: Claude.ai, Claude Desktop and
  Claude Code. OpenAI and Google likewise don't offer a way for another
  app's chat to run on their subscriptions.
- Claude Code is Anthropic's own product, so using it with a subscription is
  allowed. A terminal inside STAMP just hosts it, like the terminal inside
  VS Code.
- ChatGPT (Developer Mode) and the Gemini web app can add custom MCP
  connectors, but only at a public HTTPS address. A STAMP running on
  someone's own computer can't be reached that way without a relay
  service hosted by the STAMP project. That's left out unless there is
  demand.
- No Linux virtual machine is needed: Claude Code runs directly on Linux,
  macOS and Windows. In the browser, Linux as WebAssembly (WebVM, v86,
  container2wasm) would need a hosted network relay anyway, is a download
  of hundreds of megabytes, and would be slow and fragile.

## Phase A: describing the current view (desktop and browser), about 1–2 days

- **`assistant/describe.rs`**, with three functions:
  - `session_summary`: Markdown listing the loaded files, analysis mode,
    level, test, correction, filters and groups;
  - `results_csv(mode, top_n)`: the results table, using STAMP v2's column
    headings;
  - `plot_description(figure, kind, settings)`: the plot's kind, its options
    as JSON, its titles and axis labels (from the `Figure`'s text), and the
    item and tooltip of each region. Because this comes straight from the
    `Figure`, it works for every plot without per-plot code.
- **"Ask AI" menu:**
  - "Copy description for AI";
  - "Copy plot image", or saving a PNG if egui can't put an image on the
    clipboard on every system.
- **Tests:** compare the description of a fixed example session with a
  stored copy.

## Phase B: an MCP server that only reads, desktop only, about 3–4 days

- **`assistant/mcp.rs`:**
  - MCP's HTTP transport on `127.0.0.1` at a random port, with a fresh
    access token every run and a check on where requests come from (the
    `Origin` header);
  - JSON-RPC written by hand with `serde_json` over a small HTTP server, so
    there's no tokio;
  - replies are plain JSON (MCP's HTTP transport allows this instead of
    streaming).
- **`assistant/tools.rs`:** carries out a tool call on a `StampApp` and
  doesn't care how the request arrived, so the same code could serve a
  future relay or chat.
- **`assistant/bridge.rs`:** the server thread puts each request on a
  channel and wakes the window (`ctx.request_repaint()`). `StampApp::update`
  answers it and sends the reply back.
  - Risk: a minimised window may not be updated promptly. If so, keep a
    shared copy of the current state that read requests can be answered
    from without waiting for the window.
- **Read tools:**
  - `get_session`
  - `get_results(mode, top_n, sort)`
  - `get_feature(name)`
  - `get_current_plot(format: description|png|svg)`
  - `list_plot_kinds`
  - `get_plot_options(kind)`
- **Privacy:**
  - a Preferences switch, "Allow Claude Code to connect to this session",
    off by default;
  - a status indicator showing when Claude Code is connected;
  - an activity log listing every tool call and what it sent.
- **Tests:** call each tool through `StampApp`, plus one test that sends a
  real HTTP request.

## Phase C: tools that change things, about 3–5 days

- **Tools:**
  - `show_plot(kind, options_patch)`
  - `select_feature`
  - `set_analysis(mode, test, correction, filters, groups)`
  - `set_active_samples`
  - `export_table`, which writes a CSV into Claude's working folder so it
    can do its own analysis on it
- **Running analyses:** a tool that changes an analysis waits until it has
  finished, with a time limit, before replying.
- **Checking options:** a patch is merged into `PlotSettings`, and if it
  isn't valid, the error message goes back to Claude.
- **Describing options:** derive `schemars::JsonSchema` on `PlotSettings`
  and the options structs in `stamp-plot`. Their doc comments become the
  descriptions Claude reads, and `schemars` builds for the browser.
- **New kinds of plot:** `show_image(svg|png, title)` shows a figure Claude
  made itself in a "Custom plot" view, drawn with resvg (already a
  dependency). A small chart format drawn in STAMP's own style can come
  later if needed.
- **Undo:** a banner, "Changed by Claude — Undo", restores the previous
  settings.

## Phase D1: an "Open Claude Code" button using the computer's own terminal, about 2 days

- **`assistant/launch.rs`:**
  - finds `claude` on the PATH or where its installer puts it
    (`~/.local/bin`, `%USERPROFILE%\.local\bin`);
  - if it isn't there, offers to install it: an explanation, then
    Anthropic's official installer run in a terminal. Claude Code is not
    included in STAMP's own installer: its licence is proprietary, and
    downloading it on demand also keeps it up to date.
- **Prepares a working folder** in STAMP's data folder, rewritten at each
  launch:
  - `.mcp.json`: the address of STAMP's server and the token for this run;
  - `CLAUDE.md`: STAMP's tools, how to explain statistics to a biologist,
    and a reminder to use STAMP's built-in plots when one fits;
  - `.claude/settings.json`: pre-approves STAMP's own tools
    (`mcp__stamp__*`), so users aren't asked at each step; anything else
    still asks first.
- **Opens a terminal running `claude` in that folder:**
  - macOS: a `.command` script opened with `open`;
  - Windows: `wt`, or `cmd /c start` if `wt` isn't there;
  - Linux: `x-terminal-emulator`, `gnome-terminal`, `konsole` and `xterm`,
    tried in that order.
- **Milestone:** a Claude subscriber can use the whole feature. It's worth
  testing with real users at this point, before building Phase D2.

## Phase D2: a terminal panel inside STAMP, about 1–2 weeks

- A dockable "Claude" side panel: `alacritty_terminal` for the terminal
  itself, with our own egui widget based on `egui_term`. That crate
  (0.1.0, last released April 2025) needs porting from egui 0.31 to 0.37.
- `alacritty_terminal` runs the program in a pseudo-terminal on all three
  systems (ConPTY on Windows). D1's launch and working folder are reused.
- Claude Code is shown as an ordinary terminal the user types into. STAMP
  doesn't script it from behind its own chat window (for example with
  `claude -p`), which would come close to the ban on other products using
  subscription logins.

## Phase E: documentation and release, about 1–2 days

- `docs/site/assistant.rst`: setup, example questions, and what data is
  sent to Anthropic.
- Update `analyses.rst` and the screenshots.
- An entry in `NEW_FEATURES.md`.
- Both clippy targets and the tests pass on Linux, Windows and macOS.

## Order

A → B → C → D1. Release after D1, then build D2 if users want it. Phase A
is useful on its own, including for the browser version and for ChatGPT
and Gemini users.

## To settle before release

- Confirm with Anthropic that starting Claude Code from STAMP, preparing
  its folder and pre-approving STAMP's tools is acceptable.
- Check Claude Code's current requirements on Windows (whether Git for
  Windows is needed).
- Keep to the documented `.mcp.json` and `settings.json` formats, since
  Claude Code's configuration has changed before.

## Known limits

- Claude Code needs Pro, Max, Team or Enterprise; it isn't on the Free
  plan.
- The browser version gets Phase A only.
