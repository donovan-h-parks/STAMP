# Current task

What is being worked on right now, so that work can be picked up again if a
session ends unexpectedly. Update this file when a task starts, at each
meaningful step, and when it finishes.

## Status

Finished and committed (2026-11-24): **analysis wizard** and the six
plots and tests it needed. Asked for: a wizard from research question to
test and plots, for non-statisticians, with pictures and "i" buttons on
the choices, every choice always visible and only those the loaded data
supports selectable. `docs/WIZARD_PLAN.md` has how such guides are
structured, the flow chart (Mermaid), the selectable-when rules and what
STAMP still lacks against such guides (assumption checks, linear models
and smaller additions). Order decided by the owner: comparison first,
research question second; dead ends of the chart filled first: (1) p-value
histogram in the two-group and multiple-group analyses; (2) pairwise
PERMANOVA (`community::PairwiseRun`, `Layout::pairwise`, PCoA notes, CLI
prints all pairs); (3) `PlotKind::AlphaScatter` (correlation;
`community_plots::alpha_correlation`; CLI `correlation --plot-type
alpha-diversity`); (4) co-occurrence `View::Pair` (`pair_plot`, setting
`co_occurrence_pair`, CLI `--with A --against B`); (5)
`MultiGroupTest::Friedman` with `PostHocTest::WilcoxonHolm`,
`Metadata::matched_groups`, `MultiState::pair_field` ("Subject field"),
CLI `--test friedman --pair-field`; (6) `stats/presence.rs`:
`GroupTest::FisherPresence`, `GroupTest::McNemar`,
`MultiGroupTest::ChiSquarePresence`. Reference values from SciPy 1.18.1
(`pixi exec --spec python=3.12 --spec scipy`), named in each test; they
are not in tests/reference/reference_values.R. No default changed
(DEFAULTS_VERSION as it was). The wizard: `wizard.rs` (questions, `Facts`
about the loaded data, availability with reasons, `Recommendation` and
`Change`; unit tests), `wizard_ui.rs` (window, cards with painted
drawings, "i" explanations, trail of answers, result page),
`StampApp::{show_wizard, wizard_facts, apply_wizard}`, Help > Analysis
wizard and a button in the Statistics panel. GUI tests
`plots_and_tests_the_wizard_leads_to`, `the_wizard_leads_to_an_analysis`
and `documentation_screenshots_of_the_wizard` (writes gui_wizard_*.png,
copied by hand to docs/site/images). Docs: new wizard.rst, plots.rst,
statistics.rst, analyses.rst, command_line.rst, choosing_a_test.rst,
NEW_FEATURES.md. **Not done**: the "At a glance" table view of the plan;
an icon of its own (it uses the sliders icon); figures of the six new
plots in the documentation; trying it by hand with a mouse (clicks on
cards and "i" buttons are not covered by a test, which drives
`Wizard::choose`); the browser build was only linted, not run.

Finished and committed (2026-11-19, assistant session): **several named
custom plots**. Each new plot from `show_custom_plot` is kept and listed by
name, without a prefix, under "Drawn by the assistant" (`Session::
custom_plots` + `shown_custom` replace `custom_plot`; `CustomPlot::name`;
`StampApp::plot_name`, `choose_custom_plot`). `name` and `spec` are required
(schema and code); a name used by another custom plot or by one of STAMP's
plots is refused; `plot` names one to change in place; `show_plot` shows an
earlier one by name; undo restores the list. Tests in tests/assistant.rs;
docs assistant.rst, plots.rst, NEW_FEATURES.md, GUIDE. fmt, clippy (both
targets), stamp-gui tests and Sphinx pass. Committed ("Keep every plot the
assistant draws, by name").

Earlier: Finished and committed (2026-11-19, assistant session): **assistant tool
`load_data`**, which opens a profile and/or metadata file by full path ("~/"
for the home folder): one file in any format File > Load profile reads,
several SingleM or Sandpiper files combined, a BIOM table with
`biom_metadata_field`, a mothur taxonomy file with `mothur_groups` and
`mothur_names`. Every file is read before anything changes; errors end
"Nothing was changed". app.rs gained `choose_mothur_files`,
`cancel_pending_profile` and `use_metadata` (loading metadata now clears
the assistant's undo, as loading a profile did). GUIDE, INSTRUCTIONS,
assistant.rst, NEW_FEATURES.md; tests in tests/assistant.rs. fmt, clippy
(both targets), stamp-gui tests and Sphinx pass. Committed ("Let the
assistant load a profile and metadata"). Follow-up once directory loading
is committed: let `load_data` take a directory.

Earlier: Finished and committed (2026-11-18, assistant session): **custom plots
drawn by the assistant** (tool `show_custom_plot`). The plot is described in
a subset of Vega-Lite and drawn as a STAMP `Figure` by the new module
`crates/stamp-plot/src/custom/` (mod.rs: marks, scales, axes, legends;
data.rs: tables and transforms; expr.rs: expressions; tests.rs, with an
ignored `gallery` test that writes PNGs to `STAMP_CUSTOM_PLOTS`). In the
GUI: `PlotKind::Custom`, `plots::CustomPlot` (spec + STAMP's tables
"profile", "samples", "results" taken when drawn), `Session::custom_plot`,
undo, the plot menu ("Drawn by the assistant"), `describe` repeats the
spec, GUIDE and INSTRUCTIONS mention the tool. Docs: assistant.rst
(Custom plots, image gui_custom_plot.png), plots.rst, NEW_FEATURES.md.
fmt, clippy (both targets), the whole test suite and the Sphinx build
pass. Committed and pushed ("Let the assistant draw custom plots").

Done and committed (2026-11-18): **Antigravity not using STAMP's tools**.
Report from Windows (agy 1.5.10, Google AI Pro, Gemini): asked about the
data, the model read the folder's files, found `~/.gemini/config/
mcp_config.json` empty and began writing its own `mcp_client.py`. Here
(Linux, agy 1.5.9, isolated HOME, fake key) `/mcp` shows `stamp` connected
with its tools, so the likely cause is that Antigravity leaves MCP tools to
be discovered on demand (through `call_mcp_tool`) and the model did not.
Changes: every tool in `tools/list` carries `_meta:
{"antigravity/alwaysLoad": true}` (`assistant/server.rs`, asserted in its
test), and the guide (`launch::GUIDE`, CLAUDE.md/AGENTS.md of the working
folder) tells the assistant to reach STAMP only through the tools and to
say so if it cannot see them. `/mcp` still lists the tools with 1.5.9;
fmt, clippy (Linux, wasm) and stamp-gui tests pass. **Not verified**: a
real Gemini model using the tools (needs a Google sign-in), and Windows,
where the cause could instead be the workspace `.agents/mcp_config.json`
not being loaded or the connection failing; if it still fails, `/mcp` in
Antigravity on Windows shows which. assistant.rst has a note on `/mcp`.

Finished, committed and pushed (2026-11-18, the layout session; "Rework
the window layout, add assistant tabs and table arrow keys"): **layout,
Statistics, assistant tabs**. In the working tree: arrow keys in the
Feature and Results tables (`table_keys`/`table_focus` in app.rs); bar plot
p-value under the title (feature_plots.rs); Properties renamed Statistics
(`Frame::Statistics`, serde alias "Properties"); new default layout
(layout.rs, LAYOUT_VERSION 2); Preferences > Restore window position
defaults (`restore_window_positions`); View menu brings a frame behind a tab
to the front (`toggle_frame`); the comparison corrections moved out of the
Statistics panel into `keep_choices_valid`, run each frame; Assistant menu
order OpenCode, Claude Code, Antigravity; several assistants as tabs in the
panel (`assistant::Tab`, `show_agent`). The left frame is made exactly as wide
as the four analysis buttons need in one row when the default layout is
first drawn (`analysis_buttons_width`, `layout::set_frame_width`,
LAYOUT_VERSION 3). Docs (main_window, assistant,
quick_start, analyses), images gui_main_window, gui_frames (test
`documentation_screenshots_of_the_window`) and tg_bar (stamp-cli, 110 dpi),
CHANGES_FROM_PYTHON 63 and 67, NEW_FEATURES. fmt, clippy (both targets), all
tests and the Sphinx build pass. Another session has uncommitted edits in
assistant/server.rs (alwaysLoad), the guide in assistant/launch.rs, the
Antigravity /mcp paragraph of assistant.rst and NEW_FEATURES.md (a moved
entry): not mine, leave them out of my commit.

Finished and committed (2026-11-17, assistant session): **Assistant > Open
Google Antigravity**, Google's Antigravity CLI (`agy`) in the assistant
panel. `launch::Agent::Antigravity`: finds `agy` on the PATH, in
`~/.local/bin` or `%LOCALAPPDATA%\agy\bin`; writes
`.agents/mcp_config.json` (url, Bearer header, timeoutSeconds 900),
`stamp-antigravity-settings.json` (allow `mcp(stamp/*)`, passed with
`--settings`, never saved) and `AGENTS.md`; installs with
antigravity.google/cli/install.sh or .ps1. Tested with agy 1.5.9 in the
panel (isolated HOME, fake GEMINI_API_KEY): it reads the folder's MCP
config, connects to STAMP over http://127.0.0.1 and lists all 13 tools
(it connects lazily, on first need); the log shows the allow rule
applied. Not tested: a real Google sign-in and a model answering, Windows,
macOS. Test `antigravity_in_the_panel` (ignored) shares a helper with
`opencode_in_the_panel`. Docs: assistant.rst, main_window.rst,
NEW_FEATURES.md. My files: crates/stamp-gui/src/{app.rs,layout.rs,
assistant/{launch,window,mod}.rs}, tests/assistant.rs, the two .rst files,
and the Antigravity sentence in NEW_FEATURES.md. Committed and pushed
("Open Google's Antigravity CLI in the assistant panel").

Finished and committed (2026-11-17, the glacier-fed stream session):
**paired samples joined in the box plot**. `BoxOptions::show_pairs` and
`FeatureData::paired` (`stamp-plot/src/feature_plots.rs`), drawn by
`draw_pairs` in `multi_box.rs` (panels layout); CLI `--box-pairs`
(two-groups with a paired test, an error otherwise); GUI checkbox "Join
paired samples with lines (paired tests)" in the box plot's Configure plot;
tests in `multi_box.rs`, `stamp-cli/tests/cli.rs` and
`stamp-gui/tests/ui.rs` (screenshot `box_plot_pairs`); NEW_FEATURES.md,
plots.rst, command_line.rst. fmt, clippy (both targets) and the whole test
suite pass. No figure of it in the documentation yet. Other uncommitted
changes in the checkout (assistant, app.rs, layout.rs, main_window.rst)
belong to another session.

Idle. Last task (2026-11-16): **Claude Desktop removed, Assistant menu
icons**. Deleted `assistant/bridge.rs`, `assistant/bundle.rs`,
`src/bridge_main.rs` and the `stamp-mcp` binary; the server no longer
writes `connection.json` (`Server::publish` gone, `start_sharing_with`
folded into `start_sharing`); package.sh, RELEASING.md, assistant.rst,
NEW_FEATURES.md updated, ASSISTANT_PLAN.md notes the removal. Assistant
menu items have icons (clipboard-text, image, terminal-window, plugs,
pulse, arrow-counter-clockwise; Phosphor subset regenerated). assistant.rst
explains Stop sharing; the panel's Stop button is removed (quit the
program with /exit); the HTTP test checks that Stop sharing closes the
port. Committed and pushed ("Remove Claude Desktop sharing; add icons to
the Assistant menu").

Previous task (2026-11-16): **Storey FDR reliability**: pi_0 is used only
when its standard error is at most a quarter of it, some p-value reaches
0.95 and some lies above lambda; otherwise pi_0 = 1 (BH q-values) with a
warning (Properties frame under the correction, CLI, assistant). Correction
info adds pi_0 used, lambda, p-values above lambda, standard error.
Prompted by the coal seam gas data (41 phyla, pi_0 = 0.12 from 2
p-values). Committed and pushed ("Fall back to Benjamini-Hochberg when
Storey's pi_0 is too uncertain").

Previous task (2026-11-16): **code review fixes**, on `rust-port`
(commits "Fix crashes and wrong results found in the code review" and "Fix
plot labels, speed up tests and the window, shrink the web build"). Crashes
(10 shapes, Scheffé with single-sample groups), stale GUI results (per-mode
errors, cancel, correction during a run, empty groups), Storey pi_0 = 0,
Fisher fractional counts, ANOVA zero within-variance, metadata header with
a leading tab, empty grouping values, composition with a parent level,
histogram p = 1; plot labels (post-hoc, volcano log2 ratios, bar tooltips,
log scale, mirrored ratios); exact Mann-Whitney and White's memory; profile
loading race; per-frame caches; direct PDF writer (`stamp-plot/src/pdf.rs`)
and canvas PNGs in the browser (no resvg/svg2pdf in wasm); icon font subset
(`scripts/icons/subset.sh`). Not tested in a real browser: PNG saving and
Copy plot image. Remaining review items are listed in the session that did
the review (BIOM depth, ordination tooltip order, file errors, conversion
speed, DejaVu subset, ...).

Previous: Idle. Last task (2026-11-16): **OpenCode in the assistant panel**, on
`rust-port`, committed and pushed; Windows package built. `launch::Agent {ClaudeCode, OpenCode}` (name,
accounts, find, command, prepare, prepare_install, installer);
`find_opencode`, `opencode_major` (`--version`, cached per path+mtime),
`opencode_config` (v2: `mcp.servers.stamp` with `oauth:false`,
`codemode:false`, `timeout.execution`; `permissions` ask for shell, edit,
webfetch, allow `stamp_*`; v1: `mcp.stamp`, `permission`), `--standalone`
for v2; install: `opencode.ai/v2/install` or the GitHub zip on Windows,
into ~/.opencode/bin. window.rs: `open_agent_or_ask`, `open_agent`,
`open_agent_outside`, `Ask::Agent`, menu "Open OpenCode...", "Stop
sharing"; status bar "Sharing with assistant"/"Assistant connected";
`State.agent`, `Saved.assistant_program`, tab and View menu named after the
program (`layout::Behavior.terminal_title`); `assistant_connections()`.
Tests: launch unit tests; ignored `opencode_in_the_panel`. Both config forms
validate against opencode.ai/config.json. fmt, clippy (Linux, wasm), all
tests, sphinx -W pass. Docs: assistant.rst (OpenCode section, "assistant
panel"), main_window.rst, NEW_FEATURES. Checked with the real OpenCode
(user's OK; XDG dirs in the scratchpad): v2.3.0 and v1.19.5 both connect,
answer with get_session/get_current_plot using the free default model
("Big Pickle"), and v2 shows "Permission required" before a shell command;
`--standalone` leaves no process behind. `run_in_panel` now takes the
`Agent` (tab named after it). Not checked: Windows (the zip install,
ConPTY), macOS, a ChatGPT or Copilot sign-in.

Previous: Idle. Last task (2026-11-16): two more assistant tools, on `rust-port`,
committed and pushed: `switch_samples` (off or on by names, by a metadata
`where` {field, relation, value; "=" values matched as the metadata spells
them}, or `all`) and `set_plot_fonts` (tick, axis label, legend sizes and
font). Undo restores switched-off samples (`Session::switched_off_samples`,
`set_switched_off_samples`, `Metadata::is_switched_on` in stamp-core).
Tests `samples_are_switched_off_by_metadata_and_undone` (coal data),
`plot_fonts_are_changed_and_undone`; the ignored panel test takes
`STAMP_CLAUDE_DATA=coal`. Checked with the real Claude Code: "Remove
samples from the QS Company. Then make the tick labels 12 point and the
axis labels 14 point" worked. Tests, fmt, clippy (Linux, wasm), sphinx -W
pass. Untracked `examples/GlacierFedStreams/` is another session's.

Previous: Idle. Last task: **movable frames** (2026-11-15), on `rust-port`, committed
and pushed. Plan `docs/LAYOUT_PLAN.md`. Done: `crates/stamp-gui/src/
layout.rs` (egui_tiles 0.18: `Frame` enum, default tree with v2's widths as
shares around 1.0, `sync_visibility` (a hidden frame's room goes to the
plot and comes back), `Behavior`: plot fixed without tab bar, × hides,
no keyboard tab cycling); `app.rs` draws the frames in the central panel,
`frame_shown`/`show_frame`, `layout`/`layout_mut`/`reset_layout`, Results
table and Configure plot are frame functions (`results_table_ui`,
`plot_config_ui`), `Saved.layout` + `layout_version` (LAYOUT_VERSION 1;
incomplete or other-version layouts fall back to the default), View >
Reset layout; Claude Code is `assistant::window::terminal_frame`. Tests:
layout unit tests; GUI tests `frames_can_be_moved_and_the_layout_is_
remembered`, `frames_are_hidden_and_shown_and_open_where_they_belong`; the
q-value label test finds its pixels relative to the Properties frame. All
tests, fmt, clippy (Linux, wasm), sphinx -W, trunk build (wasm 16.2 MB,
6.4 MB gzipped) pass. Docs: main_window.rst "Arranging the window",
images gui_main_window/gui_metadata_table regenerated, new gui_frames.png;
NEW_FEATURES; CHANGES_FROM_PYTHON behaviour change 63. Dragging with the
mouse itself is not covered by a test (moves are made through egui_tiles'
API) and has not been tried by hand.

Previous: Also (2026-11-15, another session in this checkout): **p-value labels of
plots**, committed and pushed (`7df19f7`, `cb59fac`): "Uncorrected
p-value", "Corrected p-value" (Bonferroni, Holm, Sidak), "q-value"
(Benjamini-Hochberg, Storey), via `stamp-plot/src/p_values.rs`; change 61.

Idle. Last task (2026-11-15, after testing on Windows), on `rust-port`,
committed and pushed: the Claude Code panel's mouse wheel did nothing because
Claude Code runs a full-screen view that asks for mouse reports, which the
panel ignored. Now `terminal::wheel_input` sends wheel reports (SGR or
X10) to programs in mouse mode, arrows on the alternate screen, and
scrolls the scrollback otherwise (3 lines a notch, from raw MouseWheel
events); a scroll bar beside the terminal (drag/click). Dialog "Don't
show this again" (`Saved::skip_claude_code_dialog`, Preferences turns it
back on; `window::open_claude_code_or_ask`). New tool `set_group_colors`
(names or #rrggbb; colours listed in get_session; undoable via
`Session::chosen_group_colors`). Checked with the real Claude Code here:
the wheel scrolled its view, and "make Enterotype 2 red" worked. Tests,
fmt, clippy (Linux, wasm, Windows) pass.

Previous: Idle. Last task: **Claude Code panel inside STAMP** (phase D2,
2026-11-15), committed on `rust-port` and pushed. Done:
`assistant/terminal.rs` (alacritty_terminal 0.26, desktop-only dependency;
egui drawing; keys, paste, Ctrl+C/copy, scrollback, selection; look-alikes
for ⏺⏵⎿⏸⏹), bundled `assets/DejaVuSansMono.ttf` (licence note in
`crates/stamp-plot/fonts/LICENSE-DejaVu.txt`), `icons::TERMINAL`, bottom
panel `window::terminal_panel` with Hide/Stop/Restart/Separate terminal,
View > Claude Code, shortcuts skipped while it has focus,
`launch::claude_command`/`script_command`, `headless::send_events`. Tests:
`the_claude_code_panel_runs_a_terminal`; ignored `claude_code_in_the_panel`
ran the real Claude Code 2.1.325 here (answered via 4 STAMP tools,
switched to the volcano plot). Whole suite, fmt, clippy (Linux, wasm,
Windows via cargo-xwin) and sphinx -W pass. Not tried on Windows/macOS.
Docs: assistant.rst (panel section), main_window.rst, NEW_FEATURES,
ASSISTANT_PLAN.

Previous: Idle. Last task: **Claude Desktop connection** (2026-11-15), branch
`rust-port-llm`. Done: bridge `stamp-mcp` (`src/bridge_main.rs`,
`assistant/bridge.rs`); connection file while sharing
(`Server::publish`, `window::start_sharing_with`); Assistant menu: Share
with Claude Desktop, Install in Claude Desktop (`assistant/bundle.rs`
builds `stamp.mcpb`), Open Claude Code, Stop sharing with Claude;
`scripts/package.sh` ships `stamp-mcp`; docs (`assistant.rst` rewritten,
RELEASING, NEW_FEATURES, ASSISTANT_PLAN). Tests: bridge subprocess test in
`tests/assistant.rs`; whole suite, fmt, clippy (both targets) and
`sphinx-build -W` pass. Checked with the real Claude Code starting
`stamp-mcp` as a stdio server. Not checked: Claude Desktop itself, macOS
Gatekeeper on the unsigned bridge, Claude Desktop's tool-call time limit.
Committed on `rust-port-llm`, in the commit after the Windows icon change
(`023f7a7`: build.rs, build-dependency, RELEASING "Windows executables from
Linux"). Merged into `rust-port` (fast-forward, 2026-11-15); not pushed.

Previous task: **LLM assistant built on Claude Code** (2026-11-15), branch
`rust-port-llm` (from `rust-port`). Plan in `docs/ASSISTANT_PLAN.md`
(phases A, B, C, D1 now; D2 later). Code in
`crates/stamp-gui/src/assistant/`. Progress:

- [x] A: `assistant/describe.rs`, Assistant menu (copy description/image)
- [x] B: `assistant/server.rs` (hand-written MCP over HTTP), `tools.rs`
- [x] C: show_plot, set_analysis, undo (snapshots in `app.rs`)
- [x] D1: `assistant/launch.rs`, `assistant/window.rs`
- [x] Unit tests, `tests/assistant.rs`, whole workspace suite pass; fmt,
      clippy (desktop and wasm) clean; `sphinx-build -W` passes
- [x] Checked against the real Claude Code 2.1.325 (ignored test
      `serve_for_claude_code` + `claude -p`): it connected, read the plot,
      switched to the volcano plot and explained both. Permissions had to be
      passed with `--settings stamp-settings.json`; a project
      `.claude/settings.json` was not applied in an untrusted folder.
- [x] Docs: `docs/site/assistant.rst` (in index), NEW_FEATURES entry, plan
      status updated.

Not done / not verified: the terminal launch on macOS and Windows (only the
Linux terminal choice is unit-tested; no display here to open one); the
interactive (non `-p`) first run of Claude Code with its folder-trust
prompt; Phase D2 (terminal inside STAMP). Known limit: while STAMP's window
is hidden, read tools are answered (`App::logic`) but changes wait for the
window to redraw. Committed on `rust-port-llm`, since merged into `rust-port`.

## Last completed

- **Windows icon when cross-compiling** (2026-11-15), on `rust-port-llm`:
  `winresource` is now a build-dependency on every host and `build.rs`
  embeds the icon and version details whenever the target is Windows
  (llvm-rc on Linux). Checked with `cargo xwin build` on Page:
  ICON/GROUP_ICON/VERSIONINFO present; `scripts/package.sh windows`
  works. `docs/RELEASING.md` has a "Windows executables from Linux"
  section. Not run on Windows. Not committed.
- **Eggerthellaceae built-in example dataset** (2026-11-15), on top of
  HEAD 5352966. `examples/Eggerthellaceae/` (eggerthellaceae.spf 1,289 x
  544 Domain..Genus, eggerthellaceae_mgr_species.spf, metadata, readme.txt
  with provenance/caveats/reuse). GUI: `Example::EggerthellaceaeGenera`
  and `EggerthellaceaeSpecies` ("Eggerthellaceae (genera)", "Eggerthellaceae
  (focus species)"), gzipped by `build.rs`, `unpack`, and
  `StampApp::eggerthellaceae_settings` (level Genus/Species, species with
  Unclassified = frequency-only, group field Host_group, two groups
  Domestic horse vs Feral horse (Sable Island), correlation field
  Equivita_relab, Multiple groups mode). Heatmap refusal now says samples or
  features and points to the group heatmap (`Figure::message` draws one
  line per `\n`); results table window lays out only visible cells (frame
  150 -> 100 ms with 3,903 columns; still slow, columns are not
  virtualised). Load 65 ms, analysis + PCA settled ~0.8 s, ~47 MB resident
  (peak 86 MB), results table +140 MB. Size: desktop +591 KB (40.18 ->
  40.77 MB), wasm 14.71 -> 15.30 MB, gzipped 5.56 -> 6.15 MB. Tests: GUI
  `eggerthellaceae_examples_load_with_their_own_settings`,
  `both_built_in_examples_load` names, core `eggerthellaceae_example`,
  golden_profiles list, heatmap limits. Docs: new
  `example_eggerthellaceae.rst` (4 figures egg_*.png), input_files,
  quick_start, faq (two entries), index, plots note, command_line examples
  now name the example files (all checked to run), NEW_FEATURES, README;
  sphinx -W passes (`pixi exec --spec sphinx --spec sphinx_rtd_theme`).
  **Not committed** (instructed to leave changes uncommitted).

- **Map of sample locations (Natural Earth)** (2026-11-15), on top of the
  other uncommitted work. Data: `scripts/naturalearth/convert.py` + README
  (Natural Earth v5.1.2 GeoJSON, SHA-256 pinned, stdlib Python, varint
  delta format), blobs `crates/stamp-plot/data/ne_{110m,50m}_{countries,
  lakes}.bin` (170 KB, 151 KB gzipped). stamp-plot: `geo.rs` (decoder,
  Equal Earth / Robinson / equirectangular / Mercator, Sutherland-Hodgman
  ring clipping with undrawn seam/clip edges, Liang-Barsky lines),
  `map_plot.rs` (auto/world/region/custom extents, auto resolution, site
  aggregation on a grid sized to the map, jitter, size by count/field/
  feature, legends, tooltips, notes), `Shape::Area` (earcut 0.4 triangles,
  new pure-Rust dependency; SVG path, egui mesh), legend/value scale/shape
  plan moved from pca_plot into `ordination_style.rs` (`LegendItem`,
  `ValueScale`, `ShapePlan`). CLI `--plot-type map` + `MapArgs` (two-groups,
  multi-groups, correlation; correlation also got `OrdinationArgs`). GUI
  `PlotKind::Map` in new plot group "Where the samples are from",
  `PlotSettings::{map, map_lat_field, map_lon_field}` (new fields only, so
  DEFAULTS_VERSION stays 2), dialog (`Dialog::map`, shared `Dialog::styles`),
  About credit. Tests: geo, map_plot, CLI `samples_are_drawn_on_a_map`, GUI
  `samples_are_placed_on_a_map`, `map_settings_dialog`,
  `a_plot_part_way_through_its_community_tests` (regenerates
  docs/site/images/gui_plot_progress.png: PERMANOVA 408/999, 40%, on
  LargeDataset). Docs: plots.rst section + mg_map.png (from new
  examples/EnterotypesArumugam/Enterotypes.metadata.locations.tsv,
  illustrative cohort cities), command_line.rst, citing.rst, main_window.rst,
  NEW_FEATURES, README. Web wasm 14.71 MB (5.56 MB gzipped). Not done: a
  single "lat_lon" field, central meridians other than 0, country
  tooltips. **Not committed** (instructed to leave changes uncommitted).

- **Progress bars, spinners and cancelling for slow work** (2026-11-15), on
  top of the other uncommitted work. stamp-core: resumable `DistanceRun`,
  `NmdsRun`, `PermutationRun` (`CommunityTest::start`), `MantelRun` (ranks
  Spearman distances once: CLI Mantel at 927 samples 94 s -> 6.3 s, also run
  once instead of twice), `AncomRun`; `table::write_{group,multi_group,
  correlation}_table` (row by row, byte-identical; CLI `-o` at 1,307
  samples 4.7 s / 1.67 GB -> 2.9 s / 204 MB). Tests `tests/resumable.rs`,
  Mantel naive-vs-fast, tables streamed = to_tsv. stamp-plot:
  `community_plots::{CommunityRun, CommunityJob, CommunityOutput,
  StagePace, draw_mantel, draw_distance_box}`. GUI: `jobs::Progress` stage
  labels, `jobs::now_ms`; `plots::OrdinationCache` requests
  (`take_request`, `PlotRequest`, `RunningPlot`, `set_synchronous`) for
  PCA, PCoA+tests, NMDS, distance box, Mantel, ANCOM; app.rs plot overlay
  (greyed previous plot, label + bar, Cancel / Compute again, 0.3 s delay),
  status bar, `StampApp::{plot_progress, is_busy, cancel_plot,
  compute_plot, loading, cancel_loading, results_table_progress}`; profile
  loading from file events in a job (`Loading`, `profile_text`); large
  results tables built in 30 ms slices while the window is open. GUI test
  `a_slow_plot_shows_its_progress_and_can_be_cancelled`, plots test
  `slow_plots_are_computed_in_a_job`. CLI: `progress.rs` (stderr, TTY only,
  0.5 s delay, `STAMP_FORCE_PROGRESS` for tests), global `--quiet`; test
  `progress_is_shown_only_on_a_terminal`. Docs: main_window.rst (Progress
  and cancelling, figure gui_plot_progress.png), command_line.rst
  (Progress), faq.rst, NEW_FEATURES. Not done: eigen decompositions are one
  indivisible piece (browser freezes for them with the label shown); File >
  Convert in the browser converts without an indicator. **Not committed**
  (instructed to leave changes uncommitted).

- **Ordination performance** (2026-11-15), on top of the other uncommitted
  work. 619 samples x ~6,000 genera (CLI, SVG): PCoA+tests 35.1 -> 3.6 s,
  PCA 15.6 -> 2.5 s, NMDS 17.0 -> 4.5 s; 1,307 samples: 651 -> 10.4 s,
  409 -> 4.6 s, 241 -> 14.0 s. Plots byte-identical and test lines
  identical to before. stamp-core: `stats/eigen.rs` (nalgebra 0.35, pure
  Rust, new dependency: full decomposition and eigenvalues; Lanczos for
  the leading pairs, checked against the eigenvalues, falls back on the
  full decomposition), `pca::{pca_leading, coordinates, double_centre}`
  (sparse Gram matrix), `diversity::pcoa_leading`, sparse Bray-Curtis /
  Jaccard (bit-identical), Aitchison in four sums, `CommunityTest::run_with`
  (precomputed squared distances / ranks / group sizes, flat PERMDISP
  points; RNG draws unchanged), NMDS Guttman transform pair by pair and a
  division- and branch-free monotone regression; `table::num` writes whole
  numbers as integers. stamp-plot: `community_plots::{Layout, pcoa_layout,
  draw_pcoa, nmds_layout, draw_nmds}`, `pca_plot::{layout, draw}`. CLI:
  community tests run once. GUI: `plots::OrdinationCache`
  (`Session::ordinations`, `StampApp::ordinations_computed`), results table
  rebuilt only when results/filters change (`table_key`), generation kept
  increasing across profile loads (the figure cache could otherwise show a
  plot of the previous data). Tests: eigen fast paths vs Jacobi, distances
  / PCoA / NMDS vs first implementations, GUI
  `styling_an_ordination_does_not_compute_it_again`. FAQ entry on timings.
  Still slow (>2 s): NMDS at 1,307 (~9 s), PCoA at 1,307 (distances 1.8 s,
  full eigen 1.8 s, tests 2.9 s), results table output (~1.1 s / 600 MB at
  619, ~2.2 s / 1.3 GB at 1,307). **Not committed** (instructed to leave
  changes uncommitted).

- **Ordination colours and shapes, and a 20-colour palette** (2026-11-15),
  on top of the other uncommitted work. stamp-plot:
  `ordination_style.rs` (`ColorSource`, `ColorScale`, `DrawOrder`,
  `Styling`, `resolve`, `is_numeric_field`, `is_missing`), `pca_plot.rs`
  (`Options::{color_by, shape_by, color_map, color_scale, draw_order,
  opacity}`, `Data::styling`, legend beside the plot with colour bar and
  shape section, hollow grey "No value" points, draw order, tooltips),
  `community_plots::Community::styling` (PCoA note that the tests compare
  the groups), `axes::{GROUP_COLORS (20), group_color cycles,
  repeated_colors_note, Marker::ALL (10 shapes), marker_styled}`,
  `heatmap::ColorMap::{Viridis, Plasma, Magma, Cividis}`. CLI:
  `OrdinationArgs` (`--color-by`, `--shape-by`, `--color-scale`,
  `--color-map`, `--draw-order`, `--marker-size`, `--opacity`), warning
  beyond 20 groups. GUI: "Colours and shapes" group in the PCA/PCoA/NMDS
  dialogs (`plot_config::Context::numeric_fields`), styling resolved in
  `plots.rs` (`ordination_styling`), figure cache keyed on the selected
  feature when colouring by it. Tests (plot, CLI, GUI incl.
  `ordination_settings_dialogs`), docs (plots.rst section and figure
  `mg_pcoa_coloured.png`, command_line.rst), NEW_FEATURES,
  CHANGES_FROM_PYTHON 60. No existing default changed, so DEFAULTS_VERSION
  stays 2. **Not committed** (instructed to leave changes uncommitted).

- **Multi-feature box plot with a log axis, and the prevalence-abundance
  plot** (2026-11-15), on top of the (also uncommitted) SingleM/sample
  filter/group heatmap/co-occurrence work. stamp-plot: `multi_box.rs`
  (`feature_plots::box_plot` now delegates to it; grouped boxes or one
  panel per feature, shared/independent axes, log10 axis with a 0 line
  below a break, jittered points, n per group, horizontal, group order
  given/alphabetical/median/metadata field with dashed category lines),
  `BoxOptions` fields, `BarOptions::log_scale`, `axis::log_ticks*`,
  `prevalence_plot.rs` (panels, combined, specificity views; highlighted
  features). stamp-core: `stats/prevalence.rs` (`occupancy`,
  `profile_occupancy`, `FeatureOccupancy::{home_group, specificity}`),
  `table::prevalence_table`. CLI: repeatable `--plot-feature`, `BoxArgs`
  (`--box-*`, `--bar-log`), `PrevalenceArgs` (`--prevalence-*`,
  `--highlight`), plot type `prevalence-abundance`. GUI:
  `PlotKind::PrevalenceAbundance`, `BoxFeatures` (selected / most
  significant N / listed; Ctrl+click in the feature table,
  `StampApp::toggle_box_feature`), dialogs. Tests (core, plot, CLI, GUI incl.
  `box_and_prevalence_settings_dialogs`), docs (plots.rst three new figures,
  command_line.rst, main_window.rst), NEW_FEATURES, README. No existing
  default changed, so DEFAULTS_VERSION stays 2. **Not committed**
  (instructed to leave changes uncommitted). Sphinx not installed; RST
  checked by eye.

- **Group heatmap and co-occurrence heatmap** (2026-11-15), on top of the
  (also uncommitted) SingleM/Sandpiper + sample filter work. stamp-core:
  `stats/group_summary.rs` (`GroupStatistic`, `summarise`,
  `majority_values`), `stats/co_occurrence.rs` (`co_occurrence`,
  `partners`, `top_features`; reference values from SciPy 1.16.0),
  `co_occurrence_table`, `partner_table`, `group_summary_table` in
  `stats/table.rs`; serde on `CorrelationMethod`. stamp-plot:
  `group_heatmap.rs` (separate plot type; reuses `heatmap::{arrange,
  dendrogram, ColorMap, Order}`), `co_occurrence_plot.rs` (matrix and
  one-feature views). CLI: `--plot-type group-heatmap` with `HeatmapArgs`
  (`--heatmap-*`, flattened into two-groups and multi-groups; colour map,
  order, linkage and dendrograms also apply to the sample heatmap);
  `stamp-cli co-occurrence`. GUI: `PlotKind::GroupHeatmap`,
  `PlotKind::CoOccurrence`, their dialogs (`plot_config::show` now takes a
  `Context` of metadata fields and groups; the Configure plot window
  scrolls). Tests (core, plot, CLI, GUI incl. `heatmap_settings_dialogs`),
  docs (plots.rst with three new figures, statistics.rst caveat,
  command_line.rst), NEW_FEATURES, README. No default of existing plots
  changed, so DEFAULTS_VERSION stays 2. **Not committed** (instructed to
  leave changes uncommitted). Sphinx not installed; RST checked by eye.

- **SingleM/Sandpiper importers and the sample filter** (2026-11-15):
  `stamp-core/src/import/{lineage,singlem,sandpiper}.rs` (seven ranks,
  `unclassified <prefixed taxon>` padding, prefixes stripped from named
  taxa, `name (parent)` for a name with two parents at one rank, values to
  six significant digits; fixtures in `crates/stamp-core/tests/data/singlem`
  with a reference from the Python script); `stamp-cli convert
  singlem|sandpiper`; File > Load profile detects both (several files at
  once for Sandpiper) and File > Convert offers them.
  `stamp-core/src/selection.rs` (`SampleSelection`, `Condition`) with
  `Metadata::set_selection`; `--where`/`--where-not` on every command that
  reads metadata (and `info`); the window's Sample filter in the groups
  section (`Session::set_sample_filter`, `StampApp::set_sample_filter`).
  Tests, docs (input_files, command_line, analyses, main_window),
  NEW_FEATURES, CHANGES_FROM_PYTHON (59), README. **Not committed**
  (instructed to leave changes uncommitted). Sphinx was not installed, so
  the docs build was not checked.

- **User documentation** (2026-11-15): a Sphinx site in the style of
  CheckM's (Read the Docs theme) in `docs/site/`: installing, quick start,
  FAQ, input files, the main window, analyses, command line, a statistics
  guide for biologists (`statistics.rst`: p-values, effect sizes, CIs,
  corrections, every test and post-hoc test, CLR, diversity, distances,
  community tests, Mantel, ANCOM, NMDS stress, with worked examples from
  the figures), `choosing_a_test.rst`, a plots guide (`plots.rst`, every
  plot with "what to look for"), changes from v2 and citing. Builds with
  `sphinx-build -W` without warnings. The Pages workflow now also builds it
  into `dist/docs`. Small code changes made along the way: the command
  line's PCA/PCoA/NMDS plots get a legend, its p-value histogram honours
  `--plot-uncorrected-p-values`, and the window's correlation p-value
  histogram labels uncorrected p-values correctly. Committed and pushed.

- **Optimisation and fixes** (2026-11-15): rarefaction curves computed once
  per distinct count (6.9 s to 0.7 s on 120 samples x 3000 features);
  ANCOM's ANOVA path compares F with Holm critical values instead of
  computing p-values (2.7 s to 1.0 s, 4.0 s to 1.35 s with four groups;
  identical W); the window keeps the last six figures by a fingerprint of
  their inputs (`Session::plot_fingerprint`, `figure_cache` in `app.rs`);
  web build with the `web` profile (fat LTO, one codegen unit) and one copy
  of resvg/usvg (0.45, as svg2pdf uses) without image or system-font
  features: 21.3 MB to 13.4 MB (8.4 to 5.1 MB gzipped); a download progress
  bar (`crates/stamp-gui/initializer.js`). Also: plot text placed by each
  figure's own font (was wrong for every font but Arial), group headings of
  the plot selector at the size of plot names, feature names and p-values
  at 9 pt (`DEFAULTS_VERSION` 2). Committed and pushed.
- **Plot font choice** (2026-11-15): `PlotFont` in `stamp-plot/src/text.rs`
  (Arial default, Times New Roman, Courier New, DejaVu Sans; bundled as
  Liberation Sans/Serif/Mono 2.1.5 under the SIL OFL, downloaded with the
  user's consent from the liberation-fonts GitHub release, and DejaVu
  Sans); part of `axes::Fonts`, recorded in each `Figure`, used by
  measuring, the egui painter, SVG and PNG/PDF export; Preferences "Plot
  font", command line `--font`. Committed and pushed.
- **Plots grouped in the selector** (2026-11-15): `PlotGroup` and
  `PlotKind::group`/`grouped` in `stamp-gui/src/plots.rs`, drawn by
  `choose_plot` in `app.rs`; `PlotKind::available` still sets each mode's
  default (its first plot). Committed and pushed.
- **Plot font sizes** (2026-11-15): ticks 8 pt, axis labels and legends 9
  pt by default, set in Settings > Preferences (`PlotSettings::fonts`,
  `stamp_plot::axes::Fonts`, applied with `axes::with_fonts` while a figure
  is built); text particular to a plot sized by fields of its options
  (`title_font_size`, `note_font_size`, `label_font_size`,
  `count_font_size`) in a "Font sizes" frame of Configure plot; command
  line `--tick-font-size`, `--axis-label-font-size`, `--legend-font-size`.
  Committed and pushed.
- **Review of the statistics against trusted references** (2026-11-15):
  every test, post-hoc test, effect size, interval and correction in
  `stamp-core/src/stats` now has unit tests whose values come from R 4.4.3
  (stats, DescTools, PMCMRplus, dunn.test, vegan, qvalue), SciPy,
  statsmodels or scikit-bio, each naming its source; the R values are
  reproduced by `tests/reference/reference_values.R`. Fixed: the
  Asymptotic-CC interval, exact Newcombe-Wilson limits, the hypergeometric
  p-value cap, p-values below 1e-16, Storey's caps, the ratio-of-proportions
  pseudocount (CHANGES_FROM_PYTHON bugs 22-27, behaviour 55). Golden tests
  assert the intended relationship to v2's values. Then, at the user's
  request, Kruskal-Wallis follows R for groups of any size (behaviour
  change 56). Committed and pushed.
- **Roadmap tier 4** (2026-11-14): ANCOM (`stats/ancom.rs`, matches
  scikit-bio 0.7.4), the Mantel test (`community::mantel`), NMDS
  (`diversity::nmds`), and the plots `PlotKind::{Ancom, Nmds,
  SharedFeatures, RankAbundance, Mantel}` (`ancom_plot.rs`,
  `shared_features.rs`, `rank_abundance.rs`, `community_plots.rs`); the
  `stamp-cli ancom` command and plot types `ancom`, `nmds`,
  `shared-features`, `rank-abundance`, `mantel`. Tests, lint and web
  bundle done. Committed and pushed.
- **Roadmap tier 3** (2026-11-14): paired t-test and Wilcoxon signed-rank
  test (`GroupTest::PairedT`, `WilcoxonSignedRank`; pairs from
  `Metadata::paired_groups`, the session's `groups.pair_field`); centred
  log-ratio `Transform` on the two-group and multiple-group analyses
  (p-values only); correlation with a numeric metadata field as a fourth
  mode (`Mode::Correlation`, `stats/correlation.rs`, `CorrelationRun` in
  `stats/results.rs`, `stamp-plot/src/correlation_plot.rs`,
  `PlotKind::CorrelationScatter`); command line `--test paired-t|wilcoxon
  --pair-field`, `--transform clr`, and the `correlation` command. Values
  checked against SciPy 1.17. Tests, lint and web bundle done. Committed
  as `bac1bc4` and pushed.
- **Roadmap tier 2** (2026-11-14): alpha diversity, distances
  (Bray-Curtis, Jaccard, Aitchison) and PCoA in
  `stamp-core/src/stats/diversity.rs`; PERMANOVA, PERMDISP and ANOSIM in
  `stats/community.rs`; the plots in `stamp-plot/src/community_plots.rs`
  (`PlotKind::AlphaDiversity`, `Pcoa`, `DistanceBox`, in the two group
  modes) with their Configure plot groups and `--plot-type
  alpha-diversity|pcoa|distance-box` (`--alpha-metric`, `--distance`,
  `--rarefy`). Values checked against scikit-bio 0.7.4. Chao1 and Chao2
  switched to the bias-corrected form. Unclassified categories are not
  features in these analyses. Committed as `ff2ba03`.
- **Roadmap tier 1** (2026-11-14): Mann-Whitney U test
  (`GroupTest::MannWhitney`), Dunn's post-hoc test (`PostHocTest::Dunn`),
  volcano plot (`stamp-plot/src/volcano.rs`) and composition bar plot
  (`stamp-plot/src/composition.rs`), each in the GUI and on the command
  line, with tests. Recorded in `NEW_FEATURES.md` and ticked in
  `docs/ROADMAP.md`. Check `git status` for whether it is committed.
- **Remembered plot settings versioned** (2026-11-14): `DEFAULTS_VERSION`
  and `Saved::up_to_date` in `stamp-gui/src/app.rs`, so that settings
  stored before a default changed no longer override it (this was why
  legends were not at "Best"). Rule added to `CLAUDE.md`. Check
  `git status` for whether it is committed.
- **Group-level rarefaction curves** (2026-11-14): "Curves for" in the
  rarefaction plot offers each sample, each group pooled by sequences, and
  each group by number of samples (`accumulation`, `chao2` and
  `absence_probability` in `stamp-core/src/stats/rarefaction.rs`; `Curves`
  in `stamp-plot/src/rarefaction.rs`; `--rarefaction-curves` on the command
  line). Tests, lint and web bundle done; check `git status` for whether it
  is committed.
- **Example data sub-menu** (2026-11-14): File > Open example data offers
  "Enterotypes" and "Coal seam gas"; the coal profile is compressed by
  `crates/stamp-gui/build.rs` and unpacked when loaded.
- **Rarefaction curve plot** (2026-11-14): `stats/rarefaction.rs` in
  `stamp-core`, `rarefaction.rs` in `stamp-plot`, `PlotKind::Rarefaction` in
  all three analysis types with its Configure plot group, and
  `--plot-type rarefaction` on the command line. Decisions from the user:
  all three analysis types; profiles with fractional counts are refused
  with a message; unclassified categories are never counted as features.
  Recorded in `NEW_FEATURES.md`. Tests, lint and the web bundle were done;
  check `git status` for whether it has been committed.
- Before that: `NEW_FEATURES.md`, this file, the rules in `CLAUDE.md`, and
  version 3.0.0.

## State of the repository

- Branch `rust-port`, pushed to `origin` (tag `feature-parity` is on
  `6b834ba`).
- Tiers 1 to 3 are committed and pushed: `466fc38`, `ff2ba03`, `bac1bc4`.

## Next

Nothing queued. A plan for an assistant built on Claude Code (local MCP
server in the desktop app, launching Claude Code) is in
`docs/ASSISTANT_PLAN.md`, saved but not started. What v2 has that the port still lacks is listed under "Not
yet ported" in `docs/CHANGES_FROM_PYTHON.md`; `docs/ROADMAP.md` lists what
was considered and left out.
