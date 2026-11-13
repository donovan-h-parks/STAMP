//! The state of an analysis session: the loaded data, the chosen settings and
//! the results. This module has no user-interface code, so it can be tested
//! on its own.

use std::sync::Arc;

use stamp_core::stats::ci::SampleCi;
use stamp_core::stats::correction::Correction;
use stamp_core::stats::multi_group::{MultiGroupEffectSize, MultiGroupTest};
use stamp_core::stats::post_hoc::{PostHocResult, PostHocTest};
use stamp_core::stats::results::{
    run_group_analysis, run_multi_group_analysis, run_post_hoc, run_sample_analysis, GroupAnalysis,
    GroupFilter, GroupResults, GroupRow, MultiGroupAnalysis, MultiGroupFilter, MultiGroupResults,
    MultiGroupRow, SampleAnalysis, SampleFilter, SampleResults, SampleRow,
};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;
use stamp_core::stats::{rng_from_seed, StatsSettings, TestType};
use stamp_core::{
    GroupProfile, Groups, Metadata, MultiGroupProfile, ProfileTree, SampleProfile, SecondGroup,
    Unclassified,
};

use crate::jobs::Job;

/// The three kinds of analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    MultipleGroups,
    TwoGroups,
    TwoSamples,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::MultipleGroups, Mode::TwoGroups, Mode::TwoSamples];

    pub fn name(self) -> &'static str {
        match self {
            Mode::MultipleGroups => "Multiple groups",
            Mode::TwoGroups => "Two groups",
            Mode::TwoSamples => "Two samples",
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
    pub analysis: GroupAnalysis,
    pub filter: GroupFilter,
    pub output: Option<(GroupProfile, GroupResults)>,
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
    /// The most recent error, shown until the next successful analysis.
    pub error: Option<String>,

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
    /// Feature shown by the single-feature plots.
    pub selected_feature: Option<String>,

    /// Which analyses are out of date.
    dirty: [bool; 3],
    jobs: [Option<Job<Outcome>>; 3],
    /// Increases whenever results change, so that views know to refresh.
    pub generation: u64,
}

fn mode_index(mode: Mode) -> usize {
    match mode {
        Mode::MultipleGroups => 0,
        Mode::TwoGroups => 1,
        Mode::TwoSamples => 2,
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
            error: None,
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
                    test: SampleTest::Fisher,
                    test_type: TestType::TwoSided,
                    ci_method: SampleCi::NewcombeWilson,
                    coverage: 0.95,
                    correction: Correction::None,
                },
                filter: SampleFilter::default(),
                output: None,
            },
            groups: GroupState {
                group1: String::new(),
                group2: None,
                analysis: GroupAnalysis {
                    test: GroupTest::Welch,
                    test_type: TestType::TwoSided,
                    coverage: 0.95,
                    correction: Correction::None,
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
                },
                filter: MultiGroupFilter::default(),
                post_hoc_test: PostHocTest::TukeyKramer,
                post_hoc_coverage: 0.95,
                output: None,
            },
            selected_feature: None,
            dirty: [false; 3],
            jobs: [None, None, None],
            generation: 0,
        }
    }
}

impl Session {
    /// Load a profile, replacing any current data.
    pub fn load_profile(&mut self, name: &str, text: &str) -> Result<(), String> {
        let tree = ProfileTree::read(text).map_err(|e| e.to_string())?;
        *self = Session {
            stats: self.stats,
            seed: self.seed,
            ..Session::default()
        };
        self.profile_name = name.to_string();
        self.profile_level = tree.hierarchy_headings()[0].clone();
        let samples = tree.sample_names();
        self.samples.sample1 = samples[0].clone();
        self.samples.sample2 = samples[1].clone();
        self.tree = Some(Arc::new(tree));
        self.invalidate_all();
        Ok(())
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
        match field {
            Some(field) => self.set_group_field(&field),
            None => self.group_field = None,
        }
        Ok(())
    }

    /// Group the samples by another metadata field.
    pub fn set_group_field(&mut self, field: &str) {
        self.group_field = Some(field.to_string());
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

    /// The samples grouped by the chosen metadata field.
    pub fn grouping(&self) -> Option<Groups> {
        self.metadata
            .as_ref()?
            .groups(self.group_field.as_deref()?)
            .ok()
    }

    /// Mark one analysis as needing to be run again.
    pub fn invalidate(&mut self, mode: Mode) {
        self.dirty[mode_index(mode)] = true;
    }

    /// Mark every analysis as out of date, after a change to the profile
    /// level or another shared setting.
    pub fn invalidate_all(&mut self) {
        self.dirty = [true; 3];
    }

    /// True while an analysis of this kind is running.
    pub fn is_running(&self, mode: Mode) -> bool {
        self.jobs[mode_index(mode)].is_some()
    }

    /// Progress of the running analysis, from 0 to 1.
    pub fn progress(&self, mode: Mode) -> Option<f32> {
        self.jobs[mode_index(mode)].as_ref().map(Job::fraction)
    }

    pub fn cancel(&mut self, mode: Mode) {
        self.jobs[mode_index(mode)] = None;
    }

    /// Start any analysis that is out of date for `mode` and collect finished
    /// results. Returns true if work is still in progress, so the caller
    /// should check again soon.
    pub fn update(&mut self, mode: Mode) -> bool {
        let index = mode_index(mode);
        if self.dirty[index] {
            self.dirty[index] = false;
            // Dropping a running job cancels it.
            self.jobs[index] = self.start(mode);
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
                Err(message) => {
                    self.error = Some(message);
                    return false;
                }
            }
            self.error = None;
            self.keep_selection_valid(mode);
        }
        false
    }

    /// Run the analysis of `mode` to completion. For tests and scripts.
    pub fn run_to_completion(&mut self, mode: Mode) {
        while self.update(mode) {
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
                Some(Job::spawn(move |progress| {
                    let profile = tree
                        .sample_profile(&s1, &s2, parent.as_deref(), &level, unclassified)
                        .map_err(|e| e.to_string())?;
                    let results = run_sample_analysis(
                        &profile,
                        analysis,
                        &stats,
                        &mut rng_from_seed(seed),
                        &mut |d, t| progress.report(d, t),
                    );
                    Ok(Finished::Samples(results.map(|r| (profile, r))))
                }))
            }
            Mode::TwoGroups => {
                let Some(groups) = self.grouping() else {
                    self.groups.output = None;
                    return None;
                };
                let (g1, g2) = (self.groups.group1.clone(), self.groups.group2.clone());
                let analysis = self.groups.analysis;
                Some(Job::spawn(move |progress| {
                    let second = match &g2 {
                        Some(name) => SecondGroup::Named(name),
                        None => SecondGroup::AllOtherSamples,
                    };
                    let profile = tree
                        .group_profile(
                            &groups,
                            &g1,
                            second,
                            parent.as_deref(),
                            &level,
                            unclassified,
                        )
                        .map_err(|e| e.to_string())?;
                    if profile.samples_in_group1().is_empty()
                        || profile.samples_in_group2().is_empty()
                    {
                        return Err("Both groups need at least one sample.".to_string());
                    }
                    let results = run_group_analysis(
                        &profile,
                        analysis,
                        &stats,
                        &mut rng_from_seed(seed),
                        &mut |d, t| progress.report(d, t),
                    );
                    Ok(Finished::Groups(results.map(|r| (profile, r))))
                }))
            }
            Mode::MultipleGroups => {
                let Some(groups) = self.grouping() else {
                    self.multi.output = None;
                    return None;
                };
                let active = self.multi.active_groups.clone();
                let analysis = self.multi.analysis;
                if active.len() < 2 {
                    self.multi.output = None;
                    self.error = Some("Select at least two groups.".to_string());
                    return None;
                }
                Some(Job::spawn(move |progress| {
                    let names: Vec<&str> = active.iter().map(String::as_str).collect();
                    let profile = tree
                        .multi_group_profile(
                            &groups,
                            Some(&names),
                            parent.as_deref(),
                            &level,
                            unclassified,
                        )
                        .map_err(|e| e.to_string())?;
                    let results = run_multi_group_analysis(
                        &profile,
                        analysis,
                        &mut rng_from_seed(seed),
                        &mut |d, t| progress.report(d, t),
                    );
                    Ok(Finished::Multi(results.map(|r| (profile, r))))
                }))
            }
        }
    }

    /// Keep the selected feature if it still exists; otherwise select the
    /// most significant one.
    fn keep_selection_valid(&mut self, mode: Mode) {
        let features: Vec<&str> = match mode {
            Mode::TwoSamples => self
                .sample_rows()
                .iter()
                .map(|r| r.feature.as_str())
                .collect(),
            Mode::TwoGroups => self
                .group_rows()
                .iter()
                .map(|r| r.feature.as_str())
                .collect(),
            Mode::MultipleGroups => self
                .multi_rows()
                .iter()
                .map(|r| r.feature.as_str())
                .collect(),
        };
        let still_there = self
            .selected_feature
            .as_deref()
            .is_some_and(|f| features.contains(&f));
        if !still_there {
            self.selected_feature = features.first().map(|f| f.to_string());
        }
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
        }
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
    const META: &str = "Id\tGroup\tSite\nA1\tA\tgut\nA2\tA\tgut\nA3\tA\tskin\nB1\tB\tgut\nB2\tB\tskin\nB3\tB\tskin\nC1\tC\tgut\nC2\tC\tgut\nC3\tC\tskin\n";

    fn loaded() -> Session {
        let mut session = Session::default();
        session.load_profile("test.spf", SPF).unwrap();
        session.load_metadata("test.tsv", META).unwrap();
        session
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
            assert!(session.error.is_none(), "{:?}", session.error);
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
            session.error.as_deref(),
            Some("Select at least two groups.")
        );
        assert!(session.multi.output.is_none());

        session.samples.sample2 = "missing".to_string();
        session.invalidate(Mode::TwoSamples);
        session.run_to_completion(Mode::TwoSamples);
        assert_eq!(session.error.as_deref(), Some("Unknown sample 'missing'."));

        let mut empty = Session::default();
        assert!(empty.load_metadata("m", META).is_err());
        assert!(empty.load_profile("bad", "only one line\n").is_err());
        assert!(!empty.update(Mode::TwoSamples));
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
}
