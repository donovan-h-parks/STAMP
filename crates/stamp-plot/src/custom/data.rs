//! The data of custom plots: tables of values and Vega-Lite's transforms
//! of them (`filter`, `calculate`, `aggregate`, `joinaggregate`, `window`,
//! `fold`, `lookup`, `bin`).

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value};

use super::expr;

/// One value of a table.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Null,
    Bool(bool),
    Num(f64),
    Str(Arc<str>),
}

impl Cell {
    pub fn text(text: &str) -> Cell {
        Cell::Str(Arc::from(text))
    }

    /// From a JSON value; objects and arrays become their JSON text.
    pub fn from_json(value: &Value) -> Cell {
        match value {
            Value::Null => Cell::Null,
            Value::Bool(b) => Cell::Bool(*b),
            Value::Number(n) => Cell::Num(n.as_f64().unwrap_or(f64::NAN)),
            Value::String(s) => Cell::text(s),
            other => Cell::text(&other.to_string()),
        }
    }

    /// The value as a number, if it is one (or text that reads as one).
    pub fn number(&self) -> Option<f64> {
        match self {
            Cell::Num(n) => Some(*n),
            Cell::Bool(b) => Some(f64::from(u8::from(*b))),
            Cell::Str(s) => s.trim().parse().ok(),
            Cell::Null => None,
        }
    }

    /// A number that is not missing.
    pub fn finite(&self) -> Option<f64> {
        self.number().filter(|n| n.is_finite())
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Cell::Null => false,
            Cell::Num(n) => !n.is_nan(),
            _ => true,
        }
    }

    /// The value as text: numbers as short as they can be written.
    pub fn to_text(&self) -> String {
        match self {
            Cell::Null => "null".to_string(),
            Cell::Bool(b) => b.to_string(),
            Cell::Num(n) => format_number(*n),
            Cell::Str(s) => s.to_string(),
        }
    }
}

/// A number for a tooltip or label: integers whole, others to four
/// significant digits, very large or small ones in scientific notation.
pub fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 { "∞" } else { "−∞" }.to_string();
    }
    if n == n.round() && n.abs() < 1e15 {
        return format!("{n:.0}");
    }
    let magnitude = n.abs().log10().floor();
    if !(-4.0..6.0).contains(&magnitude) {
        return format!("{n:.3e}");
    }
    let decimals = (3.0 - magnitude).max(0.0) as usize;
    let text = format!("{n:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

/// A table: named columns and rows of values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

impl Table {
    pub fn new(columns: Vec<String>) -> Table {
        Table {
            columns,
            rows: Vec::new(),
        }
    }

    /// From Vega-Lite's inline `values`: an array of objects.
    pub fn from_json(values: &[Value]) -> Result<Table, String> {
        let mut table = Table::default();
        for value in values {
            let Value::Object(object) = value else {
                return Err("every item of data.values must be an object of fields".to_string());
            };
            for key in object.keys() {
                if table.column(key).is_none() {
                    table.columns.push(key.clone());
                    for row in &mut table.rows {
                        row.push(Cell::Null);
                    }
                }
            }
            let row = table
                .columns
                .iter()
                .map(|c| object.get(c).map_or(Cell::Null, Cell::from_json))
                .collect();
            table.rows.push(row);
        }
        Ok(table)
    }

    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// The column of a field that must exist.
    pub fn field(&self, name: &str) -> Result<usize, String> {
        self.column(name).ok_or_else(|| {
            let mut known = self.columns.clone();
            known.truncate(40);
            format!(
                "there is no field '{name}' (fields: {})",
                if known.is_empty() {
                    "none".to_string()
                } else {
                    known.join(", ")
                }
            )
        })
    }

    /// The column called `name`, added (empty) if there is none.
    pub fn column_or_add(&mut self, name: &str) -> usize {
        match self.column(name) {
            Some(index) => index,
            None => {
                self.columns.push(name.to_string());
                for row in &mut self.rows {
                    row.push(Cell::Null);
                }
                self.columns.len() - 1
            }
        }
    }

    /// The value of `field` in row `row`, null if there is no such field.
    pub fn get(&self, row: usize, field: &str) -> Cell {
        self.column(field)
            .map_or(Cell::Null, |c| self.rows[row][c].clone())
    }
}

/// Rows grouped by the values of some columns, in order of first
/// appearance.
pub fn group_rows(table: &Table, columns: &[usize]) -> Vec<Vec<usize>> {
    let mut index: HashMap<Vec<String>, usize> = HashMap::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (r, row) in table.rows.iter().enumerate() {
        let key: Vec<String> = columns.iter().map(|&c| row[c].to_text()).collect();
        let at = *index.entry(key).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[at].push(r);
    }
    groups
}

/// The quantile `p` of sorted values (R's type 7, as d3 computes it).
pub fn quantile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let h = (sorted.len() - 1) as f64 * p;
    let low = h.floor() as usize;
    let high = (low + 1).min(sorted.len() - 1);
    sorted[low] + (h - low as f64) * (sorted[high] - sorted[low])
}

/// The aggregate operations.
pub const AGGREGATES: [&str; 17] = [
    "count",
    "valid",
    "missing",
    "distinct",
    "sum",
    "mean",
    "average",
    "median",
    "min",
    "max",
    "stdev",
    "stdevp",
    "variance",
    "variancep",
    "stderr",
    "q1",
    "q3",
];

/// Apply an aggregate operation to the values of a group.
pub fn aggregate(op: &str, values: &[Cell]) -> Result<Cell, String> {
    let mut numbers: Vec<f64> = values.iter().filter_map(Cell::finite).collect();
    let n = numbers.len() as f64;
    let mean = numbers.iter().sum::<f64>() / n;
    let squares = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>();
    let value = match op {
        "count" => values.len() as f64,
        "valid" => values.iter().filter(|v| v.is_valid()).count() as f64,
        "missing" => values.iter().filter(|v| !v.is_valid()).count() as f64,
        "distinct" => {
            let mut seen: Vec<String> = values.iter().map(Cell::to_text).collect();
            seen.sort();
            seen.dedup();
            seen.len() as f64
        }
        "sum" => numbers.iter().sum(),
        "mean" | "average" => mean,
        "min" => numbers.iter().copied().fold(f64::NAN, f64::min),
        "max" => numbers.iter().copied().fold(f64::NAN, f64::max),
        "variance" => squares / (n - 1.0),
        "variancep" => squares / n,
        "stdev" => (squares / (n - 1.0)).sqrt(),
        "stdevp" => (squares / n).sqrt(),
        "stderr" => (squares / (n - 1.0)).sqrt() / n.sqrt(),
        "median" | "q1" | "q3" => {
            numbers.sort_by(f64::total_cmp);
            let p = match op {
                "q1" => 0.25,
                "q3" => 0.75,
                _ => 0.5,
            };
            quantile(&numbers, p)
        }
        other => {
            return Err(format!(
                "unknown aggregate '{other}'; available: {}",
                AGGREGATES.join(", ")
            ))
        }
    };
    if numbers.is_empty() && !matches!(op, "count" | "valid" | "missing" | "distinct" | "sum") {
        return Ok(Cell::Null);
    }
    Ok(Cell::Num(value))
}

/// A description of an aggregate for titles: "Mean of count".
pub fn aggregate_title(op: &str, field: Option<&str>) -> String {
    let name = match op {
        "count" => return "Count of Records".to_string(),
        "mean" | "average" => "Mean",
        "median" => "Median",
        "sum" => "Sum",
        "min" => "Min",
        "max" => "Max",
        "stdev" | "stdevp" => "Standard deviation",
        "variance" | "variancep" => "Variance",
        "stderr" => "Standard error",
        "q1" => "Q1",
        "q3" => "Q3",
        "distinct" => "Distinct",
        "valid" => "Valid",
        "missing" => "Missing",
        other => other,
    };
    format!("{name} of {}", field.unwrap_or("records"))
}

/// A row predicate of a `filter` transform.
pub enum Predicate {
    Expr(expr::Expr),
    Field { field: String, test: FieldTest },
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
}

pub enum FieldTest {
    Equal(Cell),
    OneOf(Vec<Cell>),
    Lt(f64),
    Lte(f64),
    Gt(f64),
    Gte(f64),
    Range(Option<f64>, Option<f64>),
    Valid(bool),
}

impl Predicate {
    pub fn parse(value: &Value) -> Result<Predicate, String> {
        match value {
            Value::String(text) => Ok(Predicate::Expr(
                expr::parse(text).map_err(|e| format!("in '{text}': {e}"))?,
            )),
            Value::Object(object) => {
                if let Some(Value::Array(all)) = object.get("and") {
                    return Ok(Predicate::And(
                        all.iter().map(Predicate::parse).collect::<Result<_, _>>()?,
                    ));
                }
                if let Some(Value::Array(any)) = object.get("or") {
                    return Ok(Predicate::Or(
                        any.iter().map(Predicate::parse).collect::<Result<_, _>>()?,
                    ));
                }
                if let Some(inner) = object.get("not") {
                    return Ok(Predicate::Not(Box::new(Predicate::parse(inner)?)));
                }
                let field = object
                    .get("field")
                    .and_then(Value::as_str)
                    .ok_or("a filter needs an expression, a 'field' predicate, or and/or/not")?
                    .to_string();
                let number = |key: &str| -> Result<f64, String> {
                    object[key]
                        .as_f64()
                        .ok_or_else(|| format!("'{key}' must be a number"))
                };
                let test = if let Some(v) = object.get("equal") {
                    FieldTest::Equal(Cell::from_json(v))
                } else if let Some(v) = object.get("oneOf").or_else(|| object.get("in")) {
                    FieldTest::OneOf(
                        v.as_array()
                            .ok_or("'oneOf' must be an array")?
                            .iter()
                            .map(Cell::from_json)
                            .collect(),
                    )
                } else if object.contains_key("lt") {
                    FieldTest::Lt(number("lt")?)
                } else if object.contains_key("lte") {
                    FieldTest::Lte(number("lte")?)
                } else if object.contains_key("gt") {
                    FieldTest::Gt(number("gt")?)
                } else if object.contains_key("gte") {
                    FieldTest::Gte(number("gte")?)
                } else if let Some(v) = object.get("range") {
                    let ends = v
                        .as_array()
                        .filter(|a| a.len() == 2)
                        .ok_or("'range' must be [low, high] (either may be null)")?;
                    FieldTest::Range(ends[0].as_f64(), ends[1].as_f64())
                } else if let Some(v) = object.get("valid") {
                    FieldTest::Valid(v.as_bool().unwrap_or(true))
                } else {
                    return Err(format!(
                        "the filter on '{field}' needs equal, oneOf, lt, lte, gt, gte, range or valid"
                    ));
                };
                Ok(Predicate::Field { field, test })
            }
            _ => Err("a filter must be an expression string or an object".to_string()),
        }
    }

    pub fn test(&self, table: &Table, row: usize) -> bool {
        match self {
            Predicate::Expr(e) => expr::truthy(&expr::eval(e, &|f| table.get(row, f))),
            Predicate::And(all) => all.iter().all(|p| p.test(table, row)),
            Predicate::Or(any) => any.iter().any(|p| p.test(table, row)),
            Predicate::Not(inner) => !inner.test(table, row),
            Predicate::Field { field, test } => {
                let value = table.get(row, field);
                let n = value.finite();
                match test {
                    FieldTest::Equal(v) => expr::truthy(&expr::eval(
                        &expr::Expr::Binary(
                            "==",
                            Box::new(expr::Expr::Value(value.clone())),
                            Box::new(expr::Expr::Value(v.clone())),
                        ),
                        &|_| Cell::Null,
                    )),
                    FieldTest::OneOf(values) => {
                        let text = value.to_text();
                        values.iter().any(|v| v.to_text() == text)
                    }
                    FieldTest::Lt(x) => n.is_some_and(|n| n < *x),
                    FieldTest::Lte(x) => n.is_some_and(|n| n <= *x),
                    FieldTest::Gt(x) => n.is_some_and(|n| n > *x),
                    FieldTest::Gte(x) => n.is_some_and(|n| n >= *x),
                    FieldTest::Range(low, high) => {
                        n.is_some_and(|n| low.is_none_or(|l| n >= l) && high.is_none_or(|h| n <= h))
                    }
                    FieldTest::Valid(want) => value.is_valid() == *want,
                }
            }
        }
    }
}

fn strings(value: Option<&Value>, what: &str) -> Result<Vec<String>, String> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(s)) => Ok(vec![s.clone()]),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("'{what}' must list field names"))
            })
            .collect(),
        Some(_) => Err(format!("'{what}' must list field names")),
    }
}

fn columns_of(table: &Table, fields: &[String]) -> Result<Vec<usize>, String> {
    fields.iter().map(|f| table.field(f)).collect()
}

/// One aggregate of an `aggregate`, `joinaggregate` or `window` transform.
struct Op {
    op: String,
    field: Option<String>,
    name: String,
}

fn ops(value: Option<&Value>, key: &str) -> Result<Vec<Op>, String> {
    let items = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("'{key}' must be an array of {{op, field, as}}"))?;
    items
        .iter()
        .map(|item| {
            let op = item
                .get("op")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("each item of '{key}' needs an 'op'"))?
                .to_string();
            let field = item
                .get("field")
                .and_then(Value::as_str)
                .map(str::to_string);
            let name = item
                .get("as")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| match &field {
                    Some(f) => format!("{op}_{f}"),
                    None => op.clone(),
                });
            Ok(Op { op, field, name })
        })
        .collect()
}

fn op_values(table: &Table, rows: &[usize], op: &Op) -> Result<Vec<Cell>, String> {
    match &op.field {
        Some(field) => {
            let c = table.field(field)?;
            Ok(rows.iter().map(|&r| table.rows[r][c].clone()).collect())
        }
        None if op.op == "count" => Ok(vec![Cell::Null; rows.len()]),
        None => Err(format!("the aggregate '{}' needs a 'field'", op.op)),
    }
}

/// Apply one transform. `lookup_table` finds the named data that `lookup`
/// joins.
pub fn transform(
    table: Table,
    step: &Map<String, Value>,
    lookup_table: &dyn Fn(&Value) -> Result<Arc<Table>, String>,
) -> Result<Table, String> {
    if let Some(filter) = step.get("filter") {
        let predicate = Predicate::parse(filter)?;
        let keep: Vec<bool> = (0..table.rows.len())
            .map(|r| predicate.test(&table, r))
            .collect();
        let mut table = table;
        let mut flags = keep.into_iter();
        table.rows.retain(|_| flags.next().unwrap_or(false));
        return Ok(table);
    }
    if let Some(calculate) = step.get("calculate") {
        let text = calculate
            .as_str()
            .ok_or("'calculate' must be an expression")?;
        let expr = expr::parse(text).map_err(|e| format!("in '{text}': {e}"))?;
        let name = step
            .get("as")
            .and_then(Value::as_str)
            .ok_or("'calculate' needs 'as', the name of the new field")?;
        let mut table = table;
        let values: Vec<Cell> = (0..table.rows.len())
            .map(|r| expr::eval(&expr, &|f| table.get(r, f)))
            .collect();
        let c = table.column_or_add(name);
        for (row, value) in table.rows.iter_mut().zip(values) {
            row[c] = value;
        }
        return Ok(table);
    }
    if step.contains_key("aggregate") {
        let ops = ops(step.get("aggregate"), "aggregate")?;
        let groupby = strings(step.get("groupby"), "groupby")?;
        let keys = columns_of(&table, &groupby)?;
        let mut columns = groupby.clone();
        columns.extend(ops.iter().map(|o| o.name.clone()));
        let mut out = Table::new(columns);
        for rows in group_rows(&table, &keys) {
            let mut row: Vec<Cell> = keys
                .iter()
                .map(|&k| table.rows[rows[0]][k].clone())
                .collect();
            for op in &ops {
                row.push(aggregate(&op.op, &op_values(&table, &rows, op)?)?);
            }
            out.rows.push(row);
        }
        // An aggregate of no rows at all is one row, as in Vega-Lite.
        if table.rows.is_empty() && keys.is_empty() {
            let row = ops
                .iter()
                .map(|op| aggregate(&op.op, &[]))
                .collect::<Result<_, _>>()?;
            out.rows.push(row);
        }
        return Ok(out);
    }
    if step.contains_key("joinaggregate") {
        let ops = ops(step.get("joinaggregate"), "joinaggregate")?;
        let keys = columns_of(&table, &strings(step.get("groupby"), "groupby")?)?;
        let mut table = table;
        for op in &ops {
            let groups = group_rows(&table, &keys);
            let mut values = vec![Cell::Null; table.rows.len()];
            for rows in groups {
                let value = aggregate(&op.op, &op_values(&table, &rows, op)?)?;
                for r in rows {
                    values[r] = value.clone();
                }
            }
            let c = table.column_or_add(&op.name);
            for (row, value) in table.rows.iter_mut().zip(values) {
                row[c] = value;
            }
        }
        return Ok(table);
    }
    if step.contains_key("window") {
        return window(table, step);
    }
    if let Some(fold) = step.get("fold") {
        let fields = strings(Some(fold), "fold")?;
        let names = strings(step.get("as"), "as")?;
        let (key, value) = match names.as_slice() {
            [] => ("key".to_string(), "value".to_string()),
            [k, v] => (k.clone(), v.clone()),
            _ => return Err("'as' of a fold must name two fields: [key, value]".to_string()),
        };
        let folded = columns_of(&table, &fields)?;
        let kept: Vec<usize> = (0..table.columns.len())
            .filter(|c| !folded.contains(c))
            .collect();
        let mut columns: Vec<String> = kept.iter().map(|&c| table.columns[c].clone()).collect();
        columns.push(key);
        columns.push(value);
        let mut out = Table::new(columns);
        for row in &table.rows {
            for (&c, name) in folded.iter().zip(&fields) {
                let mut new: Vec<Cell> = kept.iter().map(|&k| row[k].clone()).collect();
                new.push(Cell::text(name));
                new.push(row[c].clone());
                out.rows.push(new);
            }
        }
        return Ok(out);
    }
    if let Some(key) = step.get("lookup") {
        let key = key.as_str().ok_or("'lookup' must name a field")?;
        let from = step
            .get("from")
            .and_then(Value::as_object)
            .ok_or("'lookup' needs 'from': {data, key, fields}")?;
        let other = lookup_table(from.get("data").unwrap_or(&Value::Null))?;
        let other_key = from
            .get("key")
            .and_then(Value::as_str)
            .ok_or("'from' needs 'key', the field to match")?;
        let other_key = other.field(other_key)?;
        let fields = match from.get("fields") {
            Some(v) => strings(Some(v), "fields")?,
            None => other
                .columns
                .iter()
                .enumerate()
                .filter(|(c, _)| *c != other_key)
                .map(|(_, name)| name.clone())
                .collect(),
        };
        let names = match step.get("as") {
            Some(v) => strings(Some(v), "as")?,
            None => fields.clone(),
        };
        if names.len() != fields.len() {
            return Err("'as' of a lookup must name as many fields as 'fields'".to_string());
        }
        let source = columns_of(&other, &fields)?;
        let index: HashMap<String, usize> = other
            .rows
            .iter()
            .enumerate()
            .map(|(r, row)| (row[other_key].to_text(), r))
            .rev()
            .collect();
        let mut table = table;
        let k = table.field(key)?;
        let targets: Vec<usize> = names.iter().map(|n| table.column_or_add(n)).collect();
        for row in &mut table.rows {
            let found = index.get(&row[k].to_text()).copied();
            for (&t, &s) in targets.iter().zip(&source) {
                row[t] = found.map_or(Cell::Null, |r| other.rows[r][s].clone());
            }
        }
        return Ok(table);
    }
    if let Some(bin) = step.get("bin") {
        let field = step
            .get("field")
            .and_then(Value::as_str)
            .ok_or("a 'bin' transform needs 'field'")?;
        let names = strings(step.get("as"), "as")?;
        let (start, end) = match names.as_slice() {
            [] => (format!("bin_{field}"), format!("bin_{field}_end")),
            [s] => (s.clone(), format!("{s}_end")),
            [s, e, ..] => (s.clone(), e.clone()),
        };
        let mut table = table;
        bin_column(&mut table, field, bin, &start, &end)?;
        return Ok(table);
    }
    let names: Vec<&String> = step.keys().collect();
    Err(format!(
        "unknown transform {names:?}; available: filter, calculate, aggregate, joinaggregate, \
         window, fold, lookup, bin"
    ))
}

/// The bins of `bin` (true, or {maxbins, step, extent}) over values from
/// `min` to `max`: (first start, step).
pub fn bins(bin: &Value, min: f64, max: f64) -> Result<(f64, f64), String> {
    let options = bin.as_object();
    let get = |key: &str| options.and_then(|o| o.get(key)).and_then(Value::as_f64);
    let (min, max) = match options
        .and_then(|o| o.get("extent"))
        .and_then(Value::as_array)
    {
        Some(e) if e.len() == 2 => (e[0].as_f64().unwrap_or(min), e[1].as_f64().unwrap_or(max)),
        _ => (min, max),
    };
    let step = match get("step") {
        Some(step) if step > 0.0 => step,
        Some(_) => return Err("the 'step' of a bin must be positive".to_string()),
        None => {
            let bins = get("maxbins").unwrap_or(10.0).max(1.0);
            let raw = ((max - min) / bins).max(f64::MIN_POSITIVE);
            let magnitude = 10f64.powf(raw.log10().floor());
            [1.0, 2.0, 5.0, 10.0]
                .into_iter()
                .map(|m| m * magnitude)
                .find(|s| *s >= raw * (1.0 - 1e-9))
                .unwrap_or(10.0 * magnitude)
        }
    };
    Ok(((min / step + 1e-9).floor() * step, step))
}

/// Put the start and end of the bin of each value of `field` into columns
/// `start` and `end`.
pub fn bin_column(
    table: &mut Table,
    field: &str,
    bin: &Value,
    start: &str,
    end: &str,
) -> Result<(), String> {
    let c = table.field(field)?;
    let values: Vec<Option<f64>> = table.rows.iter().map(|r| r[c].finite()).collect();
    let (min, max) = values
        .iter()
        .flatten()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    if !min.is_finite() {
        return Err(format!("'{field}' has no numbers to put in bins"));
    }
    let (first, step) = bins(bin, min, if max > min { max } else { min + 1.0 })?;
    let s = table.column_or_add(start);
    let e = table.column_or_add(end);
    for (row, value) in table.rows.iter_mut().zip(values) {
        let Some(v) = value else {
            row[s] = Cell::Null;
            row[e] = Cell::Null;
            continue;
        };
        let mut index = ((v - first) / step + 1e-9).floor();
        // The largest value belongs to the last bin, not one of its own.
        if v == max && index > 0.0 && (first + index * step - v).abs() < step * 1e-9 {
            index -= 1.0;
        }
        let low = first + index * step;
        row[s] = Cell::Num(low);
        row[e] = Cell::Num(low + step);
    }
    Ok(())
}

/// A sort of rows: fields, each ascending or descending.
pub fn sort_rows(table: &Table, rows: &mut [usize], sort: &Value) -> Result<(), String> {
    let items: Vec<&Value> = match sort {
        Value::Array(items) => items.iter().collect(),
        Value::Null => return Ok(()),
        other => vec![other],
    };
    let mut keys = Vec::new();
    for item in items {
        let (field, descending) = match item {
            Value::String(f) => (f.as_str(), false),
            Value::Object(o) => (
                o.get("field")
                    .and_then(Value::as_str)
                    .ok_or("each sort needs a 'field'")?,
                o.get("order").and_then(Value::as_str) == Some("descending"),
            ),
            _ => return Err("a sort is {field, order}".to_string()),
        };
        keys.push((table.field(field)?, descending));
    }
    rows.sort_by(|&a, &b| {
        for &(c, descending) in &keys {
            let (x, y) = (&table.rows[a][c], &table.rows[b][c]);
            let ordering = compare_cells(x, y);
            // Missing values stay last either way.
            let ordering = if descending && x.is_valid() && y.is_valid() {
                ordering.reverse()
            } else {
                ordering
            };
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
        std::cmp::Ordering::Equal
    });
    Ok(())
}

/// Numbers before text, numbers by value, text alphabetically; missing
/// values last.
pub fn compare_cells(a: &Cell, b: &Cell) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a.is_valid(), b.is_valid()) {
        (false, false) => return Ordering::Equal,
        (false, true) => return Ordering::Greater,
        (true, false) => return Ordering::Less,
        _ => {}
    }
    match (a, b) {
        (Cell::Num(x), Cell::Num(y)) => x.total_cmp(y),
        (Cell::Num(_), _) => Ordering::Less,
        (_, Cell::Num(_)) => Ordering::Greater,
        _ => a.to_text().cmp(&b.to_text()),
    }
}

fn window(table: Table, step: &Map<String, Value>) -> Result<Table, String> {
    let ops = ops(step.get("window"), "window")?;
    let keys = columns_of(&table, &strings(step.get("groupby"), "groupby")?)?;
    let whole = match step.get("frame") {
        None => false,
        Some(Value::Array(f)) if f.len() == 2 && f[0].is_null() && f[1].is_null() => true,
        Some(Value::Array(f)) if f.len() == 2 && f[0].is_null() && f[1].as_f64() == Some(0.0) => {
            false
        }
        Some(_) => {
            return Err(
                "a window's 'frame' may only be [null, 0] (running) or [null, null] \
                        (the whole group)"
                    .to_string(),
            )
        }
    };
    let mut table = table;
    let groups = group_rows(&table, &keys);
    let sort = step.get("sort").cloned().unwrap_or(Value::Null);
    let sort_columns: Vec<usize> = match &sort {
        Value::Array(items) => items
            .iter()
            .filter_map(|i| i.get("field").and_then(Value::as_str).or(i.as_str()))
            .filter_map(|f| table.column(f))
            .collect(),
        _ => Vec::new(),
    };
    let mut results: Vec<Vec<Cell>> = vec![vec![Cell::Null; table.rows.len()]; ops.len()];
    for mut rows in groups {
        sort_rows(&table, &mut rows, &sort)?;
        let tie_key = |r: usize| -> Vec<String> {
            sort_columns
                .iter()
                .map(|&c| table.rows[r][c].to_text())
                .collect()
        };
        for (o, op) in ops.iter().enumerate() {
            let (mut rank, mut dense) = (0usize, 0usize);
            let mut previous: Option<Vec<String>> = None;
            for (i, &r) in rows.iter().enumerate() {
                let value = match op.op.as_str() {
                    "row_number" => Cell::Num((i + 1) as f64),
                    "rank" | "dense_rank" => {
                        let key = tie_key(r);
                        if previous.as_ref() != Some(&key) {
                            rank = i + 1;
                            dense += 1;
                            previous = Some(key);
                        }
                        Cell::Num(if op.op == "rank" { rank } else { dense } as f64)
                    }
                    _ => {
                        let upto = if whole { rows.len() } else { i + 1 };
                        aggregate(&op.op, &op_values(&table, &rows[..upto], op)?).map_err(|e| {
                            format!("{e}; a window also offers row_number, rank, dense_rank")
                        })?
                    }
                };
                results[o][r] = value;
            }
        }
    }
    for (op, values) in ops.iter().zip(results) {
        let c = table.column_or_add(&op.name);
        for (row, value) in table.rows.iter_mut().zip(values) {
            row[c] = value;
        }
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Table {
        Table::from_json(
            json!([
                {"site": "A", "taxon": "x", "count": 4},
                {"site": "A", "taxon": "y", "count": 6},
                {"site": "B", "taxon": "x", "count": 1},
                {"site": "B", "taxon": "y", "count": 9},
                {"site": "B", "taxon": "z", "count": null}
            ])
            .as_array()
            .unwrap(),
        )
        .unwrap()
    }

    fn apply(table: Table, step: Value) -> Result<Table, String> {
        transform(table, step.as_object().unwrap(), &|_| {
            Ok(Arc::new(
                Table::from_json(
                    json!([{"site": "A", "soil": "clay"}, {"site": "B", "soil": "sand"}])
                        .as_array()
                        .unwrap(),
                )
                .unwrap(),
            ))
        })
    }

    #[test]
    fn filters() {
        let t = apply(sample(), json!({"filter": "datum.count > 3"})).unwrap();
        assert_eq!(t.rows.len(), 3);
        let t = apply(
            sample(),
            json!({"filter": {"field": "taxon", "oneOf": ["x", "z"]}}),
        )
        .unwrap();
        assert_eq!(t.rows.len(), 3);
        let t = apply(
            sample(),
            json!({"filter": {"and": [{"field": "site", "equal": "B"}, {"field": "count", "valid": true}]}}),
        )
        .unwrap();
        assert_eq!(t.rows.len(), 2);
        let t = apply(
            sample(),
            json!({"filter": {"field": "count", "range": [2, 6]}}),
        )
        .unwrap();
        assert_eq!(t.rows.len(), 2);
    }

    #[test]
    fn aggregates_and_joins() {
        let t = apply(
            sample(),
            json!({"aggregate": [{"op": "sum", "field": "count", "as": "total"}, {"op": "count", "as": "n"}],
                   "groupby": ["site"]}),
        )
        .unwrap();
        assert_eq!(t.columns, ["site", "total", "n"]);
        assert_eq!(
            t.rows[1],
            vec![Cell::text("B"), Cell::Num(10.0), Cell::Num(3.0)]
        );

        let t = apply(
            sample(),
            json!({"joinaggregate": [{"op": "sum", "field": "count", "as": "total"}], "groupby": ["site"]}),
        )
        .unwrap();
        assert_eq!(t.get(0, "total"), Cell::Num(10.0));
        let t = apply(
            t,
            json!({"calculate": "datum.count / datum.total", "as": "share"}),
        )
        .unwrap();
        assert_eq!(t.get(1, "share"), Cell::Num(0.6));

        let t = apply(sample(), json!({"lookup": "site", "from": {"data": {"name": "samples"}, "key": "site", "fields": ["soil"]}})).unwrap();
        assert_eq!(t.get(2, "soil"), Cell::text("sand"));
    }

    #[test]
    fn windows_rank_within_groups() {
        let t = apply(
            sample(),
            json!({"window": [{"op": "rank", "as": "rank"}, {"op": "sum", "field": "count", "as": "running"}],
                   "sort": [{"field": "count", "order": "descending"}], "groupby": ["site"]}),
        )
        .unwrap();
        assert_eq!(t.get(1, "rank"), Cell::Num(1.0));
        assert_eq!(t.get(0, "rank"), Cell::Num(2.0));
        assert_eq!(t.get(0, "running"), Cell::Num(10.0));
        assert_eq!(t.get(3, "rank"), Cell::Num(1.0));
    }

    #[test]
    fn folds_and_bins() {
        let wide =
            Table::from_json(json!([{"s": "a", "x": 1, "y": 2}]).as_array().unwrap()).unwrap();
        let t = apply(wide, json!({"fold": ["x", "y"], "as": ["var", "val"]})).unwrap();
        assert_eq!(t.columns, ["s", "var", "val"]);
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.get(1, "val"), Cell::Num(2.0));

        let t = apply(
            sample(),
            json!({"bin": {"step": 5}, "field": "count", "as": "b"}),
        )
        .unwrap();
        assert_eq!(t.get(0, "b"), Cell::Num(0.0));
        assert_eq!(t.get(1, "b"), Cell::Num(5.0));
        assert_eq!(t.get(1, "b_end"), Cell::Num(10.0));
        // The largest value falls in the last bin.
        assert_eq!(t.get(3, "b"), Cell::Num(5.0));
        assert_eq!(bins(&json!(true), 0.0, 97.0).unwrap(), (0.0, 10.0));
    }

    #[test]
    fn statistics() {
        let v: Vec<Cell> = [1.0, 2.0, 3.0, 4.0].map(Cell::Num).to_vec();
        assert_eq!(aggregate("median", &v).unwrap(), Cell::Num(2.5));
        assert_eq!(aggregate("q1", &v).unwrap(), Cell::Num(1.75));
        let sd = aggregate("stdev", &v).unwrap().number().unwrap();
        assert!((sd - 1.2909944487358056).abs() < 1e-12);
        assert!(aggregate("nope", &v).is_err());
        assert_eq!(format_number(0.000012345), "1.234e-5");
        assert_eq!(format_number(12.34567), "12.35");
        assert_eq!(format_number(42.0), "42");
    }

    #[test]
    fn mistakes_are_explained() {
        assert!(apply(
            sample(),
            json!({"aggregate": [{"op": "sum", "field": "nope"}]})
        )
        .unwrap_err()
        .contains("no field 'nope'"));
        assert!(apply(sample(), json!({"pivot": "x"}))
            .unwrap_err()
            .contains("unknown transform"));
    }
}
