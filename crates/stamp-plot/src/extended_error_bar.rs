//! The extended error bar plot: for each feature, the proportion in each
//! sample or group as bars, the effect size with its confidence interval, and
//! the p-value.

use stamp_core::stats::results::{
    format_p_value, GroupResults, GroupRow, SampleResults, SampleRow,
};
use stamp_core::{GroupProfile, SampleProfile};

use crate::axis::{LinearScale, Ticks};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::p_values::PValueKind;
use crate::text;

const INCH: f64 = 72.0;

/// One feature (one row of the plot).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Feature name, also used to report which row was clicked.
    pub feature: String,
    /// Bar length for the first and second sample or group.
    pub value1: f64,
    pub value2: f64,
    /// Standard deviations of the bar values, if they are to be drawn.
    pub std_dev1: f64,
    pub std_dev2: f64,
    pub effect_size: f64,
    /// Confidence limits of the effect size.
    pub lower_ci: f64,
    pub upper_ci: f64,
    pub p_value: f64,
    pub p_value_corrected: f64,
    /// Text shown in place of the formatted p-value, such as `< 0.001`.
    pub p_text: Option<String>,
    /// Colours for this row's two bars, overriding the plot's two colours.
    pub colors: Option<(Color, Color)>,
}

/// Everything shown in an extended error bar plot.
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub rows: Vec<Row>,
    /// Names and colours of the two samples or groups.
    pub name1: String,
    pub name2: String,
    pub color1: Color,
    pub color2: Color,
    /// Label under the bars, for example `Proportion (%)`.
    pub bar_label: String,
    /// The bars show counts, labelled as whole numbers, not percentages.
    pub bar_is_count: bool,
    /// Label under the effect sizes.
    pub effect_label: String,
    /// True if the effect size is a ratio (no effect at 1, not 0).
    pub is_ratio: bool,
    /// Coverage of the confidence intervals, for example 0.95.
    pub coverage: f64,
    /// What the corrected p-values of the rows are, which names them.
    pub corrected_kind: PValueKind,
    /// Heading drawn in the top-left corner, such as a feature name.
    pub heading: Option<String>,
}

/// Order of the rows, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortBy {
    /// Smallest p-value first.
    PValues,
    /// Largest effect first.
    EffectSizes,
    /// Alphabetical.
    FeatureLabels,
}

/// Where the legend naming the two samples or groups goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Legend {
    None,
    UpperLeft,
    UpperCenter,
    UpperRight,
    LowerLeft,
    LowerCenter,
    LowerRight,
}

impl Legend {
    fn is_bottom(self) -> bool {
        matches!(
            self,
            Legend::LowerLeft | Legend::LowerCenter | Legend::LowerRight
        )
    }
}

/// Appearance of the plot. Lengths are in inches unless stated.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub row_height: f64,
    pub sort_by: SortBy,
    /// Show the bars of proportions on the left.
    pub show_bars: bool,
    /// Show p-values on the right.
    pub show_p_values: bool,
    /// Show corrected rather than uncorrected p-values.
    pub show_corrected_p_values: bool,
    /// Draw standard deviations on the bars.
    pub show_std_dev: bool,
    /// Size of the caps on standard-deviation bars, in points.
    pub end_cap_size: f64,
    /// Area of the effect-size markers, in square points.
    pub marker_size: f64,
    /// Fixed limits for the effect-size axis; chosen from the data if `None`.
    pub effect_limits: Option<(f64, f64)>,
    pub legend: Legend,
    /// Shorten feature names to this many characters.
    pub truncate_labels: Option<usize>,
    /// Features whose names are drawn in red.
    pub highlighted: Vec<String>,
    pub axes_color: Color,
    /// p-values below ten to this power are shown as a bound.
    pub min_p_value_exponent: i32,
    /// Size of the feature names and their p-values, in points.
    pub label_font_size: f64,
    /// Size of the titles above the intervals and the p-values.
    pub title_font_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            row_height: 0.2,
            sort_by: SortBy::PValues,
            show_bars: true,
            show_p_values: true,
            show_corrected_p_values: true,
            show_std_dev: false,
            end_cap_size: 0.0,
            marker_size: 30.0,
            effect_limits: None,
            legend: Legend::None,
            truncate_labels: Some(50),
            highlighted: Vec::new(),
            axes_color: Color::rgb(0x7f, 0x7f, 0x7f),
            min_p_value_exponent: -15,
            label_font_size: 9.0,
            title_font_size: crate::axes::TITLE_SIZE,
        }
    }
}

/// Default colours of the first and second sample.
pub const SAMPLE1_COLOR: Color = Color::rgb(128, 177, 211);
pub const SAMPLE2_COLOR: Color = Color::rgb(253, 180, 98);

impl Data {
    /// Plot data for two-sample results. `rows` selects the features to show,
    /// typically the output of [`SampleResults::filter`].
    pub fn from_sample_results<'a>(
        profile: &SampleProfile,
        results: &SampleResults,
        rows: impl IntoIterator<Item = &'a SampleRow>,
    ) -> Data {
        let ci = results.analysis.ci_method;
        Data {
            rows: rows
                .into_iter()
                .map(|r| Row {
                    feature: r.feature.clone(),
                    value1: r.rel_freq1,
                    value2: r.rel_freq2,
                    std_dev1: 0.0,
                    std_dev2: 0.0,
                    effect_size: r.effect_size,
                    lower_ci: r.lower_ci,
                    upper_ci: r.upper_ci,
                    p_value: r.p_value,
                    p_value_corrected: r.p_value_corrected,
                    p_text: None,
                    colors: None,
                })
                .collect(),
            name1: profile.profile.sample_names[0].clone(),
            name2: profile.profile.sample_names[1].clone(),
            color1: SAMPLE1_COLOR,
            color2: SAMPLE2_COLOR,
            bar_label: "Proportion (%)".to_string(),
            bar_is_count: false,
            effect_label: ci.plot_label().to_string(),
            is_ratio: ci.is_ratio(),
            coverage: results.analysis.coverage,
            corrected_kind: PValueKind::of(results.correction.method),
            heading: None,
        }
    }

    /// Show the number of sequences in each sample in the bars, instead of
    /// proportions. `rows` must be the rows the data was built from.
    pub fn with_sequence_counts<'a>(
        mut self,
        rows: impl IntoIterator<Item = &'a SampleRow>,
    ) -> Data {
        for (row, source) in self.rows.iter_mut().zip(rows) {
            (row.value1, row.value2) = (source.seq1, source.seq2);
        }
        self.bar_label = "Sequences".to_string();
        self.bar_is_count = true;
        self
    }

    /// Plot data for two-group results.
    pub fn from_group_results<'a>(
        profile: &GroupProfile,
        results: &GroupResults,
        rows: impl IntoIterator<Item = &'a GroupRow>,
    ) -> Data {
        Data {
            rows: rows
                .into_iter()
                .map(|r| Row {
                    feature: r.feature.clone(),
                    value1: r.mean1,
                    value2: r.mean2,
                    std_dev1: r.std_dev1,
                    std_dev2: r.std_dev2,
                    effect_size: r.effect_size,
                    lower_ci: r.lower_ci,
                    upper_ci: r.upper_ci,
                    p_value: r.p_value,
                    p_value_corrected: r.p_value_corrected,
                    p_text: None,
                    colors: None,
                })
                .collect(),
            name1: profile.group_name1.clone(),
            name2: profile.group_name2.clone(),
            color1: SAMPLE1_COLOR,
            color2: SAMPLE2_COLOR,
            bar_label: "Mean proportion (%)".to_string(),
            bar_is_count: false,
            effect_label: "Difference in mean proportions (%)".to_string(),
            is_ratio: false,
            coverage: results.analysis.coverage,
            corrected_kind: PValueKind::of(results.correction.method),
            heading: None,
        }
    }
}

/// A row prepared for drawing.
struct Drawn<'a> {
    row: &'a Row,
    label: String,
    /// Effect size as plotted, and the extent of its error bar.
    effect: f64,
    low: f64,
    high: f64,
    /// True if the feature is more abundant in the second sample or group.
    favours_second: bool,
}

/// Build the figure.
pub fn plot(data: &Data, options: &Options) -> Figure {
    if data.rows.is_empty() {
        return Figure::message("No active features or degenerate plot");
    }

    // Order the rows, top to bottom.
    let mut rows: Vec<&Row> = data.rows.iter().collect();
    match options.sort_by {
        SortBy::PValues => rows.sort_by(|a, b| a.p_value.total_cmp(&b.p_value)),
        SortBy::EffectSizes => {
            // For ratios, 0.5 and 2 are effects of the same size.
            let size = |r: &Row| {
                if data.is_ratio {
                    r.effect_size.ln().abs()
                } else {
                    r.effect_size.abs()
                }
            };
            rows.sort_by(|a, b| size(b).total_cmp(&size(a)));
        }
        SortBy::FeatureLabels => rows.sort_by(|a, b| a.feature.cmp(&b.feature)),
    }

    let drawn: Vec<Drawn<'_>> = rows
        .into_iter()
        .map(|row| {
            let label = match options.truncate_labels {
                Some(length) => text::truncate(&row.feature, length),
                None => row.feature.clone(),
            };
            if data.is_ratio && row.effect_size < 1.0 {
                // Mirror ratios below one so every bar points right.
                Drawn {
                    row,
                    label,
                    effect: 1.0 / row.effect_size,
                    low: 1.0 / row.upper_ci,
                    high: 1.0 / row.lower_ci,
                    favours_second: true,
                }
            } else {
                Drawn {
                    row,
                    label,
                    effect: row.effect_size,
                    low: row.lower_ci,
                    high: row.upper_ci,
                    favours_second: !data.is_ratio && row.effect_size < 0.0,
                }
            }
        })
        .collect();
    let n = drawn.len() as f64;

    // Page layout, in points, with room for the text at its sizes.
    let sizes = crate::axes::fonts();
    let axis_room = crate::axes::x_axis_room();
    let bottom_margin = if options.legend.is_bottom() {
        axis_room + (sizes.legend + 2.5).max(0.16 * INCH)
    } else {
        axis_room
    };
    let top_margin = (options.title_font_size + 9.0).max(0.25 * INCH);
    let plot_height = options.row_height * INCH * n;
    let width = options.width * INCH;
    let height = plot_height + bottom_margin + top_margin;
    let mut figure = Figure::new(width, height);

    let label_width = drawn
        .iter()
        .map(|d| text::width(&d.label, options.label_font_size))
        .fold(0.0, f64::max);
    let plots_left = label_width + 0.1 * INCH;
    let p_value_width = if options.show_p_values { 0.75 } else { 0.1 } * INCH;
    let plots_right = width - p_value_width;
    let (bar_width, gap) = if options.show_bars {
        (1.25 * INCH, 0.25 * INCH)
    } else {
        (0.0, 0.0)
    };
    let effect_left = plots_left + bar_width + gap;
    if plots_right - effect_left < 0.5 * INCH {
        return Figure::message("The plot is too narrow for its labels; increase the figure width");
    }

    let top = top_margin;
    let bottom = top + plot_height;
    // Rows sit at whole numbers from n-1 (top) to 0 (bottom) on an axis from -1 to n.
    let y_scale = LinearScale::new(-1.0, n, bottom, top);
    let row_y = |index: usize| y_scale.map(n - 1.0 - index as f64);
    let unit = (bottom - top) / (n + 1.0);

    let axes = Stroke::solid(options.axes_color, 1.0);
    let outline = Stroke::solid(Color::BLACK, 1.0);
    let stripe = Color::grey(0.9);
    let label_style = TextStyle::new(options.label_font_size);
    let title_style = TextStyle::new(options.title_font_size);
    let tick_length = 4.0;
    let tick_pad = 4.0;

    // Alternate rows are shaded, starting with the bottom one.
    let stripes = |figure: &mut Figure, left: f64, right: f64| {
        for (index, _) in drawn.iter().enumerate() {
            if (drawn.len() - 1 - index).is_multiple_of(2) {
                figure.rect(
                    left,
                    row_y(index) - unit / 2.0,
                    right - left,
                    unit,
                    Some(stripe),
                    None,
                );
            }
        }
    };
    // Bottom axis line with ticks pointing into the plot, and tick labels.
    let x_axis =
        |figure: &mut Figure, left: f64, right: f64, ticks: &[(f64, String)], label: &str| {
            figure.line(&[(left, bottom), (right, bottom)], axes);
            for (x, tick_label) in ticks {
                figure.line(&[(*x, bottom), (*x, bottom - tick_length)], axes);
                figure.text(
                    *x,
                    bottom + tick_pad,
                    tick_label.clone(),
                    crate::axes::tick_style().align(HAlign::Center, VAlign::Top),
                );
            }
            let label_top = bottom + tick_pad + crate::axes::line_height(sizes.tick) + 3.0;
            figure.text(
                (left + right) / 2.0,
                label_top,
                label,
                crate::axes::axis_label_style().align(HAlign::Center, VAlign::Top),
            );
        };

    // Feature names.
    for (index, d) in drawn.iter().enumerate() {
        let color = if options.highlighted.contains(&d.row.feature) {
            Color::RED
        } else {
            Color::BLACK
        };
        figure.text(
            plots_left - tick_pad,
            row_y(index),
            d.label.clone(),
            label_style
                .align(HAlign::Right, VAlign::Middle)
                .color(color),
        );
    }

    // Bars of proportions.
    if options.show_bars {
        let (left, right) = (plots_left, plots_left + bar_width);
        stripes(&mut figure, left, right);
        let largest = drawn
            .iter()
            .map(|d| d.row.value1.max(d.row.value2))
            .fold(0.0, f64::max);
        let x_max = if largest > 0.0 { largest * 1.05 } else { 1.0 };
        let x_scale = LinearScale::new(0.0, x_max, left, right);

        let bar_height = 0.3 * unit;
        for (index, d) in drawn.iter().enumerate() {
            let y = row_y(index);
            let mut bar = |value: f64, std_dev: f64, top: f64, color: Color| {
                let end = x_scale.map(value.max(0.0));
                figure.rect(
                    left,
                    top,
                    end - left,
                    bar_height,
                    Some(color),
                    Some(outline),
                );
                if options.show_std_dev && std_dev > 0.0 {
                    let middle = top + bar_height / 2.0;
                    let (from, to) = (
                        x_scale.map((value - std_dev).max(0.0)),
                        x_scale.map(value + std_dev),
                    );
                    figure.line(&[(from, middle), (to, middle)], outline);
                    if options.end_cap_size > 0.0 {
                        for x in [from, to] {
                            figure.line(
                                &[
                                    (x, middle - options.end_cap_size),
                                    (x, middle + options.end_cap_size),
                                ],
                                outline,
                            );
                        }
                    }
                }
            };
            let (color1, color2) = d.row.colors.unwrap_or((data.color1, data.color2));
            bar(d.row.value1, d.row.std_dev1, y - bar_height, color1);
            bar(d.row.value2, d.row.std_dev2, y, color2);
        }
        let ticks = [
            (left, "0.0".to_string()),
            (
                x_scale.map(largest),
                if data.bar_is_count {
                    format!("{largest:.0}")
                } else {
                    format!("{largest:.1}")
                },
            ),
        ];
        x_axis(&mut figure, left, right, &ticks, &data.bar_label);
    }

    // Effect sizes with confidence intervals.
    let (left, right) = (effect_left, plots_right);
    stripes(&mut figure, left, right);
    let no_effect = if data.is_ratio { 1.0 } else { 0.0 };
    let ticks = match options.effect_limits {
        Some((min, max)) if max > min => Ticks::inside(min, max),
        _ => {
            let finite = |v: f64| v.is_finite();
            let low = drawn
                .iter()
                .map(|d| d.low.min(d.effect))
                .filter(|v| finite(*v))
                .fold(no_effect, f64::min);
            let high = drawn
                .iter()
                .map(|d| d.high.max(d.effect))
                .filter(|v| finite(*v))
                .fold(no_effect, f64::max);
            Ticks::covering(low, high)
        }
    };
    let x_scale = LinearScale::new(ticks.min, ticks.max, left, right);
    let clamp = |x: f64| x.clamp(left, right);

    if (ticks.min..=ticks.max).contains(&no_effect) {
        let x = x_scale.map(no_effect);
        figure.line(
            &[(x, y_scale.map(-1.0)), (x, y_scale.map(n))],
            Stroke::dashed(options.axes_color, 1.0, 6.0, 6.0),
        );
    }

    let radius = options.marker_size.max(0.0).sqrt() / 2.0;
    let cap = 3.0;
    let marker_edge = Stroke::solid(Color::BLACK, 0.5);
    for (index, d) in drawn.iter().enumerate() {
        let y = row_y(index);
        if d.effect.is_finite() {
            if d.low.is_finite() && d.high.is_finite() {
                let (from, to) = (clamp(x_scale.map(d.low)), clamp(x_scale.map(d.high)));
                figure.line(&[(from, y), (to, y)], outline);
                for (x, limit) in [(from, d.low), (to, d.high)] {
                    // No cap where the interval runs off a fixed axis.
                    if (ticks.min..=ticks.max).contains(&limit) {
                        figure.line(&[(x, y - cap), (x, y + cap)], outline);
                    }
                }
            }
            if (ticks.min..=ticks.max).contains(&d.effect) {
                let (color1, color2) = d.row.colors.unwrap_or((data.color1, data.color2));
                let color = if d.favours_second { color2 } else { color1 };
                figure.circle(
                    x_scale.map(d.effect),
                    y,
                    radius,
                    Some(color),
                    Some(marker_edge),
                );
            }
        }

        let tooltip = format!(
            "{}\n{}: {:.3}\n{}: {:.3}\n{}: {:.3} ({:.3} to {:.3})\n{}: {}",
            d.row.feature,
            data.name1,
            d.row.value1,
            data.name2,
            d.row.value2,
            data.effect_label,
            d.row.effect_size,
            d.row.lower_ci,
            d.row.upper_ci,
            p_value_title(data, options),
            p_text(d.row, options),
        );
        figure.region(0.0, y - unit / 2.0, width, unit, &d.row.feature, tooltip);
    }

    let tick_labels: Vec<(f64, String)> = ticks
        .positions
        .iter()
        .map(|&p| x_scale.map(p))
        .zip(ticks.labels())
        .collect();
    x_axis(&mut figure, left, right, &tick_labels, &data.effect_label);

    let title = format!(
        "{}% confidence intervals",
        trim_number(data.coverage * 100.0)
    );
    figure.text(
        (left + right) / 2.0,
        top - 5.0,
        title,
        title_style.align(HAlign::Center, VAlign::Baseline),
    );

    // p-values.
    if options.show_p_values {
        for (index, d) in drawn.iter().enumerate() {
            figure.text(
                right + tick_pad,
                row_y(index),
                p_text(d.row, options),
                label_style.align(HAlign::Left, VAlign::Middle),
            );
        }
        // Centre the title on the rows, but keep it on the page when there
        // are only a few of them.
        let title = p_value_title(data, options);
        let half = text::width(&title, options.title_font_size) / 2.0 + 2.0;
        let centre = ((top + bottom) / 2.0)
            .max(half)
            .min((height - half).max(half));
        figure.text(
            width - 1.5,
            centre,
            title,
            title_style
                .align(HAlign::Center, VAlign::Bottom)
                .rotated(90.0),
        );
    }

    if let Some(heading) = &data.heading {
        figure.text(
            2.0,
            3.0,
            heading.clone(),
            title_style.align(HAlign::Left, VAlign::Top),
        );
    }
    draw_legend(&mut figure, data, options);
    figure
}

/// The p-value text of a row.
fn p_text(row: &Row, options: &Options) -> String {
    if let Some(text) = &row.p_text {
        return text.clone();
    }
    let p = if options.show_corrected_p_values {
        row.p_value_corrected
    } else {
        row.p_value
    };
    format_p_value(p, options.min_p_value_exponent)
}

fn p_value_title(data: &Data, options: &Options) -> String {
    if options.show_corrected_p_values {
        data.corrected_kind
    } else {
        PValueKind::Uncorrected
    }
    .label()
    .to_string()
}

/// `95` for 95.0, `99.9` for 99.9.
fn trim_number(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    rounded.to_string()
}

fn draw_legend(figure: &mut Figure, data: &Data, options: &Options) {
    if options.legend == Legend::None {
        return;
    }
    let size = crate::axes::fonts().legend;
    let style = crate::axes::legend_style().align(HAlign::Left, VAlign::Middle);
    let (swatch, pad, spacing) = (size, 4.0, 14.0);
    let entry = |name: &str| swatch + pad + text::width(name, size);
    let total = entry(&data.name1) + spacing + entry(&data.name2);

    let margin = 6.0;
    let x = match options.legend {
        Legend::UpperLeft | Legend::LowerLeft => margin,
        Legend::UpperCenter | Legend::LowerCenter => (figure.width - total) / 2.0,
        _ => figure.width - total - margin,
    };
    let y = if options.legend.is_bottom() {
        figure.height - margin - swatch / 2.0
    } else {
        margin + swatch / 2.0
    };

    let mut cursor = x;
    for (name, color) in [(&data.name1, data.color1), (&data.name2, data.color2)] {
        figure.rect(
            cursor,
            y - swatch / 2.0,
            swatch,
            swatch,
            Some(color),
            Some(Stroke::solid(Color::BLACK, 0.5)),
        );
        figure.text(cursor + swatch + pad, y, name.clone(), style);
        cursor += entry(name) + spacing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn row(feature: &str, v1: f64, v2: f64, effect: f64, half_width: f64, p: f64) -> Row {
        Row {
            feature: feature.to_string(),
            value1: v1,
            value2: v2,
            std_dev1: 1.0,
            std_dev2: 0.5,
            effect_size: effect,
            lower_ci: effect - half_width,
            upper_ci: effect + half_width,
            p_value: p,
            p_value_corrected: (p * 3.0).min(1.0),
            p_text: None,
            colors: None,
        }
    }

    fn data() -> Data {
        Data {
            rows: vec![
                row("Beta", 10.0, 4.0, 6.0, 2.0, 0.02),
                row("Alpha", 3.0, 9.0, -6.0, 1.0, 1e-20),
                row("Gamma", 5.0, 5.5, -0.5, 3.0, 0.4),
            ],
            name1: "Red".to_string(),
            name2: "Black".to_string(),
            color1: SAMPLE1_COLOR,
            color2: SAMPLE2_COLOR,
            bar_label: "Proportion (%)".to_string(),
            bar_is_count: false,
            effect_label: "Difference between proportions (%)".to_string(),
            is_ratio: false,
            coverage: 0.95,
            corrected_kind: PValueKind::Corrected,
            heading: None,
        }
    }

    fn texts(figure: &Figure) -> Vec<&str> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn circles(figure: &Figure) -> Vec<(f64, f64, Color)> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Circle { cx, cy, fill, .. } => Some((*cx, *cy, fill.unwrap())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn size_follows_the_number_of_rows() {
        let figure = plot(&data(), &Options::default());
        assert!((figure.width - 7.0 * 72.0).abs() < 1e-9);
        assert!((figure.height - (3.0 * 0.2 + 0.4 + 0.25) * 72.0).abs() < 1e-9);
        let taller = plot(
            &data(),
            &Options {
                row_height: 0.5,
                legend: Legend::LowerCenter,
                ..Options::default()
            },
        );
        assert!((taller.height - (3.0 * 0.5 + 0.56 + 0.25) * 72.0).abs() < 1e-9);
    }

    #[test]
    fn labels_and_titles() {
        let figure = plot(&data(), &Options::default());
        let texts = texts(&figure);
        for expected in [
            "Alpha",
            "Beta",
            "Gamma",
            "95% confidence intervals",
            "Proportion (%)",
            "Difference between proportions (%)",
            "Corrected p-value",
            "< 1e-15",
            "0.060",
            "0.0",
            "10.0",
            "\u{2212}8",
        ] {
            assert!(
                texts.contains(&expected),
                "missing {expected:?} in {texts:?}"
            );
        }

        // Storey's q-values, and no correction at all.
        let mut storey = data();
        storey.corrected_kind = PValueKind::QValue;
        assert!(texts_of(&plot(&storey, &Options::default())).contains(&"q-value".to_string()));
        let mut none = data();
        none.corrected_kind = PValueKind::Uncorrected;
        assert!(texts_of(&plot(&none, &Options::default()))
            .contains(&"Uncorrected p-value".to_string()));
        // Uncorrected values shown although a correction was applied.
        let uncorrected = Options {
            show_corrected_p_values: false,
            ..Options::default()
        };
        let texts = texts_of(&plot(&storey, &uncorrected));
        assert!(
            texts.contains(&"Uncorrected p-value".to_string())
                && texts.contains(&"0.020".to_string())
        );
    }

    fn texts_of(figure: &Figure) -> Vec<String> {
        texts(figure).into_iter().map(str::to_string).collect()
    }

    #[test]
    fn rows_are_ordered_top_to_bottom() {
        let order = |sort_by| {
            let figure = plot(
                &data(),
                &Options {
                    sort_by,
                    ..Options::default()
                },
            );
            figure
                .regions
                .iter()
                .map(|r| r.item.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(order(SortBy::PValues), ["Alpha", "Beta", "Gamma"]);
        assert_eq!(order(SortBy::FeatureLabels), ["Alpha", "Beta", "Gamma"]);
        assert_eq!(order(SortBy::EffectSizes)[2], "Gamma");

        let figure = plot(&data(), &Options::default());
        assert!(figure.regions[0].y < figure.regions[1].y);
        let tooltip = &figure
            .region_at(100.0, figure.regions[0].y + 1.0)
            .unwrap()
            .tooltip;
        assert!(tooltip.starts_with("Alpha\nRed: 3.000\nBlack: 9.000"));
    }

    #[test]
    fn markers_take_the_colour_of_the_more_abundant_side() {
        let figure = plot(&data(), &Options::default());
        let circles = circles(&figure);
        assert_eq!(circles.len(), 3);
        // Alpha (top) favours the second sample; Beta the first.
        assert_eq!(circles[0].2, SAMPLE2_COLOR);
        assert_eq!(circles[1].2, SAMPLE1_COLOR);
        // Negative effects are drawn to the left of positive ones.
        assert!(circles[0].0 < circles[1].0);
        assert!(circles[0].1 < circles[1].1);
    }

    #[test]
    fn ratios_are_mirrored_about_one() {
        let mut ratio = data();
        ratio.is_ratio = true;
        ratio.rows = vec![
            Row {
                effect_size: 2.0,
                lower_ci: 1.5,
                upper_ci: 3.0,
                ..row("Up", 4.0, 2.0, 0.0, 0.0, 0.01)
            },
            Row {
                effect_size: 0.5,
                lower_ci: 1.0 / 3.0,
                upper_ci: 1.0 / 1.5,
                ..row("Down", 2.0, 4.0, 0.0, 0.0, 0.02)
            },
        ];
        let circles = circles(&plot(&ratio, &Options::default()));
        // A ratio of 0.5 is drawn at 2, in the second sample's colour.
        assert!((circles[0].0 - circles[1].0).abs() < 1e-9);
        assert_eq!((circles[0].2, circles[1].2), (SAMPLE1_COLOR, SAMPLE2_COLOR));
    }

    #[test]
    fn optional_parts() {
        let count = |options: &Options| plot(&data(), options).shapes.len();
        let full = count(&Options::default());
        assert!(
            count(&Options {
                show_bars: false,
                ..Options::default()
            }) < full
        );
        assert!(
            count(&Options {
                show_p_values: false,
                ..Options::default()
            }) < full
        );
        assert!(
            count(&Options {
                show_std_dev: true,
                end_cap_size: 2.0,
                ..Options::default()
            }) > full
        );
        let with_legend = plot(
            &data(),
            &Options {
                legend: Legend::UpperLeft,
                ..Options::default()
            },
        );
        assert!(texts(&with_legend).contains(&"Red") && texts(&with_legend).contains(&"Black"));

        let highlighted = plot(
            &data(),
            &Options {
                highlighted: vec!["Beta".to_string()],
                ..Options::default()
            },
        );
        let red_text = highlighted.shapes.iter().any(|s| matches!(s, Shape::Text { text, style, .. } if text == "Beta" && style.color == Color::RED));
        assert!(red_text);
    }

    #[test]
    fn fixed_limits_clip_markers_outside_them() {
        let options = Options {
            effect_limits: Some((0.0, 10.0)),
            ..Options::default()
        };
        let figure = plot(&data(), &options);
        // Only Beta's effect (6) lies within 0..10.
        assert_eq!(circles(&figure).len(), 1);
        for shape in &figure.shapes {
            if let Shape::Line { points, .. } = shape {
                assert!(points.iter().all(|(x, _)| *x >= 0.0 && *x <= figure.width));
            }
        }
    }

    #[test]
    fn degenerate_inputs_give_a_message() {
        let mut empty = data();
        empty.rows.clear();
        assert_eq!(
            texts(&plot(&empty, &Options::default())),
            ["No active features or degenerate plot"]
        );

        let narrow = plot(
            &data(),
            &Options {
                width: 1.0,
                ..Options::default()
            },
        );
        assert!(texts(&narrow)[0].contains("too narrow"));

        // Undefined statistics do not break the plot.
        let mut nan = data();
        nan.rows[0].lower_ci = f64::NAN;
        nan.rows[1].effect_size = f64::NAN;
        nan.rows[2].p_value_corrected = f64::NAN;
        let figure = plot(&nan, &Options::default());
        assert_eq!(circles(&figure).len(), 2);
        assert!(texts(&figure).contains(&"nan"));
    }

    #[test]
    fn long_names_are_truncated() {
        let mut long = data();
        long.rows[0].feature = "x".repeat(80);
        let figure = plot(&long, &Options::default());
        let label = format!("{}...", "x".repeat(50));
        assert!(texts(&figure).contains(&label.as_str()));
        // The region still reports the full name.
        assert!(figure.regions.iter().any(|r| r.item.len() == 80));
        let full = plot(
            &long,
            &Options {
                truncate_labels: None,
                width: 10.0,
                ..Options::default()
            },
        );
        assert!(texts(&full).iter().any(|t| t.len() == 80));
    }
}
