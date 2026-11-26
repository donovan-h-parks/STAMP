# New features in STAMP v3

Features of the Rust port (STAMP v3) that STAMP v2, the Python version, did
not have. Changes to how an existing v2 feature behaves, and v2 bugs that
were fixed, are recorded in `docs/CHANGES_FROM_PYTHON.md` instead; the
numbers in brackets refer to its "Behaviour changes" table.

Add an entry here whenever a feature is added that v2 did not have.

## Running STAMP

- **Movable frames** (`crates/stamp-gui/src/layout.rs`, `egui_tiles`).
  Profile and groups, Statistics (v2's Properties), the Feature table, the
  Metadata table, the Results table, Configure plot and the Assistant are
  frames that can be dragged by their tabs to any side of the plot or of
  each other, or tabbed together; the plot stays in the middle. Configure
  plot and the Results table, windows in v2, are frames. A closed frame's
  room goes to the plot; the View menu brings it back where it was, or to
  the front when it is behind another tab. The arrangement is remembered
  between runs; View > Reset layout restores the default, and Settings >
  Preferences > Restore window position defaults also puts floating windows
  back.
- **Asking an AI assistant** (Assistant menu, `crates/stamp-gui/src/assistant/`,
  `docs/site/assistant.rst`). "Copy description for AI" copies the data,
  the analysis settings, a description of the plot on screen (its text and
  every item with its tooltip values) and the top results, to paste into
  any chat assistant; "Copy plot image" copies the plot. On the desktop,
  "Open Claude Code" connects Anthropic's Claude Code (with the user's own
  Claude subscription) to the window through a Model Context Protocol
  server on 127.0.0.1 with a per-session token: it can read the session,
  results, samples and plot, show any plot, change plot options and the
  analysis, and undo; STAMP answers changes once the analysis and plot have
  caught up. Every change can be undone from the status bar; Assistant
  activity lists every request. STAMP installs Claude Code with
  Anthropic's installer if asked. **Claude
  Code runs inside STAMP**, in the assistant panel on the right of the
  window (View > Assistant): a terminal built on
  `alacritty_terminal` (ConPTY on Windows) drawn with egui
  (`assistant/terminal.rs`), with keys, paste, Ctrl+C, mouse selection,
  a scroll bar, and the mouse wheel scrolling the scrollback or, for
  Claude Code's full-screen view, sent to it as mouse reports; the
  computer's own terminal remains available. Its dialog has "Don't show
  this again" (Preferences turns it back on). The tools include
  `set_group_colors` for the colour of any group, `switch_samples` to
  leave samples out by name or metadata value ("remove the samples from
  the QS company"), and `set_plot_fonts` for the text sizes and font of
  every plot. **OpenCode** (opencode.ai) runs in the same panel
  (Assistant > Open OpenCode), connected to the same server, for people
  with a ChatGPT plan, GitHub Copilot or another provider rather than a
  Claude plan: STAMP writes `opencode.json` (the server and token, and
  ask-first rules for commands, edits and web fetches) and `AGENTS.md` in
  its assistant folder, in OpenCode 2's form or OpenCode 1's after asking
  `opencode --version`, and installs OpenCode 2 if asked. **Google
  Antigravity** (Google's Antigravity CLI, `agy`) runs there too
  (Assistant > Open Google Antigravity), for a Google account, free or with
  a Google AI Pro or Ultra plan: STAMP writes `.agents/mcp_config.json`
  (the server and token, read as the folder's MCP servers), settings for
  the run given with `--settings` that allow STAMP's tools, and
  `AGENTS.md`, and installs it with Google's installer if asked. **Several
  assistants at once**: each one opened gets a tab of its own in the
  assistant panel, all connected to the same session; opening one that is
  running shows its tab. Plan: `docs/ASSISTANT_PLAN.md`.
- **The assistant can open data** (tool `load_data`,
  `crates/stamp-gui/src/assistant/tools.rs`): asked for other data, it
  opens a profile, a metadata file or both by their paths, in any format
  File > Load profile reads, including several SingleM or Sandpiper files
  combined, a BIOM table with the observation metadata field that gives
  its hierarchy, and a mothur taxonomy file with its groups and names
  files. Every file is read before anything changes, and what cannot be
  loaded is explained without changing the session.
- **Custom plots drawn by the assistant** (tool `show_custom_plot`,
  `crates/stamp-plot/src/custom/`, `docs/site/assistant.rst`). When none of
  STAMP's plots shows what the user asks for, the assistant describes one
  in a subset of Vega-Lite and STAMP draws it in the middle of the window
  as a figure like its own plots: STAMP's axes, fonts and group colours,
  tooltips, saving as PNG, SVG or PDF, and a description for
  `get_current_plot`. The description can use STAMP's data by name
  ("profile", "samples", "results"), taken when the plot is drawn. Marks
  bar, point, circle, square, tick, line, area, rect, rule, text, boxplot
  and errorbar; stacked, grouped (xOffset) and normalised bars and areas;
  linear, log and sqrt scales; colour schemes; layers; transforms filter,
  calculate (with Vega-Lite's expressions), aggregate, joinaggregate,
  window, fold, lookup and bin. What cannot be drawn is refused with the
  reason and changes nothing; what is ignored is reported. Every plot the
  assistant draws is kept and listed by its name under "Drawn by the
  assistant" in the plot selector. The name is required (`name`) and must
  be the assistant's own: one in use by another of its plots, or by one of
  STAMP's plots, is refused. A refinement names the plot it changes
  (`plot`) and replaces it in place; `show_plot` shows a plot drawn earlier
  by its name. Undone
  like any change of the assistant's.
- **One program per platform, no Python.** A single executable for Linux,
  Windows and macOS with no interpreter or libraries to install.
- **Command line.** `stamp-cli` runs every analysis, writes the result
  tables with v2's column headings, draws every plot, and converts files
  from other tools, so analyses can be scripted.
- **Built-in example data.** File > Open example data offers datasets
  without any files: "Enterotypes" (proportions of phyla and genera in gut
  samples), "Coal seam gas" (sequence counts with a full taxonomy, which
  suits the rarefaction curves) [26], and "Glacier-fed streams": SingleM
  coverage, domain to species, of 158 sediment metagenomes of glacier-fed
  streams in nine mountain ranges (Michoud et al. 2025), an upstream and a
  downstream reach of each stream, with the measurements made at each
  (`examples/GlacierFedStreams`). It opens in the two-group analysis with
  the downstream reaches compared with the upstream ones by the Wilcoxon
  signed-rank test, paired by glacier. The documentation has a worked
  example on it (`docs/site/example_glacier_streams.rst`): map, PCoA and
  group heatmap of the mountain ranges, the paired test with the paired
  box plot, correlation with pH, co-occurrence.
- **Online documentation** (`docs/site`, published on GitHub Pages), replacing v2's PDF user's guide: installation, a quick start on
  the example data, every control of the window and command, and pages
  that explain each test, post-hoc test, correction and plot for
  biologists, with worked examples and what to look for.
- **Progress of slow work, and cancelling it.** Analyses, plots that
  take a while to compute (PCoA with its community tests, NMDS, the
  distance box and Mantel plots, ANCOM), reading large profiles and building
  large results tables run in the background (on another thread) and show what they are doing with
  a progress bar ("PERMANOVA 412/999") or a spinner, in the status bar and,
  for plots and loading, over the plot area, with a Cancel button; the
  window stays usable meanwhile. `stamp-cli` shows progress bars and
  spinners on standard error when it is a terminal (`--quiet` turns them
  off), and writes results tables row by row instead of building them in
  memory.
- **Repeatable resampling.** Bootstrap and permutation tests use a seed that
  can be set in the preferences, so results can be reproduced exactly.

## Files

- **BIOM 2.x (HDF5) tables**, as well as the JSON format v2 read, opened
  directly from File > Load profile and on the command line [9, 48].
- **Profiles in other formats open directly.** BIOM tables and mothur files
  are loaded from File > Load profile without first being converted and
  saved [48].
- **PDF export** of plots, alongside PNG and SVG. The
  PDF is written directly from the plot, with its text as selectable text
  in the embedded plot font.
- **SingleM and Sandpiper profiles.** SingleM condensed taxonomic profiles
  (`singlem pipe -p`, `singlem summarise`, also the variant with extras;
  many samples per file) and Sandpiper condensed profiles (the JSON of
  `sandpiper.qut.edu.au/api/condensed/<run>`, GTDB or GlobDB, one sample
  per file) become profiles of seven ranks, Domain to Species, with
  coverages as values. A taxon's own coverage (SingleM's coverage does not
  include descendants) that stops above species goes into a row padded with
  `unclassified <prefixed taxon>` (`unclassified f__Oscillospiraceae`);
  named taxa lose their rank prefixes; a name with two parents at one rank
  becomes `name (parent)`. Opened from File > Load profile file (several
  files of either kind chosen together are combined) and `stamp-cli
  convert singlem|sandpiper`, and read directly by every `stamp-cli`
  analysis (`--profile`). 1,307 Sandpiper runs (12,276 features) convert in
  about two seconds.
- **Load profile directory.** File > Load profile directory loads the
  SingleM or Sandpiper profiles directly in a directory (typically one
  `singlem pipe -p` output per sample) as one profile, skipping other files
  (only their first 64 KB are read) and naming them in a message. `stamp-cli --profile` and
  `stamp-cli convert singlem|sandpiper` take a directory as well.
- **mOTUs, MetaPhlAn, Kraken, Bracken and HUMAnN outputs**, read directly
  and recognised by their contents: mOTUs 2, 3 and 4 profiles and merged
  tables (with or without full taxonomy, counts or relative abundances);
  MetaPhlAn 3 and 4 profiles (relative abundance, or estimated reads with
  `rel_ab_w_read_stats`), merged tables and GTDB profiles, with the SGB as
  a level; Kraken 1 and 2 reports (also with minimizer columns) and
  MPA-style reports, with a Strain level when reads were assigned below
  species; Bracken tables, combined tables and reports; HUMAnN 3 gene
  families (gene family, genus, species), pathway abundance and pathway
  coverage, joined or not. Clade values become own values, unplaced reads
  `unclassified Root`, missing NCBI ranks `<parent> (no <rank>)`. From a
  file, several files of one tool, or a directory (the window asks which
  kind to load when a directory holds several, such as HUMAnN's three
  tables; `--profile-format` on the command line), and
  `stamp-cli convert profiles`. Design notes and supported versions:
  `docs/PROFILER_FORMATS.md`.
- **Example inputs in every format read**, in
  `examples/input_file_formats`: one made-up community of ten gut
  metagenomes (healthy and IBD) written as SingleM, Sandpiper, mOTUs,
  MetaPhlAn, Kraken 2, Bracken, HUMAnN, BIOM (JSON and HDF5), mothur and
  IMG/M COG files, with metadata, by `make_examples.py`. Beside them, in
  `<tool>/stool_SRR17283923/`, real output of every profiler for one human
  stool metagenome subsampled to 1,000,000 read pairs, read by the tests in
  `crates/stamp-core/tests/stool_sample.rs`. All of them are kept
  gzip-compressed.
- **PICRUSt2 tables**, read directly and recognised by their contents
  (`import/picrust2.rs`): predicted functions (EC, KO, COG, Pfam, TIGRFAM;
  one level named after the kind), MetaCyc pathway abundance and coverage,
  the contributions of each sequence (`--stratified`, long or
  `--wide_table`; levels function and Sequence), and the tables
  `add_descriptions.py` writes (`<id>: <description>`). From a file or a
  directory as PICRUSt2 writes it (the window asks whether to load the
  functions or their contributions), with
  `--profile-format picrust2-...` on the command line. Real PICRUSt2 2.6.4
  output for the tutorial's 24 mouse stool samples, with metadata, is in
  `examples/input_file_formats/PICRUSt2`, read by
  `crates/stamp-core/tests/picrust2_chemerin.rs`. (STAMP v2 read PICRUSt 1
  output only as BIOM tables.)
- **Gzip-compressed files** (`stamp_core::text::decompress`): every file
  STAMP reads, profiles, metadata, profilers' outputs and the other formats,
  can be gzip-compressed, in the window and on the command line,
  and in directories of profiles. A compressed file is told by its contents,
  not its name; `.gz` is left out of sample names taken from file names. A
  compressed file cut short is reported as such. (STAMP v2 read only plain
  text, and BIOM tables in this port already could be compressed.)

## Statistics

- **Welch's ANOVA** (`MultiGroupTest::WelchAnova`, "Welch's ANOVA",
  `stats/multi_group.rs`): the multiple-group test of means that does not
  assume equal spread, each group weighted by its size over its variance
  (Welch 1951). p-values agree with R's `oneway.test(var.equal = FALSE)`;
  with two groups it equals Welch's t-test. A group with no variance (a
  feature with one value, usually zero, in all its samples) is taken at
  the limit of infinite weight, which R cannot compute; constant groups
  that differ give p = 0. Choosing it in the Statistics panel switches the
  post-hoc test from Tukey-Kramer to Games-Howell. It is also offered for
  ANCOM's log-ratio tests. Command line: `multi-groups --test welch-anova`
  (and `ancom --test welch-anova`).
- **Brown-Forsythe test of equal spread** (`multi_group::brown_forsythe`,
  `results::SpreadSummary`): Levene's test on distances from the group
  medians, run on every feature of every multiple-group analysis, on the
  values the test sees (centred log-ratios with that transform). p-values
  agree with R's `DescTools::LeveneTest(center = median)` (as
  `car::leveneTest`); NaN when a group has fewer than two samples. Each
  feature's p-value is a new last column of the multiple-group results
  table, *Equal spread p-value (Brown-Forsythe)*, after STAMP v2's
  columns. The Statistics panel and `stamp-cli multi-groups` (on stderr)
  report how many features differ in spread (p < 0.05), and caution when
  more than 10% do (twice chance) and the test is ANOVA. Not shown for the
  test of presence or the Friedman test. The analysis wizard uses the same
  count to choose Welch's ANOVA (see the wizard entry).
- **Friedman test** for matched groups in the multiple-group analysis: the
  same subjects (patients, sites) sampled once in every group. A "Subject
  field" of the metadata names the subject of each sample
  (`Metadata::matched_groups`); subjects without exactly one sample in
  every chosen group are left out and the number matched is shown. The
  p-value is the tie-corrected chi-square approximation and agrees with
  SciPy. Its post-hoc test, the only one offered with it, is **Wilcoxon
  signed-rank (Holm)**: the signed-rank test for each pair of groups with
  Holm's correction, and paired t intervals at the level a Bonferroni
  adjustment gives each pair. Command line: `multi-groups --test friedman
  --pair-field FIELD`, `post-hoc --test wilcoxon --pair-field FIELD`.
- **Tests of presence or absence** (`stats/presence.rs`): is a feature
  found in more of the samples of one group, whatever its abundance where
  it is found? **Fisher's exact test (presence/absence)** for two groups,
  **McNemar's test (presence/absence)**, exact, for paired samples, and the
  **Chi-square test (presence/absence)** for multiple groups, which notes
  features with expected counts below five. They agree with SciPy. The
  effect size and interval of the two-group tests stay the difference in
  mean proportions (bootstrap); the centred log-ratio transform does not
  apply. Command line: `--test fisher-presence`, `--test mcnemar
  --pair-field FIELD`, `--test chi-square-presence`.
- **Pairwise PERMANOVA**: with three or more groups, the PCoA plot's
  community tests are followed by PERMANOVA between every two groups, on
  their samples alone, with Holm-corrected p-values
  (`community::PairwiseRun`, resumable like the other tests; its random
  numbers are drawn after theirs, so their results are unchanged). Up to
  six pairs are listed above the plot, more are counted; the command line
  prints every pair.
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

- **ANCOM** (analysis of composition of microbiomes), for two or more
  groups: for every pair of features, the logarithm of their ratio is
  compared between the groups (ANOVA, or Kruskal-Wallis), each feature's
  p-values are adjusted by the Holm-Bonferroni method, and W counts the
  features it differs from. Features whose W reaches the cut-off are
  called different; the cut-off is chosen as scikit-bio chooses it, and W
  and the calls agree with scikit-bio 0.7.4. Zeros are replaced as for the
  centred log-ratio transform. On the command line: `stamp-cli ancom`,
  which writes a table of W, the call and the effect on the log-ratio
  scale for every feature (`--test`, `--alpha`, `--group`).
- **Mantel test** (correlation tab): are the distances between samples'
  communities related to how far apart the samples are in a numeric
  metadata field? The statistic is the Spearman or Pearson correlation
  chosen for the analysis; the two-sided p-value comes from permuting the
  samples. The statistic agrees with scikit-bio.
- **NMDS** (non-metric multidimensional scaling) of the samples by
  Bray-Curtis, Jaccard or Aitchison distance, in two dimensions, with
  Kruskal's stress. It starts from the PCoA layout, so it is the same
  every time.
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

- **Analysis wizard** (Help > Analysis wizard, and a button at the top of
  the Statistics panel; `crates/stamp-gui/src/wizard.rs`, `wizard_ui.rs`,
  `docs/site/wizard.rst`). A few questions lead from what the user wants to
  know to the analysis, test and plots that answer it: what is compared
  (two samples, two groups, multiple groups, samples along a measured
  variable), the research question (features that differ, that follow the
  variable, that occur together; communities; diversity; composition;
  sequencing depth), and, for features that differ between groups, whether
  the same subjects were sampled in every group and what the data look
  like. Each choice is a card with a small drawing and an "i" button for a
  longer explanation with an example. Every choice is always shown; those
  the loaded data cannot support are greyed and say why (no metadata, too
  few groups, no numeric field, no field that matches subjects, proportions
  where counts are needed). The size of the smallest group, or the number
  of matched subjects, marks one answer about the data as suggested. The
  last page names the test, lists what will be set, explains why, lists the
  plots with what each is for, alternatives and what to report; "Set up
  this analysis" chooses the settings (with the Benjamini-Hochberg FDR),
  runs the analysis and shows the first plot. For multiple groups with
  "most data" it chooses Welch's ANOVA with Games-Howell over ANOVA with
  Tukey-Kramer when the groups differ twofold in size or when the
  Brown-Forsythe test finds the spread of over 10% of the features
  different (`Facts::spread`, `Facts::spread_clr`), and says which and why.
  The flow chart it follows is in `docs/WIZARD_PLAN.md`. "At a glance" in the same window shows the
  tests as one table (`wizard::GLANCE`): a row for each of 18 tests under
  what it is for, with its drawing, when to use it, what it does and what
  it assumes; rows the data cannot support are greyed with the reason, and
  "Show" on a row opens its recommendation.
- **Sample filter.** Below the group field, a metadata field and the values
  whose samples every analysis and plot uses (tick or untick each value);
  samples left out are excluded from everything, groups left empty
  disappear, and the number of samples kept is shown. The command line has
  the same choice for every command that reads metadata: `--where
  FIELD=VALUE` (repeat for alternatives of one field; different fields must
  all match), numeric `--where "FIELD>=X"` (`<`, `<=`, `>`, `>=`), and
  `--where-not FIELD=VALUE`. Both use `SampleSelection` in
  `stamp-core/src/selection.rs`.
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
- **Font sizes of plots.** Settings > Preferences sets the size of the
  tick labels, axis labels and legend text of every plot (8, 9 and 9
  points to start with). Text particular to a kind of plot (titles,
  notes, feature names, curve labels, Venn counts) is sized in a "Font
  sizes" frame of that plot's Configure plot window. v2's text was a fixed
  8 points [57]. On the command line: `--tick-font-size`,
  `--axis-label-font-size` and `--legend-font-size`.
- **Font of plots.** Settings > Preferences chooses the font of all plot
  text: Arial (the default), Times New Roman, Courier New or DejaVu Sans
  (v2's font) [58]. The fonts are bundled (Liberation Sans, Serif and Mono,
  free fonts with the same letter widths as Arial, Times New Roman and
  Courier New), so plots look the same on every system;
  SVG files name the original font first. On the command line: `--font`.
- **Plots kept once drawn.** A plot is redrawn only when something it is
  drawn from changes: its data, filters, colours or settings, or the
  selected feature for plots of one feature. Returning to a plot, or
  selecting a feature while a plot of the whole data set (ANCOM, PCoA,
  rarefaction) is shown, is immediate.
- **Plots listed by group.** The plot selector beneath the plot lists the
  plots of an analysis under five headings, by the question they answer:
  differences between features (with the p-value histogram and multiple
  comparison plots, which check the tests, after a divider), one feature,
  composition, diversity, and similarity between samples. Each analysis
  still opens on the same plot.
- **A "Correlation" tab** beside the three kinds of analysis of v2, with
  its own statistics, filters, feature table, results table and plots.

## Plots

- **Map of sample locations** (two groups, multiple groups, correlation;
  plot group "Where the samples are from"). Samples placed by latitude and
  longitude metadata fields (found by name or chosen) on a map of the
  world, a continent, the area around the samples (the default) or a box of
  longitudes and latitudes, in the Equal Earth (default), Robinson,
  equirectangular or Mercator projection; countries, coasts and large
  lakes from Natural Earth (public domain; 1:110m for wide maps, 1:50m
  for regions, compiled in: about 170 KB), with optional sea colour,
  country outlines and labelled lines of longitude and latitude. Samples at
  one site (within a grid cell sized to the map, or set in degrees) share a
  marker per colour, sized by the number of samples, a numeric field or a
  feature's abundance (or fixed; linear or log, with a size legend) and
  coloured and shaped as in the ordination plots (a site takes the mean or
  maximum of a number); or one marker per sample, optionally spread around
  the site. Tooltips list the samples, groups and values; samples without
  coordinates or outside the area are counted above the map. Follows the
  sample filter. Command line `--plot-type map` with `--map-lat-field`,
  `--map-lon-field`, `--map-extent`, `--map-projection`,
  `--map-resolution`, `--map-size-by`, `--map-aggregate`/
  `--no-map-aggregate`, `--map-site-precision`, `--map-jitter`,
  `--map-graticule`/`--no-map-graticule`, `--no-map-ocean`,
  `--no-map-borders`, and the ordination plots' `--color-by`, `--shape-by`,
  `--color-scale`, `--color-map` (also offered by the correlation command
  for the map). The data are converted by
  `scripts/naturalearth/convert.py`.
- **Ordination plots coloured and shaped by any field** (PCoA, NMDS, in
  every analysis that has them). Colour by the groups (as before), another
  categorical metadata field, a numeric field on a continuous colour scale
  (viridis by default, plasma, magma, cividis or any heatmap colour map;
  linear or log10 with zeros at a floor of half the smallest positive
  value; a colour bar beside the plot), or the relative abundance of a
  feature (the selected one, following the selection, or one named). A
  second categorical field gives the marker shapes (ten shapes; beyond
  them the rarest values share an "Other" shape, with a note); without
  one, samples coloured by anything else keep a shape per group. Samples
  without a value are small grey hollow markers, counted in the legend.
  The legend goes beside the plot with a section for colours and one for
  shapes, each with sample counts; tooltips give the sample, its group and
  both values; high values are drawn on top by default (or low, or in
  sample order); opacity is adjustable. The PCoA community tests still
  compare the analysis groups, and the plot says so. Command line:
  `--color-by group|field:NAME|category:NAME|feature:NAME`, `--shape-by`,
  `--color-scale`, `--color-map`, `--draw-order`, `--marker-size`,
  `--opacity`. Viridis, plasma, magma and cividis are also offered for the
  heatmaps.
- **Twenty group colours.** The palette continues after v2's ten with ten
  stronger colours (Tableau 10 without its grey, and a yellow), so up to
  twenty groups are distinct; beyond twenty the colours repeat (with a
  warning on the command line) [60].

- **Box plot of several features, on a logarithmic axis.** The box plot
  (two groups, multiple groups) can show several features at once: side by
  side with a box per group (coloured, with a legend) at each feature, or
  one panel per feature with the same or independent value axes. The
  features are the selected one, the N most significant that pass the
  filters, or a list (typed in Configure plot, or Ctrl+click in the feature
  table). Options: a log10 value axis (labelled 0.001, 0.01, 0.1, 1, 10;
  zeros on a line labelled 0 below a break in the axis; quartiles and
  whiskers are those of the values themselves), every sample drawn as a
  jittered semi-transparent point, the number of samples after each group's
  name, horizontal boxes, and the order of the groups (as given,
  alphabetical, highest median first, or by a coarser metadata field such
  as host order, with dashed lines between its categories). The bar plot of
  one feature in every sample has the log10 axis too. Command line:
  repeatable `--plot-feature`, `--box-top`, `--box-layout`,
  `--box-independent-axes`, `--box-log`, `--box-points`, `--box-counts`,
  `--box-horizontal`, `--box-group-order`, `--box-order-field`, `--bar-log`.
- **Paired samples joined in the box plot.** After a paired test (paired
  t-test or Wilcoxon signed-rank, two groups) the box plot can draw a line
  between the two boxes for every pair of samples, with a point at each
  end, coloured by the group in which the value is higher (grey for a tie);
  the tooltip between the boxes counts the pairs each way. It shows what
  the paired test tests, which two boxes cannot. Several features get a
  panel each. *Configure plot > Samples > Join paired samples with lines*;
  command line: `--box-pairs`.
- **Prevalence-abundance plot** (occupancy-abundance), for two groups and
  multiple groups: every feature placed by its mean relative abundance (log
  axis; over all samples of the group or only those where it is present)
  and the percentage of samples in which it is present (above zero, or at
  least a chosen proportion). One panel per group on shared axes; all
  groups in one panel coloured by group (or one chosen group); or a
  specificity view with each feature once, at the group where it is most
  prevalent, coloured by that group when its prevalence there exceeds that
  in every other group by a chosen number of percentage points and grey
  when shared. Highlighted features (the selected feature, a typed list or
  the highlight control; `--highlight` on the command line) are drawn in
  red and named; tooltips give the numbers. Command line: `--plot-type
  prevalence-abundance` with the `--prevalence-*` options, and
  `--prevalence-table` for the prevalence and mean abundance of every
  feature in every group.
- **Group heatmap**, for two groups and multiple groups: features against
  groups rather than samples, each cell the feature's mean, median or
  largest proportion in the group, or its prevalence (the percentage of the
  group's samples in which it is present, optionally above a minimum
  proportion). Linear or log10 colours with zeros in grey, values written in
  the cells, the most abundant features (50 by default), clustering of rows
  and columns, or groups ordered and labelled by a coarser metadata field
  (host species by host order, say). It has no limit on the number of
  samples, unlike the heatmap of samples. Command line: `--plot-type
  group-heatmap` with the `--heatmap-*` options, and `--heatmap-table` for
  the values.
- **Co-occurrence heatmap**, for two groups, multiple groups and
  correlation: Spearman or Pearson correlations between every pair of the
  most abundant (or prevalent) features, plus any named, of proportions or
  centred log ratios, clustered, with Benjamini-Hochberg-significant cells
  marked; or the strongest correlations of the selected feature with every
  other, as bars. Within all samples of the analysis or one group. Command
  line: `stamp-cli co-occurrence`, which writes the pairs (coefficient,
  p-value, q-value) as a table.
- **Feature against feature scatter plot**: a third view of the
  co-occurrence heatmap ("The selected feature against one other"), a point
  for each sample with the selected feature's proportion against another's
  (named, or the most strongly correlated), coloured by group, with the
  fitted line, the correlation and its p-value. Command line:
  `stamp-cli co-occurrence --with A --against B`.
- **p-value histogram for the group analyses**: v2's plot for two samples
  is offered for two groups and multiple groups too (and for correlation),
  as the check that some features truly differ. Command line:
  `--plot-type p-value-histogram`.
- **Alpha diversity scatter plot** (Correlation tab): the chosen diversity
  measure of each sample against the numeric field, coloured by group, with
  the fitted line and the Spearman or Pearson correlation and its p-value.
  Command line: `stamp-cli correlation --plot-type alpha-diversity`.
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
- **ANCOM plot**, for two groups and multiple groups: W of every feature
  against how much it differs on the centred log-ratio scale (the
  difference in means for two groups, eta-squared for more), with the
  features called different in red and the cut-off as a dashed line. The
  test and significance level are options. On the command line:
  `stamp-cli ancom --plot FILE`, or `--plot-type ancom` with
  `two-groups` and `multi-groups`.
- **NMDS plot**, for two groups and multiple groups, beside the PCoA
  plot, with the stress written above it. On the command line:
  `--plot-type nmds`.
- **Shared features plot**, in all three analysis types: which features
  each group (or sample) has, and which combinations of groups share
  them, as an UpSet plot (the largest combinations, with a matrix of dots
  beneath) or as a Venn diagram for two or three groups. A feature counts
  as present in a group if it is found in at least a chosen percentage of
  the group's samples (50% to start with). Hovering a bar or a count
  lists its features. On the command line: `--plot-type shared-features`,
  with `--shared-style upset|venn` and `--min-prevalence`.
- **Rank-abundance curves**, in all three analysis types: each sample's
  (or each group's mean) features from the most abundant to the least,
  against their proportion on a logarithmic scale. On the command line:
  `--plot-type rank-abundance`, with `--rank-curves samples|groups`.
- **Mantel test plot** (correlation tab): the community distance of every
  pair of samples against their difference in the numeric field, with
  the Mantel test above it. On the command line: `stamp-cli correlation
  --plot-type mantel`, which also prints the test.
- **Alpha diversity box plot**, for two groups and multiple groups: a box
  for each group of the chosen diversity measure of its samples, with the
  Kruskal-Wallis p-value. On the command line:
  `--plot-type alpha-diversity`, with
  `--alpha-metric observed|shannon|simpson|chao1` and `--rarefy`.
- **PCoA plot**, for two groups and multiple groups, and the first plot
  shown for multiple groups: the samples laid out by principal coordinates
  analysis of a chosen distance (Bray-Curtis, Jaccard, Aitchison, or
  Euclidean, which reproduces a PCA of the proportions), with the outcome
  of PERMANOVA, ANOSIM and PERMDISP above it. It replaces v2's PCA plot
  [64]. On the command line: `--plot-type pcoa`, with
  `--distance bray-curtis|jaccard|aitchison|euclidean`.
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

- **"Best" legend position for the ordination plots and the heatmap**, and legends
  on the left and right sides as well as in the corners [36, 43].
- **Any confidence level** for the post-hoc plot [47].
