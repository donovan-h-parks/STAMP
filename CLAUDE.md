# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

STAMP (statistical analysis of taxonomic and functional profiles) being ported from Python 2 / PyQt4 to Rust. The Rust port lives in `crates/` on the `rust-port` branch; the original application is kept, unmodified, in `legacy/` as the reference for behaviour. The port must run as a desktop application on Linux, Windows and macOS and as a web page (WebAssembly).

`docs/CHANGES_FROM_PYTHON.md` is the record of every deliberate difference from the Python version: bugs fixed, behaviour changes, what is not yet ported, and open questions. **Any change that makes the port behave differently from STAMP v2, or that fixes a v2 bug, gets an entry there.** Read it before concluding that a difference from the Python code is a mistake.

## Files to keep up to date

- **`CURRENT_TASK.md`** always says what is being worked on, so that a session that ends unexpectedly (a lost connection, a server crash) can be picked up by the next one. Read it at the start of a session. Update it when a task starts (what was asked, the plan), at each meaningful step (what is done, what is left, anything half-finished on disk), and when the task finishes (set the status to idle and note what was completed and whether it was committed).
- **`NEW_FEATURES.md`** lists the features of the Rust port that STAMP v2 did not have. Add an entry whenever such a feature is added.
- **`docs/CHANGES_FROM_PYTHON.md`**, as described above, for differences from v2 and fixed v2 bugs.

## Version

The Rust port is **STAMP v3.0.0**. The version is set once, in `[workspace.package]` of the root `Cargo.toml`, and every crate and the About window take it from there. STAMP v2 is the Python version in `legacy/`.

## Commands

The compiler is pinned in `rust-toolchain.toml` (rustup installs it on first use, with the wasm target).

```bash
cargo build --release                          # everything; binaries: target/release/stamp (GUI), stamp-cli
cargo test --workspace --release               # all tests (debug builds make the resampling tests slow)
cargo test -p stamp-core --release --lib dist  # tests whose name contains "dist" in one crate
cargo test -p stamp-gui --release --test ui tables_are_sorted   # one GUI test
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p stamp-gui --target wasm32-unknown-unknown -- -D warnings   # the browser build must stay warning-free too
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy for both targets, and the tests on Linux, Windows and macOS; all three checks above must pass before a change is done.

Browser build (needs `trunk`, `cargo install trunk --locked`):

```bash
cd crates/stamp-gui && trunk build --release   # writes crates/stamp-gui/dist (untracked)
```

GUI screenshots: the GUI tests render the real interface with a software rasteriser and can write PNGs, which is the only way to look at the interface on a machine without a display:

```bash
STAMP_SCREENSHOTS=/some/dir cargo test -p stamp-gui --release --test ui
```

Reference outputs from the Python version (only needed when adding reference cases):

```bash
cd legacy/golden && pixi run golden      # regenerates tests/golden from the Python 2.7 code
cd legacy/golden && pixi run unittests   # the original Python unit tests
```

Packaging and releases are described in `docs/RELEASING.md` (`scripts/package.sh`, tag `v*` triggers the release workflow).

## Architecture

Four crates, each depending only on those above it:

- **`stamp-core`** — profiles, metadata, file import and all statistics. No file system access, threads or UI, so it compiles unchanged to WebAssembly; callers hand it text or bytes. Long analyses take a progress callback and exist in a resumable form (`SampleRun`, `GroupRun`, `MultiGroupRun` in `stats/results.rs`) that can pause between features and gives identical results wherever it pauses.
- **`stamp-plot`** — every plot builds a backend-neutral `Figure` (sizes in points; shapes plus `regions` that carry tooltips and the item each area stands for). `svg.rs` writes SVG; PNG and PDF are derived from the SVG (`export.rs`, optional features). Text is measured with the bundled DejaVu Sans so layout is the same everywhere.
- **`stamp-cli`** — analyses, plots and file conversion from the command line; output tables keep STAMP v2's column headings.
- **`stamp-gui`** — egui/eframe application, built both as the `stamp` binary and as the web page.

Inside `stamp-gui`:

- `session.rs` holds the state and logic of an analysis session (loaded data, settings, results, running jobs) with no UI code, so it is tested on its own. `app.rs` is the window and owns only presentation state.
- The same `Figure` is drawn on screen by `figure_painter.rs` using egui's painter (the "two renderers" design: native drawing on screen, SVG for export). `plots.rs` chooses and builds the figure from the session and holds `PlotSettings`, one options struct per plot, edited by `plot_config.rs`.
- `jobs.rs` runs analyses on a thread on the desktop and in short time slices on the page's single thread in the browser. `platform.rs` hides file access behind a request/event queue: native dialogs on the desktop, the browser's file picker and downloads on the web. Code that must differ between the two targets belongs in these two modules, behind `cfg(target_arch = "wasm32")`.
- `headless.rs` is the software renderer used by `tests/ui.rs`. GUI behaviour is exposed through public methods on `StampApp` (for example `set_plot_settings`, `open_profile_file`, `sort_feature_table`) so tests drive the same code paths as the controls.

## Testing against the Python version

`tests/golden/` holds 108 reference cases produced by the Python code (`manifest.json` describes them). `crates/stamp-core/tests/golden_*.rs` compare the Rust results with them, with tolerances explained beside each constant. Where the port deliberately differs (a fixed bug, the standard Benjamini-Hochberg method, computed post-hoc p-values), the test asserts the intended relationship to the Python value rather than equality, and the difference is documented in `docs/CHANGES_FROM_PYTHON.md`.

`examples/` contains real datasets used by tests and as built-in example data; a few of them are intentionally rejected by the loader (listed under `load_errors` in the manifest).

## Conventions specific to this repository

- Numerics: `libm` for `erfc`/`lgamma` and the project's own distribution functions in `stats/dist.rs`; do not swap in another crate's special functions without re-checking the reference tests, which are sensitive at the 1e-9 level.
- Random numbers are seeded (ChaCha8) so resampling results are repeatable; a change in the order random numbers are drawn changes results and reference tests.
- User-visible labels follow STAMP v2's wording where v2 has the control (see the Python `*.ui` files and `configGUI/*UI.py`).
- Dependencies must build for `wasm32-unknown-unknown`; nothing that needs a C library.
- rustfmt rewraps code narrowly, so re-read a file after `cargo fmt` before making exact-text edits to it.
