Frequently asked questions
==========================

My profile will not load. What is wrong?
   The message says which line is at fault. The usual causes are: a file
   that is not tab-separated (save it from a spreadsheet as "tab-delimited
   text"); a value that is not a number; and a hierarchy that is not strict,
   such as a genus listed under two families. See :doc:`input_files`.

Some samples are missing from my analysis.
   Samples are left out if they are missing from the metadata (or the
   metadata names them differently: names must match exactly, including
   case), or if they are unticked in the metadata table (unticking a group
   unticks all its samples there). The multiple-group tests also leave out
   any group with a single sample. In a correlation, samples without a number in the field are
   left out; in a paired test, samples without exactly one partner.

Why are no features significant after correcting for multiple tests?
   With many features and few samples, a correction may leave nothing
   significant even when real differences exist; the study may lack the
   power to show them. Look at the :ref:`p-value histogram
   <plot-p-value-histogram>`: a peak near zero means real differences are
   there. Report effect sizes and confidence intervals, consider a false
   discovery rate correction rather than Bonferroni, and treat uncorrected
   results as hypotheses. See :ref:`multiple-testing`.

A feature is significant but its difference is tiny. Is that real?
   It may be statistically real but biologically unimportant. Large
   samples make small differences significant. Use an effect size filter to
   hide differences too small to matter (see :doc:`analyses`).

A feature has a large difference but is not significant. Why?
   Usually because it varies a lot between the samples of a group, or the
   difference rests on one or two samples. Look at its bar plot or box
   plot.

Which test should I use?
   See :doc:`choosing_a_test`. In short: the defaults are good choices; for
   two groups, check important results with a rank-based test.

Why do my results change slightly each time I change the seed?
   Bootstrap, permutation and White's tests, Storey's FDR and the community
   tests use random resampling. A fixed seed (Settings > Preferences) makes
   results repeatable; differences between seeds show how much the results
   depend on chance. Increase the number of replicates to reduce them.

The rarefaction curves or diversity plots are missing or give an error.
   They need counts of sequences. A profile of proportions or normalised
   values cannot be rarefied.

Are the results the same as STAMP 2?
   For almost every test, yes, to many decimal places; this is checked
   against the Python version for 108 reference cases. Where STAMP 3
   differs, it is because a bug was fixed or a method was brought into line
   with the standard one (for example, the Benjamini-Hochberg correction).
   See :doc:`changes`.

How do I make plots for a paper?
   Use **File > Save plot as** with SVG or PDF to keep text editable in a
   drawing program, or PNG at 300 dots per inch or more. Set the font and
   sizes in Settings > Preferences (Arial suits most journals), and the
   plot's own size and options in *Configure plot*.

How long do the ordination plots take with many samples?
   PCoA, NMDS and db-RDA compare every pair of samples, so their
   time grows with the square of the number of samples. As a guide, for a
   profile of about 6,000 genera on a desktop computer: with 619 samples
   the PCoA with its three community tests (999 permutations) takes about
   1.5 seconds and the NMDS about 2 seconds; with 1,307 samples about 7
   and 10 seconds. While a plot is computed, the window stays usable and shows
   what is being done and how far it has got, with a button to cancel (see
   :doc:`main_window`). Once computed, a plot is kept: changing only how it looks
   (colours, shapes, marker size, opacity, legend, fonts) redraws it at
   once, while a change of samples, level, distance, rarefying, seed or
   number of permutations computes it again.

Can STAMP analyse profiles of coverage, such as SingleM's?
   Yes, for analyses that compare proportions: the group tests, the
   correlation, and most plots. Rarefaction, Chao1 and the count-based tests
   of two samples need counts of sequences, and STAMP refuses those that
   need whole numbers. The glacier-fed stream example data are SingleM
   coverage; :doc:`example_glacier_streams` works through them.

The heatmap says it is limited to 1,000 samples. What can I do?
   A heatmap of more samples than that cannot show them legibly. Use the
   **Group heatmap**, which shows each group of samples in one column, or
   keep fewer samples with the *Sample filter* (``--where`` on the command
   line).

How do I analyse many data sets in the same way?
   Use the command line program in a script; see :doc:`command_line`.

How do I report a bug or ask for a feature?
   Open an issue on `GitHub <https://github.com/donovan-h-parks/STAMP/issues>`_,
   with the files (or a small example) that show the problem.
