//! The post-hoc plot: every pair of groups for one feature, drawn as an
//! extended error bar plot.

use stamp_core::stats::post_hoc::{PostHocP, PostHocResult};
use stamp_core::stats::results::format_p_value;
use stamp_core::MultiGroupProfile;

use crate::axes::group_color;
use crate::extended_error_bar::{self, Data, Options, Row};
use crate::figure::{Color, Figure};

/// Build the post-hoc plot for a feature. `result` must come from a post-hoc
/// test on the same profile and feature.
///
/// With `only_significant`, pairs whose p-value is not below `1 - coverage`
/// are left out.
pub fn plot(
    profile: &MultiGroupProfile,
    feature: &str,
    result: &PostHocResult,
    coverage: f64,
    only_significant: bool,
    options: &Options,
) -> Figure {
    plot_with_colors(
        profile,
        feature,
        result,
        coverage,
        only_significant,
        &[],
        options,
    )
}

/// As [`plot`], with a colour for each group of the profile, in order. Groups
/// beyond the list use the default palette.
pub fn plot_with_colors(
    profile: &MultiGroupProfile,
    feature: &str,
    result: &PostHocResult,
    coverage: f64,
    only_significant: bool,
    colors: &[Color],
    options: &Options,
) -> Figure {
    let color = |index: usize| {
        colors
            .get(index)
            .copied()
            .unwrap_or_else(|| group_color(index))
    };
    let Some(f) = profile.profile.feature(feature) else {
        return Figure::message("No feature is selected");
    };
    let percentages = profile.percentages(f);
    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len().max(1) as f64;
    let std_dev = |values: &[f64]| {
        let m = mean(values);
        (values.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / values.len().max(1) as f64).sqrt()
    };

    let alpha = 1.0 - coverage;
    let mut rows = Vec::new();
    let mut comparisons = result.comparisons.iter();
    for i in 0..profile.group_names.len() {
        for j in i + 1..profile.group_names.len() {
            let Some(comparison) = comparisons.next() else {
                break;
            };
            let PostHocP::Value(p) = comparison.p_value;
            // A comparison that could not be made has no p-value.
            let (p_text, significant, p_numeric) = if p.is_nan() {
                ("n/a".to_string(), false, 1.0)
            } else {
                (format_p_value(p, -15), p < alpha, p)
            };
            if only_significant && !significant {
                continue;
            }
            rows.push(Row {
                feature: comparison.label.clone(),
                value1: mean(&percentages[i]),
                value2: mean(&percentages[j]),
                std_dev1: std_dev(&percentages[i]),
                std_dev2: std_dev(&percentages[j]),
                effect_size: comparison.effect_size,
                lower_ci: comparison.lower_ci,
                upper_ci: comparison.upper_ci,
                p_value: p_numeric,
                p_value_corrected: p_numeric,
                p_text: Some(p_text),
                colors: Some((color(i), color(j))),
            });
        }
    }
    if rows.is_empty() {
        return Figure::message("No pair of groups differs significantly");
    }

    let data = Data {
        rows,
        name1: String::new(),
        name2: String::new(),
        color1: group_color(0),
        color2: group_color(1),
        bar_label: "Mean proportion (%)".to_string(),
        bar_is_count: false,
        effect_label: "Difference in mean proportions (%)".to_string(),
        is_ratio: false,
        coverage,
        false_discovery_rate: false,
        corrected: false,
        heading: Some(crate::text::truncate(
            feature,
            options.truncate_labels.unwrap_or(usize::MAX - 3),
        )),
    };
    // The legend names two fixed groups, which does not apply here.
    let options = Options {
        legend: extended_error_bar::Legend::None,
        ..options.clone()
    };
    extended_error_bar::plot(&data, &options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use stamp_core::stats::post_hoc::PostHocTest;
    use stamp_core::stats::results::run_post_hoc;
    use stamp_core::{Metadata, ProfileTree, Unclassified};

    fn profile() -> MultiGroupProfile {
        let spf = "F\ta1\ta2\ta3\tb1\tb2\tb3\tc1\tc2\tc3\n\
            x\t10\t11\t12\t30\t31\t33\t11\t12\t10\n\
            y\t90\t89\t88\t70\t69\t67\t89\t88\t90\n";
        let meta = "id\tg\na1\tA\na2\tA\na3\tA\nb1\tB\nb2\tB\nb3\tB\nc1\tC\nc2\tC\nc3\tC\n";
        let tree = ProfileTree::read(spf).unwrap();
        let (meta, _) = Metadata::read(meta, &tree).unwrap();
        tree.multi_group_profile(
            &meta.groups("g").unwrap(),
            None,
            None,
            "F",
            Unclassified::Retain,
        )
        .unwrap()
    }

    fn texts(figure: &Figure) -> Vec<String> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn shows_each_pair_with_its_groups_colours() {
        let profile = profile();
        let result = run_post_hoc(&profile, "x", PostHocTest::TukeyKramer, 0.95).unwrap();
        let figure = plot(&profile, "x", &result, 0.95, false, &Options::default());
        let texts = texts(&figure);
        for expected in [
            "x", "A : B", "A : C", "B : C",
            // Computed p-values, where STAMP v2 showed "< 0.001" and
            // ">= 0.1".
            "2.04e-6", "1.000", "p-value",
        ] {
            assert!(
                texts.iter().any(|t| t == expected),
                "missing {expected} in {texts:?}"
            );
        }
        // Bars of pair A : C use the first and third group colours.
        let has = |color| {
            figure
                .shapes
                .iter()
                .any(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == color))
        };
        assert!(has(group_color(0)) && has(group_color(1)) && has(group_color(2)));

        let significant = plot(&profile, "x", &result, 0.95, true, &Options::default());
        assert_eq!(significant.regions.len(), 2);
        assert!(!self::texts(&significant).contains(&"A : C".to_string()));
    }

    #[test]
    fn exact_p_values_and_empty_results() {
        let profile = profile();
        let result = run_post_hoc(&profile, "x", PostHocTest::Scheffe, 0.95).unwrap();
        let figure = plot(&profile, "x", &result, 0.95, true, &Options::default());
        assert_eq!(figure.regions.len(), 2);

        let none = plot(
            &profile,
            "x",
            &result,
            0.999999999999,
            true,
            &Options::default(),
        );
        assert_eq!(texts(&none), ["No pair of groups differs significantly"]);
        let missing = plot(&profile, "nope", &result, 0.95, false, &Options::default());
        assert_eq!(texts(&missing), ["No feature is selected"]);
    }
}
