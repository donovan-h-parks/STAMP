//! The analysis wizard: a few questions that lead from what someone wants
//! to know to the analysis, test and plots that answer it. This module is
//! the questions, which answers the loaded data allows, and what each path
//! ends on; it has no user interface code (that is `wizard_ui.rs`), so it
//! is tested on its own. The flow chart it follows is in
//! `docs/WIZARD_PLAN.md`.

use stamp_core::stats::correction::Correction;
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::post_hoc::PostHocTest;
use stamp_core::stats::results::{SpreadSummary, Transform};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::two_sample::SampleTest;

use crate::plots::{PlotKind, PlotSettings};
use crate::session::{Mode, Session};

/// First question: what is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Comparison {
    TwoSamples,
    TwoGroups,
    MultipleGroups,
    Variable,
}

/// Second question: what the user wants to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Question {
    Differ,
    Follow,
    Together,
    Communities,
    Diversity,
    MadeOf,
    Depth,
}

/// Third question, for features that differ between groups: are the
/// groups made of the same subjects?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Design {
    Independent,
    Matched,
}

/// Fourth question, for features that differ between groups: what the
/// data look like, or what kind of difference is of interest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Data {
    Typical,
    SmallOrExtreme,
    MostlyAbsent,
    Presence,
    Dominated,
}

/// The step the wizard is at, in the order the steps are taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Comparison,
    Question,
    Design,
    Data,
    Result,
}

/// One of the wizard's choices, of any step: what a card stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Choice {
    Comparison(Comparison),
    Question(Question),
    Design(Design),
    Data(Data),
}

impl Comparison {
    pub const ALL: [Comparison; 4] = [
        Comparison::TwoSamples,
        Comparison::TwoGroups,
        Comparison::MultipleGroups,
        Comparison::Variable,
    ];

    /// The analysis of the main window that makes this comparison.
    pub fn mode(self) -> Mode {
        match self {
            Comparison::TwoSamples => Mode::TwoSamples,
            Comparison::TwoGroups => Mode::TwoGroups,
            Comparison::MultipleGroups => Mode::MultipleGroups,
            Comparison::Variable => Mode::Correlation,
        }
    }

    fn is_groups(self) -> bool {
        matches!(self, Comparison::TwoGroups | Comparison::MultipleGroups)
    }
}

impl Question {
    pub const ALL: [Question; 7] = [
        Question::Differ,
        Question::Follow,
        Question::Together,
        Question::Communities,
        Question::Diversity,
        Question::MadeOf,
        Question::Depth,
    ];
}

impl Design {
    pub const ALL: [Design; 2] = [Design::Independent, Design::Matched];
}

impl Data {
    pub const ALL: [Data; 5] = [
        Data::Typical,
        Data::SmallOrExtreme,
        Data::MostlyAbsent,
        Data::Presence,
        Data::Dominated,
    ];
}

impl Step {
    /// The question a step asks.
    pub fn heading(self) -> &'static str {
        match self {
            Step::Comparison => "What are you comparing?",
            Step::Question => "What is your research question?",
            Step::Design => "Were the same subjects sampled in every group?",
            Step::Data => "What kind of difference, in what kind of data?",
            Step::Result => "Recommended analysis",
        }
    }

    /// A word for the step in the trail of answers.
    pub fn name(self) -> &'static str {
        match self {
            Step::Comparison => "Comparison",
            Step::Question => "Question",
            Step::Design => "Design",
            Step::Data => "Data",
            Step::Result => "Result",
        }
    }
}

impl Choice {
    /// The name on the card.
    pub fn title(self) -> &'static str {
        match self {
            Choice::Comparison(c) => match c {
                Comparison::TwoSamples => "Two samples",
                Comparison::TwoGroups => "Two groups",
                Comparison::MultipleGroups => "Multiple groups",
                Comparison::Variable => "Samples along a measured variable",
            },
            Choice::Question(q) => match q {
                Question::Differ => "Which features differ?",
                Question::Follow => "Which features rise or fall with the variable?",
                Question::Together => "Which features occur together?",
                Question::Communities => "Do the communities differ as a whole?",
                Question::Diversity => "Does diversity differ?",
                Question::MadeOf => "What are the samples made of?",
                Question::Depth => "Were the samples sequenced deeply enough?",
            },
            Choice::Design(d) => match d {
                Design::Independent => "No: different subjects in each group",
                Design::Matched => "Yes: the same subjects in every group",
            },
            Choice::Data(d) => match d {
                Data::Typical => "More or less abundant: most data",
                Data::SmallOrExtreme => "More or less abundant: small groups or extreme samples",
                Data::MostlyAbsent => "More or less abundant: features absent from most samples",
                Data::Presence => "Found in more samples of one group",
                Data::Dominated => "A few features dominate every sample",
            },
        }
    }

    /// One line beneath the name.
    pub fn summary(self) -> &'static str {
        match self {
            Choice::Comparison(c) => match c {
                Comparison::TwoSamples => "One sample of each condition, without replicates.",
                Comparison::TwoGroups => "Several samples of each of two conditions.",
                Comparison::MultipleGroups => {
                    "Several samples of each of three or more conditions."
                }
                Comparison::Variable => "Each sample has a number: pH, age, depth, dose.",
            },
            Choice::Question(q) => match q {
                Question::Differ => "Taxa or functions that are more abundant on one side.",
                Question::Follow => "Features whose abundance follows the number.",
                Question::Together => "Features that are abundant in the same samples.",
                Question::Communities => "Overall composition, all features at once.",
                Question::Diversity => "How many features a sample has, and how evenly.",
                Question::MadeOf => "A description: no test, plots that show the data.",
                Question::Depth => "Would more sequences have found more features?",
            },
            Choice::Design(d) => match d {
                Design::Independent => "Patients and healthy controls; two different lakes.",
                Design::Matched => "Before and after treatment; the same sites each season.",
            },
            Choice::Data(d) => match d {
                Data::Typical => "About ten or more samples in each group.",
                Data::SmallOrExtreme => "Few samples, or one or two far from the rest.",
                Data::MostlyAbsent => "Rare features, zero in most samples.",
                Data::Presence => "Present or absent, whatever the abundance.",
                Data::Dominated => "One or two features hold most of the sequences.",
            },
        }
    }

    /// The longer explanation behind the "i" button.
    pub fn info(self) -> &'static str {
        match self {
            Choice::Comparison(c) => match c {
                Comparison::TwoSamples => {
                    "Choose this when you have a single sample of each condition: one \
                     metagenome from a healthy gut and one from a diseased gut, say. STAMP \
                     can then tell whether a feature makes up a different share of the \
                     sequences of the two samples, allowing for how many sequences were \
                     read. It cannot tell whether the two conditions differ, because \
                     without replicates there is no way to know how much two samples of \
                     the same condition would differ anyway. Treat the result as a \
                     description of these two samples."
                }
                Comparison::TwoGroups => {
                    "Choose this when you have replicate samples of two conditions: ten \
                     patients and ten controls, or six samples from each of two sites. \
                     The groups come from a column of the metadata file (the group \
                     field); each of its values is a group. The variation among the \
                     samples of a group tells STAMP how large a difference between the \
                     groups must be before it is unlikely to be chance."
                }
                Comparison::MultipleGroups => {
                    "Choose this when the group field has three or more values: three \
                     diets, four seasons, five body sites. Each feature is first tested \
                     for any difference among the groups; a second (post-hoc) test then \
                     says which pairs of groups differ. Comparing every pair with a \
                     two-group test instead would find too many differences by chance."
                }
                Comparison::Variable => {
                    "Choose this when the interesting thing about a sample is a number \
                     rather than a category: the pH of a soil, the age of a person, the \
                     depth of a water sample. The number comes from a column of the \
                     metadata file. Instead of comparing groups, STAMP asks whether \
                     features rise or fall as the number rises."
                }
            },
            Choice::Question(q) => match q {
                Question::Differ => {
                    "The most common question: which taxa or functions are more abundant \
                     in one sample or group than in another? Every feature is tested on \
                     its own, so with hundreds of features some will look different by \
                     chance; the recommended analysis corrects for that. The answer is a \
                     list of features, each with the size of its difference, a \
                     confidence interval and a p-value."
                }
                Question::Follow => {
                    "Which features become more or less abundant as the measured number \
                     rises? For example, which genera increase with the pH of the soil. \
                     Each feature gets a correlation between -1 (falls steadily) and +1 \
                     (rises steadily), and a p-value."
                }
                Question::Together => {
                    "Which features tend to be abundant in the same samples, and which \
                     replace each other? This compares features with each other rather \
                     than samples or groups. A correlation between two features is a \
                     hint, not proof that they interact: both may simply respond to the \
                     same conditions."
                }
                Question::Communities => {
                    "Instead of one feature at a time, this looks at each sample as a \
                     whole: are the samples of one group more like each other than like \
                     the samples of another group? Samples are placed on a map so that \
                     similar samples lie close together, and a test says whether the \
                     groups occupy different places. Often the first thing to look at in \
                     a new data set."
                }
                Question::Diversity => {
                    "Diversity within a sample (alpha diversity): how many different \
                     features it holds, and whether a few dominate or many share the \
                     sequences evenly. For example, is the gut community less diverse \
                     after antibiotics? Each sample gets one number, and the groups are \
                     compared by those numbers."
                }
                Question::MadeOf => {
                    "Sometimes the aim is to describe, not to test: which features are \
                     most abundant, how samples resemble each other, which features are \
                     shared between groups, where the samples were taken. These plots \
                     show the data as they are and involve no p-value."
                }
                Question::Depth => {
                    "A sample that was sequenced too shallowly misses its rare features, \
                     which can make it look less diverse than it is. Rarefaction curves \
                     show how many features would have been found with fewer sequences: \
                     a curve that has levelled off means that more sequencing would have \
                     added little."
                }
            },
            Choice::Design(d) => match d {
                Design::Independent => {
                    "The samples of one group have nothing to do with particular samples \
                     of another: different people, different animals, different sites. \
                     This is the usual design, and the one to choose if you are unsure."
                }
                Design::Matched => {
                    "Each subject (a person, an animal, a site) was sampled once in every \
                     group: before and after a treatment, or at each of several time \
                     points. A test for matched samples compares each subject only with \
                     itself, so that the differences between subjects, which are often \
                     large, do not hide a consistent change. STAMP needs a column of the \
                     metadata that names the subject of each sample."
                }
            },
            Choice::Data(d) => match d {
                Data::Typical => {
                    "With roughly ten or more samples in each group, tests that compare \
                     the average abundance (t-tests, analysis of variance) are reliable \
                     even when the values are not bell-shaped. They are also the most \
                     sensitive, and give a difference with a confidence interval that is \
                     easy to report."
                }
                Data::SmallOrExtreme => {
                    "With few samples, or when one or two samples have a far higher \
                     abundance than the rest, an average can be carried by those \
                     samples. Rank-based tests use only the order of the values, so that \
                     an extreme sample counts for no more than any other. They are a \
                     little less sensitive when the data are well behaved."
                }
                Data::MostlyAbsent => {
                    "Rare features are zero in most samples and present in a few, which \
                     suits neither averages nor ranks well. White's non-parametric test \
                     was designed for such sparse data from two groups: it shuffles the \
                     samples between the groups to find its p-values, and falls back on \
                     counts for the sparsest features."
                }
                Data::Presence => {
                    "A different question from abundance: is the feature found in more \
                     of the samples of one group? For example, a pathogen present in 8 of \
                     10 patients and 1 of 10 controls. Only whether each sample has the \
                     feature counts, not how much of it. Useful for features that are \
                     absent from many samples."
                }
                Data::Dominated => {
                    "Abundances are shares of a whole, so when one abundant feature \
                     rises, every other share must fall even if nothing else changed. \
                     When a few features hold most of the sequences this can create \
                     differences that are not real. The centred log-ratio transform \
                     expresses each feature relative to the typical feature of its \
                     sample instead, which removes the effect."
                }
            },
        }
    }
}

/// Whether a choice can be made with the loaded data, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    Yes,
    No(String),
}

impl Availability {
    pub fn is_available(&self) -> bool {
        *self == Availability::Yes
    }

    fn no(reason: impl Into<String>) -> Availability {
        Availability::No(reason.into())
    }
}

/// What the wizard needs to know about the loaded data.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Facts {
    /// Samples in the profile; zero if none is loaded.
    pub samples: usize,
    pub has_metadata: bool,
    pub group_field: Option<String>,
    /// The groups ticked in the group list that have samples, with their
    /// number of samples.
    pub groups: Vec<(String, usize)>,
    /// The two groups of the two-group analysis.
    pub two_groups: Option<(String, String)>,
    /// The two samples of the two-sample analysis.
    pub two_samples: Option<(String, String)>,
    pub numeric_fields: Vec<String>,
    /// The numeric field of the correlation analysis.
    pub variable: Option<String>,
    /// Metadata fields that match the samples of the two groups in pairs,
    /// with the number of pairs (at least two).
    pub pair_fields: Vec<(String, usize)>,
    /// Metadata fields that name subjects with one sample in every ticked
    /// group, with the number of subjects (at least two).
    pub subject_fields: Vec<(String, usize)>,
    /// True if every value of the profile is a whole number of sequences.
    pub whole_counts: bool,
    /// True if the metadata has a latitude and a longitude.
    pub has_coordinates: bool,
    /// How many features differ in spread between the ticked groups by the
    /// Brown-Forsythe test, on proportions and on centred log-ratios; with
    /// three or more groups.
    pub spread: Option<SpreadSummary>,
    pub spread_clr: Option<SpreadSummary>,
}

impl Facts {
    /// What identifies the facts of a session: they are found again only
    /// when this changes.
    pub fn key(session: &Session) -> String {
        format!(
            "{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            session.generation,
            session.group_field,
            session.multi.active_groups,
            session.groups.group1,
            session.groups.group2,
            (&session.samples.sample1, &session.samples.sample2),
            session.correlation.field,
            (
                &session.profile_level,
                &session.parent_level,
                session.unclassified
            ),
        )
    }

    pub fn of(session: &Session, settings: &PlotSettings) -> Facts {
        let Some(tree) = &session.tree else {
            return Facts::default();
        };
        let mut facts = Facts {
            samples: tree.num_samples(),
            has_metadata: session.metadata.is_some(),
            group_field: session.group_field.clone(),
            numeric_fields: session.numeric_fields(),
            variable: session.correlation.field.clone(),
            whole_counts: tree
                .rows()
                .all(|(_, counts)| counts.iter().all(|v| *v >= 0.0 && v.fract() == 0.0)),
            ..Facts::default()
        };
        if facts.samples >= 2 {
            facts.two_samples = Some((
                session.samples.sample1.clone(),
                session.samples.sample2.clone(),
            ));
        }
        let Some(metadata) = &session.metadata else {
            return facts;
        };
        let (lat, lon) = crate::plots::map_fields(metadata, settings);
        facts.has_coordinates = lat.is_some() && lon.is_some();
        let Some(grouping) = session.grouping() else {
            return facts;
        };
        facts.groups = grouping
            .names()
            .filter(|name| session.multi.active_groups.iter().any(|a| a == name))
            .filter_map(|name| {
                let size = grouping.samples(name).map_or(0, <[String]>::len);
                (size > 0).then(|| (name.to_string(), size))
            })
            .collect();
        // The two groups of the two-group analysis, or the first two if it
        // compares one group with all other samples.
        let first = session.groups.group1.clone();
        let second = session.groups.group2.clone().or_else(|| {
            facts
                .groups
                .iter()
                .map(|(name, _)| name)
                .find(|name| **name != first)
                .cloned()
        });
        if let Some(second) = second {
            if facts.groups.iter().any(|(name, _)| *name == first) {
                facts.two_groups = Some((first, second));
            }
        }
        let names: Vec<String> = facts.groups.iter().map(|(name, _)| name.clone()).collect();
        if names.len() >= 3 {
            let active: Vec<&str> = names.iter().map(String::as_str).collect();
            if let Ok(profile) = tree.multi_group_profile(
                &grouping,
                Some(&active),
                session.parent_level.as_deref(),
                &session.profile_level,
                session.unclassified,
            ) {
                facts.spread = Some(SpreadSummary::of_profile(&profile, Transform::None));
                facts.spread_clr = Some(SpreadSummary::of_profile(
                    &profile,
                    Transform::CentredLogRatio,
                ));
            }
        }
        for field in metadata.fields() {
            if Some(field) == session.group_field.as_ref() {
                continue;
            }
            if let Some((first, second)) = &facts.two_groups {
                if let Ok(pairing) = metadata.paired_groups(&grouping, first, second, field) {
                    if pairing.pairs >= 2 {
                        facts.pair_fields.push((field.clone(), pairing.pairs));
                    }
                }
            }
            if names.len() >= 3 {
                if let Ok(matching) = metadata.matched_groups(&grouping, &names, field) {
                    if matching.pairs >= 2 {
                        facts.subject_fields.push((field.clone(), matching.pairs));
                    }
                }
            }
        }
        facts
    }

    /// Samples in the smallest of the groups a comparison is between.
    fn smallest_group(&self, comparison: Comparison) -> Option<usize> {
        match comparison {
            Comparison::TwoGroups => {
                let (first, second) = self.two_groups.as_ref()?;
                self.groups
                    .iter()
                    .filter(|(name, _)| name == first || name == second)
                    .map(|(_, size)| *size)
                    .min()
            }
            Comparison::MultipleGroups => self.groups.iter().map(|(_, size)| *size).min(),
            _ => None,
        }
    }

    /// The fields that can match the subjects of a comparison's groups.
    pub fn matching_fields(&self, comparison: Comparison) -> &[(String, usize)] {
        match comparison {
            Comparison::TwoGroups => &self.pair_fields,
            Comparison::MultipleGroups => &self.subject_fields,
            _ => &[],
        }
    }

    /// The number of groups with samples, as words ("1 group", "2 groups").
    fn group_count(&self) -> String {
        match self.groups.len() {
            1 => "1 group".to_string(),
            n => format!("{n} groups"),
        }
    }

    pub fn comparison(&self, comparison: Comparison) -> Availability {
        if self.samples == 0 {
            return Availability::no("No profile is loaded.");
        }
        let field = self.group_field.as_deref().unwrap_or("");
        match comparison {
            Comparison::TwoSamples if self.samples < 2 => {
                Availability::no("The profile has only one sample.")
            }
            Comparison::TwoSamples => Availability::Yes,
            Comparison::TwoGroups | Comparison::MultipleGroups if !self.has_metadata => {
                Availability::no("No metadata file is loaded.")
            }
            Comparison::TwoGroups if self.groups.len() < 2 => Availability::no(format!(
                "The group field \"{field}\" has {} with samples; two or more are needed.",
                self.group_count()
            )),
            Comparison::MultipleGroups if self.groups.len() < 3 => Availability::no(format!(
                "The group field \"{field}\" has {} with samples; three or more are needed.",
                self.group_count()
            )),
            Comparison::TwoGroups | Comparison::MultipleGroups => Availability::Yes,
            Comparison::Variable if !self.has_metadata => {
                Availability::no("No metadata file is loaded.")
            }
            Comparison::Variable if self.numeric_fields.is_empty() => {
                Availability::no("No metadata field holds a number for three or more samples.")
            }
            Comparison::Variable => Availability::Yes,
        }
    }

    pub fn question(&self, comparison: Comparison, question: Question) -> Availability {
        use Comparison::*;
        match (question, comparison) {
            (Question::Differ, Variable) => {
                Availability::no("Samples along a variable form no groups to compare.")
            }
            (Question::Follow, Variable) => Availability::Yes,
            (Question::Follow, _) => {
                Availability::no("Only for samples along a measured variable.")
            }
            (Question::Together, TwoSamples) => {
                Availability::no("Two samples cannot show which features occur together.")
            }
            (Question::MadeOf, Variable) if !self.has_coordinates => {
                Availability::no("Samples along a variable form no groups to describe.")
            }
            (Question::Depth, Variable) => {
                Availability::no("Samples along a variable form no groups to compare.")
            }
            (Question::Depth, TwoGroups | MultipleGroups) if !self.whole_counts => {
                Availability::no("The profile holds fractional values, not counts of sequences.")
            }
            _ => Availability::Yes,
        }
    }

    pub fn design(&self, comparison: Comparison, design: Design) -> Availability {
        match design {
            Design::Independent => Availability::Yes,
            Design::Matched if self.matching_fields(comparison).is_empty() => {
                Availability::no(match comparison {
                    Comparison::TwoGroups => {
                        "No metadata field pairs the samples of the two groups."
                    }
                    _ => "No metadata field names subjects with one sample in every group.",
                })
            }
            Design::Matched => Availability::Yes,
        }
    }

    pub fn data(&self, comparison: Comparison, design: Design, data: Data) -> Availability {
        match (data, comparison, design) {
            (Data::MostlyAbsent, Comparison::TwoGroups, Design::Independent) => Availability::Yes,
            (Data::MostlyAbsent, _, Design::Matched) => {
                Availability::no("The test for sparse features compares independent groups.")
            }
            (Data::MostlyAbsent, _, _) => {
                Availability::no("The test for sparse features compares two groups.")
            }
            (Data::Presence, Comparison::MultipleGroups, Design::Matched) => {
                Availability::no("STAMP has no test of presence for three or more matched groups.")
            }
            _ => Availability::Yes,
        }
    }

    /// The answer about the data that the loaded data suggest, with the
    /// reason in a few words.
    pub fn suggested(&self, comparison: Comparison, design: Design) -> Option<(Data, String)> {
        let size = match design {
            Design::Independent => self.smallest_group(comparison)?,
            Design::Matched => self
                .matching_fields(comparison)
                .iter()
                .map(|(_, subjects)| *subjects)
                .max()?,
        };
        let what = match design {
            Design::Independent => "your smallest group has",
            Design::Matched => "your groups share",
        };
        let unit = match (design, size) {
            (Design::Independent, 1) => "sample",
            (Design::Independent, _) => "samples",
            (Design::Matched, 1) => "subject",
            (Design::Matched, _) => "subjects",
        };
        let data = if size < 10 {
            Data::SmallOrExtreme
        } else {
            Data::Typical
        };
        Some((data, format!("Suggested: {what} {size} {unit}")))
    }
}

/// What applying a recommendation changes in the session, beyond showing
/// its analysis and first plot.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Change {
    pub sample_test: Option<SampleTest>,
    pub group_test: Option<GroupTest>,
    pub multi_test: Option<(MultiGroupTest, PostHocTest)>,
    pub method: Option<CorrelationMethod>,
    pub transform: Option<Transform>,
    pub correction: Option<Correction>,
    /// The field that pairs or matches the samples.
    pub subject_field: Option<String>,
    /// The two named groups a paired test needs.
    pub two_groups: Option<(String, String)>,
    /// Join the pairs in the box plot.
    pub show_pairs: bool,
}

/// Where a path through the wizard ends.
#[derive(Debug, Clone, PartialEq)]
pub struct Recommendation {
    /// The test, or the plot where there is no test.
    pub heading: String,
    pub mode: Mode,
    /// What will be set, as "label: value" lines.
    pub settings: Vec<(&'static str, String)>,
    /// Why this is recommended, in a paragraph.
    pub why: String,
    /// The plots to look at, the first of which is shown, each with what
    /// it is for.
    pub plots: Vec<(PlotKind, &'static str)>,
    pub alternatives: Vec<&'static str>,
    /// What to report in a paper.
    pub report: Option<&'static str>,
    /// What the analysis cannot show.
    pub caution: Option<&'static str>,
    pub change: Change,
}

/// The answers given so far.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Wizard {
    pub comparison: Option<Comparison>,
    pub question: Option<Question>,
    pub design: Option<Design>,
    pub data: Option<Data>,
    /// The field that matches the subjects, chosen with a matched design.
    pub subject_field: Option<String>,
    /// The choice whose explanation is shown.
    pub info: Option<Choice>,
    /// True while the window shows the table of tests ("At a glance")
    /// and not the questions.
    pub glance: bool,
}

impl Wizard {
    /// True if the path asks about the design and the data: features that
    /// differ between groups.
    fn asks_about_groups(&self) -> bool {
        self.question == Some(Question::Differ)
            && self.comparison.is_some_and(Comparison::is_groups)
    }

    pub fn step(&self) -> Step {
        if self.comparison.is_none() {
            Step::Comparison
        } else if self.question.is_none() {
            Step::Question
        } else if self.asks_about_groups() && self.design.is_none() {
            Step::Design
        } else if self.asks_about_groups() && self.data.is_none() {
            Step::Data
        } else {
            Step::Result
        }
    }

    /// The steps of the path so far, the current one last.
    pub fn steps(&self) -> Vec<Step> {
        let mut steps = vec![Step::Comparison];
        let at = self.step();
        if at == Step::Comparison {
            return steps;
        }
        steps.push(Step::Question);
        if self.asks_about_groups() {
            steps.push(Step::Design);
            if self.design.is_some() {
                steps.push(Step::Data);
            }
        }
        if at == Step::Result {
            steps.push(Step::Result);
        }
        steps
    }

    /// The answer given at a step, for the trail of answers.
    pub fn answer(&self, step: Step) -> Option<Choice> {
        match step {
            Step::Comparison => self.comparison.map(Choice::Comparison),
            Step::Question => self.question.map(Choice::Question),
            Step::Design => self.design.map(Choice::Design),
            Step::Data => self.data.map(Choice::Data),
            Step::Result => None,
        }
    }

    /// The choices of a step, each with whether the data allow it.
    pub fn choices(&self, step: Step, facts: &Facts) -> Vec<(Choice, Availability)> {
        let comparison = self.comparison.unwrap_or(Comparison::TwoGroups);
        let design = self.design.unwrap_or(Design::Independent);
        match step {
            Step::Comparison => Comparison::ALL
                .into_iter()
                .map(|c| (Choice::Comparison(c), facts.comparison(c)))
                .collect(),
            Step::Question => Question::ALL
                .into_iter()
                .map(|q| (Choice::Question(q), facts.question(comparison, q)))
                .collect(),
            Step::Design => Design::ALL
                .into_iter()
                .map(|d| (Choice::Design(d), facts.design(comparison, d)))
                .collect(),
            Step::Data => Data::ALL
                .into_iter()
                .map(|d| (Choice::Data(d), facts.data(comparison, design, d)))
                .collect(),
            Step::Result => Vec::new(),
        }
    }

    /// Give an answer; the answers to later questions are forgotten, since
    /// they may no longer apply. Nothing changes if the data do not allow
    /// the choice.
    pub fn choose(&mut self, choice: Choice, facts: &Facts) -> bool {
        let step = match choice {
            Choice::Comparison(_) => Step::Comparison,
            Choice::Question(_) => Step::Question,
            Choice::Design(_) => Step::Design,
            Choice::Data(_) => Step::Data,
        };
        let allowed = self
            .choices(step, facts)
            .into_iter()
            .any(|(c, availability)| c == choice && availability.is_available());
        if !allowed {
            return false;
        }
        self.go_to(step);
        match choice {
            Choice::Comparison(c) => self.comparison = Some(c),
            Choice::Question(q) => self.question = Some(q),
            Choice::Design(d) => {
                self.design = Some(d);
                if d == Design::Matched {
                    let fields =
                        facts.matching_fields(self.comparison.unwrap_or(Comparison::TwoGroups));
                    self.subject_field = fields
                        .iter()
                        .max_by_key(|(_, subjects)| *subjects)
                        .map(|(field, _)| field.clone());
                }
            }
            Choice::Data(d) => self.data = Some(d),
        }
        self.info = None;
        true
    }

    /// Return to a step, forgetting its answer and those after it.
    pub fn go_to(&mut self, step: Step) {
        if step <= Step::Data {
            self.data = None;
        }
        if step <= Step::Design {
            self.design = None;
            self.subject_field = None;
        }
        if step <= Step::Question {
            self.question = None;
        }
        if step == Step::Comparison {
            self.comparison = None;
        }
        self.info = None;
    }

    /// Return to the step before the current one.
    pub fn back(&mut self) {
        let steps = self.steps();
        if let [.., previous, _] = steps.as_slice() {
            self.go_to(*previous);
        }
    }

    pub fn restart(&mut self) {
        *self = Wizard {
            glance: self.glance,
            ..Wizard::default()
        };
    }

    /// Give the answers that lead to a row of the "At a glance" table and
    /// show where they lead. Nothing changes if the data do not allow the
    /// row.
    pub fn follow(&mut self, row: &Row, facts: &Facts) -> bool {
        let Ok(comparison) = row.comparison(facts) else {
            return false;
        };
        let mut wizard = Wizard::default();
        let answers = [
            Some(Choice::Comparison(comparison)),
            Some(Choice::Question(row.question)),
            row.design.map(Choice::Design),
            row.data.map(Choice::Data),
        ];
        for answer in answers.into_iter().flatten() {
            if !wizard.choose(answer, facts) {
                return false;
            }
        }
        if wizard.step() != Step::Result {
            return false;
        }
        *self = wizard;
        true
    }

    /// Where the answers lead, once every question of the path is
    /// answered.
    pub fn recommendation(&self, facts: &Facts) -> Option<Recommendation> {
        if self.step() != Step::Result {
            return None;
        }
        let (comparison, question) = (self.comparison?, self.question?);
        Some(match question {
            Question::Differ => self.differing_features(comparison, facts)?,
            Question::Follow => follow(facts),
            Question::Together => together(comparison),
            Question::Communities => communities(comparison),
            Question::Diversity => diversity(comparison, facts),
            Question::MadeOf => made_of(comparison, facts),
            Question::Depth => depth(comparison, facts),
        })
    }

    fn differing_features(&self, comparison: Comparison, facts: &Facts) -> Option<Recommendation> {
        const FDR: &str = "Benjamini-Hochberg FDR";
        const REPORT: &str = "For each feature you report: the difference in mean proportion \
            with its confidence interval, and the corrected p-value (q-value). Name the test \
            and the correction.";
        let correction = Some(Correction::BenjaminiHochberg);
        if comparison == Comparison::TwoSamples {
            let (first, second) = facts.two_samples.clone()?;
            return Some(Recommendation {
                heading: SampleTest::GTestFisher.name().to_string(),
                mode: Mode::TwoSamples,
                settings: vec![
                    ("Samples", format!("{first} and {second}")),
                    (
                        "Statistical test",
                        SampleTest::GTestFisher.name().to_string(),
                    ),
                    ("Multiple test correction", FDR.to_string()),
                ],
                why: "With one sample of each condition, the evidence is the number of \
                    sequences assigned to each feature. This test compares each feature's \
                    share of the two samples, allowing for how many sequences were read, and \
                    switches to Fisher's exact test for features with few sequences, where \
                    the faster approximation is unreliable. Testing hundreds of features \
                    finds some differences by chance, so the p-values are corrected to keep \
                    false discoveries to about 5% of the features reported."
                    .to_string(),
                plots: vec![
                    (
                        PlotKind::ExtendedErrorBar,
                        "the features that differ, how much, and how sure",
                    ),
                    (
                        PlotKind::Scatter,
                        "every feature in one sample against the other",
                    ),
                    (
                        PlotKind::Volcano,
                        "size of difference against significance, all features",
                    ),
                    (
                        PlotKind::PValueHistogram,
                        "a check that some features truly differ",
                    ),
                ],
                alternatives: vec![
                    "Fisher's exact test for every feature: exact throughout, and slower.",
                ],
                report: Some(
                    "For each feature you report: the difference in proportions with its \
                     confidence interval, and the corrected p-value (q-value). Name the test \
                     and the correction.",
                ),
                caution: Some(
                    "Without replicates, a significant feature shows that these two samples \
                     differ, not that the conditions they came from do.",
                ),
                change: Change {
                    sample_test: Some(SampleTest::GTestFisher),
                    correction,
                    ..Change::default()
                },
            });
        }
        let (design, data) = (self.design?, self.data?);
        let clr = data == Data::Dominated;
        let transform = Some(if clr {
            Transform::CentredLogRatio
        } else {
            Transform::None
        });
        let clr_note = " The centred log-ratio transform is applied before the test, so that a \
            change in a dominant feature does not appear as a change in all the others; the \
            sizes of the differences are still shown as proportions.";
        if comparison == Comparison::TwoGroups {
            let (first, second) = facts.two_groups.clone()?;
            let test = match (design, data) {
                (Design::Independent, Data::Typical | Data::Dominated) => GroupTest::Welch,
                (Design::Independent, Data::SmallOrExtreme) => GroupTest::MannWhitney,
                (Design::Independent, Data::MostlyAbsent) => GroupTest::White,
                (Design::Independent, Data::Presence) => GroupTest::FisherPresence,
                (Design::Matched, Data::Typical | Data::Dominated) => GroupTest::PairedT,
                (Design::Matched, Data::SmallOrExtreme) => GroupTest::WilcoxonSignedRank,
                (Design::Matched, Data::Presence) => GroupTest::McNemar,
                (Design::Matched, Data::MostlyAbsent) => return None,
            };
            let mut why = match test {
                GroupTest::Welch => {
                    "Welch's t-test compares the mean abundance of each feature in the two \
                     groups, allowing each group its own spread. It is reliable when the \
                     groups differ in size or variability, and with about ten or more \
                     samples in each group it does not need bell-shaped data."
                }
                GroupTest::MannWhitney => {
                    "The Mann-Whitney U test ranks all the samples and asks whether those of \
                     one group tend to rank higher. Because it uses the order of the values \
                     and not the values themselves, one or two extreme samples cannot carry \
                     the result. With very small groups it cannot reach small p-values: with \
                     three samples in each group the smallest is 0.1."
                }
                GroupTest::White => {
                    "White's non-parametric t-test finds its p-values by shuffling the \
                     samples between the groups, so it assumes nothing about the shape of \
                     the data, and it handles features that are absent from most samples by \
                     pooling the counts of each group. It takes longer than the other tests."
                }
                GroupTest::FisherPresence => {
                    "Fisher's exact test of presence counts, in each group, the samples \
                     that have the feature at all, and asks whether the two shares differ. \
                     It is exact for any number of samples and ignores how abundant the \
                     feature is where it is found."
                }
                GroupTest::PairedT => {
                    "The paired t-test looks at the change within each subject and asks \
                     whether the changes average to zero. Comparing each subject only with \
                     itself removes the differences between subjects, so a small but \
                     consistent change can be found."
                }
                GroupTest::WilcoxonSignedRank => {
                    "The Wilcoxon signed-rank test looks at the change within each subject \
                     and asks whether the changes tend to go one way, using their order and \
                     not their size, so that one subject with a very large change cannot \
                     carry the result."
                }
                _ => {
                    "McNemar's test looks at the subjects in which the feature is present in \
                     only one of their two samples, and asks whether that is more often the \
                     sample of one group. Subjects with the feature in both samples, or in \
                     neither, say nothing about a change."
                }
            }
            .to_string();
            if clr {
                why.push_str(clr_note);
            }
            let mut settings = vec![
                ("Groups", format!("{first} and {second}")),
                ("Statistical test", test.name().to_string()),
            ];
            if design == Design::Matched {
                settings.push(("Pair field", self.subject_field.clone().unwrap_or_default()));
            }
            if clr {
                settings.push(("Transform", Transform::CentredLogRatio.name().to_string()));
            }
            settings.push(("Multiple test correction", FDR.to_string()));
            let mut plots = vec![
                (
                    PlotKind::ExtendedErrorBar,
                    "the features that differ, how much, and how sure",
                ),
                (
                    PlotKind::Volcano,
                    "size of difference against significance, all features",
                ),
                (
                    PlotKind::Box,
                    if design == Design::Matched {
                        "one feature in every sample, with each subject's pair joined"
                    } else {
                        "one feature in every sample: check that a result does not rest on a few"
                    },
                ),
                (
                    PlotKind::PValueHistogram,
                    "a check that some features truly differ",
                ),
            ];
            if test.is_presence() {
                plots.insert(
                    0,
                    (
                        PlotKind::PrevalenceAbundance,
                        "the share of each group's samples that have each feature",
                    ),
                );
            }
            if clr && design == Design::Independent {
                plots.push((
                    PlotKind::Ancom,
                    "a second method made for this problem, as confirmation",
                ));
            }
            let alternatives = match test {
                GroupTest::Welch => vec![
                    "Mann-Whitney U test, if a result seems to rest on one or two samples.",
                    "White's non-parametric t-test, for features absent from most samples.",
                ],
                GroupTest::MannWhitney => vec![
                    "White's non-parametric t-test, also free of assumptions about shape.",
                    "Welch's t-test alongside: a finding both tests agree on is robust.",
                ],
                GroupTest::White => vec![
                    "Fisher's exact test (presence/absence), to ask in how many samples a \
                     feature is found.",
                ],
                GroupTest::FisherPresence => {
                    vec!["Mann-Whitney U test, to compare abundance instead of presence."]
                }
                GroupTest::PairedT => {
                    vec!["Wilcoxon signed-rank test, with few subjects or extreme changes."]
                }
                GroupTest::WilcoxonSignedRank => {
                    vec!["Paired t-test, with about ten or more subjects and no extreme changes."]
                }
                _ => vec!["Wilcoxon signed-rank test, to compare abundance instead of presence."],
            };
            return Some(Recommendation {
                heading: test.name().to_string(),
                mode: Mode::TwoGroups,
                settings,
                why,
                plots,
                alternatives,
                report: Some(if test.is_presence() {
                    "For each feature you report: the number of samples of each group that \
                     have it, and the corrected p-value (q-value). Name the test and the \
                     correction."
                } else {
                    REPORT
                }),
                caution: None,
                change: Change {
                    group_test: Some(test),
                    transform,
                    correction,
                    subject_field: (design == Design::Matched)
                        .then(|| self.subject_field.clone())
                        .flatten(),
                    two_groups: Some((first, second)),
                    show_pairs: design == Design::Matched,
                    ..Change::default()
                },
            });
        }
        // Multiple groups.
        let sizes: Vec<usize> = facts.groups.iter().map(|(_, size)| *size).collect();
        let uneven = match (sizes.iter().min(), sizes.iter().max()) {
            (Some(&least), Some(&most)) => most >= 2 * least,
            _ => false,
        };
        // The Brown-Forsythe test of every feature, on the values the test
        // will see.
        let spread = if clr { facts.spread_clr } else { facts.spread }.filter(|s| s.tested > 0);
        let unequal_spread = spread.is_some_and(|s| s.is_unequal());
        let (test, post_hoc) = match (design, data) {
            (Design::Matched, Data::Presence | Data::MostlyAbsent) => return None,
            (Design::Matched, _) => (MultiGroupTest::Friedman, PostHocTest::WilcoxonHolm),
            (_, Data::MostlyAbsent) => return None,
            (_, Data::SmallOrExtreme) => (MultiGroupTest::KruskalWallis, PostHocTest::Dunn),
            (_, Data::Presence) => (MultiGroupTest::ChiSquarePresence, PostHocTest::Dunn),
            (_, Data::Typical | Data::Dominated) if uneven || unequal_spread => {
                (MultiGroupTest::WelchAnova, PostHocTest::GamesHowell)
            }
            _ => (MultiGroupTest::Anova, PostHocTest::TukeyKramer),
        };
        let mut why = match test {
            MultiGroupTest::Anova => {
                "Analysis of variance (ANOVA) asks, for each feature, whether the group \
                 means differ by more than the variation within the groups would explain. A \
                 significant result says that some groups differ, not which: the post-hoc \
                 test then compares every pair of groups for the feature you select."
            }
            MultiGroupTest::WelchAnova => {
                "Welch's analysis of variance asks, for each feature, whether the group \
                 means differ, weighing each group by how closely its samples agree, so that \
                 the groups need not be equally spread out. A significant result says that \
                 some groups differ, not which: the Games-Howell post-hoc test, which also \
                 allows each group its own spread, then compares every pair of groups for \
                 the feature you select."
            }
            MultiGroupTest::KruskalWallis => {
                "The Kruskal-Wallis test ranks all the samples and asks whether some groups \
                 tend to rank higher, so that one or two extreme samples cannot carry the \
                 result. A significant result says that some groups differ, not which: \
                 Dunn's test then compares every pair of groups for the feature you select."
            }
            MultiGroupTest::Friedman => {
                "The Friedman test ranks each subject's samples across the groups and asks \
                 whether some groups tend to rank higher. Comparing each subject only with \
                 itself removes the differences between subjects. A significant result says \
                 that some groups differ, not which: the Wilcoxon signed-rank test, with \
                 Holm's correction, then compares every pair of groups for the feature you \
                 select. STAMP has no matched test on means for three or more groups, so \
                 this is recommended for any number of subjects."
            }
            MultiGroupTest::ChiSquarePresence => {
                "The chi-square test of presence counts, in each group, the samples that \
                 have the feature at all, and asks whether the shares are the same in every \
                 group. It ignores how abundant the feature is where it is found. STAMP has \
                 no post-hoc test of presence: the post-hoc plot compares abundance, by \
                 Dunn's test."
            }
        }
        .to_string();
        // Why Welch's ANOVA, or why plain ANOVA is enough: said, not
        // decided silently.
        let spread_text = spread.map(|s| s.describe()).unwrap_or_default();
        match test {
            MultiGroupTest::WelchAnova => {
                why.push(' ');
                why.push_str(&match (uneven, unequal_spread) {
                    (true, true) => format!(
                        "Your groups differ at least twofold in size, and the spread of many \
                         features differs between them. {spread_text} So Welch's ANOVA and \
                         Games-Howell are chosen over ANOVA and Tukey-Kramer, which assume \
                         equal spread."
                    ),
                    (true, false) => "Your groups differ at least twofold in size, so Welch's \
                         ANOVA and Games-Howell are chosen over ANOVA and Tukey-Kramer: with \
                         groups of different sizes, unequal spread misleads those most."
                        .to_string(),
                    _ => format!(
                        "{spread_text} That is clearly more than chance, so Welch's ANOVA and \
                         Games-Howell are chosen over ANOVA and Tukey-Kramer, which assume \
                         equal spread."
                    ),
                });
            }
            MultiGroupTest::Anova if !spread_text.is_empty() => {
                why.push(' ');
                why.push_str(&spread_text);
                why.push_str(
                    " That is close to chance, so ANOVA's assumption of equal spread holds \
                     well enough.",
                );
            }
            _ => {}
        }
        if clr {
            why.push_str(clr_note);
        }
        let mut settings = vec![
            (
                "Groups",
                facts
                    .groups
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            ("Statistical test", test.name().to_string()),
        ];
        if design == Design::Matched {
            settings.push((
                "Subject field",
                self.subject_field.clone().unwrap_or_default(),
            ));
        }
        if clr {
            settings.push(("Transform", Transform::CentredLogRatio.name().to_string()));
        }
        settings.push(("Post-hoc test", post_hoc.name().to_string()));
        settings.push(("Multiple test correction", FDR.to_string()));
        let mut plots = vec![
            (
                PlotKind::PostHoc,
                "which pairs of groups differ for the selected feature",
            ),
            (PlotKind::Box, "one feature in every sample, by group"),
            (
                PlotKind::Heatmap,
                "the features that differ, across all samples",
            ),
            (
                PlotKind::PValueHistogram,
                "a check that some features truly differ",
            ),
        ];
        if test.is_presence() {
            plots.insert(
                0,
                (
                    PlotKind::PrevalenceAbundance,
                    "the share of each group's samples that have each feature",
                ),
            );
        }
        if clr && design == Design::Independent {
            plots.push((
                PlotKind::Ancom,
                "a second method made for this problem, as confirmation",
            ));
        }
        Some(Recommendation {
            heading: format!("{} with {}", test.name(), post_hoc.name()),
            mode: Mode::MultipleGroups,
            settings,
            why,
            plots,
            alternatives: match test {
                MultiGroupTest::Anova => vec![
                    "Kruskal-Wallis H-test with Dunn's test, if a result seems to rest on one \
                     or two samples.",
                    "Welch's ANOVA with Games-Howell, if the groups differ in size or spread.",
                ],
                MultiGroupTest::WelchAnova => vec![
                    "Kruskal-Wallis H-test with Dunn's test, if a result seems to rest on one \
                     or two samples.",
                    "ANOVA with Tukey-Kramer, if the groups are of similar size and spread: \
                     it is then slightly more powerful.",
                ],
                MultiGroupTest::KruskalWallis => {
                    vec!["ANOVA with Tukey-Kramer, with about ten or more samples in each group."]
                }
                MultiGroupTest::Friedman => {
                    vec!["Kruskal-Wallis H-test, if the subjects cannot be matched after all."]
                }
                MultiGroupTest::ChiSquarePresence => {
                    vec!["Kruskal-Wallis H-test, to compare abundance instead of presence."]
                }
            },
            report: Some(if test.is_presence() {
                "For each feature you report: the number of samples of each group that have \
                 it, and the corrected p-value (q-value). Name the test and the correction."
            } else {
                "For each feature you report: the mean proportion in each group, the \
                 corrected p-value (q-value) of the main test, and the post-hoc p-values of \
                 the pairs you discuss. Name both tests and the correction."
            }),
            caution: None,
            change: Change {
                multi_test: Some((test, post_hoc)),
                transform,
                correction,
                subject_field: (design == Design::Matched)
                    .then(|| self.subject_field.clone())
                    .flatten(),
                ..Change::default()
            },
        })
    }
}

/// What a test of the "At a glance" table is for: the headings its rows
/// are listed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Goal {
    TwoSamples,
    TwoGroups,
    MultipleGroups,
    Presence,
    Relationships,
    Communities,
}

impl Goal {
    pub const ALL: [Goal; 6] = [
        Goal::TwoSamples,
        Goal::TwoGroups,
        Goal::MultipleGroups,
        Goal::Presence,
        Goal::Relationships,
        Goal::Communities,
    ];

    pub fn heading(self) -> &'static str {
        match self {
            Goal::TwoSamples => "Compare two samples",
            Goal::TwoGroups => "Compare two groups",
            Goal::MultipleGroups => "Compare three or more groups",
            Goal::Presence => "Compare presence or absence",
            Goal::Relationships => "Find relationships",
            Goal::Communities => "Compare whole communities",
        }
    }
}

/// A row of the "At a glance" table: a test, what it is for, and the
/// answers to the wizard's questions that lead to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
    pub goal: Goal,
    pub test: &'static str,
    /// The comparisons the test is made for; the first the loaded data
    /// allow is used.
    pub comparisons: &'static [Comparison],
    pub question: Question,
    pub design: Option<Design>,
    pub data: Option<Data>,
    pub when: &'static str,
    pub does: &'static str,
    pub assumes: &'static str,
}

impl Row {
    /// The comparison with which the loaded data allow the test, or why
    /// they do not.
    pub fn comparison(&self, facts: &Facts) -> Result<Comparison, String> {
        let mut first = None;
        for comparison in self.comparisons {
            let mut checks = vec![
                facts.comparison(*comparison),
                facts.question(*comparison, self.question),
            ];
            if let Some(design) = self.design {
                checks.push(facts.design(*comparison, design));
                if let Some(data) = self.data {
                    checks.push(facts.data(*comparison, design, data));
                }
            }
            let reason = checks.into_iter().find_map(|check| match check {
                Availability::Yes => None,
                Availability::No(reason) => Some(reason),
            });
            match reason {
                None => return Ok(*comparison),
                Some(reason) => first = first.or(Some(reason)),
            }
        }
        Err(first.unwrap_or_default())
    }

    /// The choice whose drawing stands for the test: the most particular
    /// of the answers that lead to it.
    pub fn picture(&self) -> Choice {
        match (self.data, self.design) {
            (Some(data), _) if data != Data::Typical => Choice::Data(data),
            (_, Some(Design::Matched)) => Choice::Design(Design::Matched),
            _ if self.question != Question::Differ => Choice::Question(self.question),
            _ => Choice::Comparison(self.comparisons[0]),
        }
    }
}

/// The tests of the "At a glance" table, in the order they are listed.
pub const GLANCE: [Row; 18] = {
    use Comparison::{MultipleGroups, TwoGroups, TwoSamples, Variable};
    const fn differ(
        goal: Goal,
        test: &'static str,
        comparison: &'static [Comparison],
        design: Design,
        data: Data,
        [when, does, assumes]: [&'static str; 3],
    ) -> Row {
        Row {
            goal,
            test,
            comparisons: comparison,
            question: Question::Differ,
            design: Some(design),
            data: Some(data),
            when,
            does,
            assumes,
        }
    }
    const fn other(
        goal: Goal,
        test: &'static str,
        comparisons: &'static [Comparison],
        question: Question,
        [when, does, assumes]: [&'static str; 3],
    ) -> Row {
        Row {
            goal,
            test,
            comparisons,
            question,
            design: None,
            data: None,
            when,
            does,
            assumes,
        }
    }
    const NAMED: &str = "A metadata field that names the subject of each sample.";
    [
        other(
            Goal::TwoSamples,
            "G-test (w/ Yates') + Fisher's",
            &[TwoSamples],
            Question::Differ,
            [
                "One sample of each condition, without replicates.",
                "Compares each feature's share of the sequences of the two samples.",
                "Counts of sequences. A difference is between these two samples, not \
                 between the conditions they came from.",
            ],
        ),
        differ(
            Goal::TwoGroups,
            "Welch's t-test",
            &[TwoGroups],
            Design::Independent,
            Data::Typical,
            [
                "Different subjects in each group, about ten or more samples in each.",
                "Compares the mean abundance of each feature in the two groups.",
                "Roughly bell-shaped data, or ten or more samples in each group. The \
                 groups may differ in size and spread.",
            ],
        ),
        differ(
            Goal::TwoGroups,
            "Mann-Whitney U test",
            &[TwoGroups],
            Design::Independent,
            Data::SmallOrExtreme,
            [
                "Small groups, or a few samples with extreme values.",
                "Asks whether the samples of one group tend to rank higher than those of \
                 the other.",
                "Nothing about the shape of the data. Four or more samples in each group \
                 are needed to reach p < 0.05.",
            ],
        ),
        differ(
            Goal::TwoGroups,
            "White's non-parametric t-test",
            &[TwoGroups],
            Design::Independent,
            Data::MostlyAbsent,
            [
                "Features that are absent from most samples.",
                "Compares the means, with p-values found by shuffling the samples between \
                 the groups.",
                "Nothing about the shape of the data. Slower than the other tests.",
            ],
        ),
        differ(
            Goal::TwoGroups,
            "Welch's t-test on centred log-ratios",
            &[TwoGroups],
            Design::Independent,
            Data::Dominated,
            [
                "One or a few features make up most of every sample.",
                "Compares each feature relative to the others of its sample, so that a \
                 change in a dominant feature does not appear as a change in all the rest.",
                "As Welch's t-test, after the transform.",
            ],
        ),
        differ(
            Goal::TwoGroups,
            "Paired t-test",
            &[TwoGroups],
            Design::Matched,
            Data::Typical,
            [
                "The same subjects in both groups (before and after), about ten or more \
                 subjects.",
                "Asks whether the changes within the subjects average to zero.",
                "A metadata field that names the subject of each sample. Changes that are \
                 roughly bell-shaped, or ten or more subjects.",
            ],
        ),
        differ(
            Goal::TwoGroups,
            "Wilcoxon signed-rank test",
            &[TwoGroups],
            Design::Matched,
            Data::SmallOrExtreme,
            [
                "The same subjects in both groups, with few subjects or extreme changes.",
                "Asks whether the changes within the subjects tend to go one way.",
                "A metadata field that names the subject of each sample. Six or more \
                 subjects are needed to reach p < 0.05.",
            ],
        ),
        differ(
            Goal::MultipleGroups,
            "ANOVA",
            &[MultipleGroups],
            Design::Independent,
            Data::Typical,
            [
                "Different subjects in each group, about ten or more samples in each.",
                "Asks whether the group means differ by more than the variation within \
                 the groups explains. Welch's ANOVA, with the Games-Howell post-hoc test, \
                 is chosen when the groups differ twofold in size or in spread \
                 (Brown-Forsythe test); otherwise ANOVA with Tukey-Kramer. The post-hoc \
                 test says which pairs of groups differ.",
                "Roughly bell-shaped data, or ten or more samples in each group. ANOVA \
                 assumes equal spread; Welch's ANOVA allows each group its own.",
            ],
        ),
        differ(
            Goal::MultipleGroups,
            "Kruskal-Wallis H-test",
            &[MultipleGroups],
            Design::Independent,
            Data::SmallOrExtreme,
            [
                "Small groups, or a few samples with extreme values.",
                "Asks whether the samples of some groups tend to rank higher. Dunn's test \
                 then says which pairs of groups differ.",
                "Nothing about the shape of the data.",
            ],
        ),
        differ(
            Goal::MultipleGroups,
            "Friedman test",
            &[MultipleGroups],
            Design::Matched,
            Data::Typical,
            [
                "The same subjects sampled once in every group.",
                "Ranks each subject's samples across the groups and asks whether some \
                 groups tend to rank higher. The Wilcoxon signed-rank test then says which \
                 pairs of groups differ.",
                NAMED,
            ],
        ),
        differ(
            Goal::Presence,
            "Fisher's exact test (presence/absence)",
            &[TwoGroups],
            Design::Independent,
            Data::Presence,
            [
                "Two groups, when the question is in how many samples a feature is found \
                 and not how abundant it is.",
                "Compares the share of each group's samples that have the feature.",
                "Nothing about the data: it is exact for any number of samples.",
            ],
        ),
        differ(
            Goal::Presence,
            "McNemar's test (presence/absence)",
            &[TwoGroups],
            Design::Matched,
            Data::Presence,
            [
                "The same subjects in both groups.",
                "Asks whether a feature appears within subjects more often than it \
                 disappears.",
                "A metadata field that names the subject of each sample. Only the \
                 subjects in which the feature appears or disappears count.",
            ],
        ),
        differ(
            Goal::Presence,
            "Chi-square test (presence/absence)",
            &[MultipleGroups],
            Design::Independent,
            Data::Presence,
            [
                "Three or more groups.",
                "Asks whether the feature is found in the same share of the samples of \
                 every group.",
                "About five or more samples expected with and without the feature in \
                 each group; features with fewer are noted.",
            ],
        ),
        other(
            Goal::Relationships,
            "Spearman rank correlation",
            &[Variable],
            Question::Follow,
            [
                "Samples along a measured variable (pH, age, depth).",
                "Asks whether each feature rises or falls as the variable does.",
                "A numeric metadata field. A steady rise or fall, which need not be a \
                 straight line.",
            ],
        ),
        other(
            Goal::Relationships,
            "Co-occurrence heatmap",
            &[MultipleGroups, TwoGroups, Variable],
            Question::Together,
            [
                "Which features are found together, and which avoid each other?",
                "Correlates every pair of the most abundant features across the samples.",
                "Many samples. Features can occur together because they prefer the same \
                 kind of sample, not because they interact.",
            ],
        ),
        other(
            Goal::Relationships,
            "Mantel test",
            &[Variable],
            Question::Communities,
            [
                "Does the whole community change along a measured variable?",
                "Asks whether samples that differ more in the variable also differ more \
                 in composition.",
                "A numeric metadata field. The p-value is found by shuffling the samples.",
            ],
        ),
        other(
            Goal::Communities,
            "PERMANOVA",
            &[MultipleGroups, TwoGroups],
            Question::Communities,
            [
                "Do the groups differ in overall composition, and not only feature by \
                 feature?",
                "Asks whether samples are more alike within the groups than between \
                 them. Shown above the PCoA plot, with PERMDISP and ANOSIM.",
                "Groups that are similarly spread out, which PERMDISP checks. The p-value \
                 is found by shuffling the samples.",
            ],
        ),
        other(
            Goal::Communities,
            "Kruskal-Wallis H-test of alpha diversity",
            &[MultipleGroups, TwoGroups],
            Question::Diversity,
            [
                "Do the groups differ in how many features their samples hold, or how \
                 evenly?",
                "Compares the diversity of the samples of the groups. Shown with the \
                 alpha diversity box plot.",
                "Samples sequenced to a similar depth, or rarefied: more sequences find \
                 more features.",
            ],
        ),
    ]
};

/// A recommendation that changes no test: an analysis and its plots.
fn plots_only(
    heading: &str,
    mode: Mode,
    why: &str,
    plots: Vec<(PlotKind, &'static str)>,
    caution: Option<&'static str>,
) -> Recommendation {
    Recommendation {
        heading: heading.to_string(),
        mode,
        settings: Vec::new(),
        why: why.to_string(),
        plots,
        alternatives: Vec::new(),
        report: None,
        caution,
        change: Change::default(),
    }
}

fn follow(facts: &Facts) -> Recommendation {
    Recommendation {
        heading: CorrelationMethod::Spearman.name().to_string(),
        mode: Mode::Correlation,
        settings: vec![
            ("Numeric field", facts.variable.clone().unwrap_or_default()),
            (
                "Correlation",
                CorrelationMethod::Spearman.name().to_string(),
            ),
            (
                "Multiple test correction",
                "Benjamini-Hochberg FDR".to_string(),
            ),
        ],
        why: "Spearman's rank correlation asks whether a feature tends to rise (or fall) as \
            the number rises, whatever the shape of the trend, and is not thrown by a few \
            extreme samples. Each feature gets a coefficient from -1 to +1 and a p-value, \
            corrected for the number of features tested."
            .to_string(),
        plots: vec![
            (
                PlotKind::CorrelationScatter,
                "one feature against the number, a point per sample",
            ),
            (
                PlotKind::Volcano,
                "strength of correlation against significance, all features",
            ),
            (
                PlotKind::PValueHistogram,
                "a check that some features truly follow the number",
            ),
        ],
        alternatives: vec![
            "Pearson correlation, if you expect a straight-line relationship and there are no \
             extreme samples.",
        ],
        report: Some(
            "For each feature you report: the correlation coefficient (rho), the number of \
             samples, and the corrected p-value (q-value).",
        ),
        caution: Some(
            "A correlation across samples from different groups can come from the groups \
             alone: look at the scatter plot, where the points are coloured by group.",
        ),
        change: Change {
            method: Some(CorrelationMethod::Spearman),
            correction: Some(Correction::BenjaminiHochberg),
            ..Change::default()
        },
    }
}

fn together(comparison: Comparison) -> Recommendation {
    plots_only(
        "Co-occurrence heatmap",
        comparison.mode(),
        "The co-occurrence heatmap correlates every pair of the most abundant features \
         across the samples and clusters them, so that features that rise and fall together \
         form blocks. From there, select a feature and switch the plot to show it against \
         all others, or against one other as a scatter plot. Correlating centred log ratios \
         (in Configure plot) avoids the false negative correlations that proportions create.",
        vec![(
            PlotKind::CoOccurrence,
            "which features rise and fall together",
        )],
        Some(
            "Features can occur together because they prefer the same kind of sample, not \
             because they interact. Correlate within one group to see associations within an \
             environment.",
        ),
    )
}

fn communities(comparison: Comparison) -> Recommendation {
    match comparison {
        Comparison::TwoSamples => plots_only(
            "Scatter plot of the two samples",
            Mode::TwoSamples,
            "Two samples can be compared feature by feature, but no test can say whether \
             they differ as communities: that needs to know how much samples of the same \
             condition differ, which takes replicates. The scatter plot shows every feature \
             in one sample against the other; features far from the diagonal are those that \
             differ most.",
            vec![
                (
                    PlotKind::Scatter,
                    "every feature in one sample against the other",
                ),
                (
                    PlotKind::ProfileBar,
                    "the most abundant features side by side",
                ),
            ],
            Some("There is no test of whole communities for two samples."),
        ),
        Comparison::Variable => plots_only(
            "Mantel test",
            Mode::Correlation,
            "The Mantel test asks whether samples that are close in the measured number are \
             also alike in composition: it correlates the difference in the number between \
             every two samples with the distance between their communities, and finds its \
             p-value by shuffling the samples.",
            vec![(
                PlotKind::Mantel,
                "community distance against difference in the number",
            )],
            None,
        ),
        _ => {
            let mut plots = vec![
                (
                    PlotKind::Pcoa,
                    "a map of the samples, with the tests above it",
                ),
                (
                    PlotKind::Nmds,
                    "another map of the same distances, as a check",
                ),
                (
                    PlotKind::DistanceBox,
                    "distances within and between the groups",
                ),
            ];
            let why = if comparison == Comparison::MultipleGroups {
                "The PCoA plot places the samples so that those with similar composition lie \
                 close together. Above it, PERMANOVA tests whether the groups sit in \
                 different places, PERMDISP whether they differ in how spread out they are \
                 (which alone can make PERMANOVA significant), and ANOSIM is an older \
                 alternative. Pairwise PERMANOVA then says which groups differ from which."
            } else {
                plots.truncate(3);
                "The PCoA plot places the samples so that those with similar composition lie \
                 close together. Above it, PERMANOVA tests whether the two groups sit in \
                 different places, PERMDISP whether they differ in how spread out they are \
                 (which alone can make PERMANOVA significant), and ANOSIM is an older \
                 alternative."
            };
            let mut recommendation = plots_only(
                "PCoA plot with PERMANOVA",
                comparison.mode(),
                why,
                plots,
                None,
            );
            recommendation.alternatives = vec![
                "Another distance (Configure plot): Jaccard looks only at which features are \
                 present, Aitchison allows for a few features dominating.",
            ];
            recommendation.report = Some(
                "You report: the distance, the PERMANOVA pseudo-F and p-value with the \
                 number of permutations, and the PERMDISP p-value.",
            );
            recommendation
        }
    }
}

fn diversity(comparison: Comparison, facts: &Facts) -> Recommendation {
    match comparison {
        Comparison::TwoSamples => {
            let mut plots = vec![(
                PlotKind::RankAbundance,
                "each sample's features from most to least abundant",
            )];
            if facts.whole_counts {
                plots.insert(
                    0,
                    (
                        PlotKind::Rarefaction,
                        "features found against sequences read",
                    ),
                );
            }
            plots_only(
                "Rarefaction and rank-abundance curves",
                Mode::TwoSamples,
                "The diversity of two samples can be compared by eye, but not tested: a test \
                 needs several samples of each condition. Rarefaction curves compare the \
                 number of features at the same number of sequences, which is the fair \
                 comparison when the samples were sequenced to different depths; \
                 rank-abundance curves show how evenly the sequences are shared.",
                plots,
                Some("There is no test of diversity for two samples."),
            )
        }
        Comparison::Variable => plots_only(
            "Alpha diversity scatter plot",
            Mode::Correlation,
            "The diversity of each sample is plotted against the measured number, with their \
             correlation and its p-value. Choose the measure in Configure plot: observed \
             features counts them, Shannon and Simpson also weigh how evenly the sequences \
             are shared.",
            vec![(
                PlotKind::AlphaScatter,
                "diversity of each sample against the number",
            )],
            Some(
                "The number of features found rises with the number of sequences read: \
                 rarefy the samples (Configure plot) unless they were sequenced to similar \
                 depths.",
            ),
        ),
        _ => {
            let mut plots = vec![(
                PlotKind::AlphaDiversity,
                "the diversity of each sample, by group, with the test",
            )];
            if facts.whole_counts {
                plots.push((
                    PlotKind::Rarefaction,
                    "whether the samples were sequenced deeply enough",
                ));
            }
            let mut recommendation = plots_only(
                "Alpha diversity box plot with a Kruskal-Wallis H-test",
                comparison.mode(),
                "Each sample gets one number for its diversity, and the Kruskal-Wallis test \
                 asks whether the groups differ in it. Choose the measure in Configure plot: \
                 observed features counts them, Shannon and Simpson also weigh how evenly \
                 the sequences are shared, and Chao1 estimates the features not yet seen.",
                plots,
                Some(
                    "The number of features found rises with the number of sequences read: \
                     rarefy the samples (Configure plot) unless they were sequenced to \
                     similar depths.",
                ),
            );
            recommendation.report = Some(
                "You report: the diversity measure, whether the samples were rarefied and to \
                 what depth, the median of each group, and the Kruskal-Wallis p-value.",
            );
            recommendation
        }
    }
}

fn made_of(comparison: Comparison, facts: &Facts) -> Recommendation {
    const WHY: &str = "These plots describe the samples without testing anything. Start with \
        the composition bar plot for the most abundant features of each sample or group; the \
        heatmaps show many features at once, and the shared features plot shows which \
        features the groups have in common.";
    match comparison {
        Comparison::TwoSamples => plots_only(
            "Profile bar plot",
            Mode::TwoSamples,
            "These plots describe the two samples without testing anything: their most \
             abundant features side by side, their composition as stacked bars, and which \
             features they share.",
            vec![
                (
                    PlotKind::ProfileBar,
                    "the most abundant features side by side",
                ),
                (PlotKind::Composition, "each sample as a stacked bar"),
                (
                    PlotKind::SharedFeatures,
                    "features found in one sample, or in both",
                ),
                (
                    PlotKind::RankAbundance,
                    "features from most to least abundant",
                ),
            ],
            None,
        ),
        Comparison::Variable => plots_only(
            "Map of sample locations",
            Mode::Correlation,
            "The map places the samples where they were taken, from the latitude and \
             longitude in the metadata, and can colour them by any field.",
            vec![(PlotKind::Map, "where the samples were taken")],
            None,
        ),
        _ => {
            let mut plots = vec![
                (
                    PlotKind::Composition,
                    "the most abundant features of each sample or group",
                ),
                (PlotKind::Heatmap, "features against samples, clustered"),
                (
                    PlotKind::GroupHeatmap,
                    "features against groups: mean, median or prevalence",
                ),
                (
                    PlotKind::PrevalenceAbundance,
                    "how widespread and how abundant each feature is",
                ),
                (
                    PlotKind::SharedFeatures,
                    "features found in one group, or in several",
                ),
                (
                    PlotKind::RankAbundance,
                    "features from most to least abundant",
                ),
            ];
            if facts.has_coordinates {
                plots.push((PlotKind::Map, "where the samples were taken"));
            }
            plots_only("Composition bar plot", comparison.mode(), WHY, plots, None)
        }
    }
}

fn depth(comparison: Comparison, facts: &Facts) -> Recommendation {
    let mut plots = Vec::new();
    if facts.whole_counts {
        plots.push((
            PlotKind::Rarefaction,
            "features found against sequences read",
        ));
    }
    if comparison == Comparison::TwoSamples {
        plots.push((
            PlotKind::SequenceHistogram,
            "how many sequences the features have",
        ));
    }
    plots_only(
        if facts.whole_counts {
            "Rarefaction curves"
        } else {
            "Sequence histogram"
        },
        comparison.mode(),
        if facts.whole_counts {
            "A rarefaction curve shows how many features would have been found in a sample \
             had fewer sequences been read. A curve that has levelled off means that more \
             sequencing would have added few features; one that is still climbing means that \
             rare features were missed, and that samples of different depth should be \
             compared with care."
        } else {
            "Rarefaction curves need counts of sequences, and this profile holds proportions \
             or other fractional values. The sequence histogram shows how the values are \
             spread over the features instead."
        },
        plots,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A profile of three groups with four subjects each, a number for
    /// every sample, and counts.
    fn facts() -> Facts {
        Facts {
            samples: 12,
            has_metadata: true,
            group_field: Some("Time".to_string()),
            groups: vec![("A".into(), 4), ("B".into(), 4), ("C".into(), 4)],
            two_groups: Some(("A".into(), "B".into())),
            two_samples: Some(("A1".into(), "A2".into())),
            numeric_fields: vec!["pH".to_string()],
            variable: Some("pH".to_string()),
            pair_fields: vec![("Subject".into(), 4)],
            subject_fields: vec![("Subject".into(), 4)],
            whole_counts: true,
            has_coordinates: false,
            spread: None,
            spread_clr: None,
        }
    }

    fn walk(choices: &[Choice], facts: &Facts) -> Wizard {
        let mut wizard = Wizard::default();
        for choice in choices {
            assert!(wizard.choose(*choice, facts), "{choice:?} was refused");
        }
        wizard
    }

    fn test_of(choices: &[Choice], facts: &Facts) -> Change {
        walk(choices, facts).recommendation(facts).unwrap().change
    }

    #[test]
    fn the_steps_follow_the_answers() {
        let facts = facts();
        let mut wizard = Wizard::default();
        assert_eq!(wizard.step(), Step::Comparison);
        assert!(wizard.recommendation(&facts).is_none());
        wizard.choose(Choice::Comparison(Comparison::TwoGroups), &facts);
        assert_eq!(wizard.step(), Step::Question);
        // Only differing features ask about the design and the data.
        wizard.choose(Choice::Question(Question::Communities), &facts);
        assert_eq!(wizard.step(), Step::Result);
        assert_eq!(
            wizard.steps(),
            [Step::Comparison, Step::Question, Step::Result]
        );
        wizard.back();
        assert_eq!(wizard.step(), Step::Question);
        wizard.choose(Choice::Question(Question::Differ), &facts);
        assert_eq!(wizard.step(), Step::Design);
        wizard.choose(Choice::Design(Design::Matched), &facts);
        assert_eq!(wizard.subject_field.as_deref(), Some("Subject"));
        assert_eq!(wizard.step(), Step::Data);
        wizard.choose(Choice::Data(Data::Typical), &facts);
        assert_eq!(wizard.step(), Step::Result);
        assert_eq!(
            wizard.answer(Step::Design),
            Some(Choice::Design(Design::Matched))
        );
        // A new comparison forgets the later answers.
        wizard.choose(Choice::Comparison(Comparison::TwoSamples), &facts);
        assert_eq!(wizard.step(), Step::Question);
        assert!(wizard.design.is_none() && wizard.subject_field.is_none());
        // Two samples need no more questions.
        wizard.choose(Choice::Question(Question::Differ), &facts);
        assert_eq!(wizard.step(), Step::Result);
        wizard.restart();
        assert_eq!(wizard, Wizard::default());
    }

    #[test]
    fn every_path_of_the_flow_chart_ends_on_its_test() {
        use Choice as C;
        let facts = facts();
        let two = |design, data| {
            test_of(
                &[
                    C::Comparison(Comparison::TwoGroups),
                    C::Question(Question::Differ),
                    C::Design(design),
                    C::Data(data),
                ],
                &facts,
            )
        };
        let cases = [
            (Design::Independent, Data::Typical, GroupTest::Welch),
            (
                Design::Independent,
                Data::SmallOrExtreme,
                GroupTest::MannWhitney,
            ),
            (Design::Independent, Data::MostlyAbsent, GroupTest::White),
            (
                Design::Independent,
                Data::Presence,
                GroupTest::FisherPresence,
            ),
            (Design::Independent, Data::Dominated, GroupTest::Welch),
            (Design::Matched, Data::Typical, GroupTest::PairedT),
            (
                Design::Matched,
                Data::SmallOrExtreme,
                GroupTest::WilcoxonSignedRank,
            ),
            (Design::Matched, Data::Presence, GroupTest::McNemar),
        ];
        for (design, data, test) in cases {
            let change = two(design, data);
            assert_eq!(change.group_test, Some(test), "{design:?} {data:?}");
            assert_eq!(change.correction, Some(Correction::BenjaminiHochberg));
            assert_eq!(
                change.transform,
                Some(if data == Data::Dominated {
                    Transform::CentredLogRatio
                } else {
                    Transform::None
                })
            );
            assert_eq!(change.subject_field.is_some(), design == Design::Matched);
            assert_eq!(change.show_pairs, design == Design::Matched);
        }

        let multi = |design, data| {
            test_of(
                &[
                    C::Comparison(Comparison::MultipleGroups),
                    C::Question(Question::Differ),
                    C::Design(design),
                    C::Data(data),
                ],
                &facts,
            )
            .multi_test
        };
        assert_eq!(
            multi(Design::Independent, Data::Typical),
            Some((MultiGroupTest::Anova, PostHocTest::TukeyKramer))
        );
        assert_eq!(
            multi(Design::Independent, Data::SmallOrExtreme),
            Some((MultiGroupTest::KruskalWallis, PostHocTest::Dunn))
        );
        assert_eq!(
            multi(Design::Independent, Data::Presence).map(|t| t.0),
            Some(MultiGroupTest::ChiSquarePresence)
        );
        assert_eq!(
            multi(Design::Matched, Data::SmallOrExtreme),
            Some((MultiGroupTest::Friedman, PostHocTest::WilcoxonHolm))
        );
        // Groups of very different sizes get the tests that allow each its
        // own spread.
        let mut uneven = facts.clone();
        uneven.groups[2].1 = 9;
        let typical = [
            C::Comparison(Comparison::MultipleGroups),
            C::Question(Question::Differ),
            C::Design(Design::Independent),
            C::Data(Data::Typical),
        ];
        assert_eq!(
            test_of(&typical, &uneven).multi_test,
            Some((MultiGroupTest::WelchAnova, PostHocTest::GamesHowell))
        );

        let samples = test_of(
            &[
                C::Comparison(Comparison::TwoSamples),
                C::Question(Question::Differ),
            ],
            &facts,
        );
        assert_eq!(samples.sample_test, Some(SampleTest::GTestFisher));
        let follow = test_of(
            &[
                C::Comparison(Comparison::Variable),
                C::Question(Question::Follow),
            ],
            &facts,
        );
        assert_eq!(follow.method, Some(CorrelationMethod::Spearman));
    }

    /// The Brown-Forsythe test of the features chooses between ANOVA and
    /// Welch's ANOVA, and the recommendation says which way it went.
    #[test]
    fn unequal_spread_leads_to_welchs_anova() {
        use Choice as C;
        let typical = [
            C::Comparison(Comparison::MultipleGroups),
            C::Question(Question::Differ),
            C::Design(Design::Independent),
            C::Data(Data::Typical),
        ];
        let dominated = [
            C::Comparison(Comparison::MultipleGroups),
            C::Question(Question::Differ),
            C::Design(Design::Independent),
            C::Data(Data::Dominated),
        ];
        let close = SpreadSummary {
            differ: 6,
            tested: 100,
        };
        let far = SpreadSummary {
            differ: 31,
            tested: 100,
        };
        let recommend =
            |choices: &[Choice], facts: &Facts| walk(choices, facts).recommendation(facts).unwrap();

        // Close to chance: ANOVA, saying the assumption holds.
        let even = Facts {
            spread: Some(close),
            spread_clr: Some(far),
            ..facts()
        };
        let r = recommend(&typical, &even);
        assert_eq!(
            r.change.multi_test,
            Some((MultiGroupTest::Anova, PostHocTest::TukeyKramer))
        );
        assert!(r.why.contains("6 of 100 features (6%"), "{}", r.why);
        assert!(r.why.contains("holds well enough"), "{}", r.why);
        // With the centred log-ratio transform, the spread of the
        // transformed values counts.
        let r = recommend(&dominated, &even);
        assert_eq!(
            r.change.multi_test,
            Some((MultiGroupTest::WelchAnova, PostHocTest::GamesHowell))
        );
        assert!(r.heading.starts_with("Welch's ANOVA with Games-Howell"));
        assert!(r.why.contains("31 of 100 features (31%"), "{}", r.why);
        assert!(r.why.contains("more than chance"), "{}", r.why);
        assert!(r
            .settings
            .iter()
            .any(|(name, value)| *name == "Statistical test" && value == "Welch's ANOVA"));
        assert!(r
            .alternatives
            .iter()
            .any(|a| a.starts_with("ANOVA with Tukey-Kramer")));

        // Uneven groups alone, and both reasons together.
        let mut uneven = Facts {
            spread: Some(close),
            ..facts()
        };
        uneven.groups[2].1 = 9;
        let r = recommend(&typical, &uneven);
        assert_eq!(r.change.multi_test.unwrap().0, MultiGroupTest::WelchAnova);
        assert!(r.why.contains("twofold in size"), "{}", r.why);
        uneven.spread = Some(far);
        let r = recommend(&typical, &uneven);
        assert!(
            r.why.contains("twofold in size") && r.why.contains("31 of 100"),
            "{}",
            r.why
        );
        // Small groups still lead to Kruskal-Wallis, whatever the spread.
        let small = [
            C::Comparison(Comparison::MultipleGroups),
            C::Question(Question::Differ),
            C::Design(Design::Independent),
            C::Data(Data::SmallOrExtreme),
        ];
        let spread = Facts {
            spread: Some(far),
            ..facts()
        };
        assert_eq!(
            recommend(&small, &spread).change.multi_test.unwrap().0,
            MultiGroupTest::KruskalWallis
        );
    }

    #[test]
    fn every_question_ends_on_plots_its_analysis_offers() {
        let facts = Facts {
            has_coordinates: true,
            ..facts()
        };
        for comparison in Comparison::ALL {
            for question in Question::ALL {
                if !facts.question(comparison, question).is_available() {
                    continue;
                }
                let mut wizard = walk(
                    &[Choice::Comparison(comparison), Choice::Question(question)],
                    &facts,
                );
                if wizard.step() == Step::Design {
                    wizard.choose(Choice::Design(Design::Independent), &facts);
                    wizard.choose(Choice::Data(Data::Typical), &facts);
                }
                let recommendation = wizard.recommendation(&facts).unwrap();
                assert_eq!(recommendation.mode, comparison.mode());
                assert!(
                    !recommendation.plots.is_empty(),
                    "{comparison:?} {question:?}"
                );
                for (plot, _) in &recommendation.plots {
                    assert!(
                        PlotKind::available(recommendation.mode).contains(plot),
                        "{plot:?} is not a plot of {comparison:?} ({question:?})"
                    );
                }
                assert!(!recommendation.why.is_empty());
            }
        }
    }

    #[test]
    fn only_what_the_data_support_can_be_chosen() {
        let full = facts();
        // Nothing loaded: every comparison says to load a profile.
        let empty = Facts::default();
        for comparison in Comparison::ALL {
            let Availability::No(reason) = empty.comparison(comparison) else {
                panic!("{comparison:?} is offered without a profile");
            };
            assert!(reason.contains("No profile is loaded"));
        }
        let mut wizard = Wizard::default();
        assert!(!wizard.choose(Choice::Comparison(Comparison::TwoSamples), &empty));
        assert_eq!(wizard.step(), Step::Comparison);

        // A profile alone: two samples only.
        let profile = Facts {
            samples: 12,
            two_samples: Some(("A1".into(), "A2".into())),
            ..Facts::default()
        };
        assert!(profile.comparison(Comparison::TwoSamples).is_available());
        for comparison in [
            Comparison::TwoGroups,
            Comparison::MultipleGroups,
            Comparison::Variable,
        ] {
            assert!(
                matches!(profile.comparison(comparison), Availability::No(r) if r.contains("metadata"))
            );
        }

        // Two groups are not "multiple", and no numbers means no variable.
        let two = Facts {
            groups: full.groups[..2].to_vec(),
            numeric_fields: Vec::new(),
            ..full.clone()
        };
        assert!(two.comparison(Comparison::TwoGroups).is_available());
        assert!(matches!(
            two.comparison(Comparison::MultipleGroups),
            Availability::No(r) if r.contains("2 groups")
        ));
        assert!(!two.comparison(Comparison::Variable).is_available());

        // Questions a comparison cannot answer.
        use Comparison::*;
        assert!(!full.question(Variable, Question::Differ).is_available());
        assert!(!full.question(TwoGroups, Question::Follow).is_available());
        assert!(full.question(Variable, Question::Follow).is_available());
        assert!(!full.question(TwoSamples, Question::Together).is_available());
        assert!(!full.question(Variable, Question::MadeOf).is_available());
        let proportions = Facts {
            whole_counts: false,
            ..full.clone()
        };
        assert!(!proportions
            .question(TwoGroups, Question::Depth)
            .is_available());
        assert!(proportions
            .question(TwoSamples, Question::Depth)
            .is_available());

        // Matched designs need a field that matches the subjects.
        let unmatched = Facts {
            pair_fields: Vec::new(),
            subject_fields: Vec::new(),
            ..full.clone()
        };
        assert!(!unmatched.design(TwoGroups, Design::Matched).is_available());
        assert!(unmatched
            .design(TwoGroups, Design::Independent)
            .is_available());
        assert!(full.design(MultipleGroups, Design::Matched).is_available());

        // Tests that do not exist for a design.
        assert!(!full
            .data(TwoGroups, Design::Matched, Data::MostlyAbsent)
            .is_available());
        assert!(!full
            .data(MultipleGroups, Design::Independent, Data::MostlyAbsent)
            .is_available());
        assert!(!full
            .data(MultipleGroups, Design::Matched, Data::Presence)
            .is_available());
        assert!(full
            .data(TwoGroups, Design::Matched, Data::Presence)
            .is_available());
    }

    #[test]
    fn the_data_suggest_an_answer() {
        let facts = facts();
        let (data, reason) = facts
            .suggested(Comparison::MultipleGroups, Design::Independent)
            .unwrap();
        assert_eq!(data, Data::SmallOrExtreme);
        assert_eq!(reason, "Suggested: your smallest group has 4 samples");
        let large = Facts {
            groups: vec![("A".into(), 12), ("B".into(), 30)],
            ..facts.clone()
        };
        assert_eq!(
            large
                .suggested(Comparison::TwoGroups, Design::Independent)
                .unwrap()
                .0,
            Data::Typical
        );
        assert_eq!(
            facts
                .suggested(Comparison::TwoGroups, Design::Matched)
                .unwrap()
                .1,
            "Suggested: your groups share 4 subjects"
        );
        assert!(facts
            .suggested(Comparison::TwoSamples, Design::Independent)
            .is_none());
    }

    #[test]
    fn every_test_at_a_glance_is_where_its_answers_lead() {
        let facts = facts();
        for row in &GLANCE {
            let mut wizard = Wizard {
                glance: true,
                ..Wizard::default()
            };
            assert!(wizard.follow(row, &facts), "{}", row.test);
            assert!(!wizard.glance && wizard.step() == Step::Result);
            let recommendation = wizard.recommendation(&facts).unwrap();
            // The tests of features are named as the Statistics panel
            // names them (the transform apart).
            if row.question == Question::Differ && row.data != Some(Data::Dominated) {
                assert!(
                    recommendation.heading.starts_with(row.test),
                    "{} leads to {}",
                    row.test,
                    recommendation.heading
                );
            }
            assert_eq!(wizard.comparison.unwrap().mode(), recommendation.mode);
            for text in [row.when, row.does, row.assumes] {
                assert!(text.ends_with(['.', '?']) && !text.contains("  "), "{text}");
            }
        }
        for goal in Goal::ALL {
            assert!(GLANCE.iter().any(|row| row.goal == goal), "{goal:?}");
        }
        // Rows of one goal are listed together.
        let mut goals: Vec<Goal> = GLANCE.iter().map(|row| row.goal).collect();
        goals.dedup();
        assert_eq!(goals, Goal::ALL);

        // Without data no test can be followed, and each says why.
        let nothing = Facts::default();
        for row in &GLANCE {
            let reason = row.comparison(&nothing).unwrap_err();
            assert!(reason.contains("No profile is loaded"), "{reason}");
            let mut wizard = Wizard::default();
            assert!(!wizard.follow(row, &nothing));
            assert_eq!(wizard, Wizard::default());
        }
        // A test for several comparisons uses the first the data allow,
        // and otherwise gives the reason of the first.
        let two = Facts {
            groups: vec![("A".into(), 4), ("B".into(), 4)],
            subject_fields: Vec::new(),
            ..facts.clone()
        };
        let permanova = GLANCE.iter().find(|row| row.test == "PERMANOVA").unwrap();
        assert_eq!(permanova.comparison(&facts), Ok(Comparison::MultipleGroups));
        assert_eq!(permanova.comparison(&two), Ok(Comparison::TwoGroups));
        let friedman = GLANCE
            .iter()
            .find(|row| row.test == "Friedman test")
            .unwrap();
        assert!(friedman
            .comparison(&two)
            .unwrap_err()
            .contains("three or more are needed"));
    }

    #[test]
    fn every_choice_is_explained() {
        let choices = Comparison::ALL
            .into_iter()
            .map(Choice::Comparison)
            .chain(Question::ALL.into_iter().map(Choice::Question))
            .chain(Design::ALL.into_iter().map(Choice::Design))
            .chain(Data::ALL.into_iter().map(Choice::Data));
        for choice in choices {
            assert!(!choice.title().is_empty());
            assert!(choice.summary().ends_with('.') || choice.summary().ends_with('?'));
            assert!(choice.info().len() > 150, "{choice:?}");
        }
    }
}
