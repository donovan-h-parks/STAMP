//! Stacked bar plot of composition: what each sample, or each group, is
//! made of.

use stamp_core::Profile;

use crate::axes::{
    group_color, label_style, scaled_ticks, y_axis, y_axis_width, Frame, AXES_COLOR, FONT_SIZE,
    INCH,
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
}

impl Data {
    /// The relative frequencies of every feature of a profile, where sample
    /// `i` belongs to group `sample_groups[i]`.
    pub fn from_profile(profile: &Profile, sample_groups: Vec<usize>, groups: Vec<String>) -> Data {
        Data {
            features: profile.features().iter().map(|f| f.name.clone()).collect(),
            samples: profile.sample_names.clone(),
            values: profile
                .features()
                .iter()
                .map(|f| profile.percentages(f))
                .collect(),
            sample_groups,
            groups,
            group_colors: Vec::new(),
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
    /// Write the name of each sample or group beneath its bar.
    pub label_bars: bool,
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
            label_bars: true,
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

pub fn plot(data: &Data, options: &Options) -> Figure {
    let grouped;
    let data = match options.bars {
        Bars::Samples => data,
        Bars::Groups => {
            grouped = data.by_group();
            &grouped
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
    let line = FONT_SIZE + 3.0;
    let legend_width = if options.show_legend {
        12.0 + 14.0
            + 5.0
            + entries
                .iter()
                .map(|(name, _)| text::width(name, FONT_SIZE))
                .fold(0.0, f64::max)
    } else {
        0.0
    };
    let left = padding + y_axis_width(&y_ticks, true);
    let right = figure.width - padding - legend_width;
    let cell = (right - left) / bars as f64;
    // Names are written upright beneath the bars where the bars are wide
    // enough to tell them apart, and across when there is room for that.
    let widest_name = data
        .samples
        .iter()
        .map(|name| text::width(name, FONT_SIZE))
        .fold(0.0, f64::max);
    let across = widest_name + 4.0 <= cell;
    let labelled = options.label_bars && (across || cell >= FONT_SIZE * 0.9);
    let label_height = match (labelled, across) {
        (false, _) => 0.0,
        (true, true) => line,
        (true, false) => widest_name + 4.0,
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
        let x0 = frame.left + cell * bar as f64 + gap / 2.0;
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
                    label_style().align(HAlign::Center, VAlign::Top),
                );
            } else {
                figure.text(
                    centre,
                    below,
                    data.samples[bar].clone(),
                    label_style()
                        .align(HAlign::Right, VAlign::Middle)
                        .rotated(90.0),
                );
            }
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
            let centre = frame.top + line * (i as f64 + 0.5);
            if centre + line / 2.0 > figure.height - padding {
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
                label_style().align(HAlign::Left, VAlign::Middle),
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
                label_bars: false,
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
}
