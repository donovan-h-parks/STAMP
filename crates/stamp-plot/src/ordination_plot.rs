//! Ordination plots (PCoA, NMDS, db-RDA and CCA): samples placed on the
//! first axes of an ordination, in up to three panels, coloured and shaped
//! by group or by metadata, with the explanatory variables of a
//! constrained ordination drawn over them (arrows for numbers, centroids
//! for categories). The ordination itself is computed elsewhere
//! (`community_plots`).

use crate::axes::{
    group_color, marker_legend_at, marker_styled, repeated_colors_note, scaled_ticks,
    swatch_legend_at, swatch_legend_origin, swatch_legend_size, x_axis, y_axis, y_axis_width,
    Frame, LabelRotation, LegendPosition, Marker, AXES_COLOR, INCH,
};
use crate::axis::Ticks;
use crate::figure::{Color, Figure, Stroke};
use crate::heatmap::ColorMap;
use crate::ordination_style::{
    category_counts, draw_legend, legend_size, short_number, ColorScale, ColorSource, DrawOrder,
    LegendItem, SampleColors, ShapePlan, Styling, ValueScale, NO_VALUE_COLOR,
};

/// The explanatory variables of a constrained ordination, drawn over its
/// samples.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Biplot {
    /// Numeric variables, drawn as arrows from the origin.
    pub arrows: Vec<BiplotItem>,
    /// Categories, drawn as crosses at the centroids of their samples.
    pub centroids: Vec<BiplotItem>,
}

/// A variable or category of a [`Biplot`].
#[derive(Debug, Clone, PartialEq)]
pub struct BiplotItem {
    pub name: String,
    /// The end of the arrow or the centroid, on each axis. Arrows are
    /// stretched together to fit each panel; centroids are in the units of
    /// the samples.
    pub position: Vec<f64>,
    pub tooltip: String,
}

/// The colour of the arrows and centroids of the explanatory variables.
pub const BIPLOT_COLOR: Color = Color::rgb(31, 78, 145);

/// The gap between the tip of an arrow and its name, in points.
const LABEL_GAP: f64 = 3.0;

/// The shortest an axis is drawn at a fixed scale, as a share of the
/// longest.
const SHORTEST_AXIS: f64 = 0.25;

/// The share of the way to the edge of a panel that the longest arrow
/// reaches.
const ARROW_REACH: f64 = 0.8;

/// The names of `count` axes: `prefix` and a number.
pub fn numbered_axes(prefix: &str, count: usize) -> Vec<String> {
    (1..=count).map(|i| format!("{prefix}{i}")).collect()
}

/// A sample with its group.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub name: String,
    /// Index of the sample's group in [`Data::groups`].
    pub group: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub samples: Vec<Sample>,
    pub groups: Vec<String>,
    /// Colour of each group; groups beyond the list use the default palette.
    pub group_colors: Vec<Color>,
    /// What colours and shapes the samples, if not only their groups.
    pub styling: Styling,
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

/// Appearance of an ordination plot. Lengths are in inches.
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
    /// What colours the samples: their group (the default), a metadata
    /// field, or a feature's relative abundance. Resolved into
    /// [`Data::styling`] by [`crate::ordination_style::resolve`].
    pub color_by: ColorSource,
    /// A categorical metadata field that gives the samples their marker
    /// shapes (`None`: by group if `unique_shapes`, else circles).
    pub shape_by: Option<String>,
    /// The colour scale and how numbers are placed on it, when the samples
    /// are coloured by a number.
    pub color_map: ColorMap,
    pub color_scale: ColorScale,
    /// Which samples are drawn on top when coloured by a number.
    pub draw_order: DrawOrder,
    /// Opacity of the markers, from 0 (invisible) to 1.
    pub opacity: f64,
    /// Draw the explanatory variables of a constrained ordination.
    pub show_variables: bool,
    /// Size of the names of the explanatory variables, in points.
    pub variable_font_size: f64,
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
            color_by: ColorSource::Group,
            shape_by: None,
            color_map: ColorMap::Viridis,
            color_scale: ColorScale::Linear,
            draw_order: DrawOrder::HighOnTop,
            opacity: 1.0,
            show_variables: true,
            variable_font_size: 9.0,
        }
    }
}

/// How one sample is drawn.
struct Look {
    shape: Marker,
    fill: Option<Color>,
    edge: Stroke,
    area: f64,
    tooltip: String,
}

/// The colours, shapes and tooltips of the samples, the order to draw them
/// in, and the legend beside the plot (empty for a plot coloured and shaped
/// only by group, whose legend goes inside the frame).
fn looks(data: &Data, options: &Options) -> (Vec<Look>, Vec<usize>, Vec<LegendItem>) {
    let styling = &data.styling;
    let alpha = (options.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let with_alpha = |c: Color| Color { a: alpha, ..c };
    let edge = Stroke::solid(with_alpha(Color::BLACK), 0.75);
    let names = || data.samples.iter().map(|s| s.name.as_str());
    let group_name = |group: usize| data.groups.get(group).map(String::as_str).unwrap_or("");
    let mut items = Vec::new();
    let no_value_entry = |count: usize, shape: Marker| LegendItem::Entry {
        shape,
        fill: None,
        edge: NO_VALUE_COLOR,
        label: format!("No value ({count})"),
    };

    // Colours: a colour per sample, or `None` for a sample without a value.
    let mut fills: Vec<Option<Color>> = Vec::new();
    let mut color_text: Vec<Option<String>> = vec![None; data.samples.len()];
    let mut sort_keys: Option<Vec<f64>> = None;
    let mut color_items = Vec::new();
    match &styling.color {
        SampleColors::Group => {
            fills = data
                .samples
                .iter()
                .map(|s| Some(data.group_color(s.group)))
                .collect();
            color_items.push(LegendItem::Title(styling.group_label().to_string()));
            for (i, group) in data.groups.iter().enumerate() {
                color_items.push(LegendItem::Entry {
                    shape: Marker::Circle,
                    fill: Some(data.group_color(i)),
                    edge: Color::BLACK,
                    label: group.clone(),
                });
            }
        }
        SampleColors::Categories { label, values } => {
            let counts = category_counts(names(), values);
            let categories: Vec<&String> = counts.keys().collect();
            for sample in &data.samples {
                let category = values.get(&sample.name);
                fills
                    .push(category.map(|c| {
                        group_color(categories.iter().position(|k| *k == c).unwrap_or(0))
                    }));
            }
            for (i, sample) in data.samples.iter().enumerate() {
                color_text[i] = Some(format!(
                    "{label}: {}",
                    values.get(&sample.name).map_or("no value", String::as_str)
                ));
            }
            color_items.push(LegendItem::Title(label.clone()));
            for (i, (category, count)) in counts.iter().enumerate() {
                color_items.push(LegendItem::Entry {
                    shape: Marker::Circle,
                    fill: Some(group_color(i)),
                    edge: Color::BLACK,
                    label: format!("{category} ({count})"),
                });
            }
            let missing = fills.iter().filter(|f| f.is_none()).count();
            if missing > 0 {
                color_items.push(no_value_entry(missing, Marker::Circle));
            }
            if let Some(note) = repeated_colors_note(counts.len()) {
                color_items.push(LegendItem::Note(note.replace("groups", "values")));
            }
        }
        SampleColors::Values { label, values } => {
            let raw: Vec<Option<f64>> = names().map(|n| values.get(n).copied()).collect();
            let present: Vec<f64> = raw.iter().flatten().copied().collect();
            let scale = ValueScale::new(&present, options.color_scale);
            fills = raw
                .iter()
                .map(|v| v.map(|v| options.color_map.color(scale.fraction(v))))
                .collect();
            for (i, v) in raw.iter().enumerate() {
                color_text[i] = Some(format!(
                    "{label}: {}",
                    v.map_or("no value".to_string(), short_number)
                ));
            }
            sort_keys = Some(raw.iter().map(|v| v.unwrap_or(f64::NAN)).collect());
            color_items.extend(scale.legend(label, options.color_map));
            let missing = raw.iter().filter(|v| v.is_none()).count();
            if missing > 0 {
                color_items.push(no_value_entry(missing, Marker::Circle));
            }
        }
    }

    // Shapes: from the field asked for, else by group if wanted.
    let mut shapes = vec![Marker::Circle; data.samples.len()];
    let mut shape_text: Vec<Option<String>> = vec![None; data.samples.len()];
    let mut shape_items = Vec::new();
    // Coloured by something else, the groups keep their own shapes (if
    // wanted), treated like a shape field.
    let group_shapes =
        (styling.shape.is_none() && options.unique_shapes && styling.color != SampleColors::Group)
            .then(|| crate::ordination_style::SampleShapes {
                label: styling.group_label().to_string(),
                values: data
                    .samples
                    .iter()
                    .map(|s| (s.name.clone(), group_name(s.group).to_string()))
                    .collect(),
            });
    if let Some(field) = styling.shape.as_ref().or(group_shapes.as_ref()) {
        let counts = category_counts(names(), &field.values);
        let missing = names().filter(|n| !field.values.contains_key(*n)).count();
        let plan = ShapePlan::new(&counts, missing);
        for (i, sample) in data.samples.iter().enumerate() {
            let value = field.values.get(&sample.name);
            shapes[i] = plan.shape(value);
            if group_shapes.is_none() {
                shape_text[i] = Some(format!(
                    "{}: {}",
                    field.label,
                    value.map_or("no value", String::as_str)
                ));
            }
        }
        shape_items = plan.legend(&field.label, &counts, missing);
    } else if options.unique_shapes {
        for (i, sample) in data.samples.iter().enumerate() {
            shapes[i] = Marker::for_group(sample.group);
        }
    }
    let looks = data
        .samples
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            let mut tooltip = format!("{}\n{}", sample.name, group_name(sample.group));
            for extra in [&color_text[i], &shape_text[i]].into_iter().flatten() {
                tooltip.push('\n');
                tooltip.push_str(extra);
            }
            match fills[i] {
                Some(fill) => Look {
                    shape: shapes[i],
                    fill: Some(with_alpha(fill)),
                    edge,
                    area: options.marker_size,
                    tooltip,
                },
                // Without a value: small, hollow and grey.
                None => Look {
                    shape: shapes[i],
                    fill: None,
                    edge: Stroke::solid(with_alpha(NO_VALUE_COLOR), 0.75),
                    area: options.marker_size * 0.5,
                    tooltip,
                },
            }
        })
        .collect();

    // Samples without a value first, then by value as asked.
    let mut order: Vec<usize> = (0..data.samples.len()).collect();
    order.sort_by_key(|&i| fills[i].is_some());
    if let Some(keys) = &sort_keys {
        match options.draw_order {
            DrawOrder::HighOnTop => order.sort_by(|&a, &b| {
                fills[a]
                    .is_some()
                    .cmp(&fills[b].is_some())
                    .then(keys[a].total_cmp(&keys[b]))
            }),
            DrawOrder::LowOnTop => order.sort_by(|&a, &b| {
                fills[a]
                    .is_some()
                    .cmp(&fills[b].is_some())
                    .then(keys[b].total_cmp(&keys[a]))
            }),
            DrawOrder::AsGiven => {}
        }
    }

    if styling.is_extended() {
        items.extend(color_items);
        if !shape_items.is_empty() {
            items.push(LegendItem::Gap);
            items.extend(shape_items);
        }
    }
    (looks, order, items)
}

/// Draw samples at positions found by any ordination: `positions[sample]`
/// holds the coordinates of the sample of `data` with the same index, and
/// `variance_fractions` the share of the variation each axis accounts for.
/// `axes` names the axes, `notes` are written above the plot, one to a
/// line, and `biplot` is drawn over the samples.
pub fn plot_ordination(
    data: &Data,
    positions: &[Vec<f64>],
    variance_fractions: &[f64],
    axes: &[String],
    notes: &[String],
    biplot: &Biplot,
    options: &Options,
) -> Figure {
    if positions.len() != data.samples.len() {
        return Figure::message("Degenerate plot: the ordination does not match the samples");
    }
    let components = variance_fractions.len().min(axes.len());
    if components < 2 {
        return Figure::message("Degenerate plot: the samples vary along a single direction.");
    }
    let (looks, order, mut items) = looks(data, options);
    if options.legend == LegendPosition::None {
        items.clear();
    }
    let beside = options.show_pc3_vs_pc2 && components >= 3;
    let beneath = options.show_pc1_vs_pc3 && components >= 3;
    let coordinate = |sample: usize, component: usize| positions[sample][component];
    // An ordination with no share of the variation per axis (NMDS) gives
    // `NaN`, and its axes are named only.
    let label = |component: usize| {
        let fraction = variance_fractions[component];
        if fraction.is_nan() {
            axes[component].clone()
        } else {
            format!("{} ({:.1}%)", axes[component], fraction * 100.0)
        }
    };
    let biplot = if options.show_variables {
        biplot
    } else {
        &Biplot::default()
    };
    let ticks_for = |component: usize| {
        let values = (0..data.samples.len()).map(|s| coordinate(s, component));
        let (low, high) = values.fold((0.0f64, 0.0f64), |(lo, hi), v| (lo.min(v), hi.max(v)));
        let pad = (high - low) * 0.05;
        Ticks::inside(low - pad, high + pad)
    };
    let mut ticks: Vec<Ticks> = (0..if beside || beneath { 3 } else { 2 })
        .map(ticks_for)
        .collect();
    if options.fixed_scale {
        // At one length per unit, an axis along which the samples hardly
        // spread (a second constrained axis, say) would make a panel too
        // thin to see: it is given empty room on both sides instead.
        let widest = ticks.iter().map(|t| t.max - t.min).fold(0.0, f64::max);
        let least = widest * SHORTEST_AXIS;
        for t in ticks.iter_mut() {
            if t.max - t.min < least {
                let centre = (t.max + t.min) / 2.0;
                *t = Ticks::inside(centre - least / 2.0, centre + least / 2.0);
            }
        }
    }
    let span = |component: usize| ticks[component].max - ticks[component].min;

    let width = options.width * INCH;
    let padding = 0.25 * INCH;
    // The legend beside the plot, on the right.
    let (legend_width, legend_height) = legend_size(&items);
    let legend_room = if items.is_empty() {
        0.0
    } else {
        legend_width + 0.2 * INCH
    };
    // Room above the plot for the notes.
    let note_line = options.note_font_size + 3.0;
    let top = padding + note_line * notes.len() as f64;
    let left = padding + y_axis_width(&ticks[1], true);
    let bottom_labels = crate::axes::x_axis_room();
    let gap = 0.35 * INCH;
    let inner_width = width - left - padding - legend_room;
    // Width and height of the main panel, and the sizes of the others.
    let (main_w, main_h, beside_w, beneath_h, mut height);
    if options.fixed_scale {
        // One length per unit of distance, set by what must fit across.
        let across = span(0) + if beside { span(2) } else { 0.0 };
        let room = inner_width - if beside { gap } else { 0.0 };
        if !(across > 0.0 && span(1) > 0.0 && room > 0.0) {
            return Figure::message("Degenerate plot");
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
                "The plot would be too tall; switch off \"Fixed pixels per unit distance\"",
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
        return Figure::message("The figure is too small for this plot");
    }
    // A tall legend makes the figure taller.
    height = height.max(top + legend_height + padding);
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
    // Draws one panel and returns the y coordinate below its x axis.
    let panel = |figure: &mut Figure,
                 frame: Frame,
                 cx: usize,
                 cy: usize,
                 x_label: bool,
                 y_label: bool,
                 rotate: bool|
     -> f64 {
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
        for &index in &order {
            let look = &looks[index];
            let (px, py) = (x.map(coordinate(index, cx)), y.map(coordinate(index, cy)));
            marker_styled(figure, look.shape, px, py, look.area, look.fill, look.edge);
        }
        draw_biplot(figure, biplot, &frame, (&x, &y), (cx, cy), options);
        // Tooltips in the order of the samples.
        for (index, sample) in data.samples.iter().enumerate() {
            let (px, py) = (x.map(coordinate(index, cx)), y.map(coordinate(index, cy)));
            let r = options.marker_size.max(0.0).sqrt() / 2.0 + 1.0;
            figure.region(
                px - r,
                py - r,
                2.0 * r,
                2.0 * r,
                &sample.name,
                looks[index].tooltip.clone(),
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
        let below = x_axis(
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
        below
    };

    // Neighbouring panels share an axis: PC2 with the panel beside the main
    // one, PC1 with the panel beneath it.
    panel(&mut figure, main, 0, 1, !beneath, true, false);
    // With all three panels the cell at the bottom right is empty, and the
    // legend of the groups goes there: `spare` is that cell, from below the
    // x axis of the panel above it.
    let mut spare = None;
    if beside {
        let frame = Frame {
            left: main.right + gap,
            top: main.top,
            right: main.right + gap + beside_w,
            bottom: main.bottom,
        };
        let below = panel(
            &mut figure,
            frame,
            2,
            1,
            true,
            false,
            options.rotate_pc3_labels,
        );
        if beneath {
            spare = Some(Frame {
                left: frame.left,
                top: below,
                right: frame.right,
                bottom: (main.bottom + gap + beneath_h).max(below),
            });
        }
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

    if data.styling.is_extended() {
        let x = width - padding - legend_width;
        draw_legend(&mut figure, &items, x, top, options.marker_size);
    } else {
        let entries: Vec<(String, Color)> = data
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| (g.clone(), data.group_color(i)))
            .collect();
        // Each group's shape beside its colour, when the groups have their
        // own shapes.
        let markers: Vec<Marker> = (0..entries.len()).map(Marker::for_group).collect();
        let draw_at = |figure: &mut Figure, x: f64, y: f64| {
            if options.unique_shapes {
                marker_legend_at(figure, &entries, &markers, options.marker_size, x, y);
            } else {
                swatch_legend_at(figure, &entries, x, y);
            }
        };
        match spare {
            // The place is taken within the empty cell. A legend larger
            // than the cell is kept below the axis above it and clear of
            // the panel to its left where the figure has the room, and
            // the figure grows downwards to hold it.
            Some(cell) => {
                if let Some((x, y)) = swatch_legend_origin(&figure, &cell, &entries, options.legend)
                {
                    let (legend_width, legend_height) = swatch_legend_size(&entries);
                    let x = x.max(main.right + 8.0).min(width - 4.0 - legend_width);
                    let y = y.max(cell.top);
                    draw_at(&mut figure, x, y);
                    figure.height = figure.height.max(y + legend_height + 4.0);
                }
            }
            None => {
                if let Some((x, y)) = swatch_legend_origin(&figure, &main, &entries, options.legend)
                {
                    draw_at(&mut figure, x, y);
                }
            }
        }
    }
    figure
}

/// Draw the explanatory variables in a panel showing axes `cx` and `cy`:
/// arrows stretched together so that the longest reaches most of the way
/// to the edge, and crosses at the centroids of categories.
fn draw_biplot(
    figure: &mut Figure,
    biplot: &Biplot,
    frame: &Frame,
    (x, y): (&crate::axis::LinearScale, &crate::axis::LinearScale),
    (cx, cy): (usize, usize),
    options: &Options,
) {
    use crate::figure::{HAlign, TextStyle, VAlign};
    let on = |item: &BiplotItem, axis: usize| item.position.get(axis).copied().unwrap_or(0.0);
    let style = |dx: f64, dy: f64| {
        let h = if dx.abs() < 1e-9 {
            HAlign::Center
        } else if dx > 0.0 {
            HAlign::Left
        } else {
            HAlign::Right
        };
        let v = if dy.abs() < 1e-9 || dx.abs() > dy.abs() {
            VAlign::Middle
        } else if dy > 0.0 {
            VAlign::Bottom
        } else {
            VAlign::Top
        };
        TextStyle::new(options.variable_font_size)
            .color(BIPLOT_COLOR)
            .align(h, v)
    };
    let (x0, y0) = (x.map(0.0), y.map(0.0));
    // How far the arrows may stretch: from the origin to each edge.
    let room = |toward: f64, low: f64, high: f64, origin: f64| -> f64 {
        if toward > 0.0 {
            (high - origin) / toward
        } else if toward < 0.0 {
            (low - origin) / toward
        } else {
            f64::INFINITY
        }
    };
    // The box of an arrow's name around its tip, in page points (y
    // downwards), as `style` aligns it: (left, right, top, bottom).
    let size = options.variable_font_size;
    let label_box = |name: &str, ux: f64, uy: f64| -> (f64, f64, f64, f64) {
        let (w, h) = (crate::text::width(name, size), size);
        let (ax, ay) = (ux * LABEL_GAP, uy * LABEL_GAP);
        let (left, right) = if ux.abs() < 1e-9 {
            (ax - w / 2.0, ax + w / 2.0)
        } else if ux > 0.0 {
            (ax, ax + w)
        } else {
            (ax - w, ax)
        };
        let (top, bottom) = if uy.abs() < 1e-9 || ux.abs() > uy.abs() {
            (ay - h / 2.0, ay + h / 2.0)
        } else if uy < 0.0 {
            // Pointing up: the name above the tip.
            (ay - h, ay)
        } else {
            (ay, ay + h)
        };
        (left, right, top, bottom)
    };
    // How far the arrows may stretch: the longest most of the way to the
    // edge.
    let stretch = biplot
        .arrows
        .iter()
        .map(|a| {
            // In points on the page, y downwards.
            let dx = x.map(on(a, cx)) - x0;
            let dy = y.map(on(a, cy)) - y0;
            room(dx, frame.left, frame.right, x0).min(room(dy, frame.top, frame.bottom, y0))
                * ARROW_REACH
        })
        .fold(f64::INFINITY, f64::min);
    let stroke = Stroke::solid(BIPLOT_COLOR, 1.25);
    if stretch.is_finite() && stretch > 0.0 {
        for arrow in &biplot.arrows {
            let dx = (x.map(on(arrow, cx)) - x0) * stretch;
            let dy = (y.map(on(arrow, cy)) - y0) * stretch;
            let length = (dx * dx + dy * dy).sqrt();
            if length < 1.0 {
                continue;
            }
            let (tx, ty) = (x0 + dx, y0 + dy);
            figure.line(&[(x0, y0), (tx, ty)], stroke);
            // The head: a filled triangle at the tip.
            let (ux, uy) = (dx / length, dy / length);
            let (head, half) = (7.0, 3.0);
            let (bx, by) = (tx - ux * head, ty - uy * head);
            figure.area(
                vec![vec![
                    (tx, ty),
                    (bx - uy * half, by + ux * half),
                    (bx + uy * half, by - ux * half),
                ]],
                BIPLOT_COLOR,
            );
            // A name that would cross the edge of the panel is moved back
            // inside it.
            let (left, right, top, bottom) = label_box(&arrow.name, ux, uy);
            let inside = |at: f64, low: f64, high: f64, first: f64, last: f64| {
                if at + low < first {
                    first - low
                } else if at + high > last {
                    last - high
                } else {
                    at
                }
            };
            figure.text(
                inside(tx, left, right, frame.left, frame.right) + ux * LABEL_GAP,
                inside(ty, top, bottom, frame.top, frame.bottom) + uy * LABEL_GAP,
                arrow.name.clone(),
                style(ux, -uy),
            );
            figure.region(
                tx - 5.0,
                ty - 5.0,
                10.0,
                10.0,
                &arrow.name,
                arrow.tooltip.clone(),
            );
        }
    }
    for centroid in &biplot.centroids {
        let (px, py) = (x.map(on(centroid, cx)), y.map(on(centroid, cy)));
        let r = 4.0;
        figure.line(&[(px - r, py - r), (px + r, py + r)], stroke);
        figure.line(&[(px - r, py + r), (px + r, py - r)], stroke);
        figure.text(px + r + 2.0, py, centroid.name.clone(), style(1.0, 0.0));
        figure.region(
            px - r,
            py - r,
            2.0 * r,
            2.0 * r,
            &centroid.name,
            centroid.tooltip.clone(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn data() -> Data {
        let sample = |name: &str, group: usize| Sample {
            name: name.to_string(),
            group,
        };
        Data {
            samples: vec![
                sample("a1", 0),
                sample("a2", 0),
                sample("a3", 0),
                sample("b1", 1),
                sample("b2", 1),
            ],
            groups: vec!["A".to_string(), "B".to_string()],
            group_colors: Vec::new(),
            styling: Styling::default(),
        }
    }

    /// Where an ordination of [`data`] puts its samples, on three axes.
    fn positions(data: &Data) -> Vec<Vec<f64>> {
        data.samples
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let i = i as f64;
                vec![
                    0.3 * (1.3 * i).sin() + 0.4 * s.group as f64,
                    0.2 * (0.7 * i + 0.5).cos(),
                    0.1 * (2.1 * i).sin(),
                ]
            })
            .collect()
    }

    /// The share of the variation on each of the three axes.
    const FRACTIONS: [f64; 3] = [0.6, 0.3, 0.1];

    fn plot(data: &Data, options: &Options) -> Figure {
        plot_ordination(
            data,
            &positions(data),
            &FRACTIONS,
            &numbered_axes("PC", 3),
            &[],
            &Biplot::default(),
            options,
        )
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
        let figure = plot(&data(), &Options::default());
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
            &data(),
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

    /// What the legend draws beside a group's name: the shape just before
    /// the last text that is the name.
    fn key_of<'a>(figure: &'a Figure, name: &str) -> &'a Shape {
        let at = figure
            .shapes
            .iter()
            .rposition(|s| matches!(s, Shape::Text { text, .. } if text == name))
            .unwrap();
        &figure.shapes[at - 1]
    }

    #[test]
    fn the_legend_shows_the_shape_of_each_group() {
        for panels in [true, false] {
            let with = |unique_shapes: bool| {
                plot(
                    &data(),
                    &Options {
                        unique_shapes,
                        legend: LegendPosition::UpperRight,
                        show_pc3_vs_pc2: panels,
                        show_pc1_vs_pc3: panels,
                        ..Options::default()
                    },
                )
            };
            // A's circle and B's square, as on the plot.
            let shaped = with(true);
            assert!(
                matches!(key_of(&shaped, "A"), Shape::Circle { fill: Some(c), .. } if *c == group_color(0)),
                "{:?}",
                key_of(&shaped, "A")
            );
            assert!(matches!(
                key_of(&shaped, "B"),
                Shape::Rect { width, height, fill: Some(c), .. }
                    if width == height && *c == group_color(1)
            ));
            // Every group a circle: colour swatches.
            let swatches = with(false);
            for name in ["A", "B"] {
                assert!(matches!(
                    key_of(&swatches, name),
                    Shape::Rect { width, height, .. } if width > height
                ));
            }
        }
    }

    #[test]
    fn the_legend_of_three_panels_is_in_the_empty_cell() {
        // Where a group's name is written in the legend.
        let name_at = |figure: &Figure, name: &str| {
            figure
                .shapes
                .iter()
                .find_map(|s| match s {
                    Shape::Text { x, y, text, .. } if text == name => Some((*x, *y)),
                    _ => None,
                })
                .unwrap()
        };
        let with = |legend: LegendPosition, panels: bool| {
            plot(
                &data(),
                &Options {
                    legend,
                    show_pc3_vs_pc2: panels,
                    show_pc1_vs_pc3: panels,
                    ..Options::default()
                },
            )
        };
        // The panels are drawn main, beside, beneath, five samples each:
        // the cell is right of the main panel's samples and below those of
        // the panel beside it.
        let edges = |figure: &Figure| {
            let right = figure.regions[..5]
                .iter()
                .map(|r| r.x + r.width)
                .fold(0.0, f64::max);
            let bottom = figure.regions[5..10]
                .iter()
                .map(|r| r.y + r.height)
                .fold(0.0, f64::max);
            (right, bottom)
        };
        for position in [
            LegendPosition::Best,
            LegendPosition::UpperLeft,
            LegendPosition::LowerRight,
        ] {
            let figure = with(position, true);
            let (right, bottom) = edges(&figure);
            for name in ["A", "B"] {
                let (x, y) = name_at(&figure, name);
                assert!(x > right && y > bottom, "{position:?} {name}");
                assert!(y < figure.height);
            }
        }
        // The position is taken within the cell.
        let upper = name_at(&with(LegendPosition::UpperLeft, true), "A");
        let lower = name_at(&with(LegendPosition::LowerRight, true), "A");
        assert!(upper.0 < lower.0 && upper.1 < lower.1);
        // No legend when none is asked for.
        assert!(!texts(&with(LegendPosition::None, true)).contains(&"A".to_string()));

        // Without the three panels there is no empty cell, and the legend
        // stays in the main panel.
        let single = with(LegendPosition::UpperLeft, false);
        let (x, _) = name_at(&single, "A");
        let right = single
            .regions
            .iter()
            .map(|r| r.x + r.width)
            .fold(0.0, f64::max);
        assert!(x < right);
    }

    #[test]
    fn panels_are_chosen_one_by_one() {
        let regions = |beside: bool, beneath: bool| {
            plot(
                &data(),
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
                &data(),
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
        let fixed = plot(&data(), &Options::default());
        let free = plot(
            &data(),
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
        let placed = positions(&data());
        let (a, b) = (&placed[0], &placed[3]);
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
            &data(),
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
            &data(),
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

    fn two_panels() -> Options {
        Options {
            show_pc3_vs_pc2: false,
            show_pc1_vs_pc3: false,
            legend: LegendPosition::Best,
            ..Options::default()
        }
    }

    fn styled(color: SampleColors, shape: Option<&[(&str, &str)]>) -> Data {
        Data {
            styling: Styling {
                color,
                shape: shape.map(|pairs| crate::ordination_style::SampleShapes {
                    label: "Site".to_string(),
                    values: pairs
                        .iter()
                        .map(|(s, v)| (s.to_string(), v.to_string()))
                        .collect(),
                }),
                group_label: Some("Diet".to_string()),
            },
            ..data()
        }
    }

    /// The fill colour of the marker drawn for each sample, in drawing
    /// order (circles only).
    fn circle_fills(figure: &Figure) -> Vec<Option<Color>> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Circle { fill, .. } => Some(*fill),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn samples_are_coloured_by_a_number() {
        let values = [("a1", 1.0), ("a2", 5.0), ("a3", 3.0), ("b1", 0.0)]
            .iter()
            .map(|(s, v)| (s.to_string(), *v))
            .collect();
        let data = styled(
            SampleColors::Values {
                label: "Depth".to_string(),
                values,
            },
            None,
        );
        let options = Options {
            unique_shapes: false,
            ..two_panels()
        };
        let figure = plot(&data, &options);
        let texts = texts(&figure);
        assert!(texts.contains(&"Depth".to_string()));
        // b2 has no value: a hollow point, listed in the legend.
        assert!(texts.contains(&"No value (1)".to_string()));
        assert!(figure.regions[4].tooltip.contains("Depth: no value"));
        assert!(figure.regions[1].tooltip.ends_with("Depth: 5"));
        assert!(figure.regions[1].tooltip.starts_with("a2\nA\n"));
        // The first five circles are the samples: without a value first,
        // then from low to high, the highest drawn last in the brightest
        // colour of viridis.
        let fills = circle_fills(&figure);
        assert_eq!(fills[0], None);
        assert_eq!(fills[1], Some(ColorMap::Viridis.color(0.0)));
        assert_eq!(fills[4], Some(ColorMap::Viridis.color(1.0)));
        let low_on_top = plot(
            &data,
            &Options {
                draw_order: DrawOrder::LowOnTop,
                ..options.clone()
            },
        );
        assert_eq!(
            circle_fills(&low_on_top)[4],
            Some(ColorMap::Viridis.color(0.0))
        );

        // On a log scale the zero is drawn at a floor of half the smallest
        // positive value, and said so.
        let log = plot(
            &data,
            &Options {
                color_scale: ColorScale::Log10,
                ..options.clone()
            },
        );
        let texts = self::texts(&log);
        assert!(texts.contains(&"Logarithmic scale".to_string()));
        assert!(texts.iter().any(|t| t == "1 zero drawn at 0.5"));
        assert!(texts.contains(&"0".to_string()));
        // The legend is beside the plot, to the right of the main panel.
        let legend_x = log
            .shapes
            .iter()
            .find_map(|s| match s {
                Shape::Text { x, text, .. } if text == "Logarithmic scale" => Some(*x),
                _ => None,
            })
            .unwrap();
        let rightmost_region = log
            .regions
            .iter()
            .map(|r| r.x + r.width)
            .fold(0.0, f64::max);
        assert!(legend_x > rightmost_region);

        // Opacity applies to the markers.
        let faint = plot(
            &data,
            &Options {
                opacity: 0.5,
                ..options
            },
        );
        assert_eq!(circle_fills(&faint)[4].unwrap().a, 128);
    }

    #[test]
    fn samples_are_coloured_by_category_and_shaped_by_another_field() {
        let categories = [("a1", "x"), ("a2", "y"), ("b1", "y"), ("b2", "z")]
            .iter()
            .map(|(s, v)| (s.to_string(), v.to_string()))
            .collect();
        let data = styled(
            SampleColors::Categories {
                label: "Host".to_string(),
                values: categories,
            },
            Some(&[("a1", "North"), ("a2", "South"), ("a3", "North")]),
        );
        let figure = plot(&data, &two_panels());
        let texts = texts(&figure);
        for wanted in [
            "Host",
            "x (1)",
            "y (2)",
            "z (1)",
            "No value (1)",
            "Site",
            "North (2)",
            "South (1)",
            "No value (2)",
        ] {
            assert!(texts.contains(&wanted.to_string()), "missing {wanted}");
        }
        assert!(figure.regions[0].tooltip.ends_with("Host: x\nSite: North"));
        // Shapes by Site: circles for North, squares for South and
        // triangles for no value.
        let squares = figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect { fill: Some(c), .. } if *c == group_color(1)))
            .count();
        assert_eq!(squares, 1);

        // Too many values for the shapes: the commonest keep theirs.
        let many: Vec<(String, String)> = ["a1", "a2", "a3", "b1", "b2"]
            .iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), format!("v{i}")))
            .collect();
        let pairs: Vec<(&str, &str)> = many.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let mut data = styled(SampleColors::Group, Some(&pairs));
        data.styling.shape.as_mut().unwrap().values.clear();
        for i in 0..12 {
            data.samples.push(Sample {
                name: format!("c{i}"),
                ..data.samples[i % 5].clone()
            });
            data.styling
                .shape
                .as_mut()
                .unwrap()
                .values
                .insert(format!("c{i}"), format!("w{i:02}"));
        }
        let texts = self::texts(&plot(&data, &two_panels()));
        assert!(texts.iter().any(|t| t.starts_with("Other: ")), "{texts:?}");
        assert!(texts.iter().any(|t| t == "12 values but 10 shapes"));
        // Coloured by group with a shape field, the groups are listed under
        // the name of their field.
        assert!(texts.contains(&"Diet".to_string()));
    }

    #[test]
    fn categories_beyond_the_palette_are_noted() {
        let mut data = data();
        let mut values = std::collections::HashMap::new();
        for i in 0..22 {
            let name = format!("s{i}");
            data.samples.push(Sample {
                name: name.clone(),
                ..data.samples[i % 5].clone()
            });
            values.insert(name, format!("c{i:02}"));
        }
        data.styling.color = SampleColors::Categories {
            label: "Country".to_string(),
            values,
        };
        let texts = texts(&plot(&data, &two_panels()));
        assert!(texts.iter().any(|t| t.contains("22 values but 20 colours")));
    }

    #[test]
    fn degenerate_data() {
        // One axis only: a single direction of variation.
        let line: Vec<Vec<f64>> = (0..5).map(|i| vec![i as f64]).collect();
        let axes = numbered_axes("PC", 3);
        let none = Biplot::default();
        let single = plot_ordination(
            &data(),
            &line,
            &[1.0],
            &axes,
            &[],
            &none,
            &Options::default(),
        );
        assert!(texts(&single)[0].contains("single direction"));
        // Positions for some other samples.
        let wrong = plot_ordination(
            &data(),
            &line[..3],
            &FRACTIONS,
            &axes,
            &[],
            &none,
            &Options::default(),
        );
        assert!(texts(&wrong)[0].contains("does not match"));
        assert!(texts(&plot(
            &data(),
            &Options {
                width: 1.5,
                ..Options::default()
            }
        ))[0]
            .contains("too small"));
    }

    #[test]
    fn explanatory_variables_are_drawn_as_arrows_and_centroids() {
        let biplot = Biplot {
            arrows: vec![BiplotItem {
                name: "pH".to_string(),
                position: vec![0.9, -0.2, 0.0],
                tooltip: "pH".to_string(),
            }],
            centroids: vec![BiplotItem {
                name: "Site A".to_string(),
                position: vec![0.1, 0.05, 0.0],
                tooltip: "Site = A".to_string(),
            }],
        };
        let draw = |show_variables: bool| {
            plot_ordination(
                &data(),
                &positions(&data()),
                &FRACTIONS,
                &numbered_axes("dbRDA", 3),
                &[],
                &biplot,
                &Options {
                    show_variables,
                    // Circles only, to find the samples.
                    unique_shapes: false,
                    ..Options::default()
                },
            )
        };
        let figure = draw(true);
        let names = texts(&figure);
        // Once in each of the three panels.
        assert_eq!(names.iter().filter(|t| *t == "pH").count(), 3);
        assert_eq!(names.iter().filter(|t| *t == "Site A").count(), 3);
        assert!(names.iter().any(|t| t.starts_with("dbRDA1 (")));
        assert!(figure.regions.iter().any(|r| r.tooltip == "Site = A"));
        // The longest arrow reaches most of the way to the right of the
        // main panel: its name is right of every sample drawn there but the
        // rightmost.
        let ph_x = figure
            .shapes
            .iter()
            .find_map(|s| match s {
                Shape::Text { x, text, .. } if text == "pH" => Some(*x),
                _ => None,
            })
            .unwrap();
        let mut main: Vec<f64> = figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Circle { cx, .. } => Some(*cx),
                _ => None,
            })
            .take(5)
            .collect();
        main.sort_by(f64::total_cmp);
        assert!(ph_x > main[3], "{ph_x} vs {main:?}");
        let hidden = texts(&draw(false));
        assert!(!hidden.iter().any(|t| t == "pH" || t == "Site A"));
    }
}
