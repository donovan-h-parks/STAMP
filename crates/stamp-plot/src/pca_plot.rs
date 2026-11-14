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
    /// Colour of each group; groups beyond the list use the default palette.
    pub group_colors: Vec<Color>,
}

impl Data {
    /// Colour of the group at `index`.
    pub fn group_color(&self, index: usize) -> Color {
        self.group_colors
            .get(index)
            .copied()
            .unwrap_or_else(|| group_color(index))
    }
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
            group_colors: Vec::new(),
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
            group_colors: Vec::new(),
        }
    }
}

/// Appearance of the PCA plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Area of the markers, in square points.
    pub marker_size: f64,
    /// Show the third component against the second, beside the main panel.
    pub show_pc3_vs_pc2: bool,
    /// Show the first component against the third, beneath the main panel.
    pub show_pc1_vs_pc3: bool,
    /// Draw the tick labels of the PC3 axis beside the main panel at an
    /// angle.
    pub rotate_pc3_labels: bool,
    /// Give every axis the same length per unit of distance, so that
    /// distances between samples compare across panels. The height of the
    /// figure then follows from its width.
    pub fixed_scale: bool,
    /// Give each group its own marker shape as well as its own colour.
    pub unique_shapes: bool,
    pub legend: LegendPosition,
    /// Size of the notes above the plot (the distance and the community
    /// tests, for PCoA and NMDS), in points.
    pub note_font_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 6.0,
            marker_size: 30.0,
            show_pc3_vs_pc2: true,
            show_pc1_vs_pc3: true,
            rotate_pc3_labels: true,
            fixed_scale: true,
            unique_shapes: true,
            legend: LegendPosition::None,
            note_font_size: crate::axes::NOTE_SIZE,
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
    plot_ordination(
        data,
        &result.positions,
        &result.variance_fractions,
        "PC",
        &[],
        options,
    )
}

/// Draw samples at positions found by any ordination: `positions[sample]`
/// holds the coordinates of the sample of `data` with the same index, and
/// `variance_fractions` the share of the variation each axis accounts for.
/// Axes are named `axis` followed by their number, and `notes` are written
/// above the plot, one to a line.
pub fn plot_ordination(
    data: &Data,
    positions: &[Vec<f64>],
    variance_fractions: &[f64],
    axis: &str,
    notes: &[String],
    options: &Options,
) -> Figure {
    if positions.len() != data.samples.len() {
        return Figure::message("Degenerate plot: the ordination does not match the samples");
    }
    let components = variance_fractions.len();
    if components < 2 {
        return Figure::message("Degenerate plot: the samples vary along a single direction.");
    }
    let beside = options.show_pc3_vs_pc2 && components >= 3;
    let beneath = options.show_pc1_vs_pc3 && components >= 3;
    let coordinate = |sample: usize, component: usize| positions[sample][component];
    // An ordination with no share of the variation per axis (NMDS) gives
    // `NaN`, and its axes are named only.
    let label = |component: usize| {
        let fraction = variance_fractions[component];
        if fraction.is_nan() {
            format!("{axis}{}", component + 1)
        } else {
            format!("{axis}{} ({:.1}%)", component + 1, fraction * 100.0)
        }
    };
    let ticks_for = |component: usize| {
        let values = (0..data.samples.len()).map(|s| coordinate(s, component));
        let (low, high) = values.fold((0.0f64, 0.0f64), |(lo, hi), v| (lo.min(v), hi.max(v)));
        let pad = (high - low) * 0.05;
        Ticks::inside(low - pad, high + pad)
    };
    let ticks: Vec<Ticks> = (0..if beside || beneath { 3 } else { 2 })
        .map(ticks_for)
        .collect();
    let span = |component: usize| ticks[component].max - ticks[component].min;

    let width = options.width * INCH;
    let padding = 0.25 * INCH;
    // Room above the plot for the notes.
    let note_line = options.note_font_size + 3.0;
    let top = padding + note_line * notes.len() as f64;
    let left = padding + y_axis_width(&ticks[1], true);
    let bottom_labels = crate::axes::x_axis_room();
    let gap = 0.35 * INCH;
    let inner_width = width - left - padding;
    // Width and height of the main panel, and the sizes of the others.
    let (main_w, main_h, beside_w, beneath_h, height);
    if options.fixed_scale {
        // One length per unit of distance, set by what must fit across.
        let across = span(0) + if beside { span(2) } else { 0.0 };
        let room = inner_width - if beside { gap } else { 0.0 };
        if !(across > 0.0 && span(1) > 0.0 && room > 0.0) {
            return Figure::message("Degenerate PCA plot");
        }
        let unit = room / across;
        main_w = span(0) * unit;
        main_h = span(1) * unit;
        beside_w = if beside { span(2) * unit } else { 0.0 };
        beneath_h = if beneath { span(2) * unit } else { 0.0 };
        height =
            top + padding + main_h + if beneath { gap + beneath_h } else { 0.0 } + bottom_labels;
        if height > 6.0 * width {
            return Figure::message(
                "The PCA plot would be too tall; switch off \"fixed length per unit distance\"",
            );
        }
    } else {
        height = options.height * INCH;
        let inner_height = height - top - padding - bottom_labels;
        // With further panels the main one takes about two thirds.
        main_w = if beside {
            (inner_width - gap) * 0.66
        } else {
            inner_width
        };
        main_h = if beneath {
            (inner_height - gap) * 0.6
        } else {
            inner_height
        };
        beside_w = inner_width - gap - main_w;
        beneath_h = inner_height - gap - main_h;
    }
    if main_w < 0.75 * INCH || main_h < 0.75 * INCH {
        return Figure::message("The figure is too small for a PCA plot");
    }
    let mut figure = Figure::with_size_inches(options.width, height / INCH);
    let main = Frame {
        left,
        top,
        right: left + main_w,
        bottom: top + main_h,
    };
    for (line, note) in notes.iter().enumerate() {
        figure.text(
            main.left,
            padding + note_line * (line as f64 + 0.5) - 4.0,
            note.clone(),
            crate::figure::TextStyle::new(options.note_font_size)
                .align(crate::figure::HAlign::Left, crate::figure::VAlign::Middle),
        );
    }

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
                data.group_color(sample.group),
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

    // Neighbouring panels share an axis: PC2 with the panel beside the main
    // one, PC1 with the panel beneath it.
    panel(&mut figure, main, 0, 1, !beneath, true, false);
    if beside {
        let frame = Frame {
            left: main.right + gap,
            top: main.top,
            right: main.right + gap + beside_w,
            bottom: main.bottom,
        };
        panel(
            &mut figure,
            frame,
            2,
            1,
            true,
            false,
            options.rotate_pc3_labels,
        );
    }
    if beneath {
        let frame = Frame {
            left: main.left,
            top: main.bottom + gap,
            right: main.right,
            bottom: main.bottom + gap + beneath_h,
        };
        panel(&mut figure, frame, 0, 2, true, true, false);
    }

    let entries: Vec<(String, Color)> = data
        .groups
        .iter()
        .enumerate()
        .map(|(i, g)| (g.clone(), data.group_color(i)))
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
                show_pc3_vs_pc2: false,
                show_pc1_vs_pc3: false,
                legend: LegendPosition::UpperRight,
                ..Options::default()
            },
        );
        assert_eq!(two.regions.len(), 5);
        assert!(self::texts(&two).contains(&"A".to_string()));
    }

    #[test]
    fn panels_are_chosen_one_by_one() {
        let regions = |beside: bool, beneath: bool| {
            plot(
                &data(6),
                &Options {
                    show_pc3_vs_pc2: beside,
                    show_pc1_vs_pc3: beneath,
                    ..Options::default()
                },
            )
            .regions
            .len()
        };
        assert_eq!(regions(true, false), 10);
        assert_eq!(regions(false, true), 10);
        assert_eq!(regions(false, false), 5);

        // The PC3 tick labels beside the main panel are drawn at an angle
        // unless asked otherwise.
        let angled = |rotate: bool| {
            plot(
                &data(6),
                &Options {
                    rotate_pc3_labels: rotate,
                    ..Options::default()
                },
            )
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Text { style, .. } if style.rotation != 0.0 && style.rotation != 90.0))
            .count()
        };
        assert!(angled(true) > 0);
        assert_eq!(angled(false), 0);
    }

    #[test]
    fn a_fixed_scale_gives_every_axis_the_same_length_per_unit() {
        // The frame of each panel is its bottom and left axis lines.
        let fixed = plot(&data(6), &Options::default());
        let free = plot(
            &data(6),
            &Options {
                fixed_scale: false,
                ..Options::default()
            },
        );
        // Without it the figure has the height asked for; with it the
        // height follows from the data.
        assert_eq!(free.height, 6.0 * INCH);
        assert_eq!(fixed.width, 7.0 * INCH);
        assert!(fixed.height != free.height);
        assert_eq!(fixed.regions.len(), 15);

        // Two samples one unit apart along PC1 and along PC2 are as far
        // apart on the page in both directions.
        let matrix: Vec<Vec<f64>> = data(6)
            .samples
            .iter()
            .map(|s| s.proportions.clone())
            .collect();
        let result = pca(&matrix).unwrap();
        let (a, b) = (&result.positions[0], &result.positions[3]);
        let (ra, rb) = (&fixed.regions[0], &fixed.regions[3]);
        let per_unit_x = (rb.x - ra.x) / (b[0] - a[0]);
        let per_unit_y = (ra.y - rb.y) / (b[1] - a[1]);
        assert!(
            (per_unit_x - per_unit_y).abs() < 1e-6 * per_unit_x.abs(),
            "{per_unit_x} against {per_unit_y}"
        );
    }

    #[test]
    fn groups_get_their_own_shapes() {
        let shaped = plot(
            &data(6),
            &Options {
                show_pc3_vs_pc2: false,
                show_pc1_vs_pc3: false,
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
                show_pc3_vs_pc2: false,
                show_pc1_vs_pc3: false,
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
