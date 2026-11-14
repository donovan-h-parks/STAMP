Command line
============

``stamp-cli`` runs STAMP's analyses without a window: for scripts, servers
and pipelines. Each command reads a profile (and metadata), writes a
tab-separated results table with the same column headings as STAMP 2, and
can also draw a plot.

.. code-block:: text

   stamp-cli <COMMAND> [OPTIONS]

==================  =======================================================
Command             What it does
==================  =======================================================
``info``            Describe a profile: its levels, samples and metadata
                    fields
``two-samples``     Compare the features of two samples
``two-groups``      Compare the features of two groups of samples
``multi-groups``    Compare the features of several groups of samples
``post-hoc``        Compare every pair of groups for one feature
``correlation``     Correlate every feature with a numeric metadata field
``ancom``           Find features that differ between groups by ANCOM
``convert``         Convert another tool's output to a STAMP profile
==================  =======================================================

``stamp-cli <COMMAND> --help`` lists every option of a command, with its
default.

Describing a profile
--------------------

Start with ``info`` to see the level names and metadata fields to pass to
the other commands::

   stamp-cli info --profile coal.spf --metadata coal.metadata.tsv

Options common to the analyses
------------------------------

``-p, --profile FILE``
   The profile.
``-m, --metadata FILE``
   The metadata (all commands but ``two-samples``).
``--profile-level HEADING``
   The level to analyse, by its heading in the profile (default: the most
   specific level).
``--parent-level HEADING``
   The level that proportions are taken against (default: the entire
   sample).
``--unclassified retain|remove|frequency-only``
   The treatment of unclassified sequences (see :doc:`input_files`).
``-o, --output FILE``
   Write the results table to a file instead of the screen.
``-f, --field FIELD``
   The metadata field that defines the groups (or holds the numbers, for
   ``correlation``).
``--exclude-sample SAMPLE``
   Leave a sample out; repeat for more.
``--correction``
   ``none`` (default), ``benjamini-hochberg``, ``bonferroni``,
   ``holm-bonferroni``, ``sidak`` or ``storey``.
``--significance P``
   Keep only features whose corrected p-value is at most P.
``--seed N``, ``--replicates N``, ``--pseudocount X``
   As in the preferences of the window (see :doc:`main_window`).

Two samples
-----------

::

   stamp-cli two-samples --profile EdwardsIronMine.spf \
       --sample1 Red --sample2 Black --profile-level Subsystem \
       --correction benjamini-hochberg --significance 0.05 \
       --effect-size-filter diff-between-prop=0.5 \
       --output red_vs_black.tsv --plot red_vs_black.png

``--test``
   ``fisher`` (default), ``g-test-fisher``, ``g-test``, ``g-test-yates``,
   ``chi-square``, ``chi-square-yates``, ``diff-between-prop``,
   ``hypergeometric``, ``bootstrap``, ``permutation``.
``--type``
   ``two-sided`` (default) or ``one-sided``.
``--ci``
   ``dp-newcombe-wilson`` (default), ``dp-asymptotic``,
   ``dp-asymptotic-cc``, ``odds-ratio``, ``ratio-proportions``.
``--coverage``
   Confidence level (default 0.95).
``--sequence-filter``, ``--parent-sequence-filter``
   ``maximum=N``, ``minimum=N`` or ``independent=N1,N2`` (see
   :doc:`analyses`).
``--effect-size-filter MEASURE=MINIMUM``
   ``diff-between-prop``, ``odds-ratio`` or ``ratio-proportions``; give it
   twice for two filters, combined by ``--effect-size-operator or|and``.

.. note::

   The command line starts with Fisher's exact test and the Newcombe-Wilson
   interval, while the window starts with STAMP 2's choices (G-test +
   Fisher's and the asymptotic interval with continuity correction). Name
   the test and interval to get the same results from both.

Two groups
----------

::

   stamp-cli two-groups --profile Enterotypes.profile.spf \
       --metadata Enterotypes.metadata.tsv --field Enterotype \
       --group1 "Enterotype 1" --group2 "Enterotype 3" \
       --correction benjamini-hochberg --plot e1_vs_e3.svg

``--group2`` may be left out to compare ``--group1`` with all other
samples.

``--test``
   ``welch`` (default), ``t-test``, ``white``, ``mann-whitney``,
   ``paired-t``, ``wilcoxon``.
``--pair-field FIELD``
   For ``paired-t`` and ``wilcoxon``: the field that pairs the samples (see
   :ref:`paired-tests`).
``--transform``
   ``none`` (default) or ``clr`` (see :ref:`clr`).

Filters as for two samples, except that the effect sizes are
``diff-between-prop`` and ``ratio-proportions``, and the sequence filters
take ``maximum=N``, ``minimum=N``, ``independent-minimum=N1,N2`` or
``independent-maximum=N1,N2``.

Multiple groups and post-hoc tests
----------------------------------

::

   stamp-cli multi-groups --profile Enterotypes.profile.spf \
       --metadata Enterotypes.metadata.tsv --field Enterotype \
       --group "Enterotype 1" --group "Enterotype 2" --group "Enterotype 3" \
       --test kruskal-wallis --correction benjamini-hochberg

   stamp-cli post-hoc --profile Enterotypes.profile.spf \
       --metadata Enterotypes.metadata.tsv --field Enterotype \
       --group "Enterotype 1" --group "Enterotype 2" --group "Enterotype 3" \
       --feature Prevotella --test games-howell --plot prevotella.png

``--group`` chooses the groups (default: every group). ``multi-groups``
takes ``--test anova`` (default) or ``kruskal-wallis``; ``post-hoc`` takes
``--test tukey-kramer`` (default), ``games-howell``, ``scheffe``, ``welch``
or ``dunn``, and ``--feature`` (repeat for several features).

Correlation
-----------

::

   stamp-cli correlation --profile soil.spf --metadata soil.metadata.tsv \
       --field pH --method spearman --correction benjamini-hochberg \
       --min-correlation 0.5

``--method`` is ``spearman`` (default) or ``pearson``.
``--min-correlation`` keeps features whose coefficient is at least that far
from zero. ``--group-field`` colours the points of the correlation scatter
plot by group.

ANCOM
-----

::

   stamp-cli ancom --profile coal.spf --metadata coal.metadata.tsv \
       --field Environment --profile-level Genus --plot ancom.png

``--test`` is ``anova`` (default) or ``kruskal-wallis``, the test applied to
the log ratio of every pair of features; ``--alpha`` its significance level
(default 0.05).

Plots
-----

Any analysis command draws a plot with ``--plot FILE``; the extension
chooses the format (``.png``, ``.svg`` or ``.pdf``). The plot shows the
features left after the filters.

``--plot-type``
   Any plot available for the analysis: ``extended-error-bar``, ``bar``,
   ``box``, ``profile-bar``, ``scatter``, ``sequence-histogram``,
   ``p-value-histogram``, ``multiple-comparison``, ``pca``, ``heatmap``,
   ``post-hoc``, ``rarefaction``, ``volcano``, ``composition``,
   ``alpha-diversity``, ``pcoa``, ``distance-box``, ``correlation-scatter``,
   ``nmds``, ``shared-features``, ``rank-abundance``, ``ancom``,
   ``mantel``. The default is the extended error bar plot for two samples
   or groups and the heatmap for several groups.
``--plot-feature FEATURE``
   The feature of the bar, box and correlation scatter plots (default: the
   most significant).
``--plot-width``, ``--plot-height``, ``--plot-row-height``
   Size in inches.
``--plot-dpi``
   Resolution of PNG files (default 300).
``--plot-sort-by``
   ``p-values``, ``effect-sizes`` or ``feature-labels``.
``--plot-legend``
   Where to put the legend of the extended error bar plot.
``--plot-uncorrected-p-values``
   Show uncorrected p-values.
``--font``, ``--tick-font-size``, ``--axis-label-font-size``, ``--legend-font-size``
   Font (``arial``, ``times-new-roman``, ``courier-new``, ``dejavu-sans``)
   and sizes in points.
``--alpha-metric``, ``--distance``
   The diversity measure and distance of the community plots.

.. _cli-convert:

Converting other formats
------------------------

``stamp-cli convert`` turns the output of other tools into a STAMP profile,
written to the screen or to ``-o FILE``:

.. code-block:: text

   stamp-cli convert mgrast table.tsv --heading Level1 --heading Level2 -o profile.spf
   stamp-cli convert comet sample1.txt sample2.txt -o profile.spf
   stamp-cli convert rita sample1.txt sample2.txt -o profile.spf
   stamp-cli convert mothur --taxonomy x.taxonomy --groups x.groups --names x.names -o profile.spf
   stamp-cli convert biom table.biom --metadata-field taxonomy -o profile.spf
   stamp-cli convert cog img_profile.tsv -o profile.spf

* **mgrast**: an MG-RAST abundance table; ``--heading`` names each level of
  the hierarchy, in order.
* **comet**, **rita**: one result file per sample. For RITA, ``--group``
  chooses the classification groups to count.
* **mothur**: a taxonomy file and a groups file, and optionally a names
  file.
* **biom**: a BIOM table (JSON or HDF5, optionally gzip-compressed).
  ``--metadata-field`` names the observation metadata holding the hierarchy,
  such as ``taxonomy`` or ``KEGG_Pathways``.
* **cog**: adds COG categories to an IMG/M COG profile.
  ``--assign-to-each-code`` gives a COG with several category codes to each
  of its categories.

Scripting
---------

Results go to the screen unless ``-o`` is given, and messages to the error
stream, so commands combine with other tools. For example, to run the same
comparison at every level of a profile:

.. code-block:: bash

   for level in Phylum Class Order Family Genus; do
       stamp-cli two-groups --profile coal.spf --metadata coal.metadata.tsv \
           --field "Core vs. Water" --group1 Core --group2 "Produced water" \
           --profile-level "$level" --correction benjamini-hochberg \
           --output "core_vs_water.$level.tsv"
   done
