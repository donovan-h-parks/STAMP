//! Box plots of one or several features across groups: one panel per
//! feature (small multiples) or the features side by side with a box per
//! group at each, on a linear or logarithmic value axis, upright or lying
//! down, optionally with every sample drawn as a point.
//!
//! The statistics of a box never depend on the axis: the quartiles, the
//! median and the whiskers (the most extreme samples within 1.5
//! interquartile ranges of the box, Tukey's rule) are computed on the
//! values themselves and then placed on the axis. On a logarithmic axis
//! zeros, which have no logarithm, are drawn on a line labelled 0 below a
//! break in the axis.

use crate::axes::{
    fonts, line_height, x_axis, y_axis, Frame, LabelRotation, AXES_COLOR, INCH, TICK_PAD,
};
use crate::axis::{self, LinearScale, Ticks};
use crate::feature_plots::{
    box_stats, shorten, star, BoxLayout, BoxOptions, FeatureData, GroupOrder, NOTHING,
};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::group_heatmap::format_significant;
use crate::text;

/// A value axis, linear or logarithmic, in the units it is drawn in (the
/// logarithm of the value on a logarithmic axis).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValueAxis {
    pub min: f64,
    pub max: f64,
    pub log: bool,
    /// Where zeros are drawn on a logarithmic axis that has any.
    pub zero: Option<f64>,
    /// Where the axis is broken between the zero line and the powers of
    /// ten.
    pub break_at: Option<f64>,
    /// Positions (in axis units) and labels of the ticks.
    pub ticks: Vec<(f64, String)>,
}

impl ValueAxis {
    /// An axis covering `values`; `None` if none is finite. A linear axis
    /// ends on round numbers, as STAMP v2's did; a logarithmic one on
    /// powers of ten.
    pub fn new(values: impl IntoIterator<Item = f64>, log: bool) -> Option<ValueAxis> {
        let values: Vec<f64> = values.into_iter().filter(|v| v.is_finite()).collect();
        if values.is_empty() {
            return None;
        }
        if !log {
            let (low, high) = values
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                    (lo.min(v), hi.max(v))
                });
            let ticks = Ticks::covering(low, high);
            return Some(ValueAxis {
                min: ticks.min,
                max: ticks.max,
                log: false,
                zero: None,
                break_at: None,
                ticks: ticks
                    .positions
                    .iter()
                    .copied()
                    .zip(ticks.labels())
                    .collect(),
            });
        }
        let positive: Vec<f64> = values.iter().copied().filter(|&v| v > 0.0).collect();
        let has_zero = positive.len() < values.len();
        let (low, high) = if positive.is_empty() {
            (-1, 0)
        } else {
            let smallest = positive.iter().copied().fold(f64::INFINITY, f64::min);
            let largest = positive.iter().copied().fold(0.0, f64::max);
            let low = (smallest.log10() + 1e-9).floor() as i32;
            let high = (largest.log10() - 1e-9).ceil() as i32;
            (low, high.max(low + 1))
        };
        let mut ticks = axis::log_ticks(low, high);
        let (mut min, mut zero, mut break_at) = (low as f64, None, None);
        if has_zero {
            let gap = (0.15 * (high - low) as f64).max(0.6);
            let z = low as f64 - gap;
            zero = Some(z);
            break_at = Some(low as f64 - gap / 2.0);
            min = z - 0.4 * gap;
            ticks.insert(0, (z, "0".to_string()));
        }
        Some(ValueAxis {
            min,
            max: high as f64,
            log: true,
            zero,
            break_at,
            ticks,
        })
    }

    /// The same axis with labels thinned out so that they fit, side by
    /// side, along `length` points (a logarithmic axis only).
    pub fn fitted(&self, length: f64) -> ValueAxis {
        if !self.log {
            return self.clone();
        }
        let low = self.ticks_min().round() as i32;
        let high = self.max.round() as i32;
        let widest = self.label_width() + 8.0;
        let room = (length / widest).floor().max(2.0) as usize;
        let mut ticks = axis::log_ticks_at_most(low, high, room);
        if let Some(zero) = self.zero {
            ticks.insert(0, (zero, "0".to_string()));
        }
        ValueAxis {
            ticks,
            ..self.clone()
        }
    }

    /// The lowest power of ten (or other value) with a tick, in axis
    /// units, leaving out the zero line.
    pub fn ticks_min(&self) -> f64 {
        self.ticks
            .iter()
            .map(|t| t.0)
            .filter(|&t| Some(t) != self.zero)
            .fold(f64::INFINITY, f64::min)
    }

    /// Where a value goes, in axis units.
    pub fn coord(&self, value: f64) -> f64 {
        if !self.log {
            value
        } else if value > 0.0 {
            value.log10()
        } else {
            self.zero.unwrap_or(self.min)
        }
    }

    /// Widest tick label, in points.
    pub fn label_width(&self) -> f64 {
        self.ticks
            .iter()
            .map(|(_, label)| text::width(label, fonts().tick))
            .fold(0.0, f64::max)
    }

    /// The ticks placed by `scale`.
    pub fn scaled(&self, scale: &LinearScale) -> Vec<(f64, String)> {
        self.ticks
            .iter()
            .map(|(position, label)| (scale.map(*position), label.clone()))
            .collect()
    }

    /// Mark the break of a logarithmic axis with zeros: a gap with two
    /// slanted strokes on the axis line, and a faint dashed line across
    /// the frame separating the zeros from the rest.
    pub fn draw_break(
        &self,
        figure: &mut Figure,
        frame: &Frame,
        scale: &LinearScale,
        horizontal: bool,
    ) {
        let Some(at) = self.break_at else { return };
        let p = scale.map(at);
        let stroke = Stroke::solid(AXES_COLOR, 1.0);
        let faint = Stroke::dashed(Color::grey(0.75), 0.6, 2.0, 2.0);
        if horizontal {
            let y = frame.bottom;
            figure.line(&[(p, frame.top), (p, frame.bottom)], faint);
            figure.rect(p - 2.0, y - 4.0, 4.0, 8.0, Some(Color::WHITE), None);
            figure.line(&[(p - 3.5, y + 4.0), (p - 0.5, y - 4.0)], stroke);
            figure.line(&[(p + 0.5, y + 4.0), (p + 3.5, y - 4.0)], stroke);
        } else {
            let x = frame.left;
            figure.line(&[(frame.left, p), (frame.right, p)], faint);
            figure.rect(x - 4.0, p - 2.0, 8.0, 4.0, Some(Color::WHITE), None);
            figure.line(&[(x - 4.0, p + 3.5), (x + 4.0, p + 0.5)], stroke);
            figure.line(&[(x - 4.0, p - 0.5), (x + 4.0, p - 3.5)], stroke);
        }
    }
}

/// The order in which the groups are drawn, as indices into the groups of
/// the features. `categories` holds the category of each group, for
/// [`GroupOrder::Category`] (which falls back to the given order without
/// it).
pub fn group_order(
    features: &[FeatureData],
    categories: Option<&[String]>,
    order: GroupOrder,
) -> Vec<usize> {
    let Some(first) = features.first() else {
        return Vec::new();
    };
    let groups = &first.groups;
    let mut indices: Vec<usize> = (0..groups.len()).collect();
    let name = |g: usize| groups[g].name.to_lowercase();
    match order {
        GroupOrder::AsGiven => {}
        GroupOrder::Alphabetical => indices.sort_by_key(|&g| (name(g), g)),
        GroupOrder::Median => {
            let summary: Vec<(f64, f64)> = groups
                .iter()
                .map(|g| match box_stats(&g.values) {
                    Some(s) => (
                        s.median,
                        g.values.iter().sum::<f64>() / g.values.len() as f64,
                    ),
                    None => (f64::NEG_INFINITY, f64::NEG_INFINITY),
                })
                .collect();
            indices.sort_by(|&a, &b| {
                summary[b]
                    .0
                    .total_cmp(&summary[a].0)
                    .then(summary[b].1.total_cmp(&summary[a].1))
                    .then(name(a).cmp(&name(b)))
            });
        }
        GroupOrder::Category => {
            if let Some(categories) = categories.filter(|c| c.len() == groups.len()) {
                indices.sort_by(|&a, &b| {
                    categories[a]
                        .cmp(&categories[b])
                        .then(name(a).cmp(&name(b)))
                });
            }
        }
    }
    indices
}

/// Box plot of `features` (each with the same groups) across the groups.
/// One feature is drawn as STAMP's box plot always was; several follow
/// [`BoxOptions::layout`]. `categories` gives each group's category in a
/// coarser metadata field, for [`GroupOrder::Category`].
pub fn plot(
    features: &[FeatureData],
    categories: Option<&[String]>,
    options: &BoxOptions,
) -> Figure {
    if features.is_empty() || features.iter().any(FeatureData::is_empty) {
        return Figure::message(NOTHING);
    }
    let groups = features[0].groups.len();
    if features.iter().any(|f| f.groups.len() != groups) {
        return Figure::message("The features of the box plot have different groups");
    }
    let order = group_order(features, categories, options.group_order);
    let labels: Vec<String> = order
        .iter()
        .map(|&g| {
            let group = &features[0].groups[g];
            let name = shorten(&group.name, options.truncate_labels);
            if options.show_counts {
                format!("{name} (n={})", group.values.len())
            } else {
                name
            }
        })
        .collect();
    if features.len() == 1 || options.layout == BoxLayout::Panels {
        // Lines between the categories of groups ordered by category.
        let breaks: Vec<usize> = match categories {
            Some(c) if options.group_order == GroupOrder::Category && c.len() == groups => (1
                ..order.len())
                .filter(|&k| c[order[k]] != c[order[k - 1]])
                .collect(),
            _ => Vec::new(),
        };
        panels(features, &order, &labels, &breaks, options)
    } else {
        grouped(features, &order, &labels, options)
    }
}

/// What the value axis of a box plot must cover for a feature: its values,
/// and the quartiles of each group, which on a logarithmic axis can lie
/// below the smallest value above zero (a quartile between a zero and a
/// small value). Means are not included: the mean of a group of mostly
/// zeros can be orders of magnitude below every value, and a mean below
/// the axis is left out of the drawing (its tooltip still gives it).
fn drawn_values(feature: &FeatureData) -> Vec<f64> {
    let mut values = Vec::new();
    for group in &feature.groups {
        values.extend(group.values.iter().copied());
        if let Some(s) = box_stats(&group.values) {
            values.extend([s.q1, s.median, s.q3]);
        }
    }
    values
}

pub(crate) use crate::p_values::p_value_text as p_text;

/// How category labels fit along an axis with slots of width `slot`:
/// level if the widest fits, otherwise at 45 degrees; and the depth they
/// take below the axis.
fn category_labels(widest: f64, slot: f64) -> (LabelRotation, f64) {
    let line = line_height(fonts().tick);
    if widest <= slot * 0.95 {
        (LabelRotation::Horizontal, TICK_PAD + line)
    } else {
        (
            LabelRotation::Diagonal,
            TICK_PAD + (widest + line) * std::f64::consts::FRAC_1_SQRT_2,
        )
    }
}

fn widest(labels: &[String], size: f64) -> f64 {
    labels
        .iter()
        .map(|l| text::width(l, size))
        .fold(0.0, f64::max)
}

/// Room for the name of a value axis beside or below it.
fn axis_label_room() -> f64 {
    4.0 + line_height(fonts().axis_label)
}

/// A value as text: three decimals, or three significant digits for small
/// values (as on a logarithmic axis).
fn number(value: f64) -> String {
    if value != 0.0 && value.abs() < 0.1 {
        format_significant(value, 3)
    } else {
        format!("{value:.3}")
    }
}

/// One panel per feature, the groups along the category axis of each.
fn panels(
    features: &[FeatureData],
    order: &[usize],
    labels: &[String],
    breaks: &[usize],
    options: &BoxOptions,
) -> Figure {
    let n = features.len();
    let multi = n > 1;
    // Panels about as wide as they are high.
    let columns = if multi {
        ((n as f64 * options.width / options.height.max(0.1))
            .sqrt()
            .ceil() as usize)
            .clamp(1, n)
    } else {
        1
    };
    let rows = n.div_ceil(columns);
    let axes: Vec<ValueAxis> = if options.shared_axis || !multi {
        match ValueAxis::new(features.iter().flat_map(drawn_values), options.log_scale) {
            Some(axis) => vec![axis; n],
            None => return Figure::message(NOTHING),
        }
    } else {
        let mut axes = Vec::new();
        for feature in features {
            match ValueAxis::new(drawn_values(feature), options.log_scale) {
                Some(axis) => axes.push(axis),
                None => return Figure::message(NOTHING),
            }
        }
        axes
    };
    let sizes = fonts();
    let padding = 0.25 * INCH;
    let tick_width = axes.iter().map(ValueAxis::label_width).fold(0.0, f64::max);
    let group_width = widest(labels, sizes.tick);
    let has_p = options.show_p_value && features.iter().any(|f| f.p_value.is_some());
    let title_room = if multi {
        options.title_font_size * 1.3
            + if has_p {
                options.title_font_size * 1.25
            } else {
                0.0
            }
            + 6.0
    } else {
        2.0 * padding
    };
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let gap = if multi { 0.12 * INCH } else { 0.0 };

    // Room left of each column of panels.
    let (left_rooms, frames): (Vec<f64>, Vec<Frame>);
    let rotation;
    if !options.horizontal {
        left_rooms = (0..columns)
            .map(|c| TICK_PAD + tick_width + if c == 0 { axis_label_room() } else { 4.0 })
            .collect();
        let panel_width = (figure.width
            - 3.0 * padding
            - left_rooms.iter().sum::<f64>()
            - gap * (columns - 1) as f64)
            / columns as f64;
        let (turn, depth) = category_labels(group_width, panel_width / order.len() as f64);
        rotation = turn;
        let bottom = depth.max(0.3 * INCH);
        let panel_height = (figure.height - padding) / rows as f64 - title_room - bottom;
        if panel_width < 0.6 * INCH || panel_height < 0.6 * INCH {
            return Figure::message("The figure is too small for a box plot");
        }
        frames = (0..n)
            .map(|i| {
                let (r, c) = (i / columns, i % columns);
                let left =
                    padding + left_rooms[..=c].iter().sum::<f64>() + c as f64 * (panel_width + gap);
                let top = r as f64 * (title_room + panel_height + bottom) + title_room;
                Frame {
                    left,
                    top,
                    right: left + panel_width,
                    bottom: top + panel_height,
                }
            })
            .collect();
    } else {
        rotation = LabelRotation::Horizontal;
        left_rooms = vec![TICK_PAD + group_width + 4.0; columns];
        let bottom = TICK_PAD + line_height(sizes.tick) + axis_label_room();
        let panel_width = (figure.width
            - 3.0 * padding
            - left_rooms.iter().sum::<f64>()
            - gap * (columns - 1) as f64)
            / columns as f64;
        let panel_height = (figure.height - padding) / rows as f64 - title_room - bottom;
        if panel_width < 0.6 * INCH || panel_height < 0.6 * INCH {
            return Figure::message("The figure is too small for a box plot");
        }
        frames = (0..n)
            .map(|i| {
                let (r, c) = (i / columns, i % columns);
                let left =
                    padding + left_rooms[..=c].iter().sum::<f64>() + c as f64 * (panel_width + gap);
                let top = r as f64 * (title_room + panel_height + bottom) + title_room;
                Frame {
                    left,
                    top,
                    right: left + panel_width,
                    bottom: top + panel_height,
                }
            })
            .collect();
    }

    for (i, ((feature, frame), axis)) in features.iter().zip(&frames).zip(&axes).enumerate() {
        let column = i % columns;
        // The value axis is named once per row (upright) or column (lying).
        let name_axis = if options.horizontal {
            i + columns >= n
        } else {
            column == 0
        };
        let (category_start, category_end) = if options.horizontal {
            (frame.top, frame.bottom)
        } else {
            (frame.left, frame.right)
        };
        let categories =
            LinearScale::new(0.5, order.len() as f64 + 0.5, category_start, category_end);
        let values = if options.horizontal {
            frame.x_scale(axis.min, axis.max)
        } else {
            frame.y_scale(axis.min, axis.max)
        };
        let fitted = if options.horizontal {
            axis.fitted(frame.width())
        } else {
            axis.clone()
        };
        let axis = &fitted;
        let half = (category_end - category_start) / order.len() as f64 * 0.25;
        let between = Stroke::dashed(Color::grey(0.6), 0.75, 3.0, 2.0);
        for &k in breaks {
            let at = categories.map(k as f64 + 0.5);
            let points = if options.horizontal {
                [(frame.left, at), (frame.right, at)]
            } else {
                [(at, frame.top), (at, frame.bottom)]
            };
            figure.line(&points, between);
        }
        let mut ticks = Vec::new();
        for (k, &g) in order.iter().enumerate() {
            let centre = categories.map(k as f64 + 1.0);
            ticks.push((centre, labels[k].clone()));
            let group = &feature.groups[g];
            draw_box(
                &mut figure,
                &BoxPlace {
                    centre,
                    half,
                    frame: *frame,
                    axis,
                    scale: &values,
                },
                group.color,
                &group.values,
                &tooltip_title(multi.then_some(&feature.feature), &group.name),
                (i * 7919 + g) as u64,
                options,
            );
        }
        // Upright panels in several rows share one name for their value
        // axes, centred down the figure, drawn below.
        let one_name = multi && !options.horizontal && rows > 1;
        let label = (name_axis && !one_name)
            .then(|| crate::axes::scale_label(feature.axis_label(), options.log_scale));
        let label = label.as_deref();
        if options.horizontal {
            y_axis(&mut figure, frame, &ticks, None, AXES_COLOR);
            x_axis(
                &mut figure,
                frame,
                &axis.scaled(&values),
                label,
                LabelRotation::Horizontal,
                AXES_COLOR,
            );
        } else {
            x_axis(&mut figure, frame, &ticks, None, rotation, AXES_COLOR);
            y_axis(&mut figure, frame, &axis.scaled(&values), label, AXES_COLOR);
        }
        axis.draw_break(&mut figure, frame, &values, options.horizontal);

        let title = shorten(&feature.feature, options.truncate_labels);
        let p_value = feature
            .p_value
            .as_ref()
            .filter(|_| options.show_p_value)
            .map(|p| p_text(p, feature.p_value_kind));
        let size = options.title_font_size;
        let centre = (frame.left + frame.right) / 2.0;
        let style = TextStyle::new(size).align(HAlign::Center, VAlign::Baseline);
        if multi {
            match &p_value {
                Some(p) => {
                    figure.text(centre, frame.top - 6.0 - size * 1.25, title, style);
                    figure.text(centre, frame.top - 6.0, p.clone(), style);
                }
                None => figure.text(centre, frame.top - 6.0, title, style),
            }
        } else {
            crate::axes::title(&mut figure, frame, &title, size);
            if let Some(p) = p_value {
                crate::axes::corner_note(&mut figure, frame, &p, size);
            }
        }
    }
    if multi && !options.horizontal && rows > 1 {
        let (top, bottom) = (frames[0].top, frames[n - 1].bottom);
        figure.text(
            padding,
            (top + bottom) / 2.0,
            crate::axes::scale_label(features[0].axis_label(), options.log_scale),
            crate::axes::axis_label_style()
                .align(HAlign::Center, VAlign::Top)
                .rotated(90.0),
        );
    }
    figure
}

/// The features along the category axis, a box per group at each.
fn grouped(
    features: &[FeatureData],
    order: &[usize],
    labels: &[String],
    options: &BoxOptions,
) -> Figure {
    let all = features.iter().flat_map(drawn_values);
    let Some(axis) = ValueAxis::new(all, options.log_scale) else {
        return Figure::message(NOTHING);
    };
    let sizes = fonts();
    let padding = 0.25 * INCH;
    let names: Vec<String> = features
        .iter()
        .map(|f| shorten(&f.feature, options.truncate_labels))
        .collect();
    // Side by side, the slots are too narrow for "Corrected p-value = ..."
    // above each feature: the name of the values is written once above
    // them, and each slot shows its value. Beside the features (horizontal)
    // there is room for the whole text.
    let kinds: Vec<_> = features.iter().filter_map(|f| f.p_value_kind).collect();
    let shared_kind = (!options.horizontal
        && !kinds.is_empty()
        && kinds.len() == features.len()
        && kinds.iter().all(|k| *k == kinds[0]))
    .then(|| kinds[0]);
    let p_values: Vec<Option<String>> = features
        .iter()
        .map(|f| {
            f.p_value
                .as_ref()
                .filter(|_| options.show_p_value)
                .map(|p| match shared_kind {
                    Some(_) => p.clone(),
                    None => p_text(p, f.p_value_kind),
                })
        })
        .collect();
    let has_p = p_values.iter().any(Option::is_some);
    let p_heading = shared_kind.filter(|_| has_p).map(|kind| kind.plural());
    let p_size = options.title_font_size;
    let p_width = p_values
        .iter()
        .flatten()
        .map(|p| text::width(p, p_size))
        .fold(0.0, f64::max);
    // The legend of the groups, right of the frame.
    let swatch = 14.0;
    let legend_width = swatch + 5.0 + widest(labels, sizes.legend);
    let line = sizes.legend + 4.0;
    let mut figure = Figure::with_size_inches(options.width, options.height);
    let right = figure.width - padding - legend_width - 10.0;
    let frame = if options.horizontal {
        let left = padding + widest(&names, sizes.tick) + TICK_PAD + 4.0;
        let right = if has_p { right - p_width - 10.0 } else { right };
        Frame {
            left,
            top: padding,
            right,
            bottom: figure.height
                - padding
                - TICK_PAD
                - line_height(sizes.tick)
                - axis_label_room(),
        }
    } else {
        let left = padding + TICK_PAD + axis.label_width() + axis_label_room();
        let (_, depth) = category_labels(
            widest(&names, sizes.tick),
            (right - left) / features.len() as f64,
        );
        let top = padding
            + if has_p { p_size * 1.5 } else { 0.0 }
            + if p_heading.is_some() {
                p_size * 1.3
            } else {
                0.0
            };
        Frame {
            left,
            top,
            right,
            bottom: figure.height - padding - depth.max(0.3 * INCH),
        }
    };
    if frame.width() < INCH || frame.height() < INCH {
        return Figure::message("The figure is too small for a box plot");
    }
    let (category_start, category_end) = if options.horizontal {
        (frame.top, frame.bottom)
    } else {
        (frame.left, frame.right)
    };
    let k = features.len();
    let categories = LinearScale::new(0.5, k as f64 + 0.5, category_start, category_end);
    let values = if options.horizontal {
        frame.x_scale(axis.min, axis.max)
    } else {
        frame.y_scale(axis.min, axis.max)
    };
    let axis = if options.horizontal {
        axis.fitted(frame.width())
    } else {
        axis
    };
    if let Some(heading) = p_heading {
        figure.text(
            frame.left,
            frame.top - 5.0 - p_size * 1.3,
            heading,
            TextStyle::new(p_size).align(HAlign::Left, VAlign::Baseline),
        );
    }
    let slot = (category_end - category_start) / k as f64;
    let spacing = slot * 0.8 / order.len() as f64;
    let half = spacing * 0.38;
    let separator = Stroke::solid(Color::grey(0.85), 0.75);
    let mut ticks = Vec::new();
    for (f, feature) in features.iter().enumerate() {
        let centre = categories.map(f as f64 + 1.0);
        ticks.push((centre, names[f].clone()));
        if f > 0 {
            let at = centre - slot / 2.0;
            let points = if options.horizontal {
                [(frame.left, at), (frame.right, at)]
            } else {
                [(at, frame.top), (at, frame.bottom)]
            };
            figure.line(&points, separator);
        }
        for (j, &g) in order.iter().enumerate() {
            let offset = (j as f64 - (order.len() as f64 - 1.0) / 2.0) * spacing;
            let group = &feature.groups[g];
            draw_box(
                &mut figure,
                &BoxPlace {
                    centre: centre + offset,
                    half,
                    frame,
                    axis: &axis,
                    scale: &values,
                },
                group.color,
                &group.values,
                &tooltip_title(Some(&feature.feature), &group.name),
                (f * 7919 + g) as u64,
                options,
            );
        }
        if let Some(p) = &p_values[f] {
            let style = TextStyle::new(p_size);
            if options.horizontal {
                figure.text(
                    frame.right + 6.0,
                    centre,
                    p.clone(),
                    style.align(HAlign::Left, VAlign::Middle),
                );
            } else {
                figure.text(
                    centre,
                    frame.top - 5.0,
                    p.clone(),
                    style.align(HAlign::Center, VAlign::Baseline),
                );
            }
        }
    }
    let label = crate::axes::scale_label(features[0].axis_label(), options.log_scale);
    let label = label.as_str();
    if options.horizontal {
        y_axis(&mut figure, &frame, &ticks, None, AXES_COLOR);
        x_axis(
            &mut figure,
            &frame,
            &axis.scaled(&values),
            Some(label),
            LabelRotation::Horizontal,
            AXES_COLOR,
        );
    } else {
        let (rotation, _) = category_labels(widest(&names, sizes.tick), slot);
        x_axis(&mut figure, &frame, &ticks, None, rotation, AXES_COLOR);
        y_axis(
            &mut figure,
            &frame,
            &axis.scaled(&values),
            Some(label),
            AXES_COLOR,
        );
    }
    axis.draw_break(&mut figure, &frame, &values, options.horizontal);

    // Legend of the groups.
    let x = figure.width - padding - legend_width;
    let mut y = frame.top;
    for (j, &g) in order.iter().enumerate() {
        let centre = y + line / 2.0;
        figure.rect(
            x,
            centre - 3.0,
            swatch,
            6.0,
            Some(features[0].groups[g].color),
            Some(Stroke::solid(Color::BLACK, 1.0)),
        );
        figure.text(
            x + swatch + 5.0,
            centre,
            labels[j].clone(),
            crate::axes::legend_style().align(HAlign::Left, VAlign::Middle),
        );
        y += line;
    }
    figure
}

/// First lines of a box's tooltip: the feature (with several) and group.
fn tooltip_title(feature: Option<&String>, group: &str) -> String {
    match feature {
        Some(feature) => format!("{feature}\n{group}"),
        None => group.to_string(),
    }
}

/// Where a box goes: the centre and half-width of its box along the
/// category axis, and the frame and value axis it is drawn against.
struct BoxPlace<'a> {
    centre: f64,
    half: f64,
    frame: Frame,
    axis: &'a ValueAxis,
    scale: &'a LinearScale,
}

impl BoxPlace<'_> {
    /// The figure point of a position along the category axis and a value.
    fn point(&self, category: f64, value: f64, horizontal: bool) -> (f64, f64) {
        let v = self.scale.map(self.axis.coord(value));
        if horizontal {
            (v, category)
        } else {
            (category, v)
        }
    }
}

/// A number from 0 to 1 for the `k`th sample of a box: a low-discrepancy
/// sequence, so points spread evenly and the plot is the same every time.
fn jitter(seed: u64, k: usize) -> f64 {
    let x = (seed as f64 * 0.754_877_666 + k as f64 * 0.618_033_988_75).fract();
    2.0 * x - 1.0
}

/// One box, with whiskers, median, outliers or points, mean and tooltip.
fn draw_box(
    figure: &mut Figure,
    place: &BoxPlace,
    color: Color,
    values: &[f64],
    title: &str,
    seed: u64,
    options: &BoxOptions,
) {
    let Some(s) = box_stats(values) else { return };
    let horizontal = options.horizontal;
    let (c, half) = (place.centre, place.half);
    let black = Stroke::solid(Color::BLACK, 1.0);
    let point = |category: f64, value: f64| place.point(category, value, horizontal);
    let corners = |from: (f64, f64), to: (f64, f64)| {
        (
            from.0.min(to.0),
            from.1.min(to.1),
            (to.0 - from.0).abs(),
            (to.1 - from.1).abs(),
        )
    };
    let (bx, by, bw, bh) = corners(point(c - half, s.q1), point(c + half, s.q3));

    // Whiskers with caps.
    for (from, to) in [(s.q1, s.whisker_low), (s.q3, s.whisker_high)] {
        figure.line(&[point(c, from), point(c, to)], black);
        figure.line(
            &[point(c - half / 2.0, to), point(c + half / 2.0, to)],
            black,
        );
    }
    if options.show_points {
        // The fill under the points, the outline over them.
        figure.rect(bx, by, bw, bh, Some(color), None);
        let n = values.iter().filter(|v| v.is_finite()).count();
        let opacity = if n <= 30 {
            0.75
        } else {
            (0.75 * (30.0 / n as f64).sqrt()).max(0.12)
        };
        let fill = Color {
            a: (opacity * 255.0).round() as u8,
            ..Color::rgb(40, 40, 40)
        };
        let radius = options.point_size.max(0.5) / 2.0;
        for (k, &value) in values.iter().filter(|v| v.is_finite()).enumerate() {
            let (x, y) = point(c + jitter(seed, k) * half * 0.8, value);
            figure.circle(x, y, radius, Some(fill), None);
        }
        figure.rect(bx, by, bw, bh, None, Some(black));
    } else {
        figure.rect(bx, by, bw, bh, Some(color), Some(black));
        for &outlier in &s.outliers {
            let (px, py) = point(c, outlier);
            let arm = 3.0;
            figure.line(&[(px - arm, py), (px + arm, py)], black);
            figure.line(&[(px, py - arm), (px, py + arm)], black);
        }
    }
    figure.line(
        &[point(c - half, s.median), point(c + half, s.median)],
        black,
    );
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    let off_axis = place.axis.log && mean > 0.0 && mean.log10() < place.axis.ticks_min();
    if options.show_averages && !off_axis {
        let (x, y) = point(c, mean);
        star(figure, x, y, 4.5f64.min(half * 0.9).max(2.0));
    }
    let zeros = finite.iter().filter(|&&v| v == 0.0).count();
    let mut tooltip = format!(
        "{title}\nMedian: {}\nMean: {}\nQuartiles: {} to {}\nSamples: {}",
        number(s.median),
        number(mean),
        number(s.q1),
        number(s.q3),
        finite.len()
    );
    if options.log_scale && zeros > 0 {
        tooltip.push_str(&format!("\nZeros: {zeros}"));
    }
    let frame = place.frame;
    let item = title.lines().last().unwrap_or(title);
    if horizontal {
        figure.region(
            frame.left,
            c - half,
            frame.width(),
            2.0 * half,
            item,
            tooltip,
        );
    } else {
        figure.region(
            c - half,
            frame.top,
            2.0 * half,
            frame.height(),
            item,
            tooltip,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::group_color;
    use crate::feature_plots::{Field, Group};
    use crate::figure::Shape;
    use crate::p_values::PValueKind;

    /// Number of the shapes that are points of samples.
    fn point_count(figure: &Figure) -> usize {
        figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Circle { stroke: None, .. }))
            .count()
    }

    fn feature(name: &str, scale: f64) -> FeatureData {
        FeatureData {
            feature: name.to_string(),
            field: Field::Proportion,
            groups: vec![
                Group {
                    name: "Horse".to_string(),
                    color: group_color(0),
                    samples: (0..6).map(|i| format!("H{i}")).collect(),
                    values: [0.0, 0.01, 0.05, 0.2, 1.0, 4.0]
                        .iter()
                        .map(|v| v * scale)
                        .collect(),
                },
                Group {
                    name: "Tapir".to_string(),
                    color: group_color(1),
                    samples: vec!["T1".into(), "T2".into()],
                    values: vec![2.0 * scale, 8.0 * scale],
                },
                Group {
                    name: "Ape".to_string(),
                    color: group_color(2),
                    samples: vec!["A1".into(), "A2".into(), "A3".into()],
                    values: vec![0.0, 0.0, 0.003 * scale],
                },
            ],
            p_value: Some("0.01".to_string()),
            p_value_kind: None,
            value_label: None,
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

    fn filled(figure: &Figure, color: Color) -> usize {
        figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == color))
            .count()
    }

    #[test]
    fn logarithmic_axis_with_zeros() {
        let axis = ValueAxis::new([0.0, 0.0012, 3.0], true).unwrap();
        assert!(axis.log);
        let labels: Vec<&str> = axis.ticks.iter().map(|t| t.1.as_str()).collect();
        assert_eq!(labels, ["0", "0.001", "0.01", "0.1", "1", "10"]);
        assert_eq!(axis.max, 1.0);
        let zero = axis.zero.unwrap();
        assert!(zero < -3.0 && axis.min < zero);
        assert!(axis.break_at.unwrap() > zero && axis.break_at.unwrap() < -3.0);
        assert_eq!(axis.coord(0.0), zero);
        assert!((axis.coord(0.01) + 2.0).abs() < 1e-12);
        // Without zeros the axis starts at the power of ten below the data.
        let axis = ValueAxis::new([0.02, 0.5], true).unwrap();
        assert_eq!((axis.min, axis.max, axis.zero), (-2.0, 0.0, None));
        // Exact powers of ten are not pushed out to the next one.
        let axis = ValueAxis::new([0.001, 10.0], true).unwrap();
        assert_eq!((axis.min, axis.max), (-3.0, 1.0));
        // A linear axis is that of STAMP v2.
        let axis = ValueAxis::new([10.0, 34.0], false).unwrap();
        assert_eq!((axis.min, axis.max), (10.0, 35.0));
        assert!(ValueAxis::new([f64::NAN], true).is_none());
    }

    #[test]
    fn group_orders() {
        let data = [feature("Equivita", 1.0)];
        let names = |order: &[usize]| -> Vec<String> {
            order
                .iter()
                .map(|&g| data[0].groups[g].name.clone())
                .collect()
        };
        assert_eq!(
            names(&group_order(&data, None, GroupOrder::AsGiven)),
            ["Horse", "Tapir", "Ape"]
        );
        assert_eq!(
            names(&group_order(&data, None, GroupOrder::Alphabetical)),
            ["Ape", "Horse", "Tapir"]
        );
        // Medians: Horse 0.125, Tapir 5, Ape 0.
        assert_eq!(
            names(&group_order(&data, None, GroupOrder::Median)),
            ["Tapir", "Horse", "Ape"]
        );
        let categories = [
            "Perissodactyla".to_string(),
            "Perissodactyla".into(),
            "Primates".into(),
        ];
        assert_eq!(
            names(&group_order(&data, Some(&categories), GroupOrder::Category)),
            ["Horse", "Tapir", "Ape"]
        );
        let categories = ["b".to_string(), "c".into(), "a".into()];
        assert_eq!(
            names(&group_order(&data, Some(&categories), GroupOrder::Category)),
            ["Ape", "Horse", "Tapir"]
        );
        // Without categories, as given.
        assert_eq!(
            names(&group_order(&data, None, GroupOrder::Category)),
            ["Horse", "Tapir", "Ape"]
        );
    }

    #[test]
    fn grouped_boxes_for_several_features() {
        let data = [
            feature("Equivita", 1.0),
            feature("Tapirivita", 0.1),
            feature("Gorillivita", 0.01),
        ];
        let options = BoxOptions {
            log_scale: true,
            show_counts: true,
            ..BoxOptions::default()
        };
        let figure = plot(&data, None, &options);
        // A box per feature and group, plus a legend swatch per group.
        assert_eq!(filled(&figure, group_color(0)), 4);
        assert_eq!(figure.regions.len(), 9);
        assert!(figure.regions[0].tooltip.starts_with("Equivita\nHorse\n"));
        let texts = texts(&figure);
        for wanted in [
            "Equivita",
            "Gorillivita",
            "Horse (n=6)",
            "Tapir (n=2)",
            "0",
            "0.001",
            "p = 0.01",
        ] {
            assert!(
                texts.iter().any(|t| t == wanted),
                "missing {wanted} in {texts:?}"
            );
        }
        // Zeros are counted in the tooltip.
        assert!(figure.regions[0].tooltip.contains("Zeros: 1"));
        // p-values of an analysis: their name once above the features, the
        // value above each; lying down, the whole text beside each.
        let named: Vec<FeatureData> = data
            .iter()
            .map(|f| FeatureData {
                p_value_kind: Some(PValueKind::Corrected),
                ..f.clone()
            })
            .collect();
        let texts = texts_of(&plot(&named, None, &options));
        assert_eq!(
            texts.iter().filter(|t| *t == "Corrected p-values").count(),
            1
        );
        // Each feature's value (one more "0.01" is a tick of the axis), and no
        // value with its name.
        assert_eq!(
            texts.iter().filter(|t| *t == "0.01").count(),
            data.len() + 1
        );
        assert!(!texts.iter().any(|t| t.ends_with("p-value = 0.01")));
        let lying = texts_of(&plot(
            &named,
            None,
            &BoxOptions {
                horizontal: true,
                ..options.clone()
            },
        ));
        assert!(lying.iter().any(|t| t == "Corrected p-value = 0.01"));
        assert!(!lying.iter().any(|t| t == "Corrected p-values"));
        // Lying down, the same boxes.
        let lying = plot(
            &data,
            None,
            &BoxOptions {
                horizontal: true,
                ..options.clone()
            },
        );
        assert_eq!(lying.regions.len(), 9);
        let r = &lying.regions[0];
        assert!(r.width > r.height);
    }

    #[test]
    fn one_panel_per_feature() {
        let data = [
            feature("Equivita", 1.0),
            feature("Tapirivita", 0.1),
            feature("Gorillivita", 0.01),
        ];
        let options = BoxOptions {
            layout: BoxLayout::Panels,
            show_points: true,
            ..BoxOptions::default()
        };
        let figure = plot(&data, None, &options);
        assert_eq!(figure.regions.len(), 9);
        // Every sample is a point: 11 per feature.
        assert_eq!(point_count(&figure), 33);
        // No crosses for outliers when the points are drawn.
        let texts = texts(&figure);
        assert_eq!(texts.iter().filter(|t| *t == "Horse").count(), 3);
        assert_eq!(texts.iter().filter(|t| *t == "p = 0.01").count(), 3);
        // Independent axes: each panel ends at its own largest value.
        let shared_top = |figure: &Figure| texts_of(figure).contains(&"8".to_string());
        assert!(shared_top(&figure));
        let independent = plot(
            &data,
            None,
            &BoxOptions {
                shared_axis: false,
                ..options.clone()
            },
        );
        assert!(texts_of(&independent)
            .iter()
            .any(|t| t == "0.08" || t == "0.080"));
        let lying = plot(
            &data,
            None,
            &BoxOptions {
                horizontal: true,
                ..options
            },
        );
        assert_eq!(point_count(&lying), 33);
    }

    fn texts_of(figure: &Figure) -> Vec<String> {
        texts(figure)
    }

    #[test]
    fn one_feature_is_the_classic_box_plot_on_any_axis() {
        let data = [feature("Equivita", 1.0)];
        let linear = plot(&data, None, &BoxOptions::default());
        assert_eq!(linear.regions.len(), 3);
        assert_eq!(linear.regions[0].tooltip.lines().next(), Some("Horse"));
        let log = plot(
            &data,
            None,
            &BoxOptions {
                log_scale: true,
                ..BoxOptions::default()
            },
        );
        assert!(texts(&log).iter().any(|t| t == "0.001"));
        // The quartiles do not depend on the axis.
        assert_eq!(
            linear.regions[0].tooltip,
            log.regions[0].tooltip.replace("\nZeros: 1", "")
        );
        // A broken axis: a white gap on the axis line.
        assert!(log.shapes.iter().any(|s| matches!(
            s,
            Shape::Rect {
                fill: Some(Color::WHITE),
                stroke: None,
                ..
            }
        )));
        assert!(!linear.shapes.iter().any(|s| matches!(
            s,
            Shape::Rect {
                fill: Some(Color::WHITE),
                stroke: None,
                ..
            }
        )));
    }

    #[test]
    fn mismatched_or_empty_features() {
        let mut other = feature("B", 1.0);
        other.groups.pop();
        let figure = plot(&[feature("A", 1.0), other], None, &BoxOptions::default());
        assert!(texts(&figure)[0].contains("different groups"));
        assert_eq!(texts(&plot(&[], None, &BoxOptions::default())), [NOTHING]);
    }
}
