//! End-to-end tests: run the `stamp-cli` binary on the example data and
//! compare its output with the Python reference tables.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde::Deserialize;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn stamp(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_stamp-cli"))
        .args(args)
        .current_dir(repo())
        .output()
        .expect("stamp-cli runs")
}

fn stdout(args: &[&str]) -> String {
    let output = stamp(args);
    assert!(
        output.status.success(),
        "stamp-cli {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn failure(args: &[&str]) -> String {
    let output = stamp(args);
    assert!(!output.status.success(), "stamp-cli {args:?} should fail");
    assert!(
        output.stdout.is_empty(),
        "nothing is written to stdout on failure"
    );
    String::from_utf8(output.stderr).unwrap()
}

struct Table {
    headings: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn parse(text: &str) -> Table {
        let mut lines = text.split('\n').filter(|l| !l.is_empty());
        let headings = lines
            .next()
            .unwrap()
            .split('\t')
            .map(str::to_string)
            .collect();
        let rows = lines
            .map(|l| l.split('\t').map(str::to_string).collect())
            .collect();
        Table { headings, rows }
    }

    fn column(&self, name: &str) -> usize {
        self.headings
            .iter()
            .position(|h| h == name)
            .unwrap_or_else(|| panic!("no column {name}"))
    }

    fn column_ending(&self, suffix: &str) -> usize {
        self.headings
            .iter()
            .position(|h| h.ends_with(suffix))
            .unwrap()
    }

    fn by_key(&self, key_columns: usize) -> HashMap<Vec<String>, &Vec<String>> {
        self.rows
            .iter()
            .map(|r| (r[..key_columns].to_vec(), r))
            .collect()
    }
}

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    file: String,
    mode: String,
    key_columns: usize,
    stochastic: bool,
    profile: Option<String>,
    metadata: Option<String>,
    group_field: Option<String>,
    sample1: Option<String>,
    sample2: Option<String>,
    group1: Option<String>,
    group2: Option<String>,
    groups: Option<Vec<String>>,
    parent_level: Option<String>,
    profile_level: Option<String>,
    unclassified: Option<String>,
    test: Option<String>,
    test_type: Option<String>,
    ci_method: Option<String>,
    coverage: Option<f64>,
}

/// Command-line spelling of the names used in the reference manifest.
fn option_value(name: &str) -> &'static str {
    match name {
        "Chi-square test" => "chi-square",
        "Chi-square test (w/ Yates')" => "chi-square-yates",
        "Difference between proportions" => "diff-between-prop",
        "Fisher's exact test" => "fisher",
        "G-test" => "g-test",
        "G-test (w/ Yates')" => "g-test-yates",
        "G-test (w/ Yates') + Fisher's" => "g-test-fisher",
        "Hypergeometric" => "hypergeometric",
        "DP: Asymptotic" => "dp-asymptotic",
        "DP: Asymptotic-CC" => "dp-asymptotic-cc",
        "DP: Newcombe-Wilson" => "dp-newcombe-wilson",
        "OR: Haldane adjustment" => "odds-ratio",
        "RP: Asymptotic" => "ratio-proportions",
        "t-test (equal variance)" => "t-test",
        "Welch's t-test" => "welch",
        "ANOVA" => "anova",
        "Kruskal-Wallis H-test" => "kruskal-wallis",
        "One-sided" => "one-sided",
        "Two-sided" => "two-sided",
        "Retain unclassified reads" => "retain",
        "Remove unclassified reads" => "remove",
        "Use only for calculating frequency profiles" => "frequency-only",
        other => panic!("no command-line value for {other}"),
    }
}

fn close(context: &str, expected: &str, actual: &str) {
    let (e, a): (f64, f64) = (expected.parse().unwrap(), actual.parse().unwrap());
    let ok = if e.is_nan() || a.is_nan() {
        e.is_nan() && a.is_nan()
    } else if e.is_infinite() || a.is_infinite() {
        e == a
    } else {
        (e - a).abs() <= 1e-7 + 1e-6 * e.abs()
    };
    assert!(ok, "{context}: expected {expected}, got {actual}");
}

/// Every deterministic reference case, run through the command line.
#[test]
fn analyses_reproduce_the_python_reference_tables() {
    let manifest: Manifest = serde_json::from_str(
        &fs::read_to_string(repo().join("tests/golden/manifest.json")).unwrap(),
    )
    .unwrap();
    let mut checked = 0;
    for case in manifest.cases.iter().filter(|c| !c.stochastic) {
        let mut args: Vec<String> = Vec::new();
        let mut arg = |name: &str, value: &str| {
            args.push(name.to_string());
            args.push(value.to_string());
        };
        match case.mode.as_str() {
            "two_samples" => {
                args.push("two-samples".to_string());
                let mut arg = |name: &str, value: &str| {
                    args.push(name.to_string());
                    args.push(value.to_string());
                };
                arg("--sample1", case.sample1.as_deref().unwrap());
                arg("--sample2", case.sample2.as_deref().unwrap());
                arg("--ci", option_value(case.ci_method.as_deref().unwrap()));
                arg("--type", option_value(case.test_type.as_deref().unwrap()));
                arg("--coverage", &case.coverage.unwrap().to_string());
            }
            "two_groups" => {
                arg("--metadata", case.metadata.as_deref().unwrap());
                arg("--field", case.group_field.as_deref().unwrap());
                arg("--group1", case.group1.as_deref().unwrap());
                if case.group2.as_deref() != Some("<All other samples>") {
                    arg("--group2", case.group2.as_deref().unwrap());
                }
                arg("--type", option_value(case.test_type.as_deref().unwrap()));
                arg("--coverage", &case.coverage.unwrap().to_string());
                args.insert(0, "two-groups".to_string());
            }
            "multiple_groups" => {
                arg("--metadata", case.metadata.as_deref().unwrap());
                arg("--field", case.group_field.as_deref().unwrap());
                for group in case.groups.as_ref().unwrap() {
                    arg("--group", group);
                }
                args.insert(0, "multi-groups".to_string());
            }
            _ => continue,
        }
        let mut arg = |name: &str, value: &str| {
            args.push(name.to_string());
            args.push(value.to_string());
        };
        arg("--profile", case.profile.as_deref().unwrap());
        arg("--profile-level", case.profile_level.as_deref().unwrap());
        if case.parent_level.as_deref() != Some("Entire sample") {
            arg("--parent-level", case.parent_level.as_deref().unwrap());
        }
        arg(
            "--unclassified",
            option_value(case.unclassified.as_deref().unwrap()),
        );
        arg("--test", option_value(case.test.as_deref().unwrap()));

        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        // STAMP v2 reported an infinite p-value when a one-sided result was
        // requested from a test that has none; the command line refuses.
        let two_sided_only = [
            "Chi-square test",
            "Chi-square test (w/ Yates')",
            "G-test",
            "G-test (w/ Yates')",
        ];
        if case.test_type.as_deref() == Some("One-sided")
            && two_sided_only.contains(&case.test.as_deref().unwrap())
        {
            assert!(
                failure(&args).contains("no one-sided p-value"),
                "{}",
                case.id
            );
            continue;
        }
        let actual = Table::parse(&stdout(&args));
        let expected = Table::parse(
            &fs::read_to_string(repo().join("tests/golden").join(&case.file)).unwrap(),
        );

        // The command line writes STAMP's own columns; the reference tables
        // have extra correction and effect-size columns after them.
        let own_columns = actual.headings.len();
        assert_eq!(
            actual.headings[..],
            expected.headings[..own_columns],
            "{}",
            case.id
        );
        assert_eq!(actual.rows.len(), expected.rows.len(), "{}", case.id);

        let effect_size = if case.mode == "two_groups" {
            "Difference between means"
        } else {
            "Effect size"
        };
        let mut columns = vec![actual.column("p-values"), actual.column(effect_size)];
        // v2's "DP: Asymptotic-CC" limits had a bug, fixed here; the core's
        // golden_stats.rs checks the corrected limits against v2's. White's
        // test uses that interval for sparse features too.
        // v2's Newcombe-Wilson limits were approximate (found by scanning);
        // they are exact now, and golden_stats.rs compares them with v2's
        // at a tolerance suited to the approximation.
        let fixed_cc = matches!(
            case.ci_method.as_deref(),
            Some("DP: Asymptotic-CC" | "DP: Newcombe-Wilson")
        );
        if case.mode != "multiple_groups" && !fixed_cc {
            columns.push(actual.column_ending("% lower CI"));
            columns.push(actual.column_ending("% upper CI"));
        }
        let note = actual.column("Note");

        let expected_rows = expected.by_key(case.key_columns);
        for row in &actual.rows {
            let reference = expected_rows[&row[..case.key_columns]];
            if reference[note] == "float division by zero" {
                continue; // a documented fix; see docs/CHANGES_FROM_PYTHON.md
            }
            // v2 refused Kruskal-Wallis tests with groups of fewer than
            // five samples (p = 1); the port computes them as R does. The
            // core's golden_stats.rs checks those p-values.
            let small_kruskal = reference[note]
                == "degenerate case: at least one group contains less than 5 samples";
            if small_kruskal {
                // Computed now; a feature with the same value everywhere
                // still has no test (R gives NaN).
                assert!(
                    ["", "Invalid input data for Kruskal-Wallis H-test."]
                        .contains(&row[note].as_str()),
                    "{}: {}",
                    case.id,
                    row[note]
                );
            } else {
                assert_eq!(row[note], reference[note], "{}", case.id);
            }
            let white_fisher =
                reference[note] == "heuristic: statistics calculated with Fisher's test";
            for &column in &columns {
                let context = format!(
                    "{} {:?} {}",
                    case.id,
                    &row[..case.key_columns],
                    actual.headings[column]
                );
                if small_kruskal && column == columns[0] {
                    continue;
                }
                if white_fisher && actual.headings[column].contains("% ") {
                    continue; // the fixed "DP: Asymptotic-CC" interval, as above
                }
                let mut reference = reference[column].clone();
                if case.test.as_deref() == Some("Hypergeometric") && column == columns[0] {
                    // v2 doubled the one-sided p-value without capping it at
                    // one; the port caps it (see docs/CHANGES_FROM_PYTHON.md).
                    let p: f64 = reference.parse().unwrap();
                    reference = p.min(1.0).to_string();
                }
                close(&context, &reference, &row[column]);
            }
        }
        checked += 1;
    }
    assert!(checked >= 80, "only {checked} cases were run");
}

const ENTEROTYPES: [&str; 6] = [
    "--profile",
    "examples/EnterotypesArumugam/Enterotypes.profile.spf",
    "--metadata",
    "examples/EnterotypesArumugam/Enterotypes.metadata.tsv",
    "--field",
    "Enterotype",
];

fn with<'a>(command: &'a str, base: &[&'a str], extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![command];
    args.extend_from_slice(base);
    args.extend_from_slice(extra);
    args
}

#[test]
fn info_describes_profile_and_metadata() {
    let out = stdout(&with("info", &ENTEROTYPES[..4], &[]));
    assert!(out.contains("Hierarchical levels: Phyla, Genera"));
    assert!(out.contains("Samples (39):"));
    assert!(out.contains("Enterotype: Enterotype 1 (8), Enterotype 2 (6), Enterotype 3 (18)"));
}

#[test]
fn samples_are_chosen_by_their_metadata() {
    let info = stdout(&with(
        "info",
        &ENTEROTYPES[..4],
        &["--where", "Nationality=japanese"],
    ));
    assert!(
        info.contains("Sample filter: Nationality = japanese (13 of 39 samples kept)"),
        "{info}"
    );
    assert!(
        info.contains("Enterotype: Enterotype 1 (6), Enterotype 3 (3), Infant (4)"),
        "{info}"
    );
    assert!(info.contains("Nationality: japanese (13)"), "{info}");

    // The filter gives the same results as leaving out every other sample
    // by name.
    let metadata =
        fs::read_to_string(repo().join("examples/EnterotypesArumugam/Enterotypes.metadata.tsv"))
            .unwrap();
    let others: Vec<String> = metadata
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            (!matches!(cells[2], "japanese" | "french")).then(|| cells[0].to_string())
        })
        .collect();
    let mut excluded = with("multi-groups", &ENTEROTYPES, &[]);
    for sample in &others {
        excluded.extend(["--exclude-sample", sample.as_str()]);
    }
    let output = stamp(&with(
        "multi-groups",
        &ENTEROTYPES,
        &[
            "--where",
            "Nationality=japanese",
            "--where",
            "Nationality = french",
        ],
    ));
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "Sample filter (Nationality is one of japanese, french): kept 21 of 39 samples."
        ),
        "{stderr}"
    );
    let filtered = String::from_utf8(output.stdout).unwrap();
    // Leaving the samples out by name keeps their groups, empty; the filter
    // drops groups it empties, so compare the tables, not the group lists.
    assert_eq!(filtered, stdout(&excluded));

    // --where-not leaves samples out; different fields must all match.
    let table = |extra: &[&str]| Table::parse(&stdout(&with("multi-groups", &ENTEROTYPES, extra)));
    let not = table(&[
        "--where-not",
        "Enterotype=Infant",
        "--where",
        "Nationality=japanese",
    ]);
    let only = table(&[
        "--where",
        "Enterotype=Enterotype 1",
        "--where",
        "Enterotype=Enterotype 3",
        "--where",
        "Nationality=japanese",
    ]);
    assert_eq!(not.rows, only.rows);

    // The two-group and correlation commands take the same options.
    let two = stdout(&with(
        "two-groups",
        &ENTEROTYPES,
        &[
            "--group1",
            "Enterotype 1",
            "--group2",
            "Enterotype 3",
            "--where",
            "Nationality=japanese",
        ],
    ));
    assert!(Table::parse(&two).rows.len() > 5);

    // Mistakes are reported.
    let misspelled = failure(&with(
        "multi-groups",
        &ENTEROTYPES,
        &["--where", "Nationality=japan"],
    ));
    assert!(
        misspelled.contains("No sample has the value 'japan' in the metadata field 'Nationality'"),
        "{misspelled}"
    );
    assert!(failure(&with(
        "multi-groups",
        &ENTEROTYPES,
        &["--where", "Natonality=japanese"]
    ))
    .contains("Unknown metadata field 'Natonality'"));
    assert!(failure(&with(
        "multi-groups",
        &ENTEROTYPES,
        &[
            "--where",
            "Nationality=japanese",
            "--where-not",
            "Nationality=japanese"
        ]
    ))
    .contains("the sample filter leaves no samples"));
    assert!(failure(&with(
        "multi-groups",
        &ENTEROTYPES,
        &["--where", "Nationality"]
    ))
    .contains("FIELD=VALUE"));
    assert!(failure(&[
        "info",
        "--profile",
        ENTEROTYPES[1],
        "--where",
        "Nationality=japanese"
    ])
    .contains("need a metadata file"));
}

#[test]
fn filters_and_corrections_reduce_the_table() {
    let base = with(
        "two-groups",
        &ENTEROTYPES,
        &["--group1", "Enterotype 1", "--group2", "Enterotype 3"],
    );
    let all = Table::parse(&stdout(&base));
    assert_eq!(all.rows.len(), 248);

    let corrected = Table::parse(&stdout(&with(
        base[0],
        &base[1..],
        &["--correction", "bonferroni"],
    )));
    let (p, q) = (
        corrected.column("p-values"),
        corrected.column("p-values (corrected)"),
    );
    for row in &corrected.rows {
        let (p, q): (f64, f64) = (row[p].parse().unwrap(), row[q].parse().unwrap());
        assert!((q - (p * 248.0).min(1.0)).abs() < 1e-12);
    }

    let filtered = Table::parse(&stdout(&with(
        base[0],
        &base[1..],
        &[
            "--correction",
            "benjamini-hochberg",
            "--significance",
            "0.05",
            "--effect-size-filter",
            "diff-between-prop=1",
            "--sequence-filter",
            "maximum=100",
        ],
    )));
    assert!(
        !filtered.rows.is_empty() && filtered.rows.len() < 20,
        "{}",
        filtered.rows.len()
    );
    let (q, effect) = (
        filtered.column("p-values (corrected)"),
        filtered.column("Difference between means"),
    );
    for row in &filtered.rows {
        assert!(row[q].parse::<f64>().unwrap() <= 0.05);
        assert!(row[effect].parse::<f64>().unwrap().abs() >= 1.0);
    }
}

#[test]
fn excluding_samples_and_default_second_group() {
    let out = Table::parse(&stdout(&with(
        "two-groups",
        &ENTEROTYPES,
        &[
            "--group1",
            "Infant",
            "--exclude-sample",
            "AM-AD-1",
            "--profile-level",
            "Phyla",
        ],
    )));
    assert!(out
        .headings
        .contains(&"<All other samples>: mean rel. freq. (%)".to_string()));
    assert!(!out.headings.contains(&"AM-AD-1".to_string()));
    assert!(out.headings.contains(&"AM-AD-2".to_string()));
}

#[test]
fn groups_that_cannot_be_compared_are_refused() {
    // A group compared with itself.
    let error = failure(&with(
        "two-groups",
        &ENTEROTYPES,
        &["--group1", "Infant", "--group2", "Infant"],
    ));
    assert!(error.contains("cannot be compared with itself"), "{error}");
    // A named group whose only sample is excluded: it used to be tested,
    // and every feature came out with p = 1.
    let error = failure(&with(
        "multi-groups",
        &ENTEROTYPES,
        &[
            "--group",
            "Enterotype 1",
            "--group",
            "Enterotype 2",
            "--group",
            "Enterotype 3 - twin",
            "--exclude-sample",
            "AM-F10-T1",
        ],
    ));
    assert!(
        error.contains("'Enterotype 3 - twin' has no samples"),
        "{error}"
    );
    let error = failure(&with(
        "two-groups",
        &ENTEROTYPES,
        &[
            "--group1",
            "Enterotype 3 - twin",
            "--exclude-sample",
            "AM-F10-T1",
        ],
    ));
    assert!(error.contains("at least one sample"), "{error}");
}

#[test]
fn the_multi_group_seed_reaches_the_community_tests() {
    let directory = std::env::temp_dir().join(format!("stamp-cli-seed-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let pcoa = |seed: &str| {
        let svg = svg_plot(
            "multi-groups",
            &ENTEROTYPES,
            &["--plot-type", "pcoa", "--seed", seed],
            &directory,
            &format!("pcoa{seed}"),
        );
        let start = svg
            .find("PERMANOVA")
            .expect("the community tests are reported");
        svg[start..start + 400].to_string()
    };
    let first = pcoa("1");
    assert_eq!(first, pcoa("1"));
    assert_ne!(first, pcoa("7"));
    fs::remove_dir_all(&directory).ok();
}

#[cfg(target_os = "linux")]
#[test]
fn a_failed_write_to_standard_output_is_an_error() {
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_stamp-cli"))
        .args(with("two-groups", &ENTEROTYPES, &["--group1", "Infant"]))
        .current_dir(repo())
        .stdout(full)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("cannot write to standard output"), "{error}");
}

#[test]
fn resampling_is_reproducible_for_a_seed() {
    let args = |seed: &'static str| {
        with(
            "two-groups",
            &ENTEROTYPES,
            &[
                "--group1",
                "Enterotype 1",
                "--group2",
                "Enterotype 2",
                "--profile-level",
                "Phyla",
                "--test",
                "white",
                "--replicates",
                "200",
                "--seed",
                seed,
            ],
        )
    };
    let first = stdout(&args("11"));
    assert_eq!(first, stdout(&args("11")));
    assert_ne!(first, stdout(&args("12")));
}

#[test]
fn post_hoc_lists_every_pair() {
    let out = Table::parse(&stdout(&with(
        "post-hoc",
        &ENTEROTYPES,
        &[
            "--group",
            "Enterotype 1",
            "--group",
            "Enterotype 2",
            "--group",
            "Enterotype 3",
            "--feature",
            "Bacteroides",
            "--feature",
            "Prevotella",
            "--test",
            "scheffe",
        ],
    )));
    assert_eq!(out.rows.len(), 6);
    assert_eq!(
        out.rows[0][..2],
        ["Bacteroides", "Enterotype 1 : Enterotype 2"]
    );
    assert!(out.rows[0][2].parse::<f64>().unwrap() < 0.001);
}

#[test]
fn convert_and_output_file() {
    let directory = std::env::temp_dir().join(format!("stamp-cli-test-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let target = directory.join("cow.spf");
    let out = stdout(&[
        "convert",
        "mgrast",
        "examples/CowRumen-MG-RAST/CowRumen-MG-RAST.tsv",
        "--output",
        target.to_str().unwrap(),
    ]);
    assert!(
        out.is_empty(),
        "nothing goes to stdout when --output is given"
    );

    let sorted = |text: &str| {
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        lines[1..].sort();
        lines
    };
    let expected =
        fs::read_to_string(repo().join("examples/CowRumen-MG-RAST/CowRumen.spf")).unwrap();
    assert_eq!(
        sorted(&fs::read_to_string(&target).unwrap()),
        sorted(&expected)
    );

    // The converted file is directly usable.
    let table = Table::parse(&stdout(&[
        "two-samples",
        "--profile",
        target.to_str().unwrap(),
        "--sample1",
        "4441682.3",
        "--sample2",
        "4441680.3",
        "--profile-level",
        "level 1",
    ]));
    assert_eq!(table.rows.len(), 28);
    fs::remove_dir_all(&directory).unwrap();

    let cog = stdout(&[
        "convert",
        "cog",
        "examples/Assign_COGs_Example.tsv",
        "--assign-to-each-code",
    ]);
    assert_eq!(cog.lines().count(), 5);
}

#[test]
fn convert_singlem_and_sandpiper_profiles() {
    let data = "crates/stamp-core/tests/data/singlem";
    let runs = ["SRR28772463", "ERR4084076"];
    let files: Vec<String> = runs.iter().map(|r| format!("{data}/{r}.json")).collect();
    let mut args = vec!["convert", "sandpiper"];
    args.extend(files.iter().map(String::as_str));
    let sandpiper = stdout(&args);
    let header = "Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tSRR28772463\tERR4084076";
    assert_eq!(sandpiper.lines().next().unwrap(), header);
    assert!(sandpiper
        .contains("\tunclassified f__Oscillospiraceae\tunclassified f__Oscillospiraceae\t"));

    let singlem_file = format!("{data}/singlem_condensed.tsv");
    let singlem = stdout(&["convert", "singlem", &singlem_file]);
    assert_eq!(singlem.lines().next().unwrap(), header);
    assert_eq!(singlem.lines().count(), sandpiper.lines().count());

    // The converted profile can be analysed at once.
    let directory = std::env::temp_dir().join(format!("stamp-cli-singlem-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let target = directory.join("sandpiper.spf");
    let mut args = vec!["convert", "sandpiper", "--output", target.to_str().unwrap()];
    args.extend(files.iter().map(String::as_str));
    stdout(&args);
    let table = Table::parse(&stdout(&[
        "two-samples",
        "--profile",
        target.to_str().unwrap(),
        "--sample1",
        "SRR28772463",
        "--sample2",
        "ERR4084076",
        "--profile-level",
        "Phylum",
    ]));
    assert!(table.rows.len() > 5);
    fs::remove_dir_all(&directory).unwrap();

    assert!(failure(&["convert", "singlem", "examples/heatmap.spf"])
        .contains("does not appear to be a SingleM condensed profile"));
    assert!(failure(&["convert", "sandpiper", "examples/heatmap.spf"])
        .contains("does not appear to be a Sandpiper condensed profile"));
}

#[test]
fn problems_are_reported_clearly() {
    assert!(failure(&[
        "two-samples",
        "--profile",
        "missing.spf",
        "--sample1",
        "a",
        "--sample2",
        "b"
    ])
    .contains("cannot read missing.spf"));
    assert!(failure(&[
        "two-samples",
        "--profile",
        "examples/FinegoldAutism/Autism.spf",
        "--sample1",
        "a",
        "--sample2",
        "b"
    ])
    .contains("Data does not form a strict hierarchy"));
    assert!(
        failure(&with("two-groups", &ENTEROTYPES, &["--group1", "Nope"]))
            .contains("Groups of 'Enterotype': Enterotype 1, Enterotype 2")
    );
    assert!(failure(&with(
        "two-groups",
        &ENTEROTYPES[..4],
        &["--field", "Nope", "--group1", "x"]
    ))
    .contains("Fields in the metadata file: Enterotype, Nationality"));
    let iron = [
        "--profile",
        "examples/IronMineEdwards/EdwardsIronMine.spf",
        "--sample1",
        "Red",
        "--sample2",
        "Black",
    ];
    assert!(failure(&with(
        "two-samples",
        &iron,
        &["--test", "g-test", "--type", "one-sided"]
    ))
    .contains("no one-sided p-value"));
    assert!(failure(&with("two-samples", &iron, &["--coverage", "95"])).contains("between 0 and 1"));
    assert!(failure(&with(
        "two-samples",
        &iron,
        &["--sequence-filter", "biggest=3"]
    ))
    .contains("unknown filter 'biggest'"));
    assert!(failure(&with(
        "two-samples",
        &iron,
        &["--effect-size-filter", "odds-ratio"]
    ))
    .contains("NAME=VALUE"));
    assert!(
        failure(&with("two-samples", &iron, &["--profile-level", "Genus"]))
            .contains("Unknown hierarchical level 'Genus'")
    );
}

#[test]
fn plots_are_written_in_each_format() {
    let directory = std::env::temp_dir().join(format!("stamp-cli-plots-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let iron = [
        "--profile",
        "examples/IronMineEdwards/EdwardsIronMine.spf",
        "--sample1",
        "Black",
        "--sample2",
        "Red",
        "--profile-level",
        "Classification",
        "--correction",
        "bonferroni",
        "--significance",
        "0.05",
    ];

    let svg_path = directory.join("iron.svg");
    let table = Table::parse(&stdout(&with(
        "two-samples",
        &iron,
        &["--plot", svg_path.to_str().unwrap()],
    )));
    let svg = fs::read_to_string(&svg_path).unwrap();
    // One label per row of the filtered table, plus the titles.
    assert_eq!(table.rows.len(), 8);
    for row in &table.rows {
        assert!(
            svg.contains(&format!(">{}</text>", row[0])),
            "{} is not in the plot",
            row[0]
        );
    }
    assert!(svg.contains(">95% confidence intervals</text>"));
    // Bonferroni: corrected p-values.
    assert!(svg.contains(">Corrected p-value</text>"));
    assert!(svg.contains(">Difference between proportions (%)</text>"));
    // 7 inches wide; 8 rows of 0.2 inches plus margins.
    assert!(svg.contains("width=\"504pt\" height=\"162pt\""));

    let png_path = directory.join("iron.png");
    stdout(&with(
        "two-samples",
        &iron,
        &["--plot", png_path.to_str().unwrap(), "--plot-dpi", "100"],
    ));
    let png = fs::read(&png_path).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let dimension = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    assert_eq!((dimension(16), dimension(20)), (700, 225));

    let pdf_path = directory.join("groups.pdf");
    stdout(&with(
        "two-groups",
        &ENTEROTYPES,
        &[
            "--group1",
            "Enterotype 1",
            "--group2",
            "Enterotype 3",
            "--profile-level",
            "Phyla",
            "--plot",
            pdf_path.to_str().unwrap(),
            "--plot-legend",
            "lower-center",
            "--plot-sort-by",
            "effect-sizes",
        ],
    ));
    assert!(fs::read(&pdf_path).unwrap().starts_with(b"%PDF-"));

    let bad = directory.join("plot.eps");
    assert!(failure(&with(
        "two-samples",
        &iron,
        &["--plot", bad.to_str().unwrap()]
    ))
    .contains("use a .png, .svg or .pdf extension"));
    fs::remove_dir_all(&directory).unwrap();
}

/// Render one plot as SVG and return its text.
fn svg_plot(
    command: &str,
    base: &[&str],
    extra: &[&str],
    directory: &std::path::Path,
    name: &str,
) -> String {
    let path = directory.join(format!("{name}.svg"));
    let mut args = with(command, base, extra);
    // The table goes to a file too, so that the test also runs on Windows.
    let table = directory.join(format!("{name}.tsv"));
    args.extend_from_slice(&[
        "--plot",
        path.to_str().unwrap(),
        "--output",
        table.to_str().unwrap(),
    ]);
    stdout(&args);
    fs::read_to_string(&path).unwrap()
}

#[test]
fn every_plot_type_can_be_drawn() {
    let directory =
        std::env::temp_dir().join(format!("stamp-cli-plot-types-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let has = |svg: &str, label: &str| svg.contains(&format!(">{label}</text>"));

    let iron = [
        "--profile",
        "examples/IronMineEdwards/EdwardsIronMine.spf",
        "--sample1",
        "Black",
        "--sample2",
        "Red",
        "--profile-level",
        "Classification",
        "--correction",
        "benjamini-hochberg",
    ];
    for (kind, label) in [
        ("bar", "Proportion of sequences (%)"),
        ("profile-bar", "Proportion of sequences (%)"),
        ("scatter", "Black (%)"),
        ("sequence-histogram", "Sequences"),
        // Benjamini-Hochberg: q-values.
        ("p-value-histogram", "q-value"),
        ("multiple-comparison", "# significant features"),
    ] {
        let svg = svg_plot(
            "two-samples",
            &iron,
            &["--plot-type", kind],
            &directory,
            kind,
        );
        assert!(has(&svg, label), "two-samples {kind}: no {label:?}");
    }

    let two = [
        "--group1",
        "Enterotype 1",
        "--group2",
        "Enterotype 3",
        "--significance",
        "0.05",
    ];
    let groups: Vec<&str> = ENTEROTYPES.iter().chain(&two).copied().collect();
    for (kind, label) in [
        ("bar", "Proportion of sequences (%)"),
        ("box", "Enterotype 3"),
        ("scatter", "Enterotype 1 (%)"),
        ("heatmap", "Enterotype 3"),
    ] {
        let svg = svg_plot(
            "two-groups",
            &groups,
            &["--plot-type", kind],
            &directory,
            &format!("g-{kind}"),
        );
        assert!(has(&svg, label), "two-groups {kind}: no {label:?}");
    }
    // A PCoA of Euclidean distances, which is what the PCA plot showed.
    let euclidean = svg_plot(
        "two-groups",
        &groups,
        &["--plot-type", "pcoa", "--distance", "euclidean"],
        &directory,
        "g-euclidean",
    );
    assert!(euclidean.contains(">PCo1 (") && euclidean.contains(">PCo3 ("));
    assert!(has(&euclidean, "Euclidean distance"));
    let chosen = svg_plot(
        "two-groups",
        &groups,
        &["--plot-type", "box", "--plot-feature", "Prevotella"],
        &directory,
        "g-box-feature",
    );
    assert!(has(&chosen, "Prevotella"));

    let three = [
        "--group",
        "Enterotype 1",
        "--group",
        "Enterotype 2",
        "--group",
        "Enterotype 3",
    ];
    let multi: Vec<&str> = ENTEROTYPES.iter().chain(&three).copied().collect();
    let filtered = [
        "--correction",
        "benjamini-hochberg",
        "--significance",
        "0.05",
    ];
    let heatmap = svg_plot("multi-groups", &multi, &filtered, &directory, "m-heatmap");
    assert!(has(&heatmap, "Bacteroides") && has(&heatmap, "Enterotype 2"));
    for kind in ["pcoa", "bar", "box"] {
        let svg = svg_plot(
            "multi-groups",
            &multi,
            &["--plot-type", kind],
            &directory,
            &format!("m-{kind}"),
        );
        assert!(svg.len() > 2000, "multi-groups {kind}");
    }
    let post_hoc = svg_plot(
        "post-hoc",
        &multi,
        &["--feature", "Bacteroides"],
        &directory,
        "post-hoc",
    );
    // p-values are computed; STAMP v2 showed bounds such as "< 0.001".
    assert!(has(&post_hoc, "Enterotype 1 : Enterotype 2") && !has(&post_hoc, "&lt; 0.001"));
    // Tukey-Kramer allows for the number of pairs.
    assert!(has(&post_hoc, "Corrected p-value"));

    // The volcano and composition plots, for groups and for samples.
    let volcano = svg_plot(
        "multi-groups",
        &multi,
        &["--plot-type", "volcano"],
        &directory,
        "m-volcano",
    );
    // No correction was chosen.
    assert!(has(&volcano, "Eta-squared") && has(&volcano, "\u{2212}log10(Uncorrected p-value)"));
    let composition = svg_plot(
        "multi-groups",
        &multi,
        &["--plot-type", "composition"],
        &directory,
        "m-composition",
    );
    assert!(has(&composition, "Proportion of sequences (%)") && has(&composition, "Other"));

    // Rarefaction curves need counts of sequences; the coal-seam profile
    // has them and the enterotype profile does not.
    let coal = [
        "--profile",
        "examples/CoalSeamGas/coal.spf",
        "--metadata",
        "examples/CoalSeamGas/coal.metadata.tsv",
        "--field",
        "Core vs. Water",
    ];
    let rarefaction = svg_plot(
        "multi-groups",
        &coal,
        &["--plot-type", "rarefaction"],
        &directory,
        "rarefaction",
    );
    assert!(
        has(&rarefaction, "Expected number of features")
            && has(&rarefaction, "Number of sequences")
    );
    let pooled = svg_plot(
        "multi-groups",
        &coal,
        &[
            "--plot-type",
            "rarefaction",
            "--rarefaction-curves",
            "pooled",
        ],
        &directory,
        "rarefaction-pooled",
    );
    assert!(has(&pooled, "Number of sequences") && pooled.len() < rarefaction.len());
    let accumulated = svg_plot(
        "multi-groups",
        &coal,
        &[
            "--plot-type",
            "rarefaction",
            "--rarefaction-curves",
            "accumulated",
        ],
        &directory,
        "rarefaction-accumulated",
    );
    assert!(has(&accumulated, "Number of samples"));
    // The community plots: diversity, ordination by distance, distances.
    let alpha = svg_plot(
        "multi-groups",
        &coal,
        &[
            "--plot-type",
            "alpha-diversity",
            "--alpha-metric",
            "observed",
            "--rarefy",
        ],
        &directory,
        "alpha",
    );
    assert!(has(&alpha, "Observed features"));
    let pcoa = svg_plot(
        "multi-groups",
        &coal,
        &["--plot-type", "pcoa", "--distance", "jaccard"],
        &directory,
        "pcoa",
    );
    assert!(has(&pcoa, "Jaccard distance") && pcoa.contains("PERMANOVA: pseudo-F = "));
    // With three or more groups, every pair of them is tested too.
    assert!(!pcoa.contains("Pairwise PERMANOVA"));
    let pairwise = svg_plot(
        "multi-groups",
        &[
            "--profile",
            "examples/CoalSeamGas/coal.spf",
            "--metadata",
            "examples/CoalSeamGas/coal.metadata.tsv",
            "--field",
            "Simplified Environment",
        ],
        &["--plot-type", "pcoa"],
        &directory,
        "pairwise",
    );
    assert!(pairwise.contains("Pairwise PERMANOVA (Holm-corrected p-values)"));
    // The p-values of a group analysis as a histogram.
    let histogram = svg_plot(
        "multi-groups",
        &coal,
        &[
            "--plot-type",
            "p-value-histogram",
            "--correction",
            "bonferroni",
        ],
        &directory,
        "m-histogram",
    );
    assert!(has(&histogram, "Corrected p-value"));
    let boxes = svg_plot(
        "two-groups",
        &[
            "--profile",
            "examples/CoalSeamGas/coal.spf",
            "--metadata",
            "examples/CoalSeamGas/coal.metadata.tsv",
            "--field",
            "Core vs. Water",
            "--group1",
            "Core",
        ],
        &["--plot-type", "distance-box"],
        &directory,
        "distance-box",
    );
    assert!(has(&boxes, "Core") && has(&boxes, "Between groups"));

    let unwritten = directory.join("rarefaction-fractions.svg");
    assert!(failure(&with(
        "multi-groups",
        &multi,
        &[
            "--plot-type",
            "rarefaction",
            "--plot",
            unwritten.to_str().unwrap()
        ]
    ))
    .contains("Rarefaction needs counts"));
    assert!(!unwritten.exists());

    // Plot types that do not apply are refused, with nothing written.
    let refused = directory.join("refused.svg");
    let message = failure(&with(
        "two-samples",
        &iron,
        &["--plot-type", "pcoa", "--plot", refused.to_str().unwrap()],
    ));
    assert!(
        message.contains("the pcoa plot is not available for two samples"),
        "{message}"
    );
    assert!(!refused.exists());
    // The PCA plot is gone: a PCoA of Euclidean distances replaces it.
    let message = failure(&with(
        "two-groups",
        &groups,
        &["--plot-type", "pca", "--plot", refused.to_str().unwrap()],
    ));
    assert!(message.contains("invalid value 'pca'"), "{message}");
    assert!(!refused.exists());
    let message = failure(&with(
        "two-groups",
        &groups,
        &[
            "--plot-type",
            "box",
            "--plot-feature",
            "Nope",
            "--plot",
            refused.to_str().unwrap(),
        ],
    ));
    assert!(message.contains("unknown feature 'Nope'"));
    fs::remove_dir_all(&directory).unwrap();
}

/// A small profile with metadata that pairs the samples of two groups and
/// gives each a number, written to a directory of its own.
fn paired_example(name: &str) -> (PathBuf, String, String) {
    let directory = std::env::temp_dir().join(format!("stamp-cli-{name}-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let profile = directory.join("profile.spf");
    fs::write(
        &profile,
        "Phylum\tGenus\tA1\tA2\tA3\tA4\tB1\tB2\tB3\tB4\n\
         P1\tx\t10\t12\t11\t14\t30\t33\t31\t29\n\
         P1\ty\t50\t48\t52\t47\t50\t49\t51\t50\n\
         P2\tz\t40\t40\t37\t39\t20\t18\t18\t21\n",
    )
    .unwrap();
    let metadata = directory.join("metadata.tsv");
    fs::write(
        &metadata,
        "Id\tGroup\tSubject\tDepth\n\
         A1\tA\ts1\t1.0\nA2\tA\ts2\t2.5\nA3\tA\ts3\t4.0\nA4\tA\ts4\t5.5\n\
         B1\tB\ts4\t6.0\nB2\tB\ts3\t7.5\nB3\tB\ts2\t9.0\nB4\tB\ts9\tdeep\n",
    )
    .unwrap();
    (
        directory,
        profile.to_str().unwrap().to_string(),
        metadata.to_str().unwrap().to_string(),
    )
}

#[test]
fn the_friedman_test_matches_subjects_across_groups() {
    let directory = std::env::temp_dir().join(format!("stamp-cli-friedman-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let profile = directory.join("profile.spf");
    // x rises from A to B to C in every subject; y does not change.
    fs::write(
        &profile,
        "Phylum\tGenus\tA1\tA2\tA3\tA4\tB1\tB2\tB3\tB4\tC1\tC2\tC3\tC4\tC5\n\
         P1\tx\t10\t12\t11\t14\t20\t23\t21\t19\t30\t33\t31\t29\t5\n\
         P1\ty\t50\t48\t52\t47\t50\t49\t51\t50\t50\t48\t52\t47\t50\n\
         P2\tz\t40\t40\t37\t39\t30\t28\t28\t31\t20\t19\t17\t24\t45\n",
    )
    .unwrap();
    let metadata = directory.join("metadata.tsv");
    // C5 is a fifth subject, sampled only once.
    fs::write(
        &metadata,
        "Id\tTime\tSubject\n\
         A1\tA\ts1\nA2\tA\ts2\nA3\tA\ts3\nA4\tA\ts4\n\
         B1\tB\ts4\nB2\tB\ts3\nB3\tB\ts2\nB4\tB\ts1\n\
         C1\tC\ts2\nC2\tC\ts1\nC3\tC\ts4\nC4\tC\ts3\nC5\tC\ts5\n",
    )
    .unwrap();
    let (profile, metadata) = (profile.to_str().unwrap(), metadata.to_str().unwrap());
    let base = [
        "--profile",
        profile,
        "--metadata",
        metadata,
        "--field",
        "Time",
    ];
    let output = stamp(&with(
        "multi-groups",
        &base,
        &["--test", "friedman", "--pair-field", "Subject"],
    ));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "Matched by Subject: 4 subjects sampled in each of the 3 groups; 1 other sample left out."
    ));
    let table = Table::parse(&String::from_utf8_lossy(&output.stdout));
    // The unmatched sample takes no part.
    assert!(!table.headings.iter().any(|h| h == "C5"));
    let (genus, p) = (table.column("Genus"), table.column("p-values"));
    let value = |feature: &str| -> f64 {
        table.rows.iter().find(|r| r[genus] == feature).unwrap()[p]
            .parse()
            .unwrap()
    };
    // Four subjects ranked the same way in three groups: chi-square = 8 on
    // two degrees of freedom, p = exp(-4).
    assert!(
        (value("x") - (-4.0f64).exp()).abs() < 1e-9,
        "{}",
        value("x")
    );
    assert!(value("y") > 0.3);

    // The post-hoc test of matched groups.
    let post_hoc = stdout(&with(
        "post-hoc",
        &base,
        &[
            "--feature",
            "x",
            "--test",
            "wilcoxon",
            "--pair-field",
            "Subject",
        ],
    ));
    assert!(post_hoc.contains("A : B") && post_hoc.contains("B : C"));

    let fails = |command: &str, extra: &[&str]| failure(&with(command, &base, extra));
    assert!(fails("multi-groups", &["--test", "friedman"]).contains("--pair-field"));
    assert!(fails("multi-groups", &["--pair-field", "Subject"]).contains("Friedman"));
    assert!(fails(
        "multi-groups",
        &["--test", "friedman", "--pair-field", "Time"]
    )
    .contains("fewer than two values"));
    assert!(fails("post-hoc", &["--feature", "x", "--test", "wilcoxon"]).contains("--pair-field"));
    fs::remove_dir_all(&directory).ok();
}

#[test]
fn presence_or_absence_is_tested_between_groups() {
    let directory = std::env::temp_dir().join(format!("stamp-cli-presence-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let profile = directory.join("profile.spf");
    // w is in every sample of A and in none of B; x is in all of them.
    fs::write(
        &profile,
        "Phylum\tGenus\tA1\tA2\tA3\tA4\tB1\tB2\tB3\tB4\n\
         P1\tw\t3\t1\t2\t4\t0\t0\t0\t0\n\
         P1\tx\t10\t12\t11\t14\t30\t33\t31\t29\n\
         P2\tz\t40\t40\t37\t39\t20\t18\t18\t21\n",
    )
    .unwrap();
    let metadata = directory.join("metadata.tsv");
    fs::write(
        &metadata,
        "Id\tGroup\tSubject\n\
         A1\tA\ts1\nA2\tA\ts2\nA3\tA\ts3\nA4\tA\ts4\n\
         B1\tB\ts4\nB2\tB\ts3\nB3\tB\ts2\nB4\tB\ts9\n",
    )
    .unwrap();
    let (profile, metadata) = (profile.to_str().unwrap(), metadata.to_str().unwrap());
    let base = [
        "--profile",
        profile,
        "--metadata",
        metadata,
        "--field",
        "Group",
    ];
    let two = ["--group1", "A", "--group2", "B"];
    let p_of = |command: &str, extra: &[&str], feature: &str| -> f64 {
        let table = Table::parse(&stdout(&with(command, &base, extra)));
        let (genus, p) = (table.column("Genus"), table.column("p-values"));
        table.rows.iter().find(|r| r[genus] == feature).unwrap()[p]
            .parse()
            .unwrap()
    };
    let close = |value: f64, expected: f64| {
        assert!(
            (value - expected).abs() < 1e-9,
            "{value} against {expected}"
        )
    };
    // Fisher's exact test: one table of 70 is as extreme, in each direction.
    let fisher = [&two[..], &["--test", "fisher-presence"]].concat();
    close(p_of("two-groups", &fisher, "w"), 2.0 / 70.0);
    close(p_of("two-groups", &fisher, "x"), 1.0);
    // McNemar's test on the three pairs, all with w in A only.
    let mcnemar = [&two[..], &["--test", "mcnemar", "--pair-field", "Subject"]].concat();
    close(p_of("two-groups", &mcnemar, "w"), 0.25);
    // Chi-square = 8 on one degree of freedom.
    close(
        p_of("multi-groups", &["--test", "chi-square-presence"], "w"),
        0.004677734981047266,
    );
    assert!(failure(&with(
        "two-groups",
        &base,
        &[&fisher[..], &["--transform", "clr"]].concat()
    ))
    .contains("presence or absence"));
    assert!(failure(&with(
        "two-groups",
        &base,
        &[&two[..], &["--test", "mcnemar"]].concat()
    ))
    .contains("--pair-field"));
    fs::remove_dir_all(&directory).ok();
}

#[test]
fn paired_tests_and_the_transform() {
    let (directory, profile, metadata) = paired_example("paired");
    let base = [
        "--profile",
        &profile,
        "--metadata",
        &metadata,
        "--field",
        "Group",
        "--group1",
        "A",
        "--group2",
        "B",
    ];
    let run = |extra: &[&str]| {
        let output = stamp(&with("two-groups", &base, extra));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            Table::parse(&String::from_utf8_lossy(&output.stdout)),
            String::from_utf8_lossy(&output.stderr).to_string(),
        )
    };
    let p_of = |table: &Table, feature: &str| -> f64 {
        let (genus, p) = (table.column("Genus"), table.column("p-values"));
        table.rows.iter().find(|r| r[genus] == feature).unwrap()[p]
            .parse()
            .unwrap()
    };

    // Three subjects have a sample in each group; A1 and B4 have no partner.
    let (paired, messages) = run(&["--test", "paired-t", "--pair-field", "Subject"]);
    assert!(
        messages.contains("3 pairs; 2 samples without a partner left out"),
        "{messages}"
    );
    assert!(paired.headings.iter().any(|h| h == "A2"));
    assert!(!paired.headings.iter().any(|h| h == "A1" || h == "B4"));
    let (unpaired, _) = run(&[]);
    assert!(unpaired.headings.iter().any(|h| h == "A1"));
    assert_ne!(p_of(&paired, "x"), p_of(&unpaired, "x"));
    let (wilcoxon, _) = run(&["--test", "wilcoxon", "--pair-field", "Subject"]);
    // Three pairs all one way: the smallest two-sided p-value is 2/8.
    assert!((p_of(&wilcoxon, "x") - 0.25).abs() < 1e-9);

    // The transform changes p-values but not effect sizes.
    let (transformed, _) = run(&["--transform", "clr"]);
    assert_ne!(p_of(&transformed, "y"), p_of(&unpaired, "y"));
    let effect = unpaired.column("Difference between means");
    let genus = unpaired.column("Genus");
    for row in &unpaired.rows {
        let other = transformed
            .rows
            .iter()
            .find(|r| r[genus] == row[genus])
            .unwrap();
        assert_eq!(row[effect], other[effect]);
    }
    let multi = |extra: &[&str]| {
        let base = [
            "--profile",
            &profile,
            "--metadata",
            &metadata,
            "--field",
            "Group",
        ];
        Table::parse(&stdout(&with("multi-groups", &base, extra)))
    };
    assert_ne!(
        p_of(&multi(&["--transform", "clr"]), "y"),
        p_of(&multi(&[]), "y")
    );

    // The box plot of a paired test can join each pair with a line: three
    // pairs, so six more points than without.
    let boxes = |name: &str, extra: &[&str]| {
        let mut args = vec![
            "--test",
            "wilcoxon",
            "--pair-field",
            "Subject",
            "--plot-type",
            "box",
            "--plot-feature",
            "x",
        ];
        args.extend_from_slice(extra);
        svg_plot("two-groups", &base, &args, &directory, name)
    };
    let joined = boxes("pairs", &["--box-pairs"]);
    let plain = boxes("no-pairs", &[]);
    assert_eq!(
        joined.matches("<circle").count(),
        plain.matches("<circle").count() + 6
    );
    assert!(joined.contains("stroke-opacity"));

    // What cannot be done is said plainly.
    let fails = |extra: &[&str]| failure(&with("two-groups", &base, extra));
    assert!(fails(&["--plot", "x.svg", "--plot-type", "box", "--box-pairs"]).contains("paired"));
    assert!(fails(&["--test", "paired-t"]).contains("--pair-field"));
    assert!(fails(&["--pair-field", "Subject"]).contains("paired tests"));
    assert!(fails(&["--test", "white", "--transform", "clr"]).contains("White"));
    assert!(fails(&["--test", "wilcoxon", "--pair-field", "Nothing"]).contains("Nothing"));
    let one_group = [
        "--profile",
        &profile,
        "--metadata",
        &metadata,
        "--field",
        "Group",
        "--group1",
        "A",
        "--test",
        "paired-t",
        "--pair-field",
        "Subject",
    ];
    assert!(failure(&with("two-groups", &one_group, &[])).contains("--group2"));
    fs::remove_dir_all(&directory).ok();
}

#[test]
fn correlation_with_a_numeric_field() {
    let (directory, profile, metadata) = paired_example("correlation");
    let base = [
        "--profile",
        &profile,
        "--metadata",
        &metadata,
        "--field",
        "Depth",
    ];
    let output = stamp(&with("correlation", &base, &[]));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // B4 has no number.
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 of the samples have no number"));
    let table = Table::parse(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(table.rows.len(), 3);
    assert!(!table.headings.iter().any(|h| h == "B4"));
    let (genus, rho, p) = (
        table.column("Genus"),
        table.column("Spearman rank correlation (rho)"),
        table.column("p-values"),
    );
    let value = |feature: &str, column: usize| -> f64 {
        table.rows.iter().find(|r| r[genus] == feature).unwrap()[column]
            .parse()
            .unwrap()
    };
    // x rises with depth and z falls.
    assert!(value("x", rho) > 0.7 && value("z", rho) < -0.7);
    assert!(value("x", p) < 0.05);
    // Ordered by p-value.
    let ps: Vec<f64> = table.rows.iter().map(|r| r[p].parse().unwrap()).collect();
    assert!(ps.windows(2).all(|w| w[0] <= w[1]));

    let pearson = Table::parse(&stdout(&with(
        "correlation",
        &base,
        &["--method", "pearson"],
    )));
    assert!(pearson
        .headings
        .iter()
        .any(|h| h == "Pearson correlation (r)"));
    let filtered = Table::parse(&stdout(&with(
        "correlation",
        &base,
        &["--min-correlation", "0.7", "--correction", "bonferroni"],
    )));
    assert_eq!(filtered.rows.len(), 2);

    // The plots of a correlation.
    let has = |svg: &str, label: &str| svg.contains(&format!(">{label}</text>"));
    let scatter = svg_plot(
        "correlation",
        &base,
        &["--plot-feature", "z", "--group-field", "Group"],
        &directory,
        "scatter",
    );
    assert!(has(&scatter, "z") && has(&scatter, "Depth") && has(&scatter, "A"));
    assert!(scatter.contains("rho = -"));
    let volcano = svg_plot(
        "correlation",
        &base,
        &["--plot-type", "volcano"],
        &directory,
        "volcano",
    );
    assert!(has(&volcano, "Spearman rank correlation (rho)"));
    svg_plot(
        "correlation",
        &base,
        &["--plot-type", "p-value-histogram"],
        &directory,
        "histogram",
    );
    // The diversity of each sample against the field.
    let alpha = svg_plot(
        "correlation",
        &base,
        &[
            "--plot-type",
            "alpha-diversity",
            "--alpha-metric",
            "shannon",
        ],
        &directory,
        "alpha",
    );
    assert!(has(&alpha, "Depth") && alpha.contains("Spearman rho = "));

    let fails = |base: &[&str], extra: &[&str]| failure(&with("correlation", base, extra));
    assert!(fails(&base, &["--plot", "x.svg", "--plot-type", "heatmap"]).contains("not available"));
    let text_field = [
        "--profile",
        &profile,
        "--metadata",
        &metadata,
        "--field",
        "Subject",
    ];
    assert!(fails(&text_field, &[]).contains("at least three samples"));
    let unknown = [
        "--profile",
        &profile,
        "--metadata",
        &metadata,
        "--field",
        "Nothing",
    ];
    assert!(fails(&unknown, &[]).contains("Fields in the metadata file"));
    fs::remove_dir_all(&directory).ok();
}

#[test]
fn ancom_matches_scikit_bio_and_new_plots_are_drawn() {
    // The table scikit-bio 0.7.4's `ancom` was run on: W was 7, 7, 2, 2, 2,
    // 2, 2, 2 for three groups, with the first two features different, and
    // 7, 1, 1, 1, 1, 1, 1, 0 for the first two groups.
    let rows = [
        [57.0, 45.0, 47.0, 55.0, 43.0, 51.0, 53.0, 29.0],
        [22.0, 32.0, 31.0, 54.0, 56.0, 20.0, 39.0, 52.0],
        [25.0, 51.0, 24.0, 38.0, 52.0, 32.0, 33.0, 31.0],
        [48.0, 30.0, 59.0, 37.0, 39.0, 40.0, 43.0, 42.0],
        [160.0, 59.0, 52.0, 51.0, 48.0, 44.0, 33.0, 59.0],
        [152.0, 28.0, 53.0, 26.0, 54.0, 44.0, 24.0, 21.0],
        [148.0, 21.0, 25.0, 40.0, 58.0, 38.0, 52.0, 56.0],
        [208.0, 45.0, 37.0, 40.0, 30.0, 39.0, 35.0, 29.0],
        [472.0, 4.0, 23.0, 27.0, 58.0, 47.0, 55.0, 28.0],
        [384.0, 6.8, 39.0, 20.0, 44.0, 53.0, 46.0, 26.0],
        [328.0, 6.0, 58.0, 55.0, 27.0, 40.0, 57.0, 53.0],
        [384.0, 9.0, 21.0, 49.0, 38.0, 23.0, 29.0, 41.0],
    ];
    let directory = std::env::temp_dir().join(format!("stamp-cli-ancom-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let samples: Vec<String> = (1..=12).map(|i| format!("S{i:02}")).collect();
    let mut spf = format!("Feature\t{}\n", samples.join("\t"));
    for f in 0..8 {
        let values: Vec<String> = rows.iter().map(|r| r[f].to_string()).collect();
        spf.push_str(&format!("f{f}\t{}\n", values.join("\t")));
    }
    let mut metadata = "Id\tGroup\tDepth\n".to_string();
    for (i, sample) in samples.iter().enumerate() {
        let group = ["A", "B", "C"][i / 4];
        metadata.push_str(&format!(
            "{sample}\t{group}\t{}\n",
            (i * 5 % 12) as f64 + 0.5
        ));
    }
    let (profile, meta) = (
        directory.join("input.spf"),
        directory.join("input-metadata.tsv"),
    );
    fs::write(&profile, spf).unwrap();
    fs::write(&meta, metadata).unwrap();
    let base = [
        "--profile",
        profile.to_str().unwrap(),
        "--metadata",
        meta.to_str().unwrap(),
        "--field",
        "Group",
    ];
    let w_of = |table: &Table| -> Vec<(String, String, String)> {
        let (feature, w, reject) = (
            table.column("Feature"),
            table.column("W"),
            table.column("Reject null hypothesis"),
        );
        let mut rows: Vec<(String, String, String)> = table
            .rows
            .iter()
            .map(|r| (r[feature].clone(), r[w].clone(), r[reject].clone()))
            .collect();
        rows.sort();
        rows
    };
    let output = stamp(&with("ancom", &base, &[]));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("2 of 8 features differ"));
    let table = Table::parse(&String::from_utf8_lossy(&output.stdout));
    let expected: Vec<(String, String, String)> = [7, 7, 2, 2, 2, 2, 2, 2]
        .iter()
        .enumerate()
        .map(|(f, w)| {
            (
                format!("f{f}"),
                w.to_string(),
                if f < 2 { "True" } else { "False" }.to_string(),
            )
        })
        .collect();
    assert_eq!(w_of(&table), expected);
    // Most different first.
    assert_eq!(table.rows[0][table.column("W")], "7");
    assert!(table.headings.iter().any(|h| h == "Eta-squared of CLR"));

    let two = Table::parse(&stdout(&with(
        "ancom",
        &base,
        &["--group", "A", "--group", "B"],
    )));
    let ws: Vec<String> = w_of(&two).into_iter().map(|r| r.1).collect();
    assert_eq!(ws, ["7", "1", "1", "1", "1", "1", "1", "0"]);
    assert!(two.headings.iter().any(|h| h == "Difference in mean CLR"));
    assert!(failure(&with("ancom", &base, &["--alpha", "2"])).contains("--alpha"));

    // Plots: the ANCOM plot, NMDS, shared features, rank-abundance curves.
    let has = |svg: &str, label: &str| svg.contains(&format!(">{label}</text>"));
    let ancom_svg = svg_plot("ancom", &base, &[], &directory, "ancom");
    assert!(has(&ancom_svg, "W") && ancom_svg.contains("2 of 8 features differ"));
    let nmds = svg_plot(
        "multi-groups",
        &base,
        &["--plot-type", "nmds"],
        &directory,
        "nmds",
    );
    assert!(has(&nmds, "NMDS1") && nmds.contains("Stress = "));
    let venn = svg_plot(
        "multi-groups",
        &base,
        &[
            "--plot-type",
            "shared-features",
            "--shared-style",
            "venn",
            "--min-prevalence",
            "100",
        ],
        &directory,
        "venn",
    );
    assert!(has(&venn, "A (8)"), "{venn}");
    let upset = svg_plot(
        "two-groups",
        &base,
        &[
            "--group1",
            "A",
            "--group2",
            "B",
            "--plot-type",
            "shared-features",
        ],
        &directory,
        "upset",
    );
    assert!(has(&upset, "Features"));
    let ranks = svg_plot(
        "multi-groups",
        &base,
        &["--plot-type", "rank-abundance", "--rank-curves", "groups"],
        &directory,
        "ranks",
    );
    assert!(has(&ranks, "Rank") && has(&ranks, "C"));
    let ancom_groups = svg_plot(
        "two-groups",
        &base,
        &["--group1", "A", "--group2", "C", "--plot-type", "ancom"],
        &directory,
        "ancom-two",
    );
    assert!(has(&ancom_groups, "Difference in mean CLR"));
    let two_samples = [
        "--profile",
        profile.to_str().unwrap(),
        "--sample1",
        "S01",
        "--sample2",
        "S12",
    ];
    let ranks = svg_plot(
        "two-samples",
        &two_samples,
        &["--plot-type", "rank-abundance"],
        &directory,
        "ranks-two",
    );
    assert!(has(&ranks, "S12"));

    // The Mantel test, with correlations.
    let correlation = [
        "--profile",
        profile.to_str().unwrap(),
        "--metadata",
        meta.to_str().unwrap(),
        "--field",
        "Depth",
    ];
    let mantel = svg_plot(
        "correlation",
        &correlation,
        &["--plot-type", "mantel"],
        &directory,
        "mantel",
    );
    assert!(mantel.contains("Mantel test: rho = "), "mantel plot");
    assert!(has(&mantel, "Difference in Depth"));
    let mut args = with("correlation", &correlation, &["--plot-type", "mantel"]);
    let path = directory.join("mantel2.svg");
    args.extend_from_slice(&["--plot", path.to_str().unwrap()]);
    let output = stamp(&args);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Mantel test of Bray-Curtis distance"));
    assert!(failure(&with(
        "multi-groups",
        &base,
        &["--plot", "x.svg", "--plot-type", "mantel"]
    ))
    .contains("not available"));
    fs::remove_dir_all(&directory).ok();
}

#[test]
fn plot_font_sizes_can_be_chosen() {
    let directory = std::env::temp_dir().join(format!("stamp-cli-fonts-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let font = |svg: &str, label: &str| -> String {
        let end = svg.find(&format!(">{label}</text>")).unwrap();
        let start = svg[..end].rfind("font-size=\"").unwrap() + 11;
        svg[start..start + svg[start..].find('"').unwrap()].to_string()
    };
    let base = &ENTEROTYPES;
    let extra = [
        "--plot-type",
        "box",
        "--group",
        "Enterotype 1",
        "--group",
        "Enterotype 2",
    ];
    let plain = svg_plot("multi-groups", base, &extra, &directory, "plain");
    assert_eq!(font(&plain, "Enterotype 1"), "8");
    assert_eq!(font(&plain, "Proportion of sequences (%)"), "9");
    let mut sized = extra.to_vec();
    sized.extend_from_slice(&["--tick-font-size", "12", "--axis-label-font-size", "14"]);
    let svg = svg_plot("multi-groups", base, &sized, &directory, "sized");
    assert_eq!(font(&svg, "Enterotype 1"), "12");
    assert_eq!(font(&svg, "Proportion of sequences (%)"), "14");
    assert!(plain.contains("font-family=\"Arial, Helvetica, Liberation Sans, sans-serif\""));
    let mut times = extra.to_vec();
    times.extend_from_slice(&["--font", "times-new-roman"]);
    let svg = svg_plot("multi-groups", base, &times, &directory, "times");
    assert!(svg.contains("font-family=\"Times New Roman, Times, Liberation Serif, serif\""));
    let mut bad = with("multi-groups", base, &extra);
    bad.extend_from_slice(&["--plot", "x.svg", "--legend-font-size", "0"]);
    assert!(failure(&bad).contains("font sizes"));
    fs::remove_dir_all(&directory).ok();
}

const COAL: [&str; 8] = [
    "--profile",
    "examples/CoalSeamGas/coal.spf",
    "--metadata",
    "examples/CoalSeamGas/coal.metadata.tsv",
    "--field",
    "Environment",
    "--profile-level",
    "Genus",
];

#[test]
fn group_heatmap_summarises_each_group() {
    let directory =
        std::env::temp_dir().join(format!("stamp-cli-group-heatmap-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let table_path = directory.join("summary.tsv");
    let svg = svg_plot(
        "multi-groups",
        &COAL,
        &[
            "--plot-type",
            "group-heatmap",
            "--heatmap-statistic",
            "prevalence",
            "--heatmap-top",
            "12",
            "--heatmap-order-field",
            "Core vs. Water",
            "--heatmap-values",
            "--heatmap-table",
            table_path.to_str().unwrap(),
        ],
        &directory,
        "prevalence",
    );
    for text in [
        ">Core (n=8)</text>",
        ">Produced water (n=15)</text>",
        ">Prevalence (% of samples)</text>",
        ">The 12 features of highest mean abundance, of 631</text>",
    ] {
        assert!(svg.contains(text), "{text} is not in the plot");
    }
    let summary = Table::parse(&fs::read_to_string(&table_path).unwrap());
    assert_eq!(
        summary.headings,
        [
            "Feature (Prevalence)",
            "Core",
            "Deep cutting",
            "Produced water",
            "Shallow cutting"
        ]
    );
    assert_eq!(summary.rows.len(), 12);
    // Prevalences are percentages of a group's samples: multiples of
    // 100/8 for the eight Core samples.
    for row in &summary.rows {
        let core: f64 = row[1].parse().unwrap();
        let eighths = core / 12.5;
        assert!((eighths - eighths.round()).abs() < 1e-9, "{core}");
        for cell in &row[1..] {
            let value: f64 = cell.parse().unwrap();
            assert!((0.0..=100.0).contains(&value));
        }
    }

    // The sample filter leaves out the produced water; the maximum is at
    // least the mean.
    let filtered = |statistic: &str, name: &str| {
        let path = directory.join(format!("{name}_cells.tsv"));
        svg_plot(
            "multi-groups",
            &COAL,
            &[
                "--where",
                "Core vs. Water=Core",
                "--plot-type",
                "group-heatmap",
                "--heatmap-statistic",
                statistic,
                "--heatmap-top",
                "0",
                "--heatmap-scale",
                "log10",
                "--heatmap-table",
                path.to_str().unwrap(),
            ],
            &directory,
            name,
        );
        Table::parse(&fs::read_to_string(&path).unwrap())
    };
    let (max, mean) = (filtered("max", "max"), filtered("mean", "mean"));
    assert_eq!(max.headings.len(), 4);
    assert!(!max.headings.iter().any(|h| h == "Produced water"));
    assert_eq!(max.rows.len(), mean.rows.len());
    for (a, b) in max.rows.iter().zip(&mean.rows) {
        assert_eq!(a[0], b[0]);
        for (x, y) in a[1..].iter().zip(&b[1..]) {
            assert!(x.parse::<f64>().unwrap() >= y.parse::<f64>().unwrap() - 1e-12);
        }
    }

    // Two groups.
    let svg = svg_plot(
        "two-groups",
        &ENTEROTYPES,
        &[
            "--group1",
            "Enterotype 1",
            "--group2",
            "Enterotype 3",
            "--plot-type",
            "group-heatmap",
            "--heatmap-statistic",
            "median",
        ],
        &directory,
        "two_groups",
    );
    assert!(svg.contains(">Median proportion (%)</text>"));
    assert!(svg.contains(">Enterotype 1 (n=8)</text>"));

    // Mistakes are explained.
    let plot = directory.join("bad.svg");
    let base = with(
        "multi-groups",
        &COAL,
        &[
            "--plot",
            plot.to_str().unwrap(),
            "--plot-type",
            "group-heatmap",
        ],
    );
    let mut no_field = base.clone();
    no_field.extend(["--heatmap-column-order", "category"]);
    assert!(failure(&no_field).contains("needs --heatmap-order-field"));
    let mut unknown = base.clone();
    unknown.extend(["--heatmap-order-field", "Nope"]);
    assert!(failure(&unknown).contains("unknown metadata field 'Nope'"));
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn co_occurrence_correlates_features() {
    let directory =
        std::env::temp_dir().join(format!("stamp-cli-co-occurrence-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let base = [
        "--profile",
        "examples/CoalSeamGas/coal.spf",
        "--profile-level",
        "Genus",
    ];
    let svg = svg_plot(
        "co-occurrence",
        &base,
        &["--top", "12", "--clr", "--feature", "g__Methanobacterium"],
        &directory,
        "matrix",
    );
    assert!(svg.contains(">Spearman rho (centred log ratios)</text>"));
    assert!(svg.contains(">g__Methanobacterium</text>"));
    let table = Table::parse(&fs::read_to_string(directory.join("matrix.tsv")).unwrap());
    assert_eq!(
        table.headings,
        [
            "Feature 1",
            "Feature 2",
            "rho",
            "p-value",
            "q-value (Benjamini-Hochberg)"
        ]
    );
    // 13 features: 78 pairs, with q-values no smaller than p-values and
    // sorted by q-value.
    assert_eq!(table.rows.len(), 78);
    let q: Vec<f64> = table.rows.iter().map(|r| r[4].parse().unwrap()).collect();
    assert!(q.windows(2).all(|w| w[0] <= w[1]));
    for row in &table.rows {
        let (rho, p, q): (f64, f64, f64) = (
            row[2].parse().unwrap(),
            row[3].parse().unwrap(),
            row[4].parse().unwrap(),
        );
        assert!((-1.0..=1.0).contains(&rho) && q >= p);
    }

    // Pearson on proportions, within one group of samples.
    let mut args = vec!["co-occurrence"];
    args.extend_from_slice(&base);
    args.extend([
        "--metadata",
        "examples/CoalSeamGas/coal.metadata.tsv",
        "--field",
        "Environment",
        "--group",
        "Produced water",
        "--method",
        "pearson",
        "--top",
        "5",
    ]);
    let table = Table::parse(&stdout(&args));
    assert_eq!(table.headings[2], "r");
    assert_eq!(table.rows.len(), 10);

    // The correlations of one feature with every other.
    let svg = svg_plot(
        "co-occurrence",
        &base,
        &["--with", "g__Methanobacterium", "--partners", "8"],
        &directory,
        "partners",
    );
    assert!(svg.contains(">Correlations with g__Methanobacterium</text>"));
    let table = Table::parse(&fs::read_to_string(directory.join("partners.tsv")).unwrap());
    assert!(table.rows.len() > 100);
    assert!(table.rows.iter().all(|r| r[0] == "g__Methanobacterium"));
    let strength: Vec<f64> = table
        .rows
        .iter()
        .map(|r| r[2].parse::<f64>().unwrap().abs())
        .collect();
    assert!(strength.windows(2).all(|w| w[0] >= w[1]));

    // Two features against each other, a point for each sample.
    let svg = svg_plot(
        "co-occurrence",
        &base,
        &[
            "--with",
            "g__Methanobacterium",
            "--against",
            "g__Methanosarcina",
        ],
        &directory,
        "pair",
    );
    assert!(svg.contains(">g__Methanosarcina against g__Methanobacterium</text>"));
    assert!(svg.contains(">g__Methanobacterium (%)</text>"));
    assert!(svg.contains("Spearman rho (proportions) = "));
    let mut alone = vec!["co-occurrence"];
    alone.extend_from_slice(&base);
    alone.extend(["--against", "g__Methanosarcina"]);
    assert!(failure(&alone).contains("--against needs --with"));

    // Mistakes are explained.
    let mut unknown = vec!["co-occurrence"];
    unknown.extend_from_slice(&base);
    unknown.extend(["--feature", "g__Nothing"]);
    assert!(failure(&unknown).contains("Unknown feature 'g__Nothing'"));
    let mut no_metadata = vec!["co-occurrence"];
    no_metadata.extend_from_slice(&base);
    no_metadata.extend(["--where", "Environment=Core"]);
    assert!(failure(&no_metadata).contains("need --metadata"));
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn box_plots_of_several_features_on_a_logarithmic_axis() {
    let directory =
        std::env::temp_dir().join(format!("stamp-cli-multi-box-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let has = |svg: &str, label: &str| svg.contains(&format!(">{label}</text>"));
    let features = [
        "--plot-type",
        "box",
        "--plot-feature",
        "g__Methanobacterium",
        "--plot-feature",
        "g__Pseudomonas",
        "--plot-feature",
        "g__Thauera",
    ];
    let mut extra = features.to_vec();
    extra.extend(["--box-log", "--box-points", "--box-counts"]);
    let svg = svg_plot("multi-groups", &COAL, &extra, &directory, "grouped");
    for label in [
        "g__Methanobacterium",
        "g__Thauera",
        "Core (n=8)",
        "Produced water (n=15)",
        "0",
        "0.01",
        "1",
    ] {
        assert!(has(&svg, label), "grouped: no {label:?}");
    }
    // A point per sample of each feature: 44 samples, three features.
    assert!(svg.matches("<circle").count() >= 3 * 44);

    // One panel per feature, groups ordered by another field: the three
    // core groups before the produced water.
    extra.extend([
        "--box-layout",
        "panels",
        "--box-order-field",
        "Core vs. Water",
        "--box-horizontal",
    ]);
    let svg = svg_plot("multi-groups", &COAL, &extra, &directory, "panels");
    assert_eq!(svg.matches(">Produced water (n=15)</text>").count(), 3);
    let at = |label: &str| svg.find(&format!(">{label}</text>")).unwrap();
    assert!(at("Shallow cutting (n=11)") < at("Produced water (n=15)"));

    // The most significant features when none is named; two groups too.
    let svg = svg_plot(
        "multi-groups",
        &COAL,
        &[
            "--plot-type",
            "box",
            "--box-top",
            "4",
            "--box-group-order",
            "median",
        ],
        &directory,
        "top",
    );
    assert!(has(&svg, "g__Thauera"));
    let two = svg_plot(
        "two-groups",
        &ENTEROTYPES,
        &[
            "--group1",
            "Enterotype 1",
            "--group2",
            "Enterotype 3",
            "--plot-type",
            "box",
            "--plot-feature",
            "Prevotella",
            "--plot-feature",
            "Bacteroides",
            "--box-log",
        ],
        &directory,
        "two",
    );
    assert!(has(&two, "Prevotella") && has(&two, "Bacteroides") && has(&two, "0.1"));

    // The bar plot of one feature on a logarithmic axis.
    let bar = svg_plot(
        "multi-groups",
        &COAL,
        &[
            "--plot-type",
            "bar",
            "--plot-feature",
            "g__Thauera",
            "--bar-log",
        ],
        &directory,
        "bar",
    );
    assert!(has(&bar, "0.01") && has(&bar, "g__Thauera"));

    let mut args = with("multi-groups", &COAL, &features);
    args.extend(["--plot", "x.svg", "--box-group-order", "field"]);
    assert!(failure(&args).contains("--box-order-field"));
    let mut args = with("multi-groups", &COAL, &features);
    args.extend(["--plot", "x.svg", "--box-order-field", "Nope"]);
    assert!(failure(&args).contains("unknown metadata field 'Nope'"));
    let mut args = with(
        "multi-groups",
        &COAL,
        &[
            "--plot-type",
            "bar",
            "--plot-feature",
            "g__Thauera",
            "--plot-feature",
            "g__Pseudomonas",
        ],
    );
    args.extend(["--plot", "x.svg"]);
    assert!(failure(&args).contains("shows one feature"));
}

#[test]
fn prevalence_against_mean_abundance() {
    let directory =
        std::env::temp_dir().join(format!("stamp-cli-prevalence-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let table_path = directory.join("prevalence.tsv");
    let svg = svg_plot(
        "multi-groups",
        &COAL,
        &[
            "--plot-type",
            "prevalence-abundance",
            "--highlight",
            "g__Methanobacterium",
            "--prevalence-table",
            table_path.to_str().unwrap(),
        ],
        &directory,
        "panels",
    );
    for text in [
        ">Core (n=8)</text>",
        ">Produced water (n=15)</text>",
        ">Prevalence (% of samples)</text>",
        ">Mean relative abundance (%) (log scale)</text>",
        ">g__Methanobacterium</text>",
    ] {
        assert!(svg.contains(text), "{text} is not in the plot");
    }
    let table = Table::parse(&fs::read_to_string(&table_path).unwrap());
    assert_eq!(
        table.headings,
        [
            "Feature",
            "Group",
            "Samples",
            "Samples where present",
            "Prevalence (%)",
            "Mean abundance (%)",
            "Mean abundance where present (%)"
        ]
    );
    // Four groups and all samples for each of the 631 genera.
    assert_eq!(table.rows.len(), 631 * 5);
    for row in &table.rows {
        let samples: f64 = row[2].parse().unwrap();
        let present: f64 = row[3].parse().unwrap();
        let prevalence: f64 = row[4].parse().unwrap();
        assert!((prevalence - 100.0 * present / samples).abs() < 1e-9);
        if present > 0.0 {
            let all: f64 = row[5].parse().unwrap();
            let where_present: f64 = row[6].parse().unwrap();
            assert!((where_present * present - all * samples).abs() < 1e-6 * samples.max(1.0));
        }
    }
    let water = table
        .rows
        .iter()
        .find(|r| r[0] == "g__Methanobacterium" && r[1] == "Produced water")
        .unwrap();
    assert_eq!(water[2], "15");

    // The other views, the mean where present and the sample filter.
    let svg = svg_plot(
        "multi-groups",
        &COAL,
        &[
            "--where-not",
            "Environment=Core",
            "--plot-type",
            "prevalence-abundance",
            "--prevalence-view",
            "specificity",
            "--prevalence-mean",
            "present",
            "--prevalence-min-abundance",
            "0.1",
        ],
        &directory,
        "specificity",
    );
    assert!(
        svg.contains(">Shared (specificity &lt; 50)</text>")
            || svg.contains(">Shared (specificity < 50)</text>")
    );
    assert!(svg.contains(">Mean relative abundance where present (%) (log scale)</text>"));
    assert!(svg.contains("0.1%)</text>"));
    let svg = svg_plot(
        "two-groups",
        &COAL,
        &[
            "--group1",
            "Core",
            "--plot-type",
            "prevalence-abundance",
            "--prevalence-view",
            "combined",
            "--prevalence-group",
            "Core",
        ],
        &directory,
        "combined",
    );
    assert!(svg.contains(">Core (n=8)</text>"));

    let mut args = with(
        "multi-groups",
        &COAL,
        &[
            "--plot-type",
            "prevalence-abundance",
            "--highlight",
            "g__Nothing",
        ],
    );
    args.extend(["--plot", "x.svg"]);
    assert!(failure(&args).contains("unknown feature 'g__Nothing'"));
    let mut args = with(
        "multi-groups",
        &COAL,
        &[
            "--plot-type",
            "prevalence-abundance",
            "--prevalence-group",
            "Ocean",
        ],
    );
    args.extend(["--plot", "x.svg"]);
    assert!(failure(&args).contains("not a group of the analysis"));
}

/// The PCA, PCoA and NMDS plots coloured by a number, a category or a
/// feature's abundance, and shaped by another field.
#[test]
fn ordinations_are_coloured_and_shaped_by_other_fields() {
    let directory =
        std::env::temp_dir().join(format!("stamp-cli-ordination-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let has = |svg: &str, label: &str| svg.contains(&format!(">{label}</text>"));
    // The coal metadata with a number for every sample but the first.
    let text = fs::read_to_string(repo().join("examples/CoalSeamGas/coal.metadata.tsv")).unwrap();
    let mut metadata = String::new();
    for (i, line) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let depth = match i {
            0 => "Depth".to_string(),
            1 => "NA".to_string(),
            _ => format!("{}", 100 + 13 * i),
        };
        metadata.push_str(&format!("{line}\t{depth}\n"));
    }
    let metadata_path = directory.join("coal.metadata.tsv");
    fs::write(&metadata_path, metadata).unwrap();
    let coal = [
        "--profile",
        "examples/CoalSeamGas/coal.spf",
        "--metadata",
        metadata_path.to_str().unwrap(),
        "--field",
        "Environment",
        "--profile-level",
        "Phylum",
    ];

    let depth = svg_plot(
        "multi-groups",
        &coal,
        &[
            "--plot-type",
            "pcoa",
            "--color-by",
            "field:Depth",
            "--shape-by",
            "Company",
            "--color-map",
            "magma",
        ],
        &directory,
        "pcoa-depth",
    );
    assert!(has(&depth, "Depth") && has(&depth, "No value (1)") && has(&depth, "Company"));

    let pcoa = svg_plot(
        "multi-groups",
        &coal,
        &[
            "--plot-type",
            "pcoa",
            "--color-by",
            "feature:p__Proteobacteria",
            "--color-scale",
            "log10",
            "--opacity",
            "0.7",
            "--draw-order",
            "low-on-top",
        ],
        &directory,
        "pcoa-feature",
    );
    assert!(has(&pcoa, "p__Proteobacteria (%)") && has(&pcoa, "Logarithmic scale"));
    assert!(has(
        &pcoa,
        "The tests compare the groups of Environment, not the colours"
    ));
    // The groups keep their shapes, listed under the group field.
    assert!(has(&pcoa, "Environment"));

    let nmds = svg_plot(
        "two-groups",
        &[
            "--profile",
            "examples/CoalSeamGas/coal.spf",
            "--metadata",
            metadata_path.to_str().unwrap(),
            "--field",
            "Core vs. Water",
            "--group1",
            "Core",
        ],
        &["--plot-type", "nmds", "--color-by", "category:Company"],
        &directory,
        "nmds-company",
    );
    assert!(has(&nmds, "Company") && nmds.contains(">CP ("));

    // Mistakes are reported.
    let refused = directory.join("refused.svg");
    for (extra, message) in [
        (["--color-by", "Depth"], "expected group, field:NAME"),
        (
            ["--color-by", "field:Nothing"],
            "unknown metadata field 'Nothing'",
        ),
        (
            ["--shape-by", "Nothing"],
            "unknown metadata field 'Nothing'",
        ),
        (
            ["--color-by", "feature:Nothing"],
            "unknown feature 'Nothing'",
        ),
        (["--opacity", "2"], "--opacity"),
    ] {
        let mut args = with("multi-groups", &coal, &["--plot-type", "pcoa"]);
        args.extend_from_slice(&extra);
        args.extend_from_slice(&["--plot", refused.to_str().unwrap()]);
        let text = failure(&args);
        assert!(text.contains(message), "{text}");
        assert!(!refused.exists());
    }
    fs::remove_dir_all(&directory).ok();
}

/// Progress is drawn on standard error only when it is a terminal (here it
/// is a pipe, as in scripts, logs and tests), never with --quiet, and when
/// forced (for this test) it is drawn over one line and cleared.
#[test]
fn progress_is_shown_only_on_a_terminal() {
    let dir = std::env::temp_dir().join(format!("stamp-progress-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let plot = dir.join("pcoa.svg");
    let table = dir.join("table.tsv");
    let args = [
        "multi-groups",
        "-p",
        "examples/CoalSeamGas/coal.spf",
        "-m",
        "examples/CoalSeamGas/coal.metadata.tsv",
        "-f",
        "Environment",
        "--profile-level",
        "Phylum",
        "--plot",
        plot.to_str().unwrap(),
        "--plot-type",
        "pcoa",
        "-o",
        table.to_str().unwrap(),
    ];
    let run = |force: bool, quiet: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_stamp-cli"));
        command.args(args).current_dir(repo());
        if quiet {
            command.arg("--quiet");
        }
        if force {
            command.env("STAMP_FORCE_PROGRESS", "1");
        } else {
            command.env_remove("STAMP_FORCE_PROGRESS");
        }
        let output = command.output().expect("stamp-cli runs");
        assert!(output.status.success());
        (
            String::from_utf8(output.stderr).unwrap(),
            fs::read(&table).unwrap(),
        )
    };
    let (plain, plain_table) = run(false, false);
    assert!(!plain.contains('\r'), "{plain:?}");
    assert!(!plain.contains('\x1b'), "{plain:?}");
    assert!(plain.contains("PERMANOVA: pseudo-F"), "{plain:?}");

    let (forced, forced_table) = run(true, false);
    assert!(forced.contains("\r\x1b[2K"), "{forced:?}");
    assert!(forced.contains("Reading examples/CoalSeamGas/coal.spf..."));
    assert!(forced.contains("Writing the results table"));
    // The report is the same, once the progress is taken out, and so is
    // the table.
    let cleaned: String = forced
        .split('\n')
        .map(|line| line.rsplit("\r\x1b[2K").next().unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(cleaned, plain);
    assert_eq!(forced_table, plain_table);

    let (quiet, _) = run(true, true);
    assert_eq!(quiet, plain);
    let _ = fs::remove_dir_all(&dir);
}

const ENTEROTYPES_LOCATIONS: [&str; 6] = [
    "--profile",
    "examples/EnterotypesArumugam/Enterotypes.profile.spf",
    "--metadata",
    "examples/EnterotypesArumugam/Enterotypes.metadata.locations.tsv",
    "--field",
    "Enterotype",
];

#[test]
fn samples_are_drawn_on_a_map() {
    let directory = std::env::temp_dir().join(format!("stamp-cli-map-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let svg = svg_plot(
        "multi-groups",
        &ENTEROTYPES_LOCATIONS,
        &[
            "--plot-type",
            "map",
            "--color-by",
            "feature:Bacteroides",
            "--color-scale",
            "log10",
        ],
        &directory,
        "world",
    );
    assert!(svg.contains(">39 samples at 7 sites (within 2°)</text>"));
    assert!(svg.contains(">Bacteroides (%), mean at site</text>"));
    assert!(svg.contains(">Samples at site</text>"));
    // The land, filled.
    assert!(svg.matches("<path d=").count() > 100);

    let svg = svg_plot(
        "two-groups",
        &ENTEROTYPES_LOCATIONS,
        &[
            "--group1",
            "Enterotype 1",
            "--plot-type",
            "map",
            "--map-extent",
            "europe",
            "--map-projection",
            "robinson",
            "--map-resolution",
            "50m",
            "--no-map-aggregate",
            "--map-jitter",
            "--map-size-by",
            "fixed",
            "--shape-by",
            "Gender",
        ],
        &directory,
        "europe",
    );
    assert!(svg.contains("outside the area shown</text>"), "{svg}");
    assert!(svg.contains(">10°E</text>") && svg.contains(">50°N</text>"));
    assert!(svg.contains(">Gender</text>"));
    assert!(!svg.contains("Samples at site"));

    // A custom area, and the correlation analysis (sized by a number).
    let svg = svg_plot(
        "correlation",
        &[
            "--profile",
            "examples/EnterotypesArumugam/Enterotypes.profile.spf",
            "--metadata",
            "examples/EnterotypesArumugam/Enterotypes.metadata.locations.tsv",
            "--field",
            "Latitude",
        ],
        &[
            "--plot-type",
            "map",
            "--map-extent",
            "-130,20,20,60",
            "--map-projection",
            "mercator",
            "--map-size-by",
            "field:Longitude",
            "--group-field",
            "Nationality",
            "--no-map-graticule",
            "--no-map-ocean",
        ],
        &directory,
        "custom",
    );
    assert!(svg.contains(">Nationality</text>"));
    assert!(svg.contains(">Longitude, mean at site</text>"));
    assert!(!svg.contains("°W</text>"));

    // Mistakes are explained.
    let base = with(
        "multi-groups",
        &ENTEROTYPES_LOCATIONS,
        &["--plot", "x.svg", "--plot-type", "map"],
    );
    let mut args = base.clone();
    args.extend(["--map-extent", "mars"]);
    assert!(failure(&args).contains("--map-extent 'mars'"));
    let mut args = base.clone();
    args.extend(["--map-lat-field", "Nope"]);
    assert!(failure(&args).contains("unknown metadata field 'Nope'"));
    let mut args = base;
    args.extend(["--map-size-by", "field:Gender"]);
    assert!(failure(&args).contains("does not hold numbers"));
    let message = failure(&with(
        "multi-groups",
        &ENTEROTYPES,
        &["--plot", "x.svg", "--plot-type", "map"],
    ));
    assert!(message.contains("no latitude field"), "{message}");
    fs::remove_dir_all(&directory).ok();
}
