STAMP
=====

STAMP (statistical analysis of taxonomic and functional profiles) compares
profiles from metagenomic and amplicon studies: how much of each taxon,
gene family or pathway each sample contains. It tests which features differ
between samples or groups of samples, measures how large and how certain
each difference is, and draws the plots needed to explore and report the
results.

STAMP is meant for biologists. It asks you to choose between a handful of
well-understood statistical methods, and it reports effect sizes and
confidence intervals alongside p-values, so that you can judge whether a
difference is large enough to matter as well as whether it is real.

.. important::

   This is the documentation of **STAMP 3**, a rewrite of STAMP in Rust. It
   runs as a desktop application on Linux, Windows and macOS, as a command
   line program, and in a web browser, where your data never leaves your
   computer. It keeps the analyses and plots of STAMP 2 and adds new ones;
   the differences are listed in :doc:`changes`.

Running STAMP
-------------

1. :doc:`Install STAMP <installing>`, or open the browser version.
2. Load a **profile** (File > Load profile) and, to compare groups of
   samples, its **metadata** (File > Load metadata). File > Open example data
   loads a ready-made data set.
3. Choose the kind of analysis (two samples, two groups, multiple groups or
   correlation) and a statistical test, then explore the results in the
   plots and the results table.

The :doc:`quick_start` walks through an example. The
:doc:`statistics guide <statistics>` explains every test in plain terms,
with what to look for in the results, and the :doc:`plot guide <plots>` does
the same for every plot.

.. toctree::
   :maxdepth: 2
   :caption: Getting started

   installing
   quick_start
   example_eggerthellaceae
   faq

.. toctree::
   :maxdepth: 2
   :caption: Using STAMP

   input_files
   main_window
   analyses
   assistant
   command_line

.. toctree::
   :maxdepth: 2
   :caption: Understanding the results

   statistics
   choosing_a_test
   plots

.. toctree::
   :maxdepth: 1
   :caption: About

   changes
   citing
