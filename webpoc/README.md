# STAMP-web — proof of concept

A minimal **FastAPI + React** slice showing that STAMP's desktop analysis could move to the
browser **without rewriting the science**. The backend imports STAMP's existing
`stamp.metagenomics.*` core unchanged — no Qt, no matplotlib — loads a `.spf` profile +
metadata, builds a two-group profile, runs a statistical test through the real STAMP plugins,
applies a multiple-comparison correction, and returns JSON. The React frontend renders it as
an **interactive Plotly** extended-error-bar plot + results table (hover for exact stats,
zoom/pan, adjustable feature count).

This is a demonstrator, not a product — see "What it deliberately skips" below.

## Launch it like a desktop app

`stamp_web.py` is the "browser instead of the Qt GUI" entry point. It starts the server on a
free **local** port (127.0.0.1 only — nothing leaves your machine), builds the frontend once
if needed, and opens the UI. No accounts, no hosting, no data upload.

```bash
cd webpoc
../.venv313/bin/python stamp_web.py            # start + open your default browser
../.venv313/bin/python stamp_web.py --window   # open in its OWN app window (pip install pywebview)
../.venv313/bin/python stamp_web.py --no-browser --port 8000   # just run the server
```

`--window` uses **pywebview** (the OS's native webview) so it opens as a standalone window
rather than a browser tab — the closest "feels like the old GUI, minus Qt" experience.

## Run it (alternative: build + serve, no launcher)

```bash
cd webpoc
./run.sh          # installs deps, builds the frontend, serves everything on :8000
```

Then open <http://localhost:8000>. Both `run.sh` and `stamp_web.py` use STAMP's `.venv313`
(create it first with the repo-root `./setup_py313.sh` if you haven't).

### Dev mode (hot-reload frontend)

```bash
# terminal 1 — API
cd webpoc/backend && ../../.venv313/bin/python -m uvicorn app:app --reload --port 8000
# terminal 2 — Vite dev server (proxies /api to :8000)
cd webpoc/frontend && npm install && npm run dev      # http://localhost:5173
```

## How it reuses the STAMP core

`backend/app.py` calls the same objects the PyQt GUI does — this is the whole point:

| Step | STAMP core used (unchanged) |
|------|-----------------------------|
| Read profile / metadata | `fileIO.StampIO`, `fileIO.MetadataIO` |
| Pick a grouping field | `Metadata.setActiveField` |
| Build two-group profile | `ProfileTree.createGroupProfile` |
| Run the test | `stats.GroupStatsTests.run` + `plugins/groups/statisticalTests/*` |
| Correct p-values | `plugins/common/multipleComparisonCorrections/*` |

The one adaptation for headless use: the GUI passes a Qt `QProgressDialog` as `progress`;
the API passes `progress=None`, which the single-feature test path already supports.

## Capabilities

Five modes, all rendered as interactive Plotly:

- **Two samples**: one sample vs one sample — Fisher's exact, G-test (± Yates'), G-test+Fisher's,
  difference-between-proportions, chi-square, Hypergeometric, Bootstrap, Permutation (10 tests);
  significance filtering and six switchable plots (extended error bar, scatter, profile bar,
  p-value histogram, multiple-comparison, sequence histogram).
- **Two groups**: Welch's / Student's / White's non-parametric test, six multiple-comparison
  corrections, **significance filtering** (α on raw p or corrected q, min |effect|,
  significant-only), and five switchable plots — extended error bar, scatter, profile bar,
  p-value histogram, multiple-comparison.
- **Multi-group**: ANOVA / Kruskal-Wallis across all groups in a field, η² effect size,
  switchable plots (per-group means bar / per-feature **box plot** / p-value histogram),
  **plus post-hoc** pairwise comparisons for any feature (Tukey-Kramer, Games-Howell, Scheffé,
  Welch's uncorrected).
- **PCA** ordination: samples on PC1/PC2 (NumPy SVD of the sample × feature relative-abundance
  matrix), coloured by metadata group.
- **Heatmap**: the most-variable features × samples, hierarchically clustered on both axes
  (SciPy), with a per-sample group colour strip.

Plus **bring your own data** — an importer dropdown covers all of STAMP's input formats:
`.spf`, **BIOM**, **MG-RAST**, **Mothur**, **CoMet**, **RITA**, and **Append-COG**. Each is
converted to a profile on the server (Qt-free ports of STAMP's converters), validated, and then
usable in every mode; an optional metadata `.tsv` enables grouping.

## API

- `GET  /api/datasets` — example datasets (a `.spf` with a sibling `*metadata*.tsv`) + uploads
- `GET  /api/datasets/{id}/schema` — levels, fields + group sizes, sample list, tests, corrections
- `POST /api/sample` — two-sample → per-feature proportions, effect size, CI, p/q
- `POST /api/analyze` — two-group → per-feature means, effect size, CI, p/q
- `POST /api/multigroup` — ANOVA/Kruskal-Wallis → per-feature p/q/η² + per-group means
- `POST /api/posthoc` — pairwise group comparisons for one feature (effect + CI + p)
- `POST /api/distribution` — per-group, per-sample values of one feature (for the box plot)
- `POST /api/pca` — sample PC1/PC2 coordinates + group labels + variance explained
- `POST /api/heatmap` — clustered feature × sample matrix + per-sample group labels
- `POST /api/upload` — multipart profile (+ metadata) → registers a new dataset id
- `POST /api/import/biom` — BIOM table → profile
- `POST /api/import/{kind}` — `kind` ∈ mgrast|mothur|comet|rita|cog; multipart file(s) (+ metadata) → profile
  (`backend/importers.py` holds the Qt-free converters)

## What it still skips (the real project's remaining work)

- All ~11 of STAMP's distinct plot types are covered (extended error bar, grouped & profile
  bar, scatter, box, PCA, heatmap, post-hoc, p-value & sequence histograms, multiple-
  comparison). What remains is finish, not coverage: table export isn't wired (Plotly's toolbar
  already exports PNG), there are no effect-size filter plugins beyond the simple
  significance/min-effect filter, and none of the fine-grained desktop UI (active-group
  toggling, feature highlighting, parent level).
- No auth / persistence / multi-user, and no limits/queueing for the CPU-heavy permutation
  tests (White's/Bootstrap/Permutation take a few seconds). Fine for a **local single-user**
  app; a hosted service would need all of it.

The takeaway: the **scientific core ported straight over** — five analysis modes, ~19 tests
(10 two-sample, 3 two-group, 2 multi-group, 4 post-hoc), 6 corrections, all 6 importers, and the
full plot family run through STAMP's unchanged Python. The remaining work in a full web port is
productionization and polish, not the science.
