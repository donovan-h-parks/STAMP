# Reference outputs from STAMP v2 (Python)

These files are the baseline the Rust port is checked against. They are
produced by running the original statistics code headlessly over the datasets
in `examples/`. Do not edit them by hand.

To regenerate (needs [pixi](https://pixi.sh); creates a Python 2.7 environment):

```bash
cd legacy/golden
pixi run unittests   # the original 33 unit tests
pixi run golden      # rewrites tests/golden
```

## Layout

* `manifest.json` lists every case with the settings used to produce it
  (profile, metadata field, samples or groups, hierarchy levels, treatment of
  unclassified reads, test, confidence-interval method, coverage), and
  `load_errors`: example profiles the Python reader rejects, with its message.
* One `<case id>.tsv` per case. Prefixes: `s_` two samples, `g_` two groups,
  `m_` multiple groups, `..__posthoc` post-hoc tests on the five most
  significant features of the parent case, `storey_` Storey FDR q-values.

## Conventions

* Columns are STAMP's own results-table columns with "No correction" applied,
  followed by `corr:<name>` (p-values after Benjamini-Hochberg, Bonferroni and
  Sidak correction) and `es:<name>` (each effect-size measure used for
  filtering).
* Rows are sorted by the hierarchy columns (the first `key_columns` columns),
  not by p-value. Match rows by those columns.
* Floats are written in shortest round-trip form. Deterministic cases should
  match to about 1e-9 relative error.
* Cases with `"stochastic": true` (bootstrap, permutation, White's test,
  Storey FDR) use random resampling. Compare them within a statistical
  tolerance, never exactly.
* A cell beginning with `error:` records a crash in the Python code; see
  `docs/CHANGES_FROM_PYTHON.md`.

## Not covered here

* Filtering, PCA, clustering and plots.
* File converters. Use the paired files in `examples/` instead, for example
  `CowRumen-MG-RAST.tsv` and `CowRumen.spf`, or the `.biom` files and their
  ground-truth `.spf` files in `examples/BIOM-test`.
