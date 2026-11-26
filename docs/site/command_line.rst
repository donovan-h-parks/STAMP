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
``co-occurrence``   Correlate features with each other across samples
``convert``         Convert another tool's output to a STAMP profile
==================  =======================================================

``stamp-cli <COMMAND> --help`` lists every option of a command, with its
default.

.. _cli-progress:

Progress
--------

Steps that take a while (reading a large profile, testing every feature,
computing distances, each permutation test, NMDS iterations, ANCOM, drawing
the plot and writing the results table) show their progress on standard
error: a progress bar with a count where the work can be counted, for
example::

   / PERMANOVA [###########                   ] 372/999 0:01

and otherwise a spinner with what is being done. Progress appears only for a
step still running after half a second, is drawn over one line and cleared
when the step ends, and is shown only when standard error is a terminal:
nothing is added when the output goes to a file, a pipe or a log, so scripts
see exactly what they saw before. ``--quiet`` (``-q``), given with any
command, turns it off altogether.

The results table is written to the file row by row as it is produced, so
that tables with a column for every sample of a large study (millions of
cells) need little memory.

Describing a profile
--------------------

Start with ``info`` to see the level names and metadata fields to pass to
the other commands::

   stamp-cli info --profile coal.spf --metadata coal.metadata.tsv

The examples on this page use the data sets in the ``examples`` directory of
the source repository: ``coal.spf`` (``examples/CoalSeamGas``),
``Enterotypes.profile.spf`` (``examples/EnterotypesArumugam``), and
``glacier_streams.spf`` and ``glacier_streams.metadata.tsv``
(``examples/GlacierFedStreams``, SingleM profiles of the sediment of
glacier-fed streams, an upstream and a downstream reach of each, with the
measurements made at each; see :doc:`example_glacier_streams`).

Options common to the analyses
------------------------------

``-p, --profile FILE``
   The profile: a STAMP profile, a file written by SingleM, Sandpiper,
   mOTUs, MetaPhlAn, Kraken, Bracken or HUMAnN, or a directory of such files
   (see :ref:`profile-directory`), read without first being converted.
   Files may be gzip-compressed (``.gz``).
``--profile-format FORMAT``
   The kind of profile to read from a directory that holds several kinds
   (HUMAnN's three tables per sample, Kraken and Bracken reports side by
   side); a file given as ``--profile`` must be of this kind. One of
   ``singlem``, ``sandpiper``, ``motus``, ``metaphlan``, ``kraken``,
   ``kraken-mpa``, ``bracken-report``, ``bracken``, ``humann-genefamilies``,
   ``humann-pathabundance`` and ``humann-pathcoverage``.
``-m, --metadata FILE``
   The metadata (all commands but ``two-samples``), plain or
   gzip-compressed.
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
``--where CONDITION``, ``--where-not FIELD=VALUE``
   Choose the samples by their metadata (all commands that read metadata,
   including ``info``); see :ref:`cli-sample-filter`.
``--correction``
   ``none`` (default), ``benjamini-hochberg``, ``bonferroni``,
   ``holm-bonferroni``, ``sidak`` or ``storey``.
``--significance P``
   Keep only features whose corrected p-value is at most P.
``--seed N``, ``--replicates N``, ``--pseudocount X``
   As in the preferences of the window (see :doc:`main_window`).

.. _cli-sample-filter:

Choosing samples by their metadata
----------------------------------

``--where`` keeps only the samples whose metadata meet a condition, and
``--where-not`` leaves out the samples with a value. Samples left out take
no part in anything: groups, group statistics, distances, ordinations and
plots. Groups none of whose samples are kept disappear. The number of
samples kept is reported, for example
``Sample filter (Reach = Upstream; Mountain_range is not Rwenzori): kept 76 of 158 samples.``

``--where FIELD=VALUE``
   Keep samples whose value of FIELD is VALUE. Repeat it with the same field
   for alternatives: ``--where Nationality=japanese --where
   Nationality=french`` keeps the samples of either. Conditions on
   different fields must all hold.
``--where "FIELD<X"``, ``"FIELD<=X"``, ``"FIELD>X"``, ``"FIELD>=X"``
   Keep samples whose value of FIELD is a number in that relation to X.
   Samples without a number in the field are left out. Quote these, since
   ``<`` and ``>`` mean something to the shell.
``--where-not FIELD=VALUE``
   Leave out samples whose value of FIELD is VALUE; repeat for more.

The condition is split at its first ``=``, ``<`` or ``>``, so the value may
contain any character, spaces included (quote it); a field name cannot
contain those three characters. Spaces around the operator are ignored. A
field that is not in the metadata, or a value that no sample has, is an
error that lists the fields or values, which catches misspellings.

::

   stamp-cli multi-groups --profile glacier_streams.spf --metadata glacier_streams.metadata.tsv \
       --field Mountain_range --profile-level Family \
       --where "Reach=Upstream" \
       --where-not "Mountain_range=Rwenzori"

``--exclude-sample`` can be combined with the filter. The ``two-samples``
command names its two samples and reads no metadata, so it has no filter.

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
   ``paired-t``, ``wilcoxon``, and the tests of presence or absence
   ``fisher-presence`` and ``mcnemar`` (paired).
``--pair-field FIELD``
   For ``paired-t``, ``wilcoxon`` and ``mcnemar``: the field that pairs the
   samples (see :ref:`paired-tests`).
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
takes ``--test anova`` (default), ``welch-anova``, ``kruskal-wallis``,
``friedman`` or ``chi-square-presence``; ``post-hoc`` takes ``--test tukey-kramer``
(default), ``games-howell``, ``scheffe``, ``welch``, ``dunn`` or
``wilcoxon``, and ``--feature`` (repeat for several features).

``multi-groups`` ends its table with the column *Equal spread p-value
(Brown-Forsythe)*, after STAMP 2's columns, and reports on standard error
how many features differ in spread between the groups (see
:ref:`equal-spread`). If that is clearly more than chance gives and the
test is ANOVA, it suggests Welch's ANOVA with the Games-Howell post-hoc
test:

::

   stamp-cli multi-groups --profile glacier_streams.spf --metadata glacier_streams.metadata.tsv \
       --field Mountain_range --profile-level Family > ranges.tsv
   Brown-Forsythe test: the spread differs between the groups for 814 of 1936 features (42%; p < 0.05, 5% expected by chance).
   note: ANOVA assumes the groups are equally spread out; consider --test welch-anova (and the games-howell post-hoc test)

``friedman`` (and the ``wilcoxon`` post-hoc test) are for matched groups
and need ``--pair-field FIELD``, the metadata field that names the subject
of each sample; subjects without exactly one sample in every group are left
out, and the number matched is reported.

With ``--plot-type pcoa`` and three or more groups, PERMANOVA between
every pair of groups is reported after the community tests, with
Holm-corrected p-values.

Correlation
-----------

::

   stamp-cli correlation --profile soil.spf --metadata soil.metadata.tsv \
       --field pH --method spearman --correction benjamini-hochberg \
       --min-correlation 0.5

``--method`` is ``spearman`` (default) or ``pearson``.
``--min-correlation`` keeps features whose coefficient is at least that far
from zero. ``--group-field`` colours the points of the correlation scatter
plot by group. ``--plot-type alpha-diversity`` plots the diversity of each
sample (``--alpha-metric``, ``--rarefy``) against the field instead, with
their correlation.

.. _cli-co-occurrence:

Co-occurrence
-------------

::

   stamp-cli co-occurrence --profile glacier_streams.spf --profile-level Family \
       --metadata glacier_streams.metadata.tsv --where Reach=Downstream \
       --top 40 --feature JACQAP01 --feature RBC074 --clr \
       --mark-min-coefficient 0.3 --plot families.png --output families.tsv

Correlates the features with each other across the samples: every sample
of the profile, or with ``--metadata`` those that ``--where``,
``--where-not`` and ``--exclude-sample`` keep, or only those of one group
(``--field`` and ``--group``). The table has one row per pair of features
(``Feature 1``, ``Feature 2``, the coefficient, ``p-value``, ``q-value
(Benjamini-Hochberg)``), most significant first; ``--plot`` draws the
:ref:`co-occurrence heatmap <plot-co-occurrence>`.

``--method``
   ``spearman`` (default) or ``pearson``.
``--clr``
   Correlate centred log ratios instead of proportions (recommended; see
   :ref:`co-occurrence`).
``--top N``, ``--rank-by``
   The N features (default 30) of highest ``mean-abundance`` (default) or
   ``prevalence``.
``--feature FEATURE``
   A feature to include whatever its rank (repeat for several); with
   ``--top 0`` only these are correlated.
``--min-feature-prevalence PERCENT``
   Leave out features present in fewer than this percentage of the samples
   (unless named).
``--with FEATURE``, ``--partners N``
   Correlate one feature with every other feature instead; the table lists
   every correlation, strongest first, and the plot shows the N strongest
   (default 30) as bars.
``--against FEATURE``
   With ``--with``: the plot is a scatter plot of the two features, a point
   for each sample, with their correlation.
``--q-threshold Q``, ``--mark-min-coefficient R``, ``--no-marks``
   Asterisks mark correlations with a Benjamini-Hochberg q-value below Q (default 0.05) and,
   optionally, at least R from zero; or none.
``--order``, ``--no-dendrograms``, ``--show-values``, ``--color-map``
   Order of the features (``cluster``, default, or ``original``), the
   dendrograms, the coefficients written in the cells, and the colour map
   (``red-to-blue`` by default, red for positive).
``--cell-size INCHES``, ``--fixed-size``
   The figure is sized to give each cell 0.16 inches unless
   ``--fixed-size`` asks for ``--plot-width`` by ``--plot-height``.

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
   ``p-value-histogram``, ``multiple-comparison``, ``heatmap``,
   ``post-hoc``, ``rarefaction``, ``volcano``, ``composition``,
   ``alpha-diversity``, ``pcoa``, ``distance-box``, ``correlation-scatter``,
   ``nmds``, ``shared-features``, ``rank-abundance``, ``ancom``,
   ``mantel``, ``group-heatmap``, ``prevalence-abundance``, ``map``. The
   default is
   the extended error bar plot for two samples or groups and the heatmap
   for several groups.
``--plot-feature FEATURE``
   The feature of the bar, box and correlation scatter plots (default: the
   most significant). Repeat it for a :ref:`box plot of several features
   <cli-box-plots>`.
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
   The diversity measure, and the distance of the community plots:
   ``bray-curtis`` (default), ``jaccard``, ``aitchison`` or ``euclidean``
   (a PCoA of Euclidean distances is a PCA of the proportions).

With more than twenty groups the group colours repeat, and the command
prints a warning.

.. _cli-ordination-colours:

Colouring and shaping PCoA and NMDS plots
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

``--plot-type pcoa`` and ``nmds`` (``two-groups`` and
``multi-groups``) colour the samples by group unless asked otherwise; see
:ref:`ordination-colours`. For example, the mountain ranges (but the
single stream of the Rwenzori) by Bray-Curtis distance, coloured by the
abundance of Acidobacteriaceae on a log scale and shaped by reach::

   stamp-cli multi-groups --profile glacier_streams.spf --metadata glacier_streams.metadata.tsv \
       --field Mountain_range --where-not Mountain_range=Rwenzori \
       --profile-level Family --unclassified remove \
       --plot pcoa.png --plot-type pcoa \
       --color-by feature:Acidobacteriaceae --color-scale log10 --shape-by Reach

``--color-by SOURCE``
   ``group`` (default); ``field:NAME``, a metadata field: a colour scale if
   its values are numbers (``field:Latitude``), otherwise a colour per
   value (``field:Country``); ``category:NAME``, a colour per value even
   for numbers; ``feature:NAME``, the abundance (% of the sample) of a
   feature at the profile level. The PCoA plot's community tests still
   compare the groups of ``--field``.
``--shape-by FIELD``
   A categorical metadata field for the marker shapes (ten shapes; the
   rarest values beyond nine share one, as *Other*). Without it, samples
   coloured by anything but their group are shaped by group.
``--color-scale``
   ``linear`` (default) or ``log10`` for colouring by a number; on the log
   scale zeros are drawn at half the smallest positive value.
``--color-map MAP``
   ``viridis`` (default), ``plasma``, ``magma``, ``cividis`` or any colour
   map of ``--heatmap-color-map``.
``--draw-order``
   ``high-on-top`` (default), ``low-on-top`` or ``sample-order``: which
   samples are drawn last when coloured by a number.
``--marker-size``, ``--opacity``
   Marker area in square points (default 30) and opacity from 0 to 1
   (default 1).

.. _cli-box-plots:

Box plots of several features
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

``--plot-type box`` (``two-groups`` and ``multi-groups``) draws the
:ref:`box plot <plot-box-features>` of one feature or, with
``--plot-feature`` repeated or ``--box-top N``, of several. For example,
three families in the upstream reaches of each mountain range on a log
axis, with every sample as a point, the groups ordered by their median::

   stamp-cli multi-groups --profile glacier_streams.spf --metadata glacier_streams.metadata.tsv \
       --field Mountain_range --where Reach=Upstream \
       --profile-level Family --unclassified remove \
       --plot box.png --plot-type box --plot-feature Acidobacteriaceae \
       --plot-feature Nitrospiraceae --plot-feature Leptolyngbyaceae \
       --box-log --box-points --box-counts --box-group-order median

``--box-top N``
   Without ``--plot-feature``, the N most significant features that pass
   the filters (default 1).
``--box-layout``
   ``grouped`` (default): the features along the axis, a box per group at
   each, coloured by group. ``panels``: one panel per feature;
   ``--box-independent-axes`` gives each panel its own value axis.
``--box-log``
   A log10 value axis; zeros are drawn on a line labelled 0 below a break in
   the axis. The quartiles and whiskers are those of the values themselves.
``--box-points``, ``--box-counts``, ``--box-horizontal``
   Every sample as a point; the number of samples after each group's name;
   horizontal boxes.
``--box-pairs``
   For ``two-groups`` with a paired test (``--test paired-t`` or
   ``wilcoxon`` and ``--pair-field``): a line between the boxes for every
   pair of samples, coloured by the group in which the value is higher.
   Several features get a panel each.
``--box-group-order``, ``--box-order-field FIELD``
   The order of the groups: ``given`` (default), ``alphabetical``,
   ``median`` (highest median of the first feature first) or ``field``, by
   the value of a metadata field most common among each group's samples
   (the default when ``--box-order-field`` is given), for example
   ``--box-order-field Hemisphere``.
``--bar-log``
   A log10 axis for the bar plot of one feature in every sample.

.. _cli-prevalence-abundance:

Prevalence-abundance plot
^^^^^^^^^^^^^^^^^^^^^^^^^

``--plot-type prevalence-abundance`` (``two-groups`` and ``multi-groups``)
draws the :ref:`prevalence-abundance plot <plot-prevalence-abundance>` of
every feature of the profile level, whatever the filters (unless
``--prevalence-only-filtered``). For example, the families of every
mountain range but the Rwenzori, three of them named::

   stamp-cli multi-groups --profile glacier_streams.spf --metadata glacier_streams.metadata.tsv \
       --field Mountain_range --where-not Mountain_range=Rwenzori \
       --profile-level Family --unclassified remove \
       --plot prevalence.png --plot-type prevalence-abundance \
       --highlight Acidobacteriaceae --highlight Nitrospiraceae --highlight Leptolyngbyaceae \
       --prevalence-table prevalence.tsv

``--prevalence-view``
   ``panels`` (default, one per group), ``combined`` (every group in one
   panel, or only ``--prevalence-group GROUP``) or ``specificity`` (each
   feature at the group where it is most prevalent, coloured when its
   prevalence there exceeds that in every other group by at least
   ``--prevalence-specificity`` percentage points, default 50).
``--prevalence-min-abundance PERCENT``
   Present means above zero and at least this proportion (default 0).
``--prevalence-mean``
   ``all`` (default): the mean over all of a group's samples; ``present``:
   only over the samples where the feature is present.
``--highlight FEATURE``
   Draw a feature in red and name it (may be repeated).
``--prevalence-table FILE``
   Also write, for every feature and group (and all samples together), the
   number of samples, the number where the feature is present, the
   prevalence, and the mean abundance over all samples and where present.

.. _cli-map:

Map of sample locations
^^^^^^^^^^^^^^^^^^^^^^^

``--plot-type map`` (``two-groups``, ``multi-groups`` and ``correlation``)
draws the :ref:`map of sample locations <plot-map>` from latitude and
longitude fields of the metadata. Markers are coloured and shaped with
``--color-by``, ``--shape-by``, ``--color-scale``, ``--color-map``,
``--draw-order``, ``--marker-size`` (the smallest markers) and ``--opacity``,
as for the :ref:`ordination plots <cli-ordination-colours>`; in the
correlation command the groups are those of ``--group-field``. For
example, a world map of the mountain ranges, one marker per site sized by
its number of samples, and the European Alps alone coloured by the
abundance of Acidobacteriaceae (the area is then found around them, at
1:50m)::

   stamp-cli multi-groups --profile glacier_streams.spf --metadata glacier_streams.metadata.tsv \
       --field Mountain_range --profile-level Family \
       --plot map.png --plot-type map --map-extent world

   stamp-cli multi-groups --profile glacier_streams.spf --metadata glacier_streams.metadata.tsv \
       --field Reach --where "Mountain_range=European Alps" --profile-level Family \
       --plot alps.png --plot-type map --color-by feature:Acidobacteriaceae

``--map-lat-field FIELD``, ``--map-lon-field FIELD``
   The fields holding latitude and longitude in decimal degrees (an ``N``,
   ``S``, ``E`` or ``W`` after the number is understood). Default: a field
   named ``latitude`` or ``lat``, and ``longitude``, ``lon``, ``long`` or
   ``lng`` (any case). Samples without both are left out, and counted.
``--map-extent EXTENT``
   ``auto`` (default: around the samples, or the world if they are spread
   widely), ``world``, ``europe``, ``africa``, ``asia``, ``north-america``,
   ``south-america``, ``oceania``, or ``WEST,EAST,SOUTH,NORTH`` in degrees,
   such as ``-10,30,35,60``.
``--map-projection``
   ``equal-earth`` (default), ``robinson``, ``equirectangular`` or
   ``mercator``.
``--map-resolution``
   ``auto`` (default: ``110m`` for maps wider than 90° of longitude,
   ``50m`` otherwise), ``110m`` or ``50m``: Natural Earth's outlines.
``--map-size-by SOURCE``
   ``count`` (default: the number of samples a marker stands for),
   ``fixed``, ``field:NAME`` (a numeric field) or ``feature:NAME`` (a
   feature's abundance, %); the mean at a site. ``--map-max-marker-size``
   is the area of the largest markers (default 400 square points) and
   ``--map-size-log`` sizes by the logarithm.
``--map-aggregate``, ``--no-map-aggregate``
   One marker per site and colour (default), or one per sample.
   ``--map-site-precision DEGREES`` sets the size of a site (default:
   about 3 points on the page); ``--map-jitter`` spreads the samples at a
   site when they are drawn one by one.
``--map-site-value``
   ``mean`` (default) or ``max``: how the values of the samples at a site
   colour its marker.
``--map-graticule``, ``--no-map-graticule``, ``--no-map-ocean``, ``--no-map-borders``
   Lines of longitude and latitude (drawn by default), the colour of the
   sea, and the outlines of countries and coasts.

The command line reports how many samples have no coordinates.

.. _cli-group-heatmap:

Group heatmap
^^^^^^^^^^^^^

``--plot-type group-heatmap`` (``two-groups`` and ``multi-groups``) draws
the :ref:`group heatmap <plot-group-heatmap>`: features against groups, each
cell a summary of the feature in the group's samples. It summarises the
features that pass the filters, so filter with care. For example, the
prevalence of phyla in each country, the countries ordered by their
mountain range::

   stamp-cli multi-groups --profile glacier_streams.spf \
       --metadata glacier_streams.metadata.tsv --profile-level Phylum \
       --field Country --plot prevalence.png --plot-type group-heatmap \
       --heatmap-statistic prevalence --heatmap-order-field Mountain_range \
       --heatmap-table prevalence.tsv

``--heatmap-statistic``
   ``mean`` (default), ``median``, ``max`` or ``prevalence`` (the percentage
   of the group's samples in which the feature is present).
``--heatmap-min-abundance PERCENT``
   For prevalence: present means above zero and at least this proportion.
``--heatmap-top N``
   The N features of highest mean abundance (default 50; 0 for all).
``--heatmap-scale``, ``--heatmap-log-floor``
   ``linear`` (default) or ``log10``; on the log scale, values below the
   floor (default 0.001) take its colour.
``--heatmap-no-grey-zeros``
   Colour zeros by the scale instead of grey.
``--heatmap-values``, ``--heatmap-value-format``, ``--heatmap-value-digits``
   Write each value in its cell, with a number of ``decimals`` (default) or
   ``significant`` digits (default 1).
``--heatmap-row-order``, ``--heatmap-column-order``, ``--heatmap-linkage``, ``--heatmap-no-dendrograms``
   Rows: ``cluster`` (default), ``mean-abundance``, ``alphabetical`` or
   ``original``. Columns: ``cluster``, ``category`` (by
   ``--heatmap-order-field``, the default when that is given),
   ``alphabetical`` or ``original``. The colour map, order, linkage and
   dendrograms apply to the heatmap of samples too.
``--heatmap-order-field FIELD``
   A metadata field shown as a coloured strip above the groups, each group
   taking the value most common among its samples (host order for groups of
   host species, say).
``--heatmap-color-map MAP``
   One of the colour maps of the window, such as ``yellow-to-orange-to-red``
   (default), ``blues``, ``red-to-blue`` or ``viridis``.
``--heatmap-cell-width``, ``--heatmap-cell-height``, ``--heatmap-fixed-size``
   The figure is sized to its cells (0.22 by 0.16 inches) unless
   ``--heatmap-fixed-size`` asks for ``--plot-width`` by ``--plot-height``.
``--heatmap-table FILE``
   Also write the value of every cell, features by groups.

.. _cli-convert:

Converting other formats
------------------------

``stamp-cli convert`` turns the output of other tools into a STAMP profile,
written to the screen or to ``-o FILE``:

.. code-block:: text

   stamp-cli convert profiles kraken_reports/ -o profile.spf
   stamp-cli convert profiles humann/ --format humann-pathabundance -o profile.spf
   stamp-cli convert profiles a_profile.txt b_profile.txt -o profile.spf
   stamp-cli convert mothur --taxonomy x.taxonomy --groups x.groups --names x.names -o profile.spf
   stamp-cli convert biom table.biom --metadata-field taxonomy -o profile.spf
   stamp-cli convert cog img_profile.tsv -o profile.spf
   stamp-cli convert singlem condensed_profile.tsv -o profile.spf
   stamp-cli convert singlem singlem_profiles/ -o profile.spf
   stamp-cli convert sandpiper runs/*.json -o profile.spf

* **profiles**: the outputs of SingleM, Sandpiper, mOTUs, MetaPhlAn,
  Kraken, Bracken or HUMAnN, recognised by their contents (see
  :ref:`supported-versions`), given as files or directories; files of one
  tool are combined, one sample per file or per column. ``--format`` (the
  names of ``--profile-format``) chooses the kind read from directories that
  hold several.
* **mothur**: a taxonomy file and a groups file, and optionally a names
  file.
* **biom**: a BIOM table (JSON or HDF5, optionally gzip-compressed).
  ``--metadata-field`` names the observation metadata holding the hierarchy,
  such as ``taxonomy`` or ``KEGG_Pathways``.
* **cog**: adds COG categories to an IMG/M COG profile.
  ``--assign-to-each-code`` gives a COG with several category codes to each
  of its categories.
* **singlem**: SingleM condensed profiles (``singlem pipe -p`` or
  ``singlem summarise``); a file may hold many samples, and several files
  or directories of them may be given (other files in a directory are
  skipped).
* **sandpiper**: Sandpiper condensed profiles (JSON), one file per sample,
  given as files or directories.
  Converting 1,307 runs takes about two seconds.

:ref:`profiler-rules` describes how the profilers' outputs become profiles:
their levels, values, and the names given to what stops at a higher rank or
was not classified.

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
