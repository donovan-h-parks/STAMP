# Differences between the Rust port and STAMP v2 (Python)

This file records every place where the Rust port deliberately behaves
differently from the Python implementation in `legacy/`. Each entry is added
when the corresponding code is ported, together with a test that pins the new
behaviour. Where a fix changes numbers, the affected reference case in
`tests/golden` is marked as an expected difference rather than regenerated.

Status: **planned** = found, not yet ported. **done** = ported and tested.

## Bugs fixed

| # | Status | Python location | Problem | Behaviour in the port |
|---|---|---|---|---|
| 1 | planned | `metagenomics/stats/MultiGroupStatsTests.py`, `getPValueStr` | Compares the formatted string, not the number, with the minimum reportable p-value, so the "< 1e-15" form is never produced for post-hoc p-values. | Compare the numeric p-value. |
| 2 | planned | `plugins/groups/effectSizeFilters/RatioProportions.py` | Divides by zero when a feature has zero abundance in both groups, which aborts filtering. | Report a ratio of 1 for a feature absent from both groups. |
| 3 | planned | `plugins/multiGroups/postHoc/GamesHowell.py` | Crashes (NaN degrees of freedom) when a group has a single sample. | Return a degenerate result with an explanatory note, as the other post-hoc tests do. |
| 4 | planned | `plugins/samples/statisticalTests/Bootstrap.py` | The "parent has a count of zero" note is computed and then dropped. | Return the note. |
| 5 | planned | `plugins/groups/statisticalTests/White.py` | Prints a leftover debug line (`blah`) on every run. | Removed. |
| 6 | planned | `plugins/common/multipleComparisonCorrections/StoreyFDR.py`, resampling tests | Random numbers are unseeded, so results differ between runs. | A seeded generator with a user-visible seed; the same seed gives the same result. |
| 7 | planned | `metagenomics/ProfileTree.py`, `createMultiGroupProfile` | Stores a feature under its untruncated name while looking it up by the truncated name (the ` - #` suffix), unlike the two-sample and two-group profiles. | Use the truncated name consistently. |

## Behaviour changes

| # | Status | Change |
|---|---|---|
| 1 | planned | User-supplied plugins are not supported. Tests, corrections, effect sizes and plots are built in. |
| 2 | planned | PS and EPS export are dropped. Plots export as PNG, SVG and PDF. |
| 3 | planned | Holm-Bonferroni becomes a built-in correction returning standard adjusted p-values. The Python version (in `additional/`, never loaded by the GUI) only marked non-significant features with infinity. |
| 4 | planned | Barnard's exact test (in `additional/`, never loaded by the GUI) is not ported. |
| 5 | planned | Results-table row order is fully defined (p-value, then feature name). In Python, the order of tied p-values depended on dictionary ordering. |
| 6 | planned | Benjamini-Hochberg uses the standard adjustment (as R's `p.adjust(method = "BH")`): `p * n / rank` followed by a running minimum from the largest p-value down, capped at 1. Python omitted the running minimum, so tied p-values could get different corrected values and corrected values were not monotonic. Corrected p-values can therefore be smaller than in STAMP v2, never larger. The `corr:Benjamini-Hochberg FDR` columns in `tests/golden` hold the old values and are an expected difference. |

## Kept as is

* Tukey-Kramer and Games-Howell report binned p-values from the critical-value
  tables. Exact studentized-range p-values are a later enhancement.
* Effect-size filters combine as in Python: a feature is removed only if it
  fails **both** filters under "AND", and if it fails **either** under "OR".
  The filters are worded as removal rules ("filter out features with an effect
  size less than..."), so this is the intended logic, not a bug.

## Open questions

* **Example profiles the reader rejects.** `examples/EBPR/MartinEBPR.spf`,
  `examples/EBPR/MartinEBPR_A_phosphatis.spf`, `examples/FinegoldAutism/Autism.spf`
  and `examples/MouseGut-IMG-M/MouseGut.COG.profile.tsv` fail the hierarchy
  checks (see `load_errors` in `tests/golden/manifest.json`). The messages
  point at problems in the files themselves, for example the typo
  "Gammeproteobacteria" in the autism profile.
