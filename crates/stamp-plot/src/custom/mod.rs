//! Custom plots: a plot described in a subset of Vega-Lite (by the
//! assistant, usually) and drawn as a [`Figure`] in the style of STAMP's own
//! plots, so that it is shown, exported and described like them.
//!
//! What is understood:
//!
//! * data: inline `values`, or one of the tables the caller provides by
//!   `name` (STAMP's profile, samples and results);
//! * transforms: `filter`, `calculate`, `aggregate`, `joinaggregate`,
//!   `window`, `fold`, `lookup`, `bin` (see [`data`]);
//! * marks: `bar`, `point`, `circle`, `square`, `tick`, `line`, `area`,
//!   `rect`, `rule`, `text`, `boxplot`, `errorbar`;
//! * encodings: `x`, `y`, `x2`, `y2`, `xOffset`, `yOffset`, `color` (and
//!   `fill`, `stroke`), `size`, `shape`, `opacity`, `strokeDash`, `text`,
//!   `tooltip`, `detail`, `xError`, `yError`, with `field`, `type`,
//!   `aggregate`, `bin`, `title`, `datum`, `value`, `scale` (`type` linear,
//!   log or sqrt, `domain`, `range`, `scheme`, `zero`, `reverse`), `axis`
//!   (`title`, `labelAngle`, `grid`, null), `legend` (`title`, null),
//!   `sort` and `stack`;
//! * `layer`, `title`, `width` and `height` (in points).
//!
//! Anything else is refused with an explanation, or ignored with a note
//! ([`Built::notes`]), so that whoever wrote the description can correct
//! it.

pub mod data;
pub mod expr;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::axes::{self, LabelRotation, Marker};
use crate::axis::{self, Ticks};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::text;
pub use data::{Cell, Table};

/// A custom plot drawn, with notes on what of its description was ignored.
pub struct Built {
    pub figure: Figure,
    pub notes: Vec<String>,
}

/// The names of the data tables a description uses (`"data": {"name":
/// ...}`, in layers and lookups too).
pub fn data_names(spec: &Value) -> Vec<String> {
    let mut names = Vec::new();
    fn walk(value: &Value, names: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                if let Some(Value::Object(data)) = object.get("data") {
                    if let Some(Value::String(name)) = data.get("name") {
                        if !names.contains(name) {
                            names.push(name.clone());
                        }
                    }
                }
                object.values().for_each(|v| walk(v, names));
            }
            Value::Array(items) => items.iter().for_each(|v| walk(v, names)),
            _ => {}
        }
    }
    walk(spec, &mut names);
    names
}

/// The most marks a plot may draw.
const MAX_MARKS: usize = 200_000;
/// Size of a band (a category on an axis) when no width or height is given.
const DEFAULT_STEP: f64 = 20.0;
const DEFAULT_WIDTH: f64 = 300.0;
const DEFAULT_HEIGHT: f64 = 220.0;
/// Colour of marks that no colour is given.
const DEFAULT_COLOR: Color = Color::rgb(31, 119, 180);

/// Draw the plot that `spec` describes. `tables` are the named data it may
/// use; `known_colors` are colours of categories (groups) that the plot
/// uses when every category of a colour encoding has one.
pub fn plot(
    spec: &Value,
    tables: &HashMap<String, Arc<Table>>,
    known_colors: &HashMap<String, Color>,
) -> Result<Built, String> {
    let Value::Object(top) = spec else {
        return Err("the plot description must be a JSON object".to_string());
    };
    let mut notes = Notes::default();
    let mut specs = Vec::new();
    collect_units(top, &UnitSpec::default(), &mut specs, &mut notes, true)?;
    let mut units = Vec::new();
    for (i, unit) in specs.iter().enumerate() {
        let prefix = if specs.len() > 1 {
            format!("layer {}: ", i + 1)
        } else {
            String::new()
        };
        units.push(prepare(unit, tables, &mut notes).map_err(|e| format!("{prefix}{e}"))?);
    }
    let builder = Builder::new(top, units, known_colors, &mut notes)?;
    let figure = builder.draw(&mut notes)?;
    Ok(Built {
        figure,
        notes: notes.0,
    })
}

#[derive(Default)]
struct Notes(Vec<String>);

impl Notes {
    fn add(&mut self, note: impl Into<String>) {
        let note = note.into();
        if !self.0.contains(&note) {
            self.0.push(note);
        }
    }
}

/// One layer as described, with what it inherits.
#[derive(Default, Clone)]
struct UnitSpec {
    data: Option<Value>,
    transforms: Vec<Value>,
    mark: Option<Value>,
    encoding: Map<String, Value>,
}

const NOT_SUPPORTED: [&str; 9] = [
    "facet",
    "repeat",
    "concat",
    "hconcat",
    "vconcat",
    "spec",
    "params",
    "selection",
    "projection",
];

fn collect_units(
    spec: &Map<String, Value>,
    parent: &UnitSpec,
    out: &mut Vec<UnitSpec>,
    notes: &mut Notes,
    top: bool,
) -> Result<(), String> {
    for key in NOT_SUPPORTED {
        if spec.contains_key(key) {
            return Err(format!(
                "'{key}' is not supported: STAMP draws a single panel. Draw one plot, with \
                 colour, xOffset or layers to compare groups."
            ));
        }
    }
    for key in spec.keys() {
        let known = [
            "data",
            "transform",
            "mark",
            "encoding",
            "layer",
            "title",
            "width",
            "height",
            "$schema",
            "description",
            "name",
            "autosize",
            "padding",
            "background",
            "resolve",
        ];
        if !known.contains(&key.as_str()) {
            notes.add(format!("'{key}' is not supported and was ignored"));
        }
        if !top && matches!(key.as_str(), "title" | "width" | "height") {
            notes.add(format!(
                "'{key}' of a layer was ignored; give it at the top"
            ));
        }
    }
    if spec.contains_key("config") {
        notes.add("'config' was ignored: plots take STAMP's fonts and style");
    }
    let mut unit = parent.clone();
    if let Some(data) = spec.get("data") {
        unit.data = Some(data.clone());
    }
    match spec.get("transform") {
        None | Some(Value::Null) => {}
        Some(Value::Array(steps)) => unit.transforms.extend(steps.iter().cloned()),
        Some(_) => return Err("'transform' must be an array".to_string()),
    }
    match spec.get("encoding") {
        None | Some(Value::Null) => {}
        Some(Value::Object(encoding)) => {
            for (k, v) in encoding {
                unit.encoding.insert(k.clone(), v.clone());
            }
        }
        Some(_) => return Err("'encoding' must be an object".to_string()),
    }
    if let Some(layer) = spec.get("layer") {
        let Value::Array(layers) = layer else {
            return Err("'layer' must be an array of plots".to_string());
        };
        for child in layers {
            let Value::Object(child) = child else {
                return Err("each layer must be an object".to_string());
            };
            collect_units(child, &unit, out, notes, false)?;
        }
        return Ok(());
    }
    let Some(mark) = spec.get("mark") else {
        return Err("the plot needs a 'mark' (or a 'layer' of plots with marks)".to_string());
    };
    unit.mark = Some(mark.clone());
    out.push(unit);
    Ok(())
}

/// Data from a `data` object: inline values or a named table.
fn load(data: &Value, tables: &HashMap<String, Arc<Table>>) -> Result<Arc<Table>, String> {
    let names = || {
        let mut names: Vec<&String> = tables.keys().collect();
        names.sort();
        names
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    match data {
        Value::Object(object) => {
            if let Some(values) = object.get("values") {
                let Value::Array(values) = values else {
                    return Err("data.values must be an array of objects".to_string());
                };
                return Ok(Arc::new(Table::from_json(values)?));
            }
            if let Some(name) = object.get("name").and_then(Value::as_str) {
                return tables.get(name).cloned().ok_or_else(|| {
                    format!("there is no data named '{name}' (available: {})", names())
                });
            }
            if object.contains_key("url") {
                return Err(format!(
                    "STAMP does not load data from a URL; give data.values, or data.name for \
                     STAMP's own data ({})",
                    names()
                ));
            }
            Err("data needs 'values' or 'name'".to_string())
        }
        Value::Null => Err(format!(
            "the plot needs data: \"data\": {{\"values\": [...]}}, or {{\"name\": ...}} for \
             STAMP's own data ({})",
            names()
        )),
        _ => Err("'data' must be an object".to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkKind {
    Bar,
    Point,
    Circle,
    Square,
    Tick,
    Line,
    Area,
    Rect,
    Rule,
    Text,
    Boxplot,
    Errorbar,
}

#[derive(Clone)]
struct Mark {
    kind: MarkKind,
    props: Map<String, Value>,
}

impl Mark {
    fn parse(value: &Value, notes: &mut Notes) -> Result<Mark, String> {
        let (name, props) = match value {
            Value::String(name) => (name.as_str(), Map::new()),
            Value::Object(object) => (
                object
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or("a mark object needs a 'type'")?,
                object.clone(),
            ),
            _ => return Err("'mark' must be a name or an object with a type".to_string()),
        };
        let kind = match name {
            "bar" => MarkKind::Bar,
            "point" => MarkKind::Point,
            "circle" => MarkKind::Circle,
            "square" => MarkKind::Square,
            "tick" => MarkKind::Tick,
            "line" | "trail" => MarkKind::Line,
            "area" => MarkKind::Area,
            "rect" => MarkKind::Rect,
            "rule" => MarkKind::Rule,
            "text" => MarkKind::Text,
            "boxplot" => MarkKind::Boxplot,
            "errorbar" => MarkKind::Errorbar,
            "arc" => {
                return Err(
                    "pie and donut charts ('arc') are not supported; a stacked bar shows the \
                     same proportions"
                        .to_string(),
                )
            }
            other => {
                return Err(format!(
                    "the mark '{other}' is not supported; use bar, point, circle, square, tick, \
                     line, area, rect, rule, text, boxplot or errorbar"
                ))
            }
        };
        const KNOWN: [&str; 28] = [
            "type",
            "color",
            "fill",
            "stroke",
            "opacity",
            "fillOpacity",
            "strokeOpacity",
            "size",
            "filled",
            "strokeWidth",
            "strokeDash",
            "point",
            "orient",
            "extent",
            "ticks",
            "fontSize",
            "dx",
            "dy",
            "align",
            "baseline",
            "angle",
            "text",
            "thickness",
            "shape",
            "tooltip",
            "clip",
            "width",
            "cornerRadius",
        ];
        for key in props.keys() {
            if !KNOWN.contains(&key.as_str()) {
                notes.add(format!("the mark property '{key}' was ignored"));
            }
        }
        if let Some(Value::String(i)) = props.get("interpolate") {
            if i != "linear" {
                notes.add("lines are drawn straight between points ('interpolate' was ignored)");
            }
        }
        Ok(Mark { kind, props })
    }

    fn number(&self, key: &str) -> Option<f64> {
        self.props.get(key).and_then(Value::as_f64)
    }

    fn string(&self, key: &str) -> Option<&str> {
        self.props.get(key).and_then(Value::as_str)
    }

    fn color(&self, key: &str) -> Option<Color> {
        self.string(key).and_then(parse_color)
    }

    fn bool(&self, key: &str) -> Option<bool> {
        match self.props.get(key)? {
            Value::Bool(b) => Some(*b),
            Value::Object(_) => Some(true),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    Quantitative,
    Nominal,
}

/// One encoding channel.
#[derive(Clone, Default)]
struct Channel {
    field: Option<String>,
    ty: Option<Ty>,
    aggregate: Option<String>,
    bin: Option<Value>,
    binned: bool,
    title: Option<Option<String>>,
    default_title: Option<String>,
    datum: Option<Cell>,
    value: Option<Value>,
    scale: Map<String, Value>,
    axis: Option<Map<String, Value>>,
    axis_hidden: bool,
    legend_title: Option<Option<String>>,
    legend_hidden: bool,
    sort: Option<Value>,
    stack: Option<Value>,
}

impl Channel {
    fn parse(name: &str, value: &Value, notes: &mut Notes) -> Result<Channel, String> {
        let Value::Object(def) = value else {
            return Err(format!(
                "the encoding '{name}' must be an object such as {{\"field\": ..., \"type\": ...}}"
            ));
        };
        let mut ch = Channel::default();
        for (key, v) in def {
            match key.as_str() {
                "field" => match v {
                    Value::String(f) => ch.field = Some(f.clone()),
                    _ => return Err(format!("the field of '{name}' must be a name")),
                },
                "type" => {
                    ch.ty = Some(match v.as_str() {
                        Some("quantitative") => Ty::Quantitative,
                        Some("nominal") | Some("ordinal") => Ty::Nominal,
                        Some("temporal") => {
                            return Err(format!(
                                "'{name}' is temporal: dates are not supported; use \
                                 quantitative or ordinal"
                            ))
                        }
                        _ => {
                            return Err(format!(
                                "the type of '{name}' must be quantitative, nominal or ordinal"
                            ))
                        }
                    })
                }
                "aggregate" => {
                    let op = v
                        .as_str()
                        .ok_or_else(|| format!("the aggregate of '{name}' must be a name"))?;
                    if !data::AGGREGATES.contains(&op) {
                        return Err(format!(
                            "unknown aggregate '{op}' in '{name}'; available: {}",
                            data::AGGREGATES.join(", ")
                        ));
                    }
                    ch.aggregate = Some(op.to_string());
                }
                "bin" => match v {
                    Value::Bool(false) | Value::Null => {}
                    Value::String(s) if s == "binned" => ch.binned = true,
                    other => ch.bin = Some(other.clone()),
                },
                "title" => {
                    ch.title = Some(match v {
                        Value::Null => None,
                        Value::String(s) => Some(s.clone()),
                        Value::Array(lines) => Some(
                            lines
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(" "),
                        ),
                        _ => return Err(format!("the title of '{name}' must be text")),
                    })
                }
                "datum" => ch.datum = Some(Cell::from_json(v)),
                "value" => ch.value = Some(v.clone()),
                "scale" => match v {
                    Value::Object(s) => ch.scale = s.clone(),
                    Value::Null => {}
                    _ => return Err(format!("the scale of '{name}' must be an object")),
                },
                "axis" => match v {
                    Value::Null => ch.axis_hidden = true,
                    Value::Object(a) => ch.axis = Some(a.clone()),
                    _ => return Err(format!("the axis of '{name}' must be an object or null")),
                },
                "legend" => match v {
                    Value::Null => ch.legend_hidden = true,
                    Value::Object(l) => {
                        if let Some(t) = l.get("title") {
                            ch.legend_title = Some(t.as_str().map(str::to_string));
                        }
                    }
                    _ => {}
                },
                "sort" => ch.sort = Some(v.clone()),
                "stack" => ch.stack = Some(v.clone()),
                "format" | "formatType" | "band" => {}
                "timeUnit" => {
                    return Err(format!("'{name}' has a timeUnit: dates are not supported"))
                }
                "condition" => notes.add(format!("the condition of '{name}' was ignored")),
                other => notes.add(format!("'{other}' of the encoding '{name}' was ignored")),
            }
        }
        if ch.field.is_none() && ch.datum.is_none() && ch.value.is_none() && ch.aggregate.is_none()
        {
            return Err(format!(
                "the encoding '{name}' needs a field, a datum, a value or an aggregate"
            ));
        }
        ch.default_title = match (&ch.aggregate, &ch.field) {
            (Some(op), field) => Some(data::aggregate_title(op, field.as_deref())),
            (None, Some(field)) => Some(field.clone()),
            _ => None,
        };
        Ok(ch)
    }

    fn has_field(&self) -> bool {
        self.field.is_some()
    }

    /// The title of the axis or legend: given, or the field's.
    fn title(&self) -> Option<String> {
        match &self.title {
            Some(t) => t.clone(),
            None => self.default_title.clone(),
        }
    }
}

/// A layer ready to draw: its mark, channels and data.
struct Unit {
    mark: Mark,
    ch: HashMap<String, Channel>,
    /// Fields (and their titles) for the tooltips; `None` for none.
    tooltip: Option<Vec<(String, String)>>,
    table: Table,
}

const CHANNELS: [&str; 18] = [
    "x",
    "y",
    "x2",
    "y2",
    "xOffset",
    "yOffset",
    "color",
    "size",
    "shape",
    "opacity",
    "strokeDash",
    "text",
    "detail",
    "xError",
    "xError2",
    "yError",
    "yError2",
    "order",
];

fn prepare(
    spec: &UnitSpec,
    tables: &HashMap<String, Arc<Table>>,
    notes: &mut Notes,
) -> Result<Unit, String> {
    let source = load(spec.data.as_ref().unwrap_or(&Value::Null), tables)?;
    let mut table = (*source).clone();
    for (i, step) in spec.transforms.iter().enumerate() {
        let Value::Object(step) = step else {
            return Err(format!("transform {} must be an object", i + 1));
        };
        table = data::transform(table, step, &|data| load(data, tables))
            .map_err(|e| format!("transform {}: {e}", i + 1))?;
    }
    let mark = Mark::parse(spec.mark.as_ref().unwrap_or(&Value::Null), notes)?;

    let mut ch: HashMap<String, Channel> = HashMap::new();
    let mut tooltip: Option<Vec<(String, String)>> = Some(Vec::new());
    let mut explicit_tooltip = false;
    for (name, value) in &spec.encoding {
        let name = match name.as_str() {
            "fill" | "stroke" => "color",
            "fillOpacity" | "strokeOpacity" => "opacity",
            "row" | "column" | "facet" => {
                return Err(format!(
                    "'{name}' (small multiples) is not supported: STAMP draws a single panel; \
                     use colour or xOffset to compare groups"
                ))
            }
            "theta" | "radius" | "latitude" | "longitude" | "longitude2" | "latitude2" => {
                return Err(format!("the encoding '{name}' is not supported"))
            }
            "href" | "key" | "description" | "url" | "strokeWidth" | "angle" => {
                notes.add(format!("the encoding '{name}' was ignored"));
                continue;
            }
            other => other,
        };
        if name == "tooltip" {
            explicit_tooltip = true;
            tooltip = match value {
                Value::Null => None,
                Value::Array(items) => Some(
                    items
                        .iter()
                        .map(|item| tooltip_field(item, notes))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .flatten()
                        .collect(),
                ),
                item => Some(tooltip_field(item, notes)?.into_iter().collect()),
            };
            continue;
        }
        if !CHANNELS.contains(&name) {
            notes.add(format!(
                "the encoding '{name}' is not supported and was ignored"
            ));
            continue;
        }
        if name == "order" {
            notes.add("the 'order' encoding was ignored");
            continue;
        }
        if name == "detail" {
            // Several detail fields are not needed; the first groups lines.
            if let Value::Array(items) = value {
                if let Some(first) = items.first() {
                    ch.insert(name.to_string(), Channel::parse(name, first, notes)?);
                }
                continue;
            }
        }
        ch.insert(name.to_string(), Channel::parse(name, value, notes)?);
    }

    // Fields put into bins.
    for axis in ["x", "y"] {
        let Some(c) = ch.get(axis) else { continue };
        let Some(bin) = c.bin.clone() else { continue };
        let field = c
            .field
            .clone()
            .ok_or_else(|| format!("'{axis}' is binned but has no field"))?;
        let (start, end) = (format!("__bin_{axis}"), format!("__bin_{axis}_end"));
        data::bin_column(&mut table, &field, &bin, &start, &end)?;
        let c = ch.get_mut(axis).expect("present");
        c.field = Some(start);
        c.ty = Some(Ty::Quantitative);
        c.binned = true;
        c.default_title = Some(field);
        let second = format!("{axis}2");
        ch.entry(second).or_insert_with(|| Channel {
            field: Some(end),
            ty: Some(Ty::Quantitative),
            ..Channel::default()
        });
    }

    // Aggregates in the encoding: group by the other fields.
    let composite = matches!(mark.kind, MarkKind::Boxplot | MarkKind::Errorbar);
    if !composite && ch.values().any(|c| c.aggregate.is_some()) {
        let mut groupby: Vec<String> = Vec::new();
        for c in ch.values() {
            if c.aggregate.is_none() {
                if let Some(f) = &c.field {
                    if !groupby.contains(f) {
                        groupby.push(f.clone());
                    }
                }
            }
        }
        if let Some(fields) = &tooltip {
            for (f, _) in fields {
                if !groupby.contains(f) && table.column(f).is_some() && explicit_tooltip {
                    groupby.push(f.clone());
                }
            }
        }
        let keys: Vec<usize> = groupby
            .iter()
            .map(|f| table.field(f))
            .collect::<Result<_, _>>()?;
        let mut names: Vec<String> = ch.keys().cloned().collect();
        names.sort();
        let aggregated: Vec<String> = names
            .into_iter()
            .filter(|n| ch[n].aggregate.is_some())
            .collect();
        let mut columns = groupby.clone();
        columns.extend(aggregated.iter().map(|n| format!("__{n}")));
        let mut out = Table::new(columns);
        for rows in data::group_rows(&table, &keys) {
            let mut row: Vec<Cell> = keys
                .iter()
                .map(|&k| table.rows[rows[0]][k].clone())
                .collect();
            for name in &aggregated {
                let c = &ch[name];
                let values: Vec<Cell> = match &c.field {
                    Some(f) => {
                        let col = table.field(f)?;
                        rows.iter().map(|&r| table.rows[r][col].clone()).collect()
                    }
                    None => vec![Cell::Null; rows.len()],
                };
                row.push(data::aggregate(
                    c.aggregate.as_deref().unwrap_or("count"),
                    &values,
                )?);
            }
            out.rows.push(row);
        }
        for name in &aggregated {
            let c = ch.get_mut(name).expect("present");
            c.field = Some(format!("__{name}"));
            c.ty = Some(Ty::Quantitative);
        }
        table = out;
    }

    // Types: given, or from the values.
    for c in ch.values_mut() {
        if c.ty.is_some() {
            continue;
        }
        let first = match (&c.field, &c.datum) {
            (Some(f), _) => {
                let col = table.field(f)?;
                table
                    .rows
                    .iter()
                    .map(|r| &r[col])
                    .find(|v| v.is_valid())
                    .cloned()
            }
            (None, Some(d)) => Some(d.clone()),
            _ => None,
        };
        c.ty = Some(match first {
            Some(Cell::Num(_)) => Ty::Quantitative,
            _ => Ty::Nominal,
        });
    }
    for c in ch.values() {
        if let Some(f) = &c.field {
            table.field(f)?;
        }
    }

    // Default tooltips: the encoded fields.
    if !explicit_tooltip {
        let mut fields: Vec<(String, String)> = Vec::new();
        for name in [
            "x", "x2", "y", "y2", "xOffset", "yOffset", "color", "shape", "size", "opacity",
            "text", "detail",
        ] {
            if let Some(c) = ch.get(name) {
                if let Some(f) = &c.field {
                    if !fields.iter().any(|(g, _)| g == f) && !f.ends_with("_end") {
                        fields.push((f.clone(), c.title().unwrap_or_else(|| f.clone())));
                    }
                }
            }
        }
        for extra in ["feature", "sample"] {
            if table.column(extra).is_some() && !fields.iter().any(|(g, _)| g == extra) {
                fields.insert(0, (extra.to_string(), extra.to_string()));
            }
        }
        tooltip = Some(fields);
    }
    Ok(Unit {
        mark,
        ch,
        tooltip,
        table,
    })
}

fn tooltip_field(item: &Value, notes: &mut Notes) -> Result<Option<(String, String)>, String> {
    match item {
        Value::Object(def) => {
            let Some(field) = def.get("field").and_then(Value::as_str) else {
                notes.add("a tooltip without a field was ignored");
                return Ok(None);
            };
            if def.contains_key("aggregate") {
                notes.add("aggregates in tooltips were ignored; aggregate in a transform instead");
            }
            let title = def
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(field)
                .to_string();
            Ok(Some((field.to_string(), title)))
        }
        Value::String(field) => Ok(Some((field.clone(), field.clone()))),
        _ => Err("a tooltip must be a field definition or a list of them".to_string()),
    }
}

/// A position along an axis, in data terms.
#[derive(Debug, Clone, PartialEq)]
enum P {
    /// A value, plus an offset in points.
    Num(f64, f64),
    /// A place in the band of a category: 0 at its start, 1 at its end.
    Band(String, f64),
    /// The base of bars: zero, or the end of the axis nearest it.
    Base,
    /// A place along the whole axis: 0 at its start (left or bottom), 1 at
    /// its end.
    Frac(f64),
}

enum Prim {
    Rect {
        x: (P, P),
        y: (P, P),
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    Marker {
        x: P,
        y: P,
        shape: Marker,
        area: f64,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    Line {
        points: Vec<(P, P)>,
        stroke: Stroke,
        /// Sort the points down the y axis rather than along x.
        by_y: bool,
    },
    Area {
        /// (x, top, bottom)
        points: Vec<(P, P, P)>,
        fill: Color,
    },
    Segment {
        from: (P, P),
        to: (P, P),
        stroke: Stroke,
    },
    Text {
        x: P,
        y: P,
        text: String,
        style: TextStyle,
        offset: (f64, f64),
    },
    /// No mark: a place with a tooltip (the points of lines).
    Spot { x: P, y: P },
}

struct Item {
    prim: Prim,
    /// The item (feature) the mark stands for, and its tooltip.
    tip: Option<(String, String)>,
}

/// How categories are ordered along an axis or in a legend.
#[derive(Clone)]
enum Order {
    Ascending,
    Descending,
    Appearance,
    Given(Vec<String>),
    ByValue(HashMap<String, f64>, bool),
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => x.total_cmp(&y),
        _ => a.cmp(b),
    }
}

fn order_categories(mut cats: Vec<String>, order: &Order) -> Vec<String> {
    match order {
        Order::Appearance => cats,
        Order::Ascending => {
            cats.sort_by(|a, b| natural_cmp(a, b));
            cats
        }
        Order::Descending => {
            cats.sort_by(|a, b| natural_cmp(b, a));
            cats
        }
        Order::Given(given) => {
            let mut out: Vec<String> = given.iter().filter(|g| cats.contains(g)).cloned().collect();
            out.extend(cats.into_iter().filter(|c| !given.contains(c)));
            out
        }
        Order::ByValue(values, descending) => {
            cats.sort_by(|a, b| {
                let (x, y) = (
                    values.get(a).copied().unwrap_or(f64::NEG_INFINITY),
                    values.get(b).copied().unwrap_or(f64::NEG_INFINITY),
                );
                let o = x.total_cmp(&y);
                if *descending {
                    o.reverse()
                } else {
                    o
                }
            });
            cats
        }
    }
}

/// The order a channel's `sort` asks for, worked out from the unit's data.
fn sort_order(unit: &Unit, channel: &str) -> Result<Order, String> {
    let c = &unit.ch[channel];
    let Some(sort) = &c.sort else {
        return Ok(Order::Ascending);
    };
    let by = |other: &str, op: &str, descending: bool| -> Result<Order, String> {
        let field = c.field.as_deref().ok_or("a sorted channel needs a field")?;
        let col = unit.table.field(field)?;
        let other_col = unit.table.field(other)?;
        let mut values: HashMap<String, Vec<Cell>> = HashMap::new();
        for row in &unit.table.rows {
            values
                .entry(row[col].to_text())
                .or_default()
                .push(row[other_col].clone());
        }
        let mut out = HashMap::new();
        for (k, v) in values {
            out.insert(
                k,
                data::aggregate(op, &v)?
                    .finite()
                    .unwrap_or(f64::NEG_INFINITY),
            );
        }
        Ok(Order::ByValue(out, descending))
    };
    let encoding_field = |name: &str| -> Result<String, String> {
        unit.ch
            .get(name)
            .and_then(|c| c.field.clone())
            .ok_or_else(|| format!("the sort refers to the encoding '{name}', which has no field"))
    };
    match sort {
        Value::Null => Ok(Order::Appearance),
        Value::String(s) => match s.as_str() {
            "ascending" => Ok(Order::Ascending),
            "descending" => Ok(Order::Descending),
            other => {
                let (name, descending) = match other.strip_prefix('-') {
                    Some(name) => (name, true),
                    None => (other, false),
                };
                by(&encoding_field(name)?, "sum", descending)
            }
        },
        Value::Array(values) => Ok(Order::Given(
            values
                .iter()
                .map(|v| Cell::from_json(v).to_text())
                .collect(),
        )),
        Value::Object(o) => {
            let descending = o.get("order").and_then(Value::as_str) == Some("descending");
            if let Some(name) = o.get("encoding").and_then(Value::as_str) {
                return by(&encoding_field(name)?, "sum", descending);
            }
            match o.get("field").and_then(Value::as_str) {
                Some(field) => by(
                    field,
                    o.get("op").and_then(Value::as_str).unwrap_or("sum"),
                    descending,
                ),
                None if o.get("op").and_then(Value::as_str) == Some("count") => {
                    let field = c.field.as_deref().ok_or("a sorted channel needs a field")?;
                    by(field, "count", descending)
                }
                None => Ok(if descending {
                    Order::Descending
                } else {
                    Order::Ascending
                }),
            }
        }
        _ => Err("'sort' must be a name, an array or an object".to_string()),
    }
}

/// How colours are given to values.
enum ColorScale {
    None,
    Nominal {
        index: HashMap<String, Color>,
        entries: Vec<(String, Color)>,
    },
    Quantitative {
        low: f64,
        high: f64,
        mid: Option<f64>,
        stops: Vec<Color>,
    },
}

impl ColorScale {
    fn color(&self, cell: &Cell) -> Option<Color> {
        match self {
            ColorScale::None => None,
            ColorScale::Nominal { index, .. } => index.get(&cell.to_text()).copied(),
            ColorScale::Quantitative {
                low,
                high,
                mid,
                stops,
            } => {
                let v = cell.finite()?;
                let t = match mid {
                    Some(m) if v < *m => 0.5 * (v - low) / (m - low),
                    Some(m) => 0.5 + 0.5 * (v - m) / (high - m),
                    None => (v - low) / (high - low),
                };
                Some(gradient(stops, if t.is_finite() { t } else { 0.5 }))
            }
        }
    }
}

fn gradient(stops: &[Color], t: f64) -> Color {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let i = (t.floor() as usize).min(stops.len() - 2);
    let f = t - i as f64;
    let (a, b) = (stops[i], stops[i + 1]);
    let mix = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * f).round() as u8;
    Color::rgb(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b))
}

fn hexes(list: &[&str]) -> Vec<Color> {
    list.iter().filter_map(|h| Color::from_hex(h)).collect()
}

fn sequential_scheme(name: &str) -> Option<Vec<Color>> {
    Some(hexes(match name {
        "viridis" => &[
            "#440154", "#482878", "#3e4989", "#31688e", "#26828e", "#1f9e89", "#35b779", "#6ece58",
            "#b5de2b", "#fde725",
        ],
        "plasma" | "inferno" | "magma" => &[
            "#0d0887", "#6a00a8", "#b12a90", "#e16462", "#fca636", "#f0f921",
        ],
        "blues" => &["#deebf7", "#9ecae1", "#4292c6", "#08519c"],
        "greens" => &["#e5f5e0", "#a1d99b", "#41ab5d", "#006d2c"],
        "reds" => &["#fee0d2", "#fc9272", "#ef3b2c", "#a50f15"],
        "oranges" => &["#fee6ce", "#fdae6b", "#f16913", "#a63603"],
        "purples" => &["#efedf5", "#bcbddc", "#807dba", "#54278f"],
        "greys" => &["#f0f0f0", "#bdbdbd", "#737373", "#252525"],
        "yellowgreenblue" => &["#ffffcc", "#a1dab4", "#41b6c4", "#225ea8"],
        "redblue" => &[
            "#b2182b", "#ef8a62", "#fddbc7", "#f7f7f7", "#d1e5f0", "#67a9cf", "#2166ac",
        ],
        "blueorange" => &["#2166ac", "#67a9cf", "#f7f7f7", "#fdb863", "#e66101"],
        "redyellowblue" => &[
            "#d73027", "#fc8d59", "#fee090", "#ffffbf", "#e0f3f8", "#91bfdb", "#4575b4",
        ],
        _ => return None,
    }))
}

fn nominal_scheme(name: &str) -> Option<Vec<Color>> {
    Some(hexes(match name {
        "tableau10" => &[
            "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
            "#9c755f", "#bab0ac",
        ],
        "category10" => &[
            "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f",
            "#bcbd22", "#17becf",
        ],
        "set1" => &[
            "#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#ffff33", "#a65628", "#f781bf",
            "#999999",
        ],
        "set2" => &[
            "#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3", "#a6d854", "#ffd92f", "#e5c494", "#b3b3b3",
        ],
        "set3" => &[
            "#8dd3c7", "#ffffb3", "#bebada", "#fb8072", "#80b1d3", "#fdb462", "#b3de69", "#fccde5",
            "#d9d9d9", "#bc80bd", "#ccebc5", "#ffed6f",
        ],
        "dark2" => &[
            "#1b9e77", "#d95f02", "#7570b3", "#e7298a", "#66a61e", "#e6ab02", "#a6761d", "#666666",
        ],
        "paired" => &[
            "#a6cee3", "#1f78b4", "#b2df8a", "#33a02c", "#fb9a99", "#e31a1c", "#fdbf6f", "#ff7f00",
            "#cab2d6", "#6a3d9a", "#ffff99", "#b15928",
        ],
        "pastel1" => &[
            "#fbb4ae", "#b3cde3", "#ccebc5", "#decbe4", "#fed9a6", "#ffffcc", "#e5d8bd", "#fddaec",
            "#f2f2f2",
        ],
        _ => return None,
    }))
}

/// A colour from CSS: `#rgb`, `#rrggbb`, `rgb(r, g, b)` or a name.
pub fn parse_color(text: &str) -> Option<Color> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        if hex.len() == 3 && hex.is_ascii() {
            let doubled: String = hex.chars().flat_map(|c| [c, c]).collect();
            return Color::from_hex(&doubled);
        }
        return Color::from_hex(&hex[..hex.len().min(6)]);
    }
    let lower = text.to_ascii_lowercase();
    if let Some(inner) = lower
        .strip_prefix("rgb(")
        .or_else(|| lower.strip_prefix("rgba("))
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<f64> = inner
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        if parts.len() >= 3 {
            let c = |v: f64| v.clamp(0.0, 255.0).round() as u8;
            let mut color = Color::rgb(c(parts[0]), c(parts[1]), c(parts[2]));
            if let Some(alpha) = parts.get(3) {
                color.a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            return Some(color);
        }
        return None;
    }
    let hex = match lower.as_str() {
        "black" => "000000",
        "white" => "ffffff",
        "red" => "ff0000",
        "green" => "008000",
        "blue" => "0000ff",
        "yellow" => "ffff00",
        "orange" => "ffa500",
        "purple" => "800080",
        "pink" => "ffc0cb",
        "brown" => "a52a2a",
        "grey" | "gray" => "808080",
        "lightgrey" | "lightgray" => "d3d3d3",
        "darkgrey" | "darkgray" => "a9a9a9",
        "dimgrey" | "dimgray" => "696969",
        "silver" => "c0c0c0",
        "gainsboro" => "dcdcdc",
        "whitesmoke" => "f5f5f5",
        "cyan" | "aqua" => "00ffff",
        "magenta" | "fuchsia" => "ff00ff",
        "teal" => "008080",
        "navy" => "000080",
        "olive" => "808000",
        "maroon" => "800000",
        "lime" => "00ff00",
        "gold" => "ffd700",
        "coral" => "ff7f50",
        "tomato" => "ff6347",
        "salmon" => "fa8072",
        "crimson" => "dc143c",
        "firebrick" => "b22222",
        "darkred" => "8b0000",
        "indianred" => "cd5c5c",
        "orangered" => "ff4500",
        "darkorange" => "ff8c00",
        "khaki" => "f0e68c",
        "goldenrod" => "daa520",
        "chocolate" => "d2691e",
        "sienna" => "a0522d",
        "tan" => "d2b48c",
        "beige" => "f5f5dc",
        "lightblue" => "add8e6",
        "skyblue" => "87ceeb",
        "steelblue" => "4682b4",
        "royalblue" => "4169e1",
        "dodgerblue" => "1e90ff",
        "darkblue" => "00008b",
        "midnightblue" => "191970",
        "cornflowerblue" => "6495ed",
        "slateblue" => "6a5acd",
        "lightgreen" => "90ee90",
        "darkgreen" => "006400",
        "forestgreen" => "228b22",
        "seagreen" => "2e8b57",
        "limegreen" => "32cd32",
        "olivedrab" => "6b8e23",
        "turquoise" => "40e0d0",
        "darkcyan" => "008b8b",
        "lavender" => "e6e6fa",
        "violet" => "ee82ee",
        "orchid" => "da70d6",
        "plum" => "dda0dd",
        "indigo" => "4b0082",
        "darkviolet" => "9400d3",
        "hotpink" => "ff69b4",
        "deeppink" => "ff1493",
        "slategrey" | "slategray" => "708090",
        "darkslategrey" | "darkslategray" => "2f4f4f",
        "transparent" => {
            return Some(Color {
                a: 0,
                ..Color::WHITE
            })
        }
        _ => return None,
    };
    Color::from_hex(hex)
}

fn with_opacity(mut color: Color, opacity: f64) -> Color {
    color.a = (f64::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8;
    color
}

fn marker_named(name: &str) -> Option<Marker> {
    Some(match name {
        "circle" => Marker::Circle,
        "square" => Marker::Square,
        "triangle" | "triangle-up" => Marker::TriangleUp,
        "triangle-down" => Marker::TriangleDown,
        "diamond" => Marker::Diamond,
        "cross" => Marker::Plus,
        "stroke" | "x" => Marker::Cross,
        "hexagon" => Marker::Hexagon,
        "star" => Marker::Star,
        "pentagon" => Marker::Pentagon,
        _ => return None,
    })
}

/// A numeric scale's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumKind {
    Linear,
    Log,
    Sqrt,
}

impl NumKind {
    fn apply(self, v: f64) -> Option<f64> {
        match self {
            NumKind::Linear => Some(v),
            NumKind::Log => (v > 0.0).then(|| v.log10()),
            NumKind::Sqrt => (v >= 0.0).then(|| v.sqrt()),
        }
    }
}

/// A position scale: data along an axis of the frame.
enum Scale {
    Num {
        lo: f64,
        hi: f64,
        kind: NumKind,
        start: f64,
        end: f64,
    },
    Band {
        index: HashMap<String, usize>,
        count: usize,
        start: f64,
        end: f64,
    },
}

impl Scale {
    fn set_range(&mut self, a: f64, b: f64) {
        match self {
            Scale::Num { start, end, .. } | Scale::Band { start, end, .. } => {
                *start = a;
                *end = b;
            }
        }
    }

    fn map(&self, p: &P) -> Option<f64> {
        match (self, p) {
            (Scale::Num { start, end, .. } | Scale::Band { start, end, .. }, P::Frac(f)) => {
                Some(start + f * (end - start))
            }
            (
                Scale::Num {
                    lo,
                    hi,
                    kind,
                    start,
                    end,
                },
                P::Num(..) | P::Base,
            ) => {
                let (value, offset) = match p {
                    P::Num(v, o) => (*v, *o),
                    _ => {
                        let base = if *kind == NumKind::Log || *lo > 0.0 {
                            *lo
                        } else if *hi < 0.0 {
                            *hi
                        } else {
                            0.0
                        };
                        (base, 0.0)
                    }
                };
                let (a, b, v) = (kind.apply(*lo)?, kind.apply(*hi)?, kind.apply(value)?);
                let t = if b == a { 0.5 } else { (v - a) / (b - a) };
                Some(start + t * (end - start) + offset)
            }
            (
                Scale::Band {
                    index,
                    count,
                    start,
                    end,
                },
                P::Band(cat, f),
            ) => {
                let i = *index.get(cat)?;
                let step = (end - start) / *count as f64;
                Some(start + (i as f64 + f) * step)
            }
            _ => None,
        }
    }
}

/// What is known of an axis before it is drawn.
struct AxisPlan {
    scale: Scale,
    /// Tick positions with their labels.
    ticks: Vec<(P, String)>,
    title: Option<String>,
    hidden: bool,
    grid: bool,
    label_angle: Option<f64>,
    reversed: bool,
}

/// Markers given to categories, the legend's title, and whether the
/// legend is hidden.
type ShapeScale = (Vec<(String, Marker)>, Option<String>, bool);

struct Builder<'a> {
    top: &'a Map<String, Value>,
    units: Vec<Unit>,
    colors: ColorScale,
    color_title: Option<String>,
    color_hidden: bool,
    shapes: Option<ShapeScale>,
}

impl<'a> Builder<'a> {
    fn new(
        top: &'a Map<String, Value>,
        units: Vec<Unit>,
        known_colors: &HashMap<String, Color>,
        notes: &mut Notes,
    ) -> Result<Builder<'a>, String> {
        // The colour scale, shared by every layer.
        let mut colors = ColorScale::None;
        let mut color_title = None;
        let mut color_hidden = false;
        let coloured: Vec<&Unit> = units
            .iter()
            .filter(|u| u.ch.get("color").is_some_and(Channel::has_field))
            .collect();
        if let Some(first) = coloured.first() {
            let c = &first.ch["color"];
            color_title = match &c.legend_title {
                Some(t) => t.clone(),
                None => c.title(),
            };
            color_hidden = c.legend_hidden;
            if c.ty == Some(Ty::Quantitative) {
                let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
                for u in &coloured {
                    let col = u
                        .table
                        .field(u.ch["color"].field.as_deref().unwrap_or(""))?;
                    for row in &u.table.rows {
                        if let Some(v) = row[col].finite() {
                            low = low.min(v);
                            high = high.max(v);
                        }
                    }
                }
                if let Some(Value::Array(d)) = c.scale.get("domain") {
                    if let [Some(a), Some(b)] =
                        [d.first(), d.last()].map(|v| v.and_then(Value::as_f64))
                    {
                        (low, high) = (a, b);
                    }
                }
                if !low.is_finite() {
                    (low, high) = (0.0, 1.0);
                }
                let scheme = c.scale.get("scheme").and_then(|s| {
                    s.as_str()
                        .or_else(|| s.get("name").and_then(Value::as_str))
                        .map(str::to_ascii_lowercase)
                });
                let stops = match (c.scale.get("range"), scheme) {
                    (Some(Value::Array(range)), _) => {
                        let stops: Vec<Color> = range
                            .iter()
                            .filter_map(|v| v.as_str().and_then(parse_color))
                            .collect();
                        if stops.len() >= 2 {
                            stops
                        } else {
                            return Err(
                                "the colour range must list at least two colours".to_string()
                            );
                        }
                    }
                    (_, Some(name)) => sequential_scheme(&name).unwrap_or_else(|| {
                        notes.add(format!(
                            "the colour scheme '{name}' is not known; viridis was used (known: \
                             viridis, plasma, blues, greens, reds, oranges, purples, greys, \
                             yellowgreenblue, redblue, blueorange, redyellowblue)"
                        ));
                        sequential_scheme("viridis").unwrap_or_default()
                    }),
                    _ => sequential_scheme("viridis").unwrap_or_default(),
                };
                let mid = c.scale.get("domainMid").and_then(Value::as_f64);
                colors = ColorScale::Quantitative {
                    low,
                    high,
                    mid,
                    stops,
                };
            } else {
                let mut cats: Vec<String> = Vec::new();
                let mut seen = HashSet::new();
                for u in &coloured {
                    let col = u
                        .table
                        .field(u.ch["color"].field.as_deref().unwrap_or(""))?;
                    for row in &u.table.rows {
                        let t = row[col].to_text();
                        if seen.insert(t.clone()) {
                            cats.push(t);
                        }
                    }
                }
                let order = match c.scale.get("domain") {
                    Some(Value::Array(d)) => {
                        Order::Given(d.iter().map(|v| Cell::from_json(v).to_text()).collect())
                    }
                    _ => sort_order(first, "color")?,
                };
                let cats = order_categories(cats, &order);
                let range: Option<Vec<Color>> = match c.scale.get("range") {
                    Some(Value::Array(range)) => {
                        let colors: Vec<Color> = range
                            .iter()
                            .filter_map(|v| v.as_str().and_then(parse_color))
                            .collect();
                        if colors.len() != range.len() {
                            notes.add("colours of the range that could not be read were left out");
                        }
                        (!colors.is_empty()).then_some(colors)
                    }
                    _ => None,
                };
                let scheme = c.scale.get("scheme").and_then(|s| {
                    s.as_str()
                        .or_else(|| s.get("name").and_then(Value::as_str))
                        .map(str::to_ascii_lowercase)
                });
                let palette = match (range, scheme) {
                    (Some(range), _) => range,
                    (None, Some(name)) => nominal_scheme(&name)
                        .or_else(|| {
                            sequential_scheme(&name).map(|stops| {
                                (0..cats.len().max(2))
                                    .map(|i| {
                                        gradient(&stops, i as f64 / (cats.len().max(2) - 1) as f64)
                                    })
                                    .collect()
                            })
                        })
                        .unwrap_or_else(|| {
                            notes.add(format!(
                                "the colour scheme '{name}' is not known; STAMP's colours were \
                                 used (known: tableau10, category10, set1, set2, set3, dark2, \
                                 paired, pastel1, and the sequential schemes)"
                            ));
                            axes::GROUP_COLORS.to_vec()
                        }),
                    (None, None) => {
                        if !cats.is_empty() && cats.iter().all(|c| known_colors.contains_key(c)) {
                            cats.iter().map(|c| known_colors[c]).collect()
                        } else {
                            axes::GROUP_COLORS.to_vec()
                        }
                    }
                };
                let entries: Vec<(String, Color)> = cats
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (c.clone(), palette[i % palette.len()]))
                    .collect();
                colors = ColorScale::Nominal {
                    index: entries.iter().cloned().collect(),
                    entries,
                };
            }
        }

        // Shapes, shared too.
        let mut shapes = None;
        if let Some(u) = units
            .iter()
            .find(|u| u.ch.get("shape").is_some_and(Channel::has_field))
        {
            let c = &u.ch["shape"];
            let col = u.table.field(c.field.as_deref().unwrap_or(""))?;
            let mut cats = Vec::new();
            let mut seen = HashSet::new();
            for row in &u.table.rows {
                let t = row[col].to_text();
                if seen.insert(t.clone()) {
                    cats.push(t);
                }
            }
            let cats = order_categories(cats, &sort_order(u, "shape")?);
            let given: Vec<Marker> = match c.scale.get("range") {
                Some(Value::Array(r)) => r
                    .iter()
                    .filter_map(|v| v.as_str().and_then(marker_named))
                    .collect(),
                _ => Vec::new(),
            };
            let palette = if given.is_empty() {
                Marker::ALL.to_vec()
            } else {
                given
            };
            let title = match &c.legend_title {
                Some(t) => t.clone(),
                None => c.title(),
            };
            shapes = Some((
                cats.iter()
                    .enumerate()
                    .map(|(i, c)| (c.clone(), palette[i % palette.len()]))
                    .collect(),
                title,
                c.legend_hidden,
            ));
        }
        if units
            .iter()
            .any(|u| u.ch.get("size").is_some_and(Channel::has_field))
        {
            notes.add("no legend is drawn for size");
        }
        Ok(Builder {
            top,
            units,
            colors,
            color_title,
            color_hidden,
            shapes,
        })
    }

    fn draw(self, notes: &mut Notes) -> Result<Figure, String> {
        let mut items: Vec<Item> = Vec::new();
        let mut skipped = 0usize;
        for unit in &self.units {
            self.unit_items(unit, &mut items, notes)?;
            if items.len() > MAX_MARKS {
                return Err(format!(
                    "the plot would draw more than {MAX_MARKS} marks; filter or aggregate the data \
                     first"
                ));
            }
        }

        let x = self.axis_plan("x", &items, notes)?;
        let y = self.axis_plan("y", &items, notes)?;
        let (mut x, mut y) = (x, y);

        // Size of the frame.
        let size = |key: &str, plan: &AxisPlan, default: f64| -> f64 {
            let bands = match &plan.scale {
                Scale::Band { count, .. } => Some(*count as f64),
                Scale::Num { .. } => None,
            };
            let value = match self.top.get(key) {
                Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
                Some(Value::Object(o)) => match (o.get("step").and_then(Value::as_f64), bands) {
                    (Some(step), Some(n)) => step * n,
                    _ => default,
                },
                // Bands of the default step, but not so few that the plot
                // is a sliver.
                _ => match bands {
                    Some(n) => (DEFAULT_STEP * n).max(default * 0.6),
                    None => default,
                },
            };
            value.clamp(20.0, 5000.0)
        };
        let width = size("width", &x, DEFAULT_WIDTH);
        let height = size("height", &y, DEFAULT_HEIGHT);

        let fonts = axes::fonts();
        let title = match self.top.get("title") {
            Some(Value::String(t)) => Some(t.clone()),
            Some(Value::Object(o)) => match o.get("text") {
                Some(Value::String(t)) => Some(t.clone()),
                Some(Value::Array(lines)) => Some(
                    lines
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                _ => None,
            },
            _ => None,
        };
        let title_size = fonts.axis_label + 2.0;

        // Room for the y axis on the left.
        let y_labels: Vec<&String> = y.ticks.iter().map(|(_, l)| l).collect();
        let widest_y = y_labels
            .iter()
            .map(|l| text::width(l, fonts.tick))
            .fold(0.0, f64::max);
        let y_room = if y.hidden {
            0.0
        } else {
            axes::TICK_PAD
                + widest_y
                + if y.title.is_some() {
                    4.0 + axes::line_height(fonts.axis_label)
                } else {
                    0.0
                }
        };
        let rotation = if x.hidden {
            LabelRotation::Horizontal
        } else {
            self.x_rotation(&x, width)
        };
        // Labels turned 45 degrees reach left of their tick.
        let first_reach = match (&x.scale, rotation, x.ticks.first()) {
            (Scale::Band { count, .. }, LabelRotation::Diagonal, Some((_, label))) => {
                let step = width / *count as f64;
                (text::width(label, fonts.tick) + axes::line_height(fonts.tick))
                    * std::f64::consts::FRAC_1_SQRT_2
                    - step / 2.0
            }
            _ => 0.0,
        };
        let left = 8.0 + y_room.max(first_reach);
        let top = 8.0
            + if title.is_some() {
                axes::line_height(title_size) + 6.0
            } else {
                axes::line_height(fonts.tick) / 2.0
            };
        let frame = axes::Frame {
            left,
            top,
            right: left + width,
            bottom: top + height,
        };
        if x.reversed {
            x.scale.set_range(frame.right, frame.left);
        } else {
            x.scale.set_range(frame.left, frame.right);
        }
        let y_up = matches!(y.scale, Scale::Num { .. });
        match (y_up, y.reversed) {
            (true, false) | (false, true) => y.scale.set_range(frame.bottom, frame.top),
            _ => y.scale.set_range(frame.top, frame.bottom),
        }

        let mut figure = Figure::new(0.0, 0.0);
        // Grid lines beneath the marks.
        let grid = Stroke::solid(Color::grey(0.9), 0.75);
        if x.grid {
            for (p, _) in &x.ticks {
                if let Some(px) = x.scale.map(p) {
                    figure.line(&[(px, frame.top), (px, frame.bottom)], grid);
                }
            }
        }
        if y.grid {
            for (p, _) in &y.ticks {
                if let Some(py) = y.scale.map(p) {
                    figure.line(&[(frame.left, py), (frame.right, py)], grid);
                }
            }
        }

        for item in items {
            if !draw_item(&mut figure, &item, &x.scale, &y.scale) {
                skipped += 1;
            }
        }
        if skipped > 0 {
            notes.add(format!(
                "{skipped} marks were left out: their values are missing, or are zero or less \
                 on a logarithmic scale"
            ));
        }

        let axis_color = axes::AXES_COLOR;
        let mut bottom = frame.bottom;
        if !x.hidden {
            let ticks: Vec<(f64, String)> = x
                .ticks
                .iter()
                .filter_map(|(p, l)| Some((x.scale.map(p)?, l.clone())))
                .collect();
            bottom = axes::x_axis(
                &mut figure,
                &frame,
                &ticks,
                x.title.as_deref(),
                rotation,
                axis_color,
            );
        }
        if !y.hidden {
            let ticks: Vec<(f64, String)> = y
                .ticks
                .iter()
                .filter_map(|(p, l)| Some((y.scale.map(p)?, l.clone())))
                .collect();
            axes::y_axis(&mut figure, &frame, &ticks, y.title.as_deref(), axis_color);
        }

        let mut right = frame.right;
        let mut legend_bottom = frame.top;
        self.legends(&mut figure, &frame, &mut right, &mut legend_bottom);

        if let Some(title) = &title {
            figure.text(
                (frame.left + frame.right) / 2.0,
                8.0,
                title.clone(),
                TextStyle::new(title_size).align(HAlign::Center, VAlign::Top),
            );
            let half = text::width(title, title_size) / 2.0;
            right = right.max((frame.left + frame.right) / 2.0 + half);
        }
        figure.width = (right + 12.0).ceil();
        figure.height = (bottom.max(legend_bottom) + 8.0).ceil();
        Ok(figure)
    }

    /// How to turn the labels of the x axis: as asked, or flat when they
    /// fit, at 45 degrees when they are short and upright when they are
    /// long.
    fn x_rotation(&self, x: &AxisPlan, width: f64) -> LabelRotation {
        if let Some(angle) = x.label_angle {
            let a = angle.abs() % 180.0;
            return if a < 1.0 {
                LabelRotation::Horizontal
            } else if (a - 90.0).abs() < 1.0 {
                LabelRotation::Vertical
            } else {
                LabelRotation::Diagonal
            };
        }
        let Scale::Band { count, .. } = &x.scale else {
            return LabelRotation::Horizontal;
        };
        let step = width / *count as f64;
        let size = axes::fonts().tick;
        let widest = x
            .ticks
            .iter()
            .map(|(_, l)| text::width(l, size))
            .fold(0.0, f64::max);
        if widest + 4.0 <= step {
            LabelRotation::Horizontal
        } else if widest <= 70.0 {
            LabelRotation::Diagonal
        } else {
            LabelRotation::Vertical
        }
    }

    /// The first channel of an axis with something to say about it.
    fn axis_channel(&self, axis: &str) -> Option<(&Unit, &Channel)> {
        self.units
            .iter()
            .find_map(|u| u.ch.get(axis).filter(|c| c.has_field()).map(|c| (u, c)))
            .or_else(|| {
                self.units
                    .iter()
                    .find_map(|u| u.ch.get(axis).map(|c| (u, c)))
            })
    }

    fn axis_plan(&self, axis: &str, items: &[Item], notes: &mut Notes) -> Result<AxisPlan, String> {
        let mut numbers: Vec<f64> = Vec::new();
        let mut cats: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        let mut has_base = false;
        let mut visit = |p: &P| match p {
            P::Num(v, _) if v.is_finite() => numbers.push(*v),
            P::Band(c, _) => {
                if seen.insert(c.clone()) {
                    cats.push(c.clone());
                }
            }
            P::Base => has_base = true,
            _ => {}
        };
        for item in items {
            for_positions(&item.prim, axis == "x", &mut visit);
        }
        let channel = self.axis_channel(axis);
        let (scale_spec, axis_spec, hidden) = match channel {
            Some((_, c)) => (
                c.scale.clone(),
                c.axis.clone().unwrap_or_default(),
                c.axis_hidden,
            ),
            None => (Map::new(), Map::new(), false),
        };
        let mut title = channel.and_then(|(_, c)| c.title());
        if let Some(t) = axis_spec.get("title") {
            title = t.as_str().map(str::to_string);
        }
        // A second channel (x2, y2) may give the title when the first has none.
        let grid = axis_spec
            .get("grid")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let reversed = scale_spec
            .get("reverse")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let label_angle = axis_spec.get("labelAngle").and_then(Value::as_f64);

        if !cats.is_empty() && !numbers.is_empty() {
            return Err(format!(
                "the {axis} axis has both categories and numbers; give its fields one type \
                 (quantitative or nominal)"
            ));
        }
        if !cats.is_empty() {
            let order = match scale_spec.get("domain") {
                Some(Value::Array(d)) => {
                    Order::Given(d.iter().map(|v| Cell::from_json(v).to_text()).collect())
                }
                _ => match channel {
                    Some((u, _)) => sort_order(u, axis)?,
                    None => Order::Ascending,
                },
            };
            let cats = order_categories(cats, &order);
            let ticks = cats
                .iter()
                .map(|c| (P::Band(c.clone(), 0.5), text::truncate(c, 60)))
                .collect();
            return Ok(AxisPlan {
                scale: Scale::Band {
                    index: cats
                        .iter()
                        .enumerate()
                        .map(|(i, c)| (c.clone(), i))
                        .collect(),
                    count: cats.len().max(1),
                    start: 0.0,
                    end: 1.0,
                },
                ticks,
                title,
                hidden,
                grid,
                label_angle,
                reversed,
            });
        }

        let kind = match scale_spec.get("type").and_then(Value::as_str) {
            None | Some("linear") => NumKind::Linear,
            Some("log") => NumKind::Log,
            Some("sqrt") => NumKind::Sqrt,
            Some("symlog") | Some("pow") => {
                notes.add("symlog and pow scales are drawn as linear scales");
                NumKind::Linear
            }
            Some(other) => {
                return Err(format!(
                    "the scale type '{other}' is not supported; use linear, log or sqrt"
                ))
            }
        };
        let binned = self
            .units
            .iter()
            .any(|u| u.ch.get(axis).is_some_and(|c| c.binned));
        let domain = match scale_spec.get("domain") {
            Some(Value::Array(d)) if d.len() == 2 => match (d[0].as_f64(), d[1].as_f64()) {
                (Some(a), Some(b)) => Some((a.min(b), a.max(b))),
                _ => None,
            },
            _ => None,
        };
        let zero = scale_spec
            .get("zero")
            .and_then(Value::as_bool)
            .unwrap_or(!binned && kind != NumKind::Log);
        let nice = scale_spec
            .get("nice")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let usable: Vec<f64> = numbers
            .iter()
            .copied()
            .filter(|v| kind != NumKind::Log || *v > 0.0)
            .collect();
        let (mut min, mut max) = usable
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
                (a.min(v), b.max(v))
            });
        if !min.is_finite() {
            (min, max) = if kind == NumKind::Log {
                (1.0, 10.0)
            } else {
                (0.0, 1.0)
            };
        }
        if (zero || has_base) && kind == NumKind::Linear && domain.is_none() {
            min = min.min(0.0);
            max = max.max(0.0);
        }
        let (lo, hi, ticks): (f64, f64, Vec<(P, String)>) = match (kind, domain) {
            (NumKind::Log, domain) => {
                let (a, b) = domain.unwrap_or((min, max));
                if a <= 0.0 {
                    return Err(format!(
                        "the {axis} axis is logarithmic but its domain starts at {a}; it must be \
                         above zero"
                    ));
                }
                let (low, high) = (a.log10().floor() as i32, b.log10().ceil() as i32);
                let (lo, hi) = match domain {
                    Some(_) => (a, b),
                    None => (10f64.powi(low), 10f64.powi(high.max(low + 1))),
                };
                let ticks = axis::log_ticks(low, high.max(low + 1))
                    .into_iter()
                    .map(|(e, l)| (P::Num(10f64.powf(e), 0.0), l))
                    .filter(
                        |(p, _)| matches!(p, P::Num(v, _) if *v >= lo * 0.999 && *v <= hi * 1.001),
                    )
                    .collect();
                (lo, hi, ticks)
            }
            (_, Some((a, b))) => {
                let t = Ticks::inside(a, b);
                (a, b, ticked(&t))
            }
            (_, None) if nice => {
                let t = Ticks::covering(min, max);
                (t.min, t.max, ticked(&t))
            }
            (_, None) => {
                let t = Ticks::inside(min, max);
                (min, max, ticked(&t))
            }
        };
        Ok(AxisPlan {
            scale: Scale::Num {
                lo,
                hi,
                kind,
                start: 0.0,
                end: 1.0,
            },
            ticks,
            title,
            hidden,
            grid,
            label_angle,
            reversed,
        })
    }

    fn legends(&self, figure: &mut Figure, frame: &axes::Frame, right: &mut f64, bottom: &mut f64) {
        let fonts = axes::fonts();
        let line = fonts.legend + 4.0;
        let x = frame.right + 16.0;
        let mut y = frame.top;
        let symbol = self
            .units
            .iter()
            .find(|u| u.ch.get("color").is_some_and(Channel::has_field))
            .map(|u| u.mark.kind);
        let shape_field = self
            .units
            .iter()
            .find_map(|u| u.ch.get("shape").and_then(|c| c.field.clone()));
        let color_field = self
            .units
            .iter()
            .find_map(|u| u.ch.get("color").and_then(|c| c.field.clone()));
        let merged = shape_field.is_some() && shape_field == color_field;
        let title_at =
            |figure: &mut Figure, y: &mut f64, right: &mut f64, title: &Option<String>| {
                if let Some(t) = title {
                    figure.text(
                        x,
                        *y,
                        t.clone(),
                        axes::legend_style().align(HAlign::Left, VAlign::Top),
                    );
                    *right = right.max(x + text::width(t, fonts.legend));
                    *y += line + 1.0;
                }
            };
        match &self.colors {
            ColorScale::Nominal { entries, .. } if !self.color_hidden => {
                title_at(figure, &mut y, right, &self.color_title);
                let markers: HashMap<&str, Marker> = match (&self.shapes, merged) {
                    (Some((shapes, _, _)), true) => {
                        shapes.iter().map(|(n, m)| (n.as_str(), *m)).collect()
                    }
                    _ => HashMap::new(),
                };
                for (name, color) in entries.iter().take(60) {
                    let cy = y + line / 2.0;
                    match symbol {
                        Some(MarkKind::Line | MarkKind::Rule) => {
                            figure.line(&[(x, cy), (x + 14.0, cy)], Stroke::solid(*color, 2.0))
                        }
                        Some(MarkKind::Point) => axes::marker_styled(
                            figure,
                            markers
                                .get(name.as_str())
                                .copied()
                                .unwrap_or(Marker::Circle),
                            x + 7.0,
                            cy,
                            40.0,
                            None,
                            Stroke::solid(*color, 1.25),
                        ),
                        Some(MarkKind::Circle | MarkKind::Square | MarkKind::Text) => {
                            let default = if symbol == Some(MarkKind::Square) {
                                Marker::Square
                            } else {
                                Marker::Circle
                            };
                            axes::marker_styled(
                                figure,
                                markers.get(name.as_str()).copied().unwrap_or(default),
                                x + 7.0,
                                cy,
                                40.0,
                                Some(*color),
                                Stroke::solid(*color, 0.5),
                            )
                        }
                        _ => figure.rect(
                            x,
                            cy - 4.0,
                            14.0,
                            8.0,
                            Some(*color),
                            Some(Stroke::solid(Color::grey(0.3), 0.5)),
                        ),
                    }
                    figure.text(
                        x + 19.0,
                        cy,
                        name.clone(),
                        axes::legend_style().align(HAlign::Left, VAlign::Middle),
                    );
                    *right = right.max(x + 19.0 + text::width(name, fonts.legend));
                    y += line;
                }
                if entries.len() > 60 {
                    let more = format!("and {} more", entries.len() - 60);
                    figure.text(
                        x,
                        y + line / 2.0,
                        more,
                        axes::legend_style().align(HAlign::Left, VAlign::Middle),
                    );
                    y += line;
                }
                y += line / 2.0;
            }
            ColorScale::Quantitative {
                low,
                high,
                mid,
                stops,
            } if !self.color_hidden => {
                title_at(figure, &mut y, right, &self.color_title);
                let bar_h = (frame.height() * 0.6).clamp(60.0, 150.0);
                let slices = 50;
                for i in 0..slices {
                    let t = i as f64 / (slices - 1) as f64;
                    let value = high - t * (high - low);
                    let color = ColorScale::Quantitative {
                        low: *low,
                        high: *high,
                        mid: *mid,
                        stops: stops.clone(),
                    }
                    .color(&Cell::Num(value))
                    .unwrap_or(Color::BLACK);
                    let h = bar_h / slices as f64;
                    figure.rect(x, y + i as f64 * h, 12.0, h + 0.3, Some(color), None);
                }
                figure.rect(
                    x,
                    y,
                    12.0,
                    bar_h,
                    None,
                    Some(Stroke::solid(Color::grey(0.5), 0.5)),
                );
                let ticks = Ticks::inside(*low, *high);
                for (v, label) in ticks.positions.iter().zip(ticks.labels()) {
                    let ty = y + (high - v) / (high - low) * bar_h;
                    figure.line(
                        &[(x + 12.0, ty), (x + 15.0, ty)],
                        Stroke::solid(Color::grey(0.5), 0.5),
                    );
                    figure.text(
                        x + 17.0,
                        ty,
                        label.clone(),
                        axes::tick_style().align(HAlign::Left, VAlign::Middle),
                    );
                    *right = right.max(x + 17.0 + text::width(&label, fonts.tick));
                }
                y += bar_h + line;
            }
            _ => {}
        }
        if let Some((shapes, title, hidden)) = &self.shapes {
            if !hidden
                && !(merged
                    && matches!(self.colors, ColorScale::Nominal { .. })
                    && !self.color_hidden)
            {
                title_at(figure, &mut y, right, title);
                for (name, marker) in shapes.iter().take(60) {
                    let cy = y + line / 2.0;
                    axes::marker_styled(
                        figure,
                        *marker,
                        x + 7.0,
                        cy,
                        40.0,
                        None,
                        Stroke::solid(Color::grey(0.2), 1.0),
                    );
                    figure.text(
                        x + 19.0,
                        cy,
                        name.clone(),
                        axes::legend_style().align(HAlign::Left, VAlign::Middle),
                    );
                    *right = right.max(x + 19.0 + text::width(name, fonts.legend));
                    y += line;
                }
            }
        }
        *bottom = y;
    }

    /// The marks of one layer.
    fn unit_items(
        &self,
        unit: &Unit,
        items: &mut Vec<Item>,
        notes: &mut Notes,
    ) -> Result<(), String> {
        let t = &unit.table;
        let mark = &unit.mark;
        let ch = &unit.ch;
        let get = |name: &str, row: usize| -> Cell {
            match ch.get(name) {
                Some(c) => match (&c.field, &c.datum) {
                    (Some(f), _) => t.get(row, f),
                    (None, Some(d)) => d.clone(),
                    _ => Cell::Null,
                },
                None => Cell::Null,
            }
        };
        let ty = |name: &str| ch.get(name).and_then(|c| c.ty);
        let is_band = |name: &str| {
            ch.get(name)
                .is_some_and(|c| (c.has_field() || c.datum.is_some()) && c.ty == Some(Ty::Nominal))
        };
        let positioned = |name: &str| {
            ch.get(name)
                .is_some_and(|c| c.has_field() || c.datum.is_some())
        };

        // Sub-bands for xOffset / yOffset.
        let offsets = |name: &str| -> Result<Option<Vec<String>>, String> {
            let Some(c) = ch.get(name) else {
                return Ok(None);
            };
            let Some(f) = &c.field else { return Ok(None) };
            let col = t.field(f)?;
            let mut cats = Vec::new();
            let mut seen = HashSet::new();
            for row in &t.rows {
                let v = row[col].to_text();
                if seen.insert(v.clone()) {
                    cats.push(v);
                }
            }
            Ok(Some(order_categories(cats, &sort_order(unit, name)?)))
        };
        let x_offsets = offsets("xOffset")?;
        let y_offsets = offsets("yOffset")?;
        // The share of a band a row has: (start, end) between 0 and 1.
        let slot = |row: usize, along_x: bool| -> (f64, f64) {
            let (cats, name) = if along_x {
                (&x_offsets, "xOffset")
            } else {
                (&y_offsets, "yOffset")
            };
            match cats {
                Some(cats) if !cats.is_empty() => {
                    let j = cats
                        .iter()
                        .position(|c| *c == get(name, row).to_text())
                        .unwrap_or(0) as f64;
                    let w = 0.8 / cats.len() as f64;
                    (0.1 + j * w + w * 0.05, 0.1 + (j + 1.0) * w - w * 0.05)
                }
                _ => (0.1, 0.9),
            }
        };
        let position = |name: &str, row: usize| -> Option<P> {
            let along_x = name.starts_with('x');
            let c = ch.get(name)?;
            if !c.has_field() && c.datum.is_none() {
                return None;
            }
            let v = get(name, row);
            if c.ty == Some(Ty::Nominal) {
                if !v.is_valid() {
                    return None;
                }
                let (a, b) = slot(row, along_x);
                Some(P::Band(v.to_text(), (a + b) / 2.0))
            } else {
                Some(P::Num(v.finite().unwrap_or(f64::NAN), 0.0))
            }
        };

        let opacity_of = |row: usize| -> f64 {
            if let Some(c) = ch.get("opacity") {
                if let Some(v) = c.value.as_ref().and_then(Value::as_f64) {
                    return v;
                }
                if let Some(f) = &c.field {
                    let (lo, hi) = range_of(t, f);
                    if let Some(v) = t.get(row, f).finite() {
                        return if hi > lo {
                            0.3 + 0.7 * (v - lo) / (hi - lo)
                        } else {
                            1.0
                        };
                    }
                }
            }
            mark.number("opacity").unwrap_or(1.0)
        };
        let base_color = || {
            mark.color("color")
                .or_else(|| mark.color("fill"))
                .or_else(|| mark.color("stroke"))
        };
        let color_of = |row: usize, default: Color| -> Color {
            let color = match ch.get("color") {
                Some(c) => match (&c.value, c.has_field()) {
                    (Some(v), _) => v.as_str().and_then(parse_color),
                    (None, true) => self.colors.color(&get("color", row)),
                    _ => None,
                },
                None => None,
            }
            .or_else(base_color)
            .unwrap_or(default);
            with_opacity(color, opacity_of(row))
        };
        let size_of = |row: usize, default: f64| -> f64 {
            if let Some(c) = ch.get("size") {
                if let Some(v) = c.value.as_ref().and_then(Value::as_f64) {
                    return v;
                }
                if let Some(f) = &c.field {
                    let (lo, hi) = range_of(t, f);
                    let (a, b) = match c.scale.get("range") {
                        Some(Value::Array(r)) if r.len() == 2 => {
                            (r[0].as_f64().unwrap_or(9.0), r[1].as_f64().unwrap_or(361.0))
                        }
                        _ => (9.0, 361.0),
                    };
                    let lo = if c.scale.get("zero").and_then(Value::as_bool) == Some(false) {
                        lo
                    } else {
                        lo.min(0.0)
                    };
                    if let Some(v) = t.get(row, f).finite() {
                        return if hi > lo {
                            a + (b - a) * (v - lo) / (hi - lo)
                        } else {
                            b
                        };
                    }
                }
            }
            mark.number("size").unwrap_or(default)
        };
        let shape_of = |row: usize, default: Marker| -> Marker {
            if let Some(c) = ch.get("shape") {
                if let Some(v) = c.value.as_ref().and_then(Value::as_str) {
                    return marker_named(v).unwrap_or(default);
                }
                if c.has_field() {
                    if let Some((shapes, _, _)) = &self.shapes {
                        let v = get("shape", row).to_text();
                        if let Some((_, m)) = shapes.iter().find(|(n, _)| *n == v) {
                            return *m;
                        }
                    }
                }
            }
            mark.string("shape")
                .and_then(marker_named)
                .unwrap_or(default)
        };
        let dash_of = |row: usize| -> Option<(f64, f64)> {
            if let Some(c) = ch.get("strokeDash") {
                if let Some(Value::Array(d)) = &c.value {
                    return dash_from(d);
                }
                if let Some(f) = &c.field {
                    let mut cats: Vec<String> = t
                        .rows
                        .iter()
                        .map(|r| r[t.column(f).unwrap_or(0)].to_text())
                        .collect();
                    cats.sort();
                    cats.dedup();
                    let i = cats
                        .iter()
                        .position(|c| *c == get("strokeDash", row).to_text())
                        .unwrap_or(0);
                    return [
                        None,
                        Some((4.0, 2.0)),
                        Some((1.5, 1.5)),
                        Some((6.0, 2.0)),
                        Some((2.0, 3.0)),
                    ][i % 5];
                }
            }
            match mark.props.get("strokeDash") {
                Some(Value::Array(d)) => dash_from(d),
                _ => None,
            }
        };
        let stroke_width = mark.number("strokeWidth");
        let tip = |row: usize| -> Option<(String, String)> {
            let fields = unit.tooltip.as_ref()?;
            if fields.is_empty() {
                return None;
            }
            let lines: Vec<String> = fields
                .iter()
                .filter(|(f, _)| t.column(f).is_some())
                .map(|(f, title)| format!("{}: {}", title, t.get(row, f).to_text()))
                .collect();
            let item = match t.column("feature") {
                Some(c) => t.rows[row][c].to_text(),
                None => String::new(),
            };
            Some((item, lines.join("\n")))
        };

        let rows = 0..t.rows.len();
        match mark.kind {
            MarkKind::Bar | MarkKind::Rect => {
                let rect = mark.kind == MarkKind::Rect;
                let horizontal = if rect {
                    false
                } else {
                    match mark.string("orient") {
                        Some("horizontal") => true,
                        Some("vertical") => false,
                        _ => is_band("y") && !is_band("x"),
                    }
                };
                if !rect && is_band("x") && is_band("y") {
                    return Err(
                        "a bar needs one quantitative axis; use the rect mark for a heatmap"
                            .to_string(),
                    );
                }
                let (band, value) = if horizontal { ("y", "x") } else { ("x", "y") };
                let value2 = format!("{value}2");
                let stacking = stack_mode(unit, value, &value2, band, horizontal);
                let stacks = match stacking {
                    Some(mode) => Some(stack(unit, band, value, mode, &get, self)?),
                    None => None,
                };
                if rect {
                    for axis in ["x", "y"] {
                        if !is_band(axis) && !positioned(&format!("{axis}2")) {
                            return Err(format!(
                                "a rect needs {axis}2 when {axis} is quantitative; for a heatmap \
                                 give x and y nominal fields"
                            ));
                        }
                    }
                }
                let default = DEFAULT_COLOR;
                let outline = mark
                    .color("stroke")
                    .map(|c| Stroke::solid(c, stroke_width.unwrap_or(1.0)));
                for row in rows {
                    let span = |axis: &str, along_x: bool| -> Option<(P, P)> {
                        let second = format!("{axis}2");
                        if is_band(axis) {
                            let v = get(axis, row);
                            if !v.is_valid() {
                                return None;
                            }
                            let (a, b) = if rect { (0.0, 1.0) } else { slot(row, along_x) };
                            let (a, b) = if rect {
                                (a, b)
                            } else {
                                match mark
                                    .props
                                    .get("width")
                                    .and_then(|w| w.get("band"))
                                    .and_then(Value::as_f64)
                                {
                                    Some(f) => {
                                        let c = (a + b) / 2.0;
                                        let h = (b - a) / 2.0 * f / 0.8;
                                        (c - h, c + h)
                                    }
                                    None => (a, b),
                                }
                            };
                            return Some((P::Band(v.to_text(), a), P::Band(v.to_text(), b)));
                        }
                        let v = get(axis, row).finite()?;
                        if positioned(&second) {
                            let w = get(&second, row).finite()?;
                            return Some((P::Num(v, 0.0), P::Num(w, 0.0)));
                        }
                        if rect {
                            return None;
                        }
                        let half = mark
                            .props
                            .get("width")
                            .and_then(Value::as_f64)
                            .or(mark.number("size"))
                            .unwrap_or(5.0)
                            / 2.0;
                        Some((P::Num(v, -half), P::Num(v, half)))
                    };
                    let (bx, by) = if rect || !positioned(value) {
                        (span("x", true), span("y", false))
                    } else {
                        let along = span(band, !horizontal);
                        let across = match &stacks {
                            Some(s) => s.get(&row).map(|(a, b)| (P::Num(*a, 0.0), P::Num(*b, 0.0))),
                            None => {
                                let v = get(value, row).finite();
                                match v {
                                    None => None,
                                    Some(v) if positioned(&value2) => get(&value2, row)
                                        .finite()
                                        .map(|w| (P::Num(w, 0.0), P::Num(v, 0.0))),
                                    Some(v) => Some((P::Base, P::Num(v, 0.0))),
                                }
                            }
                        };
                        if horizontal {
                            (across, along)
                        } else {
                            (along, across)
                        }
                    };
                    let (Some(x), Some(y)) = (bx, by) else {
                        continue;
                    };
                    items.push(Item {
                        prim: Prim::Rect {
                            x,
                            y,
                            fill: Some(color_of(row, default)),
                            stroke: outline,
                        },
                        tip: tip(row),
                    });
                }
            }
            MarkKind::Point | MarkKind::Circle | MarkKind::Square => {
                let filled = mark.bool("filled").unwrap_or(mark.kind != MarkKind::Point);
                let default_shape = if mark.kind == MarkKind::Square {
                    Marker::Square
                } else {
                    Marker::Circle
                };
                for row in rows {
                    let x = position("x", row).unwrap_or(P::Frac(0.5));
                    let y = position("y", row).unwrap_or(P::Frac(0.5));
                    let color = color_of(row, DEFAULT_COLOR);
                    let (fill, stroke) = if filled {
                        (
                            Some(color),
                            Some(Stroke::solid(
                                with_opacity(Color::grey(0.15), opacity_of(row)),
                                0.4,
                            )),
                        )
                    } else {
                        (
                            None,
                            Some(Stroke::solid(color, stroke_width.unwrap_or(1.25))),
                        )
                    };
                    items.push(Item {
                        prim: Prim::Marker {
                            x,
                            y,
                            shape: shape_of(row, default_shape),
                            area: size_of(row, 30.0),
                            fill,
                            stroke,
                        },
                        tip: tip(row),
                    });
                }
            }
            MarkKind::Text => {
                let size = mark.number("fontSize").unwrap_or(axes::fonts().tick);
                let h = match mark.string("align") {
                    Some("left") => HAlign::Left,
                    Some("right") => HAlign::Right,
                    _ => HAlign::Center,
                };
                let v = match mark.string("baseline") {
                    Some("top") => VAlign::Top,
                    Some("bottom") => VAlign::Bottom,
                    Some("alphabetic") => VAlign::Baseline,
                    _ => VAlign::Middle,
                };
                for row in rows {
                    let text = match ch.get("text") {
                        Some(c) if c.has_field() || c.datum.is_some() => get("text", row).to_text(),
                        Some(c) => c
                            .value
                            .as_ref()
                            .map(|v| Cell::from_json(v).to_text())
                            .unwrap_or_default(),
                        None => mark.string("text").unwrap_or("").to_string(),
                    };
                    if text.is_empty() {
                        continue;
                    }
                    let style = TextStyle::new(size)
                        .align(h, v)
                        .color(color_of(row, Color::BLACK))
                        .rotated(-mark.number("angle").unwrap_or(0.0));
                    items.push(Item {
                        prim: Prim::Text {
                            x: position("x", row).unwrap_or(P::Frac(0.5)),
                            y: position("y", row).unwrap_or(P::Frac(0.5)),
                            text,
                            style,
                            offset: (
                                mark.number("dx").unwrap_or(0.0),
                                mark.number("dy").unwrap_or(0.0),
                            ),
                        },
                        tip: tip(row),
                    });
                }
            }
            MarkKind::Rule => {
                for row in rows {
                    let stroke = Stroke {
                        color: color_of(row, Color::BLACK),
                        width: stroke_width.unwrap_or(1.0),
                        dash: dash_of(row),
                    };
                    let (x, y) = (position("x", row), position("y", row));
                    let (x2, y2) = (position("x2", row), position("y2", row));
                    let (from, to) = match (x, y, x2.clone(), y2.clone()) {
                        (Some(x), None, _, None) => {
                            ((x.clone(), P::Frac(0.0)), (x2.unwrap_or(x), P::Frac(1.0)))
                        }
                        (None, Some(y), None, _) => {
                            ((P::Frac(0.0), y.clone()), (P::Frac(1.0), y2.unwrap_or(y)))
                        }
                        (Some(x), Some(y), x2, y2) if x2.is_some() || y2.is_some() => {
                            ((x.clone(), y.clone()), (x2.unwrap_or(x), y2.unwrap_or(y)))
                        }
                        (Some(x), Some(y), None, None) => {
                            // A rule at a category: across its band.
                            match (&x, &y) {
                                (P::Band(c, _), _) => (
                                    (P::Band(c.clone(), 0.1), y.clone()),
                                    (P::Band(c.clone(), 0.9), y),
                                ),
                                (_, P::Band(c, _)) => (
                                    (x.clone(), P::Band(c.clone(), 0.1)),
                                    (x, P::Band(c.clone(), 0.9)),
                                ),
                                _ => continue,
                            }
                        }
                        _ => continue,
                    };
                    items.push(Item {
                        prim: Prim::Segment { from, to, stroke },
                        tip: tip(row),
                    });
                }
            }
            MarkKind::Tick => {
                let thickness = mark.number("thickness").unwrap_or(1.0);
                for row in rows {
                    let stroke = Stroke::solid(color_of(row, DEFAULT_COLOR), thickness);
                    let (x, y) = (position("x", row), position("y", row));
                    let vertical_axis_band = is_band("x") || !positioned("x");
                    let (from, to) = if vertical_axis_band && ty("y") == Some(Ty::Quantitative) {
                        let Some(y) = y else { continue };
                        match x {
                            Some(P::Band(c, _)) => {
                                let (a, b) = slot(row, true);
                                (
                                    (P::Band(c.clone(), a + 0.15), y.clone()),
                                    (P::Band(c, b - 0.15), y),
                                )
                            }
                            _ => ((P::Frac(0.4), y.clone()), (P::Frac(0.6), y)),
                        }
                    } else {
                        let Some(x) = x else { continue };
                        match y {
                            Some(P::Band(c, _)) => {
                                let (a, b) = slot(row, false);
                                (
                                    (x.clone(), P::Band(c.clone(), a + 0.15)),
                                    (x, P::Band(c, b - 0.15)),
                                )
                            }
                            _ => ((x.clone(), P::Frac(0.4)), (x, P::Frac(0.6))),
                        }
                    };
                    items.push(Item {
                        prim: Prim::Segment { from, to, stroke },
                        tip: tip(row),
                    });
                }
            }
            MarkKind::Line | MarkKind::Area => {
                let by_y =
                    mark.string("orient") == Some("horizontal") || (is_band("y") && !is_band("x"));
                let group_fields: Vec<&str> = ["color", "detail", "strokeDash", "opacity"]
                    .into_iter()
                    .filter(|n| {
                        ch.get(*n)
                            .is_some_and(|c| c.has_field() && c.ty == Some(Ty::Nominal))
                    })
                    .collect();
                let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
                let mut index: HashMap<String, usize> = HashMap::new();
                for row in rows {
                    let key: String = group_fields
                        .iter()
                        .map(|n| get(n, row).to_text())
                        .collect::<Vec<_>>()
                        .join("\u{1f}");
                    let at = *index.entry(key.clone()).or_insert_with(|| {
                        groups.push((key, Vec::new()));
                        groups.len() - 1
                    });
                    groups[at].1.push(row);
                }
                let (indep, dep) = if by_y { ("y", "x") } else { ("x", "y") };
                let stacked = mark.kind == MarkKind::Area
                    && stack_mode(unit, dep, &format!("{dep}2"), indep, by_y).is_some();
                let stacks = if stacked {
                    Some(stack(
                        unit,
                        indep,
                        dep,
                        stack_mode(unit, dep, &format!("{dep}2"), indep, by_y)
                            .unwrap_or(StackMode::Zero),
                        &get,
                        self,
                    )?)
                } else {
                    None
                };
                let several = groups.len() > 1;
                for (_, group) in groups {
                    let first = group[0];
                    let color = color_of(first, DEFAULT_COLOR);
                    let mut points = Vec::new();
                    let mut area_points = Vec::new();
                    for &row in &group {
                        let (Some(a), Some(b)) = (position(indep, row), position(dep, row)) else {
                            continue;
                        };
                        if mark.kind == MarkKind::Area {
                            let (top, bottom) = match &stacks {
                                Some(s) => match s.get(&row) {
                                    Some((lo, hi)) => (P::Num(*hi, 0.0), P::Num(*lo, 0.0)),
                                    None => continue,
                                },
                                None => match position(&format!("{dep}2"), row) {
                                    Some(b2) => (b.clone(), b2),
                                    None => (b.clone(), P::Base),
                                },
                            };
                            area_points.push((a.clone(), top, bottom));
                        } else {
                            points.push(if by_y {
                                (b.clone(), a.clone())
                            } else {
                                (a.clone(), b.clone())
                            });
                        }
                        if unit.tooltip.as_ref().is_some_and(|t| !t.is_empty()) {
                            let spot = if by_y {
                                (position(dep, row), position(indep, row))
                            } else {
                                (position(indep, row), position(dep, row))
                            };
                            if let (Some(sx), Some(sy)) = spot {
                                let sy = match (&stacks, by_y) {
                                    (Some(s), false) => {
                                        s.get(&row).map_or(sy, |(_, hi)| P::Num(*hi, 0.0))
                                    }
                                    _ => sy,
                                };
                                items.push(Item {
                                    prim: Prim::Spot { x: sx, y: sy },
                                    tip: tip(row),
                                });
                            }
                        }
                    }
                    if mark.kind == MarkKind::Area {
                        if by_y {
                            notes.add("areas are drawn along the x axis only");
                            continue;
                        }
                        let opacity = mark.number("opacity").unwrap_or(if several && !stacked {
                            0.6
                        } else {
                            1.0
                        });
                        let mut fill = color;
                        if mark.number("opacity").is_none() {
                            fill = with_opacity(fill, opacity);
                        }
                        items.push(Item {
                            prim: Prim::Area {
                                points: area_points,
                                fill,
                            },
                            tip: None,
                        });
                    } else {
                        let stroke = Stroke {
                            color,
                            width: stroke_width.unwrap_or(1.5),
                            dash: dash_of(first),
                        };
                        let with_points = mark.bool("point").unwrap_or(false);
                        if with_points {
                            for (x, y) in &points {
                                items.push(Item {
                                    prim: Prim::Marker {
                                        x: x.clone(),
                                        y: y.clone(),
                                        shape: Marker::Circle,
                                        area: 20.0,
                                        fill: Some(color),
                                        stroke: None,
                                    },
                                    tip: None,
                                });
                            }
                        }
                        items.insert(
                            items.len() - if with_points { points.len() } else { 0 },
                            Item {
                                prim: Prim::Line {
                                    points,
                                    stroke,
                                    by_y,
                                },
                                tip: None,
                            },
                        );
                    }
                }
            }
            MarkKind::Boxplot | MarkKind::Errorbar => {
                self.summary_items(unit, items, notes)?;
            }
        }
        Ok(())
    }

    /// Box plots and error bars: summaries of the values of each category.
    fn summary_items(
        &self,
        unit: &Unit,
        items: &mut Vec<Item>,
        notes: &mut Notes,
    ) -> Result<(), String> {
        let t = &unit.table;
        let ch = &unit.ch;
        let mark = &unit.mark;
        let nominal = |n: &str| {
            ch.get(n)
                .is_some_and(|c| c.has_field() && c.ty == Some(Ty::Nominal))
        };
        let horizontal = match mark.string("orient") {
            Some("horizontal") => true,
            Some("vertical") => false,
            _ => nominal("y") && !nominal("x"),
        };
        let (band, value) = if horizontal { ("y", "x") } else { ("x", "y") };
        let value_field = ch.get(value).and_then(|c| c.field.clone()).ok_or_else(|| {
            format!(
                "a {} needs a quantitative {value} field",
                if mark.kind == MarkKind::Boxplot {
                    "boxplot"
                } else {
                    "errorbar"
                }
            )
        })?;
        let value_col = t.field(&value_field)?;
        let band_col = match ch.get(band).and_then(|c| c.field.as_ref()) {
            Some(f) => Some(t.field(f)?),
            None => None,
        };
        let color_col = match ch.get("color").and_then(|c| c.field.as_ref()) {
            Some(f) => Some(t.field(f)?),
            None => None,
        };
        let offset_name = if horizontal { "yOffset" } else { "xOffset" };
        let offset_col = match ch.get(offset_name).and_then(|c| c.field.as_ref()) {
            Some(f) => Some(t.field(f)?),
            None => None,
        };
        let offsets: Option<Vec<String>> = match offset_col {
            Some(c) => {
                let mut cats: Vec<String> = Vec::new();
                for row in &t.rows {
                    let v = row[c].to_text();
                    if !cats.contains(&v) {
                        cats.push(v);
                    }
                }
                Some(order_categories(cats, &sort_order(unit, offset_name)?))
            }
            None => None,
        };
        let mut keys: Vec<usize> = Vec::new();
        keys.extend(band_col);
        keys.extend(color_col);
        keys.extend(offset_col);
        keys.dedup();
        let default_fill = mark.color("color").unwrap_or(DEFAULT_COLOR);
        let second = format!("{value}2");
        let error_name = format!("{value}Error");
        let error2_name = format!("{value}Error2");
        let pre_computed = ch.get(&second).is_some_and(Channel::has_field)
            || ch.get(&error_name).is_some_and(Channel::has_field);

        let place = |cat: &str, a: f64, value: P| -> (P, P) {
            let along = P::Band(cat.to_string(), a);
            if horizontal {
                (value, along)
            } else {
                (along, value)
            }
        };
        let thickness = mark
            .number("thickness")
            .or(mark.number("strokeWidth"))
            .unwrap_or(1.0);

        if mark.kind == MarkKind::Errorbar && pre_computed {
            for row in 0..t.rows.len() {
                let cat = band_col.map_or(String::new(), |c| t.rows[row][c].to_text());
                let centre = slot_of(&offsets, offset_col.map(|c| t.rows[row][c].to_text()));
                let Some(v) = t.rows[row][value_col].finite() else {
                    continue;
                };
                let (lo, hi) = if let Some(f) = ch.get(&second).and_then(|c| c.field.as_ref()) {
                    let Some(w) = t.get(row, f).finite() else {
                        continue;
                    };
                    (v.min(w), v.max(w))
                } else {
                    let f = ch[&error_name].field.as_deref().unwrap_or("");
                    let Some(e) = t.get(row, f).finite() else {
                        continue;
                    };
                    let e2 = ch
                        .get(&error2_name)
                        .and_then(|c| c.field.as_ref())
                        .and_then(|f| t.get(row, f).finite())
                        .map_or(e.abs(), f64::abs);
                    (v - e2, v + e.abs())
                };
                let color = color_col
                    .and_then(|_| {
                        self.colors
                            .color(&t.get(row, ch["color"].field.as_deref().unwrap_or("")))
                    })
                    .or(mark.color("color"))
                    .unwrap_or(Color::BLACK);
                error_bar(
                    items,
                    &place,
                    &cat,
                    centre,
                    lo,
                    hi,
                    color,
                    thickness,
                    mark.bool("ticks").unwrap_or(false),
                    Some((
                        t.column("feature")
                            .map_or(String::new(), |c| t.rows[row][c].to_text()),
                        format!(
                            "{cat}\n{value_field}: {}\nfrom {} to {}",
                            data::format_number(v),
                            data::format_number(lo),
                            data::format_number(hi)
                        )
                        .trim_start()
                        .to_string(),
                    )),
                );
            }
            return Ok(());
        }

        let extent = mark.props.get("extent").cloned();
        for rows in data::group_rows(t, &keys) {
            let first = rows[0];
            let cat = band_col.map_or(String::new(), |c| t.rows[first][c].to_text());
            if band_col.is_some() && !t.rows[first][band_col.unwrap_or(0)].is_valid() {
                continue;
            }
            let centre = slot_of(&offsets, offset_col.map(|c| t.rows[first][c].to_text()));
            let mut values: Vec<f64> = rows
                .iter()
                .filter_map(|&r| t.rows[r][value_col].finite())
                .collect();
            if values.is_empty() {
                continue;
            }
            values.sort_by(f64::total_cmp);
            let fill = color_col
                .and_then(|c| self.colors.color(&t.rows[first][c]))
                .unwrap_or(default_fill);
            let label = {
                let mut parts = Vec::new();
                for &k in &keys {
                    parts.push(format!("{}: {}", t.columns[k], t.rows[first][k].to_text()));
                }
                parts.join("\n")
            };
            let item = t
                .column("feature")
                .filter(|c| keys.contains(c))
                .map_or(String::new(), |c| t.rows[first][c].to_text());
            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;
            let sd = (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
            if mark.kind == MarkKind::Errorbar {
                let (centre_value, lo, hi, what) = match extent
                    .as_ref()
                    .and_then(Value::as_str)
                    .unwrap_or("stderr")
                {
                    "stdev" => (mean, mean - sd, mean + sd, "mean ± standard deviation"),
                    "ci" => {
                        notes.add("confidence intervals of error bars are mean ± 1.96 standard errors (a normal approximation)");
                        let half = 1.96 * sd / n.sqrt();
                        (
                            mean,
                            mean - half,
                            mean + half,
                            "mean ± 1.96 standard errors",
                        )
                    }
                    "iqr" => (
                        data::quantile(&values, 0.5),
                        data::quantile(&values, 0.25),
                        data::quantile(&values, 0.75),
                        "median and quartiles",
                    ),
                    _ => {
                        let half = sd / n.sqrt();
                        (mean, mean - half, mean + half, "mean ± standard error")
                    }
                };
                let (lo, hi) = if n < 2.0 && what != "median and quartiles" {
                    (centre_value, centre_value)
                } else {
                    (lo, hi)
                };
                let color = color_col
                    .and_then(|c| self.colors.color(&t.rows[first][c]))
                    .or(mark.color("color"))
                    .unwrap_or(Color::BLACK);
                error_bar(
                    items,
                    &place,
                    &cat,
                    centre,
                    lo,
                    hi,
                    color,
                    thickness,
                    mark.bool("ticks").unwrap_or(false),
                    Some((
                        item,
                        format!(
                            "{label}\n{what} of {value_field}\n{}: {}, from {} to {}, n = {}",
                            if what.starts_with("median") {
                                "median"
                            } else {
                                "mean"
                            },
                            data::format_number(centre_value),
                            data::format_number(lo),
                            data::format_number(hi),
                            values.len()
                        )
                        .trim_start()
                        .to_string(),
                    )),
                );
                continue;
            }
            let (q1, median, q3) = (
                data::quantile(&values, 0.25),
                data::quantile(&values, 0.5),
                data::quantile(&values, 0.75),
            );
            let (low, high) = match &extent {
                Some(Value::String(s)) if s == "min-max" => (values[0], values[values.len() - 1]),
                other => {
                    let k = other.as_ref().and_then(Value::as_f64).unwrap_or(1.5);
                    let iqr = q3 - q1;
                    let low = values
                        .iter()
                        .copied()
                        .find(|v| *v >= q1 - k * iqr)
                        .unwrap_or(q1);
                    let high = values
                        .iter()
                        .rev()
                        .copied()
                        .find(|v| *v <= q3 + k * iqr)
                        .unwrap_or(q3);
                    (low, high)
                }
            };
            let width = 0.6;
            let (a, b) = (
                centre - width / 2.0 * slot_width(&offsets),
                centre + width / 2.0 * slot_width(&offsets),
            );
            let tooltip = format!(
                "{label}\n{value_field}\nmaximum: {}\nthird quartile: {}\nmedian: {}\nfirst quartile: {}\nminimum: {}\nsamples: {}",
                data::format_number(values[values.len() - 1]),
                data::format_number(q3),
                data::format_number(median),
                data::format_number(q1),
                data::format_number(values[0]),
                values.len()
            ).trim_start().to_string();
            let edge = Stroke::solid(Color::grey(0.15), 1.0);
            for (from, to) in [(low, q1), (q3, high)] {
                let (p, q) = (
                    place(&cat, centre, P::Num(from, 0.0)),
                    place(&cat, centre, P::Num(to, 0.0)),
                );
                items.push(Item {
                    prim: Prim::Segment {
                        from: p,
                        to: q,
                        stroke: edge,
                    },
                    tip: None,
                });
            }
            for end in [low, high] {
                let mid_a = centre - (b - a) / 4.0;
                let mid_b = centre + (b - a) / 4.0;
                items.push(Item {
                    prim: Prim::Segment {
                        from: place(&cat, mid_a, P::Num(end, 0.0)),
                        to: place(&cat, mid_b, P::Num(end, 0.0)),
                        stroke: edge,
                    },
                    tip: None,
                });
            }
            let (bx, by) = {
                let (p1, p2) = (
                    place(&cat, a, P::Num(q1, 0.0)),
                    place(&cat, b, P::Num(q3, 0.0)),
                );
                ((p1.0, p2.0), (p1.1, p2.1))
            };
            items.push(Item {
                prim: Prim::Rect {
                    x: bx,
                    y: by,
                    fill: Some(fill),
                    stroke: Some(edge),
                },
                tip: Some((item.clone(), tooltip)),
            });
            items.push(Item {
                prim: Prim::Segment {
                    from: place(&cat, a, P::Num(median, 0.0)),
                    to: place(&cat, b, P::Num(median, 0.0)),
                    stroke: Stroke::solid(Color::BLACK, 1.5),
                },
                tip: None,
            });
            for &r in &rows {
                let Some(v) = t.rows[r][value_col].finite() else {
                    continue;
                };
                if v < low || v > high {
                    let (x, y) = place(&cat, centre, P::Num(v, 0.0));
                    let item = t
                        .column("feature")
                        .map_or(String::new(), |c| t.rows[r][c].to_text());
                    let name = t
                        .column("sample")
                        .map(|c| format!("{}\n", t.rows[r][c].to_text()))
                        .unwrap_or_default();
                    items.push(Item {
                        prim: Prim::Marker {
                            x,
                            y,
                            shape: Marker::Circle,
                            area: 16.0,
                            fill: None,
                            stroke: Some(Stroke::solid(Color::grey(0.2), 0.75)),
                        },
                        tip: Some((
                            item,
                            format!("{name}{value_field}: {}", data::format_number(v)),
                        )),
                    });
                }
            }
        }
        Ok(())
    }
}

/// The centre of a row's share of its band (0 to 1).
fn slot_of(offsets: &Option<Vec<String>>, value: Option<String>) -> f64 {
    match (offsets, value) {
        (Some(cats), Some(v)) if !cats.is_empty() => {
            let j = cats.iter().position(|c| *c == v).unwrap_or(0) as f64;
            let w = 0.8 / cats.len() as f64;
            0.1 + (j + 0.5) * w
        }
        _ => 0.5,
    }
}

/// The share of a band each sub-band takes.
fn slot_width(offsets: &Option<Vec<String>>) -> f64 {
    match offsets {
        Some(cats) if !cats.is_empty() => 0.8 / cats.len() as f64 / 0.6 * 0.5,
        _ => 0.8 / 0.6 * 0.5,
    }
}

#[allow(clippy::too_many_arguments)]
fn error_bar(
    items: &mut Vec<Item>,
    place: &dyn Fn(&str, f64, P) -> (P, P),
    cat: &str,
    centre: f64,
    lo: f64,
    hi: f64,
    color: Color,
    width: f64,
    ticks: bool,
    tip: Option<(String, String)>,
) {
    let stroke = Stroke::solid(color, width);
    items.push(Item {
        prim: Prim::Segment {
            from: place(cat, centre, P::Num(lo, 0.0)),
            to: place(cat, centre, P::Num(hi, 0.0)),
            stroke,
        },
        tip,
    });
    if ticks {
        for end in [lo, hi] {
            items.push(Item {
                prim: Prim::Segment {
                    from: place(cat, centre - 0.12, P::Num(end, 0.0)),
                    to: place(cat, centre + 0.12, P::Num(end, 0.0)),
                    stroke,
                },
                tip: None,
            });
        }
    }
}

fn dash_from(d: &[Value]) -> Option<(f64, f64)> {
    let numbers: Vec<f64> = d.iter().filter_map(Value::as_f64).collect();
    match numbers.as_slice() {
        [] => None,
        [a] => Some((*a, *a)),
        [a, b, ..] => Some((*a, *b)),
    }
}

fn range_of(t: &Table, field: &str) -> (f64, f64) {
    let Some(c) = t.column(field) else {
        return (0.0, 1.0);
    };
    t.rows
        .iter()
        .filter_map(|r| r[c].finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(v), b.max(v))
        })
}

fn ticked(t: &Ticks) -> Vec<(P, String)> {
    t.positions
        .iter()
        .zip(t.labels())
        .map(|(v, l)| (P::Num(*v, 0.0), l))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackMode {
    Zero,
    Normalize,
}

/// Whether the bars or areas of a layer are stacked along `value`.
fn stack_mode(
    unit: &Unit,
    value: &str,
    value2: &str,
    band: &str,
    _horizontal: bool,
) -> Option<StackMode> {
    let c = unit.ch.get(value)?;
    if c.ty != Some(Ty::Quantitative) || unit.ch.get(value2).is_some_and(Channel::has_field) {
        return None;
    }
    match &c.stack {
        Some(Value::Null) | Some(Value::Bool(false)) => return None,
        Some(Value::String(s)) if s == "normalize" => return Some(StackMode::Normalize),
        Some(Value::String(_)) | Some(Value::Bool(true)) => return Some(StackMode::Zero),
        _ => {}
    }
    if unit
        .ch
        .contains_key(if band == "x" { "xOffset" } else { "yOffset" })
    {
        return None;
    }
    let band_field = unit.ch.get(band).and_then(|c| c.field.as_ref());
    let grouped = ["color", "detail", "opacity"].iter().any(|n| {
        unit.ch.get(*n).is_some_and(|c| {
            c.has_field() && c.ty == Some(Ty::Nominal) && c.field.as_ref() != band_field
        })
    });
    grouped.then_some(StackMode::Zero)
}

/// Where each row's piece of a stack begins and ends, rows taken in the
/// order of the colour scale.
fn stack(
    unit: &Unit,
    band: &str,
    value: &str,
    mode: StackMode,
    get: &dyn Fn(&str, usize) -> Cell,
    builder: &Builder,
) -> Result<HashMap<usize, (f64, f64)>, String> {
    let t = &unit.table;
    let color_rank = |row: usize| -> usize {
        match (
            &builder.colors,
            unit.ch.get("color").filter(|c| c.has_field()),
        ) {
            (ColorScale::Nominal { entries, .. }, Some(_)) => {
                let v = get("color", row).to_text();
                entries
                    .iter()
                    .position(|(n, _)| *n == v)
                    .unwrap_or(usize::MAX)
            }
            _ => 0,
        }
    };
    let mut order: Vec<usize> = (0..t.rows.len()).collect();
    order.sort_by_key(|&r| color_rank(r));
    let mut totals: HashMap<String, f64> = HashMap::new();
    if mode == StackMode::Normalize {
        for &r in &order {
            if let Some(v) = get(value, r).finite() {
                *totals.entry(get(band, r).to_text()).or_default() += v.abs();
            }
        }
    }
    let mut ends: HashMap<String, (f64, f64)> = HashMap::new();
    let mut out = HashMap::new();
    for r in order {
        let Some(mut v) = get(value, r).finite() else {
            continue;
        };
        let key = get(band, r).to_text();
        if mode == StackMode::Normalize {
            let total = totals.get(&key).copied().unwrap_or(0.0);
            v = if total > 0.0 { v / total } else { 0.0 };
        }
        let (pos, neg) = ends.entry(key).or_insert((0.0, 0.0));
        if v >= 0.0 {
            out.insert(r, (*pos, *pos + v));
            *pos += v;
        } else {
            out.insert(r, (*neg + v, *neg));
            *neg += v;
        }
    }
    Ok(out)
}

/// Visit the positions of a mark along one axis.
fn for_positions(prim: &Prim, x_axis: bool, visit: &mut dyn FnMut(&P)) {
    let pick = |pair: &(P, P)| -> P {
        if x_axis {
            pair.0.clone()
        } else {
            pair.1.clone()
        }
    };
    match prim {
        Prim::Rect { x, y, .. } => {
            let span = if x_axis { x } else { y };
            visit(&span.0);
            visit(&span.1);
        }
        Prim::Marker { x, y, .. } | Prim::Spot { x, y } | Prim::Text { x, y, .. } => {
            visit(if x_axis { x } else { y })
        }
        Prim::Line { points, .. } => points.iter().for_each(|p| visit(&pick(p))),
        Prim::Area { points, .. } => {
            for (a, top, bottom) in points {
                if x_axis {
                    visit(a);
                } else {
                    visit(top);
                    visit(bottom);
                }
            }
        }
        Prim::Segment { from, to, .. } => {
            visit(&pick(from));
            visit(&pick(to));
        }
    }
}

/// Draw one mark; false if it could not be placed.
fn draw_item(figure: &mut Figure, item: &Item, x: &Scale, y: &Scale) -> bool {
    let at = |px: &P, py: &P| -> Option<(f64, f64)> { Some((x.map(px)?, y.map(py)?)) };
    let region = |figure: &mut Figure, x0: f64, y0: f64, x1: f64, y1: f64| {
        if let Some((name, tip)) = &item.tip {
            figure.region(
                x0.min(x1),
                y0.min(y1),
                (x1 - x0).abs().max(1.0),
                (y1 - y0).abs().max(1.0),
                name,
                tip.clone(),
            );
        }
    };
    match &item.prim {
        Prim::Rect {
            x: xs,
            y: ys,
            fill,
            stroke,
        } => {
            let (Some(a), Some(b)) = (at(&xs.0, &ys.0), at(&xs.1, &ys.1)) else {
                return false;
            };
            let (left, top) = (a.0.min(b.0), a.1.min(b.1));
            let (w, h) = ((a.0 - b.0).abs(), (a.1 - b.1).abs());
            figure.rect(left, top, w, h, *fill, *stroke);
            region(figure, left, top, left + w, top + h);
        }
        Prim::Marker {
            x: px,
            y: py,
            shape,
            area,
            fill,
            stroke,
        } => {
            let Some((cx, cy)) = at(px, py) else {
                return false;
            };
            let stroke = stroke.unwrap_or(Stroke::solid(
                Color {
                    a: 0,
                    ..Color::WHITE
                },
                0.0,
            ));
            axes::marker_styled(figure, *shape, cx, cy, *area, *fill, stroke);
            let r = area.max(0.0).sqrt() / 2.0 + 1.5;
            region(figure, cx - r, cy - r, cx + r, cy + r);
        }
        Prim::Spot { x: px, y: py } => {
            let Some((cx, cy)) = at(px, py) else {
                return false;
            };
            region(figure, cx - 3.0, cy - 3.0, cx + 3.0, cy + 3.0);
        }
        Prim::Line {
            points,
            stroke,
            by_y,
        } => {
            let mut placed: Vec<(f64, f64)> = points.iter().filter_map(|(a, b)| at(a, b)).collect();
            if placed.len() < points.len() && placed.is_empty() {
                return false;
            }
            if *by_y {
                placed.sort_by(|a, b| a.1.total_cmp(&b.1));
            } else {
                placed.sort_by(|a, b| a.0.total_cmp(&b.0));
            }
            if placed.len() >= 2 {
                figure.line(&placed, *stroke);
            }
        }
        Prim::Area { points, fill } => {
            let mut placed: Vec<(f64, f64, f64)> = points
                .iter()
                .filter_map(|(a, top, bottom)| Some((x.map(a)?, y.map(top)?, y.map(bottom)?)))
                .collect();
            if placed.len() < 2 {
                return points.is_empty();
            }
            placed.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut ring: Vec<(f64, f64)> = placed.iter().map(|p| (p.0, p.1)).collect();
            ring.extend(placed.iter().rev().map(|p| (p.0, p.2)));
            figure.area(vec![ring], *fill);
        }
        Prim::Segment { from, to, stroke } => {
            let (Some(a), Some(b)) = (at(&from.0, &from.1), at(&to.0, &to.1)) else {
                return false;
            };
            figure.line(&[a, b], *stroke);
            if item.tip.is_some() {
                region(figure, a.0 - 2.0, a.1 - 2.0, b.0 + 2.0, b.1 + 2.0);
            }
        }
        Prim::Text {
            x: px,
            y: py,
            text: label,
            style,
            offset,
        } => {
            let Some((tx, ty)) = at(px, py) else {
                return false;
            };
            let (tx, ty) = (tx + offset.0, ty + offset.1);
            figure.text(tx, ty, label.clone(), *style);
            let w = text::width(label, style.size);
            let left = match style.h_align {
                HAlign::Left => tx,
                HAlign::Center => tx - w / 2.0,
                HAlign::Right => tx - w,
            };
            region(
                figure,
                left,
                ty - style.size / 2.0,
                left + w,
                ty + style.size / 2.0,
            );
        }
    }
    true
}

#[cfg(test)]
mod tests;
