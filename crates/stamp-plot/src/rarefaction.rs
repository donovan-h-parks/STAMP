//! Rarefaction curves: the number of features each sample is expected to
//! show against the number of its sequences read.

use stamp_core::stats::rarefaction::{chao1, expected_richness, whole_counts};
use stamp_core::{Profile, ProfileTree, Unclassified};

use crate::axes::{
    group_color, label_style, legend, scaled_ticks, x_axis, y_axis, y_axis_width, Frame,
    LabelRotation, LegendPosition, AXES_COLOR, FONT_SIZE, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, HAlign, Stroke, VAlign};
use crate::text;

/// One sample: the sequences assigned to each feature that counts towards
/// its richness, and all of its sequences.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub name: String,
    /// Index into [`Data::groups`].
    pub group: usize,
    pub counts: Vec<u64>,
    /// Every sequence of the sample, including those in categories that are
    /// not counted as features.
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub samples: Vec<Sample>,
    /// What the curves are coloured by: groups of samples or, for a pair of
    /// samples, the samples themselves.
    pub groups: Vec<String>,
    /// Colour of each group; groups beyond the list use the default palette.
    pub group_colors: Vec<Color>,
}

/// Why a profile cannot be rarefied.
pub const NOT_COUNTS: &str = "Rarefaction needs counts of sequences, and this profile holds \
                              fractional values. Use a profile of raw counts.";

impl Data {
    /// Rarefaction data for the samples of `profile`, where sample `i`
    /// belongs to group `sample_groups[i]`.
    ///
    /// The profile should hold every category at its level, unclassified
    /// ones included: those are not counted as features, being no taxon or
    /// function of their own, but their sequences are part of the sample
    /// and are drawn like any others.
    ///
    /// Fails if the profile does not hold whole numbers of sequences.
    pub fn from_profile(
        profile: &Profile,
        sample_groups: Vec<usize>,
        groups: Vec<String>,
    ) -> Result<Data, &'static str> {
        let mut samples: Vec<Sample> = profile
            .sample_names
            .iter()
            .enumerate()
            .map(|(i, name)| Sample {
                name: name.clone(),
                group: sample_groups.get(i).copied().unwrap_or(0),
                counts: Vec::new(),
                total: 0,
            })
            .collect();
        for feature in profile.features() {
            let counts = whole_counts(&feature.counts).ok_or(NOT_COUNTS)?;
            let counted = !feature.name.to_ascii_lowercase().contains("unclassified");
            for (sample, count) in samples.iter_mut().zip(counts) {
                sample.total += count;
                if counted {
                    sample.counts.push(count);
                }
            }
        }
        Ok(Data {
            samples,
            groups,
            group_colors: Vec::new(),
        })
    }

    /// Rarefaction data for some samples of a profile tree at a
    /// hierarchical level, whatever treatment of unclassified sequences the
    /// analysis itself uses.
    pub fn from_tree(
        tree: &ProfileTree,
        samples: &[String],
        profile_level: &str,
        sample_groups: Vec<usize>,
        groups: Vec<String>,
    ) -> Result<Data, String> {
        let names: Vec<&str> = samples.iter().map(String::as_str).collect();
        let profile = tree
            .profile(&names, None, profile_level, Unclassified::Retain)
            .map_err(|e| e.to_string())?;
        Data::from_profile(&profile, sample_groups, groups).map_err(str::to_string)
    }

    pub fn group_color(&self, index: usize) -> Color {
        self.group_colors
            .get(index)
            .copied()
            .unwrap_or_else(|| group_color(index))
    }
}

/// Appearance of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Number of depths at which each curve is computed.
    pub points: usize,
    /// End every curve at the depth of the smallest sample, where they can
    /// be compared, instead of at each sample's own depth.
    pub truncate_to_smallest: bool,
    /// Draw each sample's Chao1 estimate of its total richness as a dashed
    /// line.
    pub show_chao1: bool,
    /// Write each sample's name at the end of its curve.
    pub label_samples: bool,
    pub legend: LegendPosition,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 5.0,
            points: 100,
            truncate_to_smallest: false,
            show_chao1: false,
            label_samples: true,
            legend: LegendPosition::Best,
        }
    }
}

/// The depths at which a curve that ends at `end` is computed: `points` of
/// them, evenly spread from zero, always including the end.
fn depths(end: u64, points: usize) -> Vec<u64> {
    let points = points.max(2) as u64;
    if end < points {
        return (0..=end).collect();
    }
    let mut depths: Vec<u64> = (0..points)
        .map(|i| (i as u128 * end as u128 / (points - 1) as u128) as u64)
        .collect();
    depths.dedup();
    depths
}

/// A count with thousands separated, for tooltips.
fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub fn plot(data: &Data, options: &Options) -> Figure {
    let sampled: Vec<&Sample> = data.samples.iter().filter(|s| s.total > 0).collect();
    if sampled.is_empty() {
        return Figure::message("No sample has any sequences to rarefy");
    }
    let smallest = sampled.iter().map(|s| s.total).min().unwrap_or(0);
    let end_of = |sample: &Sample| {
        if options.truncate_to_smallest {
            smallest
        } else {
            sample.total
        }
    };

    // Each curve, and the estimate of where it would level off.
    struct Curve<'a> {
        sample: &'a Sample,
        depths: Vec<u64>,
        richness: Vec<f64>,
        observed: usize,
        chao1: f64,
    }
    let curves: Vec<Curve<'_>> = sampled
        .iter()
        .map(|&sample| {
            let depths = depths(end_of(sample), options.points);
            Curve {
                richness: expected_richness(&sample.counts, sample.total, &depths),
                depths,
                observed: sample.counts.iter().filter(|&&c| c > 0).count(),
                chao1: chao1(&sample.counts),
                sample,
            }
        })
        .collect();

    let deepest = curves
        .iter()
        .filter_map(|c| c.depths.last().copied())
        .max()
        .unwrap_or(1)
        .max(1);
    let mut highest = curves
        .iter()
        .flat_map(|c| c.richness.iter().copied())
        .fold(1.0, f64::max);
    if options.show_chao1 {
        highest = curves.iter().map(|c| c.chao1).fold(highest, f64::max);
    }
    let x_ticks = Ticks::covering(0.0, deepest as f64);
    let y_ticks = Ticks::covering(0.0, highest);

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let label_width = if options.label_samples {
        6.0 + curves
            .iter()
            .map(|c| text::width(&c.sample.name, FONT_SIZE))
            .fold(0.0, f64::max)
    } else {
        0.0
    };
    let frame = Frame {
        left: padding + y_axis_width(&y_ticks, true),
        top: padding,
        right: figure.width - padding - label_width,
        bottom: figure.height - padding - 0.4 * INCH,
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for the rarefaction curves");
    }
    let (x, y) = (
        frame.x_scale(x_ticks.min, x_ticks.max),
        frame.y_scale(y_ticks.min, y_ticks.max),
    );

    for curve in &curves {
        let color = data.group_color(curve.sample.group);
        let points: Vec<(f64, f64)> = curve
            .depths
            .iter()
            .zip(&curve.richness)
            .map(|(&depth, &richness)| (x.map(depth as f64), y.map(richness)))
            .collect();
        figure.line(&points, Stroke::solid(color, 1.25));
        if options.show_chao1 {
            let level = y.map(curve.chao1);
            let end = points.last().map_or(frame.right, |p| p.0);
            figure.line(
                &[(frame.left, level), (end, level)],
                Stroke::dashed(color, 0.75, 4.0, 3.0),
            );
        }
        // Pointing anywhere along a curve says which sample it is.
        let group = data
            .groups
            .get(curve.sample.group)
            .map(String::as_str)
            .unwrap_or("");
        let about = format!(
            "{}\n{group}\n\nSequences: {}\nFeatures observed: {}\nChao1 estimate: {:.1}",
            curve.sample.name,
            grouped(curve.sample.total),
            curve.observed,
            curve.chao1
        );
        for ((&depth, &richness), &(px, py)) in
            curve.depths.iter().zip(&curve.richness).zip(&points)
        {
            figure.region(
                px - 3.0,
                py - 3.0,
                6.0,
                6.0,
                &curve.sample.name,
                format!(
                    "{about}\n\nAt {} sequences: {richness:.1} features expected",
                    grouped(depth)
                ),
            );
        }
    }

    if options.label_samples {
        // A name at the end of each curve. Where names would overlap, a
        // name is moved up or down by whole lines to the nearest free
        // place; one that would have to move too far from its curve to
        // mean anything is left out, and the sample can still be told by
        // pointing at its curve.
        const FURTHEST: i32 = 4;
        let line = FONT_SIZE + 1.0;
        let mut ends: Vec<(f64, f64, &Curve<'_>)> = curves
            .iter()
            .filter_map(|curve| {
                let depth = *curve.depths.last()?;
                let richness = *curve.richness.last()?;
                Some((x.map(depth as f64) + 4.0, y.map(richness), curve))
            })
            .collect();
        // Longest curves first: their ends are the least crowded.
        ends.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
        // Left, right and middle height of the names placed so far.
        let mut placed: Vec<(f64, f64, f64)> = Vec::new();
        for (left, wanted, curve) in ends {
            let right = left + text::width(&curve.sample.name, FONT_SIZE);
            let free = |middle: f64, placed: &[(f64, f64, f64)]| {
                middle - line / 2.0 >= frame.top - line
                    && middle + line / 2.0 <= frame.bottom
                    && !placed
                        .iter()
                        .any(|&(l, r, m)| left < r && right > l && (middle - m).abs() < line)
            };
            let position = (0..=FURTHEST)
                .flat_map(|step| [f64::from(step), -f64::from(step)])
                .map(|lines| wanted + lines * line)
                .find(|&middle| free(middle, &placed));
            if let Some(middle) = position {
                placed.push((left, right, middle));
                figure.text(
                    left,
                    middle,
                    curve.sample.name.clone(),
                    label_style()
                        .color(data.group_color(curve.sample.group))
                        .align(HAlign::Left, VAlign::Middle),
                );
            }
        }
    }

    x_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&x_ticks, &x),
        Some("Number of sequences"),
        LabelRotation::Horizontal,
        AXES_COLOR,
    );
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some("Expected number of features"),
        AXES_COLOR,
    );
    let entries: Vec<(String, Color)> = data
        .groups
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), data.group_color(i)))
        .collect();
    legend(&mut figure, &frame, &entries, options.legend);
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data() -> Data {
        let spf = "Phylum\tGenus\tA1\tA2\tB1\n\
            P1\ta\t50\t5\t10\n\
            P1\tb\t30\t5\t10\n\
            P1\tunclassified\t40\t0\t10\n\
            P2\tc\t15\t5\t0\n\
            P2\td\t4\t0\t0\n\
            P2\te\t1\t0\t0\n";
        let tree = ProfileTree::read(spf).unwrap();
        let profile = tree
            .profile(&["A1", "A2", "B1"], None, "Genus", Unclassified::Retain)
            .unwrap();
        Data::from_profile(&profile, vec![0, 0, 1], vec!["A".into(), "B".into()]).unwrap()
    }

    fn curves(figure: &Figure) -> Vec<&Vec<(f64, f64)>> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Line { points, .. } if points.len() > 2 => Some(points),
                _ => None,
            })
            .collect()
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
    fn unclassified_sequences_are_drawn_but_not_counted() {
        let data = data();
        let a1 = &data.samples[0];
        // Five classified genera; the unclassified 40 sequences are part of
        // the sample only.
        assert_eq!(a1.counts, [50, 30, 15, 4, 1]);
        assert_eq!(a1.total, 140);
        assert_eq!(data.samples[2].total, 30);
    }

    #[test]
    fn fractional_profiles_are_refused() {
        let tree = ProfileTree::read("F\tS1\tS2\nx\t1.5\t2\ny\t3\t4\n").unwrap();
        let profile = tree
            .profile(&["S1", "S2"], None, "F", Unclassified::Retain)
            .unwrap();
        assert_eq!(
            Data::from_profile(&profile, vec![0, 1], vec!["S1".into(), "S2".into()]),
            Err(NOT_COUNTS)
        );
    }

    #[test]
    fn one_rising_curve_per_sample() {
        let figure = plot(&data(), &Options::default());
        let curves = curves(&figure);
        assert_eq!(curves.len(), 3);
        for curve in &curves {
            // Left to right, and never downwards (y grows down the page).
            assert!(curve
                .windows(2)
                .all(|p| p[0].0 < p[1].0 && p[0].1 >= p[1].1 - 1e-9));
        }
        // Each ends at its own depth: the deepest sample furthest right.
        let end = |i: usize| curves[i].last().unwrap().0;
        assert!(end(0) > end(2) && end(2) > end(1));
        let texts = texts(&figure);
        for expected in [
            "A1",
            "A2",
            "B1",
            "A",
            "B",
            "Number of sequences",
            "Expected number of features",
        ] {
            assert!(texts.iter().any(|t| t == expected), "missing {expected}");
        }
        // The tooltip of a curve's last point reports the whole sample.
        let last = figure.regions.iter().rfind(|r| r.item == "A1").unwrap();
        assert!(last.tooltip.contains("Sequences: 140"), "{}", last.tooltip);
        assert!(last.tooltip.contains("Features observed: 5"));
        assert!(last
            .tooltip
            .contains("At 140 sequences: 5.0 features expected"));
    }

    #[test]
    fn options_change_what_is_drawn() {
        // Truncated, every curve ends at the smallest sample's depth.
        let truncated = plot(
            &data(),
            &Options {
                truncate_to_smallest: true,
                ..Options::default()
            },
        );
        let ends: Vec<f64> = curves(&truncated)
            .iter()
            .map(|c| c.last().unwrap().0)
            .collect();
        assert!(ends.iter().all(|end| (end - ends[0]).abs() < 1e-9));

        // Chao1 adds a dashed line per sample; labels can be left out.
        let dashed = |figure: &Figure| {
            figure
                .shapes
                .iter()
                .filter(|s| matches!(s, Shape::Line { stroke, points } if stroke.dash.is_some() && points.len() == 2))
                .count()
        };
        let plain = plot(&data(), &Options::default());
        let with_chao = plot(
            &data(),
            &Options {
                show_chao1: true,
                label_samples: false,
                legend: LegendPosition::None,
                ..Options::default()
            },
        );
        assert_eq!(dashed(&with_chao) - dashed(&plain), 3);
        assert!(!texts(&with_chao).contains(&"A1".to_string()));
        assert!(!texts(&with_chao).contains(&"A".to_string()));

        // Few sequences: a point at every depth; many: the number asked for.
        assert_eq!(depths(5, 100), [0, 1, 2, 3, 4, 5]);
        let many = depths(1_000_000, 50);
        assert_eq!(many.len(), 50);
        assert_eq!((many[0], many[49]), (0, 1_000_000));
        assert_eq!(grouped(1234567), "1,234,567");
        assert_eq!(grouped(999), "999");
    }

    #[test]
    fn labels_do_not_overlap_and_empty_data_says_so() {
        // Curves ending at almost the same height.
        let sample = |name: &str, extra: u64| Sample {
            name: name.to_string(),
            group: 0,
            counts: vec![20, 20, 20 + extra],
            total: 60 + extra,
        };
        let close_together = Data {
            samples: vec![sample("one", 0), sample("two", 1), sample("three", 2)],
            groups: vec!["G".into()],
            group_colors: Vec::new(),
        };
        let figure = plot(&close_together, &Options::default());
        let mut heights: Vec<f64> = figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, y, .. } if ["one", "two", "three"].contains(&text.as_str()) => {
                    Some(*y)
                }
                _ => None,
            })
            .collect();
        heights.sort_by(f64::total_cmp);
        assert_eq!(heights.len(), 3);
        assert!(heights.windows(2).all(|p| p[1] - p[0] >= FONT_SIZE));

        let empty = Data {
            samples: vec![Sample {
                name: "x".into(),
                group: 0,
                counts: vec![0],
                total: 0,
            }],
            groups: vec!["G".into()],
            group_colors: Vec::new(),
        };
        assert!(texts(&plot(&empty, &Options::default()))[0].contains("No sample"));
    }
}
