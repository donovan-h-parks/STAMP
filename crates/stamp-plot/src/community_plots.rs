//! Plots of whole communities: the diversity within samples, the samples
//! placed by the distances between them, and those distances within and
//! between groups.

use stamp_core::stats::community::{
    mantel, CommunityTest, MantelRun, PairwiseRun, PermutationRun, PermutationTest,
};
use stamp_core::stats::constrained::{
    self, ConstrainedMethod, ConstrainedModel, ConstrainedOrdination, ConstrainedTest,
    ConstrainedTestRun, ConstrainedTests, Variable,
};
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::diversity::{
    nmds, pcoa, pcoa_leading, AlphaMetric, CommunityTable, Distance, DistanceMatrix, DistanceRun,
    NmdsRun, Normalisation, Ordination,
};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::results::format_p_value;
use stamp_core::stats::results::{Control, Step};
use stamp_core::stats::two_group::GroupTest;
use stamp_core::stats::StampRng;
use stamp_core::{ProfileTree, Unclassified};

use crate::axes::group_color;
use crate::feature_plots::{self, BoxOptions, FeatureData, Field, Group};
use crate::figure::{Color, Figure};
use crate::{correlation_plot, ordination_plot};

/// The methods of the ordination plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum OrdinationMethod {
    /// Principal coordinates analysis of a distance.
    Pcoa,
    /// Non-metric multidimensional scaling of a distance.
    Nmds,
    /// Distance-based redundancy analysis: the part of a distance that
    /// explanatory variables account for.
    DbRda,
    /// Canonical correspondence analysis of the abundances on explanatory
    /// variables.
    Cca,
}

impl OrdinationMethod {
    pub const ALL: [OrdinationMethod; 4] = [
        OrdinationMethod::Pcoa,
        OrdinationMethod::Nmds,
        OrdinationMethod::DbRda,
        OrdinationMethod::Cca,
    ];

    /// The short name, as on the plot.
    pub fn name(self) -> &'static str {
        match self {
            OrdinationMethod::Pcoa => "PCoA",
            OrdinationMethod::Nmds => "NMDS",
            OrdinationMethod::DbRda => "db-RDA",
            OrdinationMethod::Cca => "CCA",
        }
    }

    /// The name in full, for a list to choose from.
    pub fn long_name(self) -> &'static str {
        match self {
            OrdinationMethod::Pcoa => "PCoA: principal coordinates",
            OrdinationMethod::Nmds => "NMDS: non-metric multidimensional scaling",
            OrdinationMethod::DbRda => "db-RDA: distance-based redundancy analysis",
            OrdinationMethod::Cca => "CCA: canonical correspondence analysis",
        }
    }

    /// The constrained method, for those that relate the samples to
    /// explanatory variables.
    pub fn constrained(self) -> Option<ConstrainedMethod> {
        match self {
            OrdinationMethod::DbRda => Some(ConstrainedMethod::DbRda),
            OrdinationMethod::Cca => Some(ConstrainedMethod::Cca),
            OrdinationMethod::Pcoa | OrdinationMethod::Nmds => None,
        }
    }

    /// Whether the method works on the distances between samples (CCA
    /// works on the abundances).
    pub fn uses_distance(self) -> bool {
        self != OrdinationMethod::Cca
    }
}

/// The explanatory variables of a constrained ordination for some samples:
/// which samples have a value for every variable, and the variables for
/// those samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Explanatory {
    /// Indices of the samples kept, in order.
    pub kept: Vec<usize>,
    pub variables: Vec<Variable>,
    /// A note on the samples left out, if any were.
    pub note: Option<String>,
}

/// The metadata fields `fields` as explanatory variables of `samples`. A
/// field of numbers (as the colours of the ordination plots take it: an
/// occasional other value, such as "unknown", is then missing) is numeric;
/// any other is categorical. Samples without a value for every field are
/// left out.
pub fn explanatory_variables(
    metadata: &stamp_core::Metadata,
    fields: &[String],
    samples: &[String],
) -> Result<Explanatory, String> {
    use crate::ordination_style::{is_missing, is_numeric_field, number};
    if fields.is_empty() {
        return Err("Choose at least one explanatory variable".to_string());
    }
    let known = metadata.fields();
    // Each field's value for each sample: a number, a category, or none.
    let mut columns: Vec<Vec<Option<constrained::Values>>> = Vec::new();
    for field in fields {
        if !known.contains(field) {
            return Err(format!("The metadata has no field \"{field}\""));
        }
        let numeric = is_numeric_field(metadata, field);
        columns.push(
            samples
                .iter()
                .map(|s| {
                    let value = metadata.value(s, field).filter(|v| !is_missing(v))?;
                    if numeric {
                        number(value).map(|x| constrained::Values::Numbers(vec![x]))
                    } else {
                        Some(constrained::Values::Categories(vec![value
                            .trim()
                            .to_string()]))
                    }
                })
                .collect(),
        );
    }
    let kept: Vec<usize> = (0..samples.len())
        .filter(|&i| columns.iter().all(|column| column[i].is_some()))
        .collect();
    let left_out = samples.len() - kept.len();
    let variables = fields
        .iter()
        .zip(&columns)
        .map(|(field, column)| {
            let (mut numbers, mut categories) = (Vec::new(), Vec::new());
            for &i in &kept {
                match &column[i] {
                    Some(constrained::Values::Numbers(x)) => numbers.extend(x),
                    Some(constrained::Values::Categories(c)) => categories.extend(c.clone()),
                    None => {}
                }
            }
            Variable {
                name: field.clone(),
                values: if categories.is_empty() {
                    constrained::Values::Numbers(numbers)
                } else {
                    constrained::Values::Categories(categories)
                },
            }
        })
        .collect();
    let note = (left_out > 0).then(|| {
        format!(
            "{left_out} sample{} without a value for every variable left out",
            if left_out == 1 { "" } else { "s" }
        )
    });
    Ok(Explanatory {
        kept,
        variables,
        note,
    })
}

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
    /// The hierarchical level of the features, which the alpha diversity
    /// plot names on its axis (empty if not known).
    pub profile_level: String,
    pub table: CommunityTable,
}

impl Community {
    /// The community of some samples of a profile tree at a hierarchical
    /// level, with the treatment of unclassified features that the feature
    /// tests use, so that every analysis compares the same profiles.
    #[allow(clippy::too_many_arguments)]
    pub fn from_tree(
        tree: &ProfileTree,
        samples: &[String],
        profile_level: &str,
        unclassified: Unclassified,
        sample_groups: Vec<usize>,
        groups: Vec<String>,
        normalisation: Normalisation,
        rng: &mut StampRng,
    ) -> Result<Community, String> {
        let names: Vec<&str> = samples.iter().map(String::as_str).collect();
        let profile = tree
            .profile(&names, None, profile_level, unclassified)
            .map_err(|e| e.to_string())?;
        Ok(Community {
            samples: samples.to_vec(),
            sample_groups,
            groups,
            group_colors: Vec::new(),
            styling: Default::default(),
            profile_level: profile_level.to_string(),
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

    /// The names of the groups as [`dense_groups`](Self::dense_groups)
    /// numbers them.
    fn dense_group_names(&self) -> Vec<String> {
        let mut used: Vec<usize> = self.sample_groups.clone();
        used.sort_unstable();
        used.dedup();
        used.iter().map(|&g| self.groups[g].clone()).collect()
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
    (0..community.table.abundances.len())
        .map(|sample| metric.value(&community.table.alpha_abundances(sample)))
        .collect()
}

/// The test of the analysis, which the alpha diversity plot runs on the
/// diversity of the samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaTest {
    TwoGroups(GroupTest),
    MultipleGroups(MultiGroupTest),
}

impl AlphaTest {
    /// The p-value of a difference in `values` between the groups, and the
    /// test's name as the plot gives it: the analysis's test where it can
    /// take a measure such as diversity, otherwise the rank test for as
    /// many groups, saying which test it stands in for. The values of
    /// paired (matched) groups are in pair order.
    pub fn p_value(self, values: &[&[f64]]) -> (f64, String) {
        let matched = values.windows(2).all(|w| w[0].len() == w[1].len());
        let (used, instead) = match self {
            AlphaTest::TwoGroups(test) if values.len() == 2 => {
                let (used, instead) = match test {
                    GroupTest::FisherPresence => {
                        (GroupTest::MannWhitney, Some("Fisher's exact test"))
                    }
                    GroupTest::McNemar => (GroupTest::MannWhitney, Some("McNemar's test")),
                    GroupTest::White => (GroupTest::MannWhitney, Some("White's test")),
                    GroupTest::PairedT if !matched => (GroupTest::Welch, Some("the paired t-test")),
                    GroupTest::WilcoxonSignedRank if !matched => (
                        GroupTest::MannWhitney,
                        Some("the Wilcoxon signed-rank test"),
                    ),
                    test => (test, None),
                };
                let p = used
                    .p_values(values[0], values[1])
                    .map_or(1.0, |(_, two_sided)| two_sided);
                return (p, label(two_group_name(used), instead));
            }
            AlphaTest::TwoGroups(_) => (MultiGroupTest::KruskalWallis, None),
            AlphaTest::MultipleGroups(MultiGroupTest::ChiSquarePresence) => {
                (MultiGroupTest::KruskalWallis, Some("the chi-square test"))
            }
            AlphaTest::MultipleGroups(MultiGroupTest::Friedman) if !matched => {
                (MultiGroupTest::KruskalWallis, Some("the Friedman test"))
            }
            AlphaTest::MultipleGroups(test) => (test, None),
        };
        let (p, _) = used.run(values);
        (p, label(used.name(), instead))
    }
}

/// The name of a two-group test as the alpha diversity plot gives it.
fn two_group_name(test: GroupTest) -> &'static str {
    match test {
        GroupTest::TTest => "t-test",
        GroupTest::PairedT => "paired t-test",
        test => test.name(),
    }
}

fn label(name: &str, instead: Option<&str>) -> String {
    match instead {
        Some(other) => format!("{name}, in place of {other}"),
        None => name.to_string(),
    }
}

/// Box plot of an alpha diversity measure in each group, with the p-value
/// of the analysis's test of a difference between the groups (see
/// [`AlphaTest::p_value`]).
pub fn alpha_plot(
    community: &Community,
    metric: AlphaMetric,
    test: AlphaTest,
    options: &BoxOptions,
) -> Figure {
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
        let (p, name) = test.p_value(&slices);
        // Named, since nothing else on the plot says which test it is.
        format!("{} ({name})", format_p_value(p, -15))
    });
    // The value axis names the measure and the level it is of ("Simpson
    // index (Genus)"). A title would only repeat it, so there is one only
    // to say that the samples were rarefied.
    let label = if community.profile_level.is_empty() {
        metric.name().to_string()
    } else {
        format!("{} ({})", metric.name(), community.profile_level)
    };
    let data = FeatureData {
        feature: community.depth_note().unwrap_or_default(),
        field: Field::Proportion,
        groups,
        p_value,
        // One test, of the groups' diversity: no correction applies.
        p_value_kind: None,
        value_label: Some(label),
        paired: false,
    };
    feature_plots::box_plot(&data, options)
}

/// An alpha diversity measure of each sample against a number known for
/// each (`values[i]` for sample `i`), with their correlation by `method`:
/// what [`correlation_plot::plot`] draws. The samples are coloured by the
/// community's groups if it has more than one. `Err` holds a message for
/// the plot.
pub fn alpha_correlation(
    community: &Community,
    metric: AlphaMetric,
    field: &str,
    values: &[f64],
    method: CorrelationMethod,
) -> Result<correlation_plot::Data, String> {
    let diversity = alpha_values(community, metric);
    if diversity.iter().any(|v| v.is_nan()) {
        return Err(
            "The Chao1 estimate needs counts of sequences, and this profile holds fractional values"
                .to_string(),
        );
    }
    let pairs: (Vec<f64>, Vec<f64>) = values
        .iter()
        .zip(&diversity)
        .filter(|(v, _)| v.is_finite())
        .map(|(&v, &d)| (v, d))
        .unzip();
    let statistics = match method.coefficient(&pairs.0, &pairs.1) {
        Some(coefficient) => format!(
            "{} = {:.3}, p = {} (n = {})",
            match method {
                CorrelationMethod::Pearson => "Pearson r",
                CorrelationMethod::Spearman => "Spearman rho",
            },
            coefficient,
            format_p_value(
                stamp_core::stats::correlation::correlation_p_value(coefficient, pairs.0.len()),
                -15
            ),
            pairs.0.len()
        ),
        None => "No correlation: too few samples, or values that do not vary".to_string(),
    };
    let grouped = community.groups.len() > 1;
    Ok(correlation_plot::Data {
        feature: match community.depth_note() {
            Some(note) => format!("{} ({})", metric.name(), note.to_lowercase()),
            None => metric.name().to_string(),
        },
        field: field.to_string(),
        points: community
            .samples
            .iter()
            .zip(values)
            .zip(&diversity)
            .enumerate()
            .map(
                |(i, ((sample, &value), &diversity))| correlation_plot::Point {
                    sample: sample.clone(),
                    value,
                    percent: diversity,
                    group: grouped.then(|| community.sample_groups[i]),
                },
            )
            .collect(),
        groups: if grouped {
            community.groups.clone()
        } else {
            Vec::new()
        },
        group_colors: if grouped {
            (0..community.groups.len())
                .map(|g| community.group_color(g))
                .collect()
        } else {
            Vec::new()
        },
        statistics: Some(statistics),
        value_label: Some(metric.name().to_string()),
    })
}

/// The outcome of the community tests on a set of distances, as lines of
/// text.
pub fn test_notes(
    community: &Community,
    distance: Distance,
    permutations: usize,
    rng: &mut StampRng,
) -> Vec<String> {
    let matrix = community.table.distances(distance);
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
            test_line(*test, &result, &groups)
        })
        .collect()
}

/// The share of the variation among the samples that the groups account
/// for (R squared), from the pseudo-F ratio of a PERMANOVA of `samples`
/// samples in `groups` groups: the sum of squares between the groups over
/// the total, which is F (k - 1) / (F (k - 1) + (n - k)).
pub fn permanova_r2(pseudo_f: f64, samples: usize, groups: usize) -> f64 {
    let between = pseudo_f * (groups as f64 - 1.0);
    between / (between + (samples as f64 - groups as f64))
}

/// The outcome of a community test as a line of text. `groups` is the
/// group of each sample, numbered from zero. PERMANOVA also gives R
/// squared, the size of the difference its p-value is about.
fn test_line(test: CommunityTest, result: &PermutationTest, groups: &[usize]) -> String {
    if result.statistic.is_nan() {
        format!(
            "{}: needs at least two groups, and more samples than groups",
            test.name()
        )
    } else {
        let size = match test {
            CommunityTest::Permanova => {
                let count = groups.iter().copied().max().map_or(0, |g| g + 1);
                format!(
                    ", R\u{b2} = {:.3}",
                    permanova_r2(result.statistic, groups.len(), count)
                )
            }
            _ => String::new(),
        };
        format!(
            "{}: {} = {:.3}{size}, p = {} ({} permutations)",
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
    /// The names of the plotted axes.
    pub axis_names: Vec<String>,
    /// The explanatory variables of a constrained ordination.
    pub biplot: ordination_plot::Biplot,
    /// Notes above the plot: the distance, rarefying, the NMDS stress, and
    /// what the variables of a constrained ordination explain.
    pub notes: Vec<String>,
    /// The outcome of each test, if they were run: the community tests
    /// (PCoA), or the permutation tests of a constrained ordination.
    pub tests: Option<Vec<String>>,
    /// PERMANOVA between every pair of groups, if the tests were run and
    /// there are three or more groups.
    pub pairwise: Vec<Pairwise>,
}

/// PERMANOVA between two of the groups.
#[derive(Debug, Clone, PartialEq)]
pub struct Pairwise {
    pub first: String,
    pub second: String,
    /// The pseudo-F ratio; `NaN` if the two groups have too few samples.
    pub statistic: f64,
    pub p_value: f64,
    /// Corrected for the number of pairs by Holm's method.
    pub p_value_corrected: f64,
}

/// Pairs of groups beyond which a plot counts those that differ instead of
/// listing each.
const LISTED_PAIRS: usize = 6;

impl Pairwise {
    /// "A vs B: pseudo-F = 3.21, p = 0.012".
    pub fn line(&self) -> String {
        if self.statistic.is_nan() {
            format!("{} vs {}: too few samples", self.first, self.second)
        } else {
            format!(
                "{} vs {}: pseudo-F = {:.3}, p = {}",
                self.first,
                self.second,
                self.statistic,
                format_p_value(self.p_value_corrected, -15)
            )
        }
    }
}

/// The lines a plot shows for the pairwise tests: each pair if they are
/// few, and otherwise how many differ.
pub fn pairwise_notes(pairwise: &[Pairwise]) -> Vec<String> {
    const HEADING: &str = "Pairwise PERMANOVA (Holm-corrected p-values)";
    if pairwise.is_empty() {
        Vec::new()
    } else if pairwise.len() <= LISTED_PAIRS {
        std::iter::once(format!("{HEADING}:"))
            .chain(pairwise.iter().map(Pairwise::line))
            .collect()
    } else {
        let differ = pairwise
            .iter()
            .filter(|p| p.p_value_corrected <= 0.05)
            .count();
        vec![format!(
            "{HEADING}: {differ} of {} pairs of groups differ (p \u{2264} 0.05)",
            pairwise.len()
        )]
    }
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
    let matrix = community.table.distances(distance);
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
        notes.extend(pairwise_notes(&layout.pairwise));
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
        &layout.axis_names,
        &notes,
        &layout.biplot,
        options,
    )
}

/// The NMDS layout of a community by a distance, in two dimensions, with
/// its stress. `Err` holds a message for the plot.
pub fn nmds_layout(community: &Community, distance: Distance) -> Result<Layout, String> {
    let matrix = community.table.distances(distance);
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
        axis_names: ordination_plot::numbered_axes("NMDS", 2),
        biplot: Default::default(),
        notes: vec![
            distance_note(community, distance),
            format!("Stress = {:.3} ({quality})", layout.stress),
        ],
        tests: None,
        pairwise: Vec::new(),
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
        &layout.axis_names,
        &layout.notes,
        &layout.biplot,
        &options,
    )
}

/// A constrained ordination of a community on some explanatory variables
/// (db-RDA of the distances, or CCA of the abundances), with its
/// permutation tests if `tests` gives their number and the random numbers
/// for them. `Err` holds a message for the plot.
pub fn constrained_layout(
    community: &Community,
    distance: Distance,
    method: ConstrainedMethod,
    variables: &[Variable],
    tests: Option<(usize, &mut StampRng)>,
) -> Result<Layout, String> {
    let model = match method {
        ConstrainedMethod::DbRda => constrained::db_rda(
            &community.table.distances(distance),
            variables,
            PLOTTED_AXES,
        ),
        ConstrainedMethod::Cca => {
            constrained::cca(&community.table.abundances, variables, PLOTTED_AXES)
        }
    }?;
    let tests = tests.map(|(permutations, rng)| model.tests(permutations, rng));
    Ok(constrained_result(
        community,
        distance,
        &model.ordination,
        tests.as_ref(),
        &[],
    ))
}

/// The variables of a model joined for a note: "pH + Temp + Site".
fn joined_names(variables: &[String]) -> String {
    variables.join(" + ")
}

/// The line of a permutation test of a constrained ordination.
fn constrained_test_line(test: &ConstrainedTest, explained_of: &str) -> String {
    // The share the whole model explains is said on the line before.
    let (name, share) = if test.name.is_empty() {
        ("Model".to_string(), String::new())
    } else {
        (
            format!("{} given the others", test.name),
            format!("{:.1}% of the {explained_of}, ", test.explained * 100.0),
        )
    };
    if test.statistic.is_nan() {
        return format!("{name}: adds nothing to the other variables");
    }
    format!(
        "{name}: {share}pseudo-F = {:.3}, p = {} ({} permutations)",
        test.statistic,
        format_p_value(test.p_value, -15),
        test.permutations
    )
}

/// The layout of a constrained ordination: its axes, the variables drawn
/// over the samples, and the notes and tests above it.
fn constrained_result(
    community: &Community,
    distance: Distance,
    ordination: &ConstrainedOrdination,
    tests: Option<&ConstrainedTests>,
    notes: &[String],
) -> Layout {
    let explained_of = match ordination.method {
        ConstrainedMethod::DbRda => "variation",
        ConstrainedMethod::Cca => "inertia",
    };
    let mut lines: Vec<String> = notes.to_vec();
    lines.push(match ordination.method {
        ConstrainedMethod::DbRda => {
            let base = format!("db-RDA of {} distances", distance.name());
            match community.depth_note() {
                Some(note) => format!("{base}. {note}."),
                None => base,
            }
        }
        ConstrainedMethod::Cca => {
            let base = "CCA of the abundances (samples weighted by their totals)".to_string();
            match community.depth_note() {
                Some(note) => format!("{base}. {note}."),
                None => base,
            }
        }
    });
    let adjusted = ordination
        .adjusted
        .map(|r| format!(" (adjusted R\u{b2} = {r:.3})"))
        .unwrap_or_default();
    lines.push(format!(
        "{} explain {:.1}% of the {explained_of}{adjusted}",
        joined_names(&ordination.variables),
        ordination.explained * 100.0
    ));
    let test_lines = tests.map(|tests| {
        std::iter::once(&tests.overall)
            .chain(&tests.variables)
            .map(|t| constrained_test_line(t, explained_of))
            .collect()
    });
    let axis_names = &ordination.axis_names;
    let correlations = |values: &[f64]| -> String {
        values
            .iter()
            .zip(axis_names)
            .take(ordination.constrained_axes)
            .map(|(r, axis)| format!("{r:.2} with {axis}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let categorical: Vec<&String> = {
        let mut v: Vec<&String> = ordination.centroids.iter().map(|c| &c.variable).collect();
        v.dedup();
        v
    };
    let biplot = ordination_plot::Biplot {
        arrows: ordination
            .arrows
            .iter()
            .map(|a| ordination_plot::BiplotItem {
                name: a.name.clone(),
                position: a.position.clone(),
                tooltip: format!("{}\nCorrelation {}", a.name, correlations(&a.correlations)),
            })
            .collect(),
        centroids: ordination
            .centroids
            .iter()
            .map(|c| ordination_plot::BiplotItem {
                name: if categorical.len() > 1 {
                    format!("{}: {}", c.variable, c.category)
                } else {
                    c.category.clone()
                },
                position: c.position.clone(),
                tooltip: format!("{} = {}\nCentroid of its samples", c.variable, c.category),
            })
            .collect(),
    };
    Layout {
        positions: ordination.positions.clone(),
        variance_fractions: ordination.axis_fractions.clone(),
        axis_names: ordination.axis_names.clone(),
        biplot,
        notes: lines,
        tests: test_lines,
        pairwise: Vec::new(),
    }
}

/// The plot of a constrained ordination from its [`constrained_layout`].
pub fn draw_constrained(
    community: &Community,
    layout: &Layout,
    options: &ordination_plot::Options,
) -> Figure {
    let data = ordination_data(community);
    let mut notes = layout.notes.clone();
    if let Some(tests) = &layout.tests {
        notes.extend(tests.iter().cloned());
    }
    ordination_plot::plot_ordination(
        &data,
        &layout.positions,
        &layout.variance_fractions,
        &layout.axis_names,
        &notes,
        &layout.biplot,
        options,
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
    let matrix = community.table.distances(distance);
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
    let matrix = community.table.distances(distance);
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
        paired: false,
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
    /// A constrained ordination on some explanatory variables (a value for
    /// each sample of the community), with its permutation tests if this
    /// gives their number. `notes` are written above the plot first (for
    /// example, the samples left out for want of a value).
    Constrained {
        method: ConstrainedMethod,
        variables: Vec<Variable>,
        tests: Option<usize>,
        notes: Vec<String>,
    },
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
    /// The distances of a db-RDA, waiting to be fitted.
    DbRda(DistanceMatrix),
    Constrained {
        model: Box<ConstrainedModel>,
        run: Option<ConstrainedTestRun>,
    },
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
        /// The tests between pairs of groups, which follow the others.
        pairwise: Option<Box<PairwiseRun>>,
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
        let distances = community.table.distance_run(distance);
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
            if let CommunityJob::Constrained {
                method: ConstrainedMethod::Cca,
                variables,
                tests,
                ..
            } = &self.job
            {
                // CCA works on the abundances, not on distances.
                match pace("Fitting the CCA", 0, 0) {
                    Control::Continue => {}
                    Control::Pause => return Step::Paused,
                    Control::Cancel => return Step::Cancelled,
                }
                match constrained::cca(&community.table.abundances, variables, PLOTTED_AXES) {
                    Ok(model) => {
                        self.stage = Some(Stage::Constrained {
                            run: tests.map(|n| model.test_run(n)),
                            model: Box::new(model),
                        })
                    }
                    Err(message) => return Step::Done(CommunityOutput::Layout(Err(message))),
                }
            }
        }
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
                CommunityJob::Constrained { .. } => Stage::DbRda(matrix),
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
        if let Some(Stage::DbRda(matrix)) = &self.stage {
            match pace("Fitting the db-RDA", 0, 0) {
                Control::Continue => {}
                Control::Pause => return Step::Paused,
                Control::Cancel => return Step::Cancelled,
            }
            let CommunityJob::Constrained {
                variables, tests, ..
            } = &self.job
            else {
                unreachable!("a db-RDA stage comes from a constrained job");
            };
            match constrained::db_rda(matrix, variables, PLOTTED_AXES) {
                Ok(model) => {
                    self.stage = Some(Stage::Constrained {
                        run: tests.map(|n| model.test_run(n)),
                        model: Box::new(model),
                    })
                }
                Err(message) => return Step::Done(CommunityOutput::Layout(Err(message))),
            }
        }
        let Some(stage) = self.stage.as_mut() else {
            unreachable!("the stage was set above");
        };
        match stage {
            Stage::Pcoa(stage) => stage.advance(community, distance, &mut self.rng, pace),
            Stage::DbRda(_) => unreachable!("fitted above"),
            Stage::Constrained { model, run } => {
                let tests = match run {
                    Some(run) => match run.advance(&mut self.rng, pace) {
                        Step::Done(tests) => Some(tests),
                        Step::Paused => return Step::Paused,
                        Step::Cancelled => return Step::Cancelled,
                    },
                    None => None,
                };
                let CommunityJob::Constrained { notes, .. } = &self.job else {
                    unreachable!("a constrained stage comes from a constrained job");
                };
                Step::Done(CommunityOutput::Layout(Ok(constrained_result(
                    community,
                    distance,
                    &model.ordination,
                    tests.as_ref(),
                    notes,
                ))))
            }
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
                    community,
                    distance,
                    ordination,
                    None,
                    Vec::new(),
                ))));
            };
            let matrix = std::mem::replace(matrix, DistanceMatrix::from_fn(0, |_, _| 0.0));
            *self = PcoaStage::Tests {
                matrix,
                ordination,
                permutations,
                lines: Vec::new(),
                run: None,
                pairwise: None,
            };
        }
        let PcoaStage::Tests {
            matrix,
            ordination,
            permutations,
            lines,
            run,
            pairwise,
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
            lines.push(test_line(test, &result, &groups));
            *run = None;
        }
        // Which groups differ, when there are more than two. These draw
        // their random numbers after the tests above, which are therefore
        // the same with and without them.
        let group_names = community.dense_group_names();
        let pairs = if group_names.len() >= 3 {
            let current =
                pairwise.get_or_insert_with(|| Box::new(PairwiseRun::new(&groups, *permutations)));
            match current.advance(matrix, rng, &mut |done, total| {
                pace("Pairwise PERMANOVA", done, total)
            }) {
                Step::Done(tests) => tests
                    .into_iter()
                    .map(|t| Pairwise {
                        first: group_names[t.first].clone(),
                        second: group_names[t.second].clone(),
                        statistic: t.test.statistic,
                        p_value: t.test.p_value,
                        p_value_corrected: t.p_value_corrected,
                    })
                    .collect(),
                Step::Paused => return Step::Paused,
                Step::Cancelled => return Step::Cancelled,
            }
        } else {
            Vec::new()
        };
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
            pairs,
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
    pairwise: Vec<Pairwise>,
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
        axis_names: ordination_plot::numbered_axes(
            "PCo",
            ordination.variance_fractions.len().min(PLOTTED_AXES),
        ),
        variance_fractions: ordination.variance_fractions,
        biplot: Default::default(),
        notes: vec![distance_note(community, distance)],
        tests,
        pairwise,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use stamp_core::stats::rng_from_seed;

    const KRUSKAL: AlphaTest = AlphaTest::MultipleGroups(MultiGroupTest::KruskalWallis);

    #[test]
    fn the_alpha_diversity_plot_runs_the_analysis_test() {
        let (a, b, c) = (
            [1.0, 2.0, 3.0, 4.0],
            [2.5, 3.5, 4.5, 6.0],
            [5.0, 6.0, 7.0, 9.0],
        );
        let two: [&[f64]; 2] = [&a, &b];
        let two_sided = |test: GroupTest| test.p_values(&a, &b).unwrap().1;
        for (test, name) in [
            (GroupTest::TTest, "t-test"),
            (GroupTest::Welch, "Welch's t-test"),
            (GroupTest::MannWhitney, "Mann-Whitney U test"),
            // The samples of the two groups, in pair order.
            (GroupTest::PairedT, "paired t-test"),
            (GroupTest::WilcoxonSignedRank, "Wilcoxon signed-rank test"),
        ] {
            assert_eq!(
                AlphaTest::TwoGroups(test).p_value(&two),
                (two_sided(test), name.to_string())
            );
        }
        // Tests of presence, and White's test (of counts), cannot take a
        // diversity: the rank test stands in, and says so.
        for (test, name) in [
            (GroupTest::FisherPresence, "Fisher's exact test"),
            (GroupTest::McNemar, "McNemar's test"),
            (GroupTest::White, "White's test"),
        ] {
            assert_eq!(
                AlphaTest::TwoGroups(test).p_value(&two),
                (
                    two_sided(GroupTest::MannWhitney),
                    format!("Mann-Whitney U test, in place of {name}")
                )
            );
        }
        // Groups that are not in pairs cannot be compared pair by pair.
        let short = [1.0, 2.0, 3.0];
        let uneven: [&[f64]; 2] = [&a, &short];
        assert_eq!(
            AlphaTest::TwoGroups(GroupTest::PairedT).p_value(&uneven).1,
            "Welch's t-test, in place of the paired t-test"
        );

        let three: [&[f64]; 3] = [&a, &b, &c];
        for test in [
            MultiGroupTest::Anova,
            MultiGroupTest::WelchAnova,
            MultiGroupTest::KruskalWallis,
            MultiGroupTest::Friedman,
        ] {
            assert_eq!(
                AlphaTest::MultipleGroups(test).p_value(&three),
                (test.run(&three).0, test.name().to_string())
            );
        }
        assert_eq!(
            AlphaTest::MultipleGroups(MultiGroupTest::ChiSquarePresence).p_value(&three),
            (
                MultiGroupTest::KruskalWallis.run(&three).0,
                "Kruskal-Wallis H-test, in place of the chi-square test".to_string()
            )
        );
        let ragged: [&[f64]; 3] = [&a, &b, &short];
        assert_eq!(
            AlphaTest::MultipleGroups(MultiGroupTest::Friedman)
                .p_value(&ragged)
                .1,
            "Kruskal-Wallis H-test, in place of the Friedman test"
        );

        // The plot names the test it ran.
        let figure = alpha_plot(
            &community(Normalisation::Proportions),
            AlphaMetric::Shannon,
            AlphaTest::TwoGroups(GroupTest::Welch),
            &BoxOptions::default(),
        );
        assert!(
            texts(&figure)
                .iter()
                .any(|t| t.ends_with("(Welch's t-test)")),
            "{:?}",
            texts(&figure)
        );
    }

    /// Two groups of three samples that differ, at genus level, with the
    /// unclassified genus used only for calculating proportions (what
    /// STAMP starts with).
    fn community(normalisation: Normalisation) -> Community {
        community_with(normalisation, Unclassified::ProportionsOnly)
    }

    fn community_with(normalisation: Normalisation, unclassified: Unclassified) -> Community {
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
            unclassified,
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
        // The unclassified sequences of a sample, used only for calculating
        // proportions, count as one category (A3 has none).
        assert_eq!(
            alpha_values(&community, AlphaMetric::Observed),
            [6.0, 4.0, 4.0, 6.0, 5.0, 6.0]
        );
        // Removed, they are not counted; retained or pooled, they are the
        // one unclassified genus of this profile.
        let observed = |unclassified| {
            let community = community_with(Normalisation::Proportions, unclassified);
            alpha_values(&community, AlphaMetric::Observed)
        };
        assert_eq!(
            observed(Unclassified::Remove),
            [5.0, 3.0, 4.0, 5.0, 4.0, 5.0]
        );
        assert_eq!(observed(Unclassified::Pool), observed(Unclassified::Retain));
        assert_eq!(observed(Unclassified::Pool), [6.0, 4.0, 4.0, 6.0, 5.0, 6.0]);
        // The distances between samples follow the treatment too.
        let first = |unclassified| {
            community_with(Normalisation::Proportions, unclassified)
                .table
                .distances(Distance::BrayCurtis)
                .get(0, 3)
        };
        let all = Unclassified::ALL.map(first);
        assert!(all.iter().all(|d| (0.0..1.0).contains(d)));
        assert!((first(Unclassified::Remove) - first(Unclassified::ProportionsOnly)).abs() > 1e-6);
        assert!((first(Unclassified::Pool) - first(Unclassified::Remove)).abs() > 1e-6);
        let figure = alpha_plot(
            &community,
            AlphaMetric::Shannon,
            KRUSKAL,
            &BoxOptions::default(),
        );
        let texts = texts(&figure);
        // The value axis names the measure and the profile level.
        for expected in ["Shannon index (Genus)", "A", "B"] {
            assert!(
                texts.iter().any(|t| t == expected),
                "missing {expected} in {texts:?}"
            );
        }
        // No title repeats it: the measure is written once, and nothing
        // is drawn empty.
        assert_eq!(texts.iter().filter(|t| t.contains("Shannon")).count(), 1);
        assert!(texts.iter().all(|t| !t.is_empty()));
        // Without a level, the measure alone.
        let mut unnamed = community.clone();
        unnamed.profile_level.clear();
        let figure = alpha_plot(
            &unnamed,
            AlphaMetric::Shannon,
            KRUSKAL,
            &BoxOptions::default(),
        );
        assert!(self::texts(&figure).iter().any(|t| t == "Shannon index"));
        // Rarefied to the 95 sequences of A2, the plot says so, and no sample
        // has more than that.
        let rarefied = self::community(Normalisation::Rarefy);
        assert_eq!(rarefied.table.depth, Some(95));
        assert!(rarefied
            .table
            .abundances
            .iter()
            .all(|sample| sample.iter().sum::<f64>() <= 95.0));
        // The title is only the note that the samples were rarefied.
        let figure = alpha_plot(
            &rarefied,
            AlphaMetric::Chao1,
            KRUSKAL,
            &BoxOptions::default(),
        );
        let texts = self::texts(&figure);
        assert!(texts
            .iter()
            .any(|t| t == "Samples rarefied to 95 sequences"));
        assert_eq!(texts.iter().filter(|t| t.contains("Chao1")).count(), 1);
    }

    #[test]
    fn constrained_ordination_draws_the_variables_and_tests() {
        let community = community(Normalisation::Proportions);
        let variables = vec![
            Variable {
                name: "pH".to_string(),
                values: constrained::Values::Numbers(vec![5.0, 5.4, 4.8, 7.1, 6.8, 7.4]),
            },
            Variable {
                name: "Site".to_string(),
                values: constrained::Values::Categories(
                    ["x", "y", "x", "y", "x", "y"].map(String::from).to_vec(),
                ),
            },
        ];
        for method in [ConstrainedMethod::DbRda, ConstrainedMethod::Cca] {
            let mut rng = rng_from_seed(1);
            let layout = constrained_layout(
                &community,
                Distance::BrayCurtis,
                method,
                &variables,
                Some((99, &mut rng)),
            )
            .unwrap();
            let prefix = method.constrained_axis();
            assert_eq!(
                layout.axis_names[..2],
                [format!("{prefix}1"), format!("{prefix}2")]
            );
            // The third axis is the first unconstrained one.
            assert_eq!(
                layout.axis_names[2],
                format!("{}1", method.unconstrained_axis())
            );
            // The overall test and one per variable.
            assert_eq!(layout.tests.as_ref().unwrap().len(), 3);
            let figure = draw_constrained(&community, &layout, &Default::default());
            let texts = texts(&figure);
            assert!(
                texts.iter().any(|t| t.starts_with("pH + Site explain ")),
                "{texts:?}"
            );
            assert!(texts.iter().any(|t| t.starts_with("Model: ")));
            assert!(texts.iter().any(|t| t.starts_with("pH given the others: ")));
            for name in ["pH", "x", "y"] {
                assert!(texts.iter().any(|t| t == name), "missing {name}");
            }
            // The run gives the same layout, a piece at a time.
            let mut run = CommunityRun::new(
                community.clone(),
                Distance::BrayCurtis,
                CommunityJob::Constrained {
                    method,
                    variables: variables.clone(),
                    tests: Some(99),
                    notes: Vec::new(),
                },
                rng_from_seed(1),
            );
            let output = loop {
                if let Step::Done(output) = run.advance(&mut |_, _, _| Control::Continue) {
                    break output;
                }
            };
            assert_eq!(output, CommunityOutput::Layout(Ok(layout)));
        }
        // Too many variables for the samples is said on the plot.
        let names = Variable {
            name: "Sample".to_string(),
            values: constrained::Values::Categories(community.samples.clone()),
        };
        let error = constrained_layout(
            &community,
            Distance::BrayCurtis,
            ConstrainedMethod::DbRda,
            &[names],
            None,
        )
        .unwrap_err();
        assert!(error.contains("Too many"), "{error}");
    }

    #[test]
    fn pcoa_tests_every_pair_of_three_groups() {
        let mut community = community(Normalisation::Proportions);
        community.sample_groups = vec![0, 0, 1, 1, 2, 2];
        community.groups = vec!["A".into(), "B".into(), "C".into()];
        community.group_colors.clear();
        let layout = pcoa_layout(
            &community,
            Distance::BrayCurtis,
            Some((99, &mut rng_from_seed(3))),
        )
        .unwrap();
        let pairs: Vec<(&str, &str)> = layout
            .pairwise
            .iter()
            .map(|p| (p.first.as_str(), p.second.as_str()))
            .collect();
        assert_eq!(pairs, [("A", "B"), ("A", "C"), ("B", "C")]);
        let notes = pairwise_notes(&layout.pairwise);
        assert_eq!(notes[0], "Pairwise PERMANOVA (Holm-corrected p-values):");
        assert!(notes[1].starts_with("A vs B: pseudo-F = "), "{}", notes[1]);
        // The other tests are as they are without the pairs.
        let two = pcoa_layout(
            &self::community(Normalisation::Proportions),
            Distance::BrayCurtis,
            Some((99, &mut rng_from_seed(3))),
        )
        .unwrap();
        assert!(two.pairwise.is_empty());
        // Many pairs are counted, not listed.
        let many: Vec<Pairwise> = (0..10)
            .map(|i| Pairwise {
                first: "x".into(),
                second: "y".into(),
                statistic: 2.0,
                p_value: 0.01,
                p_value_corrected: if i < 3 { 0.02 } else { 0.5 },
            })
            .collect();
        assert_eq!(
            pairwise_notes(&many),
            ["Pairwise PERMANOVA (Holm-corrected p-values): 3 of 10 pairs of groups differ (p \u{2264} 0.05)"]
        );
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
        // PERMANOVA gives the size of the difference as well: the share of
        // the variation between the groups, which the pseudo-F implies.
        let line = texts.iter().find(|t| t.starts_with("PERMANOVA")).unwrap();
        let number = |after: &str| -> f64 {
            let rest = &line[line.find(after).unwrap() + after.len()..];
            rest.split([',', ' ']).next().unwrap().parse().unwrap()
        };
        let (f, r2) = (number("pseudo-F = "), number("R\u{b2} = "));
        assert!((r2 - permanova_r2(f, 6, 2)).abs() < 1e-3, "{line}");
        assert!(r2 > 0.5 && r2 < 1.0, "{line}");
        // Sums of squares 30 between 3 groups and 60 within, 12 samples:
        // F = (30 / 2) / (60 / 9) and R squared = 30 / 90.
        assert!((permanova_r2(2.25, 12, 3) - 1.0 / 3.0).abs() < 1e-12);
        assert!(texts
            .iter()
            .all(|t| !t.starts_with("ANOSIM") || !t.contains("R\u{b2}")));
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
        let matrix = community.table.distances(Distance::BrayCurtis);
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
