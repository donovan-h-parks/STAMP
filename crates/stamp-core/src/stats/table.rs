//! Results as tables of text, for saving to a file or showing in a grid.
//!
//! Column headings follow STAMP v2 so that existing downstream scripts keep
//! working.

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
    if value.is_nan() {
        "nan".to_string()
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        // Whole numbers, such as counts, without a decimal point.
        format!("{value:.0}")
    } else {
        // Shortest exact form, in scientific notation when very small or large.
        format!("{value:?}")
    }
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

fn sample_cells(profile: &Profile, feature: &Feature) -> Vec<String> {
    let parents = profile.parent_counts(feature);
    feature
        .counts
        .iter()
        .zip(parents)
        .flat_map(|(&count, &parent)| [num(count), num(parent), num(percent(count, parent))])
        .collect()
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

    let rows = rows
        .into_iter()
        .map(|row| {
            let mut cells = hierarchy(&profile.profile, &row.feature);
            cells.extend(
                [
                    row.mean1,
                    row.std_dev1,
                    row.mean2,
                    row.std_dev2,
                    row.p_value,
                    row.p_value_corrected,
                    row.effect_size,
                    row.lower_ci,
                    row.upper_ci,
                ]
                .map(num),
            );
            cells.push(row.note.clone());
            if let Some(feature) = profile.profile.feature(&row.feature) {
                cells.extend(sample_cells(&profile.profile, feature));
            }
            cells
        })
        .collect();
    Table { headings, rows }
}

/// Table of multiple-group results.
pub fn multi_group_table<'a>(
    profile: &MultiGroupProfile,
    _results: &MultiGroupResults,
    rows: impl IntoIterator<Item = &'a MultiGroupRow>,
) -> Table {
    let mut headings = profile.profile.hierarchy_headings.clone();
    headings.extend(["p-values", "p-values (corrected)", "Effect size", "Note"].map(String::from));
    for group in &profile.group_names {
        headings.push(format!("{group}: mean rel. freq. (%)"));
        headings.push(format!("{group}: std. dev. (%)"));
    }
    headings.extend(sample_headings(&profile.profile));

    let rows = rows
        .into_iter()
        .map(|row| {
            let mut cells = hierarchy(&profile.profile, &row.feature);
            cells.extend([row.p_value, row.p_value_corrected, row.effect_size].map(num));
            cells.push(row.note.clone());
            for (mean, sd) in row.group_means.iter().zip(&row.group_std_devs) {
                cells.push(num(*mean));
                cells.push(num(*sd));
            }
            if let Some(feature) = profile.profile.feature(&row.feature) {
                cells.extend(sample_cells(&profile.profile, feature));
            }
            cells
        })
        .collect();
    Table { headings, rows }
}

/// Table of correlation results. `rows` selects and orders the rows.
pub fn correlation_table<'a>(
    profile: &CorrelationProfile,
    results: &CorrelationResults,
    rows: impl IntoIterator<Item = &'a CorrelationRow>,
) -> Table {
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

    let rows = rows
        .into_iter()
        .map(|row| {
            let mut cells = hierarchy(&profile.profile, &row.feature);
            cells.extend(
                [
                    row.coefficient,
                    row.p_value,
                    row.p_value_corrected,
                    row.mean,
                    row.std_dev,
                ]
                .map(num),
            );
            cells.push(row.note.clone());
            if let Some(feature) = profile.profile.feature(&row.feature) {
                cells.extend(sample_cells(&profile.profile, feature));
            }
            cells
        })
        .collect();
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

    #[test]
    fn coverage_headings() {
        assert_eq!(ci_headings(0.95), ["95.0% lower CI", "95.0% upper CI"]);
        assert_eq!(coverage_percent(0.99), "99.0");
        assert_eq!(coverage_percent(0.999), "99.9");
        assert_eq!(coverage_percent(0.9), "90.0");
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
}
