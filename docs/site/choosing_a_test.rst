Choosing a test
===============

STAMP starts every analysis with a sensible default, so a test need only be
changed for a reason. This page gives those reasons. The tests themselves
are explained in :doc:`statistics`.

Which analysis?
---------------

.. list-table::
   :header-rows: 1
   :widths: 45 25 30

   * - Your design
     - Analysis
     - Default test
   * - One sample of each condition, no replicates
     - Two samples
     - G-test (w/ Yates') + Fisher's
   * - Replicate samples of two conditions
     - Two groups
     - Welch's t-test
   * - The same subjects or sites measured twice
     - Two groups, paired test
     - Paired t-test or Wilcoxon signed-rank
   * - Three or more conditions
     - Multiple groups
     - ANOVA, Tukey-Kramer post-hoc
   * - A measured number (pH, age, depth)
     - Correlation
     - Spearman rank correlation

Two samples
-----------

* Keep the default, **G-test (w/ Yates') + Fisher's**, or use **Fisher's
  exact test** throughout; both are reliable for rare and abundant
  features alike.
* For the interval, **DP: Newcombe-Wilson** is the most reliable for small
  counts; use **RP: Asymptotic** if you think in fold changes.
* Remember that with no replicates you are comparing two samples, not two
  conditions.

Two groups
----------

.. list-table::
   :header-rows: 1
   :widths: 50 50

   * - Situation
     - Suggested test
   * - Most data sets
     - **Welch's t-test** (default)
   * - Small groups (fewer than about 5 to 10 samples each), or skewed
       data with extreme samples
     - **Mann-Whitney U test** or **White's non-parametric t-test**,
       alongside Welch's test
   * - Many features that are absent from most samples
     - **White's non-parametric t-test**, which handles sparse features
       with Fisher's exact test
   * - Paired samples (before and after, two seasons at the same sites)
     - **Paired t-test**, or the **Wilcoxon signed-rank test** for small
       numbers of pairs or extreme values
   * - Concern that a few features dominate the composition
     - Add the **centred log-ratio** transform, or confirm with **ANCOM**

Avoid the *t-test (equal variance)* unless you have a reason to assume the
groups vary equally: Welch's test is as powerful when they do and much
more reliable when they do not.

Multiple groups
---------------

* **ANOVA** with **Tukey-Kramer** (or **Games-Howell** if the groups differ
  in size or spread) for most data.
* **Kruskal-Wallis** with **Dunn's test** for small groups or skewed data.
* **Scheffè** if you want to be especially cautious about calling pairs
  different; **Welch's (uncorrected)** only for exploration.

Correlation
-----------

* **Spearman** (default) unless you expect a straight-line relationship;
  it is not thrown by a few extreme samples.
* **Pearson** when the relationship is linear and the data have no extreme
  values.
* To ask whether the community as a whole changes with the number, use the
  **Mantel test plot**.

Multiple test correction
------------------------

.. list-table::
   :header-rows: 1
   :widths: 50 50

   * - Situation
     - Suggested correction
   * - Exploratory study of many features (the usual case)
     - **Benjamini-Hochberg FDR**
   * - Many features expected to differ
     - **Storey FDR**
   * - A few features tested, or any false positive is costly
     - **Holm-Bonferroni**
   * - A first look, generating hypotheses
     - **No correction**, with results treated as tentative

Community-level questions
-------------------------

.. list-table::
   :header-rows: 1
   :widths: 50 50

   * - Question
     - Where to look
   * - Do the groups differ in composition overall?
     - **PCoA plot** (PERMANOVA, ANOSIM and PERMDISP), **NMDS plot**
   * - Are the samples of one group more variable than another?
     - PERMDISP on the **PCoA plot**; **Distance box plot**
   * - Do the groups differ in diversity?
     - **Alpha diversity box plot**
   * - Have the samples been sequenced deeply enough?
     - **Rarefaction curves**
   * - Does composition change with a measured number?
     - **Mantel test plot**
   * - Which features differ, robustly to compositional effects?
     - **ANCOM plot**

Checks before reporting a result
--------------------------------

1. **Is anything there?** Look at the :ref:`p-value histogram
   <plot-p-value-histogram>`: a peak near zero means that some features
   truly differ.
2. **Is the difference large enough to matter?** Look at the effect size and
   its confidence interval in the extended error bar plot, not just the
   p-value.
3. **Does it rest on a few samples?** Look at the box plot or bar plot of the
   feature.
4. **Does it survive correction for testing many features?** Use an FDR
   correction.
5. **Is it robust to the choice of test?** A finding that holds with Welch's
   t-test, the Mann-Whitney test and on the CLR scale is one to trust.
