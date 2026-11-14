//! Results as tables of text, for saving to a file or showing in a grid.
//!
//! Column headings follow STAMP v2 so that existing downstream scripts keep
//! working.

use super::ancom::AncomResult;
use super::co_occurrence::{CoOccurrence, Partner};
use super::correlation::CorrelationMethod;
use super::post_hoc::PostHocResult;
use super::results::{
    CorrelationResults, CorrelationRow, GroupResults, GroupRow, MultiGroupResults, MultiGroupRow,
    SampleResults, SampleRow,
};
use crate::profile::{
    percent, CorrelationProfile, Feature, GroupProfile, MultiGroupProfile, Profile, SampleProfile,
};

/// A table of text cells with one heading per column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub headings: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// Tab-separated text, one line per row, headings first.
    pub fn to_tsv(&self) -> String {
        let mut out = self.headings.join("\t");
        out.push('\n');
        for row in &self.rows {
            out.push_str(&row.join("\t"));
            out.push('\n');
        }
        out
    }
}

/// Format a number so that it reads back exactly.
fn num(value: f64) -> String {
    let mut text = String::new();
    write_num(&mut text, value);
    text
}

/// [`num`], written at the end of `out`.
fn write_num(out: &mut String, value: f64) {
    use std::fmt::Write;
    if value.is_nan() {
        out.push_str("nan");
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        // Whole numbers, such as counts, without a decimal point; written
        // as integers, which is quicker than formatting a float to no
        // places and gives the same text (but for negative zero).
        if value == 0.0 && value.is_sign_negative() {
            out.push_str("-0");
        } else {
            let _ = write!(out, "{}", value as i64);
        }
    } else {
        // Shortest exact form, in scientific notation when very small or large.
        let _ = write!(out, "{value:?}");
    }
}

/// Where the cells of a row go: kept as text ([`Vec<String>`]), or written
/// straight into a line of tab-separated text ([`Line`]).
trait Cells {
    fn text(&mut self, text: &str);
    fn num(&mut self, value: f64);
}

impl Cells for Vec<String> {
    fn text(&mut self, text: &str) {
        self.push(text.to_string());
    }

    fn num(&mut self, value: f64) {
        self.push(num(value));
    }
}

/// A row as a line of tab-separated text, built without keeping its cells.
#[derive(Default)]
struct Line {
    text: String,
    cells: usize,
}

impl Line {
    fn separate(&mut self) {
        if self.cells > 0 {
            self.text.push('\t');
        }
        self.cells += 1;
    }
}

impl Cells for Line {
    fn text(&mut self, text: &str) {
        self.separate();
        self.text.push_str(text);
    }

    fn num(&mut self, value: f64) {
        self.separate();
        write_num(&mut self.text, value);
    }
}

/// Write a table as [`Table::to_tsv`] would, row by row, without holding
/// its cells: tables with a column for every sample run to millions of
/// cells. `progress` is told `(rows written, rows)` before each row and at
/// the end.
fn write_rows<R>(
    out: &mut dyn std::io::Write,
    headings: &[String],
    rows: &[&R],
    cells: impl Fn(&R, &mut Line),
    progress: &mut dyn FnMut(usize, usize),
) -> std::io::Result<()> {
    out.write_all(headings.join("\t").as_bytes())?;
    out.write_all(b"\n")?;
    let mut line = Line::default();
    for (index, row) in rows.iter().enumerate() {
        progress(index, rows.len());
        line.text.clear();
        line.cells = 0;
        cells(row, &mut line);
        line.text.push('\n');
        out.write_all(line.text.as_bytes())?;
    }
    progress(rows.len(), rows.len());
    Ok(())
}

/// A table of rows built by `cells`.
fn collect_rows<'a, R: 'a>(
    headings: Vec<String>,
    rows: impl IntoIterator<Item = &'a R>,
    cells: impl Fn(&R, &mut Vec<String>),
) -> Table {
    let rows = rows
        .into_iter()
        .map(|row| {
            let mut out = Vec::new();
            cells(row, &mut out);
            out
        })
        .collect();
    Table { headings, rows }
}

/// Coverage as a percentage with at least one decimal: `95.0`, `99.9`.
fn coverage_percent(coverage: f64) -> String {
    let percent = (coverage * 100.0 * 1e9).round() / 1e9;
    if percent.fract() == 0.0 {
        format!("{percent:.1}")
    } else {
        percent.to_string()
    }
}

fn ci_headings(coverage: f64) -> [String; 2] {
    let percent = coverage_percent(coverage);
    [
        format!("{percent}% lower CI"),
        format!("{percent}% upper CI"),
    ]
}

/// Count, parent count and relative frequency columns for every sample.
fn sample_headings(profile: &Profile) -> Vec<String> {
    profile
        .sample_names
        .iter()
        .flat_map(|s| {
            [
                s.clone(),
                format!("{s}: parent seq. count"),
                format!("{s}: rel. freq. (%)"),
            ]
        })
        .collect()
}

fn sample_cells(profile: &Profile, feature: &Feature, out: &mut impl Cells) {
    let parents = profile.parent_counts(feature);
    for (&count, &parent) in feature.counts.iter().zip(parents) {
        out.num(count);
        out.num(parent);
        out.num(percent(count, parent));
    }
}

/// The leading columns of a result row: its feature's hierarchy.
fn hierarchy_cells(profile: &Profile, feature: &str, out: &mut impl Cells) {
    match profile.feature(feature) {
        Some(f) => {
            for name in &f.hierarchy {
                out.text(name);
            }
        }
        None => {
            for cell in hierarchy(profile, feature) {
                out.text(&cell);
            }
        }
    }
}

/// The hierarchy of a result row's feature, for the leading columns.
fn hierarchy(profile: &Profile, feature: &str) -> Vec<String> {
    profile
        .feature(feature)
        .map(|f| f.hierarchy.clone())
        .unwrap_or_else(|| {
            let mut cells = vec![String::new(); profile.hierarchy_headings.len()];
            if let Some(last) = cells.last_mut() {
                *last = feature.to_string();
            }
            cells
        })
}

/// Table of two-sample results. `rows` selects and orders the rows, for
/// example the output of [`SampleResults::filter`] or all of `results.rows`.
pub fn sample_table<'a>(
    profile: &SampleProfile,
    results: &SampleResults,
    rows: impl IntoIterator<Item = &'a SampleRow>,
) -> Table {
    let [s1, s2] = [
        &profile.profile.sample_names[0],
        &profile.profile.sample_names[1],
    ];
    let mut headings = profile.profile.hierarchy_headings.clone();
    headings.extend([
        s1.clone(),
        s2.clone(),
        format!("{s1}: parent seq. count"),
        format!("{s2}: parent seq. count"),
        format!("{s1}: rel. freq. (%)"),
        format!("{s2}: rel. freq. (%)"),
        "p-values".to_string(),
        "p-values (corrected)".to_string(),
        "Effect size".to_string(),
    ]);
    headings.extend(ci_headings(results.analysis.coverage));
    headings.push("Note".to_string());

    let rows = rows
        .into_iter()
        .map(|row| {
            let mut cells = hierarchy(&profile.profile, &row.feature);
            cells.extend(
                [
                    row.seq1,
                    row.seq2,
                    row.parent_seq1,
                    row.parent_seq2,
                    row.rel_freq1,
                    row.rel_freq2,
                    row.p_value,
                    row.p_value_corrected,
                    row.effect_size,
                    row.lower_ci,
                    row.upper_ci,
                ]
                .map(num),
            );
            cells.push(row.note.clone());
            cells
        })
        .collect();
    Table { headings, rows }
}

/// Table of two-group results.
pub fn group_table<'a>(
    profile: &GroupProfile,
    results: &GroupResults,
    rows: impl IntoIterator<Item = &'a GroupRow>,
) -> Table {
    collect_rows(group_headings(profile, results), rows, |row, out| {
        group_cells(profile, row, out)
    })
}

/// [`group_table`] written as tab-separated text, row by row.
pub fn write_group_table(
    out: &mut dyn std::io::Write,
    profile: &GroupProfile,
    results: &GroupResults,
    rows: &[&GroupRow],
    progress: &mut dyn FnMut(usize, usize),
) -> std::io::Result<()> {
    let headings = group_headings(profile, results);
    write_rows(
        out,
        &headings,
        rows,
        |row, line| group_cells(profile, row, line),
        progress,
    )
}

fn group_headings(profile: &GroupProfile, results: &GroupResults) -> Vec<String> {
    let (g1, g2) = (&profile.group_name1, &profile.group_name2);
    let mut headings = profile.profile.hierarchy_headings.clone();
    headings.extend([
        format!("{g1}: mean rel. freq. (%)"),
        format!("{g1}: std. dev. (%)"),
        format!("{g2}: mean rel. freq. (%)"),
        format!("{g2}: std. dev. (%)"),
        "p-values".to_string(),
        "p-values (corrected)".to_string(),
        "Difference between means".to_string(),
    ]);
    headings.extend(ci_headings(results.analysis.coverage));
    headings.push("Note".to_string());
    headings.extend(sample_headings(&profile.profile));
    headings
}

fn group_cells(profile: &GroupProfile, row: &GroupRow, out: &mut impl Cells) {
    hierarchy_cells(&profile.profile, &row.feature, out);
    for value in [
        row.mean1,
        row.std_dev1,
        row.mean2,
        row.std_dev2,
        row.p_value,
        row.p_value_corrected,
        row.effect_size,
        row.lower_ci,
        row.upper_ci,
    ] {
        out.num(value);
    }
    out.text(&row.note);
    if let Some(feature) = profile.profile.feature(&row.feature) {
        sample_cells(&profile.profile, feature, out);
    }
}

/// Table of multiple-group results.
pub fn multi_group_table<'a>(
    profile: &MultiGroupProfile,
    _results: &MultiGroupResults,
    rows: impl IntoIterator<Item = &'a MultiGroupRow>,
) -> Table {
    collect_rows(multi_group_headings(profile), rows, |row, out| {
        multi_group_cells(profile, row, out)
    })
}

/// [`multi_group_table`] written as tab-separated text, row by row.
pub fn write_multi_group_table(
    out: &mut dyn std::io::Write,
    profile: &MultiGroupProfile,
    _results: &MultiGroupResults,
    rows: &[&MultiGroupRow],
    progress: &mut dyn FnMut(usize, usize),
) -> std::io::Result<()> {
    let headings = multi_group_headings(profile);
    write_rows(
        out,
        &headings,
        rows,
        |row, line| multi_group_cells(profile, row, line),
        progress,
    )
}

fn multi_group_headings(profile: &MultiGroupProfile) -> Vec<String> {
    let mut headings = profile.profile.hierarchy_headings.clone();
    headings.extend(["p-values", "p-values (corrected)", "Effect size", "Note"].map(String::from));
    for group in &profile.group_names {
        headings.push(format!("{group}: mean rel. freq. (%)"));
        headings.push(format!("{group}: std. dev. (%)"));
    }
    headings.extend(sample_headings(&profile.profile));
    headings
}

fn multi_group_cells(profile: &MultiGroupProfile, row: &MultiGroupRow, out: &mut impl Cells) {
    hierarchy_cells(&profile.profile, &row.feature, out);
    for value in [row.p_value, row.p_value_corrected, row.effect_size] {
        out.num(value);
    }
    out.text(&row.note);
    for (mean, sd) in row.group_means.iter().zip(&row.group_std_devs) {
        out.num(*mean);
        out.num(*sd);
    }
    if let Some(feature) = profile.profile.feature(&row.feature) {
        sample_cells(&profile.profile, feature, out);
    }
}

/// Table of correlation results. `rows` selects and orders the rows.
pub fn correlation_table<'a>(
    profile: &CorrelationProfile,
    results: &CorrelationResults,
    rows: impl IntoIterator<Item = &'a CorrelationRow>,
) -> Table {
    collect_rows(correlation_headings(profile, results), rows, |row, out| {
        correlation_cells(profile, row, out)
    })
}

/// [`correlation_table`] written as tab-separated text, row by row.
pub fn write_correlation_table(
    out: &mut dyn std::io::Write,
    profile: &CorrelationProfile,
    results: &CorrelationResults,
    rows: &[&CorrelationRow],
    progress: &mut dyn FnMut(usize, usize),
) -> std::io::Result<()> {
    let headings = correlation_headings(profile, results);
    write_rows(
        out,
        &headings,
        rows,
        |row, line| correlation_cells(profile, row, line),
        progress,
    )
}

fn correlation_headings(profile: &CorrelationProfile, results: &CorrelationResults) -> Vec<String> {
    let method = results.analysis.method;
    let mut headings = profile.profile.hierarchy_headings.clone();
    headings.extend([
        format!("{} ({})", method.name(), method.coefficient_name()),
        "p-values".to_string(),
        "p-values (corrected)".to_string(),
        "Mean rel. freq. (%)".to_string(),
        "Std. dev. (%)".to_string(),
        "Note".to_string(),
    ]);
    headings.extend(sample_headings(&profile.profile));
    headings
}

fn correlation_cells(profile: &CorrelationProfile, row: &CorrelationRow, out: &mut impl Cells) {
    hierarchy_cells(&profile.profile, &row.feature, out);
    for value in [
        row.coefficient,
        row.p_value,
        row.p_value_corrected,
        row.mean,
        row.std_dev,
    ] {
        out.num(value);
    }
    out.text(&row.note);
    if let Some(feature) = profile.profile.feature(&row.feature) {
        sample_cells(&profile.profile, feature, out);
    }
}

/// Table of ANCOM results for the features of `profile`, most different
/// first (by W). `groups` is the number of groups compared.
pub fn ancom_table(profile: &Profile, result: &AncomResult, groups: usize) -> Table {
    let mut headings = profile.hierarchy_headings.clone();
    headings.extend([
        "W".to_string(),
        "Reject null hypothesis".to_string(),
        AncomResult::effect_name(groups).to_string(),
    ]);
    let mut order: Vec<usize> = (0..result.rows.len()).collect();
    order.sort_by(|&a, &b| {
        result.rows[b]
            .w
            .cmp(&result.rows[a].w)
            .then_with(|| profile.features()[a].name.cmp(&profile.features()[b].name))
    });
    let rows = order
        .into_iter()
        .filter_map(|i| {
            let feature = profile.features().get(i)?;
            let row = &result.rows[i];
            let mut cells = hierarchy(profile, &feature.name);
            cells.push(row.w.to_string());
            cells.push(if row.reject { "True" } else { "False" }.to_string());
            cells.push(num(row.clr_effect));
            Some(cells)
        })
        .collect();
    Table { headings, rows }
}

/// Long table of the correlations between every pair of features: one row
/// per pair, strongest first (q-value, then distance from zero). Pairs
/// with no correlation (a feature that does not vary) come last.
pub fn co_occurrence_table(result: &CoOccurrence) -> Table {
    let coefficient = result.method.coefficient_name();
    let headings = vec![
        "Feature 1".to_string(),
        "Feature 2".to_string(),
        coefficient.to_string(),
        "p-value".to_string(),
        "q-value (Benjamini-Hochberg)".to_string(),
    ];
    let n = result.features.len();
    let mut pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .collect();
    let key = |&(i, j): &(usize, usize)| {
        let q = result.q_values[i][j];
        (
            if q.is_nan() { f64::INFINITY } else { q },
            -result.coefficients[i][j].abs(),
        )
    };
    pairs.sort_by(|a, b| {
        let (qa, ra) = key(a);
        let (qb, rb) = key(b);
        qa.total_cmp(&qb)
            .then(ra.total_cmp(&rb))
            .then_with(|| a.cmp(b))
    });
    let rows = pairs
        .into_iter()
        .map(|(i, j)| {
            vec![
                result.features[i].clone(),
                result.features[j].clone(),
                num(result.coefficients[i][j]),
                num(result.p_values[i][j]),
                num(result.q_values[i][j]),
            ]
        })
        .collect();
    Table { headings, rows }
}

/// Table of the correlations of one feature with every other, strongest
/// first.
pub fn partner_table(feature: &str, method: CorrelationMethod, partners: &[Partner]) -> Table {
    let headings = vec![
        "Feature 1".to_string(),
        "Feature 2".to_string(),
        method.coefficient_name().to_string(),
        "p-value".to_string(),
        "q-value (Benjamini-Hochberg)".to_string(),
    ];
    let rows = partners
        .iter()
        .map(|p| {
            vec![
                feature.to_string(),
                p.feature.clone(),
                num(p.coefficient),
                num(p.p_value),
                num(p.q_value),
            ]
        })
        .collect();
    Table { headings, rows }
}

/// Table of a summary of features within groups: one row per feature, one
/// column per group (headed by the group's name), cells from
/// `values[feature][group]`.
pub fn group_summary_table(
    statistic: &str,
    features: &[String],
    groups: &[String],
    values: &[Vec<f64>],
) -> Table {
    let mut headings = vec![format!("Feature ({statistic})")];
    headings.extend(groups.iter().cloned());
    let rows = features
        .iter()
        .zip(values)
        .map(|(feature, row)| {
            let mut cells = vec![feature.clone()];
            cells.extend(row.iter().map(|&v| num(v)));
            cells
        })
        .collect();
    Table { headings, rows }
}

/// Table of the prevalence and mean abundance of every feature in every
/// group (`groups` names them, in the order of each feature's groups) and
/// over all samples: one row per feature and group.
pub fn prevalence_table(
    features: &[crate::stats::prevalence::FeatureOccupancy],
    groups: &[String],
) -> Table {
    let headings = [
        "Feature",
        "Group",
        "Samples",
        "Samples where present",
        "Prevalence (%)",
        "Mean abundance (%)",
        "Mean abundance where present (%)",
    ]
    .map(String::from)
    .to_vec();
    let mut rows = Vec::new();
    for feature in features {
        let named = groups
            .iter()
            .map(String::as_str)
            .zip(&feature.groups)
            .chain(std::iter::once(("All samples", &feature.overall)));
        for (group, o) in named {
            rows.push(vec![
                feature.feature.clone(),
                group.to_string(),
                o.samples.to_string(),
                o.present.to_string(),
                num(o.prevalence),
                num(o.mean_all),
                num(o.mean_present),
            ]);
        }
    }
    Table { headings, rows }
}

/// Table of the pairwise comparisons of a post-hoc test on one feature.
pub fn post_hoc_table(feature: &str, result: &PostHocResult, coverage: f64) -> Table {
    let mut headings: Vec<String> = ["Feature", "Groups", "p-value", "Effect size"]
        .map(String::from)
        .to_vec();
    headings.extend(ci_headings(coverage));
    headings.push("Note".to_string());

    let rows = result
        .comparisons
        .iter()
        .map(|c| {
            vec![
                feature.to_string(),
                c.label.clone(),
                c.p_value.to_string(),
                num(c.effect_size),
                num(c.lower_ci),
                num(c.upper_ci),
                result.note.to_string(),
            ]
        })
        .collect();
    Table { headings, rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tables with a column for every sample, written row by row, are
    /// exactly the text of the tables built whole.
    #[test]
    fn tables_written_row_by_row_are_the_same_text() {
        use crate::stats::correction::Correction;
        use crate::stats::correlation::CorrelationMethod;
        use crate::stats::multi_group::{MultiGroupEffectSize, MultiGroupTest};
        use crate::stats::results::*;
        use crate::stats::two_group::GroupTest;
        use crate::stats::{rng_from_seed, StatsSettings, TestType};
        use crate::{Metadata, ProfileTree, SecondGroup, Unclassified};

        let mut spf = String::from("Phylum\tGenus\tA1\tA2\tA3\tB1\tB2\tB3\tC1\tC2\n");
        for row in 0..9 {
            spf.push_str(&format!("P{}\tg{row}", row % 3));
            for column in 0..8 {
                // Fractions and zeros, and one feature the same everywhere.
                let value = if row == 4 {
                    7.0
                } else {
                    ((row * 7 + column * 13 + row * column) % 11) as f64 * 1.25
                };
                spf.push_str(&format!("\t{value}"));
            }
            spf.push('\n');
        }
        let tree = ProfileTree::read(&spf).unwrap();
        let metadata = "Id\tGroup\tDepth\nA1\tA\t1\nA2\tA\t2.5\nA3\tA\t3\nB1\tB\t4\n\
                        B2\tB\t5\nB3\tB\t6\nC1\tC\t7.25\nC2\tC\t8\n";
        let (metadata, _) = Metadata::read(metadata, &tree).unwrap();
        let groups = metadata.groups("Group").unwrap();
        type Write<'a> = &'a dyn Fn(&mut Vec<u8>, &mut dyn FnMut(usize, usize));
        let written = |write: Write<'_>| {
            let mut out = Vec::new();
            let mut reports = Vec::new();
            write(&mut out, &mut |done, total| reports.push((done, total)));
            // Progress goes from the first row to all of them.
            assert!(reports.windows(2).all(|w| w[0].0 < w[1].0));
            assert_eq!(reports.last().map(|r| r.0 == r.1), Some(true));
            String::from_utf8(out).unwrap()
        };

        let profile = tree
            .group_profile(
                &groups,
                "A",
                SecondGroup::AllOtherSamples,
                None,
                "Genus",
                Unclassified::Retain,
            )
            .unwrap();
        let analysis = GroupAnalysis {
            test: GroupTest::Welch,
            test_type: TestType::TwoSided,
            coverage: 0.95,
            correction: Correction::None,
            transform: Transform::None,
        };
        let results = run_group_analysis(
            &profile,
            analysis,
            &StatsSettings::default(),
            &mut rng_from_seed(1),
            &mut |_, _| true,
        )
        .unwrap();
        let rows: Vec<&GroupRow> = results.rows.iter().collect();
        assert_eq!(
            written(&|out, progress| {
                write_group_table(out, &profile, &results, &rows, progress).unwrap()
            }),
            group_table(&profile, &results, rows.iter().copied()).to_tsv()
        );

        let profile = tree
            .multi_group_profile(&groups, None, None, "Genus", Unclassified::Retain)
            .unwrap();
        let analysis = MultiGroupAnalysis {
            test: MultiGroupTest::Anova,
            effect_size: MultiGroupEffectSize::EtaSquared,
            correction: Correction::BenjaminiHochberg,
            transform: Transform::None,
        };
        let results =
            run_multi_group_analysis(&profile, analysis, &mut rng_from_seed(1), &mut |_, _| true)
                .unwrap();
        let rows: Vec<&MultiGroupRow> = results.rows.iter().rev().collect();
        assert_eq!(
            written(&|out, progress| {
                write_multi_group_table(out, &profile, &results, &rows, progress).unwrap()
            }),
            multi_group_table(&profile, &results, rows.iter().copied()).to_tsv()
        );

        let profile = tree
            .correlation_profile(&metadata, "Depth", None, "Genus", Unclassified::Retain)
            .unwrap();
        let analysis = CorrelationAnalysis {
            method: CorrelationMethod::Pearson,
            correction: Correction::None,
        };
        let results =
            run_correlation_analysis(&profile, analysis, &mut rng_from_seed(1), &mut |_, _| true)
                .unwrap();
        let rows: Vec<&CorrelationRow> = results.rows.iter().collect();
        let text = written(&|out, progress| {
            write_correlation_table(out, &profile, &results, &rows, progress).unwrap()
        });
        assert_eq!(
            text,
            correlation_table(&profile, &results, rows.iter().copied()).to_tsv()
        );
    }

    #[test]
    fn coverage_headings() {
        assert_eq!(ci_headings(0.95), ["95.0% lower CI", "95.0% upper CI"]);
        assert_eq!(coverage_percent(0.99), "99.0");
        assert_eq!(coverage_percent(0.999), "99.9");
        assert_eq!(coverage_percent(0.9), "90.0");
    }

    #[test]
    fn whole_numbers_are_written_as_before() {
        for value in [
            0.0,
            -0.0,
            1.0,
            -7.0,
            123456789.0,
            999_999_999_999_999.0,
            -4.5e14,
        ] {
            assert_eq!(num(value), format!("{value:.0}"));
        }
    }

    #[test]
    fn numbers_read_back_exactly() {
        for value in [
            0.1,
            1.0 / 3.0,
            5.0,
            -3.0,
            0.0,
            1e-300,
            2.5e20,
            f64::INFINITY,
        ] {
            assert_eq!(num(value).parse::<f64>().unwrap(), value);
        }
        assert_eq!(num(5.0), "5");
        assert_eq!(num(1.25e-18), "1.25e-18");
        assert_eq!(num(f64::INFINITY), "inf");
        assert_eq!(num(f64::NAN), "nan");
    }

    #[test]
    fn tsv_layout() {
        let table = Table {
            headings: vec!["a".into(), "b".into()],
            rows: vec![
                vec!["1".into(), "x".into()],
                vec!["2".into(), String::new()],
            ],
        };
        assert_eq!(table.to_tsv(), "a\tb\n1\tx\n2\t\n");
    }

    #[test]
    fn co_occurrence_and_group_summary_tables() {
        use crate::stats::co_occurrence::co_occurrence;
        let rows = vec![
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            vec![2.0, 1.0, 4.0, 3.0, 6.0],
            vec![5.0, 4.0, 3.0, 2.0, 1.0],
            vec![1.0, 1.0, 1.0, 1.0, 1.0],
        ];
        let names: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let result = co_occurrence(names.clone(), &rows, CorrelationMethod::Spearman);
        let table = co_occurrence_table(&result);
        assert_eq!(table.headings[2], "rho");
        assert_eq!(table.rows.len(), 6);
        // a and c are perfectly anti-correlated: first. d does not vary: last.
        assert_eq!(table.rows[0][..3], ["a", "c", "-1"]);
        assert!(table.rows[3..].iter().all(|r| r[1] == "d" && r[2] == "nan"));

        let summary = group_summary_table(
            "Mean",
            &names[..2],
            &["G1".to_string(), "G2".to_string()],
            &[vec![1.5, 0.0], vec![2.0, f64::NAN]],
        );
        assert_eq!(
            summary.to_tsv(),
            "Feature (Mean)\tG1\tG2\na\t1.5\t0\nb\t2\tnan\n"
        );
    }
}
