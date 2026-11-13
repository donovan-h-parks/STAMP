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
        if case.mode != "multiple_groups" {
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
            assert_eq!(row[note], reference[note], "{}", case.id);
            for &column in &columns {
                let context = format!(
                    "{} {:?} {}",
                    case.id,
                    &row[..case.key_columns],
                    actual.headings[column]
                );
                close(&context, &reference[column], &row[column]);
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
    assert!(svg.contains(">p-value (corrected)</text>"));
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
        ("p-value-histogram", "p-value (corrected)"),
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
    let pca = svg_plot(
        "two-groups",
        &groups,
        &["--plot-type", "pca"],
        &directory,
        "g-pca",
    );
    assert!(pca.contains(">PC1 (") && pca.contains(">PC3 ("));
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
    for kind in ["pca", "bar", "box"] {
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

    // Plot types that do not apply are refused, with nothing written.
    let refused = directory.join("refused.svg");
    let message = failure(&with(
        "two-samples",
        &iron,
        &["--plot-type", "pca", "--plot", refused.to_str().unwrap()],
    ));
    assert!(message.contains("the pca plot is not available for two samples"));
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
