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
| 7 | done | `metagenomics/ProfileTree.py`, `createMultiGroupProfile` | Stores a feature under its untruncated name while looking it up by the truncated name (the ` - #` suffix), unlike the two-sample and two-group profiles. | Use the truncated name consistently. |
| 8 | done | `metagenomics/fileIO/StampIO.py` | A profile with two samples of the same name was accepted and their counts silently added together. | Rejected with a message naming the sample. |
| 9 | done | `metagenomics/fileIO/StampIO.py`, `MetadataIO.py` | Several malformed inputs crashed and were reported only as "Unknown parsing error": an empty file, a file with only a header, an empty cell in the first data row, a metadata row longer than the header. | Each gives a specific message, or is tolerated (short metadata rows are padded with empty values, extra cells ignored). |
| 10 | done | `metagenomics/fileIO/MetadataIO.py` | A sample listed twice was reported as an "unknown sample", and that warning was overwritten if any sample lacked metadata. | A separate warning for repeated samples; all warnings are returned. |
| 11 | done | `GUI/createProfileMgRastDlg.py` | A blank line in the middle of an MG-RAST table added a sample with an empty name. | Blank lines are skipped. |
| 12 | done | `GUI/createProfileCoMetDlg.py` | A CoMet entry with an empty description (as in the GO example, `GO:0018346 (): 5`) produced a row with an empty name, giving a profile STAMP could not load. | Such entries are named by their identifier. |
| 13 | done | `GUI/createProfileMothurDlg.py` | Sequence ids in a names file were not stripped of spaces; blank lines and ids missing from the groups or names file crashed the conversion. | Ids are stripped, blank lines skipped, and missing ids reported by name. |
| 14 | done | `GUI/createProfileBiomDlg.py` | Observation metadata given as a single string (QIIME's `k__Bacteria; p__...` taxonomy) was split into single characters; nested lists crashed. | A string is split on `;`. Nested lists (several pathways per gene) give an explanatory error. |
| 15 | done | `metagenomics/fileIO/COG_IO.py` | An unknown COG identifier or a blank line crashed the conversion. | Unknown identifiers are reported by name; blank lines are skipped. |

## Behaviour changes

| # | Status | Change |
|---|---|---|
| 1 | planned | User-supplied plugins are not supported. Tests, corrections, effect sizes and plots are built in. |
| 2 | planned | PS and EPS export are dropped. Plots export as PNG, SVG and PDF. |
| 3 | planned | Holm-Bonferroni becomes a built-in correction returning standard adjusted p-values. The Python version (in `additional/`, never loaded by the GUI) only marked non-significant features with infinity. |
| 4 | planned | Barnard's exact test (in `additional/`, never loaded by the GUI) is not ported. |
| 5 | planned | Results-table row order is fully defined (p-value, then feature name). In Python, the order of tied p-values depended on dictionary ordering. |
| 6 | planned | Benjamini-Hochberg uses the standard adjustment (as R's `p.adjust(method = "BH")`): `p * n / rank` followed by a running minimum from the largest p-value down, capped at 1. Python omitted the running minimum, so tied p-values could get different corrected values and corrected values were not monotonic. Corrected p-values can therefore be smaller than in STAMP v2, never larger. The `corr:Benjamini-Hochberg FDR` columns in `tests/golden` hold the old values and are an expected difference. |
| 7 | done | Features, and the rows of converted profiles, are kept in order of first appearance in the input. Python used dictionary order, which was arbitrary. |
| 8 | done | Counts written by the BIOM and CoMet converters use the shortest exact decimal form instead of Python's 12 significant digits. |
| 9 | done | BIOM 2.x (HDF5) tables are detected and rejected with advice to convert to JSON. |
| 10 | done | The COG lookup table is a compact extract (`crates/stamp-core/data/cog_codes.tsv`) of `whog.txt`, embedded in the program. |

## Kept as is

* Tukey-Kramer and Games-Howell report binned p-values from the critical-value
  tables. Exact studentized-range p-values are a later enhancement.
* Effect-size filters combine as in Python: a feature is removed only if it
  fails **both** filters under "AND", and if it fails **either** under "OR".
  The filters are worded as removal rules ("filter out features with an effect
  size less than..."), so this is the intended logic, not a bug.

## Open questions

* **BIOM 2.x (HDF5).** Not supported in this port so far (see behaviour
  change 9). Supporting it needs an HDF5 reader that also works in the browser.
* **Example profiles the reader rejects.** `examples/EBPR/MartinEBPR.spf`,
  `examples/EBPR/MartinEBPR_A_phosphatis.spf`, `examples/FinegoldAutism/Autism.spf`
  and `examples/MouseGut-IMG-M/MouseGut.COG.profile.tsv` fail the hierarchy
  checks (see `load_errors` in `tests/golden/manifest.json`). The messages
  point at problems in the files themselves, for example the typo
  "Gammeproteobacteria" in the autism profile. Likewise, converting
  `examples/BIOM-test/kegg_predictions_L3.biom` with its `KEGG_Pathways`
  hierarchy gives a profile that is rejected, because KEGG's top-level
  "Unclassified" category has classified children.
