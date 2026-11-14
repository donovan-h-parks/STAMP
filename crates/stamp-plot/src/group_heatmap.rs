//! Heatmap of a summary of each feature within each group of samples: rows
//! are features, columns are groups, and each cell is the feature's mean,
//! median or largest proportion in the group, or its prevalence. Unlike the
//! heatmap of samples it has no limit on the number of samples, since it
//! shows groups.

use stamp_core::stats::group_summary::{majority_values, summarise, GroupStatistic};
use stamp_core::{GroupProfile, Metadata, MultiGroupProfile, Profile};

use crate::axes::{axis_label_style, fonts, group_color, tick_style, INCH, NOTE_SIZE};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::heatmap::{arrange, dendrogram, ColorMap, Order};
use crate::text;

/// How values are turned into colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Scale {
    Linear,
    /// Logarithm to base 10; values below the floor are drawn as the floor.
    Log10,
}

/// How the columns (groups) are ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ColumnOrder {
    /// By hierarchical clustering of the groups' values.
    Cluster(stamp_core::stats::cluster::Linkage),
    /// By the category of each group in another metadata field, then by
    /// name.
    Category,
    Alphabetical,
    /// As given (the order of the analysis's groups).
    Original,
}

/// How the value written in a cell is formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NumberFormat {
    /// A fixed number of decimals.
    Decimals,
    /// A fixed number of significant digits.
    Significant,
}

/// What is summarised and which features are shown.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Summary {
    pub statistic: GroupStatistic,
    /// Smallest relative abundance (%) at which a feature counts as present,
    /// for prevalence.
    pub min_abundance: f64,
    /// Show only this many features, those of highest mean abundance across
    /// all samples.
    pub max_features: Option<usize>,
}

impl Default for Summary {
    fn default() -> Self {
        Summary {
            statistic: GroupStatistic::Mean,
            min_abundance: 0.0,
            max_features: Some(50),
        }
    }
}

/// The category of each group in a coarser metadata field.
#[derive(Debug, Clone, PartialEq)]
pub struct Categories {
    pub field: String,
    /// Category of each group, in the order of [`Data::groups`].
    pub values: Vec<String>,
}

/// What a group heatmap shows: `values[feature][group]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub features: Vec<String>,
    pub groups: Vec<String>,
    /// Number of samples in each group.
    pub group_sizes: Vec<usize>,
    pub values: Vec<Vec<f64>>,
    /// Mean relative abundance (%) of each feature across all samples.
    pub mean_abundance: Vec<f64>,
    pub statistic: GroupStatistic,
    /// Label of the colour scale.
    pub value_label: String,
    pub categories: Option<Categories>,
    /// Number of features before the cap on their number, if it removed any.
    pub features_before_cap: Option<usize>,
}

impl Data {
    /// Summarise the features of a profile whose sample `i` is in group
    /// `sample_groups[i]` (an index into `groups`). `features` restricts the
    /// rows. Groups without samples are left out.
    pub fn from_profile(
        profile: &Profile,
        sample_groups: &[usize],
        groups: &[String],
        features: Option<&[String]>,
        summary: &Summary,
    ) -> Data {
        let chosen: Vec<&stamp_core::Feature> = match features {
            Some(names) => names.iter().filter_map(|n| profile.feature(n)).collect(),
            None => profile.features().iter().collect(),
        };
        let mut rows: Vec<(String, Vec<f64>, f64)> = chosen
            .iter()
            .map(|f| {
                let values = profile.percentages(f);
                let mean = values.iter().sum::<f64>() / values.len().max(1) as f64;
                (f.name.clone(), values, mean)
            })
            .collect();
        let mut features_before_cap = None;
        if let Some(cap) = summary.max_features {
            if rows.len() > cap {
                features_before_cap = Some(rows.len());
                let mut order: Vec<usize> = (0..rows.len()).collect();
                order.sort_by(|&a, &b| rows[b].2.total_cmp(&rows[a].2).then(a.cmp(&b)));
                let mut keep = vec![false; rows.len()];
                for &i in &order[..cap] {
                    keep[i] = true;
                }
                let mut index = 0;
                rows.retain(|_| {
                    index += 1;
                    keep[index - 1]
                });
            }
        }
        let mut sizes = vec![0usize; groups.len()];
        for &g in sample_groups {
            if let Some(size) = sizes.get_mut(g) {
                *size += 1;
            }
        }
        let kept: Vec<usize> = (0..groups.len()).filter(|&g| sizes[g] > 0).collect();
        let values = rows
            .iter()
            .map(|(_, values, _)| {
                let all = summarise(
                    values,
                    sample_groups,
                    groups.len(),
                    summary.statistic,
                    summary.min_abundance,
                );
                kept.iter().map(|&g| all[g]).collect()
            })
            .collect();
        let mut value_label = summary.statistic.label().to_string();
        if summary.statistic == GroupStatistic::Prevalence && summary.min_abundance > 0.0 {
            value_label = format!(
                "Prevalence (% of samples with \u{2265} {}%)",
                text_number(summary.min_abundance)
            );
        }
        Data {
            features: rows.iter().map(|r| r.0.clone()).collect(),
            groups: kept.iter().map(|&g| groups[g].clone()).collect(),
            group_sizes: kept.iter().map(|&g| sizes[g]).collect(),
            values,
            mean_abundance: rows.iter().map(|r| r.2).collect(),
            statistic: summary.statistic,
            value_label,
            categories: None,
            features_before_cap,
        }
    }

    /// The two groups of a two-group profile.
    pub fn from_group_profile(
        profile: &GroupProfile,
        features: Option<&[String]>,
        summary: &Summary,
    ) -> Data {
        let first = profile.samples_in_group1().len();
        let sample_groups: Vec<usize> = (0..profile.profile.sample_names.len())
            .map(|i| usize::from(i >= first))
            .collect();
        Data::from_profile(
            &profile.profile,
            &sample_groups,
            &[profile.group_name1.clone(), profile.group_name2.clone()],
            features,
            summary,
        )
    }

    /// The groups of a multiple-group profile.
    pub fn from_multi_group_profile(
        profile: &MultiGroupProfile,
        features: Option<&[String]>,
        summary: &Summary,
    ) -> Data {
        let sample_groups: Vec<usize> = profile
            .samples_in_groups()
            .iter()
            .enumerate()
            .flat_map(|(g, members)| std::iter::repeat_n(g, members.len()))
            .collect();
        Data::from_profile(
            &profile.profile,
            &sample_groups,
            &profile.group_names,
            features,
            summary,
        )
    }

    /// Give each group the most common value of a metadata `field` among
    /// its samples. `members` gives the samples of a group by name.
    pub fn set_categories(
        &mut self,
        metadata: &Metadata,
        field: &str,
        members: &dyn Fn(&str) -> Vec<String>,
    ) {
        let lists: Vec<Vec<String>> = self.groups.iter().map(|g| members(g)).collect();
        let slices: Vec<&[String]> = lists.iter().map(Vec::as_slice).collect();
        self.categories = Some(Categories {
            field: field.to_string(),
            values: majority_values(metadata, &slices, field),
        });
    }
}

/// A number as short text: `0.1`, `2`, `0.005`.
fn text_number(value: f64) -> String {
    let text = format!("{value}");
    if text.len() > 8 {
        format_significant(value, 3)
    } else {
        text
    }
}

/// A value with `digits` significant digits, without trailing zeros past
/// those digits: 0.01234 -> "0.0123", 123.4 -> "123".
pub fn format_significant(value: f64, digits: usize) -> String {
    if value == 0.0 || !value.is_finite() {
        return if value.is_nan() {
            String::new()
        } else {
            "0".to_string()
        };
    }
    let digits = digits.max(1) as i32;
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (digits - 1 - magnitude).max(0) as usize;
    format!("{value:.decimals$}")
}

/// Text of a value for a cell.
pub fn format_value(value: f64, format: NumberFormat, digits: usize) -> String {
    if value.is_nan() {
        return String::new();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    match format {
        NumberFormat::Decimals => format!("{value:.digits$}"),
        NumberFormat::Significant => format_significant(value, digits),
    }
}

/// Appearance of the group heatmap. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    /// Size of the figure when it is not fitted to its cells.
    pub width: f64,
    pub height: f64,
    /// Size the figure to give every cell `cell_width` by `cell_height`.
    pub fit_to_cells: bool,
    pub cell_width: f64,
    pub cell_height: f64,
    pub scale: Scale,
    /// Smallest value on a logarithmic scale, in the units of the statistic.
    pub log_floor: f64,
    pub color_map: ColorMap,
    /// Draw cells whose value is zero in grey, apart from the colour scale.
    pub grey_zeros: bool,
    /// Write each cell's value in it.
    pub show_values: bool,
    pub number_format: NumberFormat,
    pub digits: usize,
    pub value_font_size: f64,
    pub row_order: Order,
    pub column_order: ColumnOrder,
    pub show_row_dendrogram: bool,
    pub show_column_dendrogram: bool,
    /// Space given to each dendrogram.
    pub dendrogram_size: f64,
    /// Write the number of samples after each group's name.
    pub show_group_sizes: bool,
    pub show_color_scale: bool,
    pub truncate_labels: Option<usize>,
    /// Size of the names of the features and groups, in points.
    pub label_font_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        use stamp_core::stats::cluster::Linkage;
        Options {
            width: 7.0,
            height: 6.0,
            fit_to_cells: true,
            cell_width: 0.22,
            cell_height: 0.16,
            scale: Scale::Linear,
            log_floor: 0.001,
            color_map: ColorMap::YellowToOrangeToRed,
            grey_zeros: true,
            show_values: false,
            number_format: NumberFormat::Decimals,
            digits: 1,
            value_font_size: 6.0,
            row_order: Order::Cluster(Linkage::Average),
            column_order: ColumnOrder::Cluster(Linkage::Average),
            show_row_dendrogram: true,
            show_column_dendrogram: true,
            dendrogram_size: 0.6,
            show_group_sizes: true,
            show_color_scale: true,
            truncate_labels: Some(50),
            label_font_size: NOTE_SIZE,
        }
    }
}

/// Colour of cells whose value is zero, with `grey_zeros`.
pub const ZERO_COLOR: Color = Color::rgb(0xd0, 0xd0, 0xd0);

/// Black or white, whichever reads better on a background.
pub(crate) fn text_color_on(background: Color) -> Color {
    let luminance = 0.299 * f64::from(background.r)
        + 0.587 * f64::from(background.g)
        + 0.114 * f64::from(background.b);
    if luminance < 140.0 {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

/// A horizontal colour scale: `map` from `low` (left) to `high`, ticks at
/// the given values, and a label above it. Returns its height below `top`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn color_bar(
    figure: &mut Figure,
    left: f64,
    top: f64,
    width: f64,
    color: &dyn Fn(f64) -> Color,
    (low, high): (f64, f64),
    ticks: &[(f64, String)],
    label: &str,
) {
    let label_height = fonts().axis_label * 1.2;
    let bar_top = top + label_height + 2.0;
    let bar_h = 8.0;
    let steps = 80;
    for i in 0..steps {
        let fraction = (i as f64 + 0.5) / steps as f64;
        figure.rect(
            left + width * i as f64 / steps as f64,
            bar_top,
            width / steps as f64 + 0.3,
            bar_h,
            Some(color(fraction)),
            None,
        );
    }
    figure.rect(
        left,
        bar_top,
        width,
        bar_h,
        None,
        Some(Stroke::solid(Color::BLACK, 0.5)),
    );
    let span = if high > low { high - low } else { 1.0 };
    for (value, text) in ticks {
        let x = left + width * (value - low) / span;
        figure.line(
            &[(x, bar_top + bar_h), (x, bar_top + bar_h + 2.5)],
            Stroke::solid(Color::BLACK, 0.5),
        );
        figure.text(
            x,
            bar_top + bar_h + 3.5,
            text.clone(),
            tick_style().align(HAlign::Center, VAlign::Top),
        );
    }
    figure.text(
        left + width / 2.0,
        top,
        label.to_string(),
        axis_label_style().align(HAlign::Center, VAlign::Top),
    );
}

/// Height taken by [`color_bar`].
pub(crate) fn color_bar_height() -> f64 {
    fonts().axis_label * 1.2 + 2.0 + 8.0 + 3.5 + fonts().tick * 1.3
}

/// Ticks for a logarithmic scale between `low` and `high` (logarithms):
/// at powers of ten, or at the two ends if fewer than two powers fall
/// inside.
fn log_ticks(low: f64, high: f64) -> Vec<(f64, String)> {
    let first = (low - 1e-9).ceil() as i32;
    let last = (high + 1e-9).floor() as i32;
    let power = |k: i32| {
        if k >= 0 {
            format!("{}", 10f64.powi(k))
        } else {
            format!("{:.*}", (-k) as usize, 10f64.powi(k))
        }
    };
    let mut ticks: Vec<(f64, String)> = (first..=last).map(|k| (k as f64, power(k))).collect();
    if ticks.len() > 6 {
        let step = ticks.len().div_ceil(6);
        ticks = ticks.into_iter().step_by(step).collect();
    }
    if ticks.len() < 2 {
        ticks = vec![
            (low, format_significant(10f64.powf(low), 2)),
            (high, format_significant(10f64.powf(high), 2)),
        ];
    }
    ticks
}

pub fn plot(data: &Data, options: &Options) -> Figure {
    let (rows, columns) = (data.features.len(), data.groups.len());
    if rows == 0 || columns == 0 {
        return Figure::message("No features or groups to show in the group heatmap");
    }
    if data.values.len() != rows || data.values.iter().any(|r| r.len() != columns) {
        return Figure::message("The values of the group heatmap do not match its features");
    }
    let log = options.scale == Scale::Log10;
    let floor = options.log_floor.max(1e-12);
    let is_zero = |v: f64| v == 0.0 && options.grey_zeros;
    // Value on the colour scale.
    let scaled = |v: f64| {
        if log {
            v.max(floor).log10()
        } else {
            v
        }
    };
    let shown: Vec<f64> = data
        .values
        .iter()
        .flatten()
        .copied()
        .filter(|&v| v.is_finite() && !is_zero(v))
        .map(scaled)
        .collect();
    let (mut low, mut high) = shown
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    if !log {
        // Proportions and prevalences start at zero.
        low = low.min(0.0);
    }
    if !low.is_finite() {
        (low, high) = if log {
            (floor.log10(), 0.0)
        } else {
            (0.0, 1.0)
        };
    }
    if high <= low {
        high = low + if log { 1.0 } else { low.abs().max(1.0) };
    }
    let fraction = |v: f64| (scaled(v) - low) / (high - low);
    let any_zero = data.values.iter().flatten().any(|&v| is_zero(v));

    // Values used for clustering and ordering: zeros at the bottom of the
    // scale.
    let order_value = |v: f64| {
        if v.is_finite() {
            scaled(v)
        } else {
            low
        }
    };
    let row_vectors: Vec<Vec<f64>> = data
        .values
        .iter()
        .map(|r| r.iter().map(|&v| order_value(v)).collect())
        .collect();
    let column_vectors: Vec<Vec<f64>> = (0..columns)
        .map(|c| row_vectors.iter().map(|r| r[c]).collect())
        .collect();
    let (row_order, row_tree) = match options.row_order {
        Order::MeanAbundance => {
            let mut order: Vec<usize> = (0..rows).collect();
            order.sort_by(|&a, &b| {
                data.mean_abundance[b]
                    .total_cmp(&data.mean_abundance[a])
                    .then_with(|| data.features[a].cmp(&data.features[b]))
            });
            (order, None)
        }
        other => arrange(&row_vectors, &data.features, other),
    };
    let (column_order, column_tree) = match options.column_order {
        ColumnOrder::Cluster(linkage) => {
            arrange(&column_vectors, &data.groups, Order::Cluster(linkage))
        }
        ColumnOrder::Alphabetical => arrange(&column_vectors, &data.groups, Order::Alphabetical),
        ColumnOrder::Original => arrange(&column_vectors, &data.groups, Order::Original),
        ColumnOrder::Category => {
            let mut order: Vec<usize> = (0..columns).collect();
            let category = |c: usize| {
                data.categories
                    .as_ref()
                    .and_then(|k| k.values.get(c))
                    .map(String::as_str)
                    .unwrap_or("")
            };
            order.sort_by(|&a, &b| {
                category(a)
                    .cmp(category(b))
                    .then_with(|| data.groups[a].cmp(&data.groups[b]))
            });
            (order, None)
        }
    };
    let row_tree = row_tree.filter(|_| options.show_row_dendrogram && rows > 1);
    let column_tree = column_tree.filter(|_| options.show_column_dendrogram && columns > 1);

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
        .map(|&c| {
            let name = shorten(&data.groups[c]);
            if options.show_group_sizes {
                format!("{name} (n={})", data.group_sizes[c])
            } else {
                name
            }
        })
        .collect();
    let label_size = options.label_font_size;
    let widest = |labels: &[String], size: f64| {
        labels
            .iter()
            .map(|l| text::width(l, size))
            .fold(0.0, f64::max)
    };
    let (row_label_width, column_label_height) = (
        widest(&row_labels, label_size),
        widest(&column_labels, label_size),
    );

    // Categories of the groups, in column order, and their colours.
    let categories: Option<(&str, Vec<&str>)> = data.categories.as_ref().map(|k| {
        (
            k.field.as_str(),
            column_order
                .iter()
                .map(|&c| k.values.get(c).map(String::as_str).unwrap_or(""))
                .collect(),
        )
    });
    let mut category_names: Vec<&str> = categories
        .as_ref()
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    category_names.sort_unstable();
    category_names.dedup();
    let category_color = |name: &str| {
        let index = category_names.iter().position(|n| *n == name).unwrap_or(0);
        group_color(index)
    };

    // Cell texts decide the smallest cell when the figure fits its cells.
    let cell_texts: Vec<Vec<String>> = data
        .values
        .iter()
        .map(|r| {
            r.iter()
                .map(|&v| format_value(v, options.number_format, options.digits))
                .collect()
        })
        .collect();
    let widest_value = if options.show_values {
        cell_texts
            .iter()
            .flatten()
            .map(|t| text::width(t, options.value_font_size))
            .fold(0.0, f64::max)
    } else {
        0.0
    };

    let margin = 0.15 * INCH;
    let dendrogram_space = options.dendrogram_size * INCH;
    let strip = 7.0;
    let category_label_height = label_size * 1.25;
    let left = margin
        + if row_tree.is_some() {
            dendrogram_space
        } else {
            0.0
        };
    let mut top = margin
        + if column_tree.is_some() {
            dendrogram_space
        } else {
            0.0
        };
    if categories.is_some() {
        top += strip + 3.0 + category_label_height;
    }
    let scale_height = if options.show_color_scale {
        color_bar_height() + 0.15 * INCH
    } else {
        0.0
    };
    let note = data
        .features_before_cap
        .map(|total| format!("The {rows} features of highest mean abundance, of {total}",));
    let note_height = if note.is_some() { NOTE_SIZE * 1.6 } else { 0.0 };
    let below_cells = 4.0 + column_label_height + 0.12 * INCH + scale_height + note_height;
    let right_of_cells = 4.0 + row_label_width + margin;

    let (cell_w, cell_h, mut figure) = if options.fit_to_cells {
        let cell_w = (options.cell_width * INCH).max(if options.show_values {
            widest_value + 3.0
        } else {
            0.0
        });
        let cell_h = (options.cell_height * INCH).max(if options.show_values {
            options.value_font_size * 1.15
        } else {
            0.0
        });
        let width = (left + cell_w * columns as f64 + right_of_cells).max(3.5 * INCH);
        let height = top + cell_h * rows as f64 + below_cells + margin;
        (cell_w, cell_h, Figure::new(width, height))
    } else {
        let figure = Figure::with_size_inches(options.width, options.height);
        let (w, h) = (
            figure.width - left - right_of_cells,
            figure.height - top - below_cells - margin,
        );
        if w < 0.5 * INCH || h < 0.5 * INCH {
            return Figure::message(
                "The figure is too small for this heatmap; enlarge it or fit it to its cells",
            );
        }
        (w / columns as f64, h / rows as f64, figure)
    };
    let right = left + cell_w * columns as f64;
    let bottom = top + cell_h * rows as f64;

    // Cells.
    let value_style = TextStyle::new(options.value_font_size).align(HAlign::Center, VAlign::Middle);
    for (r, &row) in row_order.iter().enumerate() {
        for (c, &column) in column_order.iter().enumerate() {
            let value = data.values[row][column];
            let (x, y) = (left + cell_w * c as f64, top + cell_h * r as f64);
            let fill = if !value.is_finite() {
                Color::WHITE
            } else if is_zero(value) {
                ZERO_COLOR
            } else {
                options.color_map.color(fraction(value))
            };
            // Cells overlap slightly so no background shows between them.
            figure.rect(x, y, cell_w + 0.3, cell_h + 0.3, Some(fill), None);
            if options.show_values && value.is_finite() {
                figure.text(
                    x + cell_w / 2.0,
                    y + cell_h / 2.0,
                    cell_texts[row][column].clone(),
                    value_style.color(text_color_on(fill)),
                );
            }
        }
    }
    // Lines between the categories of the groups.
    if let Some((_, values)) = &categories {
        for c in 1..columns {
            if values[c] != values[c - 1] {
                let x = left + cell_w * c as f64;
                figure.line(&[(x, top), (x, bottom)], Stroke::solid(Color::WHITE, 1.5));
            }
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
        for (c, &column) in column_order.iter().enumerate() {
            let value = data.values[row][column];
            if !value.is_finite() {
                continue;
            }
            figure.region(
                left + cell_w * c as f64,
                top + cell_h * r as f64,
                cell_w,
                cell_h,
                &data.features[row],
                format!(
                    "{}\n{} ({} samples)\n{}: {}",
                    data.features[row],
                    data.groups[column],
                    data.group_sizes[column],
                    data.value_label,
                    format_significant(value, 4)
                ),
            );
        }
    }

    // Names of the features and groups, when the cells are large enough.
    let style = TextStyle::new(label_size);
    if cell_h >= label_size * 0.8 {
        for (r, label) in row_labels.iter().enumerate() {
            figure.text(
                right + 4.0,
                top + cell_h * (r as f64 + 0.5),
                label.clone(),
                style.align(HAlign::Left, VAlign::Middle),
            );
        }
    }
    if cell_w >= label_size * 0.8 {
        for (c, label) in column_labels.iter().enumerate() {
            figure.text(
                left + cell_w * (c as f64 + 0.5),
                bottom + 4.0,
                label.clone(),
                style.align(HAlign::Right, VAlign::Middle).rotated(90.0),
            );
        }
    }

    // The strip of categories above the columns, named where there is room.
    let mut unnamed_categories = false;
    if let Some((_, values)) = &categories {
        let strip_top = top - strip - 2.0;
        let mut start = 0;
        while start < columns {
            let mut end = start + 1;
            while end < columns && values[end] == values[start] {
                end += 1;
            }
            let name = values[start];
            let (x0, x1) = (left + cell_w * start as f64, left + cell_w * end as f64);
            figure.rect(
                x0,
                strip_top,
                x1 - x0,
                strip,
                Some(category_color(name)),
                Some(Stroke::solid(Color::WHITE, 0.75)),
            );
            figure.region(x0, strip_top, x1 - x0, strip, name, name.to_string());
            let shown = if name.is_empty() { "(none)" } else { name };
            if text::width(shown, label_size) <= x1 - x0 + 2.0 {
                figure.text(
                    (x0 + x1) / 2.0,
                    strip_top - 2.0,
                    shown.to_string(),
                    style.align(HAlign::Center, VAlign::Bottom),
                );
            } else {
                unnamed_categories = true;
            }
            start = end;
        }
    }

    if let Some(tree) = &row_tree {
        let max = tree.max_height().max(f64::MIN_POSITIVE);
        let leaf = |position: usize| top + cell_h * (position as f64 + 0.5);
        let base = left - 2.0;
        let across = |h: f64| base - (base - margin) * h / max;
        dendrogram(&mut figure, tree, &leaf, &across, false);
    }
    if let Some(tree) = &column_tree {
        let max = tree.max_height().max(f64::MIN_POSITIVE);
        let base = if categories.is_some() {
            top - strip - 5.0 - category_label_height
        } else {
            top - 3.0
        };
        let leaf = |position: usize| left + cell_w * (position as f64 + 0.5);
        let across = |h: f64| base - (base - margin) * h / max;
        dendrogram(&mut figure, tree, &leaf, &across, true);
    }

    // Colour scale, with a grey swatch for zero, below the names of the
    // groups; then the categories that could not be named above the strip.
    let mut y = bottom + 4.0 + column_label_height + 0.12 * INCH;
    if options.show_color_scale {
        let mut x = margin;
        if any_zero {
            let swatch_top = y + fonts().axis_label * 1.2 + 2.0;
            figure.rect(
                x,
                swatch_top,
                10.0,
                8.0,
                Some(ZERO_COLOR),
                Some(Stroke::solid(Color::BLACK, 0.5)),
            );
            figure.text(
                x + 5.0,
                swatch_top + 8.0 + 3.5,
                "0",
                tick_style().align(HAlign::Center, VAlign::Top),
            );
            x += 18.0;
        }
        let bar_width = (2.0 * INCH).min(figure.width - x - margin - 6.0);
        let ticks: Vec<(f64, String)> = if log {
            log_ticks(low, high)
        } else {
            let ticks = Ticks::inside(low, high);
            ticks
                .positions
                .iter()
                .copied()
                .zip(ticks.labels())
                .collect()
        };
        let label = if log {
            format!("{} (log scale)", data.value_label)
        } else {
            data.value_label.clone()
        };
        let map = options.color_map;
        color_bar(
            &mut figure,
            x + 6.0,
            y,
            bar_width,
            &|f| map.color(f),
            (low, high),
            &ticks,
            &label,
        );
        y += scale_height;
    }
    if let Some(note) = note {
        figure.text(
            margin,
            y,
            note,
            TextStyle::new(NOTE_SIZE).align(HAlign::Left, VAlign::Top),
        );
        y += note_height;
    }
    if unnamed_categories {
        // A wrapped list of the categories and their colours, which needs
        // room the figure grows to give.
        let (field, _) = categories.as_ref().expect("categories exist");
        let size = fonts().legend;
        let line = size + 5.0;
        let mut entries: Vec<(String, Color)> = vec![(format!("{field}:"), Color::WHITE)];
        entries.extend(category_names.iter().map(|name| {
            let shown = if name.is_empty() { "(none)" } else { name };
            (shown.to_string(), category_color(name))
        }));
        let mut x = margin;
        let mut line_top = y;
        for (i, (name, color)) in entries.iter().enumerate() {
            let swatch = if i == 0 { 0.0 } else { 14.0 };
            let width = swatch + text::width(name, size) + 12.0;
            if x + width > figure.width - margin && x > margin {
                x = margin;
                line_top += line;
            }
            if i > 0 {
                figure.rect(
                    x,
                    line_top + line / 2.0 - 3.0,
                    10.0,
                    6.0,
                    Some(*color),
                    Some(Stroke::solid(Color::BLACK, 0.5)),
                );
            }
            figure.text(
                x + swatch,
                line_top + line / 2.0,
                name.clone(),
                TextStyle::new(size).align(HAlign::Left, VAlign::Middle),
            );
            x += width;
        }
        let needed = line_top + line + margin;
        if needed > figure.height {
            figure.height = needed;
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
            features: vec!["zeta".into(), "alpha".into(), "mid".into()],
            groups: vec!["G1".into(), "G2".into(), "G3".into(), "G4".into()],
            group_sizes: vec![3, 4, 2, 5],
            values: vec![
                vec![1.0, 9.0, 1.2, 9.5],
                vec![0.0, 5.1, 5.0, 0.02],
                vec![1.1, 8.0, 1.0, 9.0],
            ],
            mean_abundance: vec![5.0, 2.5, 4.0],
            statistic: GroupStatistic::Mean,
            value_label: "Mean proportion (%)".into(),
            categories: None,
            features_before_cap: None,
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

    /// Feature of each row, top to bottom, and group of each column.
    fn order(figure: &Figure) -> (Vec<String>, Vec<String>) {
        let mut rows: Vec<(f64, String)> = Vec::new();
        let mut columns: Vec<(f64, String)> = Vec::new();
        for region in figure.regions.iter().filter(|r| r.tooltip.contains('\n')) {
            let mut lines = region.tooltip.lines();
            let feature = lines.next().unwrap_or("").to_string();
            let group = lines.next().unwrap_or("").split(" (").next().unwrap_or("");
            if !rows.iter().any(|(_, f)| *f == feature) {
                rows.push((region.y, feature));
            }
            if !columns.iter().any(|(_, g)| g == group) {
                columns.push((region.x, group.to_string()));
            }
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        columns.sort_by(|a, b| a.0.total_cmp(&b.0));
        (
            rows.into_iter().map(|r| r.1).collect(),
            columns.into_iter().map(|c| c.1).collect(),
        )
    }

    fn fills(figure: &Figure) -> Vec<Color> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Rect { fill: Some(c), .. } => Some(*c),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn summaries_from_a_profile() {
        let tree = stamp_core::ProfileTree::read(
            "F\ta1\ta2\tb1\tb2\tb3\n x\t0\t10\t5\t0\t5\n y\t10\t10\t5\t10\t5\n z\t0\t0\t0\t0\t90\n"
                .replace("\n ", "\n")
                .as_str(),
        )
        .unwrap();
        let samples = ["a1", "a2", "b1", "b2", "b3"];
        let profile = tree
            .profile(&samples, None, "F", stamp_core::Unclassified::Retain)
            .unwrap();
        let groups = vec!["A".to_string(), "B".to_string(), "Empty".to_string()];
        let summary = Summary {
            statistic: GroupStatistic::Prevalence,
            min_abundance: 0.0,
            max_features: None,
        };
        let data = Data::from_profile(&profile, &[0, 0, 1, 1, 1], &groups, None, &summary);
        // The empty group is left out.
        assert_eq!(data.groups, ["A", "B"]);
        assert_eq!(data.group_sizes, [2, 3]);
        assert_eq!(data.features, ["x", "y", "z"]);
        let third = 100.0 / 3.0;
        assert_eq!(data.values[0], [50.0, 2.0 * third]);
        assert_eq!(data.values[2], [0.0, third]);
        // Capped to the two of highest mean: y (61%) and x (21%), not z (18%).
        let capped = Summary {
            statistic: GroupStatistic::Maximum,
            max_features: Some(2),
            ..summary
        };
        let data = Data::from_profile(&profile, &[0, 0, 1, 1, 1], &groups, None, &capped);
        assert_eq!(data.features, ["x", "y"]);
        assert_eq!(data.features_before_cap, Some(3));
        assert_eq!(data.values[0], [50.0, 50.0]);
        assert_eq!(data.values[1], [100.0, 100.0]);
        let figure = plot(&data, &Options::default());
        assert!(texts(&figure)
            .iter()
            .any(|t| t.starts_with("The 2 features of highest mean abundance, of 3")));
        // Restricted to named features.
        let names = vec!["x".to_string()];
        let data = Data::from_profile(&profile, &[0, 0, 1, 1, 1], &groups, Some(&names), &summary);
        assert_eq!(data.features, ["x"]);
        assert_eq!(plot(&data, &Options::default()).regions.len(), 2);
    }

    #[test]
    fn rows_and_columns_are_ordered() {
        let plain = Options {
            row_order: Order::Original,
            column_order: ColumnOrder::Original,
            ..Options::default()
        };
        let (rows, columns) = order(&plot(&data(), &plain));
        assert_eq!(rows, ["zeta", "alpha", "mid"]);
        assert_eq!(columns, ["G1", "G2", "G3", "G4"]);
        let by_mean = Options {
            row_order: Order::MeanAbundance,
            column_order: ColumnOrder::Alphabetical,
            ..plain.clone()
        };
        assert_eq!(order(&plot(&data(), &by_mean)).0, ["zeta", "mid", "alpha"]);
        // Clustering puts zeta beside mid, and G2 beside G4.
        let (rows, columns) = order(&plot(&data(), &Options::default()));
        let at = |list: &[String], name: &str| list.iter().position(|n| n == name).unwrap();
        assert_eq!(at(&rows, "zeta").abs_diff(at(&rows, "mid")), 1);
        assert_eq!(at(&columns, "G2").abs_diff(at(&columns, "G4")), 1);
        // By category: Primates (G3, G4) before Rodents (G1, G2).
        let mut data = data();
        data.categories = Some(Categories {
            field: "Order".into(),
            values: vec![
                "Rodents".into(),
                "Rodents".into(),
                "Primates".into(),
                "Primates".into(),
            ],
        });
        let by_category = Options {
            column_order: ColumnOrder::Category,
            ..plain
        };
        let figure = plot(&data, &by_category);
        assert_eq!(order(&figure).1, ["G3", "G4", "G1", "G2"]);
        let texts = texts(&figure);
        assert!(texts.contains(&"Rodents".to_string()));
        assert!(texts.contains(&"G1 (n=3)".to_string()));
    }

    #[test]
    fn zeros_log_scale_and_values() {
        let options = Options::default();
        let figure = plot(&data(), &options);
        // One zero cell, drawn grey, and the grey swatch of the scale.
        assert_eq!(
            fills(&figure).iter().filter(|&&c| c == ZERO_COLOR).count(),
            2
        );
        let no_grey = Options {
            grey_zeros: false,
            ..options.clone()
        };
        assert!(!fills(&plot(&data(), &no_grey)).contains(&ZERO_COLOR));
        // Log scale: ticks at powers of ten within 0.02 .. 9.5.
        let log = Options {
            scale: Scale::Log10,
            ..options.clone()
        };
        let texts_log = texts(&plot(&data(), &log));
        for tick in ["0.1", "1"] {
            assert!(texts_log.contains(&tick.to_string()), "{texts_log:?}");
        }
        assert!(texts_log.contains(&"Mean proportion (%) (log scale)".to_string()));
        // Values written in the cells.
        let values = Options {
            show_values: true,
            digits: 2,
            ..options.clone()
        };
        let shown = texts(&plot(&data(), &values));
        assert!(shown.contains(&"9.50".to_string()) && shown.contains(&"0".to_string()));
        let significant = Options {
            show_values: true,
            number_format: NumberFormat::Significant,
            digits: 2,
            ..options
        };
        let shown = texts(&plot(&data(), &significant));
        assert!(shown.contains(&"0.020".to_string()) && shown.contains(&"9.5".to_string()));
    }

    #[test]
    fn fitted_and_fixed_sizes() {
        let fitted = plot(&data(), &Options::default());
        let mut wide = data();
        for row in &mut wide.values {
            row.extend(vec![1.0; 96]);
        }
        wide.groups.extend((0..96).map(|i| format!("extra{i}")));
        wide.group_sizes.extend(vec![1; 96]);
        let wider = plot(&wide, &Options::default());
        assert!((wider.width - fitted.width.max(0.0)) > 90.0 * 0.2 * INCH);
        let fixed = Options {
            fit_to_cells: false,
            width: 5.0,
            height: 4.0,
            ..Options::default()
        };
        let figure = plot(&wide, &fixed);
        assert_eq!((figure.width, figure.height), (360.0, 288.0));
        let tiny = Options {
            width: 1.0,
            ..fixed
        };
        assert!(texts(&plot(&data(), &tiny))[0].contains("too small"));
        let mut empty = data();
        empty.features.clear();
        empty.values.clear();
        assert!(texts(&plot(&empty, &Options::default()))[0].contains("No features"));
    }

    #[test]
    fn formats() {
        assert_eq!(format_significant(0.012345, 3), "0.0123");
        assert_eq!(format_significant(123.4, 2), "123");
        assert_eq!(format_significant(9.96, 2), "10.0");
        assert_eq!(format_value(f64::NAN, NumberFormat::Decimals, 1), "");
        assert_eq!(format_value(2.25, NumberFormat::Decimals, 1), "2.2");
        assert_eq!(log_ticks(-2.0, 1.0).len(), 4);
        assert_eq!(log_ticks(-2.0, 1.0)[0].1, "0.01");
        assert_eq!(log_ticks(0.1, 0.5).len(), 2);
        assert_eq!(text_color_on(Color::BLACK), Color::WHITE);
        assert_eq!(text_color_on(Color::WHITE), Color::BLACK);
    }
}
