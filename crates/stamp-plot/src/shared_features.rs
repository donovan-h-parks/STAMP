//! Shared-feature plot: which features each group has, and which they have
//! in common, as an UpSet plot (Lex et al. 2014) or, for two or three
//! groups, a Venn diagram.

use crate::axes::{
    group_color, scaled_ticks, y_axis, y_axis_width, Frame, AXES_COLOR, INCH, NOTE_SIZE,
};
use crate::axis::Ticks;
use crate::composition;
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::text;

/// The features present in each group.
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub groups: Vec<String>,
    pub group_colors: Vec<Color>,
    pub features: Vec<String>,
    /// `presence[feature][group]`.
    pub presence: Vec<Vec<bool>>,
}

impl Data {
    /// A feature counts as present in a group if it is found (above zero)
    /// in at least `min_prevalence` (a share, from 0 to 1) of the group's
    /// samples, and in at least one. Groups without samples are left out.
    /// Unclassified features are those the data was made with.
    pub fn from_composition(data: &composition::Data, min_prevalence: f64) -> Data {
        let members: Vec<Vec<usize>> = (0..data.groups.len())
            .map(|g| {
                (0..data.samples.len())
                    .filter(|&s| data.sample_groups.get(s) == Some(&g))
                    .collect()
            })
            .collect();
        let used: Vec<usize> = (0..data.groups.len())
            .filter(|&g| !members[g].is_empty())
            .collect();
        let mut features = Vec::new();
        let mut presence = Vec::new();
        for (f, name) in data.features.iter().enumerate() {
            let row: Vec<bool> = used
                .iter()
                .map(|&g| {
                    let found = members[g]
                        .iter()
                        .filter(|&&s| data.values[f][s] > 0.0)
                        .count();
                    found > 0 && found as f64 >= min_prevalence * members[g].len() as f64 - 1e-9
                })
                .collect();
            if row.iter().any(|p| *p) {
                features.push(name.clone());
                presence.push(row);
            }
        }
        Data {
            groups: used.iter().map(|&g| data.groups[g].clone()).collect(),
            group_colors: used.iter().map(|&g| data.group_color(g)).collect(),
            features,
            presence,
        }
    }

    fn color(&self, group: usize) -> Color {
        self.group_colors
            .get(group)
            .copied()
            .unwrap_or_else(|| group_color(group))
    }

    /// Every combination of groups that some feature is in exactly, with
    /// its features (alphabetical), largest first.
    pub fn intersections(&self) -> Vec<(Vec<bool>, Vec<String>)> {
        let mut found: Vec<(Vec<bool>, Vec<String>)> = Vec::new();
        for (feature, row) in self.features.iter().zip(&self.presence) {
            match found.iter_mut().find(|(set, _)| set == row) {
                Some((_, names)) => names.push(feature.clone()),
                None => found.push((row.clone(), vec![feature.clone()])),
            }
        }
        for (_, names) in &mut found {
            names.sort_by_key(|n| n.to_lowercase());
        }
        // Largest first; ties by fewer groups, then by the groups' order.
        found.sort_by(|a, b| {
            b.1.len()
                .cmp(&a.1.len())
                .then(count(&a.0).cmp(&count(&b.0)))
                .then(b.0.cmp(&a.0))
        });
        found
    }

    /// The groups of a combination, by name.
    fn describe(&self, set: &[bool]) -> String {
        let names: Vec<&str> = self
            .groups
            .iter()
            .zip(set)
            .filter(|(_, in_set)| **in_set)
            .map(|(g, _)| g.as_str())
            .collect();
        if names.len() == 1 {
            format!("Only in {}", names[0])
        } else if names.len() == self.groups.len() {
            "In every group".to_string()
        } else {
            format!("In {} only", names.join(" and "))
        }
    }

    fn tooltip(&self, set: &[bool], names: &[String]) -> String {
        const LISTED: usize = 15;
        let mut text = format!("{}\n\n{} features", self.describe(set), names.len());
        for name in names.iter().take(LISTED) {
            text.push_str(&format!("\n{name}"));
        }
        if names.len() > LISTED {
            text.push_str(&format!("\n... and {} more", names.len() - LISTED));
        }
        text
    }
}

fn count(set: &[bool]) -> usize {
    set.iter().filter(|b| **b).count()
}

/// How the plot is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Style {
    UpSet,
    /// Only for two or three groups.
    Venn,
}

/// Appearance of the plot. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    pub style: Style,
    /// A feature is present in a group if found in at least this
    /// percentage of the group's samples.
    pub min_prevalence: f64,
    /// Most combinations of groups shown in an UpSet plot, largest first.
    pub max_intersections: usize,
    /// Size of the group names and of the counts above the bars, in points.
    pub label_font_size: f64,
    /// Size of the counts inside a Venn diagram.
    pub count_font_size: f64,
    /// Size of the note on prevalence above the plot.
    pub note_font_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 5.0,
            style: Style::UpSet,
            min_prevalence: 50.0,
            max_intersections: 20,
            label_font_size: NOTE_SIZE,
            count_font_size: 11.0,
            note_font_size: NOTE_SIZE,
        }
    }
}

/// Colour of the bars of combinations of several groups, and of the
/// matrix's dots for groups not in a combination.
const SHARED: Color = Color::rgb(80, 80, 80);
const ABSENT: Color = Color::rgb(220, 220, 220);

pub fn plot(data: &Data, options: &Options) -> Figure {
    if data.groups.len() < 2 {
        return Figure::message("A shared-feature plot needs at least two groups with samples");
    }
    if data.features.is_empty() {
        return Figure::message("No feature is present in any group at this prevalence");
    }
    match options.style {
        Style::UpSet => upset(data, options),
        Style::Venn if data.groups.len() <= 3 => venn(data, options),
        Style::Venn => Figure::message(
            "A Venn diagram is drawn for two or three groups; choose the UpSet style for more",
        ),
    }
}

fn upset(data: &Data, options: &Options) -> Figure {
    let all = data.intersections();
    let shown: Vec<&(Vec<bool>, Vec<String>)> =
        all.iter().take(options.max_intersections.max(1)).collect();
    let largest = shown.iter().map(|s| s.1.len()).max().unwrap_or(1) as f64;
    let y_ticks = Ticks::covering(0.0, largest * 1.1);

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let row = (options.label_font_size + 6.0).max(14.0);
    let labels: Vec<String> = (0..data.groups.len())
        .map(|g| {
            let size = data.presence.iter().filter(|p| p[g]).count();
            format!("{} ({size})", data.groups[g])
        })
        .collect();
    let label_width = labels
        .iter()
        .map(|l| text::width(l, options.label_font_size))
        .fold(0.0, f64::max);
    let left = padding + label_width.max(y_axis_width(&y_ticks, true)) + 8.0;
    let matrix_height = row * data.groups.len() as f64;
    let frame = Frame {
        left,
        top: padding + options.note_font_size + options.label_font_size + 8.0,
        right: figure.width - padding,
        bottom: figure.height - padding - matrix_height - 8.0,
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for an UpSet plot");
    }
    let y = frame.y_scale(y_ticks.min, y_ticks.max);
    let column = frame.width() / shown.len() as f64;
    let bar = (column * 0.6).min(28.0);
    let style = TextStyle::new(options.label_font_size);
    for (k, (set, names)) in shown.iter().enumerate() {
        let centre = frame.left + column * (k as f64 + 0.5);
        let only = (count(set) == 1).then(|| set.iter().position(|b| *b).unwrap_or(0));
        let fill = only.map_or(SHARED, |g| data.color(g));
        let top = y.map(names.len() as f64);
        figure.rect(
            centre - bar / 2.0,
            top,
            bar,
            frame.bottom - top,
            Some(fill),
            Some(Stroke::solid(Color::BLACK, 0.5)),
        );
        figure.text(
            centre,
            top - 3.0,
            names.len().to_string(),
            style.align(HAlign::Center, VAlign::Baseline),
        );
        figure.region(
            centre - bar / 2.0,
            top,
            bar,
            frame.bottom - top,
            &format!("intersection {k}"),
            data.tooltip(set, names),
        );
        // The dots beneath, joined where the combination has them.
        let row_y = |g: usize| frame.bottom + 8.0 + row * (g as f64 + 0.5);
        let members: Vec<usize> = (0..set.len()).filter(|&g| set[g]).collect();
        if let (Some(&first), Some(&last)) = (members.first(), members.last()) {
            if first != last {
                figure.line(
                    &[(centre, row_y(first)), (centre, row_y(last))],
                    Stroke::solid(SHARED, 2.0),
                );
            }
        }
        for (g, in_set) in set.iter().enumerate() {
            let fill = if *in_set { SHARED } else { ABSENT };
            figure.circle(centre, row_y(g), 4.0, Some(fill), None);
        }
    }
    for (g, label) in labels.iter().enumerate() {
        figure.text(
            frame.left - 8.0,
            frame.bottom + 8.0 + row * (g as f64 + 0.5),
            label.clone(),
            style
                .align(HAlign::Right, VAlign::Middle)
                .color(data.color(g)),
        );
    }
    y_axis(
        &mut figure,
        &frame,
        &scaled_ticks(&y_ticks, &y),
        Some("Features"),
        AXES_COLOR,
    );
    let mut note = format!(
        "Features present in at least {}% of a group's samples",
        options.min_prevalence
    );
    if all.len() > shown.len() {
        note.push_str(&format!(
            "; the {} largest of {} combinations",
            shown.len(),
            all.len()
        ));
    }
    figure.text(
        frame.left,
        padding,
        note,
        TextStyle::new(options.note_font_size).align(HAlign::Left, VAlign::Top),
    );
    figure
}

fn venn(data: &Data, options: &Options) -> Figure {
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let padding = 0.2 * INCH;
    let (width, height) = (figure.width, figure.height);
    let radius = (width * 0.22).min(height * 0.3);
    if radius < 0.5 * INCH {
        return Figure::message("The figure is too small for a Venn diagram");
    }
    let (cx, cy) = (width / 2.0, height / 2.0 + 0.1 * INCH);
    let centres: Vec<(f64, f64)> = if data.groups.len() == 2 {
        vec![(cx - 0.55 * radius, cy), (cx + 0.55 * radius, cy)]
    } else {
        vec![
            (cx - 0.55 * radius, cy - 0.32 * radius),
            (cx + 0.55 * radius, cy - 0.32 * radius),
            (cx, cy + 0.63 * radius),
        ]
    };
    let groups = centres.len();
    let centroid = (
        centres.iter().map(|c| c.0).sum::<f64>() / groups as f64,
        centres.iter().map(|c| c.1).sum::<f64>() / groups as f64,
    );
    // Where the count of each combination goes.
    let place = |set: &[bool]| -> (f64, f64) {
        let members: Vec<usize> = (0..groups).filter(|&g| set[g]).collect();
        match members.len() {
            1 => {
                let c = centres[members[0]];
                let push = if groups == 2 { 0.9 } else { 0.8 };
                if groups == 2 {
                    (c.0 + (c.0 - cx) * push, c.1)
                } else {
                    (
                        c.0 + (c.0 - centroid.0) * push,
                        c.1 + (c.1 - centroid.1) * push,
                    )
                }
            }
            2 if groups == 3 => {
                let (a, b) = (centres[members[0]], centres[members[1]]);
                let other = centres[(0..3).find(|g| !members.contains(g)).unwrap_or(0)];
                let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                (
                    mid.0 + (mid.0 - other.0) * 0.35,
                    mid.1 + (mid.1 - other.1) * 0.35,
                )
            }
            _ => centroid,
        }
    };
    for (g, &(x, y)) in centres.iter().enumerate() {
        figure.circle(x, y, radius, None, Some(Stroke::solid(data.color(g), 2.5)));
        let size = data.presence.iter().filter(|p| p[g]).count();
        let (label_x, label_y, v) = if g < 2 {
            (
                x + if g == 0 { -0.4 } else { 0.4 } * radius,
                y - radius - 6.0,
                VAlign::Baseline,
            )
        } else {
            (x, y + radius + 6.0, VAlign::Top)
        };
        figure.text(
            label_x,
            label_y,
            format!("{} ({size})", data.groups[g]),
            TextStyle::new(options.label_font_size)
                .align(HAlign::Center, v)
                .color(data.color(g)),
        );
    }
    let big = TextStyle::new(options.count_font_size).align(HAlign::Center, VAlign::Middle);
    for (k, (set, names)) in data.intersections().iter().enumerate() {
        let (x, y) = place(set);
        figure.text(x, y, names.len().to_string(), big);
        figure.region(
            x - 14.0,
            y - 9.0,
            28.0,
            18.0,
            &format!("intersection {k}"),
            data.tooltip(set, names),
        );
    }
    figure.text(
        padding,
        padding,
        format!(
            "Features present in at least {}% of a group's samples",
            options.min_prevalence
        ),
        TextStyle::new(options.note_font_size).align(HAlign::Left, VAlign::Top),
    );
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn composition() -> composition::Data {
        // Features by sample; samples S1, S2 in A, S3, S4 in B, S5, S6 in C.
        composition::Data {
            features: ["a", "b", "c", "d", "e"].map(String::from).to_vec(),
            samples: ["S1", "S2", "S3", "S4", "S5", "S6"]
                .map(String::from)
                .to_vec(),
            values: vec![
                vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0], // a: everywhere
                vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0], // b: A, and half of B
                vec![0.0, 0.0, 0.0, 0.0, 2.0, 2.0], // c: C only
                vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // d: half of A
                vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e: nowhere
            ],
            sample_groups: vec![0, 0, 1, 1, 2, 2],
            groups: ["A", "B", "C"].map(String::from).to_vec(),
            group_colors: Vec::new(),
            sort_keys: Vec::new(),
            unclassified: Vec::new(),
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
    fn presence_follows_the_prevalence() {
        let half = Data::from_composition(&composition(), 0.5);
        assert_eq!(half.features, ["a", "b", "c", "d"]);
        assert_eq!(half.presence[1], [true, true, false]);
        assert_eq!(half.presence[3], [true, false, false]);
        let all = Data::from_composition(&composition(), 1.0);
        // b is in only half of B, d in half of A.
        assert_eq!(all.features, ["a", "b", "c"]);
        assert_eq!(all.presence[1], [true, false, false]);

        let sets = half.intersections();
        let sizes: Vec<usize> = sets.iter().map(|s| s.1.len()).collect();
        assert_eq!(sizes, [1, 1, 1, 1]);
        // Ties: single groups first.
        assert_eq!(sets[0].0.iter().filter(|b| **b).count(), 1);
        let only_a = sets.iter().find(|s| s.0 == [true, false, false]).unwrap();
        assert_eq!(only_a.1, ["d"]);
    }

    #[test]
    fn upset_and_venn() {
        let data = Data::from_composition(&composition(), 0.5);
        let upset = plot(&data, &Options::default());
        assert_eq!(upset.regions.len(), 4);
        let shown = texts(&upset);
        assert!(shown.contains(&"A (3)".to_string()), "{shown:?}");
        assert!(shown.contains(&"Features".to_string()));
        assert!(upset
            .regions
            .iter()
            .any(|r| r.tooltip.starts_with("In every group")));
        assert!(upset
            .regions
            .iter()
            .any(|r| r.tooltip.starts_with("In A and B only")));
        let fewer = plot(
            &data,
            &Options {
                max_intersections: 2,
                ..Options::default()
            },
        );
        assert_eq!(fewer.regions.len(), 2);
        assert!(texts(&fewer)
            .iter()
            .any(|t| t.contains("the 2 largest of 4")));

        let venn = plot(
            &data,
            &Options {
                style: Style::Venn,
                ..Options::default()
            },
        );
        assert_eq!(venn.regions.len(), 4);
        assert!(texts(&venn).contains(&"C (2)".to_string()));
        let circles = venn
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Circle { .. }))
            .count();
        assert_eq!(circles, 3);

        // Two groups.
        let mut two = composition();
        two.sample_groups = vec![0, 0, 1, 1, 1, 1];
        let two = Data::from_composition(&two, 0.5);
        assert_eq!(two.groups.len(), 2);
        let venn = plot(
            &two,
            &Options {
                style: Style::Venn,
                ..Options::default()
            },
        );
        assert!(!venn.regions.is_empty());
    }

    #[test]
    fn too_few_groups_or_too_many_for_venn() {
        let mut one = composition();
        one.sample_groups = vec![0; 6];
        let data = Data::from_composition(&one, 0.5);
        assert!(plot(&data, &Options::default()).regions.is_empty());

        let mut four = composition();
        four.groups.push("D".to_string());
        four.sample_groups = vec![0, 0, 1, 1, 2, 3];
        let data = Data::from_composition(&four, 0.5);
        assert_eq!(data.groups.len(), 4);
        assert!(!plot(&data, &Options::default()).regions.is_empty());
        let venn = plot(
            &data,
            &Options {
                style: Style::Venn,
                ..Options::default()
            },
        );
        assert!(venn.regions.is_empty());
    }
}
