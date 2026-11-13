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
| 1 | done | `metagenomics/stats/MultiGroupStatsTests.py`, `getPValueStr` | Compares the formatted string, not the number, with the minimum reportable p-value, so the "< 1e-15" form is never produced for post-hoc p-values. | Compare the numeric p-value. |
| 2 | done | `plugins/groups/effectSizeFilters/RatioProportions.py` | Divides by zero when a feature has zero abundance in both groups, which aborts filtering. | Report a ratio of 1 for a feature absent from both groups. |
| 3 | done | `plugins/multiGroups/postHoc/GamesHowell.py` | Crashes (NaN degrees of freedom) when a group has a single sample. | Comparisons involving such a group report no significant difference, undefined confidence limits and an explanatory note; the other comparisons are unaffected. |
| 4 | done | `plugins/samples/statisticalTests/Bootstrap.py` | The "parent has a count of zero" note is computed and then dropped. | Return the note. |
| 5 | done | `plugins/groups/statisticalTests/White.py` | Prints a leftover debug line (`blah`) on every run. | Removed. |
| 6 | done | `plugins/common/multipleComparisonCorrections/StoreyFDR.py`, resampling tests | Random numbers are unseeded, so results differ between runs. | A seeded generator with a user-visible seed; the same seed gives the same result. |
| 7 | done | `metagenomics/ProfileTree.py`, `createMultiGroupProfile` | Stores a feature under its untruncated name while looking it up by the truncated name (the ` - #` suffix), unlike the two-sample and two-group profiles. | Use the truncated name consistently. |
| 8 | done | `metagenomics/fileIO/StampIO.py` | A profile with two samples of the same name was accepted and their counts silently added together. | Rejected with a message naming the sample. |
| 9 | done | `metagenomics/fileIO/StampIO.py`, `MetadataIO.py` | Several malformed inputs crashed and were reported only as "Unknown parsing error": an empty file, a file with only a header, an empty cell in the first data row, a metadata row longer than the header. | Each gives a specific message, or is tolerated (short metadata rows are padded with empty values, extra cells ignored). |
| 10 | done | `metagenomics/fileIO/MetadataIO.py` | A sample listed twice was reported as an "unknown sample", and that warning was overwritten if any sample lacked metadata. | A separate warning for repeated samples; all warnings are returned. |
| 11 | done | `GUI/createProfileMgRastDlg.py` | A blank line in the middle of an MG-RAST table added a sample with an empty name. | Blank lines are skipped. |
| 12 | done | `GUI/createProfileCoMetDlg.py` | A CoMet entry with an empty description (as in the GO example, `GO:0018346 (): 5`) produced a row with an empty name, giving a profile STAMP could not load. | Such entries are named by their identifier. |
| 13 | done | `GUI/createProfileMothurDlg.py` | Sequence ids in a names file were not stripped of spaces; blank lines and ids missing from the groups or names file crashed the conversion. | Ids are stripped, blank lines skipped, and missing ids reported by name. |
| 14 | done | `GUI/createProfileBiomDlg.py` | Observation metadata given as a single string (QIIME's `k__Bacteria; p__...` taxonomy) was split into single characters; nested lists crashed. | A string is split on `;`. Nested lists (several pathways per gene) give an explanatory error. |
| 15 | done | `metagenomics/fileIO/COG_IO.py` | An unknown COG identifier or a blank line crashed the conversion. | Unknown identifiers are reported by name; blank lines are skipped. |
| 16 | done | `plugins/groups/statisticalTests/Ttest.py` | When both groups had zero variance, the division-by-zero was caught by the wrong handler: the feature got p = 1, an effect size of 0 and the note "float division by zero", even if the group means differed. The intended handler was unreachable. | Handled as Welch's test does: p = 0 if the means differ (1 otherwise), the true difference between means, and the note "degenerate case: variance of both groups is zero". |
| 17 | done | `plugins/multiGroups/postHoc/WelchUncorrected.py` | Effect sizes and confidence limits were 10,000 times too large, because percentages were converted to percentages twice. p-values were unaffected. | Reported in percent, like the other post-hoc tests. |
| 18 | done | `plugins/samples/statisticalTests/DiffBetweenProp.py`, `plugins/groups/statisticalTests/White.py` | A parent category with no sequences in one sample caused a division by zero that aborted the whole analysis. | The feature is reported as a degenerate case (difference-between-proportions test), or its proportion is taken as zero (White's test). |
| 19 | done | `plugins/groups/statisticalTests/White.py` | A permutation that reproduces the observed split of values gives the observed t statistic again, but rounding could make it count as "more extreme", inflating p-values of features with many tied values. | Statistics within rounding error of the observed one are not counted as more extreme. |

## Behaviour changes

| # | Status | Change |
|---|---|---|
| 1 | done | User-supplied plugins are not supported. Tests, corrections, effect sizes and plots are built in. |
| 2 | done | PS and EPS export are dropped. Plots export as PNG, SVG and PDF. |
| 3 | done | Holm-Bonferroni becomes a built-in correction returning standard adjusted p-values. The Python version (in `additional/`, never loaded by the GUI) only marked non-significant features with infinity. |
| 4 | done | Barnard's exact test (in `additional/`, never loaded by the GUI) is not ported. |
| 5 | done | Results-table row order is fully defined (p-value, then feature name). In Python, the order of tied p-values depended on dictionary ordering. |
| 6 | done | Benjamini-Hochberg uses the standard adjustment (as R's `p.adjust(method = "BH")`): `p * n / rank` followed by a running minimum from the largest p-value down, capped at 1. Python omitted the running minimum, so tied p-values could get different corrected values and corrected values were not monotonic. Corrected p-values can therefore be smaller than in STAMP v2, never larger. The `corr:Benjamini-Hochberg FDR` columns in `tests/golden` hold the old values and are an expected difference. |
| 7 | done | Features, and the rows of converted profiles, are kept in order of first appearance in the input. Python used dictionary order, which was arbitrary. |
| 8 | done | Counts written by the BIOM and CoMet converters use the shortest exact decimal form instead of Python's 12 significant digits. |
| 9 | done | BIOM 2.x (HDF5) tables are detected and rejected with advice to convert to JSON. |
| 10 | done | The COG lookup table is a compact extract (`crates/stamp-core/data/cog_codes.tsv`) of `whog.txt`, embedded in the program. |
| 11 | done | Bonferroni-corrected p-values are capped at 1. Python reported `p * n` even when it exceeded 1. |
| 12 | done | Resampling methods (bootstrap, permutation, White's test, Storey's FDR) draw from a ChaCha8 generator seeded by the user, so results are reproducible but are not the same random draws as any Python run. |
| 13 | done | The number of significant features is counted as corrected p-value <= alpha for every correction. Python used `<` for Benjamini-Hochberg and `<=` elsewhere. |
| 14 | done | A command-line interface (`stamp-cli`) is new; STAMP v2 was graphical only. |
| 15 | done | Asking for a one-sided p-value from a test that has none (chi-square and G-tests) is an error. STAMP v2 reported an infinite p-value. |
| 16 | done | In saved tables, whole numbers such as counts are written without a decimal point (`5`, not `5.0`), and other numbers in their shortest exact form. Column headings are unchanged. |
| 17 | done | Plots are drawn by STAMP itself rather than by matplotlib, following the same layout, font (DejaVu Sans, bundled) and colours. Small details differ, such as the exact choice of axis ticks. |
| 18 | done | In the extended error bar plot, sorting by effect size treats ratios symmetrically (0.5 and 2 are the same size of effect), and a confidence interval that runs past fixed axis limits is clipped at the axis without an end cap. |
| 19 | done | The heatmap clusters features and samples on the Euclidean distances between their abundance vectors. STAMP v2 passed the distance matrix to SciPy in a form that made it cluster the rows of the distance matrix itself, which gives a different tree. |
| 20 | done | The PCA plot uses a standard principal component analysis (matching NumPy's SVD). The sign of each component, which is arbitrary, is chosen so that the sample furthest from the origin is positive. |
| 21 | done | Every plot type can be drawn from the command line with `--plot` and `--plot-type`. |
| 22 | done | The graphical application is rebuilt with egui: one window with the profile and groups on the left, the plot in the middle and statistics and filtering on the right. Analyses run in the background with a progress bar and can be cancelled. |
| 23 | done | In the graphical application, groups with a single sample start out switched off for multiple-group analyses, because they make ANOVA undefined for every feature. They can be switched on in the group list. |
| 24 | done | Changing only the multiple-test correction no longer re-runs the statistical tests. |
| 25 | done | STAMP also runs in a web browser (WebAssembly), entirely on the user's computer. Files come from the browser's file picker and are saved as downloads. |
| 26 | done | Plots are saved through File > Save plot as > PNG, SVG or PDF, rather than by choosing the format in the save dialog. A built-in example dataset can be opened from the File menu. |
| 27 | done | In the graphical application: the plot is centred in its area; Settings has a light/dark mode switch; a Reset button returns the zoom to its default; each group's colour can be changed with a colour picker in the group list; and the results table window has a Download button. |
| 28 | done | A group keeps one colour in every plot, taken from its position among all groups of the metadata field (or chosen by the user). The second group of a two-group comparison is no longer always orange. |
| 29 | done | Layout of the graphical application: the analysis type (Multiple groups, Two groups, Two samples) is chosen at the top of the right-hand panel and sets what the Statistics and Filtering sections show; Profile, Groups, Statistics and Filtering are collapsible sections; the plot type, size, sort order, legend and zoom controls sit in a bar beneath the plot; and the View menu shows or hides the side panels, the feature table and the results table. |
| 30 | done | The feature table beside the single-feature plots lists each feature with its effect size, p-value, corrected p-value and note, and can be limited to the active features. Clicking a row shows that feature. |
| 31 | done | In light mode the text of active controls is black. |
| 32 | done | The plot type, a "Configure plot" button, the zoom and the highlight choices sit beneath the plot. "Configure plot" opens a window with the settings of the plot being shown; changes apply as they are made rather than on OK, and "Restore defaults" undoes them. One set of settings is kept for each kind of plot, shared between the analysis types where the plot is the same (the extended error bar, scatter, PCA and heatmap plots). |
| 33 | done | The feature table is shown beside every plot, not only those of a single feature, and can be hidden from the View menu. |
| 34 | done | The metadata table (View > Metadata table) runs along the bottom of the window. As in v2, samples are switched off by unticking them, with "Select all" and "Remove all", or by a rule that adds or removes all samples whose value of a field is less than, equal to or greater than a chosen value. The table cannot be sorted by a column, and switching a sample off does not remove it from the sample lists of the two-sample analysis. |
| 35 | done | Highlighting: a hierarchical level and a name at that level are chosen beneath the plot, and the features under that name are drawn in red in the extended error bar, scatter, two-sample bar and profile bar plots. One choice is shared by the three analysis types. |
| 36 | done | Legend position: the bar, profile bar, sequence histogram and PCA plots offer Best, Upper right, Centre right, Lower right, Upper left, Centre left, Lower left and None. "Best" takes the first of those places (in matplotlib's order of preference) that covers the fewest drawn shapes; it is a simpler rule than matplotlib's, so it can choose a different corner than v2 did. The extended error bar plot keeps its own choices (around the whole figure), and the heatmap's legend can only be shown or hidden. |

## Kept as is

* Tukey-Kramer and Games-Howell report binned p-values from the critical-value
  tables. Exact studentized-range p-values are a later enhancement.
* Effect-size filters combine as in Python: a feature is removed only if it
  fails **both** filters under "AND", and if it fails **either** under "OR".
  The filters are worded as removal rules ("filter out features with an effect
  size less than..."), so this is the intended logic, not a bug.

## Numerical agreement with STAMP v2

Deterministic statistics are checked against 108 reference tables generated
by the Python code (`tests/golden`). Agreement is to at least eight
significant digits, with these exceptions, which come from limits of the
numerical methods rather than from different formulas:

* **Confidence limits from t and F quantiles** (t-test, Welch, Scheffè) agree
  to about nine significant digits of the interval's width. SciPy 1.2's
  quantile functions are accurate to roughly 1e-9; this port's invert the
  distribution function to 1e-14.
* **Fisher's exact test** agrees to about seven significant digits. Both
  implementations exponentiate differences of very large log-factorials.
* **The hypergeometric test** computes one tail as one minus a sum of
  thousands of terms, so p-values below about 1e-8 are rounding noise in both
  implementations and need not agree.
* Tests that compute `1 - cdf` (chi-square, G-test, difference between
  proportions) cannot report p-values below about 1e-16, as in Python.

## Not yet ported

* Signed installers. The desktop applications are distributed as plain
  archives, unsigned (see `docs/RELEASING.md`).
* In the browser build, running analyses in a web worker so that long
  resampling tests do not hold the page while they run.
* In the graphical application:
  * customising sample and axis colours (group colours can be changed);
  * the "select features" dialog;
  * choices in the file converters (BIOM metadata field, MG-RAST level
    headings, treatment of multi-category COGs), which the command line has;
  * sending a plot to its own window.
* Plot options that are not carried over yet: the heatmap's colouring of
  clusters below a distance threshold and eight of its fourteen colour maps
  (Blues, Reds, Grayscale, Orange to red, Yellow to orange to red and Blue to
  white to red are available); the PCA plot's "fixed pixels per unit
  distance"; and choosing sequence counts instead of proportions in the
  heatmap plots (the group bar and box plots offer it).

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
