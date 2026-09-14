# STAMP on Python 3.13 — setup & handoff

This document is a self-contained handoff for continuing the STAMP Python 3.13 / PyQt6
migration on a fresh machine (e.g. a VM). It covers how to stand up the environment, how to
run and test the app, what has been fixed and verified, and what is left to do.

- **Branch:** `python3_migration`
- **Target stack:** Python 3.9–3.13, PyQt6, numpy 2.x, scipy 1.1x, matplotlib 3.x, biom-format 2.1.x
- **Verified against:** Python 3.13, PyQt6 6.x, numpy 2.5, scipy 1.18, matplotlib 3.10, biom 2.1.17
- **Detailed change log:** see [`MIGRATION_NOTES.md`](MIGRATION_NOTES.md). Stages 0–2 are the
  mechanical migration; **Stage 6** lists every runtime GUI bug fixed in this pass.

---

## 1. Environment setup

STAMP needs a real Python 3.13 interpreter and the packaged dependencies from
[`pyproject.toml`](pyproject.toml). It is a **Qt GUI app**, so on a headless VM you also need
either a display or the Qt "offscreen" platform (used by the test harness).

### Linux VM (Debian/Ubuntu)

```bash
# 1. System packages: Python 3.13 + the shared libs Qt/matplotlib need at runtime.
#    (deadsnakes gives you 3.13 on older Ubuntu; skip the PPA if your distro ships 3.13.)
sudo add-apt-repository ppa:deadsnakes/ppa
sudo apt-get update
sudo apt-get install -y python3.13 python3.13-venv python3.13-dev \
    libgl1 libegl1 libxkbcommon0 libdbus-1-3 \
    libfontconfig1 libfreetype6 \
    xvfb            # only needed to run the *GUI* headlessly; the smoke-test doesn't need it

# 2. Create the virtualenv and install STAMP (editable) with all deps.
cd /path/to/STAMP
python3.13 -m venv .venv313
.venv313/bin/python -m pip install --upgrade pip
.venv313/bin/python -m pip install -e .
```

If Qt complains about a missing platform plugin when you launch the GUI, it is almost always
one of the `lib*` packages above (most commonly `libxkbcommon0`, `libegl1`, or `libgl1`).

### macOS VM

```bash
brew install python@3.13        # or use an existing /usr/local/bin/python3.13
cd /path/to/STAMP
python3.13 -m venv .venv313
.venv313/bin/python -m pip install --upgrade pip
.venv313/bin/python -m pip install -e .
```

> The venv is intentionally named `.venv313`. It is **not** committed — recreate it on the VM
> with the commands above. Everything else needed to rebuild it is in `pyproject.toml`.

---

## 2. Running the app

```bash
# Normal launch (needs a display):
.venv313/bin/python -m stamp

# Headless VM with no display — run the GUI under a virtual X server:
xvfb-run -a .venv313/bin/python -m stamp
```

Load a dataset via **File → Open** and point it at one of the bundled `examples/` profiles,
e.g. `examples/EnterotypesArumugam/Enterotypes.profile.spf` with metadata
`examples/EnterotypesArumugam/Enterotypes.metadata.tsv`.

---

## 3. Testing without a display (the regression harness)

The key debugging insight from this pass: **STAMP launches fine — the crashes are on
*actions*** (loading data, running a test, drawing a plot), not on startup. So a bare launch
proves almost nothing.

[`tools/py313_smoketest.py`](tools/py313_smoketest.py) builds the real main window, loads an
example dataset, and then exercises **every** statistical test, post-hoc test, and plot type
across the sample / group / multi-group modes, printing OK/FAIL per action. It uses Qt's
offscreen platform, so **no display or xvfb is required**.

```bash
# Full run against the default example dataset:
QT_QPA_PLATFORM=offscreen .venv313/bin/python tools/py313_smoketest.py

# Or point it at a different profile + metadata pair to widen coverage:
QT_QPA_PLATFORM=offscreen .venv313/bin/python tools/py313_smoketest.py \
    examples/FinegoldAutism/Autism.spf examples/FinegoldAutism/Austism.metadata.tsv
```

It exits non-zero if any action fails, so it can gate CI. Current status against the default
dataset: **all actions pass.**

You can also byte-compile the whole tree as a cheap syntax check:

```bash
.venv313/bin/python -m compileall -q stamp
```

---

## 4. What has been done

### Stages 0–2 (mechanical migration — pre-existing)
Build system moved to `pyproject.toml`; Python 2→3 syntax; PyQt4→PyQt6 imports/enums/signals.
Full detail in `MIGRATION_NOTES.md`.

### Stage 6 (runtime GUI bug fixes — this pass)
The tree byte-compiled but crashed once data was loaded and analyses/plots ran. Fixed the
following **classes** of bug (each fixed everywhere it occurred, not just the first hit) and
verified with the harness — 46/46 test/plot/post-hoc paths pass, plus BIOM import:

| # | Bug class | Files |
|---|-----------|-------|
| 1 | `dict.keys()/.values()` used as a list (lazy view in py3) | `metagenomics/Metadata.py`, `SampleProfile.py`, `GroupProfile.py`, `MultiGroupProfile.py`, both `plugins/**/HeatmapPlot.py` |
| 2 | `zip()` used as a list (single-use iterator in py3) | `plugins/samples/plots/ProfileBarPlots.py`, `MultCompCorrectionPlots.py`, `plugins/**/BoxPlot.py`, `plugins/PlotEventHandler.py` |
| 3 | `except X as name:` then using `name` after the block (py3 deletes it) | `plugins/groups/statisticalTests/Ttest.py` |
| 4 | Removed numpy/scipy aliases (`scipy.array`, `scipy.arange`, `random_integers`, `copy=0`) | `metagenomics/Bootstrap.py`, `plugins/common/multipleComparisonCorrections/StoreyFDR.py` |
| 5 | Removed matplotlib APIs (`Bbox.inverse_transformed`, `cm.spectral`) | `plugins/**/Abstract*PlotPlugin.py` (12 sites), both `HeatmapPlot.py` |
| 6 | PyQt6 `signal.disconnect(slot)` raises when not connected (PyQt4 returned `False`) | `GUI/metadataTableDlg.py` |
| 7 | biom-format 2.1: `table.sample_ids` removed → `table.ids(axis='sample')` | `GUI/createProfileBiomDlg.py` |
| 8 | Robustness: one failing plot poisoned every later plot switch | `plugins/PlotsManager.py` |

Fixes #1 and #2 preserve exact Python 2 behavior (both returned lists there), so **no numeric
results change**.

---

## 5. What is verified vs. not

**Verified (via the harness + manual conversion tests):**
- Load profile + metadata (`.spf` / `.tsv`).
- Every two-sample, two-group, and multi-group statistical test.
- Every multi-group post-hoc test.
- Every plot type (bar, box, heatmap, PCA, scatter, extended error bar, profile bar,
  sequence histogram, p-value histogram, multiple-comparison, post-hoc) in all three modes.
- BIOM file import → `.spf`, and reloading the produced `.spf`.
- All create-profile / preferences dialogs *construct* without error.
- Whole tree byte-compiles; real GUI launches with no traceback (macOS).

**Not yet exercised end-to-end (next work):**
- ~~The multi-step import wizards.~~ **All import wizards are now verified** (driven headlessly,
  each producing a `.spf` that reloads via `StampIO`):
  - **MG-RAST** — `examples/CowRumen-MG-RAST/CowRumen-MG-RAST.tsv`, incl. the Customize-headings
    sub-dialog; output matches the shipped `CowRumen.spf` exactly (7196 features, 4 levels,
    4 samples). No code change.
  - **Mothur** — synthetic `.taxonomy`/`.groups`/`.names` (no example ships); both with- and
    without-names paths round-trip. No code change.
  - **CoMet** — GO (8 samples, 3433 features) and Pfam (8 samples, 10554 features) datasets.
    One defensive fix in `createProfileCoMetDlg.py` (skip empty-category terms — a pre-existing
    data gap the shipped GO data hits, not a py3 bug).
  - **RITA** — synthetic RITA classifier output (the shipped `examples/EnterotypesRITA/*.tsv`
    are abundance matrices, i.e. STAMP *output*, not RITA *input*). No code change.
  - **Append COG** — `examples/Assign_COGs_Example.tsv`; both treatment options run. No code
    change. (The "Assign sequence to each COG code" option can produce a non-strict hierarchy
    on data where a leaf name repeats across categories — a data property, correctly rejected
    by `StampIO`.)
- Non-default heatmap **colourmaps** other than the default (only the `spectral` rename was
  spotted statically and fixed).
- **Saving plots to image** and the stats/results **table export**.
- A numerical regression vs. the Python 2 outputs — `MIGRATION_NOTES.md` flags true-division
  as the one silent-change risk. Running the legacy test datasets and diffing outputs is the
  intended safety net.

---

## 6. Migration agents

`.claude/agents/` holds four project-scoped Claude Code subagents that specialize in this
migration (they travel with the repo, so they work on the VM). See
[`.claude/agents/README.md`](.claude/agents/README.md):

- **migration-bug-hunter** — sweep & fix the known runtime bug classes.
- **gui-verifier** — run/extend the offscreen regression harness.
- **import-wizard-tester** — drive the file-import wizards against `examples/`.
- **numerical-regression** — guard against silent numeric drift.

## 7. Suggested next steps on the VM

1. Recreate `.venv313` (section 1) and run the harness (section 3) to confirm the environment.
2. Drive the remaining import wizards with the bundled `examples/` inputs; fix any of the same
   bug classes that surface (grep patterns are documented in Stage 6 of `MIGRATION_NOTES.md`).
3. Add the numerical regression check against the pre-migration outputs.
4. Commit the Stage-6 fixes (the migration uses one commit per logical stage).
