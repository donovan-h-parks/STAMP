//! The state of an analysis session: the loaded data, the chosen settings and
//! the results. This module has no user-interface code, so it can be tested
//! on its own.

use std::sync::Arc;

use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::multi_group::{MultiGroupEffectSize, MultiGroupTest};
use stamp_core::stats::post_hoc::{PostHocResult, PostHocTest};
use stamp_core::stats::results::{
    run_post_hoc, Control, CorrelationAnalysis, CorrelationFilter, CorrelationResults,
    CorrelationRow, CorrelationRun, GroupAnalysis, GroupFilter, GroupResults, GroupRow, GroupRun,
    MultiGroupAnalysis, MultiGroupFilter, MultiGroupResults, MultiGroupRow, MultiGroupRun,
    SampleAnalysis, SampleFilter, SampleResults, SampleRow, SampleRun, Step,
};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::{rng_from_seed, StatsSettings, TestType};
use stamp_core::{
    Condition, CorrelationProfile, GroupProfile, Groups, Metadata, MultiGroupProfile, ProfileTree,
    SampleProfile, SampleSelection, SecondGroup, Unclassified,
};

use crate::jobs::Job;

/// The kinds of analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    MultipleGroups,
    TwoGroups,
    TwoSamples,
    /// Every feature against a numeric metadata field.
    Correlation,
}

impl Mode {
    pub const ALL: [Mode; 4] = [
        Mode::MultipleGroups,
        Mode::TwoGroups,
        Mode::TwoSamples,
        Mode::Correlation,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Mode::MultipleGroups => "Multiple groups",
            Mode::TwoGroups => "Two groups",
            Mode::TwoSamples => "Two samples",
            Mode::Correlation => "Correlation",
        }
    }
}

/// How a sample's metadata value compares with a chosen value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Less,
    Equal,
    Greater,
}

impl Relation {
    pub const ALL: [Relation; 3] = [Relation::Less, Relation::Equal, Relation::Greater];

    pub fn name(self) -> &'static str {
        match self {
            Relation::Less => "<",
            Relation::Equal => "=",
            Relation::Greater => ">",
        }
    }

    fn ordering(self) -> std::cmp::Ordering {
        match self {
            Relation::Less => std::cmp::Ordering::Less,
            Relation::Equal => std::cmp::Ordering::Equal,
            Relation::Greater => std::cmp::Ordering::Greater,
        }
    }
}

/// A two-sample analysis: settings, then results once computed.
pub struct SampleState {
    pub sample1: String,
    pub sample2: String,
    pub analysis: SampleAnalysis,
    pub filter: SampleFilter,
    pub output: Option<(SampleProfile, SampleResults)>,
}

/// A two-group analysis.
pub struct GroupState {
    pub group1: String,
    /// `None` compares against all other samples.
    pub group2: Option<String>,
    /// The metadata field that says which samples of the two groups belong
    /// together, for the paired tests.
    pub pair_field: Option<String>,
    /// How many pairs the last paired analysis found, as a sentence.
    pub pairing: Option<String>,
    pub analysis: GroupAnalysis,
    pub filter: GroupFilter,
    pub output: Option<(GroupProfile, GroupResults)>,
}

/// A correlation analysis: every feature against a numeric metadata field.
pub struct CorrelationState {
    pub field: Option<String>,
    pub analysis: CorrelationAnalysis,
    pub filter: CorrelationFilter,
    pub output: Option<(CorrelationProfile, CorrelationResults)>,
}

/// A multiple-group analysis.
pub struct MultiState {
    /// Groups included in the analysis.
    pub active_groups: Vec<String>,
    pub analysis: MultiGroupAnalysis,
    pub filter: MultiGroupFilter,
    pub post_hoc_test: PostHocTest,
    pub post_hoc_coverage: f64,
    pub output: Option<(MultiGroupProfile, MultiGroupResults)>,
}

enum Finished {
    Samples(Option<(SampleProfile, SampleResults)>),
    Groups(Option<(GroupProfile, GroupResults)>),
    Multi(Option<(MultiGroupProfile, MultiGroupResults)>),
    Correlation(Option<(CorrelationProfile, CorrelationResults)>),
}

type Outcome = Result<Finished, String>;

/// Everything the application knows about the current analysis.
pub struct Session {
    pub profile_name: String,
    pub metadata_name: String,
    pub tree: Option<Arc<ProfileTree>>,
    pub metadata: Option<Metadata>,
    /// Problems found while loading, for the user to read.
    pub warnings: Vec<String>,
    /// The most recent error of each analysis, shown until it runs again.
    errors: [Option<String>; 4],
    /// See [`Session::numeric_fields`].
    numeric_fields: Vec<String>,

    pub profile_level: String,
    /// `None` is the entire sample.
    pub parent_level: Option<String>,
    pub unclassified: Unclassified,
    pub group_field: Option<String>,
    pub stats: StatsSettings,
    pub seed: u64,

    pub samples: SampleState,
    pub groups: GroupState,
    pub multi: MultiState,
    pub correlation: CorrelationState,
    /// Feature shown by the single-feature plots.
    pub selected_feature: Option<String>,
    /// The plot the assistant drew last, if any.
    pub custom_plot: Option<std::sync::Arc<crate::plots::CustomPlot>>,
    /// Colours the user has chosen for groups, by group name.
    group_colors: std::collections::HashMap<String, stamp_plot::Color>,
    /// Colour of the "All other samples" group.
    pub other_samples_color: stamp_plot::Color,

    /// Which analyses are out of date.
    dirty: [bool; 4],
    jobs: [Option<Job<Outcome>>; 4],
    /// Increases whenever results change, so that views know to refresh.
    pub generation: u64,
    /// Ordination layouts already computed, reused while only how the
    /// plot looks changes.
    pub ordinations: crate::plots::OrdinationCache,
}

/// How a run paces itself inside a job: it reports its progress, stops
/// for good once the job is cancelled, and pauses when its time is up.
fn pace<'a>(
    progress: &'a crate::jobs::Progress,
    out_of_time: &'a dyn Fn() -> bool,
) -> impl FnMut(usize, usize) -> Control + 'a {
    move |done, total| {
        if !progress.report(done, total) {
            Control::Cancel
        } else if out_of_time() {
            Control::Pause
        } else {
            Control::Continue
        }
    }
}

/// What a job's step returns after advancing a run: nothing yet if the run
/// paused, and otherwise the outcome, built from the profile and the
/// results (`None` if the run was cancelled).
fn finish<P, R, S>(
    step: Step<S>,
    state: &mut Option<(P, R, stamp_core::stats::StampRng)>,
    outcome: impl FnOnce(P, Option<S>) -> Finished,
) -> Option<Outcome> {
    let results = match step {
        Step::Paused => return None,
        Step::Cancelled => None,
        Step::Done(results) => Some(results),
    };
    let (profile, _, _) = state.take()?;
    Some(Ok(outcome(profile, results)))
}

fn mode_index(mode: Mode) -> usize {
    match mode {
        Mode::MultipleGroups => 0,
        Mode::TwoGroups => 1,
        Mode::TwoSamples => 2,
        Mode::Correlation => 3,
    }
}

impl Default for Session {
    fn default() -> Self {
        Session {
            profile_name: String::new(),
            metadata_name: String::new(),
            tree: None,
            metadata: None,
            warnings: Vec::new(),
            errors: Default::default(),
            numeric_fields: Vec::new(),
            profile_level: String::new(),
            parent_level: None,
            unclassified: Unclassified::Retain,
            group_field: None,
            stats: StatsSettings::default(),
            seed: 1,
            samples: SampleState {
                sample1: String::new(),
                sample2: String::new(),
                analysis: SampleAnalysis {
                    test: SampleTest::GTestFisher,
                    test_type: TestType::TwoSided,
                    ci_method: SampleCi::DiffBetweenPropAsymptoticCc,
                    coverage: 0.95,
                    correction: Correction::None,
                },
                filter: SampleFilter::default(),
                output: None,
            },
            groups: GroupState {
                group1: String::new(),
                group2: None,
                pair_field: None,
                pairing: None,
                analysis: GroupAnalysis {
                    test: GroupTest::Welch,
                    test_type: TestType::TwoSided,
                    coverage: 0.95,
                    correction: Correction::None,
                    transform: stamp_core::stats::results::Transform::None,
                },
                filter: GroupFilter::default(),
                output: None,
            },
            multi: MultiState {
                active_groups: Vec::new(),
                analysis: MultiGroupAnalysis {
                    test: MultiGroupTest::Anova,
                    effect_size: MultiGroupEffectSize::EtaSquared,
                    correction: Correction::None,
                    transform: stamp_core::stats::results::Transform::None,
                },
                filter: MultiGroupFilter::default(),
                post_hoc_test: PostHocTest::TukeyKramer,
                post_hoc_coverage: 0.95,
                output: None,
            },
            correlation: CorrelationState {
                field: None,
                analysis: CorrelationAnalysis {
                    method: CorrelationMethod::Spearman,
                    correction: Correction::None,
                },
                filter: CorrelationFilter::default(),
                output: None,
            },
            selected_feature: None,
            custom_plot: None,
            group_colors: std::collections::HashMap::new(),
            other_samples_color: stamp_plot::Color::rgb(127, 127, 127),
            dirty: [false; 4],
            jobs: [None, None, None, None],
            generation: 0,
            ordinations: Default::default(),
        }
    }
}

impl Session {
    /// Load a profile, replacing any current data.
    pub fn load_profile(&mut self, name: &str, text: &str) -> Result<(), String> {
        let tree = ProfileTree::read(text).map_err(|e| e.to_string())?;
        self.load_tree(name, tree);
        Ok(())
    }

    /// Use a profile already read (see [`load_profile`](Self::load_profile)).
    pub fn load_tree(&mut self, name: &str, tree: ProfileTree) {
        *self = Session {
            stats: self.stats,
            seed: self.seed,
            other_samples_color: self.other_samples_color,
            // Never the same number as for earlier data, so that plots and
            // tables kept by the window for it are not taken for this.
            generation: self.generation + 1,
            ..Session::default()
        };
        self.profile_name = name.to_string();
        self.profile_level = tree.hierarchy_headings()[0].clone();
        // The first two samples in alphabetical order, as the lists show them.
        let mut samples = tree.sample_names().to_vec();
        samples.sort();
        self.samples.sample1 = samples[0].clone();
        self.samples.sample2 = samples[1].clone();
        self.tree = Some(Arc::new(tree));
        self.invalidate_all();
    }

    /// Load metadata for the current profile.
    pub fn load_metadata(&mut self, name: &str, text: &str) -> Result<(), String> {
        let tree = self
            .tree
            .as_ref()
            .ok_or("Load a profile before its metadata.")?;
        let (metadata, warnings) = Metadata::read(text, tree).map_err(|e| e.to_string())?;
        if metadata.samples().is_empty() {
            return Err("None of the samples in the metadata file are in the profile.".to_string());
        }
        self.metadata_name = name.to_string();
        self.warnings = warnings;
        let field = metadata.fields().first().cloned();
        self.metadata = Some(metadata);
        self.numeric_fields = self.find_numeric_fields();
        match field {
            Some(field) => self.set_group_field(&field),
            None => self.group_field = None,
        }
        self.groups.pair_field = None;
        self.correlation.field = self.numeric_fields().into_iter().next();
        self.invalidate(Mode::Correlation);
        Ok(())
    }

    /// A description of everything in the session that a plot is drawn
    /// from, other than the selected feature: two plots of the same kind and
    /// settings drawn from sessions with the same fingerprint are the same.
    /// The window uses it to keep plots it has already drawn.
    pub fn plot_fingerprint(&self) -> String {
        let mut colors: Vec<(&String, &stamp_plot::Color)> = self.group_colors.iter().collect();
        colors.sort_by(|a, b| a.0.cmp(b.0));
        format!(
            "{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{:?}",
            self.generation,
            self.seed,
            self.profile_level,
            self.parent_level,
            self.unclassified,
            self.group_field,
            self.errors,
            colors,
            self.other_samples_color,
            self.samples.filter,
            self.groups.filter,
            self.multi.filter,
            self.correlation.filter,
            self.multi.post_hoc_test,
            self.multi.post_hoc_coverage,
            self.custom_plot.as_ref().map(|c| c.id),
        )
    }

    /// A description of what the results table of `mode` is made from: its
    /// results and, when it lists only the features that pass the filters,
    /// those filters. Unlike [`Session::plot_fingerprint`] it leaves out
    /// what only changes how plots look, such as group colours.
    pub fn table_fingerprint(&self, mode: Mode, only_active: bool) -> String {
        let filter = if only_active {
            match mode {
                Mode::TwoSamples => format!("{:?}", self.samples.filter),
                Mode::TwoGroups => format!("{:?}", self.groups.filter),
                Mode::MultipleGroups => format!("{:?}", self.multi.filter),
                Mode::Correlation => format!("{:?}", self.correlation.filter),
            }
        } else {
            String::new()
        };
        format!("{}|{:?}|{filter}", self.generation, self.stats)
    }

    /// The colour of the group at `position` among all groups of the field,
    /// as [`Session::group_color`] gives it, without grouping the samples
    /// again.
    pub fn group_color_at(&self, group: &str, position: usize) -> stamp_plot::Color {
        if let Some(color) = self.group_colors.get(group) {
            return *color;
        }
        stamp_plot::axes::group_color(position)
    }

    /// Colour of a group: the one the user chose, or else one from the
    /// palette by the group's position among all groups of the field.
    pub fn group_color(&self, group: &str) -> stamp_plot::Color {
        if let Some(color) = self.group_colors.get(group) {
            return *color;
        }
        if group == stamp_core::ALL_OTHER_SAMPLES {
            return self.other_samples_color;
        }
        let position = self
            .grouping()
            .and_then(|groups| groups.names().position(|name| name == group));
        position.map_or(stamp_plot::Color::BLACK, stamp_plot::axes::group_color)
    }

    /// Choose the colour of a group.
    pub fn set_group_color(&mut self, group: &str, color: stamp_plot::Color) {
        self.group_colors.insert(group.to_string(), color);
        self.generation += 1;
    }

    /// The colours chosen for groups (others take the palette's), for
    /// undoing a change.
    pub fn chosen_group_colors(&self) -> std::collections::HashMap<String, stamp_plot::Color> {
        self.group_colors.clone()
    }

    /// Put back colours returned by [`Session::chosen_group_colors`].
    pub fn set_chosen_group_colors(
        &mut self,
        colors: std::collections::HashMap<String, stamp_plot::Color>,
    ) {
        if colors != self.group_colors {
            self.group_colors = colors;
            self.generation += 1;
        }
    }

    /// Group the samples by another metadata field.
    pub fn set_group_field(&mut self, field: &str) {
        self.group_field = Some(field.to_string());
        // Colours belong to the groups of one field.
        self.group_colors.clear();
        let names: Vec<String> = match self.grouping() {
            Some(groups) => groups
                .names()
                // Groups of a single sample cannot be tested, so they start
                // out switched off; the user can still include them.
                .filter(|g| groups.samples(g).is_some_and(|s| s.len() >= 2))
                .map(str::to_string)
                .collect(),
            None => Vec::new(),
        };
        self.groups.group1 = names.first().cloned().unwrap_or_default();
        self.groups.group2 = names.get(1).cloned();
        self.multi.active_groups = names;
        self.invalidate(Mode::TwoGroups);
        self.invalidate(Mode::MultipleGroups);
    }

    /// Choose the samples of every analysis by their metadata values, as the
    /// Sample filter does; an empty selection keeps every sample. Samples
    /// left out take no part in any analysis or plot, and groups left
    /// without samples disappear. Returns how many samples are kept.
    pub fn set_sample_selection(&mut self, selection: SampleSelection) -> Result<usize, String> {
        let metadata = self
            .metadata
            .as_mut()
            .ok_or("Load metadata before choosing samples by it.")?;
        if metadata.selection() == &selection {
            return Ok(metadata.num_selected());
        }
        let kept = metadata
            .set_selection(selection)
            .map_err(|e| e.to_string())?;
        self.refresh_groups();
        self.invalidate(Mode::TwoGroups);
        self.invalidate(Mode::MultipleGroups);
        self.invalidate(Mode::Correlation);
        self.generation += 1;
        Ok(kept)
    }

    /// Keep the samples whose value of `field` is one of `values` (`None`
    /// keeps every sample): the window's Sample filter.
    pub fn set_sample_filter(
        &mut self,
        filter: Option<(&str, &[String])>,
    ) -> Result<usize, String> {
        let selection = match filter {
            Some((field, values)) => SampleSelection {
                conditions: vec![Condition::OneOf {
                    field: field.to_string(),
                    values: values.to_vec(),
                }],
            },
            None => SampleSelection::default(),
        };
        self.set_sample_selection(selection)
    }

    /// The field and values of the Sample filter, if one is set.
    pub fn sample_filter(&self) -> Option<(String, Vec<String>)> {
        match self.metadata.as_ref()?.selection().conditions.as_slice() {
            [Condition::OneOf { field, values }] => Some((field.clone(), values.clone())),
            _ => None,
        }
    }

    /// After the sample filter changes: the groups that analyses include
    /// are those with at least two samples, and the two groups compared
    /// stay as they were if they still have samples.
    fn refresh_groups(&mut self) {
        let Some(groups) = self.grouping() else {
            return;
        };
        let has_samples = |g: &str| groups.samples(g).is_some_and(|s| !s.is_empty());
        let names: Vec<String> = groups
            .names()
            .filter(|g| groups.samples(g).is_some_and(|s| s.len() >= 2))
            .map(str::to_string)
            .collect();
        if !has_samples(&self.groups.group1) {
            self.groups.group1 = names.first().cloned().unwrap_or_default();
        }
        if self
            .groups
            .group2
            .as_deref()
            .is_some_and(|g| !has_samples(g))
            || self.groups.group2.as_ref() == Some(&self.groups.group1)
        {
            self.groups.group2 = names.iter().find(|g| **g != self.groups.group1).cloned();
        }
        self.multi.active_groups = names;
    }

    /// The metadata fields a correlation can be made with: those that hold
    /// a number for at least three samples. Worked out once, when the
    /// metadata is loaded, since the window asks for them every frame.
    pub fn numeric_fields(&self) -> Vec<String> {
        self.numeric_fields.clone()
    }

    fn find_numeric_fields(&self) -> Vec<String> {
        let Some(metadata) = &self.metadata else {
            return Vec::new();
        };
        metadata
            .fields()
            .iter()
            .filter(|field| {
                metadata
                    .samples()
                    .iter()
                    .filter(|sample| {
                        metadata
                            .value(sample, field)
                            .and_then(|v| v.trim().parse::<f64>().ok())
                            .is_some_and(f64::is_finite)
                    })
                    .count()
                    >= 3
            })
            .cloned()
            .collect()
    }

    /// The samples grouped by the chosen metadata field.
    pub fn grouping(&self) -> Option<Groups> {
        self.metadata
            .as_ref()?
            .groups(self.group_field.as_deref()?)
            .ok()
    }

    /// Include or exclude samples from the group analyses, as the metadata
    /// table does.
    pub fn set_samples_active(&mut self, samples: &[String], active: bool) {
        let Some(metadata) = &mut self.metadata else {
            return;
        };
        let mut changed = false;
        for sample in samples {
            if metadata.is_active(sample) != active {
                metadata.set_active(sample, active);
                changed = true;
            }
        }
        if changed {
            self.invalidate(Mode::TwoGroups);
            self.invalidate(Mode::MultipleGroups);
            self.invalidate(Mode::Correlation);
            self.generation += 1;
        }
    }

    /// The samples switched off by hand (unticked in the metadata table),
    /// for undoing a change.
    pub fn switched_off_samples(&self) -> Vec<String> {
        let Some(metadata) = &self.metadata else {
            return Vec::new();
        };
        metadata
            .samples()
            .iter()
            .filter(|s| !metadata.is_switched_on(s))
            .cloned()
            .collect()
    }

    /// Switch off exactly `off`, and every other sample on, as returned by
    /// [`Session::switched_off_samples`].
    pub fn set_switched_off_samples(&mut self, off: &[String]) {
        let Some(metadata) = &mut self.metadata else {
            return;
        };
        let mut changed = false;
        for sample in metadata.samples().to_vec() {
            let on = !off.contains(&sample);
            if metadata.is_switched_on(&sample) != on {
                metadata.set_active(&sample, on);
                changed = true;
            }
        }
        if changed {
            self.invalidate(Mode::TwoGroups);
            self.invalidate(Mode::MultipleGroups);
            self.invalidate(Mode::Correlation);
            self.generation += 1;
        }
    }

    /// Samples whose value of a metadata field stands in `relation` to
    /// `value`. Numbers are compared as numbers when the whole field is
    /// numeric, and as text otherwise.
    pub fn samples_matching(&self, field: &str, relation: Relation, value: &str) -> Vec<String> {
        let Some(metadata) = &self.metadata else {
            return Vec::new();
        };
        let number = |text: &str| text.trim().parse::<f64>().ok();
        let numeric = metadata.is_numeric(field);
        metadata
            .samples()
            .iter()
            .filter(|sample| {
                let Some(own) = metadata.value(sample, field) else {
                    return false;
                };
                let ordering = match (numeric, number(own), number(value)) {
                    (true, Some(a), Some(b)) => a.partial_cmp(&b),
                    _ => Some(own.cmp(value)),
                };
                ordering == Some(relation.ordering())
            })
            .cloned()
            .collect()
    }

    /// Mark one analysis as needing to be run again.
    pub fn invalidate(&mut self, mode: Mode) {
        self.dirty[mode_index(mode)] = true;
    }

    /// Mark every analysis as out of date, after a change to the profile
    /// level or another shared setting.
    pub fn invalidate_all(&mut self) {
        self.dirty = [true; 4];
    }

    /// True while an analysis of this kind is running.
    pub fn is_running(&self, mode: Mode) -> bool {
        self.jobs[mode_index(mode)].is_some()
    }

    /// Progress of the running analysis, from 0 to 1.
    pub fn progress(&self, mode: Mode) -> Option<f32> {
        self.jobs[mode_index(mode)].as_ref().map(Job::fraction)
    }

    /// Why the analysis of `mode` has no results, if it failed or was
    /// cancelled.
    pub fn error(&self, mode: Mode) -> Option<&str> {
        self.errors[mode_index(mode)].as_deref()
    }

    fn set_error(&mut self, mode: Mode, message: impl Into<String>) {
        self.errors[mode_index(mode)] = Some(message.into());
    }

    /// Forget the results of `mode`, which no longer match its settings.
    fn clear_output(&mut self, mode: Mode) {
        match mode {
            Mode::TwoSamples => self.samples.output = None,
            Mode::TwoGroups => self.groups.output = None,
            Mode::MultipleGroups => self.multi.output = None,
            Mode::Correlation => self.correlation.output = None,
        }
    }

    /// Stop the running analysis of `mode`. Its earlier results are
    /// cleared too: they were made with settings that have since changed.
    pub fn cancel(&mut self, mode: Mode) {
        let index = mode_index(mode);
        if self.jobs[index].take().is_some() {
            self.clear_output(mode);
            self.set_error(
                mode,
                "The analysis was cancelled. Change a setting to run it again.",
            );
            self.generation += 1;
        }
    }

    /// Start any analysis that is out of date for `mode` and collect finished
    /// results. Returns true if work is still in progress, so the caller
    /// should check again soon.
    pub fn update(&mut self, mode: Mode) -> bool {
        let index = mode_index(mode);
        if self.dirty[index] {
            self.dirty[index] = false;
            self.errors[index] = None;
            // Dropping a running job cancels it.
            self.jobs[index] = self.start(mode);
            if self.jobs[index].is_none() {
                // Nothing could be started: the results are gone, and the
                // views should show why.
                self.generation += 1;
            }
        }

        let mut finished = None;
        if let Some(job) = &self.jobs[index] {
            match job.poll() {
                Ok(outcome) => finished = Some(outcome),
                Err(true) => return true,
                Err(false) => {
                    finished = Some(Err("The analysis stopped unexpectedly.".to_string()))
                }
            }
        }
        if let Some(outcome) = finished {
            self.jobs[index] = None;
            self.generation += 1;
            match outcome {
                Ok(Finished::Samples(output)) => self.samples.output = output,
                Ok(Finished::Groups(output)) => self.groups.output = output,
                Ok(Finished::Multi(output)) => self.multi.output = output,
                Ok(Finished::Correlation(output)) => self.correlation.output = output,
                Err(message) => {
                    // The earlier results no longer match the settings.
                    self.clear_output(mode);
                    self.set_error(mode, message);
                    return false;
                }
            }
            self.errors[index] = None;
            // The correction may have been changed while the job ran with
            // the one chosen when it started.
            if let Some(chosen) = self.output_correction_mismatch(mode) {
                self.set_correction(mode, chosen);
            }
            self.keep_selection_valid(mode);
        }
        false
    }

    /// Run the analysis of `mode` to completion. For tests and scripts.
    pub fn run_to_completion(&mut self, mode: Mode) {
        while self.update(mode) {
            // In the browser the work runs inside `update` itself.
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    fn parent(&self) -> Option<String> {
        self.parent_level.clone()
    }

    fn start(&mut self, mode: Mode) -> Option<Job<Outcome>> {
        let tree = Arc::clone(self.tree.as_ref()?);
        let (parent, level, unclassified) =
            (self.parent(), self.profile_level.clone(), self.unclassified);
        let (stats, seed) = (self.stats, self.seed);
        match mode {
            Mode::TwoSamples => {
                let (s1, s2) = (self.samples.sample1.clone(), self.samples.sample2.clone());
                let analysis = self.samples.analysis;
                // The profile, the run and its random numbers, made on the
                // first step and kept until the last.
                let mut state = None;
                Some(Job::spawn_steps(move |progress, out_of_time| {
                    if state.is_none() {
                        match tree.sample_profile(&s1, &s2, parent.as_deref(), &level, unclassified)
                        {
                            Ok(profile) => {
                                let run = SampleRun::new(analysis, &stats);
                                state = Some((profile, run, rng_from_seed(seed)));
                            }
                            Err(error) => return Some(Err(error.to_string())),
                        }
                    }
                    let (profile, run, rng) = state.as_mut()?;
                    let step = run.advance(profile, rng, &mut pace(progress, out_of_time));
                    finish(step, &mut state, |profile, results| {
                        Finished::Samples(results.map(|r| (profile, r)))
                    })
                }))
            }
            Mode::TwoGroups => {
                let Some(groups) = self.grouping() else {
                    self.groups.output = None;
                    return None;
                };
                let (g1, g2) = (self.groups.group1.clone(), self.groups.group2.clone());
                let analysis = self.groups.analysis;
                // A paired test takes the samples of the two groups pair by
                // pair, as a metadata field matches them.
                self.groups.pairing = None;
                let groups = if analysis.test.is_paired() {
                    let paired = match (&g2, &self.groups.pair_field, &self.metadata) {
                        (None, _, _) => Err("A paired test needs two named groups, not \
                            \"All other samples\"."
                            .to_string()),
                        (_, None, _) | (_, _, None) => Err("Choose the metadata field that \
                            says which samples form a pair."
                            .to_string()),
                        (Some(g2), Some(field), Some(metadata)) => metadata
                            .paired_groups(&groups, &g1, g2, field)
                            .map_err(|e| e.to_string()),
                    };
                    match paired {
                        Ok(pairing) => {
                            self.groups.pairing = Some(pairing.summary());
                            pairing.groups
                        }
                        Err(message) => {
                            self.groups.output = None;
                            self.set_error(Mode::TwoGroups, message);
                            return None;
                        }
                    }
                } else {
                    groups
                };
                let mut state = None;
                Some(Job::spawn_steps(move |progress, out_of_time| {
                    if state.is_none() {
                        let second = match &g2 {
                            Some(name) => SecondGroup::Named(name),
                            None => SecondGroup::AllOtherSamples,
                        };
                        let profile = match tree.group_profile(
                            &groups,
                            &g1,
                            second,
                            parent.as_deref(),
                            &level,
                            unclassified,
                        ) {
                            Ok(profile) => profile,
                            Err(error) => return Some(Err(error.to_string())),
                        };
                        if profile.samples_in_group1().is_empty()
                            || profile.samples_in_group2().is_empty()
                        {
                            return Some(Err("Both groups need at least one sample.".to_string()));
                        }
                        let run = GroupRun::new(analysis, &stats);
                        state = Some((profile, run, rng_from_seed(seed)));
                    }
                    let (profile, run, rng) = state.as_mut()?;
                    let step = run.advance(profile, rng, &mut pace(progress, out_of_time));
                    finish(step, &mut state, |profile, results| {
                        Finished::Groups(results.map(|r| (profile, r)))
                    })
                }))
            }
            Mode::Correlation => {
                let (Some(metadata), Some(field)) =
                    (self.metadata.clone(), self.correlation.field.clone())
                else {
                    self.correlation.output = None;
                    return None;
                };
                let analysis = self.correlation.analysis;
                let mut state = None;
                Some(Job::spawn_steps(move |progress, out_of_time| {
                    if state.is_none() {
                        match tree.correlation_profile(
                            &metadata,
                            &field,
                            parent.as_deref(),
                            &level,
                            unclassified,
                        ) {
                            Ok(profile) if profile.values.len() < 3 => {
                                return Some(Err(format!(
                                    "A correlation needs at least three samples with a \
                                     number for \"{field}\"."
                                )));
                            }
                            Ok(profile) => {
                                let run = CorrelationRun::new(analysis);
                                state = Some((profile, run, rng_from_seed(seed)));
                            }
                            Err(error) => return Some(Err(error.to_string())),
                        }
                    }
                    let (profile, run, rng) = state.as_mut()?;
                    let step = run.advance(profile, rng, &mut pace(progress, out_of_time));
                    finish(step, &mut state, |profile, results| {
                        Finished::Correlation(results.map(|r| (profile, r)))
                    })
                }))
            }
            Mode::MultipleGroups => {
                let Some(groups) = self.grouping() else {
                    self.multi.output = None;
                    return None;
                };
                // A chosen group whose samples are all switched off takes no
                // part, rather than making every test degenerate.
                let active: Vec<String> = self
                    .multi
                    .active_groups
                    .iter()
                    .filter(|g| groups.samples(g).is_some_and(|s| !s.is_empty()))
                    .cloned()
                    .collect();
                let analysis = self.multi.analysis;
                if active.len() < 2 {
                    self.multi.output = None;
                    self.set_error(
                        Mode::MultipleGroups,
                        if self.multi.active_groups.len() < 2 {
                            "Select at least two groups."
                        } else {
                            "At least two of the selected groups need samples that are \
                             switched on."
                        },
                    );
                    return None;
                }
                let mut state = None;
                Some(Job::spawn_steps(move |progress, out_of_time| {
                    if state.is_none() {
                        let names: Vec<&str> = active.iter().map(String::as_str).collect();
                        match tree.multi_group_profile(
                            &groups,
                            Some(&names),
                            parent.as_deref(),
                            &level,
                            unclassified,
                        ) {
                            Ok(profile) => {
                                let run = MultiGroupRun::new(analysis);
                                state = Some((profile, run, rng_from_seed(seed)));
                            }
                            Err(error) => return Some(Err(error.to_string())),
                        }
                    }
                    let (profile, run, rng) = state.as_mut()?;
                    let step = run.advance(profile, rng, &mut pace(progress, out_of_time));
                    finish(step, &mut state, |profile, results| {
                        Finished::Multi(results.map(|r| (profile, r)))
                    })
                }))
            }
        }
    }

    /// Keep the selected feature for as long as the analysis has it, whether
    /// or not it passes the filters: re-running a test, switching samples
    /// off or changing a filter must not change which feature is shown. If
    /// the feature is gone (after a change of profile level, say), select
    /// the most significant one that passes the filters.
    fn keep_selection_valid(&mut self, mode: Mode) {
        let exists = |feature: &str| match mode {
            Mode::TwoSamples => self
                .samples
                .output
                .as_ref()
                .is_some_and(|(_, r)| r.rows.iter().any(|row| row.feature == feature)),
            Mode::TwoGroups => self
                .groups
                .output
                .as_ref()
                .is_some_and(|(_, r)| r.rows.iter().any(|row| row.feature == feature)),
            Mode::MultipleGroups => self
                .multi
                .output
                .as_ref()
                .is_some_and(|(_, r)| r.rows.iter().any(|row| row.feature == feature)),
            Mode::Correlation => self
                .correlation
                .output
                .as_ref()
                .is_some_and(|(_, r)| r.rows.iter().any(|row| row.feature == feature)),
        };
        if self.selected_feature.as_deref().is_some_and(exists) {
            return;
        }
        let first = |passing: Option<&str>, any: Option<&str>| passing.or(any).map(str::to_string);
        self.selected_feature = match mode {
            Mode::TwoSamples => first(
                self.sample_rows().first().map(|r| r.feature.as_str()),
                self.samples
                    .output
                    .as_ref()
                    .and_then(|(_, r)| r.rows.first())
                    .map(|r| r.feature.as_str()),
            ),
            Mode::TwoGroups => first(
                self.group_rows().first().map(|r| r.feature.as_str()),
                self.groups
                    .output
                    .as_ref()
                    .and_then(|(_, r)| r.rows.first())
                    .map(|r| r.feature.as_str()),
            ),
            Mode::MultipleGroups => first(
                self.multi_rows().first().map(|r| r.feature.as_str()),
                self.multi
                    .output
                    .as_ref()
                    .and_then(|(_, r)| r.rows.first())
                    .map(|r| r.feature.as_str()),
            ),
            Mode::Correlation => first(
                self.correlation_rows().first().map(|r| r.feature.as_str()),
                self.correlation
                    .output
                    .as_ref()
                    .and_then(|(_, r)| r.rows.first())
                    .map(|r| r.feature.as_str()),
            ),
        };
    }

    /// Rows of the two-sample results that pass the filters.
    pub fn sample_rows(&self) -> Vec<&SampleRow> {
        match &self.samples.output {
            Some((_, results)) => results.filter(&self.samples.filter, &self.stats),
            None => Vec::new(),
        }
    }

    /// Rows of the two-group results that pass the filters.
    pub fn group_rows(&self) -> Vec<&GroupRow> {
        match &self.groups.output {
            Some((profile, results)) => results.filter(profile, &self.groups.filter, &self.stats),
            None => Vec::new(),
        }
    }

    /// Rows of the multiple-group results that pass the filters.
    pub fn multi_rows(&self) -> Vec<&MultiGroupRow> {
        match &self.multi.output {
            Some((_, results)) => results.filter(&self.multi.filter),
            None => Vec::new(),
        }
    }

    /// Rows of the correlation results that pass the filters.
    pub fn correlation_rows(&self) -> Vec<&CorrelationRow> {
        match &self.correlation.output {
            Some((_, results)) => results.filter(&self.correlation.filter),
            None => Vec::new(),
        }
    }

    /// Number of features in total and passing the filters, for `mode`.
    pub fn feature_counts(&self, mode: Mode) -> (usize, usize) {
        match mode {
            Mode::TwoSamples => (
                self.samples
                    .output
                    .as_ref()
                    .map_or(0, |(_, r)| r.rows.len()),
                self.sample_rows().len(),
            ),
            Mode::TwoGroups => (
                self.groups.output.as_ref().map_or(0, |(_, r)| r.rows.len()),
                self.group_rows().len(),
            ),
            Mode::MultipleGroups => (
                self.multi.output.as_ref().map_or(0, |(_, r)| r.rows.len()),
                self.multi_rows().len(),
            ),
            Mode::Correlation => (
                self.correlation
                    .output
                    .as_ref()
                    .map_or(0, |(_, r)| r.rows.len()),
                self.correlation_rows().len(),
            ),
        }
    }

    /// The correction chosen for `mode`, if its results were corrected with
    /// another.
    fn output_correction_mismatch(&self, mode: Mode) -> Option<Correction> {
        let (chosen, applied) = match mode {
            Mode::TwoSamples => (
                self.samples.analysis.correction,
                self.samples.output.as_ref()?.1.analysis.correction,
            ),
            Mode::TwoGroups => (
                self.groups.analysis.correction,
                self.groups.output.as_ref()?.1.analysis.correction,
            ),
            Mode::MultipleGroups => (
                self.multi.analysis.correction,
                self.multi.output.as_ref()?.1.analysis.correction,
            ),
            Mode::Correlation => (
                self.correlation.analysis.correction,
                self.correlation.output.as_ref()?.1.analysis.correction,
            ),
        };
        (chosen != applied).then_some(chosen)
    }

    /// Change the multiple-test correction of `mode` without re-running the
    /// tests.
    pub fn set_correction(&mut self, mode: Mode, correction: Correction) {
        let mut rng = rng_from_seed(self.seed);
        match mode {
            Mode::TwoSamples => {
                self.samples.analysis.correction = correction;
                if let Some((_, results)) = &mut self.samples.output {
                    results.apply_correction(correction, &mut rng);
                }
            }
            Mode::TwoGroups => {
                self.groups.analysis.correction = correction;
                if let Some((_, results)) = &mut self.groups.output {
                    results.apply_correction(correction, &mut rng);
                }
            }
            Mode::MultipleGroups => {
                self.multi.analysis.correction = correction;
                if let Some((_, results)) = &mut self.multi.output {
                    results.apply_correction(correction, &mut rng);
                }
            }
            Mode::Correlation => {
                self.correlation.analysis.correction = correction;
                if let Some((_, results)) = &mut self.correlation.output {
                    results.apply_correction(correction, &mut rng);
                }
            }
        }
        self.generation += 1;
    }

    /// Post-hoc test on the selected feature of the multiple-group analysis.
    pub fn post_hoc(&self) -> Option<Result<PostHocResult, String>> {
        let (profile, _) = self.multi.output.as_ref()?;
        let feature = self.selected_feature.as_deref()?;
        profile.profile.feature(feature)?;
        Some(
            run_post_hoc(
                profile,
                feature,
                self.multi.post_hoc_test,
                self.multi.post_hoc_coverage,
            )
            .map_err(|e| e.to_string()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPF: &str = "Phylum\tGenus\tA1\tA2\tA3\tB1\tB2\tB3\tC1\tC2\tC3\n\
        P1\tx\t10\t12\t11\t30\t33\t31\t20\t21\t19\n\
        P1\ty\t50\t48\t52\t50\t49\t51\t50\t50\t50\n\
        P2\tz\t40\t40\t37\t20\t18\t18\t30\t29\t31\n";
    // "Subject" pairs the samples of A with those of B; "Depth" is a
    // number for every sample but one.
    const META: &str = "Id\tGroup\tSite\tSubject\tDepth\n\
        A1\tA\tgut\ts1\t1.0\nA2\tA\tgut\ts2\t2.5\nA3\tA\tskin\ts3\t4.0\n\
        B1\tB\tgut\ts3\t5.0\nB2\tB\tskin\ts1\t7.5\nB3\tB\tskin\ts2\t8.0\n\
        C1\tC\tgut\ts4\t9.0\nC2\tC\tgut\ts5\t11.0\nC3\tC\tskin\ts6\tunknown\n";

    fn loaded() -> Session {
        let mut session = Session::default();
        session.load_profile("test.spf", SPF).unwrap();
        session.load_metadata("test.tsv", META).unwrap();
        session
    }

    #[test]
    fn samples_are_switched_off_by_their_metadata() {
        let mut session = loaded();
        let skin = session.samples_matching("Site", Relation::Equal, "skin");
        assert_eq!(skin, ["A3", "B2", "B3", "C3"]);
        assert_eq!(
            session.samples_matching("Site", Relation::Less, "h").len(),
            5
        );
        assert!(session
            .samples_matching("Site", Relation::Greater, "skin")
            .is_empty());

        session.set_samples_active(&skin, false);
        let groups = session.grouping().unwrap();
        assert_eq!(groups.samples("A").unwrap(), ["A1", "A2"]);
        assert_eq!(groups.samples("B").unwrap(), ["B1"]);
        session.run_to_completion(Mode::TwoGroups);
        let (profile, _) = session.groups.output.as_ref().unwrap();
        assert_eq!(profile.samples_in_group1().len(), 2);
        assert_eq!(profile.samples_in_group2().len(), 1);

        session.set_samples_active(&skin, true);
        session.run_to_completion(Mode::TwoGroups);
        let (profile, _) = session.groups.output.as_ref().unwrap();
        assert_eq!(profile.samples_in_group1().len(), 3);
    }

    #[test]
    fn the_sample_filter_restricts_every_analysis() {
        let mut session = loaded();
        let gut = ["gut".to_string()];
        assert_eq!(session.set_sample_filter(Some(("Site", &gut))), Ok(5));
        assert_eq!(
            session.sample_filter(),
            Some(("Site".to_string(), gut.to_vec()))
        );
        // B keeps one sample, too few to take part in the multiple-group
        // analysis; the two groups compared stay as they were.
        assert_eq!(session.multi.active_groups, ["A", "C"]);
        assert_eq!(
            (
                session.groups.group1.as_str(),
                session.groups.group2.as_deref()
            ),
            ("A", Some("B"))
        );
        session.run_to_completion(Mode::MultipleGroups);
        let (profile, _) = session.multi.output.as_ref().unwrap();
        assert_eq!(profile.group_names, ["A", "C"]);
        assert_eq!(profile.profile.sample_names, ["A1", "A2", "C1", "C2"]);
        session.run_to_completion(Mode::Correlation);
        let (profile, _) = session.correlation.output.as_ref().unwrap();
        assert_eq!(profile.values.len(), 5);

        // Values of a field that leave a group empty make it disappear.
        let skin = ["skin".to_string()];
        assert_eq!(session.set_sample_filter(Some(("Site", &skin))), Ok(4));
        let groups = session.grouping().unwrap();
        assert_eq!(groups.samples("B").unwrap(), ["B2", "B3"]);
        assert!(session.set_sample_filter(Some(("Nope", &skin))).is_err());
        assert_eq!(
            session.set_sample_filter(Some(("Group", &["C".to_string()]))),
            Ok(3)
        );
        assert_eq!(
            session.grouping().unwrap().names().collect::<Vec<_>>(),
            ["C"]
        );
        assert_eq!(session.groups.group1, "C");

        // Clearing the filter keeps every sample again; new metadata starts
        // without a filter.
        assert_eq!(session.set_sample_filter(None), Ok(9));
        assert_eq!(session.sample_filter(), None);
        session.set_sample_filter(Some(("Site", &gut))).unwrap();
        session.load_metadata("test.tsv", META).unwrap();
        assert_eq!(session.sample_filter(), None);
        assert_eq!(session.metadata.as_ref().unwrap().active_samples().len(), 9);
    }

    #[test]
    fn loading_sets_sensible_defaults() {
        let session = loaded();
        assert_eq!(session.profile_level, "Phylum");
        assert_eq!(
            (
                session.samples.sample1.as_str(),
                session.samples.sample2.as_str()
            ),
            ("A1", "A2")
        );
        assert_eq!(session.group_field.as_deref(), Some("Group"));
        assert_eq!(session.groups.group1, "A");
        assert_eq!(session.groups.group2.as_deref(), Some("B"));
        assert_eq!(session.multi.active_groups, ["A", "B", "C"]);
        assert!(session.warnings.is_empty());
    }

    #[test]
    fn analyses_run_in_the_background_and_fill_in_results() {
        let mut session = loaded();
        session.profile_level = "Genus".to_string();
        session.invalidate_all();
        for mode in Mode::ALL {
            assert!(session.update(mode) || !session.is_running(mode));
            session.run_to_completion(mode);
            assert_eq!(session.feature_counts(mode), (3, 3), "{mode:?}");
            assert!(session.error(mode).is_none(), "{:?}", session.error(mode));
        }
        assert_eq!(session.selected_feature.as_deref(), Some("x"));
        let post_hoc = session.post_hoc().unwrap().unwrap();
        assert_eq!(post_hoc.comparisons.len(), 3);
        assert!(session.generation >= 3);
    }

    #[test]
    fn settings_changes_take_effect_after_invalidation() {
        let mut session = loaded();
        session.run_to_completion(Mode::TwoGroups);
        assert_eq!(session.feature_counts(Mode::TwoGroups).0, 2);

        session.profile_level = "Genus".to_string();
        session.invalidate_all();
        session.run_to_completion(Mode::TwoGroups);
        assert_eq!(session.feature_counts(Mode::TwoGroups).0, 3);

        session.groups.filter.significance = Some(0.01);
        assert_eq!(session.feature_counts(Mode::TwoGroups), (3, 2));

        // A correction is applied without re-running the tests.
        let generation = session.generation;
        session.set_correction(Mode::TwoGroups, Correction::Bonferroni);
        assert!(!session.is_running(Mode::TwoGroups));
        assert!(session.generation > generation);
        let (_, results) = session.groups.output.as_ref().unwrap();
        assert!(results
            .rows
            .iter()
            .all(|r| (r.p_value_corrected - (r.p_value * 3.0).min(1.0)).abs() < 1e-15));

        session.groups.group2 = None;
        session.invalidate(Mode::TwoGroups);
        session.run_to_completion(Mode::TwoGroups);
        assert_eq!(
            session
                .groups
                .output
                .as_ref()
                .unwrap()
                .0
                .samples_in_group2()
                .len(),
            6
        );
    }

    #[test]
    fn regrouping_and_errors() {
        let mut session = loaded();
        session.set_group_field("Site");
        assert_eq!(session.multi.active_groups, ["gut", "skin"]);
        session.run_to_completion(Mode::MultipleGroups);
        assert_eq!(
            session.multi.output.as_ref().unwrap().0.group_names,
            ["gut", "skin"]
        );

        session.multi.active_groups.truncate(1);
        session.invalidate(Mode::MultipleGroups);
        session.run_to_completion(Mode::MultipleGroups);
        assert_eq!(
            session.error(Mode::MultipleGroups),
            Some("Select at least two groups.")
        );
        assert!(session.multi.output.is_none());

        session.samples.sample2 = "missing".to_string();
        session.invalidate(Mode::TwoSamples);
        session.run_to_completion(Mode::TwoSamples);
        assert_eq!(
            session.error(Mode::TwoSamples),
            Some("Unknown sample 'missing'.")
        );
        // Each analysis keeps its own error.
        assert_eq!(
            session.error(Mode::MultipleGroups),
            Some("Select at least two groups.")
        );
        assert!(session.error(Mode::TwoGroups).is_none());

        let mut empty = Session::default();
        assert!(empty.load_metadata("m", META).is_err());
        assert!(empty.load_profile("bad", "only one line\n").is_err());
        assert!(!empty.update(Mode::TwoSamples));
    }

    #[test]
    fn a_group_with_every_sample_switched_off_takes_no_part() {
        let mut session = loaded();
        session.profile_level = "Genus".to_string();
        session.set_group_field("Site");
        let gut: Vec<String> = session.grouping().unwrap().samples("gut").unwrap().to_vec();
        session.set_samples_active(&gut, false);
        session.run_to_completion(Mode::MultipleGroups);
        // Only one group has samples left.
        assert!(session.multi.output.is_none());
        assert!(session
            .error(Mode::MultipleGroups)
            .unwrap()
            .contains("switched on"));
        // The choice of groups is kept for when the samples come back.
        assert_eq!(session.multi.active_groups, ["gut", "skin"]);
        session.set_samples_active(&gut, true);
        session.run_to_completion(Mode::MultipleGroups);
        assert_eq!(
            session.multi.output.as_ref().unwrap().0.group_names,
            ["gut", "skin"]
        );
    }

    #[test]
    fn cancelling_clears_results_that_no_longer_match() {
        let mut session = loaded();
        session.profile_level = "Genus".to_string();
        session.run_to_completion(Mode::TwoGroups);
        assert!(session.groups.output.is_some());
        session.groups.analysis.test = GroupTest::Welch;
        session.invalidate(Mode::TwoGroups);
        session.update(Mode::TwoGroups);
        session.cancel(Mode::TwoGroups);
        assert!(!session.is_running(Mode::TwoGroups));
        assert!(session.groups.output.is_none());
        assert!(session
            .error(Mode::TwoGroups)
            .unwrap()
            .contains("cancelled"));
        // Cancelling when nothing runs changes nothing.
        session.run_to_completion(Mode::TwoSamples);
        session.cancel(Mode::TwoSamples);
        assert!(session.samples.output.is_some());
    }

    #[test]
    fn a_correction_chosen_while_the_tests_run_is_applied_to_their_results() {
        let mut session = loaded();
        session.profile_level = "Genus".to_string();
        session.invalidate(Mode::TwoGroups);
        session.update(Mode::TwoGroups);
        session.set_correction(Mode::TwoGroups, Correction::Bonferroni);
        session.run_to_completion(Mode::TwoGroups);
        let (_, results) = session.groups.output.as_ref().unwrap();
        assert_eq!(results.analysis.correction, Correction::Bonferroni);
        let count = results.rows.len() as f64;
        for row in &results.rows {
            assert!((row.p_value_corrected - (row.p_value * count).min(1.0)).abs() < 1e-12);
        }
    }

    #[test]
    fn without_metadata_only_samples_can_be_compared() {
        let mut session = Session::default();
        session.load_profile("test.spf", SPF).unwrap();
        session.run_to_completion(Mode::TwoGroups);
        session.run_to_completion(Mode::TwoSamples);
        assert!(session.groups.output.is_none());
        assert!(session.samples.output.is_some());
        assert!(session.post_hoc().is_none());
    }

    #[test]
    fn paired_tests_take_their_pairs_from_a_field() {
        let mut session = loaded();
        session.profile_level = "Genus".to_string();
        session.groups.analysis.test = GroupTest::PairedT;
        session.invalidate(Mode::TwoGroups);
        // No field chosen yet: nothing is run, and the reason is given.
        session.run_to_completion(Mode::TwoGroups);
        assert!(session.groups.output.is_none());
        assert!(session
            .error(Mode::TwoGroups)
            .unwrap()
            .contains("form a pair"));

        session.groups.pair_field = Some("Subject".to_string());
        session.invalidate(Mode::TwoGroups);
        session.run_to_completion(Mode::TwoGroups);
        assert!(session.error(Mode::TwoGroups).is_none());
        assert_eq!(session.groups.pairing.as_deref(), Some("3 pairs"));
        let (profile, results) = session.groups.output.as_ref().unwrap();
        // Pair by pair: s1, s2, s3.
        assert_eq!(profile.samples_in_group1(), ["A1", "A2", "A3"]);
        assert_eq!(profile.samples_in_group2(), ["B2", "B3", "B1"]);
        assert!(results.rows.iter().all(|r| r.note.is_empty()));

        // A sample switched off takes its partner with it.
        session.set_samples_active(&["B2".to_string()], false);
        session.run_to_completion(Mode::TwoGroups);
        assert_eq!(
            session.groups.pairing.as_deref(),
            Some("2 pairs; 1 sample without a partner left out")
        );
        let (profile, _) = session.groups.output.as_ref().unwrap();
        assert_eq!(profile.samples_in_group1(), ["A2", "A3"]);

        // "All other samples" cannot be paired.
        session.groups.group2 = None;
        session.invalidate(Mode::TwoGroups);
        session.run_to_completion(Mode::TwoGroups);
        assert!(session.groups.output.is_none());
        assert!(session
            .error(Mode::TwoGroups)
            .unwrap()
            .contains("two named groups"));

        // An unpaired test ignores the field and uses every sample again.
        session.groups.group2 = Some("B".to_string());
        session.groups.analysis.test = GroupTest::Welch;
        session.invalidate(Mode::TwoGroups);
        session.run_to_completion(Mode::TwoGroups);
        assert!(session.groups.pairing.is_none());
        let (profile, _) = session.groups.output.as_ref().unwrap();
        assert_eq!(profile.samples_in_group1().len(), 3);
    }

    #[test]
    fn features_are_correlated_with_a_numeric_field() {
        let mut session = loaded();
        // "Depth" is the only field with numbers.
        assert_eq!(session.numeric_fields(), ["Depth"]);
        assert_eq!(session.correlation.field.as_deref(), Some("Depth"));
        session.profile_level = "Genus".to_string();
        session.invalidate_all();
        session.run_to_completion(Mode::Correlation);
        let (profile, results) = session.correlation.output.as_ref().unwrap();
        // C3 has no number.
        assert_eq!(profile.values.len(), 8);
        assert_eq!(profile.without_value, 1);
        assert_eq!(results.rows.len(), 3);
        assert_eq!(results.analysis.method, CorrelationMethod::Spearman);
        assert!(results.rows.iter().all(|r| r.coefficient.abs() <= 1.0));
        let before: Vec<f64> = results.rows.iter().map(|r| r.coefficient).collect();

        // The filters, the correction and the method.
        session.correlation.filter.min_coefficient = Some(2.0);
        assert_eq!(session.feature_counts(Mode::Correlation), (3, 0));
        session.correlation.filter.min_coefficient = None;
        session.set_correction(Mode::Correlation, Correction::Bonferroni);
        let (_, results) = session.correlation.output.as_ref().unwrap();
        assert!(results
            .rows
            .iter()
            .all(|r| r.p_value_corrected >= r.p_value));
        session.correlation.analysis.method = CorrelationMethod::Pearson;
        session.invalidate(Mode::Correlation);
        session.run_to_completion(Mode::Correlation);
        let (_, results) = session.correlation.output.as_ref().unwrap();
        let after: Vec<f64> = results.rows.iter().map(|r| r.coefficient).collect();
        assert_ne!(before, after);

        // Switching samples off re-runs the analysis; with too few left it
        // says so.
        let most: Vec<String> = ["A1", "A2", "A3", "B1", "B2", "B3"]
            .map(String::from)
            .to_vec();
        session.run_to_completion(Mode::Correlation);
        assert!(session.correlation.output.is_some());
        session.set_samples_active(&most, false);
        session.run_to_completion(Mode::Correlation);
        assert!(session
            .error(Mode::Correlation)
            .unwrap()
            .contains("at least three"));
        // The earlier results are not left on show beside the error.
        assert!(session.correlation.output.is_none());
    }

    #[test]
    fn the_transform_changes_p_values_only() {
        use stamp_core::stats::results::Transform;
        let mut session = loaded();
        session.profile_level = "Genus".to_string();
        session.invalidate_all();
        session.run_to_completion(Mode::TwoGroups);
        session.run_to_completion(Mode::MultipleGroups);
        let plain: Vec<(String, f64, f64)> = session
            .groups
            .output
            .as_ref()
            .unwrap()
            .1
            .rows
            .iter()
            .map(|r| (r.feature.clone(), r.p_value, r.effect_size))
            .collect();
        let multi_plain: Vec<f64> = session
            .multi
            .output
            .as_ref()
            .unwrap()
            .1
            .rows
            .iter()
            .map(|r| r.p_value)
            .collect();
        session.groups.analysis.transform = Transform::CentredLogRatio;
        session.multi.analysis.transform = Transform::CentredLogRatio;
        session.invalidate_all();
        session.run_to_completion(Mode::TwoGroups);
        session.run_to_completion(Mode::MultipleGroups);
        let (_, results) = session.groups.output.as_ref().unwrap();
        let mut changed = false;
        for (feature, p, effect) in &plain {
            let row = results.rows.iter().find(|r| r.feature == *feature).unwrap();
            assert_eq!(row.effect_size, *effect);
            changed |= row.p_value != *p;
        }
        assert!(changed);
        let multi: Vec<f64> = session
            .multi
            .output
            .as_ref()
            .unwrap()
            .1
            .rows
            .iter()
            .map(|r| r.p_value)
            .collect();
        assert_ne!(multi, multi_plain);
    }
}
