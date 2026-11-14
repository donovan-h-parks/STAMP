//! Map of where the samples were taken: their latitude and longitude from
//! the metadata on a map of the world or a region (outlines from Natural
//! Earth, see [`crate::geo`]), with markers coloured and shaped as in the
//! ordination plots ([`crate::ordination_style`]) and sized by the number
//! of samples at a site, a number from the metadata or a feature's
//! abundance.

use std::collections::{BTreeMap, HashMap};

use stamp_core::{Metadata, Profile};

use crate::axes::{fonts, group_color, marker_styled, tick_style, Marker, INCH};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::geo::{self, Layer, LonLat, Projection, Rect, Resolution, Vertex};
use crate::heatmap::ColorMap;
use crate::ordination_style::{
    draw_legend, is_missing, legend_size, resolve, short_number, ColorScale, ColorSource,
    DrawOrder, LegendItem, SampleColors, ShapePlan, Styling, ValueScale, NO_VALUE_COLOR,
};
use crate::text;

/// A sample and where it was taken.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub name: String,
    /// Index of the sample's group in [`Data::groups`].
    pub group: usize,
    /// Longitude and latitude, if the metadata gives them.
    pub location: Option<LonLat>,
}

/// The numbers that size the markers.
#[derive(Debug, Clone, PartialEq)]
pub struct SizeValues {
    pub label: String,
    pub values: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub samples: Vec<Sample>,
    pub groups: Vec<String>,
    /// Colour of each group; groups beyond the list use the default palette.
    pub group_colors: Vec<Color>,
    /// What colours and shapes the markers.
    pub styling: Styling,
    /// What sizes them, when sized by a number of each sample.
    pub sizes: Option<SizeValues>,
    /// Samples whose latitude or longitude is given but not a coordinate
    /// (out of range or not a number).
    pub invalid: usize,
}

impl Data {
    /// Colour of the group at `index`.
    pub fn group_color(&self, index: usize) -> Color {
        self.group_colors
            .get(index)
            .copied()
            .unwrap_or_else(|| group_color(index))
    }

    /// The samples of an analysis, `sample_groups[i]` being the group of
    /// `sample_names[i]` in `groups`, placed by the fields `lat_field` and
    /// `lon_field` of `metadata`.
    pub fn new(
        sample_names: &[String],
        sample_groups: &[usize],
        groups: Vec<String>,
        metadata: &Metadata,
        lat_field: &str,
        lon_field: &str,
    ) -> Result<Data, String> {
        for field in [lat_field, lon_field] {
            if !metadata.fields().iter().any(|f| f == field) {
                return Err(format!("unknown metadata field '{field}'"));
            }
        }
        let mut invalid = 0;
        let samples = sample_names
            .iter()
            .zip(sample_groups)
            .map(|(name, &group)| {
                let lat = metadata.value(name, lat_field).unwrap_or("");
                let lon = metadata.value(name, lon_field).unwrap_or("");
                let location = match (coordinate(lat, 90.0), coordinate(lon, 180.0)) {
                    (Some(lat), Some(lon)) => Some((lon, lat)),
                    _ => {
                        if !(is_missing(lat) && is_missing(lon)) {
                            invalid += 1;
                        }
                        None
                    }
                };
                Sample {
                    name: name.clone(),
                    group,
                    location,
                }
            })
            .collect();
        Ok(Data {
            samples,
            groups,
            group_colors: Vec::new(),
            styling: Styling::default(),
            sizes: None,
            invalid,
        })
    }
}

/// A latitude (`limit` 90) or longitude (`limit` 180) in decimal degrees,
/// optionally followed by N, S, E or W; `None` if missing or out of range.
pub fn coordinate(text: &str, limit: f64) -> Option<f64> {
    let text = text.trim();
    if is_missing(text) {
        return None;
    }
    let (number, sign) = match text.chars().last()?.to_ascii_uppercase() {
        'N' | 'E' => (&text[..text.len() - 1], 1.0),
        'S' | 'W' => (&text[..text.len() - 1], -1.0),
        _ => (text, 1.0),
    };
    let value = number.trim().trim_end_matches('°').parse::<f64>().ok()? * sign;
    (value.is_finite() && value.abs() <= limit).then_some(value)
}

/// The metadata fields that look like latitude and longitude: named
/// `latitude`, `lat`, `longitude`, `lon`, `long` or `lng` (in any case,
/// also with `decimal_` before them), or failing that containing
/// "latitude" and "longitude".
pub fn coordinate_fields(metadata: &Metadata) -> (Option<String>, Option<String>) {
    let find = |names: &[&str], part: &str| -> Option<String> {
        let fields = metadata.fields();
        let simple = |f: &String| {
            f.to_ascii_lowercase()
                .replace([' ', '-', '_'], "")
                .trim_start_matches("decimal")
                .to_string()
        };
        fields
            .iter()
            .find(|f| names.contains(&simple(f).as_str()))
            .or_else(|| {
                fields
                    .iter()
                    .find(|f| f.to_ascii_lowercase().contains(part))
            })
            .cloned()
    };
    (
        find(&["latitude", "lat", "latdd"], "latitude"),
        find(
            &["longitude", "lon", "long", "lng", "londd", "longdd"],
            "longitude",
        ),
    )
}

/// The parts of the world that can be chosen by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Region {
    Europe,
    Africa,
    Asia,
    NorthAmerica,
    SouthAmerica,
    Oceania,
}

impl Region {
    pub const ALL: [Region; 6] = [
        Region::Europe,
        Region::Africa,
        Region::Asia,
        Region::NorthAmerica,
        Region::SouthAmerica,
        Region::Oceania,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Region::Europe => "Europe",
            Region::Africa => "Africa",
            Region::Asia => "Asia",
            Region::NorthAmerica => "North America",
            Region::SouthAmerica => "South America",
            Region::Oceania => "Oceania",
        }
    }

    /// Longitudes and latitudes: west, east, south, north.
    pub fn bounds(self) -> [f64; 4] {
        match self {
            Region::Europe => [-25.0, 45.0, 34.0, 72.0],
            Region::Africa => [-20.0, 55.0, -37.0, 39.0],
            Region::Asia => [25.0, 150.0, -12.0, 62.0],
            Region::NorthAmerica => [-170.0, -50.0, 7.0, 75.0],
            Region::SouthAmerica => [-95.0, -30.0, -57.0, 15.0],
            Region::Oceania => [110.0, 180.0, -50.0, 0.0],
        }
    }
}

/// The area the map shows.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum Extent {
    /// Around the samples, or the world if they are far apart.
    #[default]
    Auto,
    World,
    Region(Region),
    /// Longitudes `west` to `east` and latitudes `south` to `north`.
    Custom {
        west: f64,
        east: f64,
        south: f64,
        north: f64,
    },
}

impl Extent {
    /// Read the command line form: `auto`, `world`, a region (`europe`,
    /// `north-america`, ...) or `WEST,EAST,SOUTH,NORTH` in degrees.
    pub fn parse(text: &str) -> Result<Extent, String> {
        let simple = text.trim().to_ascii_lowercase().replace([' ', '_'], "-");
        match simple.as_str() {
            "auto" => return Ok(Extent::Auto),
            "world" => return Ok(Extent::World),
            _ => {}
        }
        if let Some(region) = Region::ALL
            .into_iter()
            .find(|r| r.name().to_ascii_lowercase().replace(' ', "-") == simple)
        {
            return Ok(Extent::Region(region));
        }
        let numbers: Vec<f64> = text
            .split(',')
            .map(|n| n.trim().parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| {
                format!(
                    "'{text}': expected auto, world, europe, africa, asia, north-america, \
                     south-america, oceania or WEST,EAST,SOUTH,NORTH"
                )
            })?;
        let [west, east, south, north] = numbers[..] else {
            return Err(format!(
                "'{text}': expected four numbers, WEST,EAST,SOUTH,NORTH"
            ));
        };
        let extent = Extent::Custom {
            west,
            east,
            south,
            north,
        };
        extent.check()?;
        Ok(extent)
    }

    /// An error if a custom extent is not a box on the globe.
    pub fn check(&self) -> Result<(), String> {
        if let Extent::Custom {
            west,
            east,
            south,
            north,
        } = *self
        {
            if !(-180.0..=180.0).contains(&west)
                || !(-180.0..=180.0).contains(&east)
                || !(-90.0..=90.0).contains(&south)
                || !(-90.0..=90.0).contains(&north)
            {
                return Err(
                    "longitudes must be from -180 to 180 and latitudes from -90 to 90".into(),
                );
            }
            if west >= east || south >= north {
                return Err("west must be less than east and south less than north".into());
            }
        }
        Ok(())
    }

    pub fn name(&self) -> &'static str {
        match self {
            Extent::Auto => "Around the samples",
            Extent::World => "World",
            Extent::Region(region) => region.name(),
            Extent::Custom { .. } => "Custom",
        }
    }
}

/// What sizes the markers.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SizeSource {
    /// The number of samples a marker stands for.
    #[default]
    Count,
    /// All the same size.
    Fixed,
    /// A numeric metadata field.
    Field(String),
    /// The relative abundance (%) of a feature.
    Feature(String),
    /// The relative abundance of the feature selected in the feature table.
    SelectedFeature,
}

impl SizeSource {
    /// Read the command line form: `count`, `fixed`, `field:NAME` or
    /// `feature:NAME`.
    pub fn parse(text: &str) -> Result<SizeSource, String> {
        match text.to_ascii_lowercase().as_str() {
            "count" => return Ok(SizeSource::Count),
            "fixed" => return Ok(SizeSource::Fixed),
            _ => {}
        }
        let expected = || format!("'{text}': expected count, fixed, field:NAME or feature:NAME");
        let (kind, name) = text.split_once(':').ok_or_else(expected)?;
        if name.is_empty() {
            return Err(format!("'{text}': the name is missing"));
        }
        match kind.to_ascii_lowercase().as_str() {
            "field" => Ok(SizeSource::Field(name.to_string())),
            "feature" => Ok(SizeSource::Feature(name.to_string())),
            _ => Err(expected()),
        }
    }

    /// The numbers of each sample this source sizes markers by, if it is a
    /// field or a feature.
    pub fn resolve(
        &self,
        metadata: Option<&Metadata>,
        profile: Option<&Profile>,
        selected_feature: Option<&str>,
    ) -> Result<Option<SizeValues>, String> {
        let color = match self {
            SizeSource::Count | SizeSource::Fixed => return Ok(None),
            SizeSource::Field(name) => ColorSource::Field(name.clone()),
            SizeSource::Feature(name) => ColorSource::Feature(name.clone()),
            SizeSource::SelectedFeature => ColorSource::SelectedFeature,
        };
        match resolve(&color, None, metadata, profile, selected_feature, None)
            .map_err(|e| e.replace("colour by", "size by"))?
            .color
        {
            SampleColors::Values { label, values } => Ok(Some(SizeValues { label, values })),
            _ => match self {
                SizeSource::Field(name) => Err(format!(
                    "metadata field '{name}' does not hold numbers, so cannot size the markers"
                )),
                _ => Err("cannot size the markers by this".to_string()),
            },
        }
    }
}

/// How the numbers of the samples at a site give the site its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SiteValue {
    #[default]
    Mean,
    Maximum,
}

impl SiteValue {
    pub const ALL: [SiteValue; 2] = [SiteValue::Mean, SiteValue::Maximum];

    pub fn name(self) -> &'static str {
        match self {
            SiteValue::Mean => "Mean",
            SiteValue::Maximum => "Maximum",
        }
    }

    fn of(self, values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        Some(match self {
            SiteValue::Mean => values.iter().sum::<f64>() / values.len() as f64,
            SiteValue::Maximum => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        })
    }
}

/// Appearance of the map. Lengths are in inches; the height follows from
/// the width and the shape of the area shown.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub projection: Projection,
    pub extent: Extent,
    pub resolution: Resolution,
    /// One marker for the samples at a site (within `site_precision`
    /// degrees) with the same colour and shape, rather than one per sample.
    pub aggregate: bool,
    /// Samples whose coordinates round to the same multiple of this many
    /// degrees are at the same site; 0 chooses a size that merges samples
    /// less than about 3 points apart on the page.
    pub site_precision: f64,
    /// Without aggregation: spread samples at the same site around it.
    pub jitter: bool,
    pub size_by: SizeSource,
    /// Area of the smallest markers (and of all of them when they are not
    /// sized), in square points.
    pub marker_size: f64,
    /// Area of the largest markers, in square points.
    pub max_marker_size: f64,
    /// Sizes from the logarithms of the numbers.
    pub size_log: bool,
    /// How the numbers of the samples at a site colour it.
    pub site_value: SiteValue,
    pub color_by: ColorSource,
    pub shape_by: Option<String>,
    pub color_map: ColorMap,
    pub color_scale: ColorScale,
    pub draw_order: DrawOrder,
    /// Opacity of the markers, from 0 (invisible) to 1.
    pub opacity: f64,
    /// Colour the sea and lakes.
    pub ocean: bool,
    pub graticule: bool,
    /// Outline the countries (borders and coasts) and lakes; without, the
    /// land is only filled.
    pub borders: bool,
    pub legend: bool,
    /// Size of the notes above the map, in points.
    pub note_font_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            projection: Projection::EqualEarth,
            extent: Extent::Auto,
            resolution: Resolution::Auto,
            aggregate: true,
            site_precision: 0.0,
            jitter: false,
            size_by: SizeSource::Count,
            marker_size: 30.0,
            max_marker_size: 400.0,
            size_log: false,
            site_value: SiteValue::Mean,
            color_by: ColorSource::Group,
            shape_by: None,
            color_map: ColorMap::Viridis,
            color_scale: ColorScale::Linear,
            draw_order: DrawOrder::HighOnTop,
            opacity: 1.0,
            ocean: true,
            graticule: true,
            borders: true,
            legend: true,
            note_font_size: crate::axes::NOTE_SIZE,
        }
    }
}

const LAND: Color = Color::rgb(236, 234, 226);
const OCEAN: Color = Color::rgb(222, 235, 245);
const BORDER: Color = Color::rgb(165, 165, 160);
const GRATICULE: Color = Color::rgb(200, 205, 212);

/// A box of longitudes and latitudes: west, east, south, north.
type Bounds = [f64; 4];

const WORLD: Bounds = [-180.0, 180.0, -90.0, 90.0];

/// The area around the samples: their spread with a margin, snapped to
/// whole multiples of 5° (1° for small areas), shaped neither too tall nor
/// too wide; the world if they are spread over much of it.
pub fn auto_bounds(points: &[LonLat]) -> Bounds {
    if points.is_empty() {
        return WORLD;
    }
    let fold = |pick: fn(&LonLat) -> f64| {
        points
            .iter()
            .map(pick)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    };
    let (mut west, mut east) = fold(|p| p.0);
    let (mut south, mut north) = fold(|p| p.1);
    // At least 10° by 6°, plus a margin of a fifth of the spread and 2°.
    let widen = |lo: &mut f64, hi: &mut f64, least: f64| {
        let span = *hi - *lo;
        if span < least {
            *lo -= (least - span) / 2.0;
            *hi += (least - span) / 2.0;
        }
        let pad = (*hi - *lo) * 0.2 + 2.0;
        *lo -= pad;
        *hi += pad;
    };
    widen(&mut west, &mut east, 10.0);
    widen(&mut south, &mut north, 6.0);
    // Neither too tall nor too wide, measuring longitude at the middle
    // latitude.
    let squash = ((south + north) / 2.0).to_radians().cos().max(0.2);
    let (width, height) = ((east - west) * squash, north - south);
    if width < 0.8 * height {
        let extra = (0.8 * height - width) / squash / 2.0;
        west -= extra;
        east += extra;
    } else if height < 0.4 * width {
        let extra = (0.4 * width - height) / 2.0;
        south -= extra;
        north += extra;
    }
    if east - west > 200.0 || north - south > 110.0 {
        return WORLD;
    }
    let step = if east - west > 40.0 { 5.0 } else { 1.0 };
    let snap_down = |v: f64| (v / step).floor() * step;
    let snap_up = |v: f64| (v / step).ceil() * step;
    // Keep the box on the globe, shifting it rather than cutting it.
    let (mut west, mut east) = (snap_down(west), snap_up(east));
    if west < -180.0 {
        east = (east + (-180.0 - west)).min(180.0);
        west = -180.0;
    }
    if east > 180.0 {
        west = (west - (east - 180.0)).max(-180.0);
        east = 180.0;
    }
    [
        west,
        east,
        snap_down(south).max(-90.0),
        snap_up(north).min(90.0),
    ]
}

/// One marker on the map.
struct Mark {
    /// Indices into [`Data::samples`].
    members: Vec<usize>,
    location: LonLat,
    fill: Option<Color>,
    shape: Marker,
    area: f64,
    /// For the order of drawing.
    value: f64,
    tooltip: String,
}

/// Where a point of the projection goes on the page.
struct Placement {
    projection: Projection,
    /// Projected units shown: x from left to right, y from bottom to top.
    frame: Rect,
    /// Page position of the frame's top-left corner, and points per
    /// projected unit.
    left: f64,
    top: f64,
    scale: f64,
}

impl Placement {
    fn page(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            self.left + (x - self.frame.left) * self.scale,
            self.top + (self.frame.top - y) * self.scale,
        )
    }

    fn lonlat(&self, (lon, lat): LonLat) -> (f64, f64) {
        self.page(self.projection.forward(lon, lat))
    }
}

/// A label for a longitude or latitude.
fn degrees(value: f64, positive: char, negative: char) -> String {
    let text = short_number(value.abs());
    if value == 0.0 {
        "0°".to_string()
    } else {
        format!("{text}°{}", if value > 0.0 { positive } else { negative })
    }
}

/// Draw the map.
pub fn plot(data: &Data, options: &Options) -> Figure {
    let located: Vec<usize> = (0..data.samples.len())
        .filter(|&i| data.samples[i].location.is_some())
        .collect();
    if data.samples.is_empty() {
        return Figure::message("No samples to show on the map");
    }
    if located.is_empty() {
        return Figure::message(
            "None of the samples has a latitude and longitude in the metadata fields chosen",
        );
    }
    if let Err(message) = options.extent.check() {
        return Figure::message(&format!("Map area: {message}"));
    }
    let projection = options.projection;
    let (lat_low, lat_high) = projection.lat_limits();
    let bounds = match options.extent {
        Extent::Auto => auto_bounds(
            &located
                .iter()
                .filter_map(|&i| data.samples[i].location)
                .collect::<Vec<_>>(),
        ),
        Extent::World => WORLD,
        Extent::Region(region) => region.bounds(),
        Extent::Custom {
            west,
            east,
            south,
            north,
        } => [west, east, south, north],
    };
    let [west, east, south, north] = [
        bounds[0],
        bounds[1],
        bounds[2].max(lat_low),
        bounds[3].min(lat_high),
    ];
    if south >= north {
        return Figure::message("The map area is beyond the latitudes this projection shows");
    }
    let lon_span = east - west;
    // The whole globe is drawn in the projection's own outline; smaller
    // areas in a rectangle.
    let whole = lon_span >= 360.0 - 1e-9;
    let resolution = options.resolution.for_span(lon_span);

    // The outline of the box of longitudes and latitudes, projected.
    let box_ring = geo::densify(
        &geo::mark_seams(&[(west, south), (east, south), (east, north), (west, north)]),
        1.0,
    );
    let outline: Vec<(f64, f64)> = box_ring
        .iter()
        .map(|&((lon, lat), _)| projection.forward(lon, lat))
        .collect();
    let fold = |pick: fn(&(f64, f64)) -> f64| {
        outline
            .iter()
            .map(pick)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    };
    let (fx0, fx1) = fold(|p| p.0);
    let (fy0, fy1) = fold(|p| p.1);
    let frame = Rect {
        left: fx0,
        right: fx1,
        bottom: fy0,
        top: fy1,
    };
    // Outlines are clipped to a wider box of longitudes and latitudes
    // first, so that the corners of the projected rectangle are filled.
    let clip_box = if whole {
        Rect {
            left: -180.0,
            right: 180.0,
            bottom: lat_low,
            top: lat_high,
        }
    } else {
        Rect {
            left: (west - lon_span).max(-180.0),
            right: (east + lon_span).min(180.0),
            bottom: (south - (north - south)).max(lat_low),
            top: (north + (north - south)).min(lat_high),
        }
    };

    // The markers and their legend.
    let precision = if options.site_precision > 0.0 {
        options.site_precision
    } else {
        // About 3 points of a map as wide as the figure.
        let points = 2.0 * lon_span / (options.width.max(1.0) * INCH);
        [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 3.0, 5.0]
            .into_iter()
            .find(|&p| p >= points)
            .unwrap_or(5.0)
    };
    let inside = |location: LonLat| {
        location.0 >= west && location.0 <= east && location.1 >= south && location.1 <= north
    };
    let shown: Vec<usize> = located
        .iter()
        .copied()
        .filter(|&i| data.samples[i].location.is_some_and(inside))
        .collect();
    if shown.is_empty() {
        return Figure::message(
            "None of the samples with a latitude and longitude is in the area shown",
        );
    }
    let (marks, items, mut notes) = marks(data, &shown, options, precision);
    let outside = located.len() - shown.len();
    if outside > 0 {
        notes.push(format!(
            "{outside} sample{} outside the area shown",
            if outside == 1 { " is" } else { "s are" }
        ));
    }
    let items = if options.legend { items } else { Vec::new() };

    // Layout: notes above, graticule labels left and below, legend right.
    let padding = 0.2 * INCH;
    let note_line = options.note_font_size + 3.0;
    let tick = fonts().tick;
    let labelled = options.graticule && !whole;
    let lat_label_room = if labelled {
        text::width("80°N", tick) + 6.0
    } else {
        0.0
    };
    let lon_label_room = if labelled { tick + 6.0 } else { 0.0 };
    let (legend_width, legend_height) = legend_size(&items);
    let legend_room = if items.is_empty() {
        0.0
    } else {
        legend_width + 0.2 * INCH
    };
    let width = options.width * INCH;
    let top = padding + note_line * notes.len() as f64;
    let mut map_w = width - 2.0 * padding - lat_label_room - legend_room;
    let aspect = (frame.top - frame.bottom) / (frame.right - frame.left);
    if !(map_w > INCH && aspect.is_finite() && aspect > 0.0) {
        return Figure::message("The figure is too narrow for the map and its legend");
    }
    let max_height = 9.0 * INCH;
    if map_w * aspect > max_height {
        map_w = max_height / aspect;
    }
    let map_h = map_w * aspect;
    let left = padding + lat_label_room;
    let height = (top + map_h + lon_label_room + padding).max(top + legend_height + padding);
    let mut figure = Figure::new(width, height);
    let place = Placement {
        projection,
        frame,
        left,
        top,
        scale: map_w / (frame.right - frame.left),
    };
    for (line, note) in notes.iter().enumerate() {
        figure.text(
            left,
            padding + note_line * (line as f64 + 0.5) - 4.0,
            note.clone(),
            TextStyle::new(options.note_font_size).align(HAlign::Left, VAlign::Middle),
        );
    }

    // Projected rings (with which edges are drawn), clipped to the map.
    let project_ring = |ring: &[Vertex]| -> Vec<Vertex> {
        let projected: Vec<Vertex> = geo::densify(ring, 1.0)
            .into_iter()
            .map(|((lon, lat), drawn)| (projection.forward(lon, lat), drawn))
            .collect();
        let clipped = if whole {
            projected
        } else {
            geo::clip_ring(&projected, &frame)
        };
        clipped
            .into_iter()
            .map(|(p, drawn)| (place.page(p), drawn))
            .collect()
    };
    let layer = |layer: Layer| -> Vec<Vec<Vertex>> {
        geo::outlines(layer, resolution)
            .iter()
            .filter_map(|ring| {
                let clipped = geo::clip_ring(&geo::mark_seams(ring), &clip_box);
                let page = project_ring(&clipped);
                (page.len() >= 3).then_some(page)
            })
            .collect()
    };
    let frame_points: Vec<(f64, f64)> = if whole {
        outline.iter().map(|&p| place.page(p)).collect()
    } else {
        vec![
            place.page((frame.left, frame.top)),
            place.page((frame.right, frame.top)),
            place.page((frame.right, frame.bottom)),
            place.page((frame.left, frame.bottom)),
        ]
    };
    if options.ocean {
        figure.area(vec![frame_points.clone()], OCEAN);
    }
    let land = layer(Layer::Countries);
    for ring in &land {
        figure.area(vec![ring.iter().map(|v| v.0).collect()], LAND);
    }
    let lakes = layer(Layer::Lakes);
    let lake_fill = if options.ocean { OCEAN } else { Color::WHITE };
    for ring in &lakes {
        figure.area(vec![ring.iter().map(|v| v.0).collect()], lake_fill);
    }
    if options.graticule {
        graticule(
            &mut figure,
            &place,
            [west, east, south, north],
            whole,
            labelled,
        );
    }
    let border = Stroke::solid(BORDER, 0.4);
    for ring in land.iter().chain(&lakes).filter(|_| options.borders) {
        for line in geo::drawn_edges(ring) {
            figure.line(&line, border);
        }
    }
    let mut closed = frame_points.clone();
    closed.push(frame_points[0]);
    figure.line(&closed, Stroke::solid(Color::rgb(90, 90, 90), 0.75));

    // Markers inside the map, in drawing order, then their tooltips with
    // the topmost last.
    let alpha = (options.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut placed: Vec<(f64, f64, f64)> = Vec::new();
    let positions = marker_positions(&marks, options, &place, precision);
    for (mark, &(x, y)) in marks.iter().zip(&positions) {
        let (fill, edge) = match mark.fill {
            Some(fill) => (
                Some(Color { a: alpha, ..fill }),
                Stroke::solid(
                    Color {
                        a: alpha.max(160),
                        ..Color::BLACK
                    },
                    0.6,
                ),
            ),
            None => (None, Stroke::solid(NO_VALUE_COLOR, 0.75)),
        };
        marker_styled(&mut figure, mark.shape, x, y, mark.area, fill, edge);
        placed.push((x, y, mark.area.max(0.0).sqrt() / 2.0 + 1.0));
    }
    for (mark, &(x, y, r)) in marks.iter().zip(&placed) {
        if r > 0.0 {
            let item = &data.samples[mark.members[0]].name;
            figure.region(x - r, y - r, 2.0 * r, 2.0 * r, item, mark.tooltip.clone());
        }
    }
    if !items.is_empty() {
        draw_legend(
            &mut figure,
            &items,
            width - padding - legend_width,
            top,
            options.marker_size.max(30.0),
        );
    }
    figure
}

/// Where each marker goes on the page: at its site, or spread around it in
/// a sunflower pattern when samples at a site are drawn one by one and
/// jittered.
fn marker_positions(
    marks: &[Mark],
    options: &Options,
    place: &Placement,
    precision: f64,
) -> Vec<(f64, f64)> {
    let mut seen: HashMap<(i64, i64), usize> = HashMap::new();
    marks
        .iter()
        .map(|mark| {
            let (x, y) = place.lonlat(mark.location);
            if options.aggregate || !options.jitter {
                return (x, y);
            }
            let key = site_key(mark.location, precision);
            let k = seen.entry(key).or_insert(0);
            *k += 1;
            let n = (*k - 1) as f64;
            if n == 0.0 {
                return (x, y);
            }
            let spacing = 0.6 * options.marker_size.max(1.0).sqrt();
            let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
            let (r, angle) = (spacing * n.sqrt(), n * golden);
            (x + r * angle.cos(), y + r * angle.sin())
        })
        .collect()
}

fn site_key((lon, lat): LonLat, precision: f64) -> (i64, i64) {
    let precision = if precision > 0.0 { precision } else { 1e-9 };
    (
        (lon / precision).round() as i64,
        (lat / precision).round() as i64,
    )
}

/// Draw lines of longitude and latitude, labelled outside a rectangular
/// map.
fn graticule(figure: &mut Figure, place: &Placement, bounds: Bounds, whole: bool, labelled: bool) {
    let [west, east, south, north] = bounds;
    let span = (east - west).max(north - south);
    let step = [1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0]
        .into_iter()
        .find(|s| span / s <= 8.0)
        .unwrap_or(30.0);
    let stroke = Stroke::solid(GRATICULE, 0.4);
    let frame = Rect {
        left: place.left,
        right: place.left + (place.frame.right - place.frame.left) * place.scale,
        bottom: place.top,
        top: place.top + (place.frame.top - place.frame.bottom) * place.scale,
    };
    // Page coordinates run downwards, so the rectangle is flipped.
    let draw = |figure: &mut Figure, points: Vec<(f64, f64)>| {
        let page: Vec<(f64, f64)> = points.into_iter().map(|p| place.lonlat(p)).collect();
        if whole {
            figure.line(&page, stroke);
        } else {
            for piece in geo::clip_line(&page, &frame) {
                figure.line(&piece, stroke);
            }
        }
    };
    let (lat_low, lat_high) = place.projection.lat_limits();
    // Wider than the map, so that the corners of the rectangle are covered.
    let (lo_lon, hi_lon, lo_lat, hi_lat) = if whole {
        (west, east, south, north)
    } else {
        (
            (west - span).max(-180.0),
            (east + span).min(180.0),
            (south - span).max(lat_low),
            (north + span).min(lat_high),
        )
    };
    let multiples = |lo: f64, hi: f64| {
        let first = (lo / step).ceil() as i64;
        let last = (hi / step).floor() as i64;
        (first..=last).map(move |k| k as f64 * step)
    };
    for lon in multiples(lo_lon, hi_lon) {
        let points = (0..=180)
            .map(|k| (lon, lo_lat + (hi_lat - lo_lat) * k as f64 / 180.0))
            .collect();
        draw(figure, points);
    }
    for lat in multiples(lo_lat, hi_lat) {
        if whole && lat.abs() >= 90.0 {
            continue;
        }
        draw(figure, vec![(lo_lon, lat), (hi_lon, lat)]);
    }
    if !labelled {
        return;
    }
    let style = tick_style().color(Color::rgb(90, 90, 90));
    // Parallels are horizontal in every projection offered, so the bottom
    // edge is the parallel `south` and a parallel meets the left edge at its
    // own height.
    for lon in multiples(lo_lon, hi_lon) {
        let (x, _) = place.lonlat((lon, south));
        if x >= frame.left - 0.5 && x <= frame.right + 0.5 {
            figure.text(
                x,
                frame.top + 3.0,
                degrees(lon, 'E', 'W'),
                style.align(HAlign::Center, VAlign::Top),
            );
        }
    }
    for lat in multiples(lo_lat, hi_lat) {
        let (_, y) = place.lonlat((west, lat));
        if y >= frame.bottom - 0.5 && y <= frame.top + 0.5 {
            figure.text(
                frame.left - 3.0,
                y,
                degrees(lat, 'N', 'S'),
                style.align(HAlign::Right, VAlign::Middle),
            );
        }
    }
}

/// The markers (one per sample, or one per site and colour and shape),
/// the legend and the notes above the map.
fn marks(
    data: &Data,
    located: &[usize],
    options: &Options,
    precision: f64,
) -> (Vec<Mark>, Vec<LegendItem>, Vec<String>) {
    let styling = &data.styling;
    let sample = |i: usize| &data.samples[i];
    let mut color_items = Vec::new();

    // The colour of each located sample, as a category or a number.
    enum Paint {
        Category(usize),
        Value(f64),
        None,
    }
    let mut category_colors: Vec<Color> = Vec::new();
    let mut value_scale = None;
    let paint: HashMap<usize, Paint> = match &styling.color {
        SampleColors::Group => {
            let mut counts = vec![0usize; data.groups.len()];
            for &i in located {
                if let Some(c) = counts.get_mut(sample(i).group) {
                    *c += 1;
                }
            }
            category_colors = (0..data.groups.len())
                .map(|g| data.group_color(g))
                .collect();
            color_items.push(LegendItem::Title(styling.group_label().to_string()));
            for (g, name) in data.groups.iter().enumerate() {
                if counts[g] > 0 {
                    color_items.push(LegendItem::Entry {
                        shape: Marker::Circle,
                        fill: Some(data.group_color(g)),
                        edge: Color::BLACK,
                        label: format!("{name} ({})", counts[g]),
                    });
                }
            }
            located
                .iter()
                .map(|&i| (i, Paint::Category(sample(i).group)))
                .collect()
        }
        SampleColors::Categories { label, values } => {
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for &i in located {
                if let Some(v) = values.get(&sample(i).name) {
                    *counts.entry(v.clone()).or_insert(0) += 1;
                }
            }
            let category_names: Vec<String> = counts.keys().cloned().collect();
            category_colors = (0..counts.len()).map(group_color).collect();
            color_items.push(LegendItem::Title(label.clone()));
            for (k, (name, count)) in counts.iter().enumerate() {
                color_items.push(LegendItem::Entry {
                    shape: Marker::Circle,
                    fill: Some(group_color(k)),
                    edge: Color::BLACK,
                    label: format!("{name} ({count})"),
                });
            }
            if let Some(note) = crate::axes::repeated_colors_note(counts.len()) {
                color_items.push(LegendItem::Note(note.replace("groups", "values")));
            }
            located
                .iter()
                .map(|&i| {
                    let paint = values
                        .get(&sample(i).name)
                        .and_then(|v| category_names.iter().position(|c| c == v))
                        .map_or(Paint::None, Paint::Category);
                    (i, paint)
                })
                .collect()
        }
        SampleColors::Values { values, .. } => located
            .iter()
            .map(|&i| {
                let paint = values
                    .get(&sample(i).name)
                    .map_or(Paint::None, |&v| Paint::Value(v));
                (i, paint)
            })
            .collect(),
    };

    // Shapes, from a field.
    let shape_field = styling.shape.as_ref();
    let mut shape_items = Vec::new();
    let shape_plan = shape_field.map(|field| {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut missing = 0;
        for &i in located {
            match field.values.get(&sample(i).name) {
                Some(v) => *counts.entry(v.clone()).or_insert(0) += 1,
                None => missing += 1,
            }
        }
        let plan = ShapePlan::new(&counts, missing);
        shape_items = plan.legend(&field.label, &counts, missing);
        plan
    });
    let shape_of = |i: usize| -> (Marker, Option<String>) {
        match (shape_field, &shape_plan) {
            (Some(field), Some(plan)) => {
                let value = field.values.get(&sample(i).name);
                (plan.shape(value), value.cloned())
            }
            _ => (Marker::Circle, None),
        }
    };

    // Group the samples into markers.
    let mut order: Vec<(i64, i64, i64, String)> = Vec::new();
    let mut groups: HashMap<(i64, i64, i64, String), Vec<usize>> = HashMap::new();
    for (n, &i) in located.iter().enumerate() {
        let location = sample(i).location.expect("located");
        let colour_key = match paint[&i] {
            Paint::Category(c) => c as i64,
            Paint::Value(_) => -1,
            Paint::None => -2,
        };
        let shape_key = shape_of(i).1.unwrap_or_default();
        let key = if options.aggregate {
            let (x, y) = site_key(location, precision);
            (x, y, colour_key, shape_key)
        } else {
            (n as i64, 0, 0, String::new())
        };
        groups
            .entry(key.clone())
            .or_insert_with(|| {
                order.push(key.clone());
                Vec::new()
            })
            .push(i);
    }
    let members: Vec<Vec<usize>> = order.iter().map(|k| groups[k].clone()).collect();

    // Colours of numbers, by site.
    let site_values: Vec<Option<f64>> = members
        .iter()
        .map(|m| {
            let values: Vec<f64> = m
                .iter()
                .filter_map(|i| match paint[i] {
                    Paint::Value(v) => Some(v),
                    _ => None,
                })
                .collect();
            options.site_value.of(&values)
        })
        .collect();
    if let SampleColors::Values { label, .. } = &styling.color {
        let present: Vec<f64> = site_values.iter().flatten().copied().collect();
        let scale = ValueScale::new(&present, options.color_scale);
        let label = if options.aggregate && members.iter().any(|m| m.len() > 1) {
            format!(
                "{label}, {} at site",
                options.site_value.name().to_lowercase()
            )
        } else {
            label.clone()
        };
        color_items.extend(scale.legend(&label, options.color_map));
        value_scale = Some(scale);
    }
    let no_value = members
        .iter()
        .zip(&site_values)
        .filter(|(m, v)| match &styling.color {
            SampleColors::Values { .. } => v.is_none(),
            _ => matches!(paint[&m[0]], Paint::None),
        })
        .map(|(m, _)| m.len())
        .sum::<usize>();
    if no_value > 0 {
        color_items.push(LegendItem::Entry {
            shape: Marker::Circle,
            fill: None,
            edge: NO_VALUE_COLOR,
            label: format!("No value ({no_value})"),
        });
    }

    // Sizes.
    let size_numbers: Vec<Option<f64>> = members
        .iter()
        .map(|m| match &options.size_by {
            SizeSource::Count => Some(m.len() as f64),
            SizeSource::Fixed => None,
            _ => {
                let values: Vec<f64> = data
                    .sizes
                    .iter()
                    .flat_map(|s| m.iter().filter_map(|i| s.values.get(&sample(*i).name)))
                    .copied()
                    .collect();
                SiteValue::Mean.of(&values)
            }
        })
        .collect();
    let sized = options.size_by != SizeSource::Fixed
        && (options.size_by != SizeSource::Count || options.aggregate);
    let present: Vec<f64> = size_numbers.iter().flatten().copied().collect();
    let positive_floor = present
        .iter()
        .copied()
        .filter(|v| *v > 0.0)
        .fold(f64::INFINITY, f64::min)
        / 2.0;
    let log = options.size_log && positive_floor.is_finite();
    let transform = |v: f64| {
        if log {
            v.max(positive_floor).log10()
        } else {
            v
        }
    };
    let (low, high) = present
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(transform(v)), hi.max(transform(v)))
        });
    let (small, large) = (
        options.marker_size.max(1.0),
        options.max_marker_size.max(options.marker_size.max(1.0)),
    );
    let area_of = |v: Option<f64>| -> f64 {
        match v {
            Some(v) if sized && high > low => {
                small + (large - small) * (transform(v) - low) / (high - low)
            }
            Some(_) if sized => (small + large) / 2.0,
            _ => small,
        }
    };
    let mut size_items = Vec::new();
    if sized && !present.is_empty() {
        let label = match &options.size_by {
            SizeSource::Count => "Samples at site".to_string(),
            _ => {
                let label = data.sizes.as_ref().map_or("Size", |s| s.label.as_str());
                if options.aggregate && members.iter().any(|m| m.len() > 1) {
                    format!("{label}, mean at site")
                } else {
                    label.to_string()
                }
            }
        };
        size_items.push(LegendItem::Title(label));
        let (lo, hi) = (
            present.iter().copied().fold(f64::INFINITY, f64::min),
            present.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        let mut shown = vec![lo];
        if hi > lo {
            let middle = if log && lo > 0.0 {
                (lo * hi).sqrt()
            } else {
                (lo + hi) / 2.0
            };
            let middle = if options.size_by == SizeSource::Count {
                middle.round()
            } else {
                middle
            };
            if middle > lo && middle < hi {
                shown.push(middle);
            }
            shown.push(hi);
        }
        for v in shown {
            size_items.push(LegendItem::Size {
                area: area_of(Some(v)),
                label: short_number(v),
            });
        }
        if log {
            size_items.push(LegendItem::Note("Logarithmic sizes".to_string()));
        }
    }

    // The markers.
    let mut out: Vec<Mark> = members
        .iter()
        .enumerate()
        .map(|(k, m)| {
            let first = m[0];
            let location = if m.len() == 1 {
                sample(first).location.expect("located")
            } else {
                let n = m.len() as f64;
                let (lon, lat) = m.iter().fold((0.0, 0.0), |(a, b), i| {
                    let (lon, lat) = sample(*i).location.expect("located");
                    (a + lon, b + lat)
                });
                (lon / n, lat / n)
            };
            let fill = match (&paint[&first], &value_scale) {
                (Paint::Category(c), _) => category_colors.get(*c).copied(),
                (Paint::Value(_), Some(scale)) => {
                    site_values[k].map(|v| options.color_map.color(scale.fraction(v)))
                }
                _ => None,
            };
            let (shape, shape_value) = shape_of(first);
            let tooltip = tooltip(
                data,
                m,
                location,
                &paint_text(&styling.color, m, site_values[k], options),
                shape_field.map(|f| {
                    format!(
                        "{}: {}",
                        f.label,
                        shape_value.as_deref().unwrap_or("no value")
                    )
                }),
                sized.then(|| size_numbers[k]).flatten().and_then(|v| {
                    let label = match &options.size_by {
                        SizeSource::Count => return None,
                        _ => data.sizes.as_ref()?.label.clone(),
                    };
                    Some(format!("{label}: {}", short_number(v)))
                }),
            );
            Mark {
                members: m.clone(),
                location,
                fill,
                shape,
                area: if fill.is_some() {
                    area_of(size_numbers[k])
                } else {
                    area_of(size_numbers[k]) * 0.6
                },
                value: site_values[k].unwrap_or(f64::NAN),
                tooltip,
            }
        })
        .collect();
    // Large markers first so that small ones stay visible; without a value
    // before those with one; by value as asked.
    out.sort_by(|a, b| {
        let by_value = match options.draw_order {
            DrawOrder::HighOnTop => a.value.total_cmp(&b.value),
            DrawOrder::LowOnTop => b.value.total_cmp(&a.value),
            DrawOrder::AsGiven => std::cmp::Ordering::Equal,
        };
        b.area
            .total_cmp(&a.area)
            .then(a.fill.is_some().cmp(&b.fill.is_some()))
            .then(by_value)
    });

    let mut items = color_items;
    for section in [shape_items, size_items] {
        if !section.is_empty() {
            items.push(LegendItem::Gap);
            items.extend(section);
        }
    }
    let mut notes = Vec::new();
    let without = data.samples.iter().filter(|s| s.location.is_none()).count();
    if options.aggregate && out.len() < located.len() {
        let sites: std::collections::HashSet<(i64, i64)> = located
            .iter()
            .map(|&i| site_key(sample(i).location.expect("located"), precision))
            .collect();
        notes.push(format!(
            "{} samples at {} site{} (within {}°)",
            located.len(),
            sites.len(),
            if sites.len() == 1 { "" } else { "s" },
            short_number(precision)
        ));
    }
    if without > 0 {
        let invalid = if data.invalid > 0 {
            format!(" ({} with values that are not coordinates)", data.invalid)
        } else {
            String::new()
        };
        notes.push(format!(
            "{without} of {} samples have no latitude and longitude{invalid} and are not shown",
            data.samples.len()
        ));
    }
    (out, items, notes)
}

/// What the tooltip says about the colour of a marker.
fn paint_text(
    color: &SampleColors,
    members: &[usize],
    site_value: Option<f64>,
    options: &Options,
) -> Option<String> {
    match color {
        SampleColors::Values { label, .. } => Some(format!(
            "{label}: {}{}",
            site_value.map_or("no value".to_string(), short_number),
            if members.len() > 1 && site_value.is_some() {
                format!(" ({})", options.site_value.name().to_lowercase())
            } else {
                String::new()
            }
        )),
        _ => None,
    }
}

/// The tooltip of a marker: how many samples, where, their groups and
/// categories, and some of their names.
fn tooltip(
    data: &Data,
    members: &[usize],
    (lon, lat): LonLat,
    color: &Option<String>,
    shape: Option<String>,
    size: Option<String>,
) -> String {
    let n = members.len();
    let mut lines = vec![if n == 1 {
        format!(
            "{}\n{}",
            data.samples[members[0]].name,
            degrees_pair(lon, lat)
        )
    } else {
        format!("{n} samples at {}", degrees_pair(lon, lat))
    }];
    let mut groups: BTreeMap<&str, usize> = BTreeMap::new();
    for &i in members {
        let g = data.samples[i].group;
        *groups
            .entry(data.groups.get(g).map_or("", String::as_str))
            .or_insert(0) += 1;
    }
    lines.push(
        groups
            .iter()
            .map(|(g, c)| {
                if n == 1 {
                    g.to_string()
                } else {
                    format!("{g} ({c})")
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    if let SampleColors::Categories { label, values } = &data.styling.color {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for &i in members {
            let v = values
                .get(&data.samples[i].name)
                .map_or("no value", String::as_str);
            *counts.entry(v).or_insert(0) += 1;
        }
        let parts: Vec<String> = counts
            .iter()
            .map(|(v, c)| {
                if n == 1 {
                    v.to_string()
                } else {
                    format!("{v} ({c})")
                }
            })
            .collect();
        lines.push(format!("{label}: {}", parts.join(", ")));
    }
    lines.extend(color.iter().cloned());
    lines.extend(shape);
    lines.extend(size);
    if n > 1 {
        const SHOWN: usize = 8;
        let names: Vec<&str> = members
            .iter()
            .take(SHOWN)
            .map(|&i| data.samples[i].name.as_str())
            .collect();
        let mut text = names.join(", ");
        if n > SHOWN {
            text.push_str(&format!(", … ({} more)", n - SHOWN));
        }
        lines.push(text);
    }
    lines.join("\n")
}

fn degrees_pair(lon: f64, lat: f64) -> String {
    // Up to three decimals (about 100 m).
    let text = |v: f64, positive: char, negative: char| {
        let number = format!("{:.3}", v.abs());
        let number = number.trim_end_matches('0').trim_end_matches('.');
        match v {
            v if v.abs() < 0.0005 => "0°".to_string(),
            v if v > 0.0 => format!("{number}°{positive}"),
            _ => format!("{number}°{negative}"),
        }
    };
    format!("{}, {}", text(lat, 'N', 'S'), text(lon, 'E', 'W'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;
    use crate::ordination_style::SampleShapes;
    use stamp_core::ProfileTree;

    /// Six samples: three at one site in Kenya, one near it, one in
    /// Germany, and one without coordinates.
    fn data() -> Data {
        let profile = "Level1\ts1\ts2\ts3\ts4\ts5\ts6\nA\t1\t2\t3\t4\t5\t6\nB\t1\t1\t1\t1\t1\t1\n";
        let tree = ProfileTree::read(profile).unwrap();
        let metadata = "Sample\tHost\tLatitude\tLongitude\tDepth\n\
                        s1\tape\t-1.5\t37.25\t1\n\
                        s2\tape\t-1.5\t37.25\t2\n\
                        s3\thuman\t-1.5\t37.25\t3\n\
                        s4\tape\t-2.0\t38.0\t4\n\
                        s5\thuman\t52.5N\t13.4E\t10\n\
                        s6\thuman\tNA\t\t5\n";
        let (metadata, _) = Metadata::read(metadata, &tree).unwrap();
        let names: Vec<String> = (1..=6).map(|i| format!("s{i}")).collect();
        Data::new(
            &names,
            &[0, 0, 1, 0, 1, 1],
            vec!["ape".to_string(), "human".to_string()],
            &metadata,
            "Latitude",
            "Longitude",
        )
        .unwrap()
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

    fn markers(figure: &Figure) -> Vec<(f64, f64, f64, Option<Color>)> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Circle {
                    cx,
                    cy,
                    radius,
                    fill,
                    ..
                } => Some((*cx, *cy, *radius, *fill)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn coordinates_are_read_and_fields_found() {
        assert_eq!(coordinate("-12.5", 90.0), Some(-12.5));
        assert_eq!(coordinate("12.5 S", 90.0), Some(-12.5));
        assert_eq!(coordinate("100.25W", 180.0), Some(-100.25));
        assert_eq!(coordinate("91", 90.0), None);
        assert_eq!(coordinate("NA", 90.0), None);
        assert_eq!(coordinate("north", 90.0), None);
        let data = data();
        assert_eq!(data.samples[4].location, Some((13.4, 52.5)));
        assert_eq!(data.samples[5].location, None);
        assert_eq!(data.invalid, 0);

        let tree = ProfileTree::read("Level1\ta\tb\nA\t1\t1\n").unwrap();
        let (metadata, _) = Metadata::read(
            "Sample\tSite\tDecimal_Latitude\tLONG\na\tx\t1\t2\nb\ty\t3\t4\n",
            &tree,
        )
        .unwrap();
        assert_eq!(
            coordinate_fields(&metadata),
            (
                Some("Decimal_Latitude".to_string()),
                Some("LONG".to_string())
            )
        );
        let (metadata, _) = Metadata::read("Sample\tSite\na\tx\nb\ty\n", &tree).unwrap();
        assert_eq!(coordinate_fields(&metadata), (None, None));
    }

    #[test]
    fn extents_and_sizes_are_read_from_the_command_line() {
        assert_eq!(Extent::parse("world"), Ok(Extent::World));
        assert_eq!(Extent::parse("Auto"), Ok(Extent::Auto));
        assert_eq!(
            Extent::parse("north-america"),
            Ok(Extent::Region(Region::NorthAmerica))
        );
        assert_eq!(
            Extent::parse("-10, 30,35,60"),
            Ok(Extent::Custom {
                west: -10.0,
                east: 30.0,
                south: 35.0,
                north: 60.0
            })
        );
        assert!(Extent::parse("30,-10,35,60").is_err());
        assert!(Extent::parse("1,2,3").is_err());
        assert!(Extent::parse("mars").is_err());
        assert_eq!(SizeSource::parse("count"), Ok(SizeSource::Count));
        assert_eq!(
            SizeSource::parse("feature:Equivita"),
            Ok(SizeSource::Feature("Equivita".to_string()))
        );
        assert!(SizeSource::parse("field:").is_err());
        assert!(SizeSource::parse("depth").is_err());
    }

    #[test]
    fn automatic_extent_fits_the_samples() {
        // Samples in East Africa: a box around them, snapped to whole
        // degrees, containing them with a margin.
        let [w, e, s, n] = auto_bounds(&[(37.25, -1.5), (38.0, -2.0)]);
        assert!(w < 35.0 && e > 40.0 && s < -4.0 && n > 1.0);
        assert_eq!([w.fract(), e.fract(), s.fract(), n.fract()], [0.0; 4]);
        // Spread over several continents: the world.
        assert_eq!(
            auto_bounds(&[(-120.0, 40.0), (150.0, -30.0)]),
            [-180.0, 180.0, -90.0, 90.0]
        );
        // Kept on the globe.
        let [w, e, _, _] = auto_bounds(&[(178.0, -40.0), (175.0, -37.0)]);
        assert!(w >= -180.0 && e <= 180.0 && e - w > 10.0);
        assert_eq!(auto_bounds(&[]), WORLD);
    }

    #[test]
    fn samples_at_a_site_share_a_marker() {
        let data = data();
        let options = Options::default();
        let figure = plot(&data, &options);
        let all = texts(&figure);
        assert!(
            all.iter()
                .any(|t| t.starts_with("5 samples at 3 sites (within")),
            "{all:?}"
        );
        assert!(all
            .iter()
            .any(|t| t.starts_with("1 of 6 samples have no latitude")));
        // Kenya: apes (2) and a human at one site, so two markers there,
        // then one ape near it and one human in Germany: four markers, and
        // the world map since the samples are far apart.
        assert_eq!(figure.regions.len(), 4);
        let tips: Vec<&str> = figure.regions.iter().map(|r| r.tooltip.as_str()).collect();
        assert!(
            tips.iter()
                .any(|t| t.starts_with("2 samples at 1.5°S, 37.25°E") && t.contains("s1, s2")),
            "{tips:?}"
        );
        // Sized by number: the two-sample marker is the largest.
        let circles = markers(&figure);
        let largest = circles.iter().map(|c| c.2).fold(0.0, f64::max);
        let smallest = circles.iter().map(|c| c.2).fold(f64::INFINITY, f64::min);
        assert!(largest > smallest);
        // Legend: groups with the number of samples on the map, and sizes.
        assert!(all.contains(&"ape (3)".to_string()));
        assert!(all.contains(&"human (2)".to_string()));
        assert!(all.contains(&"Samples at site".to_string()));
        // The land is drawn.
        let areas = figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Area { .. }))
            .count();
        assert!(areas > 200, "{areas}");

        // One marker per sample without aggregation; jitter spreads them.
        let single = plot(
            &data,
            &Options {
                aggregate: false,
                ..options.clone()
            },
        );
        assert_eq!(single.regions.len(), 5);
        let positions = |figure: &Figure| -> usize {
            let mut seen: Vec<(i64, i64)> = figure
                .regions
                .iter()
                .map(|r| {
                    (
                        (r.x + r.width / 2.0).round() as i64,
                        (r.y + r.height / 2.0).round() as i64,
                    )
                })
                .collect();
            seen.sort();
            seen.dedup();
            seen.len()
        };
        assert_eq!(positions(&single), 3);
        let jittered = plot(
            &data,
            &Options {
                aggregate: false,
                jitter: true,
                ..options
            },
        );
        assert_eq!(positions(&jittered), 5);
    }

    #[test]
    fn markers_coloured_by_a_number_take_the_site_mean() {
        let mut data = data();
        let values: HashMap<String, f64> = (1..=6).map(|i| (format!("s{i}"), i as f64)).collect();
        data.styling = Styling {
            color: SampleColors::Values {
                label: "Depth".to_string(),
                values,
            },
            shape: Some(SampleShapes {
                label: "Host".to_string(),
                values: [("s1", "ape"), ("s2", "ape"), ("s3", "human")]
                    .iter()
                    .map(|(s, v)| (s.to_string(), v.to_string()))
                    .collect(),
            }),
            group_label: None,
        };
        let options = Options {
            extent: Extent::Region(Region::Africa),
            site_value: SiteValue::Maximum,
            ..Options::default()
        };
        let figure = plot(&data, &options);
        let all = texts(&figure);
        assert!(
            all.contains(&"Depth, maximum at site".to_string()),
            "{all:?}"
        );
        assert!(all.contains(&"1 sample is outside the area shown".to_string()));
        // The site with s1 and s2 (values 1 and 2): the maximum, 2.
        let tips: Vec<&str> = figure.regions.iter().map(|r| r.tooltip.as_str()).collect();
        assert!(
            tips.iter().any(|t| t.contains("Depth: 2 (maximum)")),
            "{tips:?}"
        );
        // Graticule labels on a regional map.
        assert!(all.contains(&"20°E".to_string()), "{all:?}");
        assert!(all.contains(&"0°".to_string()));
        // Shapes by host: a square for the human sample.
        assert!(figure
            .shapes
            .iter()
            .any(|s| matches!(s, Shape::Rect { width, .. } if *width < 20.0)));
    }

    #[test]
    fn markers_sized_by_a_field() {
        let mut data = data();
        data.sizes = Some(SizeValues {
            label: "Depth".to_string(),
            values: (1..=6)
                .map(|i| (format!("s{i}"), i as f64 * 10.0))
                .collect(),
        });
        let options = Options {
            size_by: SizeSource::Field("Depth".to_string()),
            aggregate: false,
            ..Options::default()
        };
        let figure = plot(&data, &options);
        let all = texts(&figure);
        assert!(all.contains(&"Depth".to_string()));
        assert!(all.contains(&"10".to_string()) && all.contains(&"50".to_string()));
        let circles = markers(&figure);
        // Legend circles plus five markers; the largest marker is s5's.
        assert!(circles.len() >= 5);
        let tips = figure.regions.iter().find(|r| r.item == "s5").unwrap();
        assert!(tips.tooltip.contains("Depth: 50"), "{}", tips.tooltip);
    }

    #[test]
    fn every_projection_and_extent_draws() {
        let data = data();
        for projection in Projection::ALL {
            for extent in [
                Extent::Auto,
                Extent::World,
                Extent::Region(Region::Europe),
                Extent::Custom {
                    west: 30.0,
                    east: 45.0,
                    south: -10.0,
                    north: 5.0,
                },
            ] {
                let figure = plot(
                    &data,
                    &Options {
                        projection,
                        extent,
                        ..Options::default()
                    },
                );
                assert!(figure.height > 2.0 * INCH, "{projection:?} {extent:?}");
                assert!(
                    figure
                        .shapes
                        .iter()
                        .any(|s| matches!(s, Shape::Area { .. })),
                    "{projection:?} {extent:?}"
                );
                // Everything stays on the page.
                for shape in &figure.shapes {
                    if let Shape::Line { points, .. } = shape {
                        for &(x, y) in points {
                            assert!(
                                x >= -1.0
                                    && x <= figure.width + 1.0
                                    && y >= -1.0
                                    && y <= figure.height + 1.0,
                                "{projection:?} {extent:?}: {x} {y}"
                            );
                        }
                    }
                }
            }
        }
        // The world in Equal Earth is about twice as wide as tall.
        let world = plot(
            &data,
            &Options {
                extent: Extent::World,
                legend: false,
                graticule: false,
                ..Options::default()
            },
        );
        let ratio = (world.width - 0.4 * INCH) / (world.height - 0.4 * INCH - 2.0 * 11.0);
        assert!((ratio - 2.05).abs() < 0.1, "{ratio}");
    }

    #[test]
    fn nothing_to_place() {
        let mut data = data();
        for sample in &mut data.samples {
            sample.location = None;
        }
        assert!(texts(&plot(&data, &Options::default()))[0].starts_with("None of the samples"));
        let figure = plot(
            &Data {
                samples: Vec::new(),
                ..data.clone()
            },
            &Options::default(),
        );
        assert_eq!(
            texts(&figure),
            vec!["No samples to show on the map".to_string()]
        );
    }
}
