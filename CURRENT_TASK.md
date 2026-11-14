# Current task

What is being worked on right now, so that work can be picked up again if a
session ends unexpectedly. Update this file when a task starts, at each
meaningful step, and when it finishes.

## Status

Idle, **waiting on the user for the go-ahead on roadmap tier 3** (numeric
metadata, paired tests, CLR; see `docs/ROADMAP.md`). The instruction was to
prompt for tiers 2 to 4 in order, so do not start tier 3 or 4 unasked.

## Last completed

- **Roadmap tier 2** (2026-11-14): alpha diversity, distances
  (Bray-Curtis, Jaccard, Aitchison) and PCoA in
  `stamp-core/src/stats/diversity.rs`; PERMANOVA, PERMDISP and ANOSIM in
  `stats/community.rs`; the plots in `stamp-plot/src/community_plots.rs`
  (`PlotKind::AlphaDiversity`, `Pcoa`, `DistanceBox`, in the two group
  modes) with their Configure plot groups and `--plot-type
  alpha-diversity|pcoa|distance-box` (`--alpha-metric`, `--distance`,
  `--rarefy`). Values checked against scikit-bio 0.7.4. Chao1 and Chao2
  switched to the bias-corrected form. Unclassified categories are not
  features in these analyses. Tests, lint and web bundle done; check
  `git status` for whether it is committed.
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

- Branch `rust-port`, pushed to `origin` up to `b5f1820` (tag
  `feature-parity` is on `6b834ba`).
- Tier 1 is committed locally as `466fc38` and **not pushed** (the user
  asked for a commit only). Tier 2 is uncommitted unless `git log` shows
  otherwise.

## Next

Nothing queued. What v2 has that the port still lacks is listed under "Not
yet ported" in `docs/CHANGES_FROM_PYTHON.md`.
