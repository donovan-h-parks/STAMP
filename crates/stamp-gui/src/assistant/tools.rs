//! The assistant's tools: what a language model may ask of STAMP, and how
//! STAMP answers. Independent of how a request arrives (the MCP server on
//! the desktop, or tests), so the same tools could serve another route.
//!
//! Tools that change the session ([`ToolInfo::changes`]) are applied at
//! once; the window answers them once the analyses and the plot have
//! caught up, with [`settled_summary`] appended.

use serde_json::{json, Map, Value};
use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::post_hoc::PostHocTest;
use stamp_core::stats::results::Transform;
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::TestType;

use super::describe;
use crate::plots::PlotSettings;
use crate::session::Mode;
use crate::StampApp;

/// A tool as it is listed to the model.
pub struct ToolInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    /// Whether the tool changes the session (and can be undone).
    pub changes: bool,
}

/// Part of a tool's answer.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Text(String),
    /// A PNG image.
    Png(Vec<u8>),
}

/// A tool's answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub content: Vec<Content>,
    pub is_error: bool,
}

impl Outcome {
    pub fn text(text: impl Into<String>) -> Outcome {
        Outcome {
            content: vec![Content::Text(text.into())],
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Outcome {
        Outcome {
            content: vec![Content::Text(text.into())],
            is_error: true,
        }
    }

    /// All text of the answer, for logs and tests.
    pub fn joined_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| match c {
                Content::Text(text) => Some(text.as_str()),
                Content::Png(_) => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// What the assistant is told about STAMP when it connects.
pub const INSTRUCTIONS: &str = "\
STAMP is a desktop application for the statistical analysis of taxonomic and functional \
profiles (for example microbial community profiles). The user is looking at STAMP's window \
while talking to you; its tools show you their data, results and the plot on screen, and \
let you change the analysis and the plot. The user is usually a biologist, not a \
statistician or programmer: explain in plain language what a plot shows and what the \
statistics do and do not mean (effect sizes, confidence intervals, p-values and their \
correction for multiple tests). Start with get_session and get_current_plot. Prefer \
STAMP's own plots (list_plots, show_plot); when none shows what the user asks for, draw \
one with show_custom_plot. Every change you make appears in STAMP's window at once and \
the user can undo it. When the user asks for other data, load_data opens a profile or \
metadata file from their computer.";

/// The tools, in the order they are listed.
pub fn list() -> Vec<ToolInfo> {
    let empty = json!({"type": "object", "properties": {}, "additionalProperties": false});
    vec![
        ToolInfo {
            name: "get_session",
            description: "The loaded profile and metadata (files, samples, levels, metadata \
                          fields, groups) and the settings of the current analysis (its \
                          type, the samples or groups compared, statistical test, confidence \
                          interval, multiple test correction, filters).",
            input_schema: empty.clone(),
            changes: false,
        },
        ToolInfo {
            name: "get_results",
            description: "The results table of the current analysis: one row per feature \
                          that passes the filters, with means or proportions, p-values, \
                          corrected p-values, effect sizes and confidence intervals, sorted \
                          by corrected p-value. Tab-separated, STAMP's column headings.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "rows": {"type": "integer", "minimum": 1,
                             "description": "Most rows to return (default 50)."},
                    "feature": {"type": "string",
                                "description": "Return only the row of this feature."}
                },
                "additionalProperties": false
            }),
            changes: false,
        },
        ToolInfo {
            name: "get_samples",
            description: "Every sample with its metadata values, whether it is switched on \
                          (used in the analyses) and its total count. Tab-separated.",
            input_schema: empty.clone(),
            changes: false,
        },
        ToolInfo {
            name: "get_current_plot",
            description: "The plot shown in STAMP's window. 'description' (default) gives its \
                          type, its text (title, axis labels, ticks, legend), every item it \
                          draws with its tooltip values, and its options as JSON. 'png' adds \
                          an image of it; 'svg' its SVG source.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "format": {"type": "string", "enum": ["description", "png", "svg"]},
                    "max_items": {"type": "integer", "minimum": 0,
                                  "description": "Most plot items to list (default 300)."}
                },
                "additionalProperties": false
            }),
            changes: false,
        },
        ToolInfo {
            name: "list_plots",
            description: "The analysis types and, for the current one, the plots STAMP can \
                          draw, with their ids for show_plot, the plot shown now, and \
                          whether a plot shows only the selected feature.",
            input_schema: empty.clone(),
            changes: false,
        },
        ToolInfo {
            name: "get_plot_options",
            description: "The options of a plot as JSON (the fields of STAMP's plot settings \
                          that belong to it), plus the options all plots share. Change them \
                          with show_plot.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plot": {"type": "string",
                             "description": "Plot id from list_plots; the current plot if \
                                             left out."}
                },
                "additionalProperties": false
            }),
            changes: false,
        },
        ToolInfo {
            name: "show_plot",
            description: "Show a plot in STAMP's window and change its options. 'plot' is the \
                          id of one of STAMP's plots, or the name of a plot you drew with \
                          show_custom_plot. 'options' is \
                          merged into the plot settings: give only the fields to change, in \
                          the shape get_plot_options returns (nested objects are merged, \
                          other values replaced). 'feature' selects the feature that the \
                          single-feature plots draw. Answers once the plot is drawn, with \
                          what is shown.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plot": {"type": "string",
                             "description": "Plot id from list_plots, or the name of a plot \
                                             you drew."},
                    "options": {"type": "object"},
                    "feature": {"type": "string"}
                },
                "additionalProperties": false
            }),
            changes: true,
        },
        ToolInfo {
            name: "set_analysis",
            description: "Change the analysis: its type and settings. Give only what should \
                          change. Names are matched ignoring case, spaces and punctuation; an \
                          unknown name is answered with the choices. Answers once the \
                          analysis has run again. Choices: mode: 'Multiple groups', 'Two \
                          groups', 'Two samples', 'Correlation'. test: depends on mode (for \
                          example \"Welch's t-test\", 'Mann-Whitney U test', 'ANOVA', \
                          'Kruskal-Wallis H-test', \"Fisher's exact test\"). correction: for \
                          example 'No correction', 'Benjamini-Hochberg FDR', 'Bonferroni'. \
                          coverage: confidence interval coverage, 0.95 or 95.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mode": {"type": "string"},
                    "profile_level": {"type": "string"},
                    "parent_level": {"type": "string",
                                     "description": "'Entire sample' or a level above the \
                                                     profile level."},
                    "group_field": {"type": "string"},
                    "sample1": {"type": "string"},
                    "sample2": {"type": "string"},
                    "group1": {"type": "string"},
                    "group2": {"type": ["string", "null"],
                               "description": "null or 'All other samples' compares group 1 \
                                               with every other sample."},
                    "groups": {"type": "array", "items": {"type": "string"},
                               "description": "Groups compared in a multiple-group analysis."},
                    "pair_field": {"type": ["string", "null"],
                                   "description": "Metadata field naming the subject of each \
                                                   sample, for the paired two-group tests and \
                                                   the Friedman test of matched groups."},
                    "correlation_field": {"type": "string"},
                    "test": {"type": "string"},
                    "test_type": {"type": "string", "enum": ["One-sided", "Two-sided"]},
                    "ci_method": {"type": "string"},
                    "coverage": {"type": "number"},
                    "correction": {"type": "string"},
                    "transform": {"type": "string"},
                    "post_hoc_test": {"type": "string"},
                    "p_value_filter": {"type": ["number", "null"],
                                       "description": "Features with a corrected p-value \
                                                       above this are inactive; null for no \
                                                       filter."}
                },
                "additionalProperties": false
            }),
            changes: true,
        },
        ToolInfo {
            name: "switch_samples",
            description: "Switch samples off (leave them out of every analysis and plot) or back \
                          on, as the Metadata table does. Give either 'samples' (names, from \
                          get_samples) or 'where' (a metadata field, a relation and a value: \
                          for example {\"field\": \"Company\", \"relation\": \"=\", \
                          \"value\": \"QS\"} for every sample of the QS company), or 'all': \
                          true. 'active' false switches them off (the default), true on. \
                          Answers once the analyses have run again, with how many samples \
                          are switched on.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "active": {"type": "boolean",
                               "description": "false (default) switches off, true on."},
                    "samples": {"type": "array", "items": {"type": "string"}},
                    "where": {
                        "type": "object",
                        "properties": {
                            "field": {"type": "string"},
                            "relation": {"type": "string", "enum": ["=", "<", ">"]},
                            "value": {"type": "string"}
                        },
                        "required": ["field", "value"],
                        "additionalProperties": false
                    },
                    "all": {"type": "boolean",
                            "description": "Every sample (for example, to switch all back on)."}
                },
                "additionalProperties": false
            }),
            changes: true,
        },
        ToolInfo {
            name: "set_plot_fonts",
            description: "Change the text of every plot: the size in points of tick labels, \
                          axis labels and legends, and the font (Arial, Times New Roman, \
                          Courier New or DejaVu Sans). Give only what should change. Text \
                          particular to one plot (titles, feature names, p-values, labels of \
                          points) has its own *_font_size options: change those with \
                          show_plot (get_plot_options lists them).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tick_labels": {"type": "number", "minimum": 4, "maximum": 36},
                    "axis_labels": {"type": "number", "minimum": 4, "maximum": 36},
                    "legend": {"type": "number", "minimum": 4, "maximum": 36},
                    "font": {"type": "string"}
                },
                "additionalProperties": false
            }),
            changes: true,
        },
        ToolInfo {
            name: "set_group_colors",
            description: "Change the colours of groups in every plot that colours by group \
                          (any group of the group field, and \"All other samples\"). 'colors' \
                          maps group names to colours: \"#rrggbb\" or a common name such as \
                          red, blue, green, orange, purple, grey. get_session lists the groups \
                          and their colours now. (The two samples of a two-sample analysis are \
                          coloured by the plot option sample_colors instead.)",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "colors": {
                        "type": "object",
                        "additionalProperties": {"type": "string"},
                        "description": "Group name to colour, for example {\"Produced water\": \"red\"}."
                    }
                },
                "required": ["colors"],
                "additionalProperties": false
            }),
            changes: true,
        },
        ToolInfo {
            name: "show_custom_plot",
            description: "Draw a plot of your own in STAMP's window, in place of the plot \
                          shown, when none of STAMP's plots (list_plots) shows what is wanted. \
                          'spec' describes it in Vega-Lite (v5) JSON; STAMP draws it in the \
                          style of its own plots, with tooltips, and the user can save it like \
                          any plot. Data: {\"values\": [...]}, or {\"name\": ...} for STAMP's \
                          data: \"profile\" (one row per switched-on sample and feature at the \
                          current profile level: sample, feature, count, proportion (percent), \
                          group (the sample's group), and a column per higher level of the \
                          hierarchy, named by its heading), \"samples\" (sample, switched_on, \
                          total, and a column per metadata field) or \"results\" (the Results \
                          table: feature and the other columns get_results shows). Supported: \
                          marks bar, point, circle, square, tick, line, area, rect, rule, text, \
                          boxplot, errorbar; channels x, y, x2, y2, xOffset, yOffset, color, \
                          size, shape, opacity, strokeDash, text, tooltip, detail, xError, \
                          yError, with field, type, aggregate, bin, title, datum, value, scale \
                          (type linear, log or sqrt; domain, range, scheme, zero, reverse), \
                          axis (title, labelAngle, grid, null), legend, sort, stack; transforms \
                          filter, calculate, aggregate, joinaggregate, window, fold, lookup, \
                          bin; layer; title; width and height in points. Not supported: facets \
                          (row, column), concat, repeat, arc (pie), selections, dates. Field \
                          names are used as written. Groups keep their colours from STAMP's \
                          plots unless a colour scale is given. Answers with notes on anything \
                          ignored; if the plot cannot be drawn, nothing changes and the answer \
                          says why. get_current_plot describes the result. Each plot you \
                          draw is kept, under a name of its own, in the list of plots \
                          beneath STAMP's own (\"Drawn by the assistant\"), where the user \
                          can go back to it. 'name' (required) is what the plot is listed as: \
                          short, saying what it shows, and not the name of another plot you \
                          drew. A plot is added to the list unless 'plot' names one already \
                          there: to change a plot you drew (a refinement, not a new plot), give \
                          its name as 'plot', and as 'name' the same name or a new one. \
                          show_plot with its name shows a plot you drew before.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "spec": {
                        "type": "object",
                        "description": "The plot, as a Vega-Lite specification."
                    },
                    "name": {
                        "type": "string",
                        "description": "What the plot is listed as under \"Drawn by the \
                                        assistant\": short, saying what it shows, and not \
                                        the name of another plot you drew."
                    },
                    "plot": {
                        "type": "string",
                        "description": "Name of a plot you drew before, to change it instead \
                                        of adding a new plot."
                    }
                },
                "required": ["name", "spec"],
                "additionalProperties": false
            }),
            changes: true,
        },
        ToolInfo {
            name: "load_data",
            description: "Open a profile and/or a metadata file from the user's computer, in \
                          place of the data loaded now, as File > Load profile and Load \
                          metadata do. Use it only when the user asks for other data. Give \
                          full paths (\"~/\" stands for the home folder). 'profile': a \
                          STAMP profile, a BIOM table, a mothur taxonomy file, or the output \
                          of SingleM, Sandpiper, mOTUs, MetaPhlAn, Kraken, Bracken or HUMAnN; \
                          or several files of one of these profilers (one per sample), which \
                          are combined into one profile. 'metadata': a \
                          tab-separated file with a row per sample, for the profile given \
                          with it or, without 'profile', for the profile loaded now. A BIOM \
                          table takes its hierarchy from the observation metadata field \
                          'biom_metadata_field' (STAMP's usual choice if left out; null for \
                          the observation ids alone). A mothur taxonomy file needs its groups \
                          file, 'mothur_groups', and may have a names file, 'mothur_names'. A \
                          new profile starts a new analysis, and earlier changes can then no \
                          longer be undone. If a file cannot be read, nothing changes and the \
                          answer says why. Answers once the analysis has run, with what was \
                          loaded.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "profile": {"type": "array", "items": {"type": "string"},
                                "description": "Full path of the profile file, or of several \
                                                files of one profiler to combine. A single \
                                                path may also be given as a string."},
                    "metadata": {"type": "string",
                                 "description": "Full path of the metadata file."},
                    "biom_metadata_field": {"type": ["string", "null"]},
                    "mothur_groups": {"type": "string",
                                      "description": "Full path of the mothur groups file."},
                    "mothur_names": {"type": "string",
                                     "description": "Full path of the mothur names file."}
                },
                "additionalProperties": false
            }),
            changes: true,
        },
        ToolInfo {
            name: "undo",
            description: "Undo the last change made with show_plot, show_custom_plot, \
                          set_analysis or the other tools that change the session.",
            input_schema: empty,
            changes: true,
        },
    ]
}

/// Whether a tool changes the session.
pub fn changes(name: &str) -> bool {
    list().iter().any(|t| t.name == name && t.changes)
}

/// Carry out a tool. Tools that change the session leave it to the window
/// to bring the analyses and the plot up to date.
pub fn call(app: &mut StampApp, name: &str, args: &Value) -> Outcome {
    let empty = Map::new();
    let args = match args {
        Value::Object(map) => map,
        Value::Null => &empty,
        _ => return Outcome::error("The arguments must be a JSON object."),
    };
    let result = match name {
        "get_session" => Ok(Outcome::text(describe::session(app))),
        "get_results" => get_results(app, args),
        "get_samples" => Ok(Outcome::text(samples(app))),
        "get_current_plot" => get_current_plot(app, args),
        "list_plots" => Ok(Outcome::text(list_plots(app))),
        "get_plot_options" => get_plot_options(app, args),
        "show_plot" => show_plot(app, args),
        "show_custom_plot" => show_custom_plot(app, args),
        "load_data" => load_data(app, args),
        "set_analysis" => set_analysis(app, args),
        "set_group_colors" => set_group_colors(app, args),
        "switch_samples" => switch_samples(app, args),
        "set_plot_fonts" => set_plot_fonts(app, args),
        "undo" => Ok(Outcome::text(if app.undo_assistant_change() {
            "Undone."
        } else {
            "There is nothing to undo."
        })),
        _ => Err(format!("STAMP has no tool named {name}.")),
    };
    result.unwrap_or_else(Outcome::error)
}

/// What is shown once a change has been carried out: the analysis, the
/// plot and anything that went wrong.
pub fn settled_summary(app: &StampApp) -> String {
    let mut out = format!(
        "Now showing: {} analysis, {} (id `{}`).",
        app.mode.name(),
        app.plot_name(),
        describe::plot_id(app.plot_kind())
    );
    if app.plot_kind().uses_selected_feature(app.mode) {
        if let Some(feature) = &app.session.selected_feature {
            out.push_str(&format!(" Feature: {feature}."));
        }
    }
    let summary = app.status_summary();
    if !summary.is_empty() {
        out.push_str(&format!("\n{summary}"));
    }
    if let Some(error) = app.session.error(app.mode) {
        out.push_str(&format!("\nThe analysis reported a problem: {error}"));
    }
    if let Some(warning) = app.correction_warning() {
        out.push_str(&format!("\nCaution about the correction: {warning}"));
    }
    if let Some(notice) = app.notice() {
        out.push_str(&format!("\nSTAMP says: {notice}"));
    }
    if let Some(message) = describe::figure_message(app.figure()) {
        out.push_str(&format!("\nThe plot area says: {message}"));
    }
    out
}

fn get_results(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    let rows = optional_count(args, "rows")?.unwrap_or(describe::DEFAULT_RESULT_ROWS);
    let feature = optional_str(args, "feature")?;
    Ok(Outcome::text(describe::results(app, rows.max(1), feature)))
}

fn samples(app: &StampApp) -> String {
    let s = &app.session;
    let Some(tree) = &s.tree else {
        return "No profile is loaded.".to_string();
    };
    let fields: &[String] = s.metadata.as_ref().map_or(&[], |m| m.fields());
    let mut out = format!("Sample\tSwitched on\tTotal count\t{}\n", fields.join("\t"));
    for sample in tree.sample_names() {
        let active = s.metadata.as_ref().is_none_or(|m| m.is_active(sample));
        let total = tree.sample_total(sample).unwrap_or(f64::NAN);
        out.push_str(&format!(
            "{sample}\t{}\t{total}",
            if active { "yes" } else { "no" }
        ));
        for field in fields {
            let value = s
                .metadata
                .as_ref()
                .and_then(|m| m.value(sample, field))
                .unwrap_or("");
            out.push('\t');
            out.push_str(value);
        }
        out.push('\n');
    }
    out
}

fn get_current_plot(app: &StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    let max_items = optional_count(args, "max_items")?.unwrap_or(describe::MAX_PLOT_ITEMS);
    let format = optional_str(args, "format")?.unwrap_or("description");
    let mut outcome = Outcome::text(describe::plot(app, max_items));
    match format {
        "description" => {}
        "png" => {
            let png = crate::platform::encode_plot(app.figure(), "plot.png", 100.0)?;
            outcome.content.push(Content::Png(png));
        }
        "svg" => outcome
            .content
            .push(Content::Text(stamp_plot::svg::to_svg(app.figure()))),
        other => {
            return Err(format!(
                "Unknown format {other}: use description, png or svg."
            ))
        }
    }
    Ok(outcome)
}

fn list_plots(app: &StampApp) -> String {
    let mut out = String::from("Analysis types: ");
    out.push_str(
        &Mode::ALL
            .iter()
            .map(|m| m.name())
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push_str(&format!(
        "\nCurrent analysis: {}\nCurrent plot: {}\n\nPlots of this analysis (id: name, \
         group):\n",
        app.mode.name(),
        describe::plot_id(app.plot_kind())
    ));
    for kind in crate::plots::PlotKind::available(app.mode) {
        let feature = if kind.uses_selected_feature(app.mode) {
            " (shows the selected feature only)"
        } else {
            ""
        };
        out.push_str(&format!(
            "- {}: {}, {}{feature}\n",
            describe::plot_id(*kind),
            kind.name(),
            kind.group().name()
        ));
    }
    let custom = &app.session.custom_plots;
    if !custom.is_empty() {
        out.push_str(
            "\nPlots you drew (show one with show_plot and its name; change one with \
             show_custom_plot and its name as 'plot'):\n",
        );
        for (index, plot) in custom.iter().enumerate() {
            let shown = if app.plot_kind() == crate::plots::PlotKind::Custom
                && app.session.shown_custom == index
            {
                " (shown now)"
            } else {
                ""
            };
            out.push_str(&format!("- {}{shown}\n", plot.name));
        }
    }
    out
}

fn get_plot_options(app: &StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    let kind = match optional_str(args, "plot")? {
        Some(name) => plot_kind(name)?,
        None => app.plot_kind(),
    };
    let all = serde_json::to_value(app.plot_settings()).map_err(|e| e.to_string())?;
    let shared: Map<String, Value> = describe::SHARED_OPTION_FIELDS
        .iter()
        .filter_map(|f| all.get(*f).map(|v| ((*f).to_string(), v.clone())))
        .collect();
    let text = format!(
        "Options of {} (id `{}`):\n```json\n{}\n```\nOptions shared by all plots:\n```json\n{}\n```",
        kind.name(),
        describe::plot_id(kind),
        serde_json::to_string_pretty(&describe::plot_options(app, kind)).unwrap_or_default(),
        serde_json::to_string_pretty(&Value::Object(shared)).unwrap_or_default()
    );
    Ok(Outcome::text(text))
}

fn plot_kind(name: &str) -> Result<crate::plots::PlotKind, String> {
    describe::plot_kind_named(name).ok_or_else(|| {
        format!("STAMP has no plot {name}; list_plots gives the plots of the current analysis.")
    })
}

fn show_plot(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    // A plot the assistant drew, by its name.
    let mut custom = None;
    let kind = match optional_str(args, "plot")? {
        Some(name) if describe::plot_kind_named(name).is_none() => {
            custom = Some(
                app.session
                    .custom_plot_named(name)
                    .ok_or_else(|| format!("STAMP has no plot {name}; list_plots gives the plots of the current analysis and the plots you drew."))?,
            );
            None
        }
        Some(name) => {
            let kind = plot_kind(name)?;
            if !crate::plots::PlotKind::available(app.mode).contains(&kind) {
                return Err(format!(
                    "{} is not offered in the {} analysis; list_plots gives the plots that are, \
                     and set_analysis changes the analysis.",
                    kind.name(),
                    app.mode.name()
                ));
            }
            Some(kind)
        }
        None => None,
    };
    let settings = match args.get("options") {
        None | Some(Value::Null) => None,
        Some(Value::Object(patch)) => Some(patched_settings(app.plot_settings(), patch)?),
        Some(_) => return Err("'options' must be a JSON object.".to_string()),
    };
    let feature = optional_str(args, "feature")?;
    if let Some(feature) = feature {
        if !app.feature_table().iter().any(|row| row.feature == feature) {
            return Err(format!(
                "No feature named {feature} is in the current analysis; get_results lists them."
            ));
        }
    }
    app.remember_for_undo("show_plot");
    if let Some(kind) = kind {
        app.set_plot_kind(kind);
    }
    if let Some(index) = custom {
        app.choose_custom_plot(index);
    }
    if let Some(settings) = settings {
        app.set_plot_settings(settings);
    }
    if let Some(feature) = feature {
        app.select_feature(feature);
    }
    Ok(Outcome::text("Done."))
}

/// The longest name a custom plot may have.
const MAX_CUSTOM_NAME: usize = 80;

/// Why `name` names none of the plots the assistant drew.
fn no_custom_plot(app: &StampApp, name: &str) -> String {
    let names: Vec<&str> = app
        .session
        .custom_plots
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    if names.is_empty() {
        format!("There is no plot named {name}: you have not drawn any yet.")
    } else {
        format!(
            "There is no plot named {name}; the plots you drew are: {}.",
            names.join(", ")
        )
    }
}

fn show_custom_plot(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    const KNOWN: [&str; 3] = ["spec", "plot", "name"];
    if let Some(key) = args.keys().find(|k| !KNOWN.contains(&k.as_str())) {
        return Err(format!(
            "show_custom_plot has no setting {key}; it takes {}.",
            KNOWN.join(", ")
        ));
    }
    // The plot changed, if it is not a new one.
    let replace = match optional_str(args, "plot")? {
        Some(name) => Some(
            app.session
                .custom_plot_named(name)
                .ok_or_else(|| no_custom_plot(app, name))?,
        ),
        None => None,
    };
    let listed = match optional_str(args, "name")?.map(str::trim) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => {
            return Err(format!(
                "show_custom_plot needs 'name': a short name of the plot's own, saying what it \
                 shows (for example \"Bacteroidetes by enterotype\"), which it is listed as \
                 under \"Drawn by the assistant\".{}",
                match replace {
                    Some(index) => format!(
                        " To keep the name of the plot you are changing, give \"name\": \"{}\".",
                        app.session.custom_plots[index].name
                    ),
                    None => String::new(),
                }
            ))
        }
    };
    if listed.chars().count() > MAX_CUSTOM_NAME {
        return Err(format!(
            "The name {listed} is too long: give one of at most {MAX_CUSTOM_NAME} characters."
        ));
    }
    if describe::plot_kind_named(&listed).is_some() {
        return Err(format!(
            "{listed} is the name of one of STAMP's own plots: give the plot a name of its own."
        ));
    }
    if let Some(index) = app.session.custom_plot_named(&listed) {
        if Some(index) != replace {
            let existing = &app.session.custom_plots[index].name;
            return Err(format!(
                "You already drew a plot named {existing}. To change that plot, give \
                 \"plot\": \"{existing}\" (and its name, or a new one); for a new plot, give a \
                 name that no other plot has."
            ));
        }
    }
    let spec = match args.get("spec") {
        Some(Value::Object(spec)) => Value::Object(spec.clone()),
        // Some clients send objects as text.
        Some(Value::String(text)) => {
            serde_json::from_str(text).map_err(|e| format!("'spec' is not valid JSON: {e}"))?
        }
        None | Some(Value::Null) => {
            return Err(
                "show_custom_plot needs 'spec', the plot described in Vega-Lite (a JSON \
                 object). To show a plot you drew before, use show_plot with its name."
                    .to_string(),
            )
        }
        Some(_) => return Err("'spec' must be a JSON object.".to_string()),
    };
    let mut tables = std::collections::HashMap::new();
    for name in stamp_plot::custom::data_names(&spec) {
        if !crate::plots::CUSTOM_DATA.contains(&name.as_str()) {
            // The plot reports it, with the names there are.
            continue;
        }
        let table = if name == "results" {
            let table = app.results_table().clone();
            crate::plots::custom_table(&app.session, &name, (&table.headings, &table.rows))
        } else {
            crate::plots::custom_table(&app.session, &name, (&[], &[]))
        }
        .map_err(|e| format!("The data '{name}' is not available: {e}"))?;
        tables.insert(name, std::sync::Arc::new(table));
    }
    let plot = crate::plots::CustomPlot::new(listed.clone(), spec, tables);
    // Drawn here first, so that a plot that cannot be drawn changes nothing.
    let built =
        stamp_plot::axes::with_fonts(app.plot_settings().fonts, || plot.build(&app.session))
            .map_err(|e| format!("The plot could not be drawn: {e}. Nothing was changed."))?;
    app.remember_for_undo("show_custom_plot");
    app.show_custom_plot(plot, replace);
    let mut text = if replace.is_some() {
        format!("Done: changed the plot {listed}.")
    } else {
        format!(
            "Done: added the plot {listed} under \"Drawn by the assistant\". To change it, give \
             \"plot\": \"{listed}\"."
        )
    };
    if !built.notes.is_empty() {
        text.push_str("\nNotes on the description: ");
        text.push_str(&built.notes.join("; "));
        text.push('.');
    }
    Ok(Outcome::text(text))
}

fn switch_samples(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    let active = match args.get("active") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err("'active' must be true or false.".to_string()),
    };
    let Some(metadata) = &app.session.metadata else {
        return Err(
            "Samples are switched on and off through the metadata: the user has to load a \
             metadata file first."
                .to_string(),
        );
    };
    let all_samples: Vec<String> = metadata.samples().to_vec();
    let fields: Vec<String> = metadata.fields().to_vec();
    let given = [
        args.contains_key("samples"),
        args.contains_key("where"),
        matches!(args.get("all"), Some(Value::Bool(true))),
    ];
    if given.iter().filter(|g| **g).count() != 1 {
        return Err("Give exactly one of 'samples', 'where' or 'all'.".to_string());
    }
    let chosen: Vec<String> = if given[2] {
        all_samples.clone()
    } else if let Some(samples) = args.get("samples") {
        let Some(samples) = samples.as_array() else {
            return Err("'samples' must be a list of sample names.".to_string());
        };
        let mut chosen = Vec::new();
        for sample in samples {
            let name = sample
                .as_str()
                .ok_or("'samples' must be a list of names.")?;
            chosen.push(choose_string("sample", name, &all_samples)?);
        }
        chosen
    } else {
        let Some(Value::Object(rule)) = args.get("where") else {
            return Err("'where' must be an object with field, relation and value.".to_string());
        };
        let field = rule
            .get("field")
            .and_then(Value::as_str)
            .ok_or("'where' needs a 'field'.")?;
        let field = choose_string("metadata field", field, &fields)?;
        let value = match rule.get("value") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => return Err("'where' needs a 'value'.".to_string()),
        };
        let relation = match rule.get("relation").and_then(Value::as_str).unwrap_or("=") {
            "=" | "==" => crate::session::Relation::Equal,
            "<" => crate::session::Relation::Less,
            ">" => crate::session::Relation::Greater,
            other => return Err(format!("Unknown relation '{other}': use =, < or >.")),
        };
        // "=" takes the value as the metadata spells it, if it only differs
        // in case or punctuation.
        let value = if relation == crate::session::Relation::Equal {
            let values: Vec<String> = metadata
                .unique_values(&field)
                .into_iter()
                .map(str::to_string)
                .collect();
            choose_string(&format!("value of {field}"), &value, &values)?
        } else {
            value
        };
        let chosen = app.session.samples_matching(&field, relation, &value);
        if chosen.is_empty() {
            return Err(format!(
                "No sample has {field} {} {value}.",
                relation.name()
            ));
        }
        chosen
    };
    app.remember_for_undo("switch_samples");
    app.session.set_samples_active(&chosen, active);
    app.redraw_plot();
    let on = app
        .session
        .metadata
        .as_ref()
        .map_or(0, |m| m.active_samples().len());
    Ok(Outcome::text(format!(
        "{} {} sample(s); {on} of {} samples are now switched on.",
        if active {
            "Switched on"
        } else {
            "Switched off"
        },
        chosen.len(),
        all_samples.len()
    )))
}

fn set_plot_fonts(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    if args.is_empty() {
        return Err("Give at least one of tick_labels, axis_labels, legend or font.".to_string());
    }
    let mut settings = app.plot_settings().clone();
    let size = |key: &str| -> Result<Option<f64>, String> {
        match args.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => {
                let size = value
                    .as_f64()
                    .ok_or_else(|| format!("'{key}' must be a size in points."))?;
                if !(4.0..=36.0).contains(&size) {
                    return Err(format!("'{key}' must be between 4 and 36 points."));
                }
                Ok(Some(size))
            }
        }
    };
    let fonts = &mut settings.fonts;
    if let Some(size) = size("tick_labels")? {
        fonts.tick = size;
    }
    if let Some(size) = size("axis_labels")? {
        fonts.axis_label = size;
    }
    if let Some(size) = size("legend")? {
        fonts.legend = size;
    }
    if let Some(font) = optional_str(args, "font")? {
        fonts.family = choose("font", font, &stamp_plot::text::PlotFont::ALL, |f| {
            f.name().to_string()
        })?;
    }
    let text = format!(
        "Plot text: ticks {} pt, axis labels {} pt, legends {} pt, font {}.",
        fonts.tick,
        fonts.axis_label,
        fonts.legend,
        fonts.family.name()
    );
    app.remember_for_undo("set_plot_fonts");
    app.set_plot_settings(settings);
    Ok(Outcome::text(text))
}

fn set_group_colors(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    let Some(Value::Object(colors)) = args.get("colors") else {
        return Err("'colors' must be an object of group names and colours.".to_string());
    };
    if colors.is_empty() {
        return Err("Give at least one group and its colour.".to_string());
    }
    let mut groups: Vec<String> = app
        .session
        .grouping()
        .map(|g| g.names().map(str::to_string).collect())
        .unwrap_or_default();
    if groups.is_empty() {
        return Err(
            "There are no groups: load metadata and choose a group field first.".to_string(),
        );
    }
    groups.push(stamp_core::ALL_OTHER_SAMPLES.to_string());
    // Everything is checked before anything is changed.
    let mut chosen = Vec::new();
    for (name, color) in colors {
        let group = choose_string("group", name, &groups)?;
        let color = color.as_str().and_then(parse_color).ok_or_else(|| {
            format!(
                "'{color}' is not a colour: use #rrggbb or a name such as red, blue, green, \
                     orange, purple or grey."
            )
        })?;
        chosen.push((group, color));
    }
    app.remember_for_undo("set_group_colors");
    let mut done = Vec::new();
    for (group, color) in chosen {
        if group == stamp_core::ALL_OTHER_SAMPLES {
            app.session.other_samples_color = color;
            app.redraw_plot();
        } else {
            app.session.set_group_color(&group, color);
        }
        done.push(format!("{group}: {}", color.to_hex()));
    }
    Ok(Outcome::text(format!("Colours set: {}.", done.join(", "))))
}

/// A colour from `#rrggbb`, `rrggbb` or a common colour name.
pub fn parse_color(text: &str) -> Option<stamp_plot::Color> {
    if let Some(color) = stamp_plot::Color::from_hex(text.trim()) {
        return Some(color);
    }
    let hex = match simplified(text).as_str() {
        "red" => "#e41a1c",
        "blue" => "#377eb8",
        "green" => "#4daf4a",
        "purple" => "#984ea3",
        "orange" => "#ff7f00",
        "yellow" => "#ffd92f",
        "brown" => "#a65628",
        "pink" => "#f781bf",
        "grey" | "gray" => "#999999",
        "lightgrey" | "lightgray" => "#d9d9d9",
        "darkgrey" | "darkgray" => "#555555",
        "black" => "#000000",
        "white" => "#ffffff",
        "cyan" => "#17becf",
        "magenta" => "#e7298a",
        "teal" => "#1b9e77",
        "navy" => "#1f3a93",
        "olive" => "#808000",
        "maroon" => "#800000",
        "gold" => "#d4a017",
        "lightblue" => "#a6cee3",
        "darkblue" => "#08519c",
        "lightgreen" => "#b2df8a",
        "darkgreen" => "#1a7a1a",
        "darkred" => "#a50f15",
        _ => return None,
    };
    stamp_plot::Color::from_hex(hex)
}

/// The plot settings with a JSON patch merged in (RFC 7386: objects are
/// merged, other values replaced).
fn patched_settings(
    settings: &PlotSettings,
    patch: &Map<String, Value>,
) -> Result<PlotSettings, String> {
    let mut value = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    let known = value
        .as_object()
        .map(|o| o.keys().cloned().collect::<Vec<_>>());
    for key in patch.keys() {
        if !known.as_ref().is_some_and(|k| k.contains(key)) {
            return Err(format!(
                "Plots have no option {key}; get_plot_options gives the options."
            ));
        }
    }
    merge(&mut value, &Value::Object(patch.clone()));
    let patched: PlotSettings =
        serde_json::from_value(value).map_err(|e| format!("These options are not valid: {e}"))?;
    // Unknown fields deep inside are dropped by serde rather than refused:
    // catch them by comparing what was asked for with what was kept.
    let kept = serde_json::to_value(&patched).map_err(|e| e.to_string())?;
    if let Some(path) = missing_path(&Value::Object(patch.clone()), &kept, String::new()) {
        return Err(format!(
            "Plots have no option {path}; get_plot_options gives the options."
        ));
    }
    Ok(patched)
}

fn merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                match target.get_mut(key) {
                    Some(existing) if value.is_object() && existing.is_object() => {
                        merge(existing, value)
                    }
                    _ => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

/// The first key of `patch` (as a dotted path) that is not in `kept`.
fn missing_path(patch: &Value, kept: &Value, prefix: String) -> Option<String> {
    let (Value::Object(patch), Value::Object(kept)) = (patch, kept) else {
        return None;
    };
    for (key, value) in patch {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match kept.get(key) {
            None => return Some(path),
            Some(inner) => {
                if let Some(path) = missing_path(value, inner, path) {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Letters and digits only, in lower case: how names are compared.
fn simplified(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

/// The choice whose name matches `wanted`.
fn choose<T: Copy>(
    what: &str,
    wanted: &str,
    choices: &[T],
    name: impl Fn(T) -> String,
) -> Result<T, String> {
    let simple = simplified(wanted);
    choices
        .iter()
        .copied()
        .find(|c| simplified(&name(*c)) == simple)
        .ok_or_else(|| {
            let names: Vec<String> = choices.iter().map(|c| format!("'{}'", name(*c))).collect();
            format!("Unknown {what} '{wanted}'. Choices: {}.", names.join(", "))
        })
}

fn choose_string(what: &str, wanted: &str, choices: &[String]) -> Result<String, String> {
    let simple = simplified(wanted);
    choices
        .iter()
        .find(|c| **c == wanted)
        .or_else(|| choices.iter().find(|c| simplified(c) == simple))
        .cloned()
        .ok_or_else(|| {
            let shown: Vec<&str> = choices.iter().take(50).map(String::as_str).collect();
            let more = if choices.len() > 50 { ", ..." } else { "" };
            format!(
                "Unknown {what} '{wanted}'. Choices: {}{more}.",
                shown.join(", ")
            )
        })
}

/// Open the files named by the assistant: a profile, its metadata, or
/// both. Every file is read before anything changes.
fn load_data(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    const KNOWN: [&str; 5] = [
        "profile",
        "metadata",
        "biom_metadata_field",
        "mothur_groups",
        "mothur_names",
    ];
    if let Some(key) = args.keys().find(|k| !KNOWN.contains(&k.as_str())) {
        return Err(format!(
            "load_data has no setting {key}; it takes {}.",
            KNOWN.join(", ")
        ));
    }
    let profile: Vec<&str> = match args.get("profile") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(path)) => vec![path.as_str()],
        Some(Value::Array(paths)) => paths
            .iter()
            .map(Value::as_str)
            .collect::<Option<_>>()
            .ok_or("'profile' must be a path or a list of paths.")?,
        Some(_) => return Err("'profile' must be a path or a list of paths.".to_string()),
    };
    let metadata = optional_str(args, "metadata")?;
    let groups = optional_str(args, "mothur_groups")?;
    let names = optional_str(args, "mothur_names")?;
    // Outer `None`: not given (STAMP chooses); inner `None`: the ids alone.
    let biom_field = match args.get("biom_metadata_field") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(field)) => Some(Some(field.as_str())),
        Some(_) => return Err("'biom_metadata_field' must be a string or null.".to_string()),
    };
    if profile.is_empty() {
        if metadata.is_none() {
            return Err("Give 'profile', 'metadata' or both.".to_string());
        }
        if groups.is_some() || names.is_some() || biom_field.is_some() {
            return Err(
                "'biom_metadata_field', 'mothur_groups' and 'mothur_names' go with 'profile'."
                    .to_string(),
            );
        }
        if app.session.tree.is_none() {
            return Err(
                "No profile is loaded: give 'profile' as well as 'metadata'. Nothing was \
                 changed."
                    .to_string(),
            );
        }
    }
    let unchanged = |error: String| format!("{error} Nothing was changed.");
    let profile = profile
        .into_iter()
        .map(read_file)
        .collect::<Result<Vec<_>, _>>()
        .map_err(unchanged)?;
    let metadata = metadata.map(read_file).transpose().map_err(unchanged)?;
    let groups = groups.map(read_file).transpose().map_err(unchanged)?;
    let names = names.map(read_file).transpose().map_err(unchanged)?;

    let mut out = Vec::new();
    let loaded_profile = !profile.is_empty();
    if loaded_profile {
        load_profile(app, profile, biom_field, groups, names).map_err(unchanged)?;
        let s = &app.session;
        if let Some(tree) = &s.tree {
            out.push(format!(
                "Loaded the profile {}: {} samples; hierarchy: {}.",
                s.profile_name,
                tree.num_samples(),
                tree.hierarchy_headings().join(" > ")
            ));
        }
    }
    match metadata {
        Some(file) => match app.use_metadata(&file.name, &file.text()) {
            Ok(()) => {
                let s = &app.session;
                if let Some(m) = &s.metadata {
                    out.push(format!(
                        "Loaded the metadata {}: {} samples of the profile; fields: {}.",
                        s.metadata_name,
                        m.samples().len(),
                        m.fields().join(", ")
                    ));
                }
                if !s.warnings.is_empty() {
                    out.push(format!("Warnings: {}", s.warnings.join(" ")));
                }
            }
            // The profile has changed: say what did not.
            Err(message) if loaded_profile => out.push(format!(
                "The metadata {} could not be used, so no metadata is loaded: {message}",
                file.name
            )),
            Err(message) => {
                return Err(unchanged(format!(
                    "Could not load {}: {message}",
                    file.name
                )))
            }
        },
        None if loaded_profile => out.push(
            "No metadata is loaded; give 'metadata' to load a file for this profile.".to_string(),
        ),
        None => {}
    }
    if loaded_profile {
        out.push("Earlier changes can no longer be undone.".to_string());
    }
    Ok(Outcome::text(out.join("\n")))
}

/// Open a profile as File > Load profile does, making for a BIOM table or
/// mothur file the choices its dialog would ask for. An error if it could
/// not be loaded, when the profile loaded before stays.
fn load_profile(
    app: &mut StampApp,
    files: Vec<crate::platform::OpenedFile>,
    biom_field: Option<Option<&str>>,
    groups: Option<crate::platform::OpenedFile>,
    names: Option<crate::platform::OpenedFile>,
) -> Result<(), String> {
    let before = app.session.tree.clone();
    let mothur = groups.is_some() || names.is_some();
    app.open_profile_files(files);
    match app.pending_profile() {
        Some("BIOM") => {
            if mothur {
                app.cancel_pending_profile();
                return Err("This is a BIOM table, not a mothur taxonomy file.".to_string());
            }
            if let Some(field) = biom_field {
                app.choose_biom_field(field);
            }
            app.accept_pending_profile();
        }
        Some(_) => match groups {
            Some(groups) => {
                app.choose_mothur_files(groups, names);
                app.accept_pending_profile();
            }
            None => {
                app.cancel_pending_profile();
                return Err("A mothur taxonomy file needs its groups file: give \
                            'mothur_groups'."
                    .to_string());
            }
        },
        // Loaded already, or failed.
        None => {}
    }
    let loaded = match (&before, &app.session.tree) {
        (Some(before), Some(now)) => !std::sync::Arc::ptr_eq(before, now),
        (None, Some(_)) => true,
        (_, None) => false,
    };
    if loaded {
        Ok(())
    } else {
        app.cancel_pending_profile();
        Err(app
            .notice()
            .unwrap_or("STAMP could not read the profile.")
            .to_string())
    }
}

/// Read a file named by the assistant, by its full path.
#[cfg(not(target_arch = "wasm32"))]
fn read_file(path: &str) -> Result<crate::platform::OpenedFile, String> {
    use std::path::{Path, PathBuf};
    let home = || {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    };
    let full = match path.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => home()
            .map(|home| home.join(rest.trim_start_matches(['/', '\\'])))
            .unwrap_or_else(|| PathBuf::from(path)),
        _ => PathBuf::from(path),
    };
    if !full.is_absolute() {
        return Err(format!(
            "{path} is not a full path: give the whole path of the file (\"~/\" stands for \
             the home folder)."
        ));
    }
    if full.is_dir() {
        return Err(format!(
            "{path} is a folder: give the files in it (several files of one profiler, such \
             as Kraken reports, are combined)."
        ));
    }
    let bytes = std::fs::read(&full).map_err(|e| format!("Could not read {path}: {e}."))?;
    let name = Path::new(&full)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned());
    crate::platform::OpenedFile::new(name, bytes)
}

/// In the browser, files are only opened by the user.
#[cfg(target_arch = "wasm32")]
fn read_file(path: &str) -> Result<crate::platform::OpenedFile, String> {
    Err(format!(
        "{path} cannot be opened: in the browser, only the user can open files."
    ))
}

fn optional_str<'a>(args: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(format!("'{key}' must be a string.")),
    }
}

fn optional_count(args: &Map<String, Value>, key: &str) -> Result<Option<usize>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(|n| Some(n as usize))
            .ok_or_else(|| format!("'{key}' must be a whole number of at least zero.")),
    }
}

fn set_analysis(app: &mut StampApp, args: &Map<String, Value>) -> Result<Outcome, String> {
    const KNOWN: [&str; 19] = [
        "mode",
        "profile_level",
        "parent_level",
        "group_field",
        "sample1",
        "sample2",
        "group1",
        "group2",
        "groups",
        "pair_field",
        "correlation_field",
        "test",
        "test_type",
        "ci_method",
        "coverage",
        "correction",
        "transform",
        "post_hoc_test",
        "p_value_filter",
    ];
    if let Some(key) = args.keys().find(|k| !KNOWN.contains(&k.as_str())) {
        return Err(format!("set_analysis has no setting {key}."));
    }
    if args.is_empty() {
        return Err("Give at least one setting to change.".to_string());
    }
    let Some(tree) = app.session.tree.clone() else {
        return Err("No profile is loaded: the user has to load one first.".to_string());
    };
    // Everything is checked before anything is changed, so that a mistake
    // leaves the session as it was.
    let mode = match optional_str(args, "mode")? {
        Some(name) => choose("analysis type", name, &Mode::ALL, |m| m.name().to_string())?,
        None => app.mode,
    };
    let headings: Vec<String> = tree.hierarchy_headings().to_vec();
    let profile_level = optional_str(args, "profile_level")?
        .map(|l| choose_string("profile level", l, &headings))
        .transpose()?;
    let parent_level = match optional_str(args, "parent_level")? {
        None => None,
        Some(name) if simplified(name) == "entiresample" => Some(None),
        Some(name) => Some(Some(choose_string("parent level", name, &headings)?)),
    };
    let level = profile_level
        .clone()
        .unwrap_or_else(|| app.session.profile_level.clone());
    if let Some(Some(parent)) = &parent_level {
        let depth = |h: &str| headings.iter().position(|x| x == h);
        if depth(parent) >= depth(&level) {
            return Err(format!(
                "The parent level must be above the profile level ({level})."
            ));
        }
    }
    let metadata_fields: Vec<String> = app
        .session
        .metadata
        .as_ref()
        .map(|m| m.fields().to_vec())
        .unwrap_or_default();
    let needs_metadata = |key: &str| -> Result<(), String> {
        if metadata_fields.is_empty() {
            Err(format!(
                "'{key}' needs metadata: the user has to load a metadata file."
            ))
        } else {
            Ok(())
        }
    };
    let group_field = match optional_str(args, "group_field")? {
        Some(field) => {
            needs_metadata("group_field")?;
            Some(choose_string("metadata field", field, &metadata_fields)?)
        }
        None => None,
    };
    let group_names: Vec<String> = {
        let field = group_field
            .clone()
            .or_else(|| app.session.group_field.clone());
        app.session
            .metadata
            .as_ref()
            .zip(field)
            .and_then(|(m, f)| m.groups(&f).ok())
            .map(|g| g.names().map(str::to_string).collect())
            .unwrap_or_default()
    };
    let group = |key: &str| -> Result<Option<String>, String> {
        match optional_str(args, key)? {
            Some(name) => {
                needs_metadata(key)?;
                Ok(Some(choose_string("group", name, &group_names)?))
            }
            None => Ok(None),
        }
    };
    let group1 = group("group1")?;
    let group2 = match args.get("group2") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(name)) if simplified(name) == "allothersamples" => Some(None),
        Some(_) => Some(group("group2")?),
    };
    let groups = match args.get("groups") {
        None | Some(Value::Null) => None,
        Some(Value::Array(items)) => {
            needs_metadata("groups")?;
            let mut chosen = Vec::new();
            for item in items {
                let name = item.as_str().ok_or("'groups' must be a list of names.")?;
                chosen.push(choose_string("group", name, &group_names)?);
            }
            chosen.sort();
            chosen.dedup();
            if chosen.len() < 2 {
                return Err("A multiple-group analysis needs at least two groups.".to_string());
            }
            Some(chosen)
        }
        Some(_) => return Err("'groups' must be a list of names.".to_string()),
    };
    let pair_field = match args.get("pair_field") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(field)) => {
            needs_metadata("pair_field")?;
            Some(Some(choose_string(
                "metadata field",
                field,
                &metadata_fields,
            )?))
        }
        Some(_) => return Err("'pair_field' must be a string or null.".to_string()),
    };
    let samples: Vec<String> = tree.sample_names().to_vec();
    let sample1 = optional_str(args, "sample1")?
        .map(|s| choose_string("sample", s, &samples))
        .transpose()?;
    let sample2 = optional_str(args, "sample2")?
        .map(|s| choose_string("sample", s, &samples))
        .transpose()?;
    let correlation_field = match optional_str(args, "correlation_field")? {
        Some(field) => {
            needs_metadata("correlation_field")?;
            Some(choose_string(
                "numeric metadata field",
                field,
                &app.session.numeric_fields(),
            )?)
        }
        None => None,
    };
    let test = optional_str(args, "test")?;
    let test_type = optional_str(args, "test_type")?
        .map(|t| choose("test type", t, &TestType::ALL, |t| t.name().to_string()))
        .transpose()?;
    let ci_method = optional_str(args, "ci_method")?
        .map(|t| {
            choose("confidence interval method", t, &SampleCi::ALL, |c| {
                c.name().to_string()
            })
        })
        .transpose()?;
    let correction = optional_str(args, "correction")?
        .map(|t| choose("correction", t, &Correction::ALL, |c| c.name().to_string()))
        .transpose()?;
    let transform = optional_str(args, "transform")?
        .map(|t| choose("transform", t, &Transform::ALL, |c| c.name().to_string()))
        .transpose()?;
    let post_hoc_test = optional_str(args, "post_hoc_test")?
        .map(|t| {
            choose("post-hoc test", t, &PostHocTest::ALL, |c| {
                c.name().to_string()
            })
        })
        .transpose()?;
    let coverage = match args.get("coverage") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let mut c = value.as_f64().ok_or("'coverage' must be a number.")?;
            if c > 1.0 {
                c /= 100.0;
            }
            if !(0.5..1.0).contains(&c) {
                return Err("'coverage' must be between 0.5 and 1 (or 50 and 100).".to_string());
            }
            Some(c)
        }
    };
    let p_value_filter = match args.get("p_value_filter") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(value) => {
            let p = value
                .as_f64()
                .ok_or("'p_value_filter' must be a number or null.")?;
            if !(0.0..=1.0).contains(&p) {
                return Err("'p_value_filter' must be between 0 and 1.".to_string());
            }
            Some(Some(p))
        }
    };
    let only_in = |key: &str, present: bool, modes: &[Mode]| -> Result<(), String> {
        if present && !modes.contains(&mode) {
            let names: Vec<&str> = modes.iter().map(|m| m.name()).collect();
            Err(format!(
                "'{key}' belongs to the {} analysis, not {}.",
                names.join(" or "),
                mode.name()
            ))
        } else {
            Ok(())
        }
    };
    only_in("sample1", sample1.is_some(), &[Mode::TwoSamples])?;
    only_in("sample2", sample2.is_some(), &[Mode::TwoSamples])?;
    only_in("ci_method", ci_method.is_some(), &[Mode::TwoSamples])?;
    only_in("group1", group1.is_some(), &[Mode::TwoGroups])?;
    only_in("group2", group2.is_some(), &[Mode::TwoGroups])?;
    only_in(
        "pair_field",
        pair_field.is_some(),
        &[Mode::TwoGroups, Mode::MultipleGroups],
    )?;
    only_in("groups", groups.is_some(), &[Mode::MultipleGroups])?;
    only_in(
        "post_hoc_test",
        post_hoc_test.is_some(),
        &[Mode::MultipleGroups],
    )?;
    only_in(
        "correlation_field",
        correlation_field.is_some(),
        &[Mode::Correlation],
    )?;
    only_in(
        "test_type",
        test_type.is_some(),
        &[Mode::TwoSamples, Mode::TwoGroups],
    )?;
    only_in(
        "transform",
        transform.is_some(),
        &[Mode::TwoGroups, Mode::MultipleGroups],
    )?;
    only_in(
        "coverage",
        coverage.is_some(),
        &[Mode::TwoSamples, Mode::TwoGroups, Mode::MultipleGroups],
    )?;
    enum Test {
        Sample(SampleTest),
        Group(GroupTest),
        Multi(MultiGroupTest),
        Correlation(CorrelationMethod),
    }
    let test = match test {
        None => None,
        Some(name) => Some(match mode {
            Mode::TwoSamples => Test::Sample(choose("test", name, &SampleTest::ALL, |t| {
                t.name().to_string()
            })?),
            Mode::TwoGroups => Test::Group(choose("test", name, &GroupTest::ALL, |t| {
                t.name().to_string()
            })?),
            Mode::MultipleGroups => Test::Multi(choose("test", name, &MultiGroupTest::ALL, |t| {
                t.name().to_string()
            })?),
            Mode::Correlation => {
                Test::Correlation(choose("correlation", name, &CorrelationMethod::ALL, |t| {
                    t.name().to_string()
                })?)
            }
        }),
    };

    app.remember_for_undo("set_analysis");
    if mode != app.mode {
        app.set_mode(mode);
    }
    if profile_level.is_some() || parent_level.is_some() {
        let parent = match parent_level {
            Some(parent) => parent,
            None => app.session.parent_level.clone(),
        };
        app.set_profile_level(&level, parent.as_deref());
    }
    if let Some(field) = group_field {
        app.session.set_group_field(&field);
    }
    let s = &mut app.session;
    let mut rerun = false;
    if let Some(name) = sample1 {
        s.samples.sample1 = name;
        rerun = true;
    }
    if let Some(name) = sample2 {
        s.samples.sample2 = name;
        rerun = true;
    }
    if let Some(name) = group1 {
        s.groups.group1 = name;
        rerun = true;
    }
    if let Some(name) = group2 {
        s.groups.group2 = name;
        rerun = true;
    }
    if let Some(field) = pair_field {
        if mode == Mode::MultipleGroups {
            s.multi.pair_field = field;
        } else {
            s.groups.pair_field = field;
        }
        rerun = true;
    }
    if let Some(groups) = groups {
        s.multi.active_groups = groups;
        rerun = true;
    }
    if let Some(field) = correlation_field {
        s.correlation.field = Some(field);
        rerun = true;
    }
    match test {
        Some(Test::Sample(t)) => s.samples.analysis.test = t,
        Some(Test::Group(t)) => s.groups.analysis.test = t,
        Some(Test::Multi(t)) => s.multi.analysis.test = t,
        Some(Test::Correlation(t)) => s.correlation.analysis.method = t,
        None => {}
    }
    rerun |= test.is_some();
    if let Some(t) = test_type {
        match mode {
            Mode::TwoSamples => s.samples.analysis.test_type = t,
            _ => s.groups.analysis.test_type = t,
        }
        rerun = true;
    }
    if let Some(c) = ci_method {
        s.samples.analysis.ci_method = c;
        rerun = true;
    }
    if let Some(c) = coverage {
        match mode {
            Mode::TwoSamples => s.samples.analysis.coverage = c,
            Mode::TwoGroups => s.groups.analysis.coverage = c,
            _ => s.multi.post_hoc_coverage = c,
        }
        rerun = mode != Mode::MultipleGroups || rerun;
    }
    if let Some(t) = transform {
        match mode {
            Mode::TwoGroups => s.groups.analysis.transform = t,
            _ => s.multi.analysis.transform = t,
        }
        rerun = true;
    }
    if let Some(t) = post_hoc_test {
        s.multi.post_hoc_test = t;
    }
    if rerun {
        s.invalidate(mode);
    }
    if let Some(correction) = correction {
        s.set_correction(mode, correction);
    }
    if let Some(level) = p_value_filter {
        app.set_p_value_filter(level);
    }
    // The post-hoc settings and the coverage of a multiple-group analysis
    // change only the plot.
    app.redraw_plot();
    Ok(Outcome::text("Done."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_follows_json_merge_patch() {
        let mut target = json!({"a": {"b": 1, "c": 2}, "d": [1, 2]});
        merge(&mut target, &json!({"a": {"b": 5}, "d": [3]}));
        assert_eq!(target, json!({"a": {"b": 5, "c": 2}, "d": [3]}));
    }

    #[test]
    fn patches_with_unknown_options_are_refused() {
        let settings = PlotSettings::default();
        let unknown = json!({"no_such_plot": 1});
        let error = patched_settings(&settings, unknown.as_object().unwrap()).unwrap_err();
        assert!(error.contains("no_such_plot"), "{error}");
        let nested = json!({"volcano": {"no_such_option": 1}});
        let error = patched_settings(&settings, nested.as_object().unwrap()).unwrap_err();
        assert!(error.contains("volcano.no_such_option"), "{error}");
        let wrong_type = json!({"ancom_alpha": "high"});
        assert!(patched_settings(&settings, wrong_type.as_object().unwrap()).is_err());
    }

    #[test]
    fn a_valid_patch_changes_only_what_it_names() {
        let settings = PlotSettings::default();
        let patch = json!({"ancom_alpha": 0.01});
        let patched = patched_settings(&settings, patch.as_object().unwrap()).unwrap();
        assert_eq!(patched.ancom_alpha, 0.01);
        let mut expected = settings.clone();
        expected.ancom_alpha = 0.01;
        assert_eq!(patched, expected);
    }

    #[test]
    fn names_match_ignoring_case_and_punctuation() {
        let test = choose("test", "welchs t test", &GroupTest::ALL, |t| {
            t.name().to_string()
        });
        assert_eq!(test, Ok(GroupTest::Welch));
        let error = choose("test", "magic", &GroupTest::ALL, |t| t.name().to_string());
        assert!(error.unwrap_err().contains("Choices"));
    }

    #[test]
    fn colours_by_hex_or_name() {
        assert_eq!(
            parse_color("#ff0000"),
            Some(stamp_plot::Color::rgb(255, 0, 0))
        );
        assert_eq!(
            parse_color("00ff00"),
            Some(stamp_plot::Color::rgb(0, 255, 0))
        );
        assert_eq!(parse_color("Red"), stamp_plot::Color::from_hex("#e41a1c"));
        assert_eq!(
            parse_color("light grey"),
            stamp_plot::Color::from_hex("#d9d9d9")
        );
        assert_eq!(parse_color("sparkly"), None);
    }

    #[test]
    fn every_tool_has_an_object_schema_and_a_unique_name() {
        let tools = list();
        for tool in &tools {
            assert_eq!(tool.input_schema["type"], "object", "{}", tool.name);
            assert_eq!(tools.iter().filter(|t| t.name == tool.name).count(), 1);
        }
    }
}
