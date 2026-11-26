Input files
===========

STAMP reads two files: a **profile**, which says how many sequences of each
feature each sample contains, and optionally a **metadata** file, which
says what is known about each sample and so defines groups of samples.

Every file STAMP reads can be **gzip-compressed** (``profile.spf.gz``,
``metadata.tsv.gz``, ``H01.k2report.gz``): profiles, metadata, the outputs
of profilers and the other formats below, in the window and on the
command line, and in directories of profiles. STAMP tells a
compressed file by its contents rather than its name, and leaves ``.gz`` out
of the sample names it takes from file names.

Profiles
--------

A STAMP profile (``.spf``) is a tab-separated text file, as written by a
spreadsheet's "Save as tab-delimited text". The first row holds headings.
The first columns give the hierarchy of each feature, from the most general
level to the most specific; the remaining columns hold one sample each.

.. code-block:: text

   Phylum          Genus           Sample1  Sample2  Sample3
   Firmicutes      Clostridium     120      98       15
   Firmicutes      Ruminococcus    45       60       3
   Bacteroidetes   Bacteroides     310      402      880
   Bacteroidetes   unclassified    12       8        30

* A **feature** is one row at the level being analysed: a genus, a gene
  family, a pathway. Analysing the profile above at the *Phylum* level adds
  up the rows of each phylum.
* The values are usually **counts of sequences** (reads, or genes). STAMP
  turns them into proportions itself. Some analyses need counts:
  :ref:`rarefaction <plot-rarefaction>` and rarefying for the diversity
  plots. Profiles of proportions or normalised values can be analysed with
  every test, but not with those.
* The hierarchy must be strict: each name must have one parent. A genus
  listed under two different families is reported as an error.
* Names containing **unclassified** stand for sequences that could not be
  assigned at that level. How they are treated is a choice (below).

Unclassified sequences
^^^^^^^^^^^^^^^^^^^^^^

On the left of the window, *Unclassified* offers three choices:

* **Retain unclassified reads**: unclassified categories are features like
  any other and are tested.
* **Remove unclassified reads**: they are dropped altogether, and
  proportions are taken among the classified sequences only.
* **Use only for calculating frequency profiles**: they are not tested,
  but they still count in the total of each sample, so proportions are
  taken among all sequences.

The plots of whole communities (diversity, distances, ordination,
rarefaction) never count unclassified categories as features, since they
are not one taxon or function.

Parent level
^^^^^^^^^^^^

Proportions are normally taken against the **entire sample**. Choosing a
*parent level* takes them against the parent category instead: with the
parent level set to *Phylum*, each genus is a proportion of its phylum.
This asks a different question: whether a genus makes up a larger share of
its phylum, rather than of the whole community.

Metadata
--------

The metadata file is also tab-separated. Its first column holds the sample
names, exactly as in the profile; each further column is a **field**.

.. code-block:: text

   Sample   Site    Treatment   Subject   pH
   S1       gut     control     A         6.8
   S2       gut     drug        A         6.5
   S3       skin    control     B         5.2

* A field with names (here *Site*, *Treatment*) defines groups: choose it as
  the *Group field* to compare its groups.
* A field of numbers (here *pH*) can be correlated with every feature in
  the :ref:`correlation analysis <correlation>`. Samples without a number
  are left out.
* A field naming the subject or site each sample came from (here *Subject*)
  pairs samples for the :ref:`paired tests <paired-tests>`.

Samples in the profile that are missing from the metadata, or the other
way round, are reported when the file is loaded.

Other formats
-------------

**File > Load profile file** also opens these formats directly, converting
them as they are read. STAMP tells the formats apart by their contents, not
by the file extension:

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Format
     - Notes
   * - BIOM (JSON or HDF5, ``.biom``)
     - From QIIME and similar tools. STAMP asks which observation metadata
       field holds the hierarchy (usually *taxonomy*).
   * - mothur
     - A taxonomy file with a groups file, and optionally a names file.
       STAMP asks for the other files after the first.
   * - SingleM condensed profile (usually ``.tsv``)
     - The output of ``singlem pipe -p`` or ``singlem summarise``: a
       tab-separated table whose first line names the columns ``sample``,
       ``coverage`` and ``taxonomy``, with any number of samples in one
       file. The variant written with
       ``--output-taxonomic-profile-with-extras`` is read too (its
       ``coverage`` column is used). See :ref:`singlem-profiles`.
   * - Sandpiper profile (``.json``)
     - The condensed profile of one run from the Sandpiper web service
       (``https://sandpiper.qut.edu.au/api/condensed/<run>``, GTDB or GlobDB
       taxonomy). Each file is one sample, named by its ``sample_name`` (or
       else the file name).
   * - mOTUs profile
     - ``motus profile`` (with or without ``-c``, ``-p``, ``-u``, ``-q``,
       ``-k`` or ``-A``) and ``motus merge`` tables of mOTUs 2 and 3, and the
       counts and ``.relab`` files of mOTUs 4. See :ref:`motus-profiles`.
   * - MetaPhlAn profile
     - ``metaphlan -t rel_ab`` or ``-t rel_ab_w_read_stats``,
       ``merge_metaphlan_tables.py`` and ``sgb_to_gtdb_profile.py`` output.
       See :ref:`metaphlan-profiles`.
   * - Kraken report
     - ``kraken2 --report`` (also with ``--report-minimizer-data``), Kraken 1
       reports, and MPA-style reports (``--use-mpa-style``, KrakenTools'
       ``kreport2mpa.py`` and ``combine_mpa.py``). See :ref:`kraken-reports`.
   * - Bracken output
     - The abundance table (``.bracken``), ``combine_bracken_outputs.py``
       tables, and the report Bracken writes in Kraken's layout (``-w``).
   * - HUMAnN tables
     - ``<sample>_genefamilies.tsv``, ``<sample>_pathabundance.tsv`` and
       ``<sample>_pathcoverage.tsv``, and the tables HUMAnN's tools make
       from them (joined, renormalised, regrouped, renamed, split). See
       :ref:`humann-tables`.

Profilers usually write one file per sample. Choose the files of all
samples together in the file dialog (they must come from one tool), or load
their directory.

.. _supported-versions:

Supported versions
^^^^^^^^^^^^^^^^^^

The readers follow each tool's own source and documentation, as of
November 2026. The example files (below) are illustrative, not real output.

.. list-table::
   :header-rows: 1
   :widths: 16 20 64

   * - Tool
     - Versions
     - Outputs read
   * - SingleM
     - 0.13 and later
     - ``pipe -p``; ``summarise --output-taxonomic-profile`` (and
       ``-with-extras``)
   * - Sandpiper
     - 1.0 and later
     - condensed profile (JSON)
   * - mOTUs
     - 2.x, 3.x, 4.x
     - ``profile`` (default, ``-c``, ``-p``, ``-u``, ``-q``, ``-k``, ``-A``;
       in 4.x the counts and ``.relab`` files); ``merge``
   * - MetaPhlAn
     - 3.x, 4.x (2.x without its header)
     - ``-t rel_ab``, ``-t rel_ab_w_read_stats``;
       ``merge_metaphlan_tables.py``; ``sgb_to_gtdb_profile.py``
   * - Kraken
     - 1.x, 2.x
     - ``--report`` (6 columns, or 8 with ``--report-minimizer-data``);
       ``--use-mpa-style``; KrakenTools ``kreport2mpa.py``, ``combine_mpa.py``
   * - Bracken
     - 2.x
     - ``.bracken`` tables; ``combine_bracken_outputs.py``; ``-w`` reports
   * - HUMAnN
     - 3.x (and 2.x, whose tables are the same)
     - gene families, pathway abundance, pathway coverage; tables from
       ``humann_join_tables``, ``humann_renorm_table``,
       ``humann_regroup_table``, ``humann_rename_table``,
       ``humann_split_stratified_table``

Not read: KrakenUniq reports, KrakenTools ``combine_kreports.py`` tables,
CAMI-format output (mOTUs ``-C``, MetaPhlAn ``--CAMI_format_output``), the
Krona pages SingleM can write, and HUMAnN 4, which is still in alpha
(tables in its layout load as far as they follow HUMAnN 3's, but have not
been checked against real output).

.. _profile-directory:

Loading a directory of profiles
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**File > Load profile directory** loads every profile directly in a
directory (not in its subdirectories) as one profile, one sample per file
or per column of a merged table, its samples in the order of the file
names. Other files in the directory, such as reads or logs, are skipped and
named in a message; only their first 64 KB are read to tell what they are.
A sample may be in only one file.

A directory may hold profiles of several kinds: HUMAnN writes three tables
for each sample, and a Bracken run leaves Kraken's reports beside
Bracken's. STAMP then asks which kind to load, offering the kind with the
most files first; the files of the other kinds are skipped. Files of two
kinds are never combined.

On the command line, ``--profile`` takes any of these files, or a directory
of them, in the same way; ``--profile-format`` chooses the kind to read from
a directory that holds several (see :ref:`cli-convert` for the names).

The command line also converts files to STAMP profiles (see
:ref:`cli-convert`): the outputs of the profilers above, BIOM tables,
mothur files, and **IMG/M COG** profiles, to which COG categories are added.

.. _profiler-rules:

How profilers' outputs become profiles
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

The same rules apply to every profiler:

* A profile holds each row's **own** value; STAMP adds the rows up to get
  higher levels. Tools that list every rank with the value of the whole
  clade (MetaPhlAn, mOTUs with ``-A``, MPA-style Kraken reports) are turned
  into own values: a clade's own value is its value less those of the
  clades listed below it. Differences smaller than the rounding of the
  printed numbers are taken as zero, so rounding makes no rows.
* A value that stops at a rank above the lowest (reads Kraken assigned to a
  genus, abundance MetaPhlAn left at a genus) goes into a row padded with
  ``unclassified <prefix><taxon>``, as for SingleM (below).
* What a tool could not place at all (Kraken's ``unclassified`` reads and
  reads assigned to ``root``; MetaPhlAn's ``UNCLASSIFIED`` or ``UNKNOWN``;
  mOTUs' ``unassigned``) is the row ``unclassified Root``, at every level.
  The *Unclassified* choice decides whether it is tested.
* A rank missing from a lineage (NCBI genera without a family, say) is
  filled with ``<parent> (no <rank>)``, for example ``Bacillales (no
  family)``.
* Read counts are kept as whole numbers; relative abundances and coverages
  to six significant digits.
* A file that holds one sample and does not name it gives the sample the
  file's name, without its extensions and a tool's usual suffix
  (``gut1_bracken_species.kreport`` is sample ``gut1``,
  ``S3_profile.txt`` is ``S3``).

.. _singlem-profiles:

SingleM and Sandpiper profiles
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

SingleM, and Sandpiper (which holds SingleM profiles of public
metagenomes), give each taxon of the GTDB taxonomy a **coverage**: an
estimate of how many times over its genomes were sequenced. A taxon's
coverage is its **own**, not including that of its descendants: coverage
given to a family is what SingleM could not place in any of its genera.
STAMP makes a profile with seven levels, *Domain*, *Phylum*, *Class*,
*Order*, *Family*, *Genus* and *Species*:

* Taxa are named without their GTDB rank prefixes (``Bacillota``, not
  ``p__Bacillota``).
* Coverage that stops at a higher rank goes into a row padded with
  ``unclassified <taxon>`` at every lower level, the taxon written with its
  rank prefix. Coverage of the family ``f__Oscillospiraceae`` becomes:

  .. code-block:: text

     Bacteria  Bacillota  Clostridia  Oscillospirales  Oscillospiraceae  unclassified f__Oscillospiraceae  unclassified f__Oscillospiraceae

  The prefix keeps these names distinct where GTDB uses one name at two
  ranks (``UBA644`` is both an order and a family). As the names contain
  *unclassified*, the *Unclassified* choice of the window (or
  ``--unclassified``) decides whether they are tested. Coverage that was
  not placed in any domain goes into ``unclassified Root`` rows.
* A name found under two different parents at the same rank (a genus in two
  families, which can happen when profiles come from different GTDB
  releases) is written as ``name (parent)``, so the hierarchy stays strict.
* Values are kept to six significant digits.

The values are coverages, not counts of sequences. Analyses that compare
proportions work with them, but those that need counts do not:
rarefaction and rarefying, and the count-based tests of two samples
(Fisher's exact test, the G-test and the chi-square test treat the values
as numbers of sequences). Prefer the group tests, which compare
proportions between samples.

.. _motus-profiles:

mOTUs profiles
^^^^^^^^^^^^^^

* Without ``-q``, mOTUs gives names at one rank only: a profile with one
  level, *mOTU* (``Kandleria vitulina [ref_mOTU_v31_00001]``) or the rank
  chosen with ``-k`` (*Genus*, ...).
* With ``-q`` (or ``-A``), and always in mOTUs 4, each mOTU has a lineage:
  levels *Kingdom* to *Species* (NCBI taxonomy, mOTUs 2 and 3) or *Domain*
  to *Species* (GTDB, mOTUs 4). The species keeps the mOTU's id
  (``Escherichia coli [ref_mOTU_v31_00095]``), so each mOTU stays a species
  of its own.
* Values are relative abundances, or with ``-c`` (and in mOTUs 4's counts
  file) counts of inserts.
* Profiles with and without lineages cannot be loaded together.

.. _metaphlan-profiles:

MetaPhlAn profiles
^^^^^^^^^^^^^^^^^^

* Levels *Kingdom*, *Phylum*, *Class*, *Order*, *Family*, *Genus*,
  *Species* and, for MetaPhlAn 4, *SGB* (the species-level genome bin,
  ``t__SGB4951``). Names are kept as MetaPhlAn writes them
  (``Roseburia_intestinalis``). GTDB profiles from ``sgb_to_gtdb_profile.py``
  have levels *Domain* to *Species*.
* Values are relative abundances (percent, adding up to 100 with
  ``UNCLASSIFIED``), or with ``-t rel_ab_w_read_stats`` the estimated
  numbers of reads, which are used when they are there.
* A profile is named by its ``#SampleID`` line, unless that is MetaPhlAn's
  default (``Metaphlan_Analysis``), and then by its file name; a merged
  table by its column headings.

.. _kraken-reports:

Kraken and Bracken
^^^^^^^^^^^^^^^^^^

* A Kraken report gives each taxon its **direct** reads (the third
  column), which are the values. Levels are *Domain*, *Phylum*, *Class*,
  *Order*, *Family*, *Genus* and *Species*, and *Strain* when reads were
  assigned below a species (rank ``S1``; deeper ranks such as ``S2`` are
  counted with their ``S1``).
* Taxa without one of these ranks (``R1``, ``D1``, ``G1``, kingdoms) are
  not levels; their reads go to the ranked taxon above them. Taxa with no
  domain above them (viruses, in recent NCBI taxonomies) take the top
  unranked taxon below ``root``, such as ``Viruses``, as their domain.
* Bracken's ``.bracken`` table gives the reads it estimated for one rank
  only: a profile of one level, *Species* (or the rank Bracken was run at).
  Its ``-w`` report gives the same reads with their lineages; STAMP reads a
  Bracken report by its clade reads (a taxon's reads less those of the taxa
  listed below it), since Bracken 3.1 leaves the direct reads of a
  genus-level report at zero.
* Bracken leaves out unclassified reads; Kraken's reports have them.

.. _humann-tables:

HUMAnN tables
^^^^^^^^^^^^^

* **Gene families**: a family's strata (``|g__Bacteroides.s__Bacteroides_dorei``)
  add up to its community total, so a table with strata has the levels
  *Gene family*, *Genus* and *Species*; the ``unclassified`` stratum, and
  any part of the total in no stratum, is ``unclassified <family>`` at the
  lower levels. Without strata, the profile has the families only.
* **Pathway abundance**: HUMAnN computes a pathway's community abundance on
  its own, and it is not the sum of the pathway's strata. The profile holds
  the community values, one level, *Pathway*. A table of strata alone (the
  ``_stratified.tsv`` of ``humann_split_stratified_table``) gives *Pathway*,
  *Genus* and *Species* instead.
* **Pathway coverage**: coverage cannot be added up, so the profile holds
  the community coverage only; a table of strata alone is refused.
* ``UNMAPPED``, ``UNINTEGRATED`` and ``UNGROUPED`` are features like any
  other; leave them out with the feature selection if they should not be
  tested.
* Sample names are the column headings without ``_Abundance-RPKs``,
  ``_Abundance``, ``_Coverage`` and the like.

Example data
------------

**File > Open example data** loads one of these data sets:

* **Enterotypes**: genus-level profiles of 39 human faecal metagenomes and
  their enterotypes (proportions). Good for comparing groups.
* **Coal seam gas**: taxonomic profiles of 44 samples of coal cores, drill
  cuttings and produced water (An et al. 2013, *Environ. Sci. Technol.*
  47:10708), as counts of sequences with a full taxonomy. Good for the
  diversity, rarefaction and community plots, which need counts.
* **Glacier-fed streams**: SingleM profiles (GTDB R232, most from
  Sandpiper) of 158 metagenomes of the sediment of glacier-fed streams in
  nine mountain ranges (Michoud et al. 2025, *Nature Microbiology* 10:217),
  Domain to Species, an upstream and a downstream reach of each stream,
  with the measurements made at each reach. Values are coverage, not
  counts. Good for the paired tests of two groups, comparing many groups
  of unequal size, correlations with measurements, the sample filter, the
  group heatmap, the map and co-occurrence; see
  :doc:`example_glacier_streams`. It opens in the *Two groups* analysis
  with the downstream reaches compared with the upstream reaches, paired
  by ``Glacier``.

More examples are in the ``examples`` directory of the source repository.
``examples/input_file_formats`` holds small example inputs in every format
STAMP reads (SingleM, Sandpiper, mOTUs, MetaPhlAn, Kraken 2, Bracken,
HUMAnN, BIOM, mothur and IMG/M COG), all of one made-up community of ten
gut metagenomes, five healthy and five with IBD, with a metadata file
(``metadata.tsv.gz``, field *Group*). They are illustrative: the numbers
were drawn at random by ``make_examples.py`` in that directory, in each
tool's exact format. Every file there is gzip-compressed, and STAMP reads
them as they are. Each tool's directory also holds real output of that tool in a
``stool_SRR17283923`` folder: one human stool metagenome (run SRR17283923,
subsampled to 1,000,000 read pairs), profiled with SingleM
0.21.4, mOTUs 3.1 and 4.1, MetaPhlAn 4.1.1, HUMAnN 3.9, Kraken 2.17.2 and
Bracken 3.1 (and Sandpiper's profile of the whole run). A single sample
cannot be analysed on its own; these files show what each tool writes.
