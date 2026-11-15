//! The current session, results and plot as text for a language model:
//! what "Copy description for AI" puts on the clipboard and what the
//! assistant's tools return.
//!
//! Plots are described from their [`Figure`]: its text (titles, axis
//! labels, legends) and the item and tooltip of each region. That works for
//! every plot without code particular to each.

use std::fmt::Write as _;

use stamp_core::profile::Unclassified;
use stamp_plot::{Figure, Shape};

use crate::plots::PlotKind;
use crate::session::Mode;
use crate::StampApp;

/// Rows of the results table in a full description.
pub const DEFAULT_RESULT_ROWS: usize = 50;
/// Items of a plot listed at most, so that a heatmap of thousands of cells
/// does not swamp the reader.
pub const MAX_PLOT_ITEMS: usize = 300;
/// Lines of text of a plot listed at most.
const MAX_PLOT_TEXT: usize = 200;

/// Everything at once: the session, the current plot and the top of the
/// results table.
pub fn everything(app: &mut StampApp) -> String {
    let mut out = String::new();
    out.push_str(
        "The following describes an analysis in STAMP (statistical analysis of taxonomic \
         and functional profiles). Use it to answer questions about the data and the plot.\n\n",
    );
    out.push_str(&session(app));
    out.push('\n');
    out.push_str(&plot(app, MAX_PLOT_ITEMS));
    out.push('\n');
    out.push_str(&results(app, DEFAULT_RESULT_ROWS, None));
    out
}

/// The loaded data and the settings of the current analysis, as Markdown.
pub fn session(app: &StampApp) -> String {
    let s = &app.session;
    let mut out = String::from("# STAMP session\n\n");
    let Some(tree) = &s.tree else {
        out.push_str("No profile is loaded.\n");
        return out;
    };
    let _ = writeln!(out, "- Profile file: {}", s.profile_name);
    let _ = writeln!(
        out,
        "- Samples in the profile: {}; hierarchy: {}",
        tree.num_samples(),
        tree.hierarchy_headings().join(" > ")
    );
    let _ = writeln!(out, "- Profile level: {}", s.profile_level);
    let _ = writeln!(
        out,
        "- Parent level: {}",
        s.parent_level.as_deref().unwrap_or("Entire sample")
    );
    let _ = writeln!(out, "- Unclassified: {}", unclassified(s.unclassified));
    match &s.metadata {
        Some(metadata) => {
            let _ = writeln!(out, "- Metadata file: {}", s.metadata_name);
            let fields: Vec<String> = metadata
                .fields()
                .iter()
                .map(|f| {
                    if metadata.is_numeric(f) {
                        format!("{f} (numeric)")
                    } else {
                        f.clone()
                    }
                })
                .collect();
            let _ = writeln!(out, "- Metadata fields: {}", fields.join(", "));
            let _ = writeln!(
                out,
                "- Samples switched on: {} of {}",
                metadata.active_samples().len(),
                metadata.samples().len()
            );
        }
        None => out.push_str("- No metadata loaded\n"),
    }
    if let Some(groups) = s.grouping() {
        let sizes: Vec<String> = groups
            .names()
            .map(|g| {
                format!(
                    "{g} ({} samples, colour {})",
                    groups.samples(g).map_or(0, <[String]>::len),
                    s.group_color(g).to_hex()
                )
            })
            .collect();
        let _ = writeln!(
            out,
            "- Group field: {}; groups (samples switched on, colour in plots): {}",
            groups.field,
            sizes.join(", ")
        );
        let _ = writeln!(
            out,
            "- Colour of \"All other samples\": {}",
            s.other_samples_color.to_hex()
        );
    }

    let _ = writeln!(out, "\n## Analysis: {}\n", app.mode.name());
    match app.mode {
        Mode::TwoSamples => {
            let st = &s.samples;
            let a = &st.analysis;
            let _ = writeln!(
                out,
                "- Sample 1: {}\n- Sample 2: {}",
                st.sample1, st.sample2
            );
            let _ = writeln!(
                out,
                "- Test: {} ({})\n- Confidence interval: {} at {}%",
                a.test.name(),
                a.test_type.name(),
                a.ci_method.name(),
                percent(a.coverage)
            );
            let _ = writeln!(out, "- Multiple test correction: {}", a.correction.name());
            significance(&mut out, st.filter.significance);
        }
        Mode::TwoGroups => {
            let st = &s.groups;
            let a = &st.analysis;
            let _ = writeln!(
                out,
                "- Group 1: {}\n- Group 2: {}",
                st.group1,
                st.group2.as_deref().unwrap_or("all other samples")
            );
            if let Some(field) = &st.pair_field {
                let _ = writeln!(out, "- Samples paired by: {field}");
            }
            let _ = writeln!(
                out,
                "- Test: {} ({})\n- Confidence interval coverage: {}%",
                a.test.name(),
                a.test_type.name(),
                percent(a.coverage)
            );
            let _ = writeln!(out, "- Transform for the test: {}", a.transform.name());
            let _ = writeln!(out, "- Multiple test correction: {}", a.correction.name());
            significance(&mut out, st.filter.significance);
        }
        Mode::MultipleGroups => {
            let st = &s.multi;
            let a = &st.analysis;
            let _ = writeln!(out, "- Groups compared: {}", st.active_groups.join(", "));
            let _ = writeln!(
                out,
                "- Test: {}\n- Effect size: {}\n- Transform for the test: {}",
                a.test.name(),
                a.effect_size.name(),
                a.transform.name()
            );
            let _ = writeln!(out, "- Multiple test correction: {}", a.correction.name());
            let _ = writeln!(
                out,
                "- Post-hoc test: {} at {}%",
                st.post_hoc_test.name(),
                percent(st.post_hoc_coverage)
            );
            significance(&mut out, st.filter.significance);
        }
        Mode::Correlation => {
            let st = &s.correlation;
            let _ = writeln!(
                out,
                "- Numeric field: {}\n- Correlation: {}",
                st.field.as_deref().unwrap_or("(none)"),
                st.analysis.method.name()
            );
            let _ = writeln!(
                out,
                "- Multiple test correction: {}",
                st.analysis.correction.name()
            );
            significance(&mut out, st.filter.significance);
        }
    }
    let summary = app.status_summary();
    if !summary.is_empty() {
        let _ = writeln!(out, "- {summary}");
    }
    if let Some(error) = &s.error {
        let _ = writeln!(out, "- Error: {error}");
    }
    out
}

fn unclassified(value: Unclassified) -> &'static str {
    match value {
        Unclassified::Retain => "retained",
        Unclassified::Remove => "removed",
        Unclassified::FrequencyOnly => "used only for calculating frequency profiles",
    }
}

fn percent(fraction: f64) -> String {
    let value = fraction * 100.0;
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

fn significance(out: &mut String, level: Option<f64>) {
    match level {
        Some(level) => {
            let _ = writeln!(
                out,
                "- Filter: features with a corrected p-value above {level} are inactive"
            );
        }
        None => out.push_str("- Filter: no p-value filter\n"),
    }
}

/// The plot being shown: its kind, its text, the items it draws and its
/// options, as Markdown. At most `max_items` items are listed.
pub fn plot(app: &StampApp, max_items: usize) -> String {
    let kind = app.plot_kind();
    let mut out = String::from("# Current plot\n\n");
    let _ = writeln!(out, "- Plot: {} (id `{}`)", kind.name(), plot_id(kind));
    if kind.uses_selected_feature(app.mode) {
        let _ = writeln!(
            out,
            "- Feature shown: {}",
            app.session.selected_feature.as_deref().unwrap_or("(none)")
        );
    }
    let highlighted = app.highlighted_features();
    if !highlighted.is_empty() && kind.supports_highlight(app.mode) {
        let _ = writeln!(out, "- Features drawn in red: {}", highlighted.join(", "));
    }
    out.push_str(&figure(app.figure(), max_items));
    let _ = writeln!(
        out,
        "\n## Options of this plot (JSON)\n\n```json\n{}\n```",
        plot_options_json(app, kind)
    );
    out
}

/// The text and items of a figure.
pub fn figure(figure: &Figure, max_items: usize) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "- Size: {:.0} x {:.0} points",
        figure.width, figure.height
    );
    let mut texts: Vec<&str> = Vec::new();
    for shape in &figure.shapes {
        if let Shape::Text { text, .. } = shape {
            let text = text.trim();
            if !text.is_empty() && !texts.contains(&text) {
                texts.push(text);
            }
        }
    }
    out.push_str("\n## Text in the plot\n\n");
    out.push_str("Titles, axis labels, tick labels and legends, in drawing order.\n\n");
    for text in texts.iter().take(MAX_PLOT_TEXT) {
        let _ = writeln!(out, "- {text}");
    }
    if texts.len() > MAX_PLOT_TEXT {
        let _ = writeln!(out, "- ... and {} more", texts.len() - MAX_PLOT_TEXT);
    }
    if !figure.regions.is_empty() {
        out.push_str("\n## Items in the plot\n\n");
        out.push_str("Each item the plot draws and its tooltip.\n\n");
        for region in figure.regions.iter().take(max_items) {
            let tooltip = region
                .tooltip
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            if tooltip.is_empty() || tooltip == region.item {
                let _ = writeln!(out, "- {}", region.item);
            } else {
                let _ = writeln!(out, "- {}: {tooltip}", region.item);
            }
        }
        if figure.regions.len() > max_items {
            let _ = writeln!(
                out,
                "- ... and {} more items",
                figure.regions.len() - max_items
            );
        }
    }
    out
}

/// The text of a figure that is only a message in place of a plot (why it
/// cannot be drawn, or that it is being computed), or `None` for a plot.
pub fn figure_message(figure: &Figure) -> Option<String> {
    let lines: Vec<&str> = figure
        .shapes
        .iter()
        .map(|shape| match shape {
            Shape::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let message = Figure::message(&lines.join("\n"));
    (figure.regions.is_empty()
        && !lines.is_empty()
        && figure.width == message.width
        && figure.height == message.height)
        .then(|| lines.join(" "))
}

/// The top of the results table as tab-separated text, sorted by
/// corrected p-value (smallest first). `None` for `rows` gives every row.
/// `feature` limits it to one feature.
pub fn results(app: &mut StampApp, rows: usize, feature: Option<&str>) -> String {
    let mode = app.mode;
    let table = app.results_table().clone();
    let mut out = format!("# Results table ({})\n\n", mode.name());
    if table.headings.is_empty() {
        out.push_str("No results yet: the analysis has not been run.\n");
        return out;
    }
    let mut order: Vec<usize> = (0..table.rows.len()).collect();
    if let Some(column) = p_value_column(&table.headings) {
        let value = |row: usize| -> f64 {
            table.rows[row]
                .get(column)
                .and_then(|cell| cell.parse::<f64>().ok())
                .unwrap_or(f64::INFINITY)
        };
        order.sort_by(|&a, &b| value(a).total_cmp(&value(b)));
    }
    if let Some(feature) = feature {
        order.retain(|&row| table.rows[row].first().is_some_and(|cell| cell == feature));
        if order.is_empty() {
            let _ = writeln!(out, "No feature named {feature} is in the results table.");
            return out;
        }
    }
    let shown = order.len().min(rows);
    if feature.is_none() {
        let _ = writeln!(
            out,
            "{} of {} rows that pass the filters, smallest corrected p-value first:\n",
            shown,
            table.rows.len()
        );
    }
    out.push_str("```tsv\n");
    out.push_str(&table.headings.join("\t"));
    out.push('\n');
    for &row in order.iter().take(shown) {
        out.push_str(&table.rows[row].join("\t"));
        out.push('\n');
    }
    out.push_str("```\n");
    out
}

/// The column of corrected p-values, or of p-values if there is none.
fn p_value_column(headings: &[String]) -> Option<usize> {
    let lower: Vec<String> = headings.iter().map(|h| h.to_lowercase()).collect();
    lower
        .iter()
        .position(|h| h.starts_with("p-value") && h.contains("corrected"))
        .or_else(|| lower.iter().position(|h| h.starts_with("p-value")))
}

/// The name a plot kind goes by in the assistant's tools.
pub fn plot_id(kind: PlotKind) -> String {
    format!("{kind:?}")
}

/// The plot kinds, for looking one up by id or name.
pub const ALL_PLOT_KINDS: [PlotKind; 26] = [
    PlotKind::ExtendedErrorBar,
    PlotKind::Bar,
    PlotKind::Box,
    PlotKind::ProfileBar,
    PlotKind::Scatter,
    PlotKind::SequenceHistogram,
    PlotKind::PValueHistogram,
    PlotKind::MultipleComparison,
    PlotKind::Heatmap,
    PlotKind::PostHoc,
    PlotKind::Rarefaction,
    PlotKind::Volcano,
    PlotKind::Composition,
    PlotKind::AlphaDiversity,
    PlotKind::Pcoa,
    PlotKind::DistanceBox,
    PlotKind::CorrelationScatter,
    PlotKind::Nmds,
    PlotKind::SharedFeatures,
    PlotKind::RankAbundance,
    PlotKind::Ancom,
    PlotKind::Mantel,
    PlotKind::GroupHeatmap,
    PlotKind::CoOccurrence,
    PlotKind::PrevalenceAbundance,
    PlotKind::Map,
];

/// A plot kind by its id (`ExtendedErrorBar`) or its name ("Extended error
/// bar"), ignoring case.
pub fn plot_kind_named(name: &str) -> Option<PlotKind> {
    let wanted = name.trim().to_lowercase();
    ALL_PLOT_KINDS.into_iter().find(|kind| {
        plot_id(*kind).to_lowercase() == wanted || kind.name().to_lowercase() == wanted
    })
}

/// The fields of [`crate::plots::PlotSettings`] that belong to a plot (as
/// `PlotSettings::reset` resets them).
pub fn option_fields(mode: Mode, kind: PlotKind) -> &'static [&'static str] {
    const COMMUNITY: [&str; 3] = [
        "community_normalisation",
        "community_distance",
        "community_permutations",
    ];
    match kind {
        PlotKind::ExtendedErrorBar => &["error_bar", "error_bar_sequence_counts"],
        PlotKind::PostHoc => &["post_hoc", "post_hoc_only_significant"],
        PlotKind::Bar if mode == Mode::TwoSamples => &["stat_bar"],
        PlotKind::Bar => &["bar", "feature_field"],
        PlotKind::Box => &[
            "box_plot",
            "feature_field",
            "box_features",
            "box_feature_count",
            "box_feature_list",
            "box_category_field",
        ],
        PlotKind::ProfileBar => &["profile_bar", "profile_bar_only_active"],
        PlotKind::Scatter => &["scatter", "group_spread"],
        PlotKind::SequenceHistogram => &["sequence_histogram"],
        PlotKind::PValueHistogram => &["p_value_histogram", "p_value_histogram_corrected"],
        PlotKind::MultipleComparison => &["multiple_comparison"],
        PlotKind::Rarefaction => &["rarefaction"],
        PlotKind::Volcano => &["volcano"],
        PlotKind::CorrelationScatter => &["correlation"],
        PlotKind::Composition => &["composition"],
        PlotKind::SharedFeatures => &["shared"],
        PlotKind::RankAbundance => &["rank_abundance"],
        PlotKind::Ancom => &["ancom", "ancom_test", "ancom_alpha"],
        PlotKind::Nmds => &["nmds", COMMUNITY[0], COMMUNITY[1], COMMUNITY[2]],
        PlotKind::Mantel => &["mantel", COMMUNITY[0], COMMUNITY[1], COMMUNITY[2]],
        PlotKind::AlphaDiversity => &[
            "alpha_metric",
            "alpha_box",
            COMMUNITY[0],
            COMMUNITY[1],
            COMMUNITY[2],
        ],
        PlotKind::Pcoa => &[
            "pcoa",
            "pcoa_show_tests",
            COMMUNITY[0],
            COMMUNITY[1],
            COMMUNITY[2],
        ],
        PlotKind::DistanceBox => &["distance_box", COMMUNITY[0], COMMUNITY[1], COMMUNITY[2]],
        PlotKind::Heatmap => &["heatmap", "heatmap_only_active", "heatmap_sequence_counts"],
        PlotKind::GroupHeatmap => &[
            "group_heatmap",
            "group_heatmap_summary",
            "group_heatmap_only_active",
            "group_heatmap_category_field",
        ],
        PlotKind::PrevalenceAbundance => &[
            "prevalence",
            "prevalence_min_abundance",
            "prevalence_only_active",
            "prevalence_highlight_selected",
            "prevalence_highlight",
        ],
        PlotKind::Map => &["map", "map_lat_field", "map_lon_field"],
        PlotKind::CoOccurrence => &[
            "co_occurrence",
            "co_occurrence_options",
            "co_occurrence_features",
            "co_occurrence_group",
        ],
    }
}

/// Fields of the plot settings shared by every plot.
pub const SHARED_OPTION_FIELDS: [&str; 5] = [
    "highlighted",
    "min_p_exponent",
    "axes_color",
    "sample_colors",
    "fonts",
];

/// The options of a plot as a JSON object of its fields.
pub fn plot_options(app: &StampApp, kind: PlotKind) -> serde_json::Value {
    let all = serde_json::to_value(app.plot_settings()).unwrap_or_default();
    let mut options = serde_json::Map::new();
    for field in option_fields(app.mode, kind) {
        if let Some(value) = all.get(*field) {
            options.insert((*field).to_string(), value.clone());
        }
    }
    serde_json::Value::Object(options)
}

fn plot_options_json(app: &StampApp, kind: PlotKind) -> String {
    serde_json::to_string_pretty(&plot_options(app, kind)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plots::PlotSettings;

    #[test]
    fn every_option_field_exists_and_belongs_somewhere() {
        let all = serde_json::to_value(PlotSettings::default()).unwrap();
        let all = all.as_object().unwrap();
        let mut claimed: Vec<&str> = SHARED_OPTION_FIELDS.to_vec();
        for mode in Mode::ALL {
            for kind in ALL_PLOT_KINDS {
                for field in option_fields(mode, kind) {
                    assert!(all.contains_key(*field), "{field} is not a plot setting");
                    claimed.push(field);
                }
            }
        }
        for field in all.keys() {
            assert!(
                claimed.contains(&field.as_str()),
                "plot setting {field} belongs to no plot in `option_fields`"
            );
        }
    }

    #[test]
    fn every_plot_kind_is_listed_once_and_found_by_name() {
        for mode in Mode::ALL {
            for kind in PlotKind::available(mode) {
                assert!(ALL_PLOT_KINDS.contains(kind), "{kind:?} missing");
            }
        }
        for kind in ALL_PLOT_KINDS {
            assert_eq!(plot_kind_named(&plot_id(kind)), Some(kind));
            assert_eq!(plot_kind_named(&kind.name().to_uppercase()), Some(kind));
        }
        assert_eq!(plot_kind_named("no such plot"), None);
    }

    #[test]
    fn p_value_column_prefers_corrected() {
        let headings: Vec<String> = ["Feature", "p-values", "p-values (corrected)"]
            .map(String::from)
            .to_vec();
        assert_eq!(p_value_column(&headings), Some(2));
        assert_eq!(p_value_column(&headings[..2]), Some(1));
    }
}
