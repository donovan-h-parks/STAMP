//! What the samples of an ordination plot (PCA, PCoA, NMDS) are coloured
//! and shaped by: their group, another metadata field, a number from the
//! metadata, or the relative abundance of a feature.
//!
//! The choice is made in [`ordination_plot::Options`](crate::ordination_plot::Options)
//! (`color_by`, `shape_by`); [`resolve`] looks the values up in the
//! metadata and the profile and gives the [`Styling`] that the plot draws.

use std::collections::{BTreeMap, HashMap};

use stamp_core::{Metadata, Profile};

use crate::axes::{fonts, legend_style, marker_styled, tick_style, Marker};
use crate::axis::{log_ticks, Ticks};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::heatmap::ColorMap;
use crate::text;

/// What the user asked the samples to be coloured by.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ColorSource {
    /// The groups of the analysis.
    #[default]
    Group,
    /// A metadata field: a colour scale if every value given is a number,
    /// otherwise a colour for each value.
    Field(String),
    /// A metadata field, a colour for each value even if they are numbers.
    Category(String),
    /// The relative abundance (%) of a feature in each sample.
    Feature(String),
    /// The relative abundance of the feature selected in the feature table.
    SelectedFeature,
}

impl ColorSource {
    /// Read the command line form: `group`, `field:NAME`,
    /// `category:NAME` or `feature:NAME`.
    pub fn parse(text: &str) -> Result<ColorSource, String> {
        if text.eq_ignore_ascii_case("group") {
            return Ok(ColorSource::Group);
        }
        let (kind, name) = text.split_once(':').ok_or_else(|| {
            format!("'{text}': expected group, field:NAME, category:NAME or feature:NAME")
        })?;
        if name.is_empty() {
            return Err(format!("'{text}': the name is missing"));
        }
        match kind.to_ascii_lowercase().as_str() {
            "field" => Ok(ColorSource::Field(name.to_string())),
            "category" => Ok(ColorSource::Category(name.to_string())),
            "feature" => Ok(ColorSource::Feature(name.to_string())),
            _ => Err(format!(
                "'{text}': expected group, field:NAME, category:NAME or feature:NAME"
            )),
        }
    }
}

/// How numbers are mapped onto the colour scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ColorScale {
    #[default]
    Linear,
    /// Logarithms of the values; zeros (and negative values) are drawn at
    /// a floor of half the smallest positive value.
    Log10,
}

impl ColorScale {
    pub const ALL: [ColorScale; 2] = [ColorScale::Linear, ColorScale::Log10];

    pub fn name(self) -> &'static str {
        match self {
            ColorScale::Linear => "Linear",
            ColorScale::Log10 => "Logarithmic (log10)",
        }
    }
}

/// The order samples coloured by a number are drawn in, later ones on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DrawOrder {
    /// Highest values last, so that they are not hidden.
    #[default]
    HighOnTop,
    LowOnTop,
    /// In the order of the samples.
    AsGiven,
}

impl DrawOrder {
    pub const ALL: [DrawOrder; 3] = [
        DrawOrder::HighOnTop,
        DrawOrder::LowOnTop,
        DrawOrder::AsGiven,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DrawOrder::HighOnTop => "High on top",
            DrawOrder::LowOnTop => "Low on top",
            DrawOrder::AsGiven => "Sample order",
        }
    }
}

/// The colour of each sample, as found by [`resolve`].
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SampleColors {
    /// By the group of each sample.
    #[default]
    Group,
    /// By a category; samples missing from `values` have none.
    Categories {
        label: String,
        values: HashMap<String, String>,
    },
    /// By a number on a colour scale; samples missing from `values` have
    /// none.
    Values {
        label: String,
        values: HashMap<String, f64>,
    },
}

/// The marker shape of each sample by a category.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleShapes {
    pub label: String,
    /// The category of each sample; samples missing from it have none.
    pub values: HashMap<String, String>,
}

/// How the samples of an ordination plot are coloured and shaped.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Styling {
    pub color: SampleColors,
    pub shape: Option<SampleShapes>,
    /// Name of the field that defines the groups of the analysis, used in
    /// the legend and notes (`None`: "Group").
    pub group_label: Option<String>,
}

impl Styling {
    /// True if anything but the groups decides colours or shapes, so that
    /// the plot needs the fuller legend.
    pub fn is_extended(&self) -> bool {
        self.color != SampleColors::Group || self.shape.is_some()
    }

    /// The name given to the groups of the analysis.
    pub fn group_label(&self) -> &str {
        self.group_label.as_deref().unwrap_or("Group")
    }
}

/// True if a metadata value stands for no value: empty, `NA`, `N/A`,
/// `NaN`, `null` or `None` (in any case).
pub fn is_missing(value: &str) -> bool {
    let value = value.trim();
    value.is_empty()
        || ["na", "n/a", "nan", "null", "none"]
            .iter()
            .any(|m| value.eq_ignore_ascii_case(m))
}

/// The number a metadata value holds, if any.
fn number(value: &str) -> Option<f64> {
    if is_missing(value) {
        return None;
    }
    value.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// True if a metadata field holds numbers: at least three of its values
/// are numbers, and they make up at least 90% of the values given (not
/// missing by [`is_missing`]). Other values (an occasional "unknown") are
/// then treated as missing.
pub fn is_numeric_field(metadata: &Metadata, field: &str) -> bool {
    if !metadata.fields().iter().any(|f| f == field) {
        return false;
    }
    let (mut given, mut numbers) = (0usize, 0usize);
    for sample in metadata.samples() {
        let Some(value) = metadata.value(sample, field) else {
            continue;
        };
        if is_missing(value) {
            continue;
        }
        given += 1;
        if number(value).is_some() {
            numbers += 1;
        }
    }
    numbers >= 3 && numbers as f64 >= 0.9 * given as f64
}

/// The values of a categorical field for each sample that has one.
fn categories(metadata: &Metadata, field: &str) -> Result<HashMap<String, String>, String> {
    if !metadata.fields().iter().any(|f| f == field) {
        return Err(format!("unknown metadata field '{field}'"));
    }
    Ok(metadata
        .samples()
        .iter()
        .filter_map(|sample| {
            let value = metadata.value(sample, field)?;
            (!is_missing(value)).then(|| (sample.clone(), value.trim().to_string()))
        })
        .collect())
}

/// Find the colours and shapes asked for: `color` and `shape` (a metadata
/// field) from `metadata`, a feature's relative abundance from `profile`
/// (its percentage of the sample, as the bar and box plots show it), and
/// [`ColorSource::SelectedFeature`] from `selected_feature`. `group_field`
/// names the groups of the analysis.
pub fn resolve(
    color: &ColorSource,
    shape: Option<&str>,
    metadata: Option<&Metadata>,
    profile: Option<&Profile>,
    selected_feature: Option<&str>,
    group_field: Option<&str>,
) -> Result<Styling, String> {
    let need_metadata = || metadata.ok_or_else(|| "no metadata loaded".to_string());
    let feature_values = |name: &str| -> Result<SampleColors, String> {
        let profile = profile.ok_or_else(|| "no profile loaded".to_string())?;
        let feature = profile
            .feature(name)
            .ok_or_else(|| format!("unknown feature '{name}' at this level of the profile"))?;
        let values = profile
            .sample_names
            .iter()
            .cloned()
            .zip(profile.percentages(feature))
            .collect();
        Ok(SampleColors::Values {
            label: format!("{name} (%)"),
            values,
        })
    };
    let color = match color {
        ColorSource::Group => SampleColors::Group,
        ColorSource::Field(field) => {
            let metadata = need_metadata()?;
            if is_numeric_field(metadata, field) {
                SampleColors::Values {
                    label: field.clone(),
                    values: metadata
                        .samples()
                        .iter()
                        .filter_map(|s| {
                            let value = number(metadata.value(s, field)?)?;
                            Some((s.clone(), value))
                        })
                        .collect(),
                }
            } else {
                SampleColors::Categories {
                    label: field.clone(),
                    values: categories(metadata, field)?,
                }
            }
        }
        ColorSource::Category(field) => SampleColors::Categories {
            label: field.clone(),
            values: categories(need_metadata()?, field)?,
        },
        ColorSource::Feature(name) => feature_values(name)?,
        ColorSource::SelectedFeature => match selected_feature {
            Some(name) => feature_values(name)?,
            None => return Err("Select a feature in the feature table to colour by".to_string()),
        },
    };
    let shape = match shape {
        Some(field) => Some(SampleShapes {
            label: field.to_string(),
            values: categories(need_metadata()?, field)?,
        }),
        None => None,
    };
    Ok(Styling {
        color,
        shape,
        group_label: group_field.map(str::to_string),
    })
}

/// The number of shapes categories get; the rest share an "other" shape.
pub const MAX_SHAPES: usize = crate::axes::Marker::ALL.len();

/// The categories of the samples that have one, sorted, each with the
/// number of samples in it.
pub(crate) fn category_counts<'a>(
    samples: impl Iterator<Item = &'a str>,
    values: &HashMap<String, String>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for sample in samples {
        if let Some(value) = values.get(sample) {
            *counts.entry(value.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// A line of the legend beside a plot whose markers are coloured and shaped
/// by a [`Styling`]: the ordination plots and the map.
pub(crate) enum LegendItem {
    Title(String),
    /// A marker and its name.
    Entry {
        shape: Marker,
        fill: Option<Color>,
        edge: Color,
        label: String,
    },
    /// A circle of the given area (square points) and its name, for the
    /// sizes of the markers of a map.
    Size {
        area: f64,
        label: String,
    },
    /// A vertical colour scale from `low` (bottom) to `high`, with ticks.
    Bar {
        map: ColorMap,
        low: f64,
        high: f64,
        ticks: Vec<(f64, String)>,
    },
    /// A smaller line of text.
    Note(String),
    Gap,
}

const BAR_HEIGHT: f64 = 110.0;
const BAR_WIDTH: f64 = 10.0;
const ENTRY_MARKER: f64 = 14.0;
/// The edge of the markers of samples without a value.
pub(crate) const NO_VALUE_COLOR: Color = Color::rgb(150, 150, 150);
/// The fill of the markers in the legend of shapes.
pub(crate) const SHAPE_FILL: Color = Color::rgb(210, 210, 210);

impl LegendItem {
    fn line() -> f64 {
        fonts().legend + 4.0
    }

    /// The width and height the item takes.
    pub(crate) fn size(&self) -> (f64, f64) {
        let size = fonts().legend;
        match self {
            LegendItem::Title(t) => (text::width(t, size), LegendItem::line()),
            LegendItem::Entry { label, .. } => (
                ENTRY_MARKER + 5.0 + text::width(label, size),
                LegendItem::line(),
            ),
            LegendItem::Size { area, label } => {
                let diameter = area.max(0.0).sqrt();
                (
                    diameter.max(ENTRY_MARKER) + 5.0 + text::width(label, size),
                    (diameter + 3.0).max(LegendItem::line()),
                )
            }
            LegendItem::Bar { ticks, .. } => {
                let widest = ticks
                    .iter()
                    .map(|(_, t)| text::width(t, fonts().tick))
                    .fold(0.0, f64::max);
                (BAR_WIDTH + 6.0 + widest, BAR_HEIGHT + 8.0)
            }
            LegendItem::Note(t) => (text::width(t, size - 1.0), LegendItem::line() - 1.0),
            LegendItem::Gap => (0.0, 6.0),
        }
    }

    /// Draw the item with its top-left corner at `x`, `y`; entries get
    /// markers of about `marker_area` square points.
    pub(crate) fn draw(&self, figure: &mut Figure, x: f64, y: f64, marker_area: f64) {
        let size = fonts().legend;
        let (_, height) = self.size();
        let middle = y + height / 2.0;
        match self {
            LegendItem::Title(t) => figure.text(
                x,
                middle,
                t.clone(),
                legend_style().align(HAlign::Left, VAlign::Middle),
            ),
            LegendItem::Entry {
                shape,
                fill,
                edge,
                label,
            } => {
                let area = marker_area.clamp(16.0, 64.0) * if fill.is_some() { 1.0 } else { 0.5 };
                marker_styled(
                    figure,
                    *shape,
                    x + ENTRY_MARKER / 2.0,
                    middle,
                    area,
                    *fill,
                    Stroke::solid(*edge, 0.75),
                );
                figure.text(
                    x + ENTRY_MARKER + 5.0,
                    middle,
                    label.clone(),
                    legend_style().align(HAlign::Left, VAlign::Middle),
                );
            }
            LegendItem::Size { area, label } => {
                let diameter = area.max(0.0).sqrt();
                let room = diameter.max(ENTRY_MARKER);
                figure.circle(
                    x + room / 2.0,
                    middle,
                    diameter / 2.0,
                    Some(SHAPE_FILL),
                    Some(Stroke::solid(Color::BLACK, 0.75)),
                );
                figure.text(
                    x + room + 5.0,
                    middle,
                    label.clone(),
                    legend_style().align(HAlign::Left, VAlign::Middle),
                );
            }
            LegendItem::Bar {
                map,
                low,
                high,
                ticks,
            } => {
                let top = y + 4.0;
                let steps = 60;
                for i in 0..steps {
                    let fraction = (i as f64 + 0.5) / steps as f64;
                    let step = BAR_HEIGHT / steps as f64;
                    figure.rect(
                        x,
                        top + BAR_HEIGHT - (i + 1) as f64 * step,
                        BAR_WIDTH,
                        step + 0.3,
                        Some(map.color(fraction)),
                        None,
                    );
                }
                figure.rect(
                    x,
                    top,
                    BAR_WIDTH,
                    BAR_HEIGHT,
                    None,
                    Some(Stroke::solid(Color::BLACK, 0.5)),
                );
                let span = if high > low { high - low } else { 1.0 };
                for (value, label) in ticks {
                    let ty = top + BAR_HEIGHT - BAR_HEIGHT * (value - low) / span;
                    figure.line(
                        &[(x + BAR_WIDTH, ty), (x + BAR_WIDTH + 2.5, ty)],
                        Stroke::solid(Color::BLACK, 0.5),
                    );
                    figure.text(
                        x + BAR_WIDTH + 4.0,
                        ty,
                        label.clone(),
                        tick_style().align(HAlign::Left, VAlign::Middle),
                    );
                }
            }
            LegendItem::Note(t) => figure.text(
                x,
                middle,
                t.clone(),
                TextStyle::new(size - 1.0).align(HAlign::Left, VAlign::Middle),
            ),
            LegendItem::Gap => {}
        }
    }
}

/// The width and height of a column of legend items.
pub(crate) fn legend_size(items: &[LegendItem]) -> (f64, f64) {
    items.iter().fold((0.0f64, 0.0f64), |(w, h), item| {
        let (iw, ih) = item.size();
        (w.max(iw), h + ih)
    })
}

/// Draw a column of legend items from `x`, `y` down.
pub(crate) fn draw_legend(
    figure: &mut Figure,
    items: &[LegendItem],
    x: f64,
    mut y: f64,
    marker_area: f64,
) {
    for item in items {
        item.draw(figure, x, y, marker_area);
        y += item.size().1;
    }
}

/// A number written with four significant digits at most.
pub(crate) fn short_number(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return format!("{value}");
    }
    let magnitude = value.abs().log10().floor() as i32;
    if !(-4..6).contains(&magnitude) {
        return format!("{value:.3e}");
    }
    let decimals = (3 - magnitude).max(0) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

/// How the numbers that colour markers are placed on the colour scale.
pub(crate) struct ValueScale {
    log: bool,
    /// Where zeros are drawn on a logarithmic scale.
    floor: f64,
    /// The ends of the scale, after the transform.
    low: f64,
    high: f64,
    /// How many of the values are zero or below.
    zeros: usize,
    empty: bool,
}

impl ValueScale {
    /// The scale for the values `present`.
    pub(crate) fn new(present: &[f64], scale: ColorScale) -> ValueScale {
        let smallest_positive = present
            .iter()
            .copied()
            .filter(|v| *v > 0.0)
            .fold(f64::INFINITY, f64::min);
        let log = scale == ColorScale::Log10 && smallest_positive.is_finite();
        let mut scale = ValueScale {
            log,
            floor: smallest_positive / 2.0,
            low: 0.0,
            high: 0.0,
            zeros: present.iter().filter(|&&v| v <= 0.0).count(),
            empty: present.is_empty(),
        };
        scale.low = present
            .iter()
            .map(|&v| scale.transform(v))
            .fold(f64::INFINITY, f64::min);
        scale.high = present
            .iter()
            .map(|&v| scale.transform(v))
            .fold(f64::NEG_INFINITY, f64::max);
        scale
    }

    fn transform(&self, v: f64) -> f64 {
        if self.log {
            v.max(self.floor).log10()
        } else {
            v
        }
    }

    /// Where a value falls on the scale, from 0 to 1.
    pub(crate) fn fraction(&self, v: f64) -> f64 {
        let span = self.high - self.low;
        if span > 0.0 {
            (self.transform(v) - self.low) / span
        } else {
            0.5
        }
    }

    /// The legend of the scale: its title, a colour bar with ticks and
    /// notes on how it is drawn.
    pub(crate) fn legend(&self, label: &str, map: ColorMap) -> Vec<LegendItem> {
        let (log, low, high, zeros, floor) =
            (self.log, self.low, self.high, self.zeros, self.floor);
        let span = high - low;
        let mut items = vec![LegendItem::Title(label.to_string())];
        if log {
            items.push(LegendItem::Note("Logarithmic scale".to_string()));
        }
        if self.empty {
            items.push(LegendItem::Note("No sample has a value".to_string()));
            return items;
        }
        let ticks: Vec<(f64, String)> = if span <= 0.0 {
            vec![(low, short_number(if log { 10f64.powf(low) } else { low }))]
        } else if log {
            let mut ticks: Vec<(f64, String)> = log_ticks(low.floor() as i32, high.ceil() as i32)
                .into_iter()
                .filter(|(t, _)| *t >= low - 1e-9 && *t <= high + 1e-9)
                .collect();
            if ticks.len() < 2 {
                ticks = vec![
                    (low, short_number(10f64.powf(low))),
                    (high, short_number(10f64.powf(high))),
                ];
            }
            if zeros > 0 {
                // The floor stands for zero, with room for its label.
                let room = (fonts().tick + 2.0) / BAR_HEIGHT * span;
                ticks.retain(|(t, _)| *t > low + room);
                ticks.insert(0, (low, "0".to_string()));
            }
            ticks
        } else {
            let t = Ticks::inside(low, high);
            t.positions.iter().copied().zip(t.labels()).collect()
        };
        items.push(LegendItem::Bar {
            map,
            low: if span > 0.0 { low } else { low - 1.0 },
            high: if span > 0.0 { high } else { high + 1.0 },
            ticks,
        });
        if log && zeros > 0 {
            items.push(LegendItem::Note(format!(
                "{zeros} zero{} drawn at {}",
                if zeros == 1 { "" } else { "s" },
                short_number(floor)
            )));
        }
        items
    }
}

/// Which marker shape each category of a field gets: the most common keep
/// their own shapes when there are more categories than shapes, and the
/// rest share one.
pub(crate) struct ShapePlan {
    /// Categories with a shape of their own, in the order of
    /// [`Marker::ALL`].
    pub kept: Vec<String>,
    /// The shape shared by the other categories.
    pub other: Marker,
    /// The shape of samples without a value.
    pub none: Marker,
    pub lumped: bool,
}

impl ShapePlan {
    /// The plan for categories with these counts, when `missing` samples
    /// have no value.
    pub(crate) fn new(counts: &BTreeMap<String, usize>, missing: usize) -> ShapePlan {
        let room = MAX_SHAPES - usize::from(missing > 0);
        let mut kept: Vec<String> = counts.keys().cloned().collect();
        let lumped = kept.len() > room;
        if lumped {
            kept.sort_by(|a, b| counts[b].cmp(&counts[a]).then(a.cmp(b)));
            kept.truncate(room - 1);
            kept.sort();
        }
        // Only used when categories are lumped; with exactly as many
        // categories as shapes every shape is taken.
        let other = Marker::ALL[kept.len().min(MAX_SHAPES - 1)];
        let none = Marker::ALL[(kept.len() + usize::from(lumped)).min(MAX_SHAPES - 1)];
        ShapePlan {
            kept,
            other,
            none,
            lumped,
        }
    }

    /// The shape of a category (`None`: no value).
    pub(crate) fn shape(&self, value: Option<&String>) -> Marker {
        match value {
            Some(v) => self
                .kept
                .iter()
                .position(|k| k == v)
                .map_or(self.other, |p| Marker::ALL[p]),
            None => self.none,
        }
    }

    /// The legend of the shapes, with the number of samples (or what
    /// `counts` counts) of each.
    pub(crate) fn legend(
        &self,
        label: &str,
        counts: &BTreeMap<String, usize>,
        missing: usize,
    ) -> Vec<LegendItem> {
        let mut items = vec![LegendItem::Title(label.to_string())];
        for (i, category) in self.kept.iter().enumerate() {
            items.push(LegendItem::Entry {
                shape: Marker::ALL[i],
                fill: Some(SHAPE_FILL),
                edge: Color::BLACK,
                label: format!("{category} ({})", counts[category]),
            });
        }
        if self.lumped {
            let others: Vec<&String> = counts.keys().filter(|k| !self.kept.contains(k)).collect();
            let samples: usize = others.iter().map(|k| counts[*k]).sum();
            items.push(LegendItem::Entry {
                shape: self.other,
                fill: Some(SHAPE_FILL),
                edge: Color::BLACK,
                label: format!("Other: {} values ({samples})", others.len()),
            });
            items.push(LegendItem::Note(format!(
                "{} values but {MAX_SHAPES} shapes",
                counts.len()
            )));
        }
        if missing > 0 {
            items.push(LegendItem::Entry {
                shape: self.none,
                fill: Some(SHAPE_FILL),
                edge: Color::BLACK,
                label: format!("No value ({missing})"),
            });
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stamp_core::ProfileTree;

    fn metadata() -> (Metadata, ProfileTree) {
        let profile = "Level1\tLevel2\ts1\ts2\ts3\ts4\ts5\n\
                       A\ta1\t10\t0\t5\t5\t1\n\
                       A\ta2\t10\t10\t5\t15\t1\n\
                       B\tb1\t0\t10\t10\t0\t1\n";
        let tree = ProfileTree::read(profile).unwrap();
        let metadata = "Sample\tGroup\tDepth\tSite\tYear\n\
                        s1\tx\t1.5\tNorth\t2001\n\
                        s2\tx\tNA\tSouth\t2002\n\
                        s3\ty\t\tNA\t2001\n\
                        s4\ty\t4\tNorth\t2003\n\
                        s5\ty\t2\tunknown\t2003\n";
        let (metadata, _) = Metadata::read(metadata, &tree).unwrap();
        (metadata, tree)
    }

    #[test]
    fn sources_are_read_from_the_command_line() {
        assert_eq!(ColorSource::parse("group"), Ok(ColorSource::Group));
        assert_eq!(
            ColorSource::parse("field:Lat:itude"),
            Ok(ColorSource::Field("Lat:itude".to_string()))
        );
        assert_eq!(
            ColorSource::parse("feature:Equivita"),
            Ok(ColorSource::Feature("Equivita".to_string()))
        );
        assert_eq!(
            ColorSource::parse("category:Year"),
            Ok(ColorSource::Category("Year".to_string()))
        );
        assert!(ColorSource::parse("Latitude").is_err());
        assert!(ColorSource::parse("field:").is_err());
        assert!(ColorSource::parse("colour:x").is_err());
    }

    #[test]
    fn as_many_categories_as_shapes_each_get_their_own() {
        // Exactly MAX_SHAPES categories used to index past the shapes.
        let counts: BTreeMap<String, usize> =
            (0..MAX_SHAPES).map(|i| (format!("c{i:02}"), 1)).collect();
        let plan = ShapePlan::new(&counts, 0);
        assert!(!plan.lumped);
        assert_eq!(plan.kept.len(), MAX_SHAPES);
        let shapes: Vec<Marker> = counts.keys().map(|k| plan.shape(Some(k))).collect();
        assert_eq!(shapes, Marker::ALL.to_vec());
        assert_eq!(plan.legend("Field", &counts, 0).len(), MAX_SHAPES + 1);
        // One more, or a missing value, lumps the rarest together.
        let plan = ShapePlan::new(&counts, 1);
        assert!(plan.lumped);
    }

    #[test]
    fn numeric_fields_allow_missing_values() {
        let (metadata, _) = metadata();
        assert!(is_numeric_field(&metadata, "Depth"));
        assert!(is_numeric_field(&metadata, "Year"));
        assert!(!is_numeric_field(&metadata, "Site"));
        assert!(!is_numeric_field(&metadata, "Group"));
        assert!(!is_numeric_field(&metadata, "Nothing"));
        assert!(is_missing(" na ") && is_missing("") && !is_missing("Unknown"));
    }

    #[test]
    fn values_are_found_for_each_kind_of_source() {
        let (metadata, tree) = metadata();
        let profile = tree
            .profile(
                &["s1", "s2", "s3", "s4", "s5"],
                None,
                "Level2",
                stamp_core::Unclassified::Retain,
            )
            .unwrap();
        let resolve = |color: &ColorSource, shape: Option<&str>| {
            resolve(
                color,
                shape,
                Some(&metadata),
                Some(&profile),
                Some("b1"),
                Some("Group"),
            )
        };
        let styling = resolve(&ColorSource::Field("Depth".to_string()), Some("Site")).unwrap();
        match &styling.color {
            SampleColors::Values { label, values } => {
                assert_eq!(label, "Depth");
                assert_eq!(values.len(), 3);
                assert_eq!(values["s4"], 4.0);
            }
            other => panic!("{other:?}"),
        }
        let shape = styling.shape.as_ref().unwrap();
        assert_eq!(shape.values.len(), 4);
        assert!(!shape.values.contains_key("s3"));
        assert!(styling.is_extended());

        // A field of numbers drawn by category.
        match resolve(&ColorSource::Category("Year".to_string()), None)
            .unwrap()
            .color
        {
            SampleColors::Categories { values, .. } => assert_eq!(values["s2"], "2002"),
            other => panic!("{other:?}"),
        }
        // Relative abundance, in percent.
        match resolve(&ColorSource::Feature("a1".to_string()), None)
            .unwrap()
            .color
        {
            SampleColors::Values { label, values } => {
                assert_eq!(label, "a1 (%)");
                assert_eq!(values["s1"], 50.0);
                assert_eq!(values["s2"], 0.0);
            }
            other => panic!("{other:?}"),
        }
        match resolve(&ColorSource::SelectedFeature, None).unwrap().color {
            SampleColors::Values { values, .. } => assert_eq!(values["s3"], 50.0),
            other => panic!("{other:?}"),
        }
        let plain = resolve(&ColorSource::Group, None).unwrap();
        assert!(!plain.is_extended());
        assert_eq!(plain.group_label(), "Group");

        assert!(resolve(&ColorSource::Field("Nothing".to_string()), None).is_err());
        assert!(resolve(&ColorSource::Feature("zz".to_string()), None).is_err());
        assert!(resolve(&ColorSource::Group, Some("Nothing")).is_err());
    }
}
