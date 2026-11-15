//! Plots of whole communities: the diversity within samples, the samples
//! placed by the distances between them, and those distances within and
//! between groups.

use stamp_core::stats::community::{
    mantel, CommunityTest, MantelRun, PermutationRun, PermutationTest,
};
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::diversity::{
    distances, nmds, pcoa, pcoa_leading, AlphaMetric, CommunityTable, Distance, DistanceMatrix,
    DistanceRun, NmdsRun, Normalisation, Ordination,
};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::results::format_p_value;
use stamp_core::stats::results::{Control, Step};
use stamp_core::stats::StampRng;
use stamp_core::{ProfileTree, Unclassified};

use crate::axes::group_color;
use crate::feature_plots::{self, BoxOptions, FeatureData, Field, Group};
use crate::figure::{Color, Figure};
use crate::{correlation_plot, ordination_plot};

/// The samples of a community with their groups.
#[derive(Debug, Clone, PartialEq)]
pub struct Community {
    pub samples: Vec<String>,
    /// Group of each sample, as an index into `groups`.
    pub sample_groups: Vec<usize>,
    pub groups: Vec<String>,
    /// Colour of each group; groups beyond the list use the default palette.
    pub group_colors: Vec<Color>,
    /// What colours and shapes the samples of the ordination plots, if not
    /// only their groups. The community tests always compare the groups.
    pub styling: crate::ordination_style::Styling,
    pub table: CommunityTable,
}

impl Community {
    /// The community of some samples of a profile tree at a hierarchical
    /// level, whatever treatment of unclassified sequences the feature
    /// tests use: unclassified categories are never features here.
    pub fn from_tree(
        tree: &ProfileTree,
        samples: &[String],
        profile_level: &str,
        sample_groups: Vec<usize>,
        groups: Vec<String>,
        normalisation: Normalisation,
        rng: &mut StampRng,
    ) -> Result<Community, String> {
        let names: Vec<&str> = samples.iter().map(String::as_str).collect();
        let profile = tree
            .profile(&names, None, profile_level, Unclassified::Retain)
            .map_err(|e| e.to_string())?;
        Ok(Community {
            samples: samples.to_vec(),
            sample_groups,
            groups,
            group_colors: Vec::new(),
            styling: Default::default(),
            table: CommunityTable::new(&profile, normalisation, rng)?,
        })
    }

    pub fn group_color(&self, index: usize) -> Color {
        self.group_colors
            .get(index)
            .copied()
            .unwrap_or_else(|| group_color(index))
    }

    /// The samples of each group, as indices.
    fn members(&self) -> Vec<Vec<usize>> {
        (0..self.groups.len())
            .map(|group| {
                (0..self.samples.len())
                    .filter(|&s| self.sample_groups.get(s) == Some(&group))
                    .collect()
            })
            .collect()
    }

    /// The groups that have samples, renumbered from zero, and the group of
    /// each sample in that numbering: what the community tests need.
    fn dense_groups(&self) -> Vec<usize> {
        let mut used: Vec<usize> = self.sample_groups.clone();
        used.sort_unstable();
        used.dedup();
        self.sample_groups
            .iter()
            .map(|g| used.iter().position(|u| u == g).unwrap_or(0))
            .collect()
    }

    /// A note on how the samples were made comparable, if they were rarefied.
    fn depth_note(&self) -> Option<String> {
        self.table
            .depth
            .map(|depth| format!("Samples rarefied to {depth} sequences"))
    }
}

/// The value of an alpha diversity measure for each sample, in order.
pub fn alpha_values(community: &Community, metric: AlphaMetric) -> Vec<f64> {
    community
        .table
        .abundances
        .iter()
        .map(|sample| metric.value(sample))
        .collect()
}

/// Box plot of an alpha diversity measure in each group, with the p-value
/// of a Kruskal-Wallis test of a difference between the groups.
pub fn alpha_plot(community: &Community, metric: AlphaMetric, options: &BoxOptions) -> Figure {
    let values = alpha_values(community, metric);
    if values.iter().any(|v| v.is_nan()) {
        return Figure::message(
            "The Chao1 estimate needs counts of sequences, and this profile holds fractional values",
        );
    }
    let groups: Vec<Group> = community
        .members()
        .into_iter()
        .enumerate()
        .filter(|(_, members)| !members.is_empty())
        .map(|(group, members)| Group {
            name: community.groups[group].clone(),
            color: community.group_color(group),
            samples: members
                .iter()
                .map(|&s| community.samples[s].clone())
                .collect(),
            values: members.iter().map(|&s| values[s]).collect(),
        })
        .collect();
    if groups.is_empty() {
        return Figure::message("No samples to show");
    }
    let p_value = (groups.len() >= 2).then(|| {
        let slices: Vec<&[f64]> = groups.iter().map(|g| g.values.as_slice()).collect();
        let (p, _) = MultiGroupTest::KruskalWallis.run(&slices);
        // Named, since nothing else on the plot says which test it is.
        format!("{} (Kruskal-Wallis H-test)", format_p_value(p, -15))
    });
    let data = FeatureData {
        feature: match community.depth_note() {
            Some(note) => format!("{} ({})", metric.name(), note.to_lowercase()),
            None => metric.name().to_string(),
        },
        field: Field::Proportion,
        groups,
        p_value,
        // One test, of the groups' diversity: no correction applies.
        p_value_kind: None,
        value_label: Some(metric.name().to_string()),
    };
    feature_plots::box_plot(&data, options)
}

/// The outcome of the community tests on a set of distances, as lines of
/// text.
pub fn test_notes(
    community: &Community,
    distance: Distance,
    permutations: usize,
    rng: &mut StampRng,
) -> Vec<String> {
    let matrix = distances(&community.table.abundances, distance);
    test_lines(community, &matrix, permutations, rng, None)
}

/// The community tests on a distance matrix, given its principal
/// coordinates on every axis if they are already known.
fn test_lines(
    community: &Community,
    matrix: &DistanceMatrix,
    permutations: usize,
    rng: &mut StampRng,
    ordination: Option<&Ordination>,
) -> Vec<String> {
    let groups = community.dense_groups();
    CommunityTest::ALL
        .iter()
        .map(|test| {
            let result = test.run_with(matrix, &groups, permutations, rng, ordination);
            test_line(*test, &result)
        })
        .collect()
}

/// The outcome of a community test as a line of text.
fn test_line(test: CommunityTest, result: &PermutationTest) -> String {
    if result.statistic.is_nan() {
        format!(
            "{}: needs at least two groups, and more samples than groups",
            test.name()
        )
    } else {
        format!(
            "{}: {} = {:.3}, p = {} ({} permutations)",
            test.name(),
            test.statistic_name(),
            result.statistic,
            format_p_value(result.p_value, -15),
            result.permutations
        )
    }
}

/// Axes of an ordination kept for its plot.
const PLOTTED_AXES: usize = 3;

/// What an ordination plot of a community shows, apart from how it looks:
/// the slow part of drawing it, kept by the window so that changing the
/// colours, shapes or sizes does not compute it again.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Coordinates of each sample on the first (up to three) axes.
    pub positions: Vec<Vec<f64>>,
    /// Share of the variation of every axis (`NaN` for NMDS, two axes).
    pub variance_fractions: Vec<f64>,
    /// Notes above the plot: the distance, rarefying, and the NMDS stress.
    pub notes: Vec<String>,
    /// The outcome of each community test, if they were run (PCoA).
    pub tests: Option<Vec<String>>,
}

/// The note on the distance and on rarefying.
fn distance_note(community: &Community, distance: Distance) -> String {
    match community.depth_note() {
        Some(note) => format!("{} distance. {note}.", distance.name()),
        None => format!("{} distance", distance.name()),
    }
}

/// The principal coordinates of a community by a distance, and the
/// community tests if `tests` gives the number of permutations and the
/// random numbers for them. `Err` holds a message for the plot.
pub fn pcoa_layout(
    community: &Community,
    distance: Distance,
    tests: Option<(usize, &mut StampRng)>,
) -> Result<Layout, String> {
    let matrix = distances(&community.table.abundances, distance);
    let (permutations, rng) = match tests {
        Some((permutations, rng)) => (Some(permutations), Some(rng)),
        None => (None, None),
    };
    let mut spare = stamp_core::stats::rng_from_seed(0);
    let rng = rng.unwrap_or(&mut spare);
    let mut stage = PcoaStage::Start {
        matrix,
        permutations,
    };
    match stage.advance(community, distance, rng, &mut |_, _, _| Control::Continue) {
        Step::Done(CommunityOutput::Layout(layout)) => layout,
        Step::Done(_) => unreachable!("a PCoA gives a layout"),
        Step::Paused | Step::Cancelled => unreachable!("never paused or cancelled"),
    }
}

/// Principal coordinates plot of the samples by a distance, with the
/// community tests written above it if `tests` gives the number of
/// permutations and the random numbers for them.
pub fn pcoa_plot(
    community: &Community,
    distance: Distance,
    tests: Option<(usize, &mut StampRng)>,
    options: &ordination_plot::Options,
) -> Figure {
    match pcoa_layout(community, distance, tests) {
        Ok(layout) => draw_pcoa(community, &layout, options),
        Err(message) => Figure::message(&message),
    }
}

/// The PCoA plot of a community from its [`pcoa_layout`].
pub fn draw_pcoa(
    community: &Community,
    layout: &Layout,
    options: &ordination_plot::Options,
) -> Figure {
    let data = ordination_data(community);
    let mut notes = layout.notes.clone();
    if let Some(tests) = &layout.tests {
        notes.extend(tests.iter().cloned());
        // The tests compare the groups whatever colours the samples.
        if community.styling.color != crate::ordination_style::SampleColors::Group {
            notes.push(format!(
                "The tests compare the groups of {}, not the colours",
                community.styling.group_label()
            ));
        }
    }
    ordination_plot::plot_ordination(
        &data,
        &layout.positions,
        &layout.variance_fractions,
        "PCo",
        &notes,
        options,
    )
}

/// The NMDS layout of a community by a distance, in two dimensions, with
/// its stress. `Err` holds a message for the plot.
pub fn nmds_layout(community: &Community, distance: Distance) -> Result<Layout, String> {
    let matrix = distances(&community.table.abundances, distance);
    nmds_result(community, distance, nmds(&matrix, 2))
}

/// The layout from an NMDS, or the message if there is none.
fn nmds_result(
    community: &Community,
    distance: Distance,
    layout: Option<stamp_core::stats::diversity::Nmds>,
) -> Result<Layout, String> {
    let Some(layout) = layout else {
        return Err(
            "Degenerate NMDS plot: it needs three samples that are not all the same".to_string(),
        );
    };
    let quality = match layout.stress {
        s if s < 0.05 => "excellent",
        s if s < 0.1 => "good",
        s if s < 0.2 => "fair",
        _ => "poor",
    };
    Ok(Layout {
        positions: layout.positions,
        variance_fractions: vec![f64::NAN, f64::NAN],
        notes: vec![
            distance_note(community, distance),
            format!("Stress = {:.3} ({quality})", layout.stress),
        ],
        tests: None,
    })
}

/// Non-metric multidimensional scaling plot of the samples by a distance,
/// in two dimensions, with its stress.
pub fn nmds_plot(
    community: &Community,
    distance: Distance,
    options: &ordination_plot::Options,
) -> Figure {
    match nmds_layout(community, distance) {
        Ok(layout) => draw_nmds(community, &layout, options),
        Err(message) => Figure::message(&message),
    }
}

/// The NMDS plot of a community from its [`nmds_layout`].
pub fn draw_nmds(
    community: &Community,
    layout: &Layout,
    options: &ordination_plot::Options,
) -> Figure {
    let data = ordination_data(community);
    let options = ordination_plot::Options {
        show_pc3_vs_pc2: false,
        show_pc1_vs_pc3: false,
        ..options.clone()
    };
    ordination_plot::plot_ordination(
        &data,
        &layout.positions,
        &layout.variance_fractions,
        "NMDS",
        &layout.notes,
        &options,
    )
}

/// The samples of a community as an ordination plot wants them.
fn ordination_data(community: &Community) -> ordination_plot::Data {
    ordination_plot::Data {
        samples: community
            .samples
            .iter()
            .zip(&community.sample_groups)
            .map(|(name, &group)| ordination_plot::Sample {
                name: name.clone(),
                group,
            })
            .collect(),
        groups: community.groups.clone(),
        group_colors: community.group_colors.clone(),
        styling: community.styling.clone(),
    }
}

/// The Mantel test between the distances of a community and the
/// differences of a number known for each sample (`values`, in the order of
/// the community's samples), drawn as one point per pair of samples.
#[allow(clippy::too_many_arguments)]
pub fn mantel_plot(
    community: &Community,
    values: &[f64],
    field: &str,
    distance: Distance,
    method: CorrelationMethod,
    permutations: usize,
    rng: &mut StampRng,
    options: &correlation_plot::Options,
) -> Figure {
    let n = community.samples.len();
    if values.len() != n || n < 3 {
        return Figure::message("The Mantel test needs at least three samples");
    }
    let matrix = distances(&community.table.abundances, distance);
    let differences = differences_of(values);
    let test = mantel(&differences, &matrix, method, permutations, rng);
    draw_mantel(
        community,
        field,
        distance,
        method,
        &matrix,
        &differences,
        &test,
        options,
    )
}

/// The differences between the values of every pair of samples, as a
/// matrix of distances.
fn differences_of(values: &[f64]) -> DistanceMatrix {
    DistanceMatrix::from_fn(values.len(), |i, j| (values[i] - values[j]).abs())
}

/// The Mantel plot of a community from its distances (`matrix`), the
/// differences of the numeric field and the outcome of the test, as a
/// [`CommunityRun`] finds them. Only the samples and the rarefying depth of
/// the community's table are used.
#[allow(clippy::too_many_arguments)]
pub fn draw_mantel(
    community: &Community,
    field: &str,
    distance: Distance,
    method: CorrelationMethod,
    matrix: &DistanceMatrix,
    differences: &DistanceMatrix,
    test: &PermutationTest,
    options: &correlation_plot::Options,
) -> Figure {
    let n = community.samples.len();
    if matrix.size() != n || n < 3 {
        return Figure::message("The Mantel test needs at least three samples");
    }
    let label = format!("{} distance", distance.name());
    let mut statistics = if test.statistic.is_nan() {
        "Mantel test: undefined, the distances do not vary".to_string()
    } else {
        format!(
            "Mantel test: {} = {:.3}, p = {} ({} permutations)",
            method.coefficient_name(),
            test.statistic,
            format_p_value(test.p_value, -15),
            test.permutations
        )
    };
    if let Some(note) = community.depth_note() {
        statistics.push_str(&format!(". {note}"));
    }
    let data = correlation_plot::Data {
        feature: format!("{label} against difference in {field}"),
        field: format!("Difference in {field}"),
        points: (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .map(|(i, j)| correlation_plot::Point {
                sample: format!("{} : {}", community.samples[i], community.samples[j]),
                value: differences.get(i, j),
                percent: matrix.get(i, j),
                group: None,
            })
            .collect(),
        groups: Vec::new(),
        group_colors: Vec::new(),
        statistics: Some(statistics),
        value_label: Some(label),
    };
    correlation_plot::plot(&data, options)
}

/// Colour of the distances between samples of different groups.
const BETWEEN_COLOR: Color = Color::rgb(190, 190, 190);

/// Box plot of the distances between samples of the same group, group by
/// group, and between samples of different groups.
pub fn distance_box_plot(
    community: &Community,
    distance: Distance,
    options: &BoxOptions,
) -> Figure {
    let matrix = distances(&community.table.abundances, distance);
    draw_distance_box(community, distance, &matrix, options)
}

/// The distance box plot of a community from its distances, as a
/// [`CommunityRun`] finds them. Only the samples, groups and rarefying depth
/// of the community's table are used.
pub fn draw_distance_box(
    community: &Community,
    distance: Distance,
    matrix: &DistanceMatrix,
    options: &BoxOptions,
) -> Figure {
    if matrix.size() != community.samples.len() {
        return Figure::message("Distances need at least two samples");
    }
    let pair = |a: usize, b: usize| format!("{} : {}", community.samples[a], community.samples[b]);
    let members = community.members();
    let mut groups: Vec<Group> = members
        .iter()
        .enumerate()
        .filter(|(_, members)| members.len() >= 2)
        .map(|(group, members)| {
            let pairs: Vec<(usize, usize)> = members
                .iter()
                .enumerate()
                .flat_map(|(i, &a)| members[i + 1..].iter().map(move |&b| (a, b)))
                .collect();
            Group {
                // The title says these are distances within the group; the
                // bare name keeps the labels of neighbouring boxes apart.
                name: community.groups[group].clone(),
                color: community.group_color(group),
                samples: pairs.iter().map(|&(a, b)| pair(a, b)).collect(),
                values: pairs.iter().map(|&(a, b)| matrix.get(a, b)).collect(),
            }
        })
        .collect();
    let between: Vec<(usize, usize)> = (0..community.samples.len())
        .flat_map(|a| (a + 1..community.samples.len()).map(move |b| (a, b)))
        .filter(|&(a, b)| community.sample_groups[a] != community.sample_groups[b])
        .collect();
    if !between.is_empty() {
        groups.push(Group {
            name: "Between groups".to_string(),
            color: BETWEEN_COLOR,
            samples: between.iter().map(|&(a, b)| pair(a, b)).collect(),
            values: between.iter().map(|&(a, b)| matrix.get(a, b)).collect(),
        });
    }
    if groups.is_empty() {
        return Figure::message("Distances need at least two samples");
    }
    let label = format!("{} distance", distance.name());
    let data = FeatureData {
        // Kept short: long titles are cut off.
        feature: match community.table.depth {
            Some(depth) => format!("{label} (rarefied to {depth})"),
            None => format!("{label} within and between groups"),
        },
        field: Field::Proportion,
        groups,
        p_value: None,
        p_value_kind: None,
        value_label: Some(label),
    };
    feature_plots::box_plot(&data, options)
}

/// What a slow community calculation reports before each piece of work:
/// what it is doing (for example "PERMANOVA"), and how many of how many
/// pieces of that are done (a total of zero when it cannot be told).
pub type StagePace<'a> = &'a mut dyn FnMut(&str, usize, usize) -> Control;

/// The slow part of a community plot, computed by a [`CommunityRun`].
#[derive(Debug, Clone, PartialEq)]
pub enum CommunityJob {
    /// The PCoA layout, with the community tests if this gives their
    /// number of permutations.
    Pcoa { tests: Option<usize> },
    /// The NMDS layout.
    Nmds,
    /// The distances alone (the distance box plot).
    Distances,
    /// The Mantel test against the differences of a number known for each
    /// sample.
    Mantel {
        values: Vec<f64>,
        method: CorrelationMethod,
        permutations: usize,
    },
}

/// What a [`CommunityRun`] finds.
#[derive(Debug, Clone, PartialEq)]
pub enum CommunityOutput {
    Layout(Result<Layout, String>),
    Distances(DistanceMatrix),
    Mantel {
        matrix: DistanceMatrix,
        differences: DistanceMatrix,
        test: PermutationTest,
    },
    /// Nothing could be computed, for the reason given.
    Unavailable(String),
}

/// The slow part of a community plot (distances, ordination, permutation
/// tests), carried out a piece at a time, so that whatever runs it can show
/// what it is doing, attend to other things in between, or give up. The
/// outcome does not depend on where it pauses. The PCoA itself, and the
/// PCoA that NMDS starts from, are one piece each.
pub struct CommunityRun {
    community: Community,
    distance: Distance,
    rng: StampRng,
    job: CommunityJob,
    distances: DistanceRun,
    stage: Option<Stage>,
}

enum Stage {
    Pcoa(PcoaStage),
    Nmds(DistanceMatrix, NmdsRun),
    Mantel {
        matrix: DistanceMatrix,
        differences: DistanceMatrix,
        run: MantelRun,
    },
}

/// The PCoA and the community tests that follow it.
enum PcoaStage {
    /// Waiting to find the principal coordinates.
    Start {
        matrix: DistanceMatrix,
        permutations: Option<usize>,
    },
    Tests {
        matrix: DistanceMatrix,
        ordination: Ordination,
        permutations: usize,
        lines: Vec<String>,
        run: Option<PermutationRun>,
    },
}

impl CommunityRun {
    /// The `job` for `community` by `distance`, drawing random numbers
    /// (for the permutations) from `rng`.
    pub fn new(
        community: Community,
        distance: Distance,
        job: CommunityJob,
        rng: StampRng,
    ) -> CommunityRun {
        let distances = DistanceRun::new(&community.table.abundances, distance);
        CommunityRun {
            community,
            distance,
            rng,
            job,
            distances,
            stage: None,
        }
    }

    /// The community the run is for.
    pub fn community(&self) -> &Community {
        &self.community
    }

    /// The depth the samples were rarefied to, if they were.
    pub fn depth(&self) -> Option<u64> {
        self.community.table.depth
    }

    /// Carry the work further. `pace` is asked before each piece.
    pub fn advance(&mut self, pace: StagePace<'_>) -> Step<CommunityOutput> {
        let (community, distance) = (&self.community, self.distance);
        if self.stage.is_none() {
            if let CommunityJob::Mantel { values, .. } = &self.job {
                if values.len() != community.samples.len() || values.len() < 3 {
                    return Step::Done(CommunityOutput::Unavailable(
                        "The Mantel test needs at least three samples".to_string(),
                    ));
                }
            }
            let label = format!("Computing {} distances", distance.name());
            let matrix = match self
                .distances
                .advance(&mut |done, total| pace(&label, done, total))
            {
                Step::Done(matrix) => matrix,
                Step::Paused => return Step::Paused,
                Step::Cancelled => return Step::Cancelled,
            };
            self.stage = Some(match &self.job {
                CommunityJob::Pcoa { tests } => Stage::Pcoa(PcoaStage::Start {
                    matrix,
                    permutations: *tests,
                }),
                CommunityJob::Nmds => Stage::Nmds(matrix, NmdsRun::new(2)),
                CommunityJob::Distances => return Step::Done(CommunityOutput::Distances(matrix)),
                CommunityJob::Mantel {
                    values,
                    method,
                    permutations,
                } => {
                    let differences = differences_of(values);
                    let run = MantelRun::new(&differences, &matrix, *method, *permutations);
                    Stage::Mantel {
                        matrix,
                        differences,
                        run,
                    }
                }
            });
        }
        let Some(stage) = self.stage.as_mut() else {
            unreachable!("the stage was set above");
        };
        match stage {
            Stage::Pcoa(stage) => stage.advance(community, distance, &mut self.rng, pace),
            Stage::Nmds(matrix, run) => {
                match run.advance(matrix, &mut |done, total| {
                    if done == 0 {
                        pace("Arranging the samples by NMDS", 0, 0)
                    } else {
                        pace("NMDS iterations", done, total)
                    }
                }) {
                    Step::Done(layout) => Step::Done(CommunityOutput::Layout(nmds_result(
                        community, distance, layout,
                    ))),
                    Step::Paused => Step::Paused,
                    Step::Cancelled => Step::Cancelled,
                }
            }
            Stage::Mantel {
                matrix,
                differences,
                run,
            } => match run.advance(&mut self.rng, &mut |done, total| {
                pace("Mantel test", done, total)
            }) {
                Step::Done(test) => Step::Done(CommunityOutput::Mantel {
                    matrix: std::mem::replace(matrix, DistanceMatrix::from_fn(0, |_, _| 0.0)),
                    differences: std::mem::replace(
                        differences,
                        DistanceMatrix::from_fn(0, |_, _| 0.0),
                    ),
                    test,
                }),
                Step::Paused => Step::Paused,
                Step::Cancelled => Step::Cancelled,
            },
        }
    }
}

impl PcoaStage {
    fn advance(
        &mut self,
        community: &Community,
        distance: Distance,
        rng: &mut StampRng,
        pace: StagePace<'_>,
    ) -> Step<CommunityOutput> {
        if let PcoaStage::Start {
            matrix,
            permutations,
        } = self
        {
            match pace("Finding the principal coordinates", 0, 0) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            // PERMDISP needs every axis; the plot alone, only the first few.
            let ordination = if permutations.is_some() {
                pcoa(matrix)
            } else {
                pcoa_leading(matrix, PLOTTED_AXES)
            };
            let Some(ordination) = ordination else {
                return Step::Done(CommunityOutput::Layout(Err(
                    "Degenerate PCoA plot: the samples are all the same by this distance"
                        .to_string(),
                )));
            };
            let Some(permutations) = *permutations else {
                return Step::Done(CommunityOutput::Layout(Ok(pcoa_result(
                    community, distance, ordination, None,
                ))));
            };
            let matrix = std::mem::replace(matrix, DistanceMatrix::from_fn(0, |_, _| 0.0));
            *self = PcoaStage::Tests {
                matrix,
                ordination,
                permutations,
                lines: Vec::new(),
                run: None,
            };
        }
        let PcoaStage::Tests {
            matrix,
            ordination,
            permutations,
            lines,
            run,
        } = self
        else {
            unreachable!("the PCoA was found above");
        };
        let groups = community.dense_groups();
        while lines.len() < CommunityTest::ALL.len() {
            let test = CommunityTest::ALL[lines.len()];
            let current = run.get_or_insert_with(|| {
                test.start(matrix, &groups, *permutations, Some(&*ordination))
            });
            let result =
                match current.advance(rng, &mut |done, total| pace(test.name(), done, total)) {
                    Step::Done(result) => result,
                    Step::Paused => return Step::Paused,
                    Step::Cancelled => return Step::Cancelled,
                };
            lines.push(test_line(test, &result));
            *run = None;
        }
        let ordination = std::mem::replace(
            ordination,
            Ordination {
                positions: Vec::new(),
                variance_fractions: Vec::new(),
            },
        );
        Step::Done(CommunityOutput::Layout(Ok(pcoa_result(
            community,
            distance,
            ordination,
            Some(std::mem::take(lines)),
        ))))
    }
}

/// The PCoA layout from the principal coordinates and the outcome of the
/// tests.
fn pcoa_result(
    community: &Community,
    distance: Distance,
    ordination: Ordination,
    tests: Option<Vec<String>>,
) -> Layout {
    Layout {
        positions: ordination
            .positions
            .into_iter()
            .map(|mut row| {
                row.truncate(PLOTTED_AXES);
                row
            })
            .collect(),
        variance_fractions: ordination.variance_fractions,
        notes: vec![distance_note(community, distance)],
        tests,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use stamp_core::stats::rng_from_seed;

    /// Two groups of three samples that differ, at genus level.
    fn community(normalisation: Normalisation) -> Community {
        let spf = "Phylum\tGenus\tA1\tA2\tA3\tB1\tB2\tB3\n\
            P1\ta\t50\t45\t55\t5\t8\t4\n\
            P1\tb\t30\t35\t25\t10\t12\t9\n\
            P1\tunclassified\t10\t5\t0\t20\t10\t5\n\
            P2\tc\t15\t10\t20\t60\t55\t70\n\
            P2\td\t4\t0\t6\t20\t25\t15\n\
            P2\te\t1\t0\t0\t5\t0\t2\n";
        let tree = ProfileTree::read(spf).unwrap();
        let samples: Vec<String> = ["A1", "A2", "A3", "B1", "B2", "B3"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        Community::from_tree(
            &tree,
            &samples,
            "Genus",
            vec![0, 0, 0, 1, 1, 1],
            vec!["A".into(), "B".into()],
            normalisation,
            &mut rng_from_seed(1),
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
    fn alpha_diversity_by_group() {
        let community = community(Normalisation::Proportions);
        // Unclassified genera are not counted.
        assert_eq!(
            alpha_values(&community, AlphaMetric::Observed),
            [5.0, 3.0, 4.0, 5.0, 4.0, 5.0]
        );
        let figure = alpha_plot(&community, AlphaMetric::Shannon, &BoxOptions::default());
        let texts = texts(&figure);
        for expected in ["Shannon index", "A", "B"] {
            assert!(
                texts.iter().any(|t| t == expected),
                "missing {expected} in {texts:?}"
            );
        }
        // Rarefied to the 95 sequences of A2, the plot says so, and no sample
        // has more than that.
        let rarefied = self::community(Normalisation::Rarefy);
        assert_eq!(rarefied.table.depth, Some(95));
        assert!(rarefied
            .table
            .abundances
            .iter()
            .all(|sample| sample.iter().sum::<f64>() <= 95.0));
        let figure = alpha_plot(&rarefied, AlphaMetric::Chao1, &BoxOptions::default());
        assert!(self::texts(&figure)
            .iter()
            .any(|t| t.contains("rarefied to 95 sequences")));
    }

    #[test]
    fn pcoa_separates_the_groups_and_reports_the_tests() {
        let community = community(Normalisation::Proportions);
        let mut rng = rng_from_seed(3);
        let figure = pcoa_plot(
            &community,
            Distance::BrayCurtis,
            Some((199, &mut rng)),
            &ordination_plot::Options {
                show_pc3_vs_pc2: false,
                show_pc1_vs_pc3: false,
                ..ordination_plot::Options::default()
            },
        );
        assert_eq!(figure.regions.len(), 6);
        // The first axis puts the three samples of A on one side.
        let x = |name: &str| figure.regions.iter().find(|r| r.item == name).unwrap().x;
        let (a, b) = (["A1", "A2", "A3"].map(x), ["B1", "B2", "B3"].map(x));
        let between = |v: f64, lo: f64, hi: f64| v >= lo && v <= hi;
        let (a_lo, a_hi) = (
            a.iter().copied().fold(f64::MAX, f64::min),
            a.iter().copied().fold(f64::MIN, f64::max),
        );
        assert!(b.iter().all(|&v| !between(v, a_lo, a_hi)));
        let texts = texts(&figure);
        assert!(texts.iter().any(|t| t.starts_with("PCo1 (")));
        assert!(texts.iter().any(|t| t == "Bray-Curtis distance"));
        for test in [
            "PERMANOVA: pseudo-F = ",
            "ANOSIM: R = 1.000, p = ",
            "PERMDISP: F = ",
        ] {
            assert!(
                texts.iter().any(|t| t.starts_with(test)),
                "missing {test} in {texts:?}"
            );
        }
        // Without tests only the distance is named.
        let plain = pcoa_plot(
            &community,
            Distance::Jaccard,
            None,
            &ordination_plot::Options::default(),
        );
        assert!(!self::texts(&plain)
            .iter()
            .any(|t| t.starts_with("PERMANOVA")));
    }

    #[test]
    fn distances_within_and_between_groups() {
        let community = community(Normalisation::Proportions);
        let figure = distance_box_plot(&community, Distance::BrayCurtis, &BoxOptions::default());
        let texts = texts(&figure);
        for expected in ["A", "B", "Between groups", "Bray-Curtis distance"] {
            assert!(
                texts.iter().any(|t| t == expected),
                "missing {expected} in {texts:?}"
            );
        }
        // One sample cannot be compared with anything.
        let mut single = community.clone();
        single.samples.truncate(1);
        single.sample_groups.truncate(1);
        single.table.abundances.truncate(1);
        assert_eq!(
            distance_box_plot(&single, Distance::BrayCurtis, &BoxOptions::default())
                .shapes
                .len(),
            1
        );
    }

    #[test]
    fn nmds_and_mantel_plots() {
        let community = community(Normalisation::Proportions);
        let figure = nmds_plot(
            &community,
            Distance::BrayCurtis,
            &ordination_plot::Options::default(),
        );
        assert_eq!(figure.regions.len(), 6);
        let shown = texts(&figure);
        assert!(
            shown.iter().any(|t| t.starts_with("Stress = ")),
            "{shown:?}"
        );
        assert!(shown.contains(&"NMDS1".to_string()) && shown.contains(&"NMDS2".to_string()));

        let values = [1.0, 2.0, 3.0, 10.0, 11.0, 12.0];
        let figure = mantel_plot(
            &community,
            &values,
            "Depth",
            Distance::BrayCurtis,
            CorrelationMethod::Spearman,
            99,
            &mut rng_from_seed(1),
            &correlation_plot::Options::default(),
        );
        // A point for each of the 15 pairs of samples.
        assert_eq!(figure.regions.len(), 15);
        let shown = texts(&figure);
        assert!(
            shown.iter().any(|t| t.starts_with("Mantel test: rho = 0.")),
            "{shown:?}"
        );
        assert!(shown.contains(&"Difference in Depth".to_string()));
        assert!(shown.contains(&"Bray-Curtis distance".to_string()));
        assert!(figure.regions[0].tooltip.contains("A1 : A2"));
        let short = mantel_plot(
            &community,
            &values[..2],
            "Depth",
            Distance::BrayCurtis,
            CorrelationMethod::Pearson,
            9,
            &mut rng_from_seed(1),
            &correlation_plot::Options::default(),
        );
        assert!(short.regions.is_empty());
    }

    /// The slow part of the community plots, carried out a piece at a time
    /// and paused often, gives what the plots compute in one go, and says
    /// what it is doing as it goes.
    #[test]
    fn community_runs_give_the_same_wherever_they_pause() {
        let community = community(Normalisation::Proportions);
        let run_paused = |job: CommunityJob| {
            let mut run = CommunityRun::new(
                community.clone(),
                Distance::BrayCurtis,
                job,
                rng_from_seed(9),
            );
            let mut stages: Vec<String> = Vec::new();
            let mut calls = 0;
            loop {
                let step = run.advance(&mut |stage, done, total| {
                    assert!(total == 0 || done <= total);
                    if stages.last().map(String::as_str) != Some(stage) {
                        stages.push(stage.to_string());
                    }
                    calls += 1;
                    if calls % 2 == 0 {
                        Control::Pause
                    } else {
                        Control::Continue
                    }
                });
                match step {
                    Step::Done(output) => return (output, stages),
                    Step::Paused => {}
                    Step::Cancelled => panic!("cancelled"),
                }
            }
        };
        let (output, stages) = run_paused(CommunityJob::Pcoa { tests: Some(99) });
        let mut rng = rng_from_seed(9);
        let whole = pcoa_layout(&community, Distance::BrayCurtis, Some((99, &mut rng)));
        assert_eq!(output, CommunityOutput::Layout(whole));
        assert_eq!(
            stages,
            [
                "Computing Bray-Curtis distances",
                "Finding the principal coordinates",
                "PERMANOVA",
                "ANOSIM",
                "PERMDISP"
            ]
        );

        let (output, stages) = run_paused(CommunityJob::Nmds);
        let whole = nmds_layout(&community, Distance::BrayCurtis);
        // Compared as text: the variance fractions of NMDS are NaN.
        assert_eq!(
            format!("{output:?}"),
            format!("{:?}", CommunityOutput::Layout(whole))
        );
        assert_eq!(stages[1], "Arranging the samples by NMDS");

        let (output, _) = run_paused(CommunityJob::Distances);
        let matrix = distances(&community.table.abundances, Distance::BrayCurtis);
        assert_eq!(output, CommunityOutput::Distances(matrix.clone()));

        let values = vec![1.0, 2.0, 4.0, 8.0, 9.0, 3.0];
        let (output, stages) = run_paused(CommunityJob::Mantel {
            values: values.clone(),
            method: CorrelationMethod::Spearman,
            permutations: 99,
        });
        let differences = differences_of(&values);
        let test = mantel(
            &differences,
            &matrix,
            CorrelationMethod::Spearman,
            99,
            &mut rng_from_seed(9),
        );
        assert_eq!(
            output,
            CommunityOutput::Mantel {
                matrix,
                differences,
                test
            }
        );
        assert_eq!(stages.last().map(String::as_str), Some("Mantel test"));
    }
}
