//! Stacked bar plot of composition: what each sample, or each group, is
//! made of.

use stamp_core::stats::cluster::{cluster, Linkage};
use stamp_core::Profile;

use crate::axes::{
    fonts, group_color, legend_style, scaled_ticks, tick_style, y_axis, y_axis_width, Frame,
    AXES_COLOR, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, HAlign, Stroke, VAlign};
use crate::text;

/// What a plot shows: `values[feature][sample]`, in percent.
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub features: Vec<String>,
    pub samples: Vec<String>,
    pub values: Vec<Vec<f64>>,
    /// Group of each sample, as an index into `groups`.
    pub sample_groups: Vec<usize>,
    pub groups: Vec<String>,
    /// Colour of each group; groups beyond the list use the default palette.
    pub group_colors: Vec<Color>,
    /// Each sample's value of the metadata field the samples are sorted by
    /// ([`SampleOrder::MetadataField`]); empty if there is none.
    pub sort_keys: Vec<String>,
}

impl Data {
    /// The relative frequencies of every feature of a profile in each whole
    /// sample, where sample `i` belongs to group `sample_groups[i]`. They
    /// are percentages of the sample even when the profile has a parent
    /// level, so that a sample's features add up to 100%.
    pub fn from_profile(profile: &Profile, sample_groups: Vec<usize>, groups: Vec<String>) -> Data {
        let totals = profile.sample_totals();
        Data {
            features: profile.features().iter().map(|f| f.name.clone()).collect(),
            samples: profile.sample_names.clone(),
            values: profile
                .features()
                .iter()
                .map(|f| {
                    f.counts
                        .iter()
                        .zip(&totals)
                        .map(|(&count, &total)| stamp_core::profile::percent(count, total))
                        .collect()
                })
                .collect(),
            sample_groups,
            groups,
            group_colors: Vec::new(),
            sort_keys: Vec::new(),
        }
    }

    pub fn group_color(&self, index: usize) -> Color {
        self.group_colors
            .get(index)
            .copied()
            .unwrap_or_else(|| group_color(index))
    }

    /// One bar per group: the mean of each feature over the group's samples.
    pub(crate) fn by_group(&self) -> Data {
        let members: Vec<Vec<usize>> = (0..self.groups.len())
            .map(|group| {
                (0..self.samples.len())
                    .filter(|&s| self.sample_groups.get(s) == Some(&group))
                    .collect()
            })
            .collect();
        let kept: Vec<usize> = (0..self.groups.len())
            .filter(|&g| !members[g].is_empty())
            .collect();
        Data {
            features: self.features.clone(),
            samples: kept.iter().map(|&g| self.groups[g].clone()).collect(),
            values: self
                .values
                .iter()
                .map(|row| {
                    kept.iter()
                        .map(|&g| {
                            members[g].iter().map(|&s| row[s]).sum::<f64>()
                                / members[g].len() as f64
                        })
                        .collect()
                })
                .collect(),
            sample_groups: kept.clone(),
            groups: self.groups.clone(),
            group_colors: self.group_colors.clone(),
            sort_keys: Vec::new(),
        }
    }
}

/// What each bar stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Bars {
    Samples,
    /// The mean composition of each group's samples.
    Groups,
}

/// The order of the samples within each group, when there is a bar for
/// each sample. Groups stay in their own order, one after another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SampleOrder {
    /// Alphabetically by name.
    Name,
    /// Samples with similar profiles side by side: the leaf order of an
    /// average-linkage (UPGMA) clustering of their proportions.
    Composition,
    /// By the proportion of the most abundant feature (the lowest in the
    /// stacks), highest first.
    DominantFeature,
    /// By the value of a metadata field ([`Options::sort_field`]): as
    /// numbers, smallest first, when every value is one, and otherwise
    /// alphabetically; samples without a value last.
    MetadataField,
    /// As in the profile file.
    File,
}

impl SampleOrder {
    pub const ALL: [SampleOrder; 5] = [
        SampleOrder::Composition,
        SampleOrder::Name,
        SampleOrder::DominantFeature,
        SampleOrder::MetadataField,
        SampleOrder::File,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SampleOrder::Name => "Sample name",
            SampleOrder::Composition => "Composition",
            SampleOrder::DominantFeature => "Dominant feature",
            SampleOrder::MetadataField => "Metadata field",
            SampleOrder::File => "Order in the profile file",
        }
    }
}

/// What the names beneath the bars name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Labels {
    /// One name for each group, beneath its bars, tilted at 30 degrees.
    /// With a bar for each group, the name of each bar.
    Groups,
    /// The name of each bar.
    Samples,
    None,
}

impl Labels {
    pub const ALL: [Labels; 3] = [Labels::Groups, Labels::Samples, Labels::None];

    pub fn name(self) -> &'static str {
        match self {
            Labels::Groups => "Each group",
            Labels::Samples => "Each sample",
            Labels::None => "None",
        }
    }
}

/// Appearance of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    pub bars: Bars,
    /// Number of features drawn in their own colour, most abundant first.
    /// The rest are added up as "Other".
    pub features_to_show: usize,
    /// The order of the samples within each group.
    pub sample_order: SampleOrder,
    /// The metadata field of [`SampleOrder::MetadataField`].
    pub sort_field: Option<String>,
    /// What the names beneath the bars name.
    pub labels: Labels,
    /// Space between the bars of one group and those of the next, in
    /// points, when there is a bar for each sample.
    pub group_space: f64,
    /// Name the features beside the plot.
    pub show_legend: bool,
    /// Shorten feature names to this many characters.
    pub truncate_labels: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 5.0,
            bars: Bars::Samples,
            features_to_show: 10,
            sample_order: SampleOrder::Composition,
            sort_field: None,
            labels: Labels::Groups,
            group_space: 6.0,
            show_legend: true,
            truncate_labels: Some(40),
        }
    }
}

/// Colours of the features, most abundant first: a palette of pairs of a
/// strong and a pale shade (matplotlib's "tab20" without its greys, which
/// are kept for "Other"). They are used in turn if more features are asked
/// for.
const FEATURE_COLORS: [Color; 18] = [
    Color::rgb(31, 119, 180),
    Color::rgb(255, 127, 14),
    Color::rgb(44, 160, 44),
    Color::rgb(214, 39, 40),
    Color::rgb(148, 103, 189),
    Color::rgb(140, 86, 75),
    Color::rgb(227, 119, 194),
    Color::rgb(188, 189, 34),
    Color::rgb(23, 190, 207),
    Color::rgb(174, 199, 232),
    Color::rgb(255, 187, 120),
    Color::rgb(152, 223, 138),
    Color::rgb(255, 152, 150),
    Color::rgb(197, 176, 213),
    Color::rgb(196, 156, 148),
    Color::rgb(247, 182, 210),
    Color::rgb(219, 219, 141),
    Color::rgb(158, 218, 229),
];
const OTHER_COLOR: Color = Color::rgb(199, 199, 199);
/// Name of the features that are not drawn on their own.
pub const OTHER: &str = "Other";

/// The bars of each sample, group by group (in the order of the groups)
/// and within a group in `order`.
fn sample_order(data: &Data, order: SampleOrder) -> Vec<usize> {
    // The feature most abundant on average, which lies lowest in the stacks.
    let bars = data.samples.len().max(1) as f64;
    let dominant = (0..data.values.len()).max_by(|&a, &b| {
        let mean = |f: usize| data.values[f].iter().sum::<f64>() / bars;
        mean(a)
            .total_cmp(&mean(b))
            .then_with(|| data.features[b].cmp(&data.features[a]))
    });
    let mut groups: Vec<usize> = data.sample_groups.clone();
    groups.sort_unstable();
    groups.dedup();
    let mut bars = Vec::with_capacity(data.samples.len());
    for group in groups {
        let mut members: Vec<usize> = (0..data.samples.len())
            .filter(|&s| data.sample_groups.get(s) == Some(&group))
            .collect();
        match order {
            SampleOrder::File => {}
            SampleOrder::Name => members.sort_by(|&a, &b| {
                data.samples[a]
                    .to_lowercase()
                    .cmp(&data.samples[b].to_lowercase())
                    .then_with(|| data.samples[a].cmp(&data.samples[b]))
            }),
            SampleOrder::DominantFeature => {
                if let Some(f) = dominant {
                    members.sort_by(|&a, &b| {
                        data.values[f][b]
                            .total_cmp(&data.values[f][a])
                            .then_with(|| data.samples[a].cmp(&data.samples[b]))
                    });
                }
            }
            SampleOrder::MetadataField => {
                if data.sort_keys.len() == data.samples.len() {
                    let keys = &data.sort_keys;
                    let number = |s: usize| keys[s].trim().parse::<f64>().ok();
                    let numeric = members
                        .iter()
                        .filter(|&&s| !keys[s].trim().is_empty())
                        .all(|&s| number(s).is_some());
                    members.sort_by(|&a, &b| {
                        let (ka, kb) = (keys[a].trim(), keys[b].trim());
                        // Samples without a value come last.
                        ka.is_empty()
                            .cmp(&kb.is_empty())
                            .then_with(|| {
                                if numeric {
                                    number(a)
                                        .unwrap_or(f64::INFINITY)
                                        .total_cmp(&number(b).unwrap_or(f64::INFINITY))
                                } else {
                                    ka.to_lowercase().cmp(&kb.to_lowercase())
                                }
                            })
                            .then_with(|| data.samples[a].cmp(&data.samples[b]))
                    });
                }
            }
            SampleOrder::Composition => {
                let profiles: Vec<Vec<f64>> = members
                    .iter()
                    .map(|&s| {
                        data.values
                            .iter()
                            .map(|row| if row[s].is_finite() { row[s] } else { 0.0 })
                            .collect()
                    })
                    .collect();
                if let Some(tree) = cluster(&profiles, Linkage::Average) {
                    members = tree.order.iter().map(|&i| members[i]).collect();
                }
            }
        }
        bars.extend(members);
    }
    bars
}

/// The data with the samples in the order of `bars`.
fn reordered(data: &Data, bars: &[usize]) -> Data {
    Data {
        features: data.features.clone(),
        samples: bars.iter().map(|&s| data.samples[s].clone()).collect(),
        values: data
            .values
            .iter()
            .map(|row| bars.iter().map(|&s| row[s]).collect())
            .collect(),
        sample_groups: bars
            .iter()
            .map(|&s| data.sample_groups.get(s).copied().unwrap_or(0))
            .collect(),
        groups: data.groups.clone(),
        group_colors: data.group_colors.clone(),
        sort_keys: Vec::new(),
    }
}

/// The angle of the group names beneath bars of each sample, in degrees.
const GROUP_LABEL_ANGLE: f64 = 30.0;

pub fn plot(data: &Data, options: &Options) -> Figure {
    let arranged;
    let data = match options.bars {
        Bars::Samples => {
            if data
                .values
                .iter()
                .any(|row| row.len() != data.samples.len())
            {
                return Figure::message("No active features or degenerate plot");
            }
            arranged = reordered(data, &sample_order(data, options.sample_order));
            &arranged
        }
        Bars::Groups => {
            arranged = data.by_group();
            &arranged
        }
    };
    let bars = data.samples.len();
    if data.features.is_empty()
        || bars == 0
        || data.values.len() != data.features.len()
        || data.values.iter().any(|row| row.len() != bars)
    {
        return Figure::message("No active features or degenerate plot");
    }

    // The most abundant features on average, largest first; ties by name
    // so that the order does not depend on the order of the profile.
    let mean = |feature: usize| data.values[feature].iter().sum::<f64>() / bars as f64;
    let mut order: Vec<usize> = (0..data.features.len()).collect();
    order.sort_by(|&a, &b| {
        mean(b)
            .total_cmp(&mean(a))
            .then_with(|| data.features[a].cmp(&data.features[b]))
    });
    let shown = options.features_to_show.max(1).min(order.len());
    let (top, rest) = order.split_at(shown);
    let other: Vec<f64> = (0..bars)
        .map(|bar| rest.iter().map(|&f| data.values[f][bar]).sum())
        .collect();
    let has_other = !rest.is_empty();

    let shorten = |name: &str| match options.truncate_labels {
        Some(length) => text::truncate(name, length),
        None => name.to_string(),
    };
    // Legend entries from the bottom of the stack to the top.
    let mut entries: Vec<(String, Color)> = top
        .iter()
        .enumerate()
        .map(|(rank, &f)| {
            (
                shorten(&data.features[f]),
                FEATURE_COLORS[rank % FEATURE_COLORS.len()],
            )
        })
        .collect();
    if has_other {
        entries.push((OTHER.to_string(), OTHER_COLOR));
    }

    // Bars reach the tallest stack: 100 unless the features do not account
    // for every sequence (as when unclassified ones are set aside).
    let tallest = (0..bars)
        .map(|bar| top.iter().map(|&f| data.values[f][bar]).sum::<f64>() + other[bar])
        .fold(0.0, f64::max);
    let y_ticks = Ticks::covering(
        0.0,
        if tallest > 0.0 {
            tallest.min(100.0)
        } else {
            100.0
        },
    );

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let strip = 6.0; // group colour beneath each bar
                     // The names beneath the bars are sized as tick labels, the list of
                     // features beside the plot as a legend.
    let sizes = fonts();
    let line = sizes.tick + 3.0;
    let legend_line = sizes.legend + 3.0;
    let legend_width = if options.show_legend {
        12.0 + 14.0
            + 5.0
            + entries
                .iter()
                .map(|(name, _)| text::width(name, sizes.legend))
                .fold(0.0, f64::max)
    } else {
        0.0
    };
    // With a bar for each sample, the groups are runs of bars, named once
    // each, tilted, beneath the middle of their run.
    let runs: Vec<(usize, usize, usize)> = {
        let mut runs: Vec<(usize, usize, usize)> = Vec::new();
        for (bar, &group) in data.sample_groups.iter().enumerate() {
            match runs.last_mut() {
                Some((g, _, end)) if *g == group => *end = bar + 1,
                _ => runs.push((group, bar, bar + 1)),
            }
        }
        runs
    };
    let group_labels = options.bars == Bars::Samples && options.labels == Labels::Groups;
    // The run (group) of each bar, and the space left between runs.
    let run_of: Vec<usize> = (0..bars)
        .map(|bar| {
            runs.iter()
                .position(|&(_, s, e)| (s..e).contains(&bar))
                .unwrap_or(0)
        })
        .collect();
    let space = if options.bars == Bars::Samples {
        options.group_space.clamp(0.0, 72.0)
    } else {
        0.0
    };
    let spaces = space * runs.len().saturating_sub(1) as f64;
    // Where a bar starts, or a run's middle lies, from the left of the bars.
    let at = |cell: f64, position: f64, run: usize| cell * position + space * run as f64;
    let (sin, cos) = GROUP_LABEL_ANGLE.to_radians().sin_cos();
    let group_name = |group: usize| data.groups.get(group).cloned().unwrap_or_default();
    let mut left = padding + y_axis_width(&y_ticks, true);
    let right = figure.width - padding - legend_width;
    // A long name of the first group would reach past the left edge.
    if group_labels {
        if let Some(&(group, start, end)) = runs.first() {
            let cell = (right - left - spaces) / bars as f64;
            let centre = left + at(cell, (start + end) as f64 / 2.0, 0);
            let reach = text::width(&group_name(group), sizes.tick) * cos;
            left += (padding - (centre - reach)).max(0.0);
        }
    }
    let cell = (right - left - spaces) / bars as f64;
    // Names are written upright beneath the bars where the bars are wide
    // enough to tell them apart, and across when there is room for that.
    let widest_name = data
        .samples
        .iter()
        .map(|name| text::width(name, sizes.tick))
        .fold(0.0, f64::max);
    let across = widest_name + 4.0 <= cell;
    let labelled =
        !group_labels && options.labels != Labels::None && (across || cell >= sizes.tick * 0.9);
    let label_height = if group_labels {
        runs.iter()
            .map(|&(group, _, _)| text::width(&group_name(group), sizes.tick) * sin + line * cos)
            .fold(0.0, f64::max)
            + 2.0
    } else {
        match (labelled, across) {
            (false, _) => 0.0,
            (true, true) => line,
            (true, false) => widest_name + 4.0,
        }
    };
    let frame = Frame {
        left,
        top: padding,
        right,
        bottom: figure.height - padding - strip - 3.0 - label_height,
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message(
            "The figure is too small for this plot; widen it or shorten the names",
        );
    }
    let y = frame.y_scale(y_ticks.min, y_ticks.max);

    let gap = (cell * 0.1).min(3.0);
    for (bar, &other_value) in other.iter().enumerate() {
        let x0 = frame.left + at(cell, bar as f64, run_of[bar]) + gap / 2.0;
        let width = cell - gap;
        let mut base = 0.0;
        let segments = top
            .iter()
            .enumerate()
            .map(|(rank, &f)| {
                (
                    data.features[f].as_str(),
                    data.values[f][bar],
                    FEATURE_COLORS[rank % FEATURE_COLORS.len()],
                )
            })
            .chain(has_other.then_some((OTHER, other_value, OTHER_COLOR)));
        for (name, value, color) in segments {
            // Nothing to draw for a feature that is absent (or not a number).
            if value.is_nan() || value <= 0.0 {
                continue;
            }
            let (bottom, top_edge) = (y.map(base), y.map(base + value));
            figure.rect(x0, top_edge, width, bottom - top_edge, Some(color), None);
            figure.region(
                x0,
                top_edge,
                width,
                bottom - top_edge,
                name,
                format!("{name}\n{}\n{value:.2}%", data.samples[bar]),
            );
            base += value;
        }
        // The group the bar belongs to.
        let group = data.sample_groups.get(bar).copied().unwrap_or(0);
        figure.rect(
            x0,
            frame.bottom + 3.0,
            width,
            strip,
            Some(data.group_color(group)),
            None,
        );
        if labelled {
            let centre = x0 + width / 2.0;
            let below = frame.bottom + 3.0 + strip + 3.0;
            if across {
                figure.text(
                    centre,
                    below,
                    data.samples[bar].clone(),
                    tick_style().align(HAlign::Center, VAlign::Top),
                );
            } else {
                figure.text(
                    centre,
                    below,
                    data.samples[bar].clone(),
                    tick_style()
                        .align(HAlign::Right, VAlign::Middle)
                        .rotated(90.0),
                );
            }
        }
    }

    if group_labels {
        let below = frame.bottom + 3.0 + strip + 3.0;
        for (run, &(group, start, end)) in runs.iter().enumerate() {
            let centre = frame.left + at(cell, (start + end) as f64 / 2.0, run);
            figure.text(
                centre,
                below,
                group_name(group),
                tick_style()
                    .align(HAlign::Right, VAlign::Top)
                    .rotated(GROUP_LABEL_ANGLE),
            );
        }
    }

    let stroke = Stroke::solid(AXES_COLOR, 1.0);
    figure.line(
        &[(frame.left, frame.bottom), (frame.right, frame.bottom)],
        stroke,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some("Proportion of sequences (%)"),
        AXES_COLOR,
    );

    if options.show_legend {
        // Listed from the top of the stack down, as the eye meets them.
        let x = frame.right + 12.0;
        for (i, (name, color)) in entries.iter().rev().enumerate() {
            let centre = frame.top + legend_line * (i as f64 + 0.5);
            if centre + legend_line / 2.0 > figure.height - padding {
                break;
            }
            figure.rect(
                x,
                centre - 3.0,
                14.0,
                6.0,
                Some(*color),
                Some(Stroke::solid(Color::BLACK, 0.5)),
            );
            figure.text(
                x + 19.0,
                centre,
                name.clone(),
                legend_style().align(HAlign::Left, VAlign::Middle),
            );
        }
    }
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data() -> Data {
        Data {
            features: vec![
                "rare".into(),
                "common".into(),
                "middling".into(),
                "trace".into(),
            ],
            samples: vec!["a1".into(), "a2".into(), "b1".into()],
            values: vec![
                vec![5.0, 3.0, 1.0],
                vec![60.0, 70.0, 20.0],
                vec![30.0, 25.0, 70.0],
                vec![5.0, 2.0, 9.0],
            ],
            sample_groups: vec![0, 0, 1],
            groups: vec!["A".into(), "B".into()],
            group_colors: Vec::new(),
            sort_keys: Vec::new(),
        }
    }

    #[test]
    fn a_parent_level_does_not_change_the_whole_sample_percentages() {
        // Percentages of the parent would add up to 100% per parent, so a
        // stack of every feature would pass 100%.
        let tree = stamp_core::ProfileTree::read(
            "Phylum\tGenus\ts1\ts2\n\
             P1\tg1\t30\t10\n\
             P1\tg2\t10\t30\n\
             P2\tg3\t60\t20\n",
        )
        .unwrap();
        let samples = ["s1", "s2"];
        let whole = tree
            .profile(&samples, None, "Genus", stamp_core::Unclassified::Retain)
            .unwrap();
        let within = tree
            .profile(
                &samples,
                Some("Phylum"),
                "Genus",
                stamp_core::Unclassified::Retain,
            )
            .unwrap();
        let groups = vec!["G".to_string()];
        let a = Data::from_profile(&whole, vec![0, 0], groups.clone());
        let b = Data::from_profile(&within, vec![0, 0], groups);
        assert_eq!(a.values, b.values);
        assert_eq!(b.values[0], [30.0, 10.0 * 100.0 / 60.0]);
        for sample in 0..2 {
            let sum: f64 = b.values.iter().map(|v| v[sample]).sum();
            assert!((sum - 100.0).abs() < 1e-9);
        }
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

    /// Height on the page of the segment of a feature in a bar.
    fn segment(figure: &Figure, feature: &str, bar: &str) -> Option<f64> {
        figure
            .regions
            .iter()
            .find(|r| r.item == feature && r.tooltip.contains(&format!("\n{bar}\n")))
            .map(|r| r.height)
    }

    #[test]
    fn the_most_abundant_features_are_stacked_and_the_rest_pooled() {
        let options = Options {
            features_to_show: 2,
            labels: Labels::Samples,
            ..Options::default()
        };
        let figure = plot(&data(), &options);
        // Two features and "Other" in each of three bars.
        assert_eq!(figure.regions.len(), 9);
        let texts = texts(&figure);
        for expected in [
            "common",
            "middling",
            OTHER,
            "a1",
            "b1",
            "Proportion of sequences (%)",
        ] {
            assert!(texts.iter().any(|t| t == expected), "missing {expected}");
        }
        assert!(!texts.contains(&"rare".to_string()));
        // "Other" is the two features left out: 10% of a1, 5% of a2.
        let other = figure
            .regions
            .iter()
            .find(|r| r.item == OTHER && r.tooltip.contains("\na1\n"))
            .unwrap();
        assert!(other.tooltip.ends_with("10.00%"), "{}", other.tooltip);
        // Heights are in proportion to the values, and each bar is full.
        let unit = segment(&figure, "common", "a1").unwrap() / 60.0;
        assert!((segment(&figure, "middling", "b1").unwrap() / 70.0 - unit).abs() < 1e-9);
        let full: f64 = ["common", "middling", OTHER]
            .iter()
            .filter_map(|f| segment(&figure, f, "a2"))
            .sum();
        assert!((full / unit - 100.0).abs() < 1e-6);
        // The most abundant feature has the first colour and sits lowest.
        let lowest = figure
            .regions
            .iter()
            .filter(|r| r.tooltip.contains("\na1\n"))
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap();
        assert_eq!(lowest.item, "common");
        assert!(figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == FEATURE_COLORS[0])));
    }

    #[test]
    fn groups_are_drawn_as_their_mean_composition() {
        let options = Options {
            bars: Bars::Groups,
            features_to_show: 10,
            ..Options::default()
        };
        let figure = plot(&data(), &options);
        // Every feature in both groups, and nothing left for "Other".
        assert_eq!(figure.regions.len(), 8);
        assert!(!texts(&figure).contains(&OTHER.to_string()));
        let common_a = figure
            .regions
            .iter()
            .find(|r| r.item == "common" && r.tooltip.contains("\nA\n"))
            .unwrap();
        assert!(common_a.tooltip.ends_with("65.00%"), "{}", common_a.tooltip);
        // A strip in the colour of each group lies beneath its bar.
        for group in 0..2 {
            assert!(figure.shapes.iter().any(
                |s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == group_color(group))
            ));
        }
    }

    #[test]
    fn options_and_degenerate_data() {
        let bare = plot(
            &data(),
            &Options {
                labels: Labels::None,
                show_legend: false,
                ..Options::default()
            },
        );
        let texts = texts(&bare);
        assert!(!texts.contains(&"a1".to_string()) && !texts.contains(&"common".to_string()));

        let mut empty = data();
        empty.features.clear();
        empty.values.clear();
        assert_eq!(plot(&empty, &Options::default()).shapes.len(), 1);

        // Proportions that do not add up to 100 leave the bars short.
        let mut partial = data();
        for row in &mut partial.values {
            for value in row.iter_mut() {
                *value /= 2.0;
            }
        }
        let figure = plot(&partial, &Options::default());
        assert!(self::texts(&figure).contains(&"50".to_string()));
        assert!(!self::texts(&figure).contains(&"100".to_string()));
    }

    /// The bars of each sample, in order, by the tooltips of the regions.
    fn bar_order(figure: &Figure) -> Vec<String> {
        let mut bars: Vec<(f64, String)> = figure
            .regions
            .iter()
            .map(|r| (r.x, r.tooltip.lines().nth(1).unwrap_or("").to_string()))
            .collect();
        bars.sort_by(|a, b| a.0.total_cmp(&b.0));
        bars.dedup_by(|a, b| a.1 == b.1);
        bars.into_iter().map(|(_, name)| name).collect()
    }

    fn unsorted() -> Data {
        // Group A: z is mostly "common", m mostly "middling", a in between
        // but closest to z; group B after it.
        Data {
            features: vec!["common".into(), "middling".into()],
            samples: vec!["z".into(), "b1".into(), "m".into(), "a".into()],
            values: vec![vec![90.0, 50.0, 10.0, 80.0], vec![10.0, 50.0, 90.0, 20.0]],
            sample_groups: vec![0, 1, 0, 0],
            groups: vec!["A".into(), "B".into()],
            group_colors: Vec::new(),
            sort_keys: Vec::new(),
        }
    }

    #[test]
    fn samples_are_ordered_within_their_groups() {
        let order = |sample_order| {
            bar_order(&plot(
                &unsorted(),
                &Options {
                    sample_order,
                    ..Options::default()
                },
            ))
        };
        assert_eq!(order(SampleOrder::Name), ["a", "m", "z", "b1"]);
        assert_eq!(order(SampleOrder::File), ["z", "m", "a", "b1"]);
        // "common" is the most abundant feature on average.
        assert_eq!(order(SampleOrder::DominantFeature), ["z", "a", "m", "b1"]);
        // z and a are alike, m apart from them.
        let clustered = order(SampleOrder::Composition);
        let at = |name: &str| clustered.iter().position(|s| s == name).unwrap();
        assert_eq!(at("z").abs_diff(at("a")), 1, "{clustered:?}");
        assert_eq!(at("b1"), 3);
        // By a metadata field: as numbers (2 before 10), a sample without
        // a value last; as text when a value is not a number.
        let by_field = |keys: [&str; 4]| {
            let mut data = unsorted();
            data.sort_keys = keys.iter().map(|k| k.to_string()).collect();
            bar_order(&plot(
                &data,
                &Options {
                    sample_order: SampleOrder::MetadataField,
                    ..Options::default()
                },
            ))
        };
        // Samples in file order: z, b1, m, a.
        assert_eq!(by_field(["10", "1", "2", ""]), ["m", "z", "a", "b1"]);
        assert_eq!(by_field(["b", "1", "C", "a"]), ["a", "z", "m", "b1"]);
        // Without values for the samples, the file's order.
        assert_eq!(by_field_none(), ["z", "m", "a", "b1"]);
    }

    fn by_field_none() -> Vec<String> {
        bar_order(&plot(
            &unsorted(),
            &Options {
                sample_order: SampleOrder::MetadataField,
                ..Options::default()
            },
        ))
    }

    #[test]
    fn groups_are_named_once_beneath_their_bars_at_an_angle() {
        let figure = plot(&unsorted(), &Options::default());
        let labels: Vec<(String, f64)> = figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, style, .. } => Some((text.clone(), style.rotation)),
                _ => None,
            })
            .filter(|(text, _)| ["A", "B", "a", "m", "z", "b1"].contains(&text.as_str()))
            .collect();
        assert_eq!(
            labels,
            [
                ("A".to_string(), GROUP_LABEL_ANGLE),
                ("B".to_string(), GROUP_LABEL_ANGLE)
            ]
        );
        // Each sample named instead, when asked.
        let samples = plot(
            &unsorted(),
            &Options {
                labels: Labels::Samples,
                ..Options::default()
            },
        );
        let texts = texts(&samples);
        assert!(["a", "m", "z", "b1"]
            .iter()
            .all(|n| texts.contains(&n.to_string())));
        assert!(!texts.contains(&"A".to_string()));
    }

    #[test]
    fn groups_are_set_apart_by_a_space() {
        let left_edges = |group_space| {
            let figure = plot(
                &unsorted(),
                &Options {
                    group_space,
                    sample_order: SampleOrder::File,
                    ..Options::default()
                },
            );
            let mut xs: Vec<f64> = figure.regions.iter().map(|r| r.x).collect();
            xs.sort_by(f64::total_cmp);
            xs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            xs
        };
        // Three bars of A, then B: the step from A's last bar to B's first
        // is wider than between A's bars by the space.
        let plain = left_edges(0.0);
        let spaced = left_edges(20.0);
        let steps = |xs: &[f64]| xs.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>();
        let (p, s) = (steps(&plain), steps(&spaced));
        assert!((p[0] - p[2]).abs() < 1e-9);
        assert!((s[2] - s[0] - 20.0).abs() < 1e-9, "{s:?}");
    }
}
