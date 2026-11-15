# Planned additions to STAMP v3

Plots and statistical tests that are common in microbiome and metagenomic
studies and that STAMP v2 did not have, in the order they are to be added.
The list was drawn up on 2026-11-14, with the functionality of scikit-bio in
mind. Each tier is started on the owner's go-ahead; an item is ticked when it
is in the program, at which point it is described in `NEW_FEATURES.md`.

The largest gap is that STAMP tests one feature at a time and has nothing
for the community as a whole (diversity, distances between samples,
ordination on them, and tests on them), which is where most studies begin.

## Tier 1: small additions that build on what exists

- [x] **Stacked bar plot of composition**: the most abundant features of
  each sample or group, the rest as "Other".
- [x] **Volcano plot**: effect size against -log10 of the p-value for every
  feature.
- [x] **Mann-Whitney U test** (Wilcoxon rank-sum) for two groups.
- [x] **Dunn's test** as a post-hoc test, the rank-based companion of
  Kruskal-Wallis.

## Tier 2: the community as a whole

- [x] **Alpha diversity**: observed richness, Shannon, Simpson and Chao1 for
  each sample; a box plot by group; a test between groups.
- [x] **Distances between samples**: Bray-Curtis, Jaccard, Aitchison and Euclidean.
- [x] **PCoA** on a chosen distance; replaces the PCA plot.
- [x] **PERMANOVA**, with **PERMDISP** to tell a difference in location from
  a difference in spread; **ANOSIM** as the older alternative.
- [x] **Box plot of distances** within and between groups.
- [x] A stated **normalisation** for these analyses (rarefy to a depth, or
  proportions), since diversity and most distances assume comparable depth.

## Tier 3: numeric metadata, paired designs, compositional data

- [x] **Correlation with a numeric metadata field**: Spearman (and Pearson)
  for every feature, with the multiple-test corrections.
- [x] **Plot of a feature against a numeric field**, with a fitted line.
- [x] **Paired tests**: paired t-test and Wilcoxon signed-rank, with a
  metadata field that identifies the pairs.
- [x] **Centred log-ratio transform** as an option before the existing
  tests, to address the compositional nature of proportions.

## Tier 4: further methods

- [x] **ANCOM** for compositional differential abundance.
- [x] **Mantel test** between two distance matrices.
- [x] **Shared-feature plot** (UpSet or Venn) for "core" features.
- [x] **NMDS** ordination.
- [x] **Rank-abundance curves**.

## Considered and left out

- **Phylogenetic measures** (UniFrac, Faith's PD): they need a tree with
  branch lengths, and a STAMP profile carries a hierarchy only.
- **Negative-binomial models** (DESeq2, edgeR): a large amount of numerical
  work for something its users already do in R.
- **Constrained ordination** (CCA, RDA): worthwhile, but after PCoA.
- **Sequence, alignment and tree handling** from scikit-bio: outside what
  STAMP does.
