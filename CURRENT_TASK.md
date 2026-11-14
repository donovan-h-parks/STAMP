# Current task

What is being worked on right now, so that work can be picked up again if a
session ends unexpectedly. Update this file when a task starts, at each
meaningful step, and when it finishes.

## Status

Idle, **waiting on the user**: roadmap tier 1 is done, and they asked to be
prompted for tiers 2, 3 and 4 in order (`docs/ROADMAP.md`). Tier 2 (alpha
diversity, distances, PCoA, PERMANOVA/PERMDISP/ANOSIM, distance box plot,
normalisation) is next and has not been started.

## Last completed

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

- Branch `rust-port`, pushed to `origin` up to `5113b3f` (tag
  `feature-parity` is on `6b834ba`).
- Everything up to the example data sub-menu is committed and pushed
  (`5113b3f`). The group-level curves are uncommitted work in progress.

## Next

Nothing queued. What v2 has that the port still lacks is listed under "Not
yet ported" in `docs/CHANGES_FROM_PYTHON.md`.
