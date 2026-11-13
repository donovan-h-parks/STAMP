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

### Command line

The statistics and file converters are usable now through `stamp-cli` (build it with `cargo build --release -p stamp-cli`; the binary is `target/release/stamp-cli`). Results are written as tab-separated tables with the same column headings as STAMP v2.

```bash
# What is in a profile?
stamp-cli info --profile examples/EnterotypesArumugam/Enterotypes.profile.spf \
               --metadata examples/EnterotypesArumugam/Enterotypes.metadata.tsv

# Two samples: Fisher's exact test with Benjamini-Hochberg correction
stamp-cli two-samples --profile examples/IronMineEdwards/EdwardsIronMine.spf \
    --sample1 Red --sample2 Black --profile-level Subsystem \
    --correction benjamini-hochberg --significance 0.05 --output iron_mine.tsv

# Two groups: Welch's t-test, keeping features that differ by at least 1%
stamp-cli two-groups --profile examples/EnterotypesArumugam/Enterotypes.profile.spf \
    --metadata examples/EnterotypesArumugam/Enterotypes.metadata.tsv --field Enterotype \
    --group1 "Enterotype 1" --group2 "Enterotype 3" --effect-size-filter diff-between-prop=1

# Several groups: ANOVA, then a post-hoc test on one feature
stamp-cli multi-groups --profile examples/EnterotypesArumugam/Enterotypes.profile.spf \
    --metadata examples/EnterotypesArumugam/Enterotypes.metadata.tsv --field Enterotype \
    --group "Enterotype 1" --group "Enterotype 2" --group "Enterotype 3"
stamp-cli post-hoc --profile examples/EnterotypesArumugam/Enterotypes.profile.spf \
    --metadata examples/EnterotypesArumugam/Enterotypes.metadata.tsv --field Enterotype \
    --group "Enterotype 1" --group "Enterotype 2" --group "Enterotype 3" --feature Bacteroides

# Convert another tool's output to a STAMP profile
stamp-cli convert mgrast examples/CowRumen-MG-RAST/CowRumen-MG-RAST.tsv --output cow_rumen.spf
```

The two-sample and two-group commands can also draw the filtered results as an extended error bar plot, as PNG, SVG or PDF:

```bash
stamp-cli two-samples --profile examples/IronMineEdwards/EdwardsIronMine.spf \
    --sample1 Black --sample2 Red --profile-level Classification \
    --correction bonferroni --significance 0.05 --plot iron_mine.png
```

![Extended error bar plot of the iron mine example](docs/images/extended_error_bar.png)

Run `stamp-cli <command> --help` for every option. Resampling methods take `--seed`, so a run can be repeated exactly.

## License

STAMP is open source and released under the GNU General Public License v3.0. See LICENSE for further details.
