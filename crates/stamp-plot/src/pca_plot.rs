//! PCA plot: samples projected onto their first three principal components.

use stamp_core::stats::pca::pca;
use stamp_core::{GroupProfile, MultiGroupProfile, Profile};

use crate::axes::{
    group_color, legend, marker, scaled_ticks, x_axis, y_axis, y_axis_width, Frame, LabelRotation,
    LegendPosition, Marker, AXES_COLOR, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, Stroke};

/// A sample with its group.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub name: String,
    /// Index of the sample's group in [`Data::groups`].
    pub group: usize,
    /// Proportion of each feature in the sample.
    pub proportions: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub samples: Vec<Sample>,
    pub groups: Vec<String>,
}

fn proportions(profile: &Profile, column: usize) -> Vec<f64> {
    profile
        .features()
        .iter()
        .map(|f| {
            let parent = profile.parent_counts(f)[column];
            if parent > 0.0 {
                f.counts[column] / parent
            } else {
                0.0
            }
        })
        .collect()
}

impl Data {
    pub fn from_group_profile(profile: &GroupProfile) -> Data {
        let first = profile.samples_in_group1().len();
        let samples = profile
            .profile
            .sample_names
            .iter()
            .enumerate()
            .map(|(i, name)| Sample {
                name: name.clone(),
                group: usize::from(i >= first),
                proportions: proportions(&profile.profile, i),
            })
            .collect();
        Data {
            samples,
            groups: vec![profile.group_name1.clone(), profile.group_name2.clone()],
        }
    }

    pub fn from_multi_group_profile(profile: &MultiGroupProfile) -> Data {
        let mut samples = Vec::new();
        let mut column = 0;
        for (group, members) in profile.samples_in_groups().into_iter().enumerate() {
            for name in members {
                samples.push(Sample {
                    name: name.clone(),
                    group,
                    proportions: proportions(&profile.profile, column),
                });
                column += 1;
            }
        }
        Data {
            samples,
            groups: profile.group_names.clone(),
        }
    }
}

/// Appearance of the PCA plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Area of the markers, in square points.
    pub marker_size: f64,
    /// Also show the third component, against the first and the second.
    pub show_pc3: bool,
    /// Give each group its own marker shape as well as its own colour.
    pub unique_shapes: bool,
    pub legend: LegendPosition,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 6.0,
            marker_size: 30.0,
            show_pc3: true,
            unique_shapes: true,
            legend: LegendPosition::None,
        }
    }
}

pub fn plot(data: &Data, options: &Options) -> Figure {
    let matrix: Vec<Vec<f64>> = data.samples.iter().map(|s| s.proportions.clone()).collect();
    let Some(result) = pca(&matrix) else {
        return Figure::message(
            "Degenerate PCA plot. Try plotting at a different level in the hierarchy.",
        );
    };
    let components = result.variance_fractions.len();
    if components < 2 {
        return Figure::message("Degenerate PCA plot: the samples vary along a single direction.");
    }
    let show_pc3 = options.show_pc3 && components >= 3;
    let coordinate = |sample: usize, component: usize| result.positions[sample][component];
    let label = |component: usize| {
        format!(
            "PC{} ({:.1}%)",
            component + 1,
            result.variance_fractions[component] * 100.0
        )
    };
    let ticks_for = |component: usize| {
        let values = (0..data.samples.len()).map(|s| coordinate(s, component));
        let (low, high) = values.fold((0.0f64, 0.0f64), |(lo, hi), v| (lo.min(v), hi.max(v)));
        let pad = (high - low) * 0.05;
        Ticks::inside(low - pad, high + pad)
    };
    let ticks: Vec<Ticks> = (0..if show_pc3 { 3 } else { 2 }).map(ticks_for).collect();

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.25 * INCH;
    let left = padding + y_axis_width(&ticks[1], true);
    let bottom_labels = 0.4 * INCH;
    let gap = 0.35 * INCH;
    let inner_width = figure.width - left - padding;
    let inner_height = figure.height - 2.0 * padding - bottom_labels;
    // With three components the main panel takes two thirds of each direction.
    let (main_w, main_h) = if show_pc3 {
        (
            (inner_width - gap) * 0.66,
            (inner_height - gap - bottom_labels) * 0.58,
        )
    } else {
        (inner_width, inner_height)
    };
    if main_w < INCH || main_h < INCH {
        return Figure::message("The figure is too small for a PCA plot");
    }
    let main = Frame {
        left,
        top: padding,
        right: left + main_w,
        bottom: padding + main_h,
    };

    let dashed = Stroke::dashed(AXES_COLOR, 1.0, 5.0, 4.0);
    let panel = |figure: &mut Figure,
                 frame: Frame,
                 cx: usize,
                 cy: usize,
                 x_label: bool,
                 y_label: bool,
                 rotate: bool| {
        let (x, y) = (
            frame.x_scale(ticks[cx].min, ticks[cx].max),
            frame.y_scale(ticks[cy].min, ticks[cy].max),
        );
        if (ticks[cx].min..=ticks[cx].max).contains(&0.0) {
            figure.line(
                &[(x.map(0.0), frame.top), (x.map(0.0), frame.bottom)],
                dashed,
            );
        }
        if (ticks[cy].min..=ticks[cy].max).contains(&0.0) {
            figure.line(
                &[(frame.left, y.map(0.0)), (frame.right, y.map(0.0))],
                dashed,
            );
        }
        for (index, sample) in data.samples.iter().enumerate() {
            let (px, py) = (x.map(coordinate(index, cx)), y.map(coordinate(index, cy)));
            let shape = if options.unique_shapes {
                Marker::for_group(sample.group)
            } else {
                Marker::Circle
            };
            marker(
                figure,
                shape,
                px,
                py,
                options.marker_size,
                group_color(sample.group),
            );
            let r = options.marker_size.max(0.0).sqrt() / 2.0 + 1.0;
            let group = data
                .groups
                .get(sample.group)
                .map(String::as_str)
                .unwrap_or("");
            figure.region(
                px - r,
                py - r,
                2.0 * r,
                2.0 * r,
                &sample.name,
                format!("{}\n{group}", sample.name),
            );
        }
        let x_ticks = if x_label {
            scaled_ticks(&ticks[cx], &x)
        } else {
            scaled_ticks(&ticks[cx], &x)
                .into_iter()
                .map(|(p, _)| (p, String::new()))
                .collect()
        };
        let rotation = if rotate {
            LabelRotation::Diagonal
        } else {
            LabelRotation::Horizontal
        };
        // Unlabelled axes still get their tick marks.
        let stroke = Stroke::solid(AXES_COLOR, 1.0);
        if !x_label {
            for (p, _) in &x_ticks {
                figure.line(&[(*p, frame.bottom), (*p, frame.bottom - 4.0)], stroke);
            }
        }
        x_axis(
            figure,
            &frame,
            &x_ticks,
            x_label.then(|| label(cx)).as_deref(),
            rotation,
            AXES_COLOR,
        );
        if y_label {
            y_axis(
                figure,
                &frame,
                &scaled_ticks(&ticks[cy], &y),
                Some(&label(cy)),
                AXES_COLOR,
            );
        } else {
            figure.line(
                &[(frame.left, frame.top), (frame.left, frame.bottom)],
                stroke,
            );
            for (p, _) in scaled_ticks(&ticks[cy], &y) {
                figure.line(&[(frame.left, p), (frame.left + 4.0, p)], stroke);
            }
        }
    };

    if show_pc3 {
        // PC1 against PC2, with PC3 against PC2 to its right and PC1 against
        // PC3 below, so that neighbouring panels share an axis.
        panel(&mut figure, main, 0, 1, false, true, false);
        let right = Frame {
            left: main.right + gap,
            top: main.top,
            right: figure.width - padding,
            bottom: main.bottom,
        };
        panel(&mut figure, right, 2, 1, true, false, true);
        let below = Frame {
            left: main.left,
            top: main.bottom + gap,
            right: main.right,
            bottom: figure.height - padding - bottom_labels,
        };
        panel(&mut figure, below, 0, 2, true, true, false);
    } else {
        panel(&mut figure, main, 0, 1, true, true, false);
    }

    let entries: Vec<(String, Color)> = data
        .groups
        .iter()
        .enumerate()
        .map(|(i, g)| (g.clone(), group_color(i)))
        .collect();
    legend(&mut figure, &main, &entries, options.legend);
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data(features: usize) -> Data {
        let sample = |name: &str, group: usize, seed: f64| Sample {
            name: name.to_string(),
            group,
            proportions: (0..features)
                .map(|f| {
                    ((seed * (f as f64 + 1.3)).sin() + 1.0) / 10.0
                        + group as f64 * 0.2 * (f % 2) as f64
                })
                .collect(),
        };
        Data {
            samples: vec![
                sample("a1", 0, 1.0),
                sample("a2", 0, 2.0),
                sample("a3", 0, 3.0),
                sample("b1", 1, 4.0),
                sample("b2", 1, 5.0),
            ],
            groups: vec!["A".to_string(), "B".to_string()],
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

    #[test]
    fn three_panels_show_every_sample() {
        let figure = plot(&data(6), &Options::default());
        assert_eq!(figure.regions.len(), 15);
        let texts = texts(&figure);
        for component in ["PC1 (", "PC2 (", "PC3 ("] {
            assert!(
                texts.iter().any(|t| t.starts_with(component)),
                "missing {component}"
            );
        }
        // Percentages are of the total variance.
        let percent = |prefix: &str| -> f64 {
            let t = texts.iter().find(|t| t.starts_with(prefix)).unwrap();
            t[prefix.len()..t.len() - 2].parse().unwrap()
        };
        assert!(percent("PC1 (") >= percent("PC2 (") && percent("PC2 (") >= percent("PC3 ("));
        assert!(figure.regions[3].tooltip.ends_with("\nB"));

        let two = plot(
            &data(6),
            &Options {
                show_pc3: false,
                legend: LegendPosition::UpperRight,
                ..Options::default()
            },
        );
        assert_eq!(two.regions.len(), 5);
        assert!(self::texts(&two).contains(&"A".to_string()));
    }

    #[test]
    fn groups_get_their_own_shapes() {
        let shaped = plot(
            &data(6),
            &Options {
                show_pc3: false,
                ..Options::default()
            },
        );
        let squares = shaped
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == group_color(1)))
            .count();
        assert_eq!(squares, 2);
        let round = plot(
            &data(6),
            &Options {
                show_pc3: false,
                unique_shapes: false,
                ..Options::default()
            },
        );
        assert_eq!(
            round
                .shapes
                .iter()
                .filter(|s| matches!(s, Shape::Circle { .. }))
                .count(),
            5
        );
    }

    #[test]
    fn degenerate_data() {
        let mut same = data(4);
        for s in same.samples.iter_mut() {
            s.proportions = vec![0.25; 4];
        }
        assert!(texts(&plot(&same, &Options::default()))[0].starts_with("Degenerate PCA plot"));
        // One feature: a single direction of variation.
        assert!(texts(&plot(&data(1), &Options::default()))[0].contains("single direction"));
        assert!(texts(&plot(
            &data(6),
            &Options {
                width: 1.5,
                ..Options::default()
            }
        ))[0]
            .contains("too small"));
    }
}
