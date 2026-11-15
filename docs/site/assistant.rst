Asking an AI assistant
======================

STAMP can share what you are looking at with an AI assistant, so that you
can ask about your data in plain language: *what does this plot show?*,
*which genera differ most between these groups?*, *is this difference
large enough to matter?* There are four ways to do it, all in the
**Assistant** menu:

- **Claude Desktop** (desktop application): chat with Claude in Anthropic's
  Claude Desktop app beside STAMP. Claude sees your session and can change
  the analysis and plots for you.
- **Claude Code** (desktop application): the same, with Claude Code in a
  panel of STAMP's window.
- **OpenCode** (desktop application): the same, with OpenCode in that
  panel, for a **ChatGPT** plan, **GitHub Copilot** or other providers.
- **Copy and paste** (desktop and browser): copy a description of the
  session into any chat assistant. It cannot change anything in STAMP.

Claude Desktop
--------------

What you need
~~~~~~~~~~~~~

- The `Claude Desktop <https://claude.com/download>`_ app, signed in with
  your Claude account. It runs on macOS and Windows; on Linux it is in beta.
- STAMP's extension in Claude Desktop, added once (below).

Adding STAMP to Claude Desktop (once)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Choose **Assistant > Install in Claude Desktop**. STAMP writes its extension
(``stamp.mcpb``) to its data folder and hands it to Claude Desktop, which asks
whether to install it: choose **Install**. If Claude Desktop does not ask (on
Linux it cannot be handed the file), open Claude Desktop's settings, choose
**Extensions**, then **Install Extension**, and pick the file STAMP names.

The extension holds ``stamp-mcp``, a small program from STAMP's package that
Claude Desktop starts to reach STAMP. On its own it shares nothing.

Asking Claude
~~~~~~~~~~~~~

1. Load your data in STAMP and choose **Assistant > Share with Claude
   Desktop**. STAMP says what will be shared and asks you to confirm.
2. In Claude Desktop, ask about your session: *What does the plot in STAMP
   show?*, *Show me the volcano plot*, *Compare enterotype 1 with enterotype
   3 using the Mann-Whitney U test*, *Make the points larger and label only
   the significant genera*. Each change appears in STAMP's window as it is
   made.

If you ask Claude about STAMP while STAMP is not sharing, Claude is told to
ask you to share first.

Claude Code
-----------

**Assistant > Open Claude Code** connects `Claude Code
<https://claude.com/claude-code>`_, Anthropic's assistant for the terminal, in
the same way. It runs in the **assistant panel** along the bottom of STAMP's
window, like the Metadata table (a frame you can move elsewhere; see
:ref:`arranging-the-window`), and you type your questions there. Claude
Code needs a Claude **Pro, Max, Team or Enterprise** plan; you sign in with
your own Claude account the first time it starts, and STAMP never sees your
sign-in. If Claude Code is not installed, STAMP offers to install it with
Anthropic's installer (``claude.ai/install.sh``, or ``claude.ai/install.ps1``
on Windows), in the same panel, and then starts it.

The first time, Claude Code also asks whether you trust the folder it runs in
(STAMP's assistant folder, below): choose **Yes, I trust this folder** with
the arrow keys and Enter.

Before opening Claude Code, STAMP says what will be shared. Tick **Don't show
this again** to open it straight away in future; **Settings > Preferences >
Explain what is shared before opening Claude Code** brings the message back.
STAMP always asks before installing Claude Code.

.. _assistant-panel:

The assistant panel
~~~~~~~~~~~~~~~~~~~

The panel runs Claude Code or OpenCode, whichever was opened last; its tab
and the View menu are named after it. Opening the other one ends the one
running.

- Click in the panel to type into Claude Code. While it has the keyboard,
  every key goes to Claude Code, including STAMP's own shortcuts; click
  elsewhere in the window to use those again.
- **Ctrl+C** copies selected text (drag over it with the mouse); with nothing
  selected it interrupts Claude Code, as in any terminal. **Ctrl+V** pastes.
  **Shift+Enter** starts a new line without sending.
- Scroll with the mouse wheel to see earlier output, or drag the scroll bar
  on the right. When Claude Code uses its full-screen view it scrolls its
  own conversation with the wheel (and *Ctrl+End* returns to the bottom);
  the scroll bar then has nothing to show.
- **Hide** hides the panel and leaves Claude Code running; **View > Claude
  Code** (or **View > OpenCode**) shows it again. **Stop** ends Claude Code, **Restart** starts it
  again, and **Separate terminal** opens it in your computer's own terminal
  instead.
- The panel is always dark, whatever STAMP's theme, because Claude Code's
  colours are made for a dark background.

The same goes for OpenCode in the panel.

Claude Code is started in a folder STAMP prepares in its data folder (on Linux
``~/.local/share/stamp/assistant``, on macOS ``~/Library/Application
Support/STAMP/assistant``, on Windows ``%APPDATA%\STAMP\data\assistant``),
holding:

- ``stamp-mcp.json``: STAMP's address and the password for this session,
  readable only by you;
- ``stamp-settings.json``: lets Claude Code use STAMP's tools without asking
  each time (anything else it wants to do, such as reading or changing your
  files, still asks first);
- ``CLAUDE.md``: tells Claude Code what STAMP is and that its users are
  mostly biologists;
- a script that starts Claude Code with the first two, for **Separate
  terminal**.

If no terminal program is found on Linux for Separate terminal, run the script
in that folder yourself.

OpenCode
--------

**Assistant > Open OpenCode** connects `OpenCode <https://opencode.ai>`_, an
open-source assistant for the terminal that works with many AI providers, in
the same way, in the same panel (see :ref:`assistant-panel`). Use it if you
have a **ChatGPT Plus or Pro** plan, **GitHub Copilot**, OpenCode's own plans,
or an API key from a provider such as OpenAI or Google, rather than a Claude
plan.

OpenCode starts with one of its own free models, which needs no sign-in (see
OpenCode's site for what its free models' providers do with what is sent to
them). To use your own account instead, type ``/connect`` and choose how to
sign in: for a ChatGPT plan, **OpenAI** and then **Sign in with ChatGPT**; for
GitHub Copilot, **GitHub Copilot**, which shows a code to enter on GitHub's
site. Then type ``/models`` to choose a model, and ask your questions.
OpenCode remembers the sign-in. STAMP never sees it.

When OpenCode wants to do something other than use STAMP's tools, such as run
a command, it shows *Permission required* with **Allow once**, **Always
allow** and **Reject**.

If OpenCode is not installed, STAMP offers to install it, in the panel, and
then starts it: with OpenCode's installer (``opencode.ai/v2/install``) on
Linux and macOS, and from OpenCode's release on GitHub on Windows. Both put it
in ``.opencode/bin`` in your home folder. STAMP works with OpenCode 1 too.

OpenCode runs in the same assistant folder as Claude Code, where STAMP writes:

- ``opencode.json``: STAMP's address and the password for this session,
  readable only by you, and rules that make OpenCode ask before it runs
  commands, changes files or fetches web pages (it uses STAMP's tools
  without asking);
- ``AGENTS.md``: the same guidance as ``CLAUDE.md``;
- a script that starts OpenCode, for **Separate terminal**.

OpenCode reads ``opencode.json`` as the settings of that folder, on top of your
own OpenCode settings.

What the assistant can do
-------------------------

With Claude Desktop, Claude Code or OpenCode, the assistant uses these tools
of STAMP's (OpenCode names them ``stamp_get_session`` and so on):

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
   * - ``switch_samples``
     - Switch samples off (leave them out of every analysis) or back on, by
       name or by a metadata value, as the metadata table does: *Remove the
       samples from the QS company*.
   * - ``set_plot_fonts``
     - Change the size of the tick labels, axis labels and legends of every
       plot, and their font (Arial, Times New Roman, Courier New, DejaVu
       Sans). Text particular to one plot, such as its title or feature
       names, is changed with ``show_plot``.
   * - ``set_group_colors``
     - Change the colour of any group (and of *All other samples*) in every
       plot, by name (*red*, *blue*, ...) or as ``#rrggbb``.
   * - ``undo``
     - Undo its last change.

Claude waits for an analysis to finish and the plot to be drawn before it is
told the outcome of a change, so it describes what you see.

Undoing a change
~~~~~~~~~~~~~~~~

Every change Claude makes can be undone: click **Undo assistant's change** in
the status bar, choose **Assistant > Undo assistant's last change**, or ask
Claude to undo it. Each click undoes one change, most recent first. Undoing
puts back the analysis and plot settings as they were before that change,
including anything you changed yourself since.

Seeing what was shared
~~~~~~~~~~~~~~~~~~~~~~

The status bar says *Sharing with assistant* while STAMP is sharing, and
*Assistant connected* once Claude or OpenCode has connected. **Assistant >
Assistant activity** lists every request the assistant has made and the
first line of each answer, latest first.

Privacy
~~~~~~~

What the assistant reads (descriptions of your data, results and plots, and
images of plots when it asks for them) is sent to answer your questions:
with Claude, to Anthropic, under your Claude account and its terms; with
OpenCode, to the provider you signed in to (OpenAI, GitHub and so on), under
your account with it. Do not use it with data you are not allowed to share
with a third party.

Sharing is off until you choose Share with Claude Desktop, Open Claude Code
or Open OpenCode, and stops when you choose **Assistant > Stop sharing** or
close STAMP. STAMP listens for Claude only on your own computer (address
127.0.0.1), and only answers requests that carry a password made afresh each
time sharing starts; requests from web pages are refused. While it shares,
STAMP writes that password and its address to ``connection.json`` in the
folder above, readable only by you, for Claude Desktop's ``stamp-mcp``; the
file is removed when sharing stops.

How it works
~~~~~~~~~~~~

STAMP answers Claude through the `Model Context Protocol
<https://modelcontextprotocol.io>`_. Claude Code and OpenCode connect to it
directly;
Claude Desktop starts ``stamp-mcp``, which answers Claude Desktop itself and
passes each request on to the STAMP that is sharing.

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
anything in STAMP. The browser version cannot reach programs on your
computer, so it offers only this route.
