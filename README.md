# STAMP

[![version status](https://img.shields.io/pypi/v/stamp.svg)](https://pypi.python.org/pypi/stamp)
[![Downloads](https://pepy.tech/badge/stamp/month)](https://pepy.tech/project/stamp)

STAMP is a software package for analyzing taxonomic or metabolic profiles that promotes ‘best practices’ in choosing appropriate statistical techniques and reporting results. Statistical hypothesis tests for pairs of samples or groups of samples is support along with a wide range of exploratory plots. STAMP encourages the use of effect sizes and confidence intervals in assessing biological importance. A user friendly, graphical interface permits easy exploration of statistical results and generation of publication quality plots for inferring the biological relevance of features in a metagenomic profile. STAMP is open source, extensible via a plugin framework, and available for all major platforms.

STAMP can be obtained from the [Releases](https://github.com/dparks1134/STAMP/releases) page of this GitHub repository.

Further information, including installation and user documentation, can be found [here](https://beikolab.cs.dal.ca/software/STAMP).

The STAMP Google Group can be found [here](https://groups.google.com/forum/?hl=en#!forum/stamp_help).

## Rust port (in progress)

STAMP is being ported from Python to Rust, to run as a standalone application on Linux, Windows and macOS and in the browser. The port is not yet usable; STAMP v2 on the Releases page remains the current version.

- `crates/stamp-core`: data model, file parsers and statistics
- `crates/stamp-plot`: plots, with PNG, SVG and PDF export
- `crates/stamp-cli`: command-line interface
- `crates/stamp-gui`: graphical application (egui), for desktop and browser
- `legacy/`: the Python implementation, kept as the reference
- `tests/golden/`: reference outputs generated from the Python implementation
- `docs/CHANGES_FROM_PYTHON.md`: deliberate differences from STAMP v2

Build and test with `cargo build --workspace` and `cargo test --workspace`.

## License

STAMP is open source and released under the GNU General Public License v3.0. See LICENSE for further details.
