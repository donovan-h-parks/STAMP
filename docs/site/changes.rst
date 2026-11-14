Changes from STAMP 2
====================

STAMP 3 is a rewrite of STAMP 2 (Python 2 and PyQt4) in Rust. It keeps
STAMP 2's analyses, plots, file formats and the column headings of its
result tables, and gives the same results for almost every test. It runs as
a desktop program on Linux, Windows and macOS, as a command line program and
in a web browser.

New in STAMP 3
--------------

Analyses
   Mann-Whitney U, paired t and Wilcoxon signed-rank tests for two groups;
   Kruskal-Wallis for any group size; Dunn's post-hoc test; correlation with
   a numeric metadata field (Pearson or Spearman); the centred log-ratio
   transform; ANCOM.

Communities
   Rarefaction and rank-abundance curves; alpha diversity (observed
   features, Shannon, Simpson, Chao1); Bray-Curtis, Jaccard and Aitchison
   distances; PCoA and NMDS; PERMANOVA, ANOSIM, PERMDISP and the Mantel
   test; shared-feature plots (UpSet and Venn).

Plots
   Volcano, composition and correlation plots; plots grouped by the
   question they answer; a choice of font and font sizes; PDF export; hover
   tooltips; zoom; a colour for each group, used in every plot.

Program
   The browser version; the command line program, with converters for
   other tools' output; BIOM 2 (HDF5) tables; built-in example data;
   repeatable resampling with a set seed; light and dark themes.

Differences in results
----------------------

STAMP 3 fixes a number of STAMP 2 bugs and changes some methods to the
standard ones. The most noticeable are:

* the **Benjamini-Hochberg** correction follows the published method, so its
  q-values can be lower than STAMP 2's;
* the **Kruskal-Wallis** test is run for groups of any size, as in R;
* the **Tukey-Kramer** and **Games-Howell** tests report computed p-values,
  where STAMP 2 reported a bound read from tables (such as "< 0.05").

Every difference is listed, with the reason for it, in
`CHANGES_FROM_PYTHON.md
<https://github.com/donovan-h-parks/STAMP/blob/rust-port/docs/CHANGES_FROM_PYTHON.md>`_,
and every new feature in `NEW_FEATURES.md
<https://github.com/donovan-h-parks/STAMP/blob/rust-port/NEW_FEATURES.md>`_.

Not available
-------------

STAMP 2's plugin system, which loaded user-written Python tests and plots,
is not part of STAMP 3.
