Installing STAMP
================

STAMP is a single program with nothing else to install. Download the
package for your system from the
`releases page <https://github.com/donovan-h-parks/STAMP/releases>`_,
unpack it, and run it.

.. list-table::
   :header-rows: 1
   :widths: 35 65

   * - Package
     - Contents
   * - ``stamp-<version>-linux-x86_64.tar.gz``
     - ``stamp`` (the application), ``stamp-cli`` (the command line
       program), a desktop entry and icon, example data
   * - ``stamp-<version>-windows-x86_64.zip``
     - ``stamp.exe``, ``stamp-cli.exe``, example data
   * - ``stamp-<version>-macos-arm64.zip`` (Apple silicon) and
       ``stamp-<version>-macos-x86_64.zip`` (Intel)
     - ``STAMP.app``, ``stamp-cli``, example data
   * - ``stamp-<version>-web.zip``
     - The browser version, as static files for any web server

Each package also holds the licences of STAMP and of the fonts it uses.

Desktop application
-------------------

**Linux.** Unpack the archive and run ``stamp``. The application needs
OpenGL, and a desktop environment that provides file dialogs (an "XDG
desktop portal"), as current GNOME, KDE and similar desktops do. The
programs are built against Ubuntu 22.04 and need a Linux distribution of
that age or newer; on older systems, build STAMP from source (below).

**Windows.** Unpack the zip file and run ``stamp.exe``. The program is not
yet signed, so Windows SmartScreen may warn the first time it is run;
choose *More info* and then *Run anyway*.

**macOS.** Unpack the zip file and move ``STAMP.app`` to your Applications
folder. The application is not yet signed by Apple, so the first time,
right-click (or Control-click) it and choose *Open*, then confirm. If macOS
reports that the application is damaged, remove the quarantine flag that was
set on the download::

   xattr -dr com.apple.quarantine STAMP.app

A profile and its metadata can also be named when starting the
application::

   stamp my_profile.spf my_metadata.tsv

In the browser
--------------

The browser version is the same application compiled to WebAssembly. It
runs entirely on your own computer: files are opened with the browser's
file picker and never uploaded, and results are saved as downloads. It
needs a current browser with WebGL 2 (any recent Chrome, Edge, Firefox or
Safari).

The download is about 5 MB. While it loads, a bar shows its progress.

.. note::

   The browser runs analyses on the page's only thread, a few features at a
   time, so the page keeps responding during long resampling tests. White's
   non-parametric t-test is the exception: it holds the page until it has
   finished.

To host the browser version yourself, unpack ``stamp-<version>-web.zip``
into any directory served by a web server. No server-side software is
needed.

Command line program
--------------------

``stamp-cli`` is included in every desktop package. It runs the same
analyses without a window and writes tab-separated tables and plots; see
:doc:`command_line`. Check that it works with::

   stamp-cli --version

Building from source
--------------------

STAMP is written in `Rust <https://www.rust-lang.org>`_. With
`rustup <https://rustup.rs>`_ installed::

   git clone https://github.com/donovan-h-parks/STAMP.git
   cd STAMP
   git checkout rust-port
   cargo build --release -p stamp-gui -p stamp-cli

The programs are written to ``target/release/stamp`` and
``target/release/stamp-cli``. The compiler version is pinned by the
repository, and rustup installs it on first use.

The browser version is built with `trunk <https://trunkrs.dev>`_::

   cargo install trunk --locked
   cd crates/stamp-gui
   trunk build --release      # writes the site to crates/stamp-gui/dist
   trunk serve --release      # or build it and open it at http://127.0.0.1:8080

Testing the installation
------------------------

The quickest test is to open the example data: start STAMP and choose
File > Open example data > Enterotypes. A plot should appear within a few
seconds.

From the command line, the examples in the source repository can be
analysed directly::

   stamp-cli two-groups \
       --profile examples/EnterotypesArumugam/Enterotypes.profile.spf \
       --metadata examples/EnterotypesArumugam/Enterotypes.metadata.tsv \
       --field Enterotype --group1 "Enterotype 1" --group2 "Enterotype 3"

This prints a table with one row per genus.
