Analyses
========

STAMP offers four kinds of analysis, chosen by the tabs at the top of the
properties panel. Each tests every feature of the profile on its own, then
lets you filter the results and explore them in plots.

.. list-table::
   :header-rows: 1
   :widths: 20 40 40

   * - Analysis
     - Question
     - Example
   * - **Two samples**
     - Does a feature make up a different proportion of two samples?
     - One metagenome from a polluted site against one from a clean site.
   * - **Two groups**
     - Does a feature differ in proportion between two groups of samples?
     - Ten healthy and ten diseased gut samples.
   * - **Multiple groups**
     - Does a feature differ between three or more groups of samples?
     - Soils from four land uses.
   * - **Correlation**
     - Does a feature rise or fall with a number known for each sample?
     - Gut samples and the patients' age; lake samples and depth.

The :doc:`statistics` explains each test, and :doc:`choosing_a_test`
suggests which to use.

Two samples
-----------

Choose *Sample 1* and *Sample 2*. Each feature is tested on its counts in
the two samples: is its proportion different, given how many sequences each
sample has? With no replicate samples, these tests can say only that the
two *samples* differ, not that the conditions they came from do. Use them
for exploration, or where only one sample of each kind exists.

The window starts with the *G-test (w/ Yates') + Fisher's* test and the
*DP: Asymptotic-CC* confidence interval, as STAMP 2 did.

Two groups
----------

Choose *Group 1* and *Group 2*, or *All other samples* as the second group
to compare one group with everything else. Each feature is tested on its
proportions in the samples of each group. The window starts with Welch's
t-test.

.. _paired-tests:

Paired samples
^^^^^^^^^^^^^^

When each sample of one group belongs with one sample of the other (the
same patient before and after treatment, the same plot in two seasons),
choose a paired test: the **paired t-test** or the **Wilcoxon signed-rank
test**. Then choose the **Pair field**, the metadata field that says which
samples belong together (the patient's identifier, say). STAMP pairs the
samples of the two groups with the same value in that field; a sample
without exactly one partner is left out, and the number of pairs is shown
beneath the field.

Pairing removes the differences between subjects from the comparison, so a
paired test finds smaller consistent changes than an unpaired one.

Multiple groups
---------------

Tick the groups to compare in the groups list. Each feature is tested for
any difference between the groups (ANOVA or the Kruskal-Wallis test). A
significant result says that *some* groups differ, not which ones; the
**post-hoc test** chosen in the statistics panel then compares every pair
of groups for the selected feature, shown in the post-hoc plot. The window
starts with ANOVA and the Tukey-Kramer post-hoc test.

.. _correlation:

Correlation
-----------

Choose a **Numeric field**, a metadata field holding a number for each
sample. Each feature's proportion is correlated with it by Spearman's rank
correlation (the default) or Pearson's correlation. Samples without a number
in the field are left out; the status bar says how many.

Common settings
---------------

Type
   For two samples and two groups, a **two-sided** test asks whether the
   feature differs in either direction; a **one-sided** test asks whether it
   is larger in the first sample or group. Use two-sided unless you decided
   on the direction before looking at the data. Some tests are two-sided
   only.

Coverage
   The confidence level of the intervals: 0.95 gives 95% confidence
   intervals.

Transform
   For two groups and multiple groups, *Centred log-ratio* applies the
   test to log ratios rather than proportions; effect sizes, confidence
   intervals and plots stay in proportions. See :ref:`clr`.

Multiple test correction
   Adjusts the p-values for the number of features tested; see
   :ref:`multiple-testing`.

Filtering
---------

Filters decide which features are **active**: listed in the plots of all
features and kept by "Show only active features". They do not change the
statistics of any feature, only which are shown.

Select specific features
   Choose features by hand, at any level of the hierarchy: choosing a phylum
   chooses every feature beneath it.

p-value filter
   Keeps features whose p-value (corrected, if a correction is chosen) is at
   or below the threshold. It starts on, at 0.05.

Sequence filter and parent sequence filter (two samples, two groups)
   Remove features seen in too few sequences to judge, or whose parent
   category has too few. *Maximum* keeps a feature if its count reaches the
   threshold in at least one sample; *minimum* keeps it only if every sample
   reaches it. The *independent* choices set a separate threshold for each
   sample or group (for two groups: *independent, maximum* needs some sample
   of each group to reach its group's threshold, *independent, minimum*
   every sample).

Effect size filters
   Keep features whose difference is large enough to matter, whatever their
   p-value. Up to two filters can be combined: *either filter (OR)* removes
   a feature that fails either one, *both filters (AND)* removes it only if
   it fails both. The measures are:

   * two samples: difference between proportions (in percentage points),
     ratio of proportions, odds ratio;
   * two groups: difference between proportions (of the group means), ratio
     of proportions;
   * multiple groups: eta-squared (starts at 0.8);
   * correlation: the size of the correlation, whatever its sign (starts at
     0.5).

   Ratios count in either direction: a filter of 2 keeps features at least
   twice as abundant in either sample or group.

.. tip::

   A good habit is to combine a p-value filter with an effect size filter:
   the first keeps differences that are unlikely to be chance, the second
   those large enough to matter. With many samples, tiny differences become
   "significant"; with few, large differences can fail to.

The number of active features is shown at the bottom of the filtering
panel and in the status bar.
