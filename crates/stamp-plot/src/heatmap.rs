//! Heatmap of features against samples, with optional dendrograms.

use stamp_core::stats::cluster::{cluster, Dendrogram, Linkage};
use stamp_core::{GroupProfile, MultiGroupProfile, Profile};

use crate::axes::{group_color, label_style, legend, Frame, LegendPosition, FONT_SIZE, INCH};
use crate::figure::{Color, Figure, HAlign, Stroke, VAlign};
use crate::text;

/// Colour scales for the cells, from low to high values: the fourteen of
/// STAMP v2, under the same names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMap {
    Blues,
    BlueToRedToGreen,
    BlueToWhiteToRed,
    CoolToWarm,
    Grayscale,
    Jet,
    OrangeToRed,
    Paired,
    PurpleToGreen,
    Reds,
    RedToBlue,
    RedToYellowToBlue,
    Spectral,
    YellowToOrangeToRed,
}

impl ColorMap {
    pub const ALL: [ColorMap; 14] = [
        ColorMap::Blues,
        ColorMap::BlueToRedToGreen,
        ColorMap::BlueToWhiteToRed,
        ColorMap::CoolToWarm,
        ColorMap::Grayscale,
        ColorMap::Jet,
        ColorMap::OrangeToRed,
        ColorMap::Paired,
        ColorMap::PurpleToGreen,
        ColorMap::Reds,
        ColorMap::RedToBlue,
        ColorMap::RedToYellowToBlue,
        ColorMap::Spectral,
        ColorMap::YellowToOrangeToRed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ColorMap::Blues => "Blues",
            ColorMap::BlueToRedToGreen => "Blue to red to green",
            ColorMap::BlueToWhiteToRed => "Blue to white to red",
            ColorMap::CoolToWarm => "Cool to warm",
            ColorMap::Grayscale => "Grayscale",
            ColorMap::Jet => "Jet",
            ColorMap::OrangeToRed => "Orange to red",
            ColorMap::Paired => "Paired",
            ColorMap::PurpleToGreen => "Purple to green",
            ColorMap::Reds => "Reds",
            ColorMap::RedToBlue => "Red to blue",
            ColorMap::RedToYellowToBlue => "Red to yellow to blue",
            ColorMap::Spectral => "Spectral",
            ColorMap::YellowToOrangeToRed => "Yellow to orange to red",
        }
    }

    /// Anchor colours, evenly spaced from 0 to 1, between which colours
    /// are interpolated. These follow the matplotlib colour maps STAMP v2
    /// used (`Blues`, `brg`, `bwr`, `cool`, `gist_yarg`, `jet`, `OrRd`,
    /// `Paired`, `PRGn`, `Reds`, `RdBu`, `RdYlBu`, `spectral`, `YlOrRd`).
    fn anchors(self) -> &'static [(u8, u8, u8)] {
        match self {
            ColorMap::Blues => &[
                (247, 251, 255),
                (198, 219, 239),
                (107, 174, 214),
                (33, 113, 181),
                (8, 48, 107),
            ],
            ColorMap::Reds => &[
                (255, 245, 240),
                (252, 187, 161),
                (251, 106, 74),
                (203, 24, 29),
                (103, 0, 13),
            ],
            ColorMap::Grayscale => &[(255, 255, 255), (0, 0, 0)],
            ColorMap::OrangeToRed => &[
                (255, 247, 236),
                (253, 212, 158),
                (252, 141, 89),
                (215, 48, 31),
                (127, 0, 0),
            ],
            ColorMap::YellowToOrangeToRed => &[
                (255, 255, 204),
                (254, 217, 118),
                (253, 141, 60),
                (227, 26, 28),
                (128, 0, 38),
            ],
            ColorMap::BlueToWhiteToRed => &[(0, 0, 255), (255, 255, 255), (255, 0, 0)],
            ColorMap::BlueToRedToGreen => &[(0, 0, 255), (255, 0, 0), (0, 255, 0)],
            ColorMap::CoolToWarm => &[(0, 255, 255), (255, 0, 255)],
            ColorMap::Jet => &[
                (0, 0, 128),
                (0, 0, 255),
                (0, 128, 255),
                (0, 255, 255),
                (128, 255, 128),
                (255, 255, 0),
                (255, 128, 0),
                (255, 0, 0),
                (128, 0, 0),
            ],
            ColorMap::Paired => &[
                (166, 206, 227),
                (31, 120, 180),
                (178, 223, 138),
                (51, 160, 44),
                (251, 154, 153),
                (227, 26, 28),
                (253, 191, 111),
                (255, 127, 0),
                (202, 178, 214),
                (106, 61, 154),
                (255, 255, 153),
                (177, 89, 40),
            ],
            ColorMap::PurpleToGreen => &[
                (64, 0, 75),
                (118, 42, 131),
                (153, 112, 171),
                (194, 165, 207),
                (231, 212, 232),
                (247, 247, 247),
                (217, 240, 211),
                (166, 219, 160),
                (90, 174, 97),
                (27, 120, 55),
                (0, 68, 27),
            ],
            ColorMap::RedToBlue => &[
                (103, 0, 31),
                (178, 24, 43),
                (214, 96, 77),
                (244, 165, 130),
                (253, 219, 199),
                (247, 247, 247),
                (209, 229, 240),
                (146, 197, 222),
                (67, 147, 195),
                (33, 102, 172),
                (5, 48, 97),
            ],
            ColorMap::RedToYellowToBlue => &[
                (165, 0, 38),
                (215, 48, 39),
                (244, 109, 67),
                (253, 174, 97),
                (254, 224, 144),
                (255, 255, 191),
                (224, 243, 248),
                (171, 217, 233),
                (116, 173, 209),
                (69, 117, 180),
                (49, 54, 149),
            ],
            ColorMap::Spectral => &[
                (0, 0, 0),
                (119, 0, 136),
                (136, 0, 153),
                (0, 0, 170),
                (0, 0, 221),
                (0, 119, 221),
                (0, 153, 221),
                (0, 170, 170),
                (0, 170, 136),
                (0, 153, 0),
                (0, 187, 0),
                (0, 221, 0),
                (0, 255, 0),
                (187, 255, 0),
                (238, 238, 0),
                (255, 204, 0),
                (255, 153, 0),
                (255, 0, 0),
                (221, 0, 0),
                (204, 0, 0),
                (204, 204, 204),
            ],
        }
    }

    /// Colour for a value between 0 and 1.
    pub fn color(self, fraction: f64) -> Color {
        let anchors = self.anchors();
        let t = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let position = t * (anchors.len() - 1) as f64;
        let low = (position.floor() as usize).min(anchors.len() - 2);
        let mix = position - low as f64;
        let channel =
            |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * mix).round() as u8;
        let (a, b) = (anchors[low], anchors[low + 1]);
        Color::rgb(channel(a.0, b.0), channel(a.1, b.1), channel(a.2, b.2))
    }
}

/// How rows or columns are ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// By hierarchical clustering, which can also be drawn as a dendrogram.
    Cluster(Linkage),
    Alphabetical,
    /// Most abundant first.
    MeanAbundance,
    /// As given.
    Original,
}

/// What a heatmap shows: `values[feature][sample]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub features: Vec<String>,
    pub samples: Vec<String>,
    pub values: Vec<Vec<f64>>,
    /// Group of each sample, as an index into `groups`.
    pub sample_groups: Vec<usize>,
    pub groups: Vec<String>,
    /// Label of the colour scale.
    pub value_label: String,
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

impl Data {
    fn from_profile(
        profile: &Profile,
        sample_groups: Vec<usize>,
        groups: Vec<String>,
        features: Option<&[String]>,
        sequence_counts: bool,
    ) -> Data {
        let chosen: Vec<&stamp_core::Feature> = match features {
            Some(names) => names.iter().filter_map(|n| profile.feature(n)).collect(),
            None => profile.features().iter().collect(),
        };
        Data {
            features: chosen.iter().map(|f| f.name.clone()).collect(),
            samples: profile.sample_names.clone(),
            values: chosen
                .iter()
                .map(|f| {
                    if sequence_counts {
                        f.counts.clone()
                    } else {
                        profile.percentages(f)
                    }
                })
                .collect(),
            sample_groups,
            groups,
            value_label: if sequence_counts {
                "Number of sequences"
            } else {
                "Proportion of sequences (%)"
            }
            .to_string(),
            group_colors: Vec::new(),
        }
    }

    /// Proportions in a two-group profile; `features` restricts the rows.
    pub fn from_group_profile(profile: &GroupProfile, features: Option<&[String]>) -> Data {
        Data::from_group_profile_field(profile, features, false)
    }

    /// As [`Data::from_group_profile`], showing sequence counts instead of
    /// proportions if `sequence_counts` is set.
    pub fn from_group_profile_field(
        profile: &GroupProfile,
        features: Option<&[String]>,
        sequence_counts: bool,
    ) -> Data {
        let first = profile.samples_in_group1().len();
        let groups = (0..profile.profile.sample_names.len())
            .map(|i| usize::from(i >= first))
            .collect();
        Data::from_profile(
            &profile.profile,
            groups,
            vec![profile.group_name1.clone(), profile.group_name2.clone()],
            features,
            sequence_counts,
        )
    }

    /// Proportions in a multiple-group profile; `features` restricts the rows.
    pub fn from_multi_group_profile(
        profile: &MultiGroupProfile,
        features: Option<&[String]>,
    ) -> Data {
        Data::from_multi_group_profile_field(profile, features, false)
    }

    /// As [`Data::from_multi_group_profile`], showing sequence counts instead
    /// of proportions if `sequence_counts` is set.
    pub fn from_multi_group_profile_field(
        profile: &MultiGroupProfile,
        features: Option<&[String]>,
        sequence_counts: bool,
    ) -> Data {
        let groups = profile
            .samples_in_groups()
            .iter()
            .enumerate()
            .flat_map(|(g, members)| std::iter::repeat_n(g, members.len()))
            .collect();
        Data::from_profile(
            &profile.profile,
            groups,
            profile.group_names.clone(),
            features,
            sequence_counts,
        )
    }
}

/// Appearance of the heatmap. Lengths are in inches.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    pub row_order: Order,
    pub column_order: Order,
    pub show_row_dendrogram: bool,
    pub show_column_dendrogram: bool,
    /// Space given to each dendrogram.
    pub row_dendrogram_width: f64,
    pub column_dendrogram_height: f64,
    pub color_map: ColorMap,
    /// Show the scale that relates colours to values.
    pub show_color_scale: bool,
    /// Where the legend naming the groups goes, within the whole figure.
    pub legend: LegendPosition,
    /// Mark clusters of features with a coloured strip beside the feature
    /// dendrogram.
    pub show_row_clusters: bool,
    /// Features joined below this fraction of the greatest distance in the
    /// dendrogram form one cluster.
    pub row_cluster_threshold: f64,
    pub truncate_labels: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 7.0,
            row_order: Order::Cluster(Linkage::Average),
            column_order: Order::Cluster(Linkage::Average),
            show_row_dendrogram: true,
            show_column_dendrogram: true,
            row_dendrogram_width: 1.5,
            column_dendrogram_height: 1.5,
            color_map: ColorMap::Blues,
            show_color_scale: true,
            legend: LegendPosition::LowerLeft,
            show_row_clusters: true,
            row_cluster_threshold: 0.75,
            truncate_labels: Some(50),
        }
    }
}

/// Colours of the clusters of features (ColorBrewer Set3), used in turn.
const CLUSTER_COLORS: [(u8, u8, u8); 12] = [
    (141, 211, 199),
    (255, 255, 179),
    (190, 186, 218),
    (251, 128, 114),
    (128, 177, 211),
    (253, 180, 98),
    (179, 222, 105),
    (252, 205, 229),
    (217, 217, 217),
    (188, 128, 189),
    (204, 235, 197),
    (255, 237, 111),
];

/// The cluster of each item when a dendrogram is cut at a height: items
/// joined at or below `cutoff` share a cluster. Clusters are numbered from
/// zero in the order of the dendrogram's leaves.
pub fn flat_clusters(tree: &Dendrogram, cutoff: f64) -> Vec<usize> {
    let n = tree.order.len();
    let mut parent: Vec<usize> = (0..n + tree.merges.len()).collect();
    fn root(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    for (i, merge) in tree.merges.iter().enumerate() {
        if merge.height <= cutoff {
            for child in [merge.left, merge.right] {
                let top = root(&mut parent, child);
                parent[top] = n + i;
            }
        }
    }
    let mut numbers: Vec<(usize, usize)> = Vec::new();
    let mut clusters = vec![0; n];
    for &item in &tree.order {
        let top = root(&mut parent, item);
        let number = match numbers.iter().find(|(node, _)| *node == top) {
            Some((_, number)) => *number,
            None => {
                numbers.push((top, numbers.len()));
                numbers.len() - 1
            }
        };
        clusters[item] = number;
    }
    clusters
}

/// The most rows or columns a heatmap will draw.
pub const MAX_ITEMS: usize = 1000;

/// Order of items (each a vector of values) and, if clustered, their tree.
fn arrange(items: &[Vec<f64>], names: &[String], order: Order) -> (Vec<usize>, Option<Dendrogram>) {
    let identity: Vec<usize> = (0..items.len()).collect();
    match order {
        Order::Original => (identity, None),
        Order::Alphabetical => {
            let mut sorted = identity;
            sorted.sort_by(|&a, &b| names[a].cmp(&names[b]));
            (sorted, None)
        }
        Order::MeanAbundance => {
            let mean = |i: usize| items[i].iter().sum::<f64>() / items[i].len().max(1) as f64;
            let mut sorted = identity;
            sorted.sort_by(|&a, &b| {
                mean(b)
                    .total_cmp(&mean(a))
                    .then_with(|| names[a].cmp(&names[b]))
            });
            (sorted, None)
        }
        Order::Cluster(linkage) => match cluster(items, linkage) {
            Some(tree) => (tree.order.clone(), Some(tree)),
            None => (identity, None),
        },
    }
}

/// Draw a dendrogram. `leaf` gives the position of each leaf along the items;
/// `height` maps a merge height to a position across them. With `vertical`,
/// leaves run along x and heights along y; otherwise the reverse.
fn dendrogram(
    figure: &mut Figure,
    tree: &Dendrogram,
    leaf: &dyn Fn(usize) -> f64,
    height: &dyn Fn(f64) -> f64,
    vertical: bool,
) {
    let n = tree.order.len();
    let stroke = Stroke::solid(Color::BLACK, 0.75);
    let mut rank = vec![0usize; n];
    for (position, &item) in tree.order.iter().enumerate() {
        rank[item] = position;
    }
    // Position along the items and height of every cluster.
    let mut along: Vec<f64> = (0..n).map(|item| leaf(rank[item])).collect();
    let mut across: Vec<f64> = vec![height(0.0); n];
    for merge in &tree.merges {
        let (a, b) = (along[merge.left], along[merge.right]);
        let (ha, hb, top) = (
            across[merge.left],
            across[merge.right],
            height(merge.height),
        );
        let path = [(a, ha), (a, top), (b, top), (b, hb)];
        let points: Vec<(f64, f64)> = if vertical {
            path.to_vec()
        } else {
            path.iter().map(|&(p, q)| (q, p)).collect()
        };
        figure.line(&points, stroke);
        along.push((a + b) / 2.0);
        across.push(top);
    }
}

pub fn plot(data: &Data, options: &Options) -> Figure {
    let (rows, columns) = (data.features.len(), data.samples.len());
    if rows < 2
        || columns < 2
        || data.values.len() != rows
        || data.values.iter().any(|r| r.len() != columns)
    {
        return Figure::message("A heatmap needs at least two features and two samples");
    }
    if rows > MAX_ITEMS || columns > MAX_ITEMS {
        return Figure::message("Heatmap plots are limited to 1000 samples and 1000 features. Filter the features first.");
    }

    let column_vectors: Vec<Vec<f64>> = (0..columns)
        .map(|c| data.values.iter().map(|r| r[c]).collect())
        .collect();
    let (row_order, row_tree) = arrange(&data.values, &data.features, options.row_order);
    let (column_order, column_tree) = arrange(&column_vectors, &data.samples, options.column_order);
    let row_tree = row_tree.filter(|_| options.show_row_dendrogram);
    let column_tree = column_tree.filter(|_| options.show_column_dendrogram);

    let shorten = |name: &str| match options.truncate_labels {
        Some(length) => text::truncate(name, length),
        None => name.to_string(),
    };
    let row_labels: Vec<String> = row_order
        .iter()
        .map(|&r| shorten(&data.features[r]))
        .collect();
    let column_labels: Vec<String> = column_order
        .iter()
        .map(|&c| shorten(&data.samples[c]))
        .collect();
    let widest = |labels: &[String]| {
        labels
            .iter()
            .map(|l| text::width(l, FONT_SIZE))
            .fold(0.0, f64::max)
    };
    let (row_label_width, column_label_height) = (widest(&row_labels), widest(&column_labels));

    let mut figure = Figure::with_size_inches(options.width, options.height);
    let margin = 0.15 * INCH;
    let strip = 6.0; // group colour strip above the columns
    let legend_height = if options.show_color_scale {
        0.45 * INCH
    } else {
        0.0
    };
    let row_clusters = row_tree
        .as_ref()
        .filter(|_| options.show_row_clusters)
        .map(|tree| flat_clusters(tree, options.row_cluster_threshold * tree.max_height()));
    // Room for the strip of cluster colours beside the feature dendrogram.
    let cluster_strip = if row_clusters.is_some() {
        strip + 2.0
    } else {
        0.0
    };
    let left = margin
        + if row_tree.is_some() {
            options.row_dendrogram_width * INCH
        } else {
            0.0
        }
        + cluster_strip;
    let top = margin
        + if column_tree.is_some() {
            options.column_dendrogram_height * INCH
        } else {
            0.0
        }
        + strip
        + 2.0;
    let right = figure.width - margin - row_label_width - 4.0;
    let bottom = figure.height - margin - column_label_height - 4.0 - legend_height;
    if right - left < 0.5 * INCH || bottom - top < 0.5 * INCH {
        return Figure::message("The figure is too small for this heatmap; increase its size or shorten the dendrograms");
    }
    let (cell_w, cell_h) = (
        (right - left) / columns as f64,
        (bottom - top) / rows as f64,
    );

    let finite = data
        .values
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite());
    let (low, high) = finite.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    let span = if high > low { high - low } else { 1.0 };

    for (r, &row) in row_order.iter().enumerate() {
        for (c, &column) in column_order.iter().enumerate() {
            let value = data.values[row][column];
            let color = options.color_map.color((value - low) / span);
            // Cells overlap slightly so no background shows between them.
            figure.rect(
                left + cell_w * c as f64,
                top + cell_h * r as f64,
                cell_w + 0.3,
                cell_h + 0.3,
                Some(color),
                None,
            );
        }
    }
    figure.rect(
        left,
        top,
        right - left,
        bottom - top,
        None,
        Some(Stroke::solid(Color::BLACK, 0.5)),
    );
    for (r, &row) in row_order.iter().enumerate() {
        figure.region(
            left,
            top + cell_h * r as f64,
            right - left,
            cell_h,
            &data.features[row],
            data.features[row].clone(),
        );
    }

    // Labels, when the cells are large enough to read them.
    let style = label_style();
    if cell_h >= FONT_SIZE * 0.8 {
        for (r, label) in row_labels.iter().enumerate() {
            figure.text(
                right + 4.0,
                top + cell_h * (r as f64 + 0.5),
                label.clone(),
                style.align(HAlign::Left, VAlign::Middle),
            );
        }
    }
    if cell_w >= FONT_SIZE * 0.8 {
        for (c, label) in column_labels.iter().enumerate() {
            figure.text(
                left + cell_w * (c as f64 + 0.5),
                bottom + 4.0,
                label.clone(),
                style.align(HAlign::Right, VAlign::Middle).rotated(90.0),
            );
        }
    }

    // Group of each sample.
    for (c, &column) in column_order.iter().enumerate() {
        let group = data.sample_groups.get(column).copied().unwrap_or(0);
        figure.rect(
            left + cell_w * c as f64,
            top - strip - 2.0,
            cell_w + 0.3,
            strip,
            Some(data.group_color(group)),
            None,
        );
    }

    if let Some(clusters) = &row_clusters {
        for (r, &row) in row_order.iter().enumerate() {
            let (red, green, blue) = CLUSTER_COLORS[clusters[row] % CLUSTER_COLORS.len()];
            figure.rect(
                left - cluster_strip,
                top + cell_h * r as f64,
                strip,
                cell_h + 0.3,
                Some(Color::rgb(red, green, blue)),
                None,
            );
        }
    }
    if let Some(tree) = &row_tree {
        let max = tree.max_height().max(f64::MIN_POSITIVE);
        let leaf = |position: usize| top + cell_h * (position as f64 + 0.5);
        let base = left - cluster_strip - 2.0;
        let across = |h: f64| base - (base - margin) * h / max;
        dendrogram(&mut figure, tree, &leaf, &across, false);
    }
    if let Some(tree) = &column_tree {
        let max = tree.max_height().max(f64::MIN_POSITIVE);
        let base = top - strip - 4.0;
        let leaf = |position: usize| left + cell_w * (position as f64 + 0.5);
        let across = |h: f64| base - (base - margin) * h / max;
        dendrogram(&mut figure, tree, &leaf, &across, true);
    }

    let entries: Vec<(String, Color)> = data
        .groups
        .iter()
        .enumerate()
        .map(|(g, name)| (name.clone(), data.group_color(g)))
        .collect();
    let legend_width = 14.0
        + 5.0
        + entries
            .iter()
            .map(|(name, _)| text::width(name, FONT_SIZE))
            .fold(0.0, f64::max);

    if options.show_color_scale {
        // The scale sits in the band along the bottom, clear of a legend in
        // the lower-left corner.
        let bar_left = if options.legend == LegendPosition::LowerLeft {
            margin + 8.0 + legend_width + 0.25 * INCH
        } else {
            margin
        };
        let (bar_top, bar_w, bar_h) = (figure.height - margin - 22.0, 1.5 * INCH, 8.0);
        let steps = 60;
        for i in 0..steps {
            let color = options.color_map.color(i as f64 / (steps - 1) as f64);
            figure.rect(
                bar_left + bar_w * i as f64 / steps as f64,
                bar_top,
                bar_w / steps as f64 + 0.3,
                bar_h,
                Some(color),
                None,
            );
        }
        figure.rect(
            bar_left,
            bar_top,
            bar_w,
            bar_h,
            None,
            Some(Stroke::solid(Color::BLACK, 0.5)),
        );
        figure.text(
            bar_left,
            bar_top + bar_h + 3.0,
            format!("{low:.2}"),
            style.align(HAlign::Left, VAlign::Top),
        );
        figure.text(
            bar_left + bar_w,
            bar_top + bar_h + 3.0,
            format!("{high:.2}"),
            style.align(HAlign::Right, VAlign::Top),
        );
        figure.text(
            bar_left + bar_w / 2.0,
            bar_top - 3.0,
            data.value_label.clone(),
            style.align(HAlign::Center, VAlign::Bottom),
        );
    }

    // The groups, in a corner of the whole figure as in STAMP v2.
    let whole = Frame {
        left: margin - 8.0,
        top: margin - 8.0,
        right: figure.width - margin + 8.0,
        bottom: figure.height - margin + 8.0,
    };
    legend(&mut figure, &whole, &entries, options.legend);
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data() -> Data {
        Data {
            features: vec!["zeta".into(), "alpha".into(), "mid".into()],
            samples: vec!["s1".into(), "s2".into(), "s3".into(), "s4".into()],
            values: vec![
                vec![1.0, 9.0, 1.2, 9.5],
                vec![5.0, 5.1, 5.0, 5.2],
                vec![1.1, 8.0, 1.0, 9.0],
            ],
            sample_groups: vec![0, 1, 0, 1],
            groups: vec!["Lean".into(), "Obese".into()],
            value_label: "Proportion of sequences (%)".to_string(),
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

    fn row_order(figure: &Figure) -> Vec<String> {
        figure.regions.iter().map(|r| r.item.clone()).collect()
    }

    #[test]
    fn colour_maps_run_from_light_to_dark() {
        assert_eq!(ColorMap::Blues.color(0.0), Color::rgb(247, 251, 255));
        assert_eq!(ColorMap::Blues.color(1.0), Color::rgb(8, 48, 107));
        assert_eq!(ColorMap::Grayscale.color(0.5), Color::rgb(128, 128, 128));
        assert_eq!(ColorMap::BlueToWhiteToRed.color(0.5), Color::WHITE);
        assert_eq!(ColorMap::Reds.color(f64::NAN), ColorMap::Reds.color(0.0));
        assert_eq!(ColorMap::Reds.color(7.0), ColorMap::Reds.color(1.0));
    }

    #[test]
    fn clustering_puts_similar_rows_and_columns_together() {
        let figure = plot(&data(), &Options::default());
        let order = row_order(&figure);
        let position = |name: &str| order.iter().position(|n| n == name).unwrap();
        assert_eq!(position("zeta").abs_diff(position("mid")), 1);
        // 12 cells, 4 group strips, 3 cluster strips, the colour bar and
        // the legend.
        let filled = figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect { fill: Some(_), .. }))
            .count();
        assert_eq!(filled, 12 + 4 + 3 + 60 + 2);
        // Two dendrograms: 2 + 3 merges.
        let trees = figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Line { points, .. } if points.len() == 4))
            .count();
        assert_eq!(trees, 5);
        let texts = texts(&figure);
        for expected in [
            "zeta",
            "s4",
            "Lean",
            "Obese",
            "1.00",
            "9.50",
            "Proportion of sequences (%)",
        ] {
            assert!(texts.iter().any(|t| t == expected), "missing {expected}");
        }
    }

    #[test]
    fn other_orders_and_options() {
        let plain = |row_order| Options {
            row_order,
            column_order: Order::Original,
            show_color_scale: false,
            ..Options::default()
        };
        assert_eq!(
            row_order(&plot(&data(), &plain(Order::Alphabetical))),
            ["alpha", "mid", "zeta"]
        );
        assert_eq!(
            row_order(&plot(&data(), &plain(Order::MeanAbundance))),
            ["zeta", "alpha", "mid"]
        );
        assert_eq!(
            row_order(&plot(&data(), &plain(Order::Original))),
            ["zeta", "alpha", "mid"]
        );

        let no_trees = Options {
            show_row_dendrogram: false,
            show_column_dendrogram: false,
            ..Options::default()
        };
        let figure = plot(&data(), &no_trees);
        assert!(!figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Line { points, .. } if points.len() == 4)));
    }

    #[test]
    fn limits_and_degenerate_data() {
        let mut one = data();
        one.features.truncate(1);
        one.values.truncate(1);
        assert!(texts(&plot(&one, &Options::default()))[0].contains("at least two features"));

        let big = Data {
            features: (0..1001).map(|i| format!("f{i}")).collect(),
            samples: vec!["a".into(), "b".into()],
            values: vec![vec![1.0, 2.0]; 1001],
            sample_groups: vec![0, 1],
            groups: vec!["A".into(), "B".into()],
            value_label: String::new(),
            group_colors: Vec::new(),
        };
        assert!(texts(&plot(&big, &Options::default()))[0].contains("limited to 1000"));
        assert!(texts(&plot(
            &data(),
            &Options {
                width: 2.0,
                ..Options::default()
            }
        ))[0]
            .contains("too small"));

        // Constant values still draw.
        let mut flat = data();
        flat.values = vec![vec![2.0; 4]; 3];
        assert_eq!(plot(&flat, &Options::default()).regions.len(), 3);
    }

    #[test]
    fn every_colour_map_has_a_name_and_colours() {
        let mut names: Vec<&str> = ColorMap::ALL.iter().map(|m| m.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 14);
        for map in ColorMap::ALL {
            // The two ends differ, and nothing goes wrong in between.
            assert_ne!(map.color(0.0), map.color(1.0), "{}", map.name());
            for step in 0..=100 {
                map.color(f64::from(step) / 100.0);
            }
        }
        assert_eq!(ColorMap::Jet.color(0.0), Color::rgb(0, 0, 128));
        assert_eq!(ColorMap::CoolToWarm.color(0.5), Color::rgb(128, 128, 255));
        assert_eq!(ColorMap::BlueToRedToGreen.color(0.5), Color::rgb(255, 0, 0));
        assert_eq!(ColorMap::RedToBlue.color(0.5), Color::rgb(247, 247, 247));
        assert_eq!(ColorMap::Spectral.color(1.0), Color::rgb(204, 204, 204));
    }

    #[test]
    fn dendrograms_are_cut_into_clusters() {
        let items = vec![
            vec![0.0, 0.0],
            vec![10.0, 10.0],
            vec![0.1, 0.0],
            vec![10.0, 10.2],
            vec![30.0, 30.0],
        ];
        let tree = cluster(&items, Linkage::Average).unwrap();
        let max = tree.max_height();
        // Cut near the top: the two tight pairs join, the outlier stays apart.
        let wide = flat_clusters(&tree, 0.75 * max);
        assert_eq!(wide[0], wide[2]);
        assert_eq!(wide[1], wide[3]);
        assert_eq!(wide[0], wide[1]);
        assert_ne!(wide[0], wide[4]);
        // Cut low: only the tight pairs remain together.
        let tight = flat_clusters(&tree, 0.05 * max);
        assert_eq!(tight[0], tight[2]);
        assert_eq!(tight[1], tight[3]);
        assert_ne!(tight[0], tight[1]);
        // At the top everything is one cluster; below every merge, none.
        assert!(flat_clusters(&tree, max).iter().all(|&c| c == 0));
        let mut apart = flat_clusters(&tree, -1.0);
        apart.sort_unstable();
        assert_eq!(apart, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn cluster_strip_legend_and_counts() {
        let filled = |options: &Options| {
            plot(&data(), options)
                .shapes
                .iter()
                .filter(|s| matches!(s, Shape::Rect { fill: Some(_), .. }))
                .count()
        };
        let all = filled(&Options::default());
        let no_clusters = Options {
            show_row_clusters: false,
            ..Options::default()
        };
        assert_eq!(filled(&no_clusters), all - 3);
        let no_legend = Options {
            legend: LegendPosition::None,
            ..Options::default()
        };
        assert_eq!(filled(&no_legend), all - 2);
        assert!(!texts(&plot(&data(), &no_legend)).contains(&"Lean".to_string()));

        // The legend goes to the corner asked for.
        let swatch_x = |position| {
            let figure = plot(
                &data(),
                &Options {
                    legend: position,
                    ..Options::default()
                },
            );
            let label = figure
                .shapes
                .iter()
                .position(|s| matches!(s, Shape::Text { text, .. } if text == "Lean"));
            match &figure.shapes[label.unwrap() - 1] {
                Shape::Rect { x, y, .. } => (*x, *y),
                other => panic!("{other:?}"),
            }
        };
        let (left, low) = swatch_x(LegendPosition::LowerLeft);
        let (right, high) = swatch_x(LegendPosition::UpperRight);
        assert!(left < 100.0 && right > 300.0 && low > 300.0 && high < 100.0);
    }
}
