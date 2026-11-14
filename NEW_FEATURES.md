# New features in STAMP v3

Features of the Rust port (STAMP v3) that STAMP v2, the Python version, did
not have. Changes to how an existing v2 feature behaves, and v2 bugs that
were fixed, are recorded in `docs/CHANGES_FROM_PYTHON.md` instead; the
numbers in brackets refer to its "Behaviour changes" table.

Add an entry here whenever a feature is added that v2 did not have.

## Running STAMP

- **In the browser.** STAMP runs as a web page (WebAssembly), entirely on the
  user's computer: files are read with the browser's file picker and saved as
  downloads, and nothing is uploaded [25]. Long tests are carried out in
  short pieces so the page keeps responding [51].
- **One program per platform, no Python.** A single executable for Linux,
  Windows and macOS with no interpreter or libraries to install.
- **Command line.** `stamp-cli` runs every analysis, writes the result
  tables with v2's column headings, draws every plot, and converts files
  from other tools, so analyses can be scripted.
- **Built-in example data.** File > Open example data offers two datasets
  without any files: "Enterotypes" (proportions of phyla and genera in gut
  samples) and "Coal seam gas" (sequence counts with a full taxonomy, which
  suits the rarefaction curves) [26].
- **Repeatable resampling.** Bootstrap and permutation tests use a seed that
  can be set in the preferences, so results can be reproduced exactly.

## Files

- **BIOM 2.x (HDF5) tables**, as well as the JSON format v2 read, opened
  directly from File > Load profile and on the command line [9, 48].
- **Profiles in other formats open directly.** BIOM tables and mothur files
  are loaded from File > Load profile without first being converted and
  saved [48].
- **PDF export** of plots, alongside PNG and SVG.

## Statistics

- **Mann-Whitney U test** (Wilcoxon rank-sum test) for two groups: a
  rank-based test that does not assume the proportions are normally
  distributed. The p-value is exact when there are no ties and either
  group has eight samples or fewer, and otherwise from the normal
  approximation with corrections for ties and continuity; it agrees with
  SciPy. The confidence interval for the difference in mean proportions is
  a bootstrap interval, as for White's test.
- **Dunn's test** as a post-hoc test, the rank-based companion of the
  Kruskal-Wallis test: every pair of groups compared by their mean ranks,
  with the Bonferroni adjustment. Its confidence intervals are Welch
  intervals at the adjusted level, since the test itself gives none.

- **Correlation with a numeric metadata field**, as a fourth kind of
  analysis beside multiple groups, two groups and two samples. Every
  feature's proportion is correlated with a number known for each sample
  (a depth, an age, a concentration) by Spearman's rank correlation or
  Pearson's r, with a two-sided p-value and the usual multiple-test
  corrections; the values agree with SciPy. Samples with no number in the
  field are left out, and the status bar says how many. Features can be
  filtered by corrected p-value and by the size of the correlation. On the
  command line: `stamp-cli correlation --field FIELD`, with `--method
  spearman|pearson`, `--correction`, `--significance` and
  `--min-correlation`.
- **Paired tests for two groups**: the **paired t-test** and the
  **Wilcoxon signed-rank test**, for designs where each sample of one
  group belongs with a sample of the other (the same subject before and
  after, say). A "Pair field" names the metadata field that says which
  samples belong together; a value shared by exactly one sample of each
  group makes a pair, and samples without such a partner are left out of
  the analysis and its plots, with a count of both shown. The signed-rank
  p-value is exact for up to 50 pairs (tied differences included) and from
  the normal approximation beyond that, as in SciPy; its confidence
  interval is a bootstrap over the pairs. On the command line: `--test
  paired-t|wilcoxon --pair-field FIELD`.
- **Centred log-ratio transform** as an option ("Transform") before the
  two-group and multiple-group tests. Proportions sum to one, so a rise in
  one feature lowers all the others; with the transform, each p-value is
  computed from the logarithm of the feature relative to the geometric
  mean of all the features of its sample (zeros replaced as for the
  Aitchison distance). Only the p-values change: effect sizes, confidence
  intervals, group means, the post-hoc tests and the plots stay in
  proportions. It does not apply to White's test, which works on counts.
  On the command line: `--transform clr`.
- **Tests on the community as a whole**, for two or more groups, on the
  distances between samples: **PERMANOVA** (do the groups differ?),
  **PERMDISP** (do the groups differ in how spread out their samples are,
  which can by itself make PERMANOVA significant?) and **ANOSIM** (the
  older rank-based alternative). Each p-value comes from permuting the
  group labels (999 times unless changed), with the random numbers seeded
  so that a result can be repeated. The statistics agree with scikit-bio.
  They are written above the PCoA plot, and printed by the command line
  when it draws one.
- **Distances between samples**: Bray-Curtis (on proportions), Jaccard (on
  which features are present) and Aitchison (Euclidean distance after the
  centred log-ratio transform, with zeros replaced multiplicatively).
- **Alpha diversity** of each sample: observed features, the Shannon index
  (natural logarithm), the Simpson index (1 - sum of squared proportions)
  and the Chao1 estimate, compared between groups with the Kruskal-Wallis
  H-test.
- **A stated normalisation** for these analyses: the proportions of each
  sample as they are, or every sample rarefied (randomly subsampled,
  seeded) to the depth of the smallest sample, which the plot then states.
  Rarefying needs counts of sequences, as does Chao1; a profile of
  fractional values is refused with a message. As for rarefaction curves,
  unclassified categories are not counted as features.

- **Computed post-hoc p-values.** Tukey-Kramer and Games-Howell report a
  p-value rather than a bound such as `< 0.05`, and confidence intervals at
  any coverage rather than five tabulated ones [47].

## Window

- **Light and dark themes** [27].
- **A colour for each group**, kept across every plot and chosen with a
  colour picker in the group list [27, 28].
- **Sortable tables.** The feature, results and metadata tables sort by any
  column [38].
- **Feature table beside every plot**, with each feature's effect size and
  p-values, not only beside the single-feature plots [30, 33].
- **Hover tooltips on plots**, where v2 showed a message box on a click.
- **Zoom** of the plot on screen, independent of the size it is saved at.
- **Live plot settings.** Changes in "Configure plot" show at once, and each
  plot's settings can be returned to their defaults [32].
- **Menu icons** (Phosphor Icons) [49].
- **A "Correlation" tab** beside the three kinds of analysis of v2, with
  its own statistics, filters, feature table, results table and plots.

## Plots

- **Volcano plot**, in all three analysis types: the effect size of every
  feature against -log10 of its p-value. Features past the chosen
  significance level (and, optionally, a minimum effect size) are coloured
  by the sample or group they are more abundant in; the rest are grey.
  Dashed lines mark the thresholds. Corrected or uncorrected p-values can
  be plotted, features can be highlighted, and clicking a point selects
  the feature. On the command line: `--plot-type volcano`.
- **Correlation scatter plot** (Correlation tab): the proportion of the
  selected feature against the numeric field, one point per sample
  coloured by the group field, with the least-squares line and the
  correlation, its p-value and the number of samples written above. The
  line and the text can be switched off. On the command line:
  `stamp-cli correlation --plot FILE` (`--plot-feature`, `--group-field`).
  The **volcano plot** and **p-value histogram** are also available for
  correlations; the volcano plot then shows the correlation of every
  feature against its p-value, coloured by its direction.
- **Alpha diversity box plot**, for two groups and multiple groups: a box
  for each group of the chosen diversity measure of its samples, with the
  Kruskal-Wallis p-value. On the command line:
  `--plot-type alpha-diversity`, with
  `--alpha-metric observed|shannon|simpson|chao1` and `--rarefy`.
- **PCoA plot**, for two groups and multiple groups: the samples laid out
  by principal coordinates analysis of a chosen distance, beside the
  existing PCA plot and with the same options, with the outcome of
  PERMANOVA, ANOSIM and PERMDISP above it. On the command line:
  `--plot-type pcoa`, with `--distance bray-curtis|jaccard|aitchison`.
- **Distance box plot**, for two groups and multiple groups: the distances
  between samples of the same group, group by group, beside the distances
  between samples of different groups. On the command line:
  `--plot-type distance-box`.
- **Composition bar plot**, in all three analysis types: a stacked bar for
  each sample, or the mean of each group, showing the most abundant
  features in their own colours and the rest as "Other". A strip beneath
  each bar shows its group. The number of features shown is an option. On
  the command line: `--plot-type composition`.

- **Rarefaction curves.** A new plot in all three analysis types: for each
  sample, the number of features expected against the number of its
  sequences read, so that samples of different depth can be compared and it
  can be seen whether sampling has levelled off.
  - One curve per sample, coloured by group (by sample for two samples),
    with the sample names at the ends of the curves.
  - The expectation is exact (the hypergeometric formula of Hurlbert 1971),
    not an average of random subsamples, so the curves are smooth and the
    same every time.
  - Features are the categories at the profile level. Unclassified
    categories are not counted as features, whatever the "Unclassified"
    setting, but their sequences are part of the sample and are drawn like
    any others.
  - For groups, "Curves for" offers two further kinds, one curve per group:
    - pooled sequences: the counts of a group's samples added up and
      rarefied as one sample, to see whether the group as a whole has been
      sequenced deeply enough;
    - by number of samples: the features expected in one, two, three ...
      of the group's samples, averaged exactly over every choice of them
      (a species accumulation curve), to see whether more samples would
      find more features. Its estimate of the total is Chao2.
  - Options: the number of points computed; ending every curve at the depth
    of the smallest sample (for curves by number of samples, rarefying
    every sample to that depth first, so that deep samples do not count
    for more); the Chao estimate as a dashed line (Chao1 against
    sequences, Chao2 against samples; both in the bias-corrected form,
    which is defined when no feature is seen exactly twice); labels at the curve ends; and the
    legend position.
  - A profile must hold counts of sequences. One with fractional values
    (proportions, normalised counts) is refused with a message.
  - On the command line: `--plot-type rarefaction`, with
    `--rarefaction-curves samples|pooled|accumulated`.
  - The idea follows the rarefaction plot of the GenomicaMicrob fork of
    STAMP; the implementation is independent of it.

- **"Best" legend position for the PCA plot and the heatmap**, and legends
  on the left and right sides as well as in the corners [36, 43].
- **Any confidence level** for the post-hoc plot [47].
