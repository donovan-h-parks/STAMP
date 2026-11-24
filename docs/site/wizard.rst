The analysis wizard
===================

The wizard asks a few questions about what you want to know and leads to
the analysis, statistical test and plots in STAMP that answer it. It is for
anyone who is unsure which test suits their data; it chooses nothing that
cannot also be chosen by hand in the Statistics panel. Open it with **Help >
Analysis wizard**, or with the *Analysis wizard* button at the top of the
Statistics panel.

Every choice is always shown. A choice that the loaded data cannot support
is greyed and says why (for example, "No metadata field holds a number"),
so that the wizard also shows what a data set would need for an analysis.
Each choice has a small drawing of what it means and an **i** button that
opens a longer explanation with an example.

.. figure:: images/gui_wizard_question.png
   :width: 100%

   The second question, for the enterotype example. Two questions are
   greyed: the samples have no measured variable, and the profile holds
   proportions rather than counts of sequences.

The questions
-------------

1. **What are you comparing?**

   * *Two samples*: one sample of each condition, without replicates.
   * *Two groups*: several samples of each of two conditions.
   * *Multiple groups*: several samples of each of three or more
     conditions.
   * *Samples along a measured variable*: each sample has a number (pH,
     age, depth) in the metadata.

   Groups come from the **group field**, which can be changed at the top of
   this step; the groups and their sizes are listed beside it.

2. **What is your research question?**

   * *Which features differ?*
   * *Which features rise or fall with the variable?*
   * *Which features occur together?*
   * *Do the communities differ as a whole?*
   * *Does diversity differ?*
   * *What are the samples made of?* (a description, with no test)
   * *Were the samples sequenced deeply enough?*

   Only "Which features differ?" between groups needs the two further
   questions; every other question leads straight to a recommendation.

3. **Were the same subjects sampled in every group?** Different subjects in
   each group (patients and controls), or the same subjects in every group
   (before and after a treatment). The second can be chosen when a metadata
   field names the subject of each sample; the wizard finds the fields that
   do, and how many subjects each one matches.

4. **What kind of difference, in what kind of data?**

   * *More or less abundant: most data* (about ten or more samples in each
     group).
   * *More or less abundant: small groups or extreme samples.*
   * *More or less abundant: features absent from most samples.*
   * *Found in more samples of one group* (presence, whatever the
     abundance).
   * *A few features dominate every sample.*

   STAMP counts the samples of your smallest group (or the subjects your
   groups share) and marks one of the first two answers as **suggested**,
   with the number it is based on. You can still choose another.

.. figure:: images/gui_wizard_data.png
   :width: 100%

   The fourth question. The smallest enterotype has two samples, so the
   rank-based tests are suggested; the test for sparse features is greyed
   because it compares two groups only.

The answers so far are listed along the top of the window; click one to
return to its question. *Back* returns to the previous question and *Start
over* to the first.

The recommendation
------------------

The last page names the recommended test (or, for a question without a
test, the plot), and gives:

* **What will be set**: the groups or samples compared, the test, the
  post-hoc test, any transform, and the multiple test correction.
* **Why** this test suits the answers given, in a paragraph, and anything
  the analysis cannot show.
* **Plots to look at**, each with what it is for.
* **Alternatives**, and when to prefer them.
* **What to report** in a paper.

.. figure:: images/gui_wizard_result.png
   :width: 100%

   The recommendation for small groups: the Kruskal-Wallis H-test with
   Dunn's test.

**Set up this analysis** chooses those settings in the Statistics panel,
runs the analysis, shows the first of the plots and closes the wizard.
*Show* beside a plot does the same and shows that plot. For a test of many
features the wizard always chooses the Benjamini-Hochberg FDR correction;
everything it sets can be changed afterwards in the Statistics panel. The
two samples, or the two groups, compared are those chosen there.

Where each path leads
---------------------

.. list-table::
   :header-rows: 1
   :widths: 22 39 39

   * - Comparison
     - Answers
     - Recommendation
   * - Two samples
     - Which features differ?
     - G-test (w/ Yates') + Fisher's
   * - Two groups
     - Different subjects; most data
     - Welch's t-test
   * -
     - Different subjects; small groups or extreme samples
     - Mann-Whitney U test
   * -
     - Different subjects; features absent from most samples
     - White's non-parametric t-test
   * -
     - Different subjects; found in more samples
     - Fisher's exact test (presence/absence)
   * -
     - The same subjects; most data
     - Paired t-test
   * -
     - The same subjects; small groups or extreme samples
     - Wilcoxon signed-rank test
   * -
     - The same subjects; found in more samples
     - McNemar's test (presence/absence)
   * - Multiple groups
     - Different subjects; most data
     - ANOVA with Tukey-Kramer (Games-Howell if the groups differ twofold
       in size)
   * -
     - Different subjects; small groups or extreme samples
     - Kruskal-Wallis H-test with Dunn's test
   * -
     - Different subjects; found in more samples
     - Chi-square test (presence/absence)
   * -
     - The same subjects
     - Friedman test with Wilcoxon signed-rank (Holm)
   * - Two or multiple groups
     - A few features dominate
     - The test for "most data" (or the matched test), with the centred
       log-ratio transform; the ANCOM plot as confirmation
   * -
     - Which features occur together?
     - Co-occurrence heatmap
   * -
     - Do the communities differ?
     - PCoA plot with PERMANOVA, ANOSIM and PERMDISP (and pairwise
       PERMANOVA for multiple groups); NMDS and distance box plots
   * -
     - Does diversity differ?
     - Alpha diversity box plot with a Kruskal-Wallis H-test
   * -
     - What are the samples made of?
     - Composition bar plot, heatmaps, prevalence-abundance, shared
       features, rank-abundance, map
   * -
     - Sequenced deeply enough?
     - Rarefaction curves
   * - Measured variable
     - Which features rise or fall?
     - Spearman rank correlation
   * -
     - Which features occur together?
     - Co-occurrence heatmap
   * -
     - Do the communities differ?
     - Mantel test plot
   * -
     - Does diversity differ?
     - Alpha diversity scatter plot

The reasons behind these choices are given in :doc:`choosing_a_test`, and
the tests themselves are explained in :doc:`statistics`.
