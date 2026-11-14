Input files
===========

STAMP reads two files: a **profile**, which says how many sequences of each
feature each sample contains, and optionally a **metadata** file, which
says what is known about each sample and so defines groups of samples.

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

**File > Load profile** also opens these formats directly, converting them
as they are read:

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
   * - SingleM condensed profile
     - The output of ``singlem pipe -p`` or ``singlem summarise``: columns
       ``sample``, ``coverage`` and ``taxonomy``, any number of samples in
       one file. The variant written with
       ``--output-taxonomic-profile-with-extras`` is read too (its
       ``coverage`` column is used).
   * - Sandpiper profile (``.json``)
     - The condensed profile of one run from the Sandpiper web service
       (``https://sandpiper.qut.edu.au/api/condensed/<run>``, GTDB or GlobDB
       taxonomy). Choose the files of all runs together in the file dialog;
       each is one sample, named by its ``sample_name`` (or else the file
       name).

The command line also converts the output of other tools to STAMP
profiles (see :ref:`cli-convert`):

* **MG-RAST** functional or taxonomic abundance tables,
* **CoMet** profiles, one file per sample,
* **RITA** classifications, one file per sample,
* **SingleM** condensed profiles and **Sandpiper** profiles,
* **IMG/M COG** profiles, to which COG categories are added.

The window's **File > Convert** menu converts the same formats, saves the
profile and opens it.

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

Example data
------------

**File > Open example data** loads one of two data sets:

* **Enterotypes**: genus-level profiles of 39 human faecal metagenomes and
  their enterotypes (proportions). Good for comparing groups.
* **Coal seam gas**: taxonomic profiles of 44 samples of coal cores, drill
  cuttings and produced water (An et al. 2013, *Environ. Sci. Technol.*
  47:10708), as counts of sequences with a full taxonomy. Good for the
  diversity, rarefaction and community plots, which need counts.

More examples are in the ``examples`` directory of the source repository.
