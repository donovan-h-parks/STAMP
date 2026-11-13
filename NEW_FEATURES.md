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

## Plots

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
  - Options: the number of points computed, ending every curve at the depth
    of the smallest sample, each sample's Chao1 estimate as a dashed line,
    sample labels, and the legend position.
  - A profile must hold counts of sequences. One with fractional values
    (proportions, normalised counts) is refused with a message.
  - On the command line: `--plot-type rarefaction`.
  - The idea follows the rarefaction plot of the GenomicaMicrob fork of
    STAMP; the implementation is independent of it.

- **"Best" legend position for the PCA plot and the heatmap**, and legends
  on the left and right sides as well as in the corners [36, 43].
- **Any confidence level** for the post-hoc plot [47].
