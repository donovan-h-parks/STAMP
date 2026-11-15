Asking an AI assistant
======================

STAMP can share what you are looking at with an AI assistant, so that you
can ask about your data in plain language: *what does this plot show?*,
*which genera differ most between these groups?*, *is this difference
large enough to matter?* There are two ways to do it, both in the
**Assistant** menu.

Copy and paste, with any assistant
----------------------------------

**Assistant > Copy description for AI** puts a description of the session on
the clipboard:

- the loaded profile and metadata, the samples and groups;
- the analysis: its type, the samples or groups compared, the statistical
  test, confidence interval, multiple test correction and filters;
- the plot on screen: its title, axis labels and legend, every item it draws
  with the values shown in its tooltip, and its options;
- the top 50 rows of the results table, smallest corrected p-value first.

Paste it into a chat with Claude, ChatGPT, Gemini or another assistant, and
ask your question below it. **Assistant > Copy plot image** copies the plot
itself, for assistants that read images; paste it into the same message.

This works in the desktop application and in the browser version, with any
subscription. The assistant sees only what you paste, and cannot change
anything in STAMP.

Claude Code: an assistant that works in STAMP
---------------------------------------------

In the desktop application, **Assistant > Open Claude Code** connects
`Claude Code <https://claude.com/claude-code>`_, Anthropic's assistant, to
the STAMP window. Claude Code opens in a terminal beside STAMP; you type your
questions there. It can look at the data, the results and the plot on
screen, and it can change the analysis and the plot for you: *show me the
volcano plot*, *compare enterotype 1 with enterotype 3 using the
Mann-Whitney U test*, *make the points larger and only label significant
genera*. Each change appears in STAMP's window as it is made.

What you need
~~~~~~~~~~~~~

- A Claude **Pro, Max, Team or Enterprise** plan. Claude Code is not part of
  the free plan. You sign in with your own Claude account the first time
  Claude Code starts; STAMP never sees your sign-in.
- Claude Code installed. If it is not, STAMP offers to open a terminal that
  installs it with Anthropic's installer (``claude.ai/install.sh``, or
  ``claude.ai/install.ps1`` on Windows) and then starts it.

Before connecting, STAMP says what will be shared and asks you to confirm.

What Claude Code can do
~~~~~~~~~~~~~~~~~~~~~~~

Claude Code uses these tools of STAMP's:

.. list-table::
   :header-rows: 1
   :widths: 25 75

   * - Tool
     - What it does
   * - ``get_session``
     - The loaded data and the settings of the analysis.
   * - ``get_results``
     - The results table, or the row of one feature.
   * - ``get_samples``
     - Every sample with its metadata and whether it is switched on.
   * - ``get_current_plot``
     - A description of the plot on screen, and optionally an image of it or
       its SVG.
   * - ``list_plots``, ``get_plot_options``
     - The plots of the current analysis, and the options of each.
   * - ``show_plot``
     - Show a plot, change its options, or choose the feature shown by the
       single-feature plots.
   * - ``set_analysis``
     - Change the analysis: its type, the profile level, the samples or
       groups compared, the test, confidence interval, correction and the
       p-value filter.
   * - ``undo``
     - Undo its last change.

Claude Code waits for an analysis to finish and the plot to be drawn before
it is told the outcome of a change, so it describes what you see.

Undoing a change
~~~~~~~~~~~~~~~~

Every change Claude Code makes can be undone: click **Undo assistant's
change** in the status bar, choose **Assistant > Undo assistant's last
change**, or ask Claude Code to undo it. Each click undoes one change, most
recent first. Undoing puts back the analysis and plot settings as they were
before that change, including anything you changed yourself since.

Seeing what was shared
~~~~~~~~~~~~~~~~~~~~~~

The status bar says *Claude Code connected* while it is. **Assistant >
Assistant activity** lists every request Claude Code has made and the first
line of each answer, latest first.

Privacy
~~~~~~~

What Claude Code reads (descriptions of your data, results and plots, and
images of plots when it asks for them) is sent to Anthropic to answer your
questions, under your Claude account and its terms. Do not use it with data
you are not allowed to share with a third party.

Sharing is off until you choose Open Claude Code, and stops when you choose
**Assistant > Stop sharing with Claude Code** or close STAMP. STAMP listens
for Claude Code only on your own computer (address 127.0.0.1), and only
answers requests that carry a password made afresh each time sharing starts;
requests from web pages are refused.

How it works
~~~~~~~~~~~~

STAMP answers Claude Code through the `Model Context Protocol
<https://modelcontextprotocol.io>`_. When you choose Open Claude Code, STAMP
writes a folder for Claude Code in its data folder (on Linux
``~/.local/share/stamp/assistant``, on macOS ``~/Library/Application
Support/STAMP/assistant``, on Windows ``%APPDATA%\STAMP\data\assistant``)
holding:

- ``stamp-mcp.json``: STAMP's address and the password for this session,
  readable only by you;
- ``stamp-settings.json``: lets Claude Code use STAMP's tools without asking
  each time (anything else it wants to do, such as reading or changing your
  files, still asks first);
- ``CLAUDE.md``: tells Claude Code what STAMP is and that its users are
  mostly biologists;
- a script that starts Claude Code with the first two.

It then opens your computer's terminal running that script. If no terminal
program is found on Linux, run the script in that folder yourself.

The browser version cannot start programs on your computer, so it offers
only the copy and paste route.
