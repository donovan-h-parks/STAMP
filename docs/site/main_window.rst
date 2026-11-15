The main window
===============

.. figure:: images/gui_main_window.png
   :width: 100%

   The STAMP window: profile and groups (left), plot and feature table
   (centre), properties (right).

The plot is always in the middle. The other parts of the window are frames
that can be moved around it (see `Arranging the window`_); the sections
below describe them where STAMP first puts them.

Profile and groups (left)
-------------------------

* **Parent level**, **Profile level** and **Unclassified** choose what is
  analysed; see :doc:`input_files`.
* **Group field** chooses the metadata field whose values form the groups.
  Each group is listed with its number of samples. Untick a group to leave it
  out of every analysis; groups with a single sample start unticked, since
  they cannot be tested. Click a group's colour to change it; the colour is
  used in every plot.
* **Sample filter** chooses the samples of every analysis by a metadata
  field: pick a field, then untick the values whose samples should be left
  out (each value is listed with its number of samples; *All* and *None*
  tick or untick them all). Samples left out take no part in any analysis
  or plot, including group statistics, distances and ordinations, and are
  not offered in the two-sample lists; groups left without samples
  disappear from the list of groups. The number of samples kept is shown
  beneath the values. Choose *None* as the field to keep every sample again.
  Loading other metadata or another profile clears the filter. The command
  line does the same with ``--where`` (see :ref:`cli-sample-filter`).

Plot (centre)
-------------

Beneath the plot are:

* the **plot selector**, listing the plots available for the current kind of
  analysis under six headings, by the question they answer (see
  :doc:`plots`);
* **Configure plot**, which opens the options of the current plot in a frame
  of their own (beside the properties to start with): its size, what it
  shows, its legend and the font sizes of its own text. Changes apply at
  once; **Restore defaults** returns this plot's options to STAMP's, and
  **Close** (or the × of its tab) closes the frame;
* **Zoom**, which enlarges the plot on screen without changing its size when
  saved;
* **Highlight**, which draws chosen features in red, in the plots that can:
  choose a level and a name, and every feature under that name is
  highlighted.

Point at any part of a plot to see what it stands for: a sample, a feature,
a group. Clicking a feature selects it.

.. figure:: images/gui_plot_selector.png
   :width: 70%

   The plot selector, with the plots of a multiple-group analysis by group.

Feature table (centre right)
----------------------------

The feature table lists every feature with its effect size and p-value.
Tick *Show only active features* to hide those removed by the filters.
Click a feature to show it in the plots of a single feature (bar, box and
post-hoc plots) and to highlight it in the prevalence-abundance plot. While
the box plot is shown, Ctrl+click (Cmd+click on a Mac) adds a feature to the
box plot's list of features, or removes it, and the plot then shows the list
(see :ref:`box plots of several features <plot-box-features>`).
Click a heading to sort by that column; click again to reverse the order.

Properties (right)
------------------

The tabs at the top choose the kind of analysis: **Multiple groups**, **Two
groups**, **Two samples** or **Correlation** (see :doc:`analyses`). Below
them:

* **Statistics**: the samples or groups compared, the statistical test, the
  confidence interval method and coverage, and the multiple-test correction.
  *Correction info* summarises the correction (for example, the estimated
  proportion of true null hypotheses for Storey's FDR).
* **Filtering**: which features are *active*, by p-value, number of
  sequences and effect size, or chosen by hand with *Select specific
  features*.

Progress and cancelling
-----------------------

Work that can take more than a moment is done in the background, so the
window keeps answering while it runs (on the desktop it runs alongside the
window; in the browser it is done in short pieces between redraws, which is
slower). Its progress is shown in the status bar at the bottom of the window,
with a **Cancel** button:

* **Analyses.** While the test of every feature runs, the status bar shows
  *Running test...* with a progress bar.
* **Slow plots.** The PCA, PCoA (with its community tests), NMDS, distance
  box, Mantel and ANCOM plots compute distances, ordinations and
  permutation tests that take several seconds for hundreds of samples (NMDS
  of 1,300 samples takes about ten seconds on the desktop). If the plot is
  still being computed after a moment, the plot shown before is greyed out
  and a panel in the middle of the plot area says what is being done, with
  a progress bar where the work can be counted (for example *PERMANOVA
  412/999* or *NMDS iterations 120/500*; steps that cannot be counted, such
  as finding the principal coordinates, show a moving bar). **Cancel**
  stops the calculation; the plot area then offers **Compute again**.
  Everything else in the window can be used meanwhile, and once computed a
  plot is kept, so changing its colours, shapes or sizes redraws it at
  once.
* **Large profiles.** Reading a large profile, or many Sandpiper files
  together, shows *Reading ...* in the status bar and the plot area.
* **The results table** of a large analysis (a column for every sample of
  a thousand samples or more) is built a few rows at a time while
  **View > Results table** is open, which shows how far it has got; saving
  it before it is complete saves it once it is.

.. figure:: images/gui_plot_progress.png
   :width: 100%

   A PCoA of the large example dataset (600 samples) being computed: the
   plot before is greyed out, and the panel says which community test is
   running and how far it has got (permutations done of 999).

Tables
------

* **View > Results table** shows the full results along the bottom of the
  window: every feature with its hierarchy, statistics and the counts of
  every sample, as saved by **File > Save results table**. *Show only active
  features* hides those removed by the filters; **Download** saves the
  table.
* **View > Metadata table** shows the metadata along the bottom of the
  window. Untick a sample to leave it out of every analysis, or use the
  filter to tick or untick all samples matching a condition (for example,
  *Remove all samples where Nationality = american*). Samples left out by
  the *Sample filter* are greyed out here.
* **View > Claude Code** (desktop application) shows or hides the Claude
  Code panel along the bottom of the window; see :doc:`assistant`.

The tables along the bottom share the room as tabs: click a tab to bring
its table to the front.

.. figure:: images/gui_metadata_table.png
   :width: 100%

   The metadata table along the bottom of the window.

.. _arranging-the-window:

Arranging the window
--------------------

Seven parts of the window are frames you can move: *Profile and groups*,
*Properties*, the *Feature table*, the *Metadata table*, the *Results
table*, *Configure plot* and, in the desktop application, *Claude Code*.
Each has a tab with its name.

* **Move a frame** by dragging its tab. While you drag, STAMP shows where
  the frame would go: drop it at the left, right, top or bottom edge of the
  plot or of another frame to put it beside that one, or on another frame's
  tabs to share its place as a tab.
* **Resize** frames by dragging the gap between them.
* **Close a frame** with the × on its tab; the View menu (or the button that
  opens it) brings it back in the same place. The room of a closed frame
  goes to the plot.
* **View > Reset layout** puts every frame back where STAMP first put it.

STAMP remembers the arrangement from one run to the next.

.. figure:: images/gui_frames.png
   :width: 100%

   Configure plot open beside the properties, and the results table along
   the bottom with the metadata table and Claude Code.

Saving plots
------------

**File > Save plot as** saves the current plot:

* **PNG**, a picture, at a resolution you choose (300 dots per inch suits
  most journals);
* **SVG** or **PDF**, drawings whose text and shapes stay editable in
  programs such as Inkscape or Adobe Illustrator.

**View > Send plot to window** opens the current plot in a window of its own,
so that several plots can be compared side by side.

Preferences
-----------

**Settings > Preferences** holds settings that apply everywhere:

* the **random seed** and number of **replicates** of the resampling methods
  (bootstrap, permutation, White's test, Storey's FDR, the community tests).
  The same seed gives the same results every time;
* the **pseudocount** added where a ratio would otherwise be undefined
  (a feature absent from a sample);
* the **smallest p-value reported**: smaller ones are shown as a bound, such
  as "< 1e-15";
* the colours of the two samples, of "All other samples" and of axes;
* the **plot font** (Arial, Times New Roman, Courier New or DejaVu Sans) and
  the font sizes of axis labels, tick labels and legends.

Settings are remembered between sessions.

Keyboard shortcuts
------------------

========================  =========================================
Ctrl+O (Cmd+O on macOS)   Load profile
Ctrl+S                    Save the plot as a PNG image
Ctrl+T                    Show or hide the results table
Ctrl+M                    Show or hide the metadata table
Ctrl+L                    Show or hide the profile and groups frame
Ctrl+P                    Show or hide the properties frame
Ctrl+W                    Send the plot to a window of its own
Ctrl+Q                    Exit
========================  =========================================
