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
``eggerthellaceae.spf``, ``eggerthellaceae_mgr_species.spf`` and
``eggerthellaceae.metadata.tsv`` (``examples/Eggerthellaceae``, SingleM
profiles of gut metagenomes of mammals with curated host metadata; see
:doc:`example_eggerthellaceae`).

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
``Sample filter (Host_order = Perissodactyla): kept 1202 of 1307 samples.``

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

   stamp-cli multi-groups --profile eggerthellaceae.spf --metadata eggerthellaceae.metadata.tsv \
       --field Host_group --profile-level Genus \
       --where "Host_order=Perissodactyla" \
       --where-not "Host_group=Feral horse (Sable Island)"

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

.. _cli-co-occurrence:

Co-occurrence
-------------

::

   stamp-cli co-occurrence --profile eggerthellaceae.spf --profile-level Genus \
       --metadata eggerthellaceae.metadata.tsv --where Host_order=Perissodactyla \
       --top 40 --feature Equivita --feature Tapirivita --clr \
       --mark-min-coefficient 0.3 --plot genera.png --output genera.tsv

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
:ref:`ordination-colours`. For example, the host groups (but the feral
horses of Sable Island) by Bray-Curtis distance, coloured by the abundance
of *Equivita* on a log scale and shaped by captivity::

   stamp-cli multi-groups --profile eggerthellaceae.spf --metadata eggerthellaceae.metadata.tsv \
       --field Host_group --where-not "Host_group=Feral horse (Sable Island)" \
       --profile-level Genus --unclassified remove \
       --plot pcoa.png --plot-type pcoa \
       --color-by feature:Equivita --color-scale log10 --shape-by Captivity

``--color-by SOURCE``
   ``group`` (default); ``field:NAME``, a metadata field: a colour scale if
   its values are numbers (``field:Latitude``), otherwise a colour per
   value (``field:Host_order``); ``category:NAME``, a colour per value even
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
three genera in the host groups of the perissodactyls on a log axis, with
every sample as a point, the groups ordered by their median::

   stamp-cli multi-groups --profile eggerthellaceae.spf --metadata eggerthellaceae.metadata.tsv \
       --field Host_group --where Host_order=Perissodactyla \
       --profile-level Genus --unclassified remove \
       --plot box.png --plot-type box --plot-feature Equivita \
       --plot-feature Tapirivita --plot-feature Gorillivita \
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
   ``--box-order-field Host_order``.
``--bar-log``
   A log10 axis for the bar plot of one feature in every sample.

.. _cli-prevalence-abundance:

Prevalence-abundance plot
^^^^^^^^^^^^^^^^^^^^^^^^^

``--plot-type prevalence-abundance`` (``two-groups`` and ``multi-groups``)
draws the :ref:`prevalence-abundance plot <plot-prevalence-abundance>` of
every feature of the profile level, whatever the filters (unless
``--prevalence-only-filtered``). For example, the genera of every host group
but the unknown and environmental samples, three of them named::

   stamp-cli multi-groups --profile eggerthellaceae.spf --metadata eggerthellaceae.metadata.tsv \
       --field Host_group --where-not Host_group=Unknown \
       --where-not Host_group=Environmental \
       --profile-level Genus --unclassified remove \
       --plot prevalence.png --plot-type prevalence-abundance \
       --highlight Equivita --highlight Tapirivita --highlight Gorillivita \
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
example, a world map of the host groups, one marker per site sized by its
number of samples, and the great apes alone coloured by the abundance of
*Gorillivita* (the area is then found around them, at 1:50m)::

   stamp-cli multi-groups --profile eggerthellaceae.spf --metadata eggerthellaceae.metadata.tsv \
       --field Host_group --profile-level Genus \
       --plot map.png --plot-type map --map-extent world

   stamp-cli multi-groups --profile eggerthellaceae.spf --metadata eggerthellaceae.metadata.tsv \
       --field Country --where "Host_group=Great apes" --profile-level Genus \
       --plot apes.png --plot-type map --color-by feature:Gorillivita

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
prevalence of species in each host species, the hosts ordered by their
order::

   stamp-cli multi-groups --profile eggerthellaceae_mgr_species.spf \
       --metadata eggerthellaceae.metadata.tsv --unclassified frequency-only \
       --field Host_species --plot prevalence.png --plot-type group-heatmap \
       --heatmap-statistic prevalence --heatmap-order-field Host_order \
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

   stamp-cli convert mgrast table.tsv --heading Level1 --heading Level2 -o profile.spf
   stamp-cli convert comet sample1.txt sample2.txt -o profile.spf
   stamp-cli convert rita sample1.txt sample2.txt -o profile.spf
   stamp-cli convert mothur --taxonomy x.taxonomy --groups x.groups --names x.names -o profile.spf
   stamp-cli convert biom table.biom --metadata-field taxonomy -o profile.spf
   stamp-cli convert cog img_profile.tsv -o profile.spf
   stamp-cli convert singlem condensed_profile.tsv -o profile.spf
   stamp-cli convert sandpiper runs/*.json -o profile.spf

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
* **singlem**: SingleM condensed profiles (``singlem pipe -p`` or
  ``singlem summarise``); a file may hold many samples, and several files
  may be given.
* **sandpiper**: Sandpiper condensed profiles (JSON), one file per sample.
  Converting 1,307 runs takes about two seconds.

SingleM and Sandpiper profiles become seven levels from *Domain* to
*Species*, with coverages as values; :ref:`singlem-profiles` describes the
names given to coverage that stops at a higher rank.

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
