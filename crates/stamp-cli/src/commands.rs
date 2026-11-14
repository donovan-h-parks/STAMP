//! Implementation of the commands.

use std::fs;
use std::io::Write;
use std::path::Path;

use stamp_core::import::{biom, cog, comet, mgrast, mothur, rita, NamedText};
use stamp_core::stats::correction::Correction;
use stamp_core::stats::multi_group::MultiGroupEffectSize;
use stamp_core::stats::results::{
    run_correlation_analysis, run_group_analysis, run_multi_group_analysis, run_post_hoc,
    run_sample_analysis, CorrectionSummary, CorrelationAnalysis, CorrelationFilter, GroupAnalysis,
    GroupCountFilter, GroupFilter, MultiGroupAnalysis, MultiGroupFilter, SampleAnalysis,
    SampleCountFilter, SampleFilter,
};
use stamp_core::stats::table::{
    correlation_table, group_table, multi_group_table, post_hoc_table, sample_table, Table,
};
use stamp_core::stats::{rng_from_seed, StatsSettings};
use stamp_core::{Groups, Metadata, MultiGroupProfile, ProfileTree, SecondGroup};

use stamp_plot::export::{export, Format};
use stamp_plot::extended_error_bar::{self, Legend, SortBy};

use crate::args::*;

type CliResult<T = ()> = Result<T, String>;

pub fn run(cli: Cli) -> CliResult {
    match cli.command {
        Command::Info(args) => info(args),
        Command::TwoSamples(args) => two_samples(args),
        Command::TwoGroups(args) => two_groups(args),
        Command::MultiGroups(args) => multi_groups(args),
        Command::PostHoc(args) => post_hoc(args),
        Command::Correlation(args) => correlation(args),
        Command::Convert(args) => convert(args.format),
    }
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
    core(ProfileTree::read(&read_text(path)?)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Read the metadata, report its warnings, and leave out the samples that
/// were excluded.
fn load_metadata(tree: &ProfileTree, path: &Path, excluded: &[String]) -> CliResult<Metadata> {
    let (mut metadata, warnings) = core(Metadata::read(&read_text(path)?, tree))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
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

/// Group the samples by a metadata field.
fn groups_of(metadata: &Metadata, field: &str) -> CliResult<Groups> {
    core(metadata.groups(field)).map_err(|e| {
        format!(
            "{e} Fields in the metadata file: {}.",
            metadata.fields().join(", ")
        )
    })
}

/// Read the metadata, report its warnings, and group the samples.
fn load_groups(tree: &ProfileTree, args: &MetadataArgs) -> CliResult<Groups> {
    let metadata = load_metadata(tree, &args.metadata, &args.excluded_samples)?;
    groups_of(&metadata, &args.field)
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
        let (metadata, warnings) = core(Metadata::read(&read_text(path)?, &tree))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        for warning in warnings {
            eprintln!("warning: {warning}");
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
    let results = run_sample_analysis(&profile, analysis, &settings, &mut rng, &mut |_, _| true)
        .expect("analysis is never cancelled");
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
                let p: Vec<f64> = results.rows.iter().map(|r| r.p_value_corrected).collect();
                histogram::p_value_histogram(
                    &p,
                    &histogram::PValueOptions {
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
    let results = run_group_analysis(&profile, analysis, &settings, &mut rng, &mut |_, _| true)
        .expect("analysis is never cancelled");
    report_correction(&results.correction);

    let rows = results.filter(&profile, &filter, &settings);
    if let Some(path) = &args.plot.plot {
        use stamp_plot::{composition, feature_plots, heatmap, pca_plot, scatter, volcano};
        let plot = &args.plot;
        let kind = plot.plot_type.unwrap_or(PlotTypeArg::ExtendedErrorBar);
        let size = |default: f64| plot.plot_height.unwrap_or(default);
        // The bar and box plots show one feature, with its corrected p-value.
        let feature_data = || -> CliResult<feature_plots::FeatureData> {
            let name = plot
                .plot_feature
                .clone()
                .or_else(|| rows.first().map(|r| r.feature.clone()))
                .ok_or("no feature passes the filters; name one with --plot-feature")?;
            let mut data = feature_plots::FeatureData::from_group_profile(
                &profile,
                &name,
                feature_plots::Field::Proportion,
            )
            .ok_or_else(|| format!("unknown feature '{name}'"))?;
            data.p_value = results
                .rows
                .iter()
                .find(|r| r.feature == name)
                .map(|r| stamp_core::stats::results::format_p_value(r.p_value_corrected, -15));
            Ok(data)
        };
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
                &feature_data()?,
                &feature_plots::BarOptions {
                    height: size(6.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Box => feature_plots::box_plot(
                &feature_data()?,
                &feature_plots::BoxOptions {
                    width: plot.plot_width,
                    height: size(7.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Scatter => scatter::plot(
                &scatter::Data::from_group_profile(&profile, scatter::Spread::Percentiles2And98),
                &scatter::Options {
                    width: plot.plot_width,
                    height: size(7.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Pca => pca_plot::plot(
                &pca_plot::Data::from_group_profile(&profile),
                &pca_plot::Options {
                    width: plot.plot_width,
                    height: size(6.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Heatmap => {
                let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                heatmap::plot(
                    &heatmap::Data::from_group_profile(&profile, Some(&names)),
                    &heatmap::Options {
                        width: plot.plot_width,
                        height: size(7.0),
                        ..Default::default()
                    },
                )
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
            PlotTypeArg::AlphaDiversity | PlotTypeArg::Pcoa | PlotTypeArg::DistanceBox => {
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
                )?
            }
            other => return Err(unavailable(other, "two groups")),
        };
        write_figure(path, &figure, plot.plot_dpi)?;
    }
    let table = group_table(&profile, &results, rows);
    write_table(args.profile.output.as_deref(), &table)
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
    let groups = load_groups(tree, metadata_args)?;
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
    core(tree.multi_group_profile(
        &groups,
        Some(&active),
        parent_level,
        profile_level,
        profile_args.unclassified.into(),
    ))
    .map_err(|e| with_group_names(e, &groups))
}

fn multi_groups(args: MultiGroupsArgs) -> CliResult {
    let tree = load_profile(&args.profile.profile)?;
    let profile = multi_group_profile(&tree, &args.profile, &args.metadata, &args.groups)?;

    let analysis = MultiGroupAnalysis {
        test: args.test.into(),
        effect_size: MultiGroupEffectSize::EtaSquared,
        correction: args.correction.into(),
        transform: args.transform.into(),
    };
    let mut rng = rng_from_seed(args.seed);
    let results = run_multi_group_analysis(&profile, analysis, &mut rng, &mut |_, _| true)
        .expect("analysis is never cancelled");
    report_correction(&results.correction);

    let filter = MultiGroupFilter {
        selected_features: None,
        significance: args.significance,
        min_effect_size: args.min_effect_size,
    };
    let rows = results.filter(&filter);
    if let Some(path) = &args.plot.plot {
        use stamp_plot::{composition, feature_plots, heatmap, pca_plot, volcano};
        let plot = &args.plot;
        let size = |default: f64| plot.plot_height.unwrap_or(default);
        let feature_data = || -> CliResult<feature_plots::FeatureData> {
            let name = plot
                .plot_feature
                .clone()
                .or_else(|| rows.first().map(|r| r.feature.clone()))
                .ok_or("no feature passes the filters; name one with --plot-feature")?;
            let mut data = feature_plots::FeatureData::from_multi_group_profile(
                &profile,
                &name,
                feature_plots::Field::Proportion,
            )
            .ok_or_else(|| format!("unknown feature '{name}'"))?;
            data.p_value = results
                .rows
                .iter()
                .find(|r| r.feature == name)
                .map(|r| stamp_core::stats::results::format_p_value(r.p_value_corrected, -15));
            Ok(data)
        };
        let figure = match plot.plot_type.unwrap_or(PlotTypeArg::Heatmap) {
            PlotTypeArg::Heatmap => {
                let names: Vec<String> = rows.iter().map(|r| r.feature.clone()).collect();
                heatmap::plot(
                    &heatmap::Data::from_multi_group_profile(&profile, Some(&names)),
                    &heatmap::Options {
                        width: plot.plot_width,
                        height: size(7.0),
                        ..Default::default()
                    },
                )
            }
            PlotTypeArg::Pca => pca_plot::plot(
                &pca_plot::Data::from_multi_group_profile(&profile),
                &pca_plot::Options {
                    width: plot.plot_width,
                    height: size(6.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Bar => feature_plots::bar_plot(
                &feature_data()?,
                &feature_plots::BarOptions {
                    height: size(6.0),
                    ..Default::default()
                },
            ),
            PlotTypeArg::Box => feature_plots::box_plot(
                &feature_data()?,
                &feature_plots::BoxOptions {
                    width: plot.plot_width,
                    height: size(7.0),
                    ..Default::default()
                },
            ),
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
            kind @ (PlotTypeArg::AlphaDiversity | PlotTypeArg::Pcoa | PlotTypeArg::DistanceBox) => {
                community_figure(
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
                )?
            }
            other => return Err(unavailable(other, "several groups")),
        };
        write_figure(path, &figure, plot.plot_dpi)?;
    }
    let table = multi_group_table(&profile, &results, rows);
    write_table(args.profile.output.as_deref(), &table)
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
    let metadata = load_metadata(&tree, &args.metadata, &args.excluded_samples)?;
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
    let results = run_correlation_analysis(&profile, analysis, &mut rng, &mut |_, _| true)
        .expect("analysis is never cancelled");
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
                let name = plot
                    .plot_feature
                    .clone()
                    .or_else(|| rows.first().map(|r| r.feature.clone()))
                    .ok_or("no feature passes the filters; name one with --plot-feature")?;
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
                        "{} = {:.3}, p = {} (n = {})",
                        analysis.method.coefficient_name(),
                        r.coefficient,
                        stamp_core::stats::results::format_p_value(r.p_value_corrected, -15),
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
                let p: Vec<f64> = results.rows.iter().map(|r| r.p_value_corrected).collect();
                histogram::p_value_histogram(
                    &p,
                    &histogram::PValueOptions {
                        width: plot.plot_width,
                        height: plot.plot_height.unwrap_or(7.0),
                        ..Default::default()
                    },
                )
            }
            other => return Err(unavailable(other, "a correlation")),
        };
        write_figure(path, &figure, plot.plot_dpi)?;
    }
    let table = correlation_table(&profile, &results, rows);
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
) -> CliResult<stamp_plot::Figure> {
    use stamp_core::stats::diversity::{AlphaMetric, Distance, Normalisation};
    use stamp_plot::community_plots::{self, Community};
    use stamp_plot::{feature_plots, pca_plot};

    let mut rng = rng_from_seed(seed);
    let community = Community::from_tree(
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
    let distance = match plot.distance {
        DistanceArg::BrayCurtis => Distance::BrayCurtis,
        DistanceArg::Jaccard => Distance::Jaccard,
        DistanceArg::Aitchison => Distance::Aitchison,
    };
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
        PlotTypeArg::Pcoa => {
            // The same permutations for the report as for the plot.
            for line in
                community_plots::test_notes(&community, distance, 999, &mut rng_from_seed(seed))
            {
                eprintln!("{line}");
            }
            community_plots::pcoa_plot(
                &community,
                distance,
                Some((999, &mut rng_from_seed(seed))),
                &pca_plot::Options {
                    width: plot.plot_width,
                    height: plot.plot_height.unwrap_or(6.0),
                    ..Default::default()
                },
            )
        }
        _ => community_plots::distance_box_plot(&community, distance, &boxes),
    })
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
    let bytes = export(figure, format, dpi).map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn write_table(path: Option<&Path>, table: &Table) -> CliResult {
    write_output(path, &table.to_tsv())
}

/// File name of a path, for converters that name samples after their files.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn convert(format: ConvertFormat) -> CliResult {
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
    if let Err(problem) = ProfileTree::read(&spf) {
        eprintln!("warning: the converted profile cannot be loaded by STAMP as it is: {problem}");
    }
    write_output(output.output.as_deref(), &spf)
}

fn read_named(paths: &[std::path::PathBuf]) -> CliResult<(Vec<String>, Vec<String>)> {
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
