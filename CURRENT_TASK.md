# Current task

What is being worked on right now, so that work can be picked up again if a
session ends unexpectedly. Update this file when a task starts, at each
meaningful step, and when it finishes.

## Status

Idle. Roadmap tiers 1 to 4 are all done, committed and pushed. Nothing further is queued.

## Last completed

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

Nothing queued. What v2 has that the port still lacks is listed under "Not
yet ported" in `docs/CHANGES_FROM_PYTHON.md`; `docs/ROADMAP.md` lists what
was considered and left out.
