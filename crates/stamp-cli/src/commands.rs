//! Implementation of the commands.

use std::fs;
use std::io::Write;
use std::path::Path;

use stamp_core::import::{biom, cog, comet, mgrast, mothur, rita, sandpiper, singlem, NamedText};
use stamp_core::stats::correction::Correction;
use stamp_core::stats::multi_group::MultiGroupEffectSize;
use stamp_core::stats::results::{
    run_correlation_analysis, run_group_analysis, run_multi_group_analysis, run_post_hoc,
    run_sample_analysis, CorrectionSummary, CorrelationAnalysis, CorrelationFilter, GroupAnalysis,
    GroupCountFilter, GroupFilter, MultiGroupAnalysis, MultiGroupFilter, SampleAnalysis,
    SampleCountFilter, SampleFilter,
};
use stamp_core::stats::table::{
    ancom_table, post_hoc_table, sample_table, write_correlation_table, write_group_table,
    write_multi_group_table, Table,
};
use stamp_core::stats::{rng_from_seed, StatsSettings};
use stamp_core::{Groups, Metadata, MultiGroupProfile, ProfileTree, SampleSelection, SecondGroup};

use stamp_plot::export::{export, Format};
use stamp_plot::extended_error_bar::{self, Legend, SortBy};
use stamp_plot::p_values::{p_value_text, PValueKind};

use crate::args::*;
use crate::progress::Task;

type CliResult<T = ()> = Result<T, String>;

pub fn run(cli: Cli) -> CliResult {
    use stamp_plot::axes::with_fonts;
    // Plots are drawn with the font sizes the command asks for.
    match cli.command {
        Command::Info(args) => info(args),
        Command::TwoSamples(args) => with_fonts(fonts_of(&args.plot)?, || two_samples(args)),
        Command::TwoGroups(args) => with_fonts(fonts_of(&args.plot)?, || two_groups(args)),
        Command::MultiGroups(args) => with_fonts(fonts_of(&args.plot)?, || multi_groups(args)),
        Command::PostHoc(args) => with_fonts(fonts_of(&args.plot)?, || post_hoc(args)),
        Command::Correlation(args) => with_fonts(fonts_of(&args.plot)?, || correlation(args)),
        Command::Ancom(args) => with_fonts(fonts_of(&args.plot)?, || ancom(args)),
        Command::CoOccurrence(args) => with_fonts(fonts_of(&args.plot)?, || co_occurrence(args)),
        Command::Convert(args) => convert(args.format),
    }
}

/// The font sizes of the tick labels, axis labels and legends of a plot.
fn fonts_of(plot: &PlotArgs) -> CliResult<stamp_plot::axes::Fonts> {
    for size in [
        plot.tick_font_size,
        plot.axis_label_font_size,
        plot.legend_font_size,
    ] {
        if !(1.0..=72.0).contains(&size) {
            return Err("font sizes must lie between 1 and 72 points".to_string());
        }
    }
    Ok(stamp_plot::axes::Fonts {
        tick: plot.tick_font_size,
        axis_label: plot.axis_label_font_size,
        legend: plot.legend_font_size,
        family: plot.font.into(),
    })
}

// ------------------------------------------------------------------- files

fn read_bytes(path: &Path) -> CliResult<Vec<u8>> {
    fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn read_text(path: &Path) -> CliResult<String> {
    Ok(stamp_core::text::decode(&read_bytes(path)?).into_owned())
}

fn write_output(path: Option<&Path>, text: &str) -> CliResult {
    match path {
        Some(path) => {
            fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
        }
        None => {
            // A closed pipe (for example `| head`) is not an error.
            let _ = std::io::stdout().lock().write_all(text.as_bytes());
            Ok(())
        }
    }
}

fn core<T>(result: stamp_core::Result<T>) -> CliResult<T> {
    result.map_err(|e| e.to_string())
}

fn load_profile(path: &Path) -> CliResult<ProfileTree> {
    let _task = Task::start(&format!("Reading {}", path.display()));
    core(ProfileTree::read(&read_text(path)?)).map_err(|e| format!("{}: {e}", path.display()))
}

/// A progress callback for the analyses of every feature, shown by `task`.
fn testing_features(task: &Task) -> impl FnMut(usize, usize) -> bool + '_ {
    move |done, total| {
        task.set("Testing features", done, total);
        true
    }
}

/// Carry out the slow part of a community plot, showing its progress.
fn run_community(
    run: &mut stamp_plot::community_plots::CommunityRun,
) -> stamp_plot::community_plots::CommunityOutput {
    use stamp_core::stats::results::{Control, Step};
    let task = Task::start("Computing");
    match run.advance(&mut |stage, done, total| {
        task.set(stage, done, total);
        Control::Continue
    }) {
        Step::Done(output) => output,
        Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
    }
}

/// Read the metadata, report its warnings, apply the sample filter and
/// leave out the samples that were excluded.
fn load_metadata(
    tree: &ProfileTree,
    path: &Path,
    excluded: &[String],
    filter: &SampleFilterArgs,
) -> CliResult<Metadata> {
    let (mut metadata, warnings) = core(Metadata::read(&read_text(path)?, tree))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    apply_sample_filter(&mut metadata, filter)?;
    for sample in excluded {
        if !metadata.samples().contains(sample) {
            return Err(format!(
                "cannot exclude '{sample}': no such sample in the metadata"
            ));
        }
        metadata.set_active(sample, false);
    }
    Ok(metadata)
}

/// Keep only the samples that `--where` and `--where-not` choose, and say
/// how many that is.
fn apply_sample_filter(metadata: &mut Metadata, filter: &SampleFilterArgs) -> CliResult {
    let selection = core(SampleSelection::parse(&filter.where_, &filter.where_not))?;
    if selection.is_empty() {
        return Ok(());
    }
    let description = selection.describe();
    let kept = core(metadata.set_selection(selection))?;
    eprintln!(
        "Sample filter ({description}): kept {kept} of {} samples.",
        metadata.samples().len()
    );
    if kept == 0 {
        return Err("the sample filter leaves no samples".to_string());
    }
    Ok(())
}

/// Group the samples by a metadata field.
fn groups_of(metadata: &Metadata, field: &str) -> CliResult<Groups> {
    core(metadata.groups(field)).map_err(|e| {
        format!(
            "{e} Fields in the metadata file: {}.",
            metadata.fields().join(", ")
        )
    })
}

/// The level to analyse, and the parent level as the core expects it.
fn levels<'a>(tree: &'a ProfileTree, args: &'a ProfileArgs) -> (Option<&'a str>, &'a str) {
    let profile_level = args.profile_level.as_deref().unwrap_or_else(|| {
        tree.hierarchy_headings()
            .last()
            .expect("profiles have a level")
    });
    (args.parent_level.as_deref(), profile_level)
}

fn report_correction(summary: &CorrectionSummary) {
    if summary.method != Correction::None {
        for (label, value) in &summary.info {
            eprintln!("{}: {label}: {value}", summary.method.name());
        }
    }
    if let Some(warning) = summary.warning {
        eprintln!("warning: {warning}");
    }
}

fn settings(args: &ResamplingArgs) -> StatsSettings {
    StatsSettings {
        pseudocount: args.pseudocount,
        replicates: args.replicates,
    }
}

fn check_coverage(coverage: f64) -> CliResult {
    if coverage > 0.0 && coverage < 1.0 {
        Ok(())
    } else {
        Err(format!(
            "--coverage must be between 0 and 1, for example 0.95 (got {coverage})"
        ))
    }
}

/// Up to two effect-size filters.
fn effect_size_filters<T: Copy>(
    texts: &[String],
    parse: impl Fn(&str) -> Result<(T, f64), String>,
) -> CliResult<[Option<(T, f64)>; 2]> {
    if texts.len() > 2 {
        return Err("at most two --effect-size-filter options can be given".to_string());
    }
    let mut filters = [None, None];
    for (slot, text) in filters.iter_mut().zip(texts) {
        *slot = Some(parse(text).map_err(|e| format!("--effect-size-filter: {e}"))?);
    }
    Ok(filters)
}

// ---------------------------------------------------------------- commands

fn info(args: InfoArgs) -> CliResult {
    let filter = &args.sample_filter;
    if args.metadata.is_none() && !(filter.where_.is_empty() && filter.where_not.is_empty()) {
        return Err("--where and --where-not need a metadata file (--metadata)".to_string());
    }
    let tree = load_profile(&args.profile)?;
    let mut out = String::new();
    out.push_str(&format!("Profile: {}\n", args.profile.display()));
    out.push_str(&format!(
        "Hierarchical levels: {}\n",
        tree.hierarchy_headings().join(", ")
    ));
    out.push_str(&format!(
        "Features at the most specific level: {}\n",
        tree.num_leaves()
    ));
    out.push_str(&format!("Samples ({}):\n", tree.num_samples()));
    for sample in tree.sample_names() {
        let total = tree.sample_total(sample).unwrap_or(0.0);
        out.push_str(&format!("  {sample}\t{total} sequences\n"));
    }

    if let Some(path) = &args.metadata {
        let (mut metadata, warnings) = core(Metadata::read(&read_text(path)?, &tree))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        for warning in warnings {
            eprintln!("warning: {warning}");
        }
        apply_sample_filter(&mut metadata, &args.sample_filter)?;
        if !metadata.selection().is_empty() {
            out.push_str(&format!(
                "Sample filter: {} ({} of {} samples kept)\n",
                metadata.selection().describe(),
                metadata.num_selected(),
                metadata.samples().len()
            ));
        }
        out.push_str(&format!("Metadata fields ({}):\n", metadata.fields().len()));
        for field in metadata.fields() {
            let groups = core(metadata.groups(field))?;
            let sizes: Vec<String> = groups
                .names()
                .map(|g| format!("{g} ({})", groups.samples(g).map_or(0, <[String]>::len)))
                .collect();
            let shown = if sizes.len() > 12 {
                format!("{} distinct values", sizes.len())
            } else {
                sizes.join(", ")
            };
            out.push_str(&format!("  {field}: {shown}\n"));
        }
    }
    write_output(None, &out)
}

fn sample_count_filter(text: &str, option: &str) -> CliResult<SampleCountFilter> {
    let (kind, values) = parse_count_filter(text).map_err(|e| format!("{option}: {e}"))?;
    match kind.as_str() {
        "maximum" => Ok(SampleCountFilter::Maximum(values[0])),
        "minimum" => Ok(SampleCountFilter::Minimum(values[0])),
        "independent" => Ok(SampleCountFilter::Independent(values[0], values[1])),
        other => Err(format!(
            "{option}: unknown filter '{other}' (use maximum, minimum or independent)"
        )),
    }
}

fn group_count_filter(text: &str, option: &str) -> CliResult<GroupCountFilter> {
    let (kind, values) = parse_count_filter(text).map_err(|e| format!("{option}: {e}"))?;
    match kind.as_str() {
        "maximum" => Ok(GroupCountFilter::Maximum(values[0])),
        "minimum" => Ok(GroupCountFilter::Minimum(values[0])),
        "independent-minimum" => Ok(GroupCountFilter::IndependentMinimum(values[0], values[1])),
        "independent-maximum" => Ok(GroupCountFilter::IndependentMaximum(values[0], values[1])),
        other => Err(format!(
            "{option}: unknown filter '{other}' (use maximum, minimum, independent-minimum or independent-maximum)"
        )),
    }
}

fn two_samples(args: TwoSamplesArgs) -> CliResult {
    check_coverage(args.coverage)?;
    let [effect_size1, effect_size2] =
        effect_size_filters(&args.effect_size_filters, parse_sample_effect_size)?;
    let filter = SampleFilter {
        selected_features: None,
        significance: args.significance,
        sequences: args
            .sequence_filter
            .as_deref()
            .map(|t| sample_count_filter(t, "--sequence-filter"))
            .transpose()?,
        parent_sequences: args
            .parent_sequence_filter
            .as_deref()
            .map(|t| sample_count_filter(t, "--parent-sequence-filter"))
            .transpose()?,
        effect_size1,
        effect_size2,
        operator: args.effect_size_operator.into(),
    };

    let tree = load_profile(&args.profile.profile)?;
    let (parent_level, profile_level) = levels(&tree, &args.profile);
    let profile = core(tree.sample_profile(
        &args.sample1,
        &args.sample2,
        parent_level,
        profile_level,
        args.profile.unclassified.into(),
    ))?;

    let analysis = SampleAnalysis {
        test: args.test.into(),
        test_type: args.test_type.into(),
        ci_method: args.ci.into(),
        coverage: args.coverage,
        correction: args.correction.into(),
    };
    if !analysis.test.has_one_sided() && matches!(args.test_type, TestTypeArg::OneSided) {
        return Err(format!(
            "the {} has no one-sided p-value",
            analysis.test.name()
        ));
    }
    let settings = settings(&args.resampling);
    let mut rng = rng_from_seed(args.resampling.seed);
    let task = Task::start("Testing features");
    let results = run_sample_analysis(
        &profile,
        analysis,
        &settings,
        &mut rng,
        &mut testing_features(&task),
    )
    .expect("analysis is never cancelled");
    drop(task);
    report_correction(&results.correction);

    let rows = results.filter(&filter, &settings);
    if let Some(path) = &args.plot.plot {
        use stamp_plot::{composition, histogram, sample_plots, scatter, volcano};
        let plot = &args.plot;
        let kind = plot.plot_type.unwrap_or(PlotTypeArg::ExtendedErrorBar);
        let size = |default: f64| plot.plot_height.unwrap_or(default);
        let figure = match kind {
            PlotTypeArg::ExtendedErrorBar => {
                let data = extended_error_bar::Data::from_sample_results(
                    &profile,
                    &results,
                    rows.iter().copied(),
                );
                extended_error_bar::plot(&data, &error_bar_options(plot))
            }
            PlotTypeArg::Bar => sample_plots::stat_bar(
                &profile,
                &results,
                rows.iter().copied(),
                &sample_plots::StatBarOptions {
                    width: plot.plot_width,
                    row_height: plot.plot_row_height,
                    ..Default::default()
                },
            ),
            PlotTypeArg::ProfileBar => sample_plots::profile_bar(
                &profile,
                PValueKind::of(results.correction.method),
                rows.iter().copied(),
                &sample_plots::ProfileBarOptions {
                    height: size(6.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Scatter => scatter::plot(
                &scatter::Data::from_sample_profile(&profile),
                &scatter::Options {
                    width: plot.plot_width,
                    height: size(7.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::SequenceHistogram => histogram::sequence_histogram(
                &profile,
                &histogram::SequenceOptions {
                    width: plot.plot_width,
                    height: size(7.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::PValueHistogram => {
                let uncorrected = plot.plot_uncorrected_p_values;
                let p: Vec<f64> = results
                    .rows
                    .iter()
                    .map(|r| {
                        if uncorrected {
                            r.p_value
                        } else {
                            r.p_value_corrected
                        }
                    })
                    .collect();
                histogram::p_value_histogram(
                    &p,
                    &histogram::PValueOptions {
                        label: PValueKind::shown(results.correction.method, !uncorrected)
                            .label()
                            .to_string(),
                        width: plot.plot_width,
                        height: size(7.0),
                        ..Default::default()
                    },
                )
            }
            PlotTypeArg::MultipleComparison => sample_plots::multiple_comparison(
                &results,
                &sample_plots::MultCompOptions {
                    width: plot.plot_width,
                    height: size(3.5),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Rarefaction => {
                let names = profile.profile.sample_names.clone();
                rarefaction_figure(
                    &tree,
                    &names,
                    profile_level,
                    vec![0, 1],
                    names.clone(),
                    plot,
                )?
            }
            PlotTypeArg::Volcano => volcano::plot(
                &volcano::Data::from_sample_results(&profile, &results),
                &volcano::Options {
                    width: plot.plot_width,
                    height: size(6.0),
                    corrected: !plot.plot_uncorrected_p_values,
                    ..Default::default()
                },
            ),
            PlotTypeArg::Composition => composition::plot(
                &composition::Data::from_profile(
                    &profile.profile,
                    vec![0, 1],
                    profile.profile.sample_names.clone(),
                ),
                &composition::Options {
                    width: plot.plot_width,
                    height: size(5.0),
                    ..Default::default()
                },
            ),
            kind @ (PlotTypeArg::SharedFeatures | PlotTypeArg::RankAbundance) => proportion_figure(
                kind,
                &composition::Data::from_profile(
                    &profile.profile,
                    vec![0, 1],
                    profile.profile.sample_names.clone(),
                ),
                plot,
            )?,
            other => return Err(unavailable(other, "two samples")),
        };
        write_figure(path, &figure, plot.plot_dpi)?;
    }
    let table = sample_table(&profile, &results, rows);
    write_table(args.profile.output.as_deref(), &table)
}

fn two_groups(args: TwoGroupsArgs) -> CliResult {
    check_coverage(args.coverage)?;
    let [effect_size1, effect_size2] =
        effect_size_filters(&args.effect_size_filters, parse_group_effect_size)?;
    let filter = GroupFilter {
        selected_features: None,
        significance: args.significance,
        sequences: args
            .sequence_filter
            .as_deref()
            .map(|t| group_count_filter(t, "--sequence-filter"))
            .transpose()?,
        parent_sequences: args
            .parent_sequence_filter
            .as_deref()
            .map(|t| group_count_filter(t, "--parent-sequence-filter"))
            .transpose()?,
        effect_size1,
        effect_size2,
        operator: args.effect_size_operator.into(),
    };

    let tree = load_profile(&args.profile.profile)?;
    let metadata = load_metadata(
        &tree,
        &args.metadata.metadata,
        &args.metadata.excluded_samples,
        &args.metadata.sample_filter,
    )?;
    let mut groups = groups_of(&metadata, &args.metadata.field)?;
    let test: stamp_core::stats::two_group::GroupTest = args.test.into();
    match (test.is_paired(), &args.pair_field) {
        (true, Some(field)) => {
            let second = args
                .group2
                .as_deref()
                .ok_or("a paired test needs two named groups: give --group2")?;
            let pairing = core(metadata.paired_groups(&groups, &args.group1, second, field))
                .map_err(|e| with_group_names(e, &groups))?;
            eprintln!("Paired by {field}: {}.", pairing.summary());
            groups = pairing.groups;
        }
        (true, None) => {
            return Err(
                "a paired test needs --pair-field, the metadata field that says \
                which samples form a pair"
                    .to_string(),
            )
        }
        (false, Some(_)) => {
            return Err(
                "--pair-field is for the paired tests (--test paired-t or wilcoxon)".to_string(),
            )
        }
        (false, None) => {}
    }
    let transform: stamp_core::stats::results::Transform = args.transform.into();
    if transform != stamp_core::stats::results::Transform::None && !transform.applies_to(test) {
        return Err(
            "White's test works on counts and cannot be combined with --transform".to_string(),
        );
    }
    let (parent_level, profile_level) = levels(&tree, &args.profile);
    let group2 = match args.group2.as_deref() {
        Some(name) => SecondGroup::Named(name),
        None => SecondGroup::AllOtherSamples,
    };
    let profile = core(tree.group_profile(
        &groups,
        &args.group1,
        group2,
        parent_level,
        profile_level,
        args.profile.unclassified.into(),
    ))
    .map_err(|e| with_group_names(e, &groups))?;

    let analysis = GroupAnalysis {
        test,
        test_type: args.test_type.into(),
        coverage: args.coverage,
        correction: args.correction.into(),
        transform,
    };
    let settings = settings(&args.resampling);
    let mut rng = rng_from_seed(args.resampling.seed);
    let task = Task::start("Testing features");
    let results = run_group_analysis(
        &profile,
        analysis,
        &settings,
        &mut rng,
        &mut testing_features(&task),
    )
    .expect("analysis is never cancelled");
    drop(task);
    report_correction(&results.correction);

    let rows = results.filter(&profile, &filter, &settings);
    if let Some(path) = &args.plot.plot {
        use stamp_plot::{composition, feature_plots, heatmap, scatter, volcano};
        let plot = &args.plot;
        let kind = plot.plot_type.unwrap_or(PlotTypeArg::ExtendedErrorBar);
        let size = |default: f64| plot.plot_height.unwrap_or(default);
        // The bar and box plots show features with their corrected p-values.
        let ranked: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
        let feature_data = |name: &str| -> CliResult<feature_plots::FeatureData> {
            let mut data = feature_plots::FeatureData::from_group_profile(
                &profile,
                name,
                feature_plots::Field::Proportion,
            )
            .ok_or_else(|| format!("unknown feature '{name}'"))?;
            data.p_value = results
                .rows
                .iter()
                .find(|r| r.feature == name)
                .map(|r| stamp_core::stats::results::format_p_value(r.p_value_corrected, -15));
            data.p_value_kind = Some(PValueKind::of(results.correction.method));
            Ok(data)
        };
        let group_samples = [
            profile.samples_in_group1().to_vec(),
            profile.samples_in_group2().to_vec(),
        ];
        let two_group_names = [profile.group_name1.clone(), profile.group_name2.clone()];
        let figure = match kind {
            PlotTypeArg::ExtendedErrorBar => {
                let data = extended_error_bar::Data::from_group_results(
                    &profile,
                    &results,
                    rows.iter().copied(),
                );
                extended_error_bar::plot(&data, &error_bar_options(plot))
            }
            PlotTypeArg::Bar => feature_plots::bar_plot(
                &feature_data(&bar_feature(plot, &ranked)?)?,
                &feature_plots::BarOptions {
                    height: size(6.0),
                    log_scale: args.boxes.bar_log,
                    ..Default::default()
                },
            ),
            PlotTypeArg::Box => {
                let names = box_features(plot, &args.boxes, &ranked)?;
                let data = names
                    .iter()
                    .map(|name| feature_data(name))
                    .collect::<CliResult<Vec<_>>>()?;
                box_figure(
                    &data,
                    &two_group_names,
                    &group_samples,
                    &metadata,
                    plot,
                    &args.boxes,
                )?
            }
            PlotTypeArg::PrevalenceAbundance => {
                let first = profile.samples_in_group1().len();
                let sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
                    .map(|i| usize::from(i >= first))
                    .collect();
                prevalence_figure(
                    &profile.profile,
                    &sample_groups,
                    &two_group_names,
                    &ranked,
                    plot,
                    &args.prevalence,
                )?
            }
            PlotTypeArg::Scatter => scatter::plot(
                &scatter::Data::from_group_profile(&profile, scatter::Spread::Percentiles2And98),
                &scatter::Options {
                    width: plot.plot_width,
                    height: size(7.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Map => {
                let first = profile.samples_in_group1().len();
                let sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
                    .map(|i| usize::from(i >= first))
                    .collect();
                map_figure(
                    &profile.profile,
                    &sample_groups,
                    two_group_names.to_vec(),
                    &metadata,
                    Some(&args.metadata.field),
                    plot,
                    &args.ordination,
                    &args.map,
                )?
            }
            PlotTypeArg::Heatmap => {
                let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                heatmap::plot(
                    &heatmap::Data::from_group_profile(&profile, Some(&names)),
                    &heatmap_options(plot, &args.heatmap, size(7.0))?,
                )
            }
            PlotTypeArg::GroupHeatmap => {
                let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                let first = profile.samples_in_group1().len();
                let sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
                    .map(|i| usize::from(i >= first))
                    .collect();
                group_heatmap_figure(
                    &profile.profile,
                    &sample_groups,
                    &[profile.group_name1.clone(), profile.group_name2.clone()],
                    &names,
                    &metadata,
                    plot,
                    &args.heatmap,
                )?
            }
            PlotTypeArg::Rarefaction => {
                let first = profile.samples_in_group1().len();
                let samples = &profile.profile.sample_names;
                rarefaction_figure(
                    &tree,
                    samples,
                    profile_level,
                    (0..samples.len())
                        .map(|i| usize::from(i >= first))
                        .collect(),
                    vec![profile.group_name1.clone(), profile.group_name2.clone()],
                    plot,
                )?
            }
            PlotTypeArg::Volcano => volcano::plot(
                &volcano::Data::from_group_results(&profile, &results),
                &volcano::Options {
                    width: plot.plot_width,
                    height: size(6.0),
                    corrected: !plot.plot_uncorrected_p_values,
                    ..Default::default()
                },
            ),
            PlotTypeArg::Composition => {
                let first = profile.samples_in_group1().len();
                let samples = profile.profile.sample_names.len();
                composition::plot(
                    &composition::Data::from_profile(
                        &profile.profile,
                        (0..samples).map(|i| usize::from(i >= first)).collect(),
                        vec![profile.group_name1.clone(), profile.group_name2.clone()],
                    ),
                    &composition::Options {
                        width: plot.plot_width,
                        height: size(5.0),
                        ..Default::default()
                    },
                )
            }
            PlotTypeArg::SharedFeatures | PlotTypeArg::RankAbundance | PlotTypeArg::Ancom => {
                let first = profile.samples_in_group1().len();
                let sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
                    .map(|i| usize::from(i >= first))
                    .collect();
                let names = vec![profile.group_name1.clone(), profile.group_name2.clone()];
                if kind == PlotTypeArg::Ancom {
                    let result = run_ancom(
                        &profile.profile,
                        &sample_groups,
                        stamp_core::stats::multi_group::MultiGroupTest::Anova,
                        0.05,
                    )?;
                    ancom_figure(&profile.profile, &result, &names, plot)
                } else {
                    proportion_figure(
                        kind,
                        &composition::Data::from_profile(&profile.profile, sample_groups, names),
                        plot,
                    )?
                }
            }
            PlotTypeArg::AlphaDiversity
            | PlotTypeArg::Pcoa
            | PlotTypeArg::Nmds
            | PlotTypeArg::DistanceBox => {
                let first = profile.samples_in_group1().len();
                let samples = &profile.profile.sample_names;
                community_figure(
                    kind,
                    &tree,
                    samples,
                    profile_level,
                    (0..samples.len())
                        .map(|i| usize::from(i >= first))
                        .collect(),
                    vec![profile.group_name1.clone(), profile.group_name2.clone()],
                    plot,
                    args.resampling.seed,
                    &args.ordination,
                    ordination_styling(
                        &args.ordination,
                        &metadata,
                        &profile.profile,
                        &args.metadata.field,
                    )?,
                )?
            }
            other => return Err(unavailable(other, "two groups")),
        };
        write_figure(path, &figure, plot.plot_dpi)?;
    }
    write_streamed(args.profile.output.as_deref(), |out, progress| {
        write_group_table(out, &profile, &results, &rows, progress)
    })
}

fn with_group_names(error: String, groups: &Groups) -> String {
    if error.starts_with("Unknown group") {
        format!(
            "{error} Groups of '{}': {}.",
            groups.field,
            groups.names().collect::<Vec<_>>().join(", ")
        )
    } else {
        error
    }
}

/// Profile of the requested groups, or of every group with samples.
fn multi_group_profile(
    tree: &ProfileTree,
    profile_args: &ProfileArgs,
    metadata_args: &MetadataArgs,
    requested: &[String],
) -> CliResult<MultiGroupProfile> {
    multi_group_profile_and_metadata(tree, profile_args, metadata_args, requested).map(|(p, _)| p)
}

/// As [`multi_group_profile`], with the metadata it was made from.
fn multi_group_profile_and_metadata(
    tree: &ProfileTree,
    profile_args: &ProfileArgs,
    metadata_args: &MetadataArgs,
    requested: &[String],
) -> CliResult<(MultiGroupProfile, Metadata)> {
    let metadata = load_metadata(
        tree,
        &metadata_args.metadata,
        &metadata_args.excluded_samples,
        &metadata_args.sample_filter,
    )?;
    let groups = groups_of(&metadata, &metadata_args.field)?;
    let (parent_level, profile_level) = levels(tree, profile_args);
    let active: Vec<&str> = if requested.is_empty() {
        groups
            .names()
            .filter(|g| groups.samples(g).is_some_and(|s| !s.is_empty()))
            .collect()
    } else {
        requested.iter().map(String::as_str).collect()
    };
    if active.len() < 2 {
        return Err("at least two groups are needed".to_string());
    }
    let profile = core(tree.multi_group_profile(
        &groups,
        Some(&active),
        parent_level,
        profile_level,
        profile_args.unclassified.into(),
    ))
    .map_err(|e| with_group_names(e, &groups))?;
    Ok((profile, metadata))
}

fn multi_groups(args: MultiGroupsArgs) -> CliResult {
    let tree = load_profile(&args.profile.profile)?;
    let (profile, metadata) =
        multi_group_profile_and_metadata(&tree, &args.profile, &args.metadata, &args.groups)?;

    let analysis = MultiGroupAnalysis {
        test: args.test.into(),
        effect_size: MultiGroupEffectSize::EtaSquared,
        correction: args.correction.into(),
        transform: args.transform.into(),
    };
    let mut rng = rng_from_seed(args.seed);
    let task = Task::start("Testing features");
    let results =
        run_multi_group_analysis(&profile, analysis, &mut rng, &mut testing_features(&task))
            .expect("analysis is never cancelled");
    drop(task);
    report_correction(&results.correction);

    let filter = MultiGroupFilter {
        selected_features: None,
        significance: args.significance,
        min_effect_size: args.min_effect_size,
    };
    let rows = results.filter(&filter);
    if let Some(path) = &args.plot.plot {
        use stamp_plot::{composition, feature_plots, heatmap, volcano};
        let plot = &args.plot;
        let size = |default: f64| plot.plot_height.unwrap_or(default);
        let ranked: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
        let feature_data = |name: &str| -> CliResult<feature_plots::FeatureData> {
            let mut data = feature_plots::FeatureData::from_multi_group_profile(
                &profile,
                name,
                feature_plots::Field::Proportion,
            )
            .ok_or_else(|| format!("unknown feature '{name}'"))?;
            data.p_value = results
                .rows
                .iter()
                .find(|r| r.feature == name)
                .map(|r| stamp_core::stats::results::format_p_value(r.p_value_corrected, -15));
            data.p_value_kind = Some(PValueKind::of(results.correction.method));
            Ok(data)
        };
        let group_samples: Vec<Vec<String>> = profile
            .samples_in_groups()
            .iter()
            .map(|members| members.to_vec())
            .collect();
        let figure = match plot.plot_type.unwrap_or(PlotTypeArg::Heatmap) {
            PlotTypeArg::Heatmap => {
                let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                heatmap::plot(
                    &heatmap::Data::from_multi_group_profile(&profile, Some(&names)),
                    &heatmap_options(plot, &args.heatmap, size(7.0))?,
                )
            }
            PlotTypeArg::GroupHeatmap => {
                let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                let sample_groups: Vec<usize> = profile
                    .samples_in_groups()
                    .iter()
                    .enumerate()
                    .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                    .collect();
                group_heatmap_figure(
                    &profile.profile,
                    &sample_groups,
                    &profile.group_names,
                    &names,
                    &metadata,
                    plot,
                    &args.heatmap,
                )?
            }
            PlotTypeArg::Map => {
                let sample_groups: Vec<usize> = profile
                    .samples_in_groups()
                    .iter()
                    .enumerate()
                    .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                    .collect();
                map_figure(
                    &profile.profile,
                    &sample_groups,
                    profile.group_names.clone(),
                    &metadata,
                    Some(&args.metadata.field),
                    plot,
                    &args.ordination,
                    &args.map,
                )?
            }
            PlotTypeArg::Bar => feature_plots::bar_plot(
                &feature_data(&bar_feature(plot, &ranked)?)?,
                &feature_plots::BarOptions {
                    height: size(6.0),
                    log_scale: args.boxes.bar_log,
                    ..Default::default()
                },
            ),
            PlotTypeArg::Box => {
                let names = box_features(plot, &args.boxes, &ranked)?;
                let data = names
                    .iter()
                    .map(|name| feature_data(name))
                    .collect::<CliResult<Vec<_>>>()?;
                box_figure(
                    &data,
                    &profile.group_names,
                    &group_samples,
                    &metadata,
                    plot,
                    &args.boxes,
                )?
            }
            PlotTypeArg::PrevalenceAbundance => {
                let sample_groups: Vec<usize> = profile
                    .samples_in_groups()
                    .iter()
                    .enumerate()
                    .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                    .collect();
                prevalence_figure(
                    &profile.profile,
                    &sample_groups,
                    &profile.group_names,
                    &ranked,
                    plot,
                    &args.prevalence,
                )?
            }
            PlotTypeArg::Rarefaction => rarefaction_figure(
                &tree,
                &profile.profile.sample_names,
                levels(&tree, &args.profile).1,
                profile
                    .samples_in_groups()
                    .iter()
                    .enumerate()
                    .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                    .collect(),
                profile.group_names.clone(),
                plot,
            )?,
            PlotTypeArg::Volcano => volcano::plot(
                &volcano::Data::from_multi_group_results(&results),
                &volcano::Options {
                    width: plot.plot_width,
                    height: plot.plot_height.unwrap_or(6.0),
                    corrected: !plot.plot_uncorrected_p_values,
                    ..Default::default()
                },
            ),
            PlotTypeArg::Composition => composition::plot(
                &composition::Data::from_profile(
                    &profile.profile,
                    profile
                        .samples_in_groups()
                        .iter()
                        .enumerate()
                        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                        .collect(),
                    profile.group_names.clone(),
                ),
                &composition::Options {
                    width: plot.plot_width,
                    height: plot.plot_height.unwrap_or(5.0),
                    ..Default::default()
                },
            ),
            kind @ (PlotTypeArg::SharedFeatures
            | PlotTypeArg::RankAbundance
            | PlotTypeArg::Ancom) => {
                let sample_groups: Vec<usize> = profile
                    .samples_in_groups()
                    .iter()
                    .enumerate()
                    .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                    .collect();
                if kind == PlotTypeArg::Ancom {
                    let result = run_ancom(
                        &profile.profile,
                        &sample_groups,
                        stamp_core::stats::multi_group::MultiGroupTest::Anova,
                        0.05,
                    )?;
                    ancom_figure(&profile.profile, &result, &profile.group_names, plot)
                } else {
                    proportion_figure(
                        kind,
                        &composition::Data::from_profile(
                            &profile.profile,
                            sample_groups,
                            profile.group_names.clone(),
                        ),
                        plot,
                    )?
                }
            }
            kind @ (PlotTypeArg::AlphaDiversity
            | PlotTypeArg::Pcoa
            | PlotTypeArg::Nmds
            | PlotTypeArg::DistanceBox) => community_figure(
                kind,
                &tree,
                &profile.profile.sample_names,
                levels(&tree, &args.profile).1,
                profile
                    .samples_in_groups()
                    .iter()
                    .enumerate()
                    .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
                    .collect(),
                profile.group_names.clone(),
                plot,
                COMMUNITY_SEED,
                &args.ordination,
                ordination_styling(
                    &args.ordination,
                    &metadata,
                    &profile.profile,
                    &args.metadata.field,
                )?,
            )?,
            other => return Err(unavailable(other, "several groups")),
        };
        warn_of_repeated_colors(profile.group_names.len());
        write_figure(path, &figure, plot.plot_dpi)?;
    }
    write_streamed(args.profile.output.as_deref(), |out, progress| {
        write_multi_group_table(out, &profile, &results, &rows, progress)
    })
}

fn post_hoc(args: PostHocArgs) -> CliResult {
    check_coverage(args.coverage)?;
    let tree = load_profile(&args.profile.profile)?;
    let profile = multi_group_profile(&tree, &args.profile, &args.metadata, &args.groups)?;

    let mut combined: Option<Table> = None;
    for feature in &args.features {
        let result = core(run_post_hoc(
            &profile,
            feature,
            args.test.into(),
            args.coverage,
        ))?;
        if let (Some(path), true) = (&args.plot.plot, combined.is_none()) {
            match args.plot.plot_type.unwrap_or(PlotTypeArg::PostHoc) {
                PlotTypeArg::PostHoc => {}
                other => return Err(unavailable(other, "the post-hoc command")),
            }
            if args.features.len() > 1 {
                eprintln!("warning: the plot shows the first feature only ({feature})");
            }
            let figure = stamp_plot::post_hoc::plot(
                &profile,
                feature,
                &result,
                args.coverage,
                false,
                &error_bar_options(&args.plot),
            );
            write_figure(path, &figure, args.plot.plot_dpi)?;
        }
        let table = post_hoc_table(feature, &result, args.coverage);
        match &mut combined {
            Some(all) => all.rows.extend(table.rows),
            None => combined = Some(table),
        }
    }
    write_table(
        args.profile.output.as_deref(),
        &combined.expect("at least one feature is required"),
    )
}

/// Correlate every feature with a numeric metadata field.
fn correlation(args: CorrelationArgs) -> CliResult {
    let tree = load_profile(&args.profile.profile)?;
    let metadata = load_metadata(
        &tree,
        &args.metadata,
        &args.excluded_samples,
        &args.sample_filter,
    )?;
    let (parent_level, profile_level) = levels(&tree, &args.profile);
    let profile = core(tree.correlation_profile(
        &metadata,
        &args.field,
        parent_level,
        profile_level,
        args.profile.unclassified.into(),
    ))
    .map_err(|e| {
        format!(
            "{e} Fields in the metadata file: {}.",
            metadata.fields().join(", ")
        )
    })?;
    if profile.values.len() < 3 {
        return Err(format!(
            "a correlation needs at least three samples with a number for '{}'; {} found",
            args.field,
            profile.values.len()
        ));
    }
    if profile.without_value > 0 {
        eprintln!(
            "{} of the samples have no number for '{}' and are left out.",
            profile.without_value, args.field
        );
    }

    let analysis = CorrelationAnalysis {
        method: args.method.into(),
        correction: args.correction.into(),
    };
    let mut rng = rng_from_seed(args.seed);
    let task = Task::start("Testing features");
    let results =
        run_correlation_analysis(&profile, analysis, &mut rng, &mut testing_features(&task))
            .expect("analysis is never cancelled");
    drop(task);
    report_correction(&results.correction);
    let filter = CorrelationFilter {
        selected_features: None,
        significance: args.significance,
        min_coefficient: args.min_correlation,
    };
    let rows = results.filter(&filter);

    if let Some(path) = &args.plot.plot {
        use stamp_plot::{correlation_plot, histogram, volcano};
        let plot = &args.plot;
        let height = plot.plot_height.unwrap_or(6.0);
        let figure = match plot.plot_type.unwrap_or(PlotTypeArg::CorrelationScatter) {
            PlotTypeArg::CorrelationScatter => {
                let ranked: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                let name = bar_feature(plot, &ranked)?;
                let mut data = correlation_plot::Data::from_profile(&profile, &name)
                    .ok_or_else(|| format!("unknown feature '{name}'"))?;
                if let Some(field) = &args.group_field {
                    let groups = groups_of(&metadata, field)?;
                    let names: Vec<String> = groups.names().map(str::to_string).collect();
                    for point in &mut data.points {
                        point.group = names.iter().position(|g| {
                            groups
                                .samples(g)
                                .is_some_and(|members| members.contains(&point.sample))
                        });
                    }
                    data.groups = names;
                }
                data.statistics = results.rows.iter().find(|r| r.feature == name).map(|r| {
                    format!(
                        "{} = {:.3}, {} (n = {})",
                        analysis.method.coefficient_name(),
                        r.coefficient,
                        p_value_text(
                            &stamp_core::stats::results::format_p_value(r.p_value_corrected, -15),
                            Some(PValueKind::of(results.correction.method))
                        ),
                        profile.values.len()
                    )
                });
                correlation_plot::plot(
                    &data,
                    &correlation_plot::Options {
                        width: plot.plot_width,
                        height,
                        ..Default::default()
                    },
                )
            }
            PlotTypeArg::Volcano => volcano::plot(
                &volcano::Data::from_correlation_results(&results, &profile.field),
                &volcano::Options {
                    width: plot.plot_width,
                    height,
                    corrected: !plot.plot_uncorrected_p_values,
                    ..Default::default()
                },
            ),
            PlotTypeArg::PValueHistogram => {
                let uncorrected = plot.plot_uncorrected_p_values;
                let p: Vec<f64> = results
                    .rows
                    .iter()
                    .map(|r| {
                        if uncorrected {
                            r.p_value
                        } else {
                            r.p_value_corrected
                        }
                    })
                    .collect();
                histogram::p_value_histogram(
                    &p,
                    &histogram::PValueOptions {
                        label: PValueKind::shown(results.correction.method, !uncorrected)
                            .label()
                            .to_string(),
                        width: plot.plot_width,
                        height: plot.plot_height.unwrap_or(7.0),
                        ..Default::default()
                    },
                )
            }
            PlotTypeArg::Mantel => {
                use stamp_core::stats::diversity::Normalisation;
                use stamp_plot::community_plots::{self, Community};
                let samples = &profile.profile.sample_names;
                let mut rng = rng_from_seed(args.seed);
                let community = Community::from_tree(
                    &tree,
                    samples,
                    profile_level,
                    vec![0; samples.len()],
                    vec![String::new()],
                    if plot.rarefy {
                        Normalisation::Rarefy
                    } else {
                        Normalisation::Proportions
                    },
                    &mut rng,
                )?;
                let distance = distance_of(plot);
                // The test is run once, for the report and the plot.
                let mut run = community_plots::CommunityRun::new(
                    community,
                    distance,
                    community_plots::CommunityJob::Mantel {
                        values: profile.values.clone(),
                        method: analysis.method,
                        permutations: 999,
                    },
                    rng_from_seed(args.seed),
                );
                let output = run_community(&mut run);
                let community = run.community();
                let options = correlation_plot::Options {
                    width: plot.plot_width,
                    height,
                    unique_shapes: false,
                    legend: stamp_plot::axes::LegendPosition::None,
                    ..Default::default()
                };
                match output {
                    community_plots::CommunityOutput::Mantel {
                        matrix,
                        differences,
                        test,
                    } => {
                        eprintln!(
                            "Mantel test of {} distance against difference in {}: {} = {:.4}, p = {} (999 permutations)",
                            distance.name(),
                            profile.field,
                            analysis.method.coefficient_name(),
                            test.statistic,
                            stamp_core::stats::results::format_p_value(test.p_value, -15)
                        );
                        community_plots::draw_mantel(
                            community,
                            &profile.field,
                            distance,
                            analysis.method,
                            &matrix,
                            &differences,
                            &test,
                            &options,
                        )
                    }
                    community_plots::CommunityOutput::Unavailable(message) => {
                        stamp_plot::Figure::message(&message)
                    }
                    _ => unreachable!("a Mantel test was asked for"),
                }
            }
            PlotTypeArg::Map => {
                let samples = &profile.profile.sample_names;
                let (sample_groups, groups) = match &args.group_field {
                    Some(field) => {
                        let groups = groups_of(&metadata, field)?;
                        let mut names: Vec<String> = groups.names().map(str::to_string).collect();
                        let sample_groups = samples
                            .iter()
                            .map(|sample| {
                                names
                                    .iter()
                                    .position(|g| {
                                        groups
                                            .samples(g)
                                            .is_some_and(|members| members.contains(sample))
                                    })
                                    .unwrap_or(usize::MAX)
                            })
                            .collect::<Vec<_>>();
                        // Samples outside every group are put in one of
                        // their own.
                        let sample_groups = if sample_groups.contains(&usize::MAX) {
                            names.push("No group".to_string());
                            let last = names.len() - 1;
                            sample_groups
                                .into_iter()
                                .map(|g| if g == usize::MAX { last } else { g })
                                .collect()
                        } else {
                            sample_groups
                        };
                        (sample_groups, names)
                    }
                    None => (vec![0; samples.len()], vec!["Samples".to_string()]),
                };
                map_figure(
                    &profile.profile,
                    &sample_groups,
                    groups,
                    &metadata,
                    args.group_field.as_deref(),
                    plot,
                    &args.ordination,
                    &args.map,
                )?
            }
            other => return Err(unavailable(other, "a correlation")),
        };
        write_figure(path, &figure, plot.plot_dpi)?;
    }
    write_streamed(args.profile.output.as_deref(), |out, progress| {
        write_correlation_table(out, &profile, &results, &rows, progress)
    })
}

/// Options of the heatmap of samples from the command line.
/// The one feature of a bar or correlation scatter plot: the one named, or
/// the most significant of `ranked` (features in order of significance).
fn bar_feature(plot: &PlotArgs, ranked: &[String]) -> CliResult<String> {
    match plot.plot_feature.as_slice() {
        [] => ranked.first().cloned().ok_or_else(|| {
            "no feature passes the filters; name one with --plot-feature".to_string()
        }),
        [one] => Ok(one.clone()),
        _ => Err("this plot shows one feature; give --plot-feature once \
             (several are for the box plot)"
            .to_string()),
    }
}

/// The features of a box plot: those named, or the `--box-top` most
/// significant of `ranked`.
fn box_features(plot: &PlotArgs, boxes: &BoxArgs, ranked: &[String]) -> CliResult<Vec<String>> {
    if !plot.plot_feature.is_empty() {
        return Ok(plot.plot_feature.clone());
    }
    if boxes.box_top == 0 {
        return Err("--box-top must be at least 1".to_string());
    }
    if ranked.is_empty() {
        return Err("no feature passes the filters; name one with --plot-feature".to_string());
    }
    Ok(ranked.iter().take(boxes.box_top).cloned().collect())
}

/// The box plot of some features in groups whose samples are
/// `group_samples` (in the order of `group_names`).
fn box_figure(
    data: &[stamp_plot::feature_plots::FeatureData],
    group_names: &[String],
    group_samples: &[Vec<String>],
    metadata: &Metadata,
    plot: &PlotArgs,
    args: &BoxArgs,
) -> CliResult<stamp_plot::Figure> {
    use stamp_plot::feature_plots::{BoxLayout, BoxOptions, GroupOrder};
    let group_order = match (args.box_group_order, &args.box_order_field) {
        (Some(GroupOrderArg::Field), None) => {
            return Err(
                "--box-group-order field needs --box-order-field, the field that orders the \
                 groups"
                    .to_string(),
            )
        }
        (Some(GroupOrderArg::Given), _) => GroupOrder::AsGiven,
        (Some(GroupOrderArg::Alphabetical), _) => GroupOrder::Alphabetical,
        (Some(GroupOrderArg::Median), _) => GroupOrder::Median,
        (Some(GroupOrderArg::Field), Some(_)) | (None, Some(_)) => GroupOrder::Category,
        (None, None) => GroupOrder::AsGiven,
    };
    let categories = match &args.box_order_field {
        Some(field) => {
            if !metadata.fields().iter().any(|f| f == field) {
                return Err(format!(
                    "unknown metadata field '{field}' for --box-order-field. Fields in the \
                     metadata file: {}.",
                    metadata.fields().join(", ")
                ));
            }
            // In the order of the groups of the features.
            let slices: Vec<&[String]> = data
                .first()
                .map(|f| {
                    f.groups
                        .iter()
                        .map(|g| {
                            group_names
                                .iter()
                                .position(|n| *n == g.name)
                                .map_or(&[][..], |i| group_samples[i].as_slice())
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(stamp_core::stats::group_summary::majority_values(
                metadata, &slices, field,
            ))
        }
        None => None,
    };
    let options = BoxOptions {
        width: plot.plot_width,
        height: plot
            .plot_height
            .unwrap_or(if data.len() > 1 { 5.0 } else { 7.0 }),
        log_scale: args.box_log,
        show_points: args.box_points,
        show_counts: args.box_counts,
        horizontal: args.box_horizontal,
        layout: match args.box_layout {
            BoxLayoutArg::Grouped => BoxLayout::Grouped,
            BoxLayoutArg::Panels => BoxLayout::Panels,
        },
        shared_axis: !args.box_independent_axes,
        group_order,
        ..Default::default()
    };
    Ok(stamp_plot::multi_box::plot(
        data,
        categories.as_deref(),
        &options,
    ))
}

/// The prevalence-abundance plot of a profile whose sample `i` is in group
/// `sample_groups[i]`; `filtered` are the features that pass the filters.
/// Also writes `--prevalence-table` if given.
fn prevalence_figure(
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    group_names: &[String],
    filtered: &[String],
    plot: &PlotArgs,
    args: &PrevalenceArgs,
) -> CliResult<stamp_plot::Figure> {
    use stamp_core::stats::prevalence::MeanOver;
    use stamp_plot::prevalence_plot::{Data, Options, View};
    if !(args.prevalence_min_abundance >= 0.0 && args.prevalence_min_abundance <= 100.0) {
        return Err("--prevalence-min-abundance must be a percentage from 0 to 100".to_string());
    }
    if !(args.prevalence_specificity >= 0.0 && args.prevalence_specificity <= 100.0) {
        return Err("--prevalence-specificity must be from 0 to 100 percentage points".to_string());
    }
    if let Some(group) = &args.prevalence_group {
        if !group_names.contains(group) {
            return Err(format!(
                "--prevalence-group '{group}' is not a group of the analysis. Groups: {}.",
                group_names.join(", ")
            ));
        }
    }
    for feature in &args.highlight {
        if profile.feature(feature).is_none() {
            return Err(format!(
                "--highlight: unknown feature '{feature}' at this profile level"
            ));
        }
    }
    let mut data = Data::from_profile(
        profile,
        sample_groups,
        group_names,
        args.prevalence_only_filtered.then_some(filtered),
        args.prevalence_min_abundance,
    );
    data.highlighted = args.highlight.clone();
    if let Some(path) = &args.prevalence_table {
        let table = stamp_core::stats::table::prevalence_table(&data.features, group_names);
        write_table(Some(path), &table)?;
    }
    let options = Options {
        width: plot.plot_width,
        height: plot.plot_height.unwrap_or(7.0),
        view: match args.prevalence_view {
            PrevalenceViewArg::Panels => View::Panels,
            PrevalenceViewArg::Combined => View::Combined,
            PrevalenceViewArg::Specificity => View::Specificity,
        },
        mean_over: match args.prevalence_mean {
            MeanOverArg::All => MeanOver::AllSamples,
            MeanOverArg::Present => MeanOver::PresentSamples,
        },
        group: args.prevalence_group.clone(),
        specificity: args.prevalence_specificity,
        ..Default::default()
    };
    Ok(stamp_plot::prevalence_plot::plot(&data, &options))
}

fn heatmap_options(
    plot: &PlotArgs,
    args: &HeatmapArgs,
    height: f64,
) -> CliResult<stamp_plot::heatmap::Options> {
    use stamp_plot::heatmap::{ColorMap, Options, Order};
    let linkage = args.heatmap_linkage.into();
    let row_order = match args.heatmap_row_order {
        RowOrderArg::Cluster => Order::Cluster(linkage),
        RowOrderArg::MeanAbundance => Order::MeanAbundance,
        RowOrderArg::Alphabetical => Order::Alphabetical,
        RowOrderArg::Original => Order::Original,
    };
    let column_order = match args.heatmap_column_order.unwrap_or(ColumnOrderArg::Cluster) {
        ColumnOrderArg::Cluster => Order::Cluster(linkage),
        ColumnOrderArg::Alphabetical => Order::Alphabetical,
        ColumnOrderArg::Original => Order::Original,
        ColumnOrderArg::Category => {
            return Err(
                "--heatmap-column-order category is for the group heatmap (--plot-type \
                 group-heatmap)"
                    .to_string(),
            )
        }
    };
    Ok(Options {
        width: plot.plot_width,
        height,
        row_order,
        column_order,
        show_row_dendrogram: !args.heatmap_no_dendrograms,
        show_column_dendrogram: !args.heatmap_no_dendrograms,
        color_map: args.heatmap_color_map.map_or(ColorMap::Blues, Into::into),
        ..Default::default()
    })
}

/// The group heatmap of the features `features` of a profile whose sample
/// `i` is in group `sample_groups[i]`; also writes its values to
/// `--heatmap-table` if given.
fn group_heatmap_figure(
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    group_names: &[String],
    features: &[String],
    metadata: &Metadata,
    plot: &PlotArgs,
    args: &HeatmapArgs,
) -> CliResult<stamp_plot::Figure> {
    use stamp_plot::group_heatmap::{ColumnOrder, Data, NumberFormat, Options, Scale, Summary};
    use stamp_plot::heatmap::Order;
    if !(args.heatmap_min_abundance >= 0.0 && args.heatmap_min_abundance <= 100.0) {
        return Err("--heatmap-min-abundance must be a percentage from 0 to 100".to_string());
    }
    if args.heatmap_log_floor.is_nan() || args.heatmap_log_floor <= 0.0 {
        return Err("--heatmap-log-floor must be above zero".to_string());
    }
    if !(args.heatmap_cell_width > 0.0 && args.heatmap_cell_height > 0.0) {
        return Err(
            "--heatmap-cell-width and --heatmap-cell-height must be above zero".to_string(),
        );
    }
    let summary = Summary {
        statistic: args.heatmap_statistic.into(),
        min_abundance: args.heatmap_min_abundance,
        max_features: (args.heatmap_top > 0).then_some(args.heatmap_top),
    };
    let mut data = Data::from_profile(
        profile,
        sample_groups,
        group_names,
        Some(features),
        &summary,
    );
    if let Some(field) = &args.heatmap_order_field {
        if !metadata.fields().iter().any(|f| f == field) {
            return Err(format!(
                "unknown metadata field '{field}' for --heatmap-order-field. Fields in the \
                 metadata file: {}.",
                metadata.fields().join(", ")
            ));
        }
        let members = |group: &str| -> Vec<String> {
            profile
                .sample_names
                .iter()
                .zip(sample_groups)
                .filter(|(_, &g)| group_names.get(g).is_some_and(|name| name == group))
                .map(|(sample, _)| sample.clone())
                .collect()
        };
        data.set_categories(metadata, field, &members);
    }
    let linkage = args.heatmap_linkage.into();
    let column_order = match args.heatmap_column_order {
        Some(ColumnOrderArg::Cluster) => ColumnOrder::Cluster(linkage),
        Some(ColumnOrderArg::Alphabetical) => ColumnOrder::Alphabetical,
        Some(ColumnOrderArg::Original) => ColumnOrder::Original,
        Some(ColumnOrderArg::Category) if args.heatmap_order_field.is_none() => {
            return Err(
                "--heatmap-column-order category needs --heatmap-order-field, the field \
                 that orders the groups"
                    .to_string(),
            )
        }
        Some(ColumnOrderArg::Category) => ColumnOrder::Category,
        None if args.heatmap_order_field.is_some() => ColumnOrder::Category,
        None => ColumnOrder::Cluster(linkage),
    };
    let options = Options {
        width: plot.plot_width,
        height: plot.plot_height.unwrap_or(6.0),
        fit_to_cells: !args.heatmap_fixed_size,
        cell_width: args.heatmap_cell_width,
        cell_height: args.heatmap_cell_height,
        scale: match args.heatmap_scale {
            ScaleArg::Linear => Scale::Linear,
            ScaleArg::Log10 => Scale::Log10,
        },
        log_floor: args.heatmap_log_floor,
        color_map: args
            .heatmap_color_map
            .map_or(Options::default().color_map, Into::into),
        grey_zeros: !args.heatmap_no_grey_zeros,
        show_values: args.heatmap_values,
        number_format: match args.heatmap_value_format {
            NumberFormatArg::Decimals => NumberFormat::Decimals,
            NumberFormatArg::Significant => NumberFormat::Significant,
        },
        digits: args.heatmap_value_digits,
        row_order: match args.heatmap_row_order {
            RowOrderArg::Cluster => Order::Cluster(linkage),
            RowOrderArg::MeanAbundance => Order::MeanAbundance,
            RowOrderArg::Alphabetical => Order::Alphabetical,
            RowOrderArg::Original => Order::Original,
        },
        column_order,
        show_row_dendrogram: !args.heatmap_no_dendrograms,
        show_column_dendrogram: !args.heatmap_no_dendrograms,
        ..Default::default()
    };
    if let Some(path) = &args.heatmap_table {
        let table = stamp_core::stats::table::group_summary_table(
            summary.statistic.name(),
            &data.features,
            &data.groups,
            &data.values,
        );
        write_table(Some(path), &table)?;
    }
    Ok(stamp_plot::group_heatmap::plot(&data, &options))
}

/// Correlations between features across samples.
fn co_occurrence(args: CoOccurrenceArgs) -> CliResult {
    use stamp_plot::co_occurrence_plot::{self, Settings, View};
    use stamp_plot::heatmap::Order;
    if let Some(kind) = args.plot.plot_type {
        return Err(unavailable(kind, "co-occurrence"));
    }
    if !(args.q_threshold > 0.0 && args.q_threshold <= 1.0) {
        return Err("--q-threshold must lie between 0 and 1".to_string());
    }
    if args.cell_size.is_nan() || args.cell_size <= 0.0 {
        return Err("--cell-size must be above zero".to_string());
    }
    if !(0.0..=100.0).contains(&args.min_feature_prevalence) {
        return Err("--min-feature-prevalence must be a percentage from 0 to 100".to_string());
    }
    let tree = load_profile(&args.profile.profile)?;
    let (parent_level, profile_level) = levels(&tree, &args.profile);
    // The samples: those the metadata keeps (and of one group), or all.
    let samples: Vec<String> = match &args.metadata {
        Some(path) => {
            let metadata = load_metadata(&tree, path, &args.excluded_samples, &args.sample_filter)?;
            match (&args.field, &args.group) {
                (Some(field), Some(group)) => {
                    let groups = groups_of(&metadata, field)?;
                    groups
                        .samples(group)
                        .ok_or_else(|| {
                            with_group_names(format!("Unknown group '{group}'."), &groups)
                        })?
                        .to_vec()
                }
                (None, Some(_)) => return Err("--group needs --field".to_string()),
                _ => metadata
                    .active_samples()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            }
        }
        None => {
            if args.group.is_some()
                || !args.excluded_samples.is_empty()
                || !args.sample_filter.where_.is_empty()
                || !args.sample_filter.where_not.is_empty()
            {
                return Err(
                    "--group, --exclude-sample, --where and --where-not need --metadata"
                        .to_string(),
                );
            }
            tree.sample_names().to_vec()
        }
    };
    let names: Vec<&str> = samples.iter().map(String::as_str).collect();
    let profile = core(tree.profile(
        &names,
        parent_level,
        profile_level,
        args.profile.unclassified.into(),
    ))?;
    let settings = Settings {
        method: args.method.into(),
        clr: args.clr,
        ranking: args.rank_by.into(),
        top: args.top,
        forced: args.features.clone(),
        view: if args.with.is_some() {
            View::OneFeature
        } else {
            View::Matrix
        },
        partner_count: args.partners,
        mark_significant: !args.no_marks,
        q_threshold: args.q_threshold,
        mark_min_coefficient: args.mark_min_coefficient,
        min_prevalence: args.min_feature_prevalence,
    };
    let options = co_occurrence_plot::Options {
        width: args.plot.plot_width,
        height: args.plot.plot_height.unwrap_or(7.0),
        fit_to_cells: !args.fixed_size,
        cell_size: args.cell_size,
        order: match args.order {
            CoOrderArg::Cluster => Order::Cluster(stamp_core::stats::cluster::Linkage::Average),
            CoOrderArg::Original => Order::Original,
        },
        show_dendrogram: !args.no_dendrograms,
        color_map: args.color_map.into(),
        show_values: args.show_values,
        ..Default::default()
    };
    eprintln!(
        "Correlating features in {} samples at the {profile_level} level.",
        profile.sample_names.len()
    );
    let (table, figure) = match &args.with {
        Some(feature) => {
            let task = Task::start("Correlating features");
            let found = co_occurrence_plot::compute_partners(&profile, None, feature, &settings)?;
            drop(task);
            let figure = co_occurrence_plot::partner_plot(
                feature,
                &found,
                profile.sample_names.len(),
                settings.clr,
                &settings,
                &options,
            );
            (
                stamp_core::stats::table::partner_table(feature, settings.method, &found),
                figure,
            )
        }
        None => {
            let task = Task::start("Correlating features");
            let data = co_occurrence_plot::Data::compute(&profile, None, &settings)?;
            drop(task);
            let figure = co_occurrence_plot::matrix_plot(&data, &settings, &options);
            (
                stamp_core::stats::table::co_occurrence_table(&data.result),
                figure,
            )
        }
    };
    if let Some(path) = &args.plot.plot {
        write_figure(path, &figure, args.plot.plot_dpi)?;
    }
    write_table(args.profile.output.as_deref(), &table)
}

fn unavailable(kind: PlotTypeArg, context: &str) -> String {
    let name = clap::ValueEnum::to_possible_value(&kind)
        .map(|v| v.get_name().to_string())
        .unwrap_or_default();
    format!("the {name} plot is not available for {context}")
}

/// Seed of the community tests where the command has no seed option.
const COMMUNITY_SEED: u64 = 1;

/// One of the plots of whole communities. The outcome of the community
/// tests is also reported for the PCoA plot.
#[allow(clippy::too_many_arguments)]
fn community_figure(
    kind: PlotTypeArg,
    tree: &ProfileTree,
    samples: &[String],
    profile_level: &str,
    sample_groups: Vec<usize>,
    groups: Vec<String>,
    plot: &PlotArgs,
    seed: u64,
    ordination: &OrdinationArgs,
    styling: stamp_plot::ordination_style::Styling,
) -> CliResult<stamp_plot::Figure> {
    use stamp_core::stats::diversity::{AlphaMetric, Normalisation};
    use stamp_plot::community_plots::{self, Community};
    use stamp_plot::feature_plots;

    let mut rng = rng_from_seed(seed);
    let mut community = Community::from_tree(
        tree,
        samples,
        profile_level,
        sample_groups,
        groups,
        if plot.rarefy {
            Normalisation::Rarefy
        } else {
            Normalisation::Proportions
        },
        &mut rng,
    )?;
    community.styling = styling;
    let ordination_options = ordination_options(plot, ordination)?;
    let distance = distance_of(plot);
    let boxes = feature_plots::BoxOptions {
        width: plot.plot_width,
        height: plot.plot_height.unwrap_or(6.0),
        ..Default::default()
    };
    Ok(match kind {
        PlotTypeArg::AlphaDiversity => community_plots::alpha_plot(
            &community,
            match plot.alpha_metric {
                AlphaMetricArg::Observed => AlphaMetric::Observed,
                AlphaMetricArg::Shannon => AlphaMetric::Shannon,
                AlphaMetricArg::Simpson => AlphaMetric::Simpson,
                AlphaMetricArg::Chao1 => AlphaMetric::Chao1,
            },
            &boxes,
        ),
        kind => {
            use community_plots::{CommunityJob, CommunityOutput, CommunityRun};
            let job = match kind {
                PlotTypeArg::Pcoa => CommunityJob::Pcoa { tests: Some(999) },
                PlotTypeArg::Nmds => CommunityJob::Nmds,
                _ => CommunityJob::Distances,
            };
            // The tests are run once, for the report and the plot.
            let mut run = CommunityRun::new(community, distance, job, rng_from_seed(seed));
            let output = run_community(&mut run);
            let community = run.community();
            match output {
                CommunityOutput::Layout(Ok(layout)) => {
                    for line in layout.tests.iter().flatten() {
                        eprintln!("{line}");
                    }
                    if kind == PlotTypeArg::Pcoa {
                        community_plots::draw_pcoa(community, &layout, &ordination_options)
                    } else {
                        community_plots::draw_nmds(community, &layout, &ordination_options)
                    }
                }
                CommunityOutput::Distances(matrix) => {
                    community_plots::draw_distance_box(community, distance, &matrix, &boxes)
                }
                CommunityOutput::Layout(Err(message)) | CommunityOutput::Unavailable(message) => {
                    stamp_plot::Figure::message(&message)
                }
                CommunityOutput::Mantel { .. } => unreachable!("no Mantel test was asked for"),
            }
        }
    })
}

/// The map of where the samples of `profile` were taken, `sample_groups[i]`
/// being the group (in `groups`) of its sample `i`; `group_field` names the
/// groups.
#[allow(clippy::too_many_arguments)]
fn map_figure(
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    groups: Vec<String>,
    metadata: &Metadata,
    group_field: Option<&str>,
    plot: &PlotArgs,
    ordination: &OrdinationArgs,
    args: &MapArgs,
) -> CliResult<stamp_plot::Figure> {
    use stamp_plot::geo::{Projection, Resolution};
    use stamp_plot::map_plot::{self, Extent, SiteValue, SizeSource};
    let (lat, lon) = map_plot::coordinate_fields(metadata);
    let fields = || metadata.fields().join(", ");
    let lat = args.map_lat_field.clone().or(lat).ok_or_else(|| {
        format!(
            "no latitude field found in the metadata; name it with --map-lat-field. Fields: {}.",
            fields()
        )
    })?;
    let lon = args.map_lon_field.clone().or(lon).ok_or_else(|| {
        format!(
            "no longitude field found in the metadata; name it with --map-lon-field. Fields: {}.",
            fields()
        )
    })?;
    let mut data = map_plot::Data::new(
        &profile.sample_names,
        sample_groups,
        groups,
        metadata,
        &lat,
        &lon,
    )
    .map_err(|e| format!("{e}. Fields: {}.", fields()))?;
    data.styling = ordination_styling(
        ordination,
        metadata,
        profile,
        group_field.unwrap_or("Group"),
    )?;
    if group_field.is_none() {
        data.styling.group_label = Some("Samples".to_string());
    }
    let size_by = SizeSource::parse(&args.map_size_by).map_err(|e| format!("--map-size-by {e}"))?;
    data.sizes = size_by
        .resolve(Some(metadata), Some(profile), None)
        .map_err(|e| format!("--map-size-by: {e}"))?;
    let located = data.samples.iter().filter(|s| s.location.is_some()).count();
    if located < data.samples.len() {
        eprintln!(
            "{} of {} samples have no latitude and longitude in '{lat}' and '{lon}' and are not on the map.",
            data.samples.len() - located,
            data.samples.len()
        );
    }
    let style = ordination_options(plot, ordination)?;
    if args
        .map_site_precision
        .is_some_and(|p| p.is_nan() || p <= 0.0)
    {
        return Err("--map-site-precision must be above 0".into());
    }
    if args.map_max_marker_size.is_nan() || args.map_max_marker_size <= 0.0 {
        return Err("--map-max-marker-size must be above 0".into());
    }
    let options = map_plot::Options {
        width: plot.plot_width,
        projection: match args.map_projection {
            MapProjectionArg::EqualEarth => Projection::EqualEarth,
            MapProjectionArg::Robinson => Projection::Robinson,
            MapProjectionArg::Equirectangular => Projection::Equirectangular,
            MapProjectionArg::Mercator => Projection::Mercator,
        },
        extent: Extent::parse(&args.map_extent).map_err(|e| format!("--map-extent {e}"))?,
        resolution: match args.map_resolution {
            MapResolutionArg::Auto => Resolution::Auto,
            MapResolutionArg::Low => Resolution::Low,
            MapResolutionArg::Medium => Resolution::Medium,
        },
        aggregate: !args.no_map_aggregate,
        site_precision: args.map_site_precision.unwrap_or(0.0),
        jitter: args.map_jitter,
        size_by,
        marker_size: style.marker_size,
        max_marker_size: args.map_max_marker_size,
        size_log: args.map_size_log,
        site_value: match args.map_site_value {
            MapSiteValueArg::Mean => SiteValue::Mean,
            MapSiteValueArg::Max => SiteValue::Maximum,
        },
        color_by: style.color_by,
        shape_by: style.shape_by,
        color_map: style.color_map,
        color_scale: style.color_scale,
        draw_order: style.draw_order,
        opacity: style.opacity,
        ocean: !args.no_map_ocean,
        graticule: !args.no_map_graticule,
        borders: !args.no_map_borders,
        ..Default::default()
    };
    Ok(map_plot::plot(&data, &options))
}

/// The appearance of the PCoA and NMDS plots.
fn ordination_options(
    plot: &PlotArgs,
    args: &OrdinationArgs,
) -> CliResult<stamp_plot::ordination_plot::Options> {
    use stamp_plot::ordination_style::{ColorScale, ColorSource, DrawOrder};
    if !(args.opacity > 0.0 && args.opacity <= 1.0) {
        return Err("--opacity must be above 0 and at most 1".into());
    }
    if args.marker_size.is_nan() || args.marker_size <= 0.0 {
        return Err("--marker-size must be above 0".into());
    }
    Ok(stamp_plot::ordination_plot::Options {
        width: plot.plot_width,
        height: plot.plot_height.unwrap_or(6.0),
        // A legend of the groups, placed where it is clearest, as in the
        // graphical application; beside the plot when the samples are
        // coloured or shaped by anything else.
        legend: stamp_plot::axes::LegendPosition::Best,
        marker_size: args.marker_size,
        opacity: args.opacity,
        color_by: ColorSource::parse(&args.color_by).map_err(|e| format!("--color-by {e}"))?,
        shape_by: args.shape_by.clone(),
        color_map: args.color_map.into(),
        color_scale: match args.color_scale {
            ColorScaleArg::Linear => ColorScale::Linear,
            ColorScaleArg::Log10 => ColorScale::Log10,
        },
        draw_order: match args.draw_order {
            DrawOrderArg::HighOnTop => DrawOrder::HighOnTop,
            DrawOrderArg::LowOnTop => DrawOrder::LowOnTop,
            DrawOrderArg::SampleOrder => DrawOrder::AsGiven,
        },
        ..Default::default()
    })
}

/// What colours and shapes the samples of the PCoA and NMDS plots, from
/// the metadata and `profile` (for a feature's relative abundance).
fn ordination_styling(
    args: &OrdinationArgs,
    metadata: &Metadata,
    profile: &stamp_core::Profile,
    group_field: &str,
) -> CliResult<stamp_plot::ordination_style::Styling> {
    use stamp_plot::ordination_style::{resolve, ColorSource};
    let color = ColorSource::parse(&args.color_by).map_err(|e| format!("--color-by {e}"))?;
    resolve(
        &color,
        args.shape_by.as_deref(),
        Some(metadata),
        Some(profile),
        None,
        Some(group_field),
    )
}

/// Warn when there are more groups than colours, so that some share one.
fn warn_of_repeated_colors(groups: usize) {
    if let Some(note) = stamp_plot::axes::repeated_colors_note(groups) {
        eprintln!("Warning: {note}");
    }
}

/// The distance chosen for the plots of whole communities.
fn distance_of(plot: &PlotArgs) -> stamp_core::stats::diversity::Distance {
    use stamp_core::stats::diversity::Distance;
    match plot.distance {
        DistanceArg::BrayCurtis => Distance::BrayCurtis,
        DistanceArg::Jaccard => Distance::Jaccard,
        DistanceArg::Aitchison => Distance::Aitchison,
        DistanceArg::Euclidean => Distance::Euclidean,
    }
}

/// The plots drawn from the proportions of every feature: shared features
/// and rank-abundance curves.
fn proportion_figure(
    kind: PlotTypeArg,
    data: &stamp_plot::composition::Data,
    plot: &PlotArgs,
) -> CliResult<stamp_plot::Figure> {
    use stamp_plot::{rank_abundance, shared_features};
    if !(0.0..=100.0).contains(&plot.min_prevalence) {
        return Err("--min-prevalence must be a percentage, from 0 to 100".to_string());
    }
    Ok(match kind {
        PlotTypeArg::SharedFeatures => shared_features::plot(
            &shared_features::Data::from_composition(data, plot.min_prevalence / 100.0),
            &shared_features::Options {
                width: plot.plot_width,
                height: plot.plot_height.unwrap_or(5.0),
                style: match plot.shared_style {
                    SharedStyleArg::Upset => shared_features::Style::UpSet,
                    SharedStyleArg::Venn => shared_features::Style::Venn,
                },
                min_prevalence: plot.min_prevalence,
                ..Default::default()
            },
        ),
        _ => rank_abundance::plot(
            data,
            &rank_abundance::Options {
                width: plot.plot_width,
                height: plot.plot_height.unwrap_or(5.0),
                curves: match plot.rank_curves {
                    RankCurvesArg::Samples => rank_abundance::Curves::Samples,
                    RankCurvesArg::Groups => rank_abundance::Curves::Groups,
                },
                ..Default::default()
            },
        ),
    })
}

/// ANCOM on a profile whose sample `i` is in group `sample_groups[i]`.
fn run_ancom(
    profile: &stamp_core::Profile,
    sample_groups: &[usize],
    test: stamp_core::stats::multi_group::MultiGroupTest,
    alpha: f64,
) -> CliResult<stamp_core::stats::ancom::AncomResult> {
    if !(alpha > 0.0 && alpha < 1.0) {
        return Err("--alpha must lie between 0 and 1".to_string());
    }
    let features = profile.features();
    let samples: Vec<Vec<f64>> = (0..profile.sample_names.len())
        .map(|s| features.iter().map(|f| f.counts[s]).collect())
        .collect();
    let mut run = stamp_core::stats::ancom::AncomRun::new(&samples, sample_groups, test, alpha)
        .ok_or_else(|| {
            "ANCOM needs two groups with samples, a group of more than one sample, and two features"
                .to_string()
        })?;
    let task = Task::start("ANCOM");
    match run.advance(&mut |done, total| {
        task.set("ANCOM: log ratios of feature", done, total);
        stamp_core::stats::results::Control::Continue
    }) {
        stamp_core::stats::results::Step::Done(result) => Ok(result),
        _ => unreachable!("never paused or cancelled"),
    }
}

fn ancom_figure(
    profile: &stamp_core::Profile,
    result: &stamp_core::stats::ancom::AncomResult,
    groups: &[String],
    plot: &PlotArgs,
) -> stamp_plot::Figure {
    let names: Vec<String> = profile.features().iter().map(|f| f.name.clone()).collect();
    stamp_plot::ancom_plot::plot(
        result,
        &names,
        groups,
        &stamp_plot::ancom_plot::Options {
            width: plot.plot_width,
            height: plot.plot_height.unwrap_or(6.0),
            ..Default::default()
        },
    )
}

/// ANCOM between groups, written as a table.
fn ancom(args: AncomArgs) -> CliResult {
    let tree = load_profile(&args.profile.profile)?;
    let profile = multi_group_profile(&tree, &args.profile, &args.metadata, &args.groups)?;
    let sample_groups: Vec<usize> = profile
        .samples_in_groups()
        .iter()
        .enumerate()
        .flat_map(|(group, members)| std::iter::repeat_n(group, members.len()))
        .collect();
    let result = run_ancom(
        &profile.profile,
        &sample_groups,
        args.test.into(),
        args.alpha,
    )?;
    eprintln!(
        "ANCOM: {} of {} features differ between the groups.",
        result.rejected(),
        result.rows.len()
    );
    if let Some(path) = &args.plot.plot {
        let figure = match args.plot.plot_type.unwrap_or(PlotTypeArg::Ancom) {
            PlotTypeArg::Ancom => {
                ancom_figure(&profile.profile, &result, &profile.group_names, &args.plot)
            }
            other => return Err(unavailable(other, "the ancom command")),
        };
        write_figure(path, &figure, args.plot.plot_dpi)?;
    }
    let table = ancom_table(&profile.profile, &result, profile.group_names.len());
    write_table(args.profile.output.as_deref(), &table)
}

/// Rarefaction curves of some samples of a profile, coloured by group.
fn rarefaction_figure(
    tree: &ProfileTree,
    samples: &[String],
    profile_level: &str,
    sample_groups: Vec<usize>,
    groups: Vec<String>,
    plot: &PlotArgs,
) -> CliResult<stamp_plot::Figure> {
    use stamp_plot::rarefaction;
    let data = rarefaction::Data::from_tree(tree, samples, profile_level, sample_groups, groups)?;
    Ok(rarefaction::plot(
        &data,
        &rarefaction::Options {
            curves: match plot.rarefaction_curves {
                RarefactionCurvesArg::Samples => rarefaction::Curves::Samples,
                RarefactionCurvesArg::Pooled => rarefaction::Curves::GroupsPooled,
                RarefactionCurvesArg::Accumulated => rarefaction::Curves::GroupsAccumulated,
            },
            width: plot.plot_width,
            height: plot.plot_height.unwrap_or(5.0),
            ..Default::default()
        },
    ))
}

/// Options of the extended error bar (and post-hoc) plot from the command line.
fn error_bar_options(args: &PlotArgs) -> extended_error_bar::Options {
    extended_error_bar::Options {
        width: args.plot_width,
        row_height: args.plot_row_height,
        sort_by: match args.plot_sort_by {
            PlotSortArg::PValues => SortBy::PValues,
            PlotSortArg::EffectSizes => SortBy::EffectSizes,
            PlotSortArg::FeatureLabels => SortBy::FeatureLabels,
        },
        legend: match args.plot_legend {
            PlotLegendArg::None => Legend::None,
            PlotLegendArg::UpperLeft => Legend::UpperLeft,
            PlotLegendArg::UpperCenter => Legend::UpperCenter,
            PlotLegendArg::UpperRight => Legend::UpperRight,
            PlotLegendArg::LowerLeft => Legend::LowerLeft,
            PlotLegendArg::LowerCenter => Legend::LowerCenter,
            PlotLegendArg::LowerRight => Legend::LowerRight,
        },
        show_corrected_p_values: !args.plot_uncorrected_p_values,
        ..Default::default()
    }
}

/// Save a figure in the format of the file's extension.
fn write_figure(path: &Path, figure: &stamp_plot::Figure, dpi: f64) -> CliResult {
    let format = Format::from_file_name(&path.to_string_lossy()).ok_or_else(|| {
        format!(
            "cannot tell the image format of {}: use a .png, .svg or .pdf extension",
            path.display()
        )
    })?;
    if figure.regions.is_empty() && figure.shapes.len() == 1 {
        // A figure holding only a message: nothing could be plotted.
        if let stamp_plot::Shape::Text { text, .. } = &figure.shapes[0] {
            eprintln!("warning: the plot is empty: {text}");
        }
    }
    let _task = Task::start(&format!("Writing {}", path.display()));
    let bytes = export(figure, format, dpi).map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn write_table(path: Option<&Path>, table: &Table) -> CliResult {
    write_output(path, &table.to_tsv())
}

/// Write a table row by row to a file, or else to standard output, as
/// `write` produces it, showing its progress: tables with a column for every
/// sample are too large to build whole in memory first.
fn write_streamed(
    path: Option<&Path>,
    write: impl FnOnce(&mut dyn Write, &mut dyn FnMut(usize, usize)) -> std::io::Result<()>,
) -> CliResult {
    let task = Task::start("Writing the results table");
    let mut progress = |done, total| task.count(done, total);
    match path {
        Some(path) => {
            let failed = |e: std::io::Error| format!("cannot write {}: {e}", path.display());
            let file = fs::File::create(path).map_err(failed)?;
            let mut out = std::io::BufWriter::new(file);
            write(&mut out, &mut progress)
                .and_then(|()| out.flush())
                .map_err(failed)
        }
        None => {
            let mut out = std::io::BufWriter::new(std::io::stdout().lock());
            // A closed pipe (for example `| head`) is not an error.
            let _ = write(&mut out, &mut progress).and_then(|()| out.flush());
            Ok(())
        }
    }
}

/// File name of a path, for converters that name samples after their files.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn convert(format: ConvertFormat) -> CliResult {
    let task = Task::start("Converting");
    let (spf, output) = match format {
        ConvertFormat::Mgrast {
            input,
            headings,
            output,
        } => {
            let mut profile = core(mgrast::MgRastProfile::parse(&read_text(&input)?))?;
            if !headings.is_empty() {
                if headings.len() != profile.headings.len() {
                    return Err(format!(
                        "this MG-RAST profile has {} hierarchical levels ({}), but {} --heading options were given",
                        profile.headings.len(),
                        profile.headings.join(", "),
                        headings.len()
                    ));
                }
                profile.headings = headings;
            }
            (core(profile.to_spf())?, output)
        }
        ConvertFormat::Comet { inputs, output } => {
            let (names, texts) = read_named(&inputs)?;
            (core(comet::to_spf(&named(&names, &texts)))?, output)
        }
        ConvertFormat::Rita {
            inputs,
            groups,
            output,
        } => {
            let (names, texts) = read_named(&inputs)?;
            let groups: Vec<&str> = if groups.is_empty() {
                rita::GROUPS.to_vec()
            } else {
                for group in &groups {
                    if !rita::GROUPS.contains(&group.as_str()) {
                        return Err(format!(
                            "unknown RITA group '{group}' (known groups: {})",
                            rita::GROUPS.join(", ")
                        ));
                    }
                }
                groups.iter().map(String::as_str).collect()
            };
            (core(rita::to_spf(&named(&names, &texts), &groups))?, output)
        }
        ConvertFormat::Mothur {
            taxonomy,
            groups,
            names,
            output,
        } => {
            let names = names.as_deref().map(read_text).transpose()?;
            let spf = mothur::to_spf(
                &read_text(&taxonomy)?,
                &read_text(&groups)?,
                names.as_deref(),
            );
            (core(spf)?, output)
        }
        ConvertFormat::Biom {
            input,
            metadata_field,
            output,
        } => (
            core(biom::to_spf(
                &read_bytes(&input)?,
                metadata_field.as_deref(),
            ))?,
            output,
        ),
        ConvertFormat::Singlem { inputs, output } => {
            let (names, texts) = read_named(&inputs)?;
            (core(singlem::to_spf(&named(&names, &texts)))?, output)
        }
        ConvertFormat::Sandpiper { inputs, output } => {
            let (names, texts) = read_named(&inputs)?;
            (core(sandpiper::to_spf(&named(&names, &texts)))?, output)
        }
        ConvertFormat::Cog {
            input,
            assign_to_each_code,
            output,
        } => {
            let treatment = if assign_to_each_code {
                cog::MultiCodeTreatment::AssignToEachCode
            } else {
                cog::MultiCodeTreatment::AsFeature
            };
            (
                core(cog::append_categories(&read_text(&input)?, treatment))?,
                output,
            )
        }
    };

    // Tell the user now if STAMP will not be able to load the result.
    task.set("Checking the profile", 0, 0);
    let problem = ProfileTree::read(&spf).err();
    drop(task);
    if let Some(problem) = problem {
        eprintln!("warning: the converted profile cannot be loaded by STAMP as it is: {problem}");
    }
    write_output(output.output.as_deref(), &spf)
}

fn read_named(paths: &[std::path::PathBuf]) -> CliResult<(Vec<String>, Vec<String>)> {
    let _task = Task::start(&format!("Reading {} files", paths.len()));
    let names = paths.iter().map(|p| file_name(p)).collect();
    let texts = paths
        .iter()
        .map(|p| read_text(p))
        .collect::<CliResult<Vec<String>>>()?;
    Ok((names, texts))
}

fn named<'a>(names: &'a [String], texts: &'a [String]) -> Vec<NamedText<'a>> {
    names
        .iter()
        .zip(texts)
        .map(|(name, text)| NamedText { name, text })
        .collect()
}
