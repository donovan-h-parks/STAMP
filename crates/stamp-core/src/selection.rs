//! Choosing the samples of an analysis by their metadata values: the sample
//! filter shared by the command line (`--where`, `--where-not`) and the
//! window's Sample filter.
//!
//! A selection is a list of conditions, all of which a sample must meet:
//!
//! * [`Condition::OneOf`]: the sample's value of a field is one of a list
//!   (repeating `--where FIELD=VALUE` for one field lists several values);
//! * [`Condition::NoneOf`]: its value is none of a list (`--where-not`);
//! * [`Condition::Compare`]: its value is a number that compares with a
//!   given number as asked (`--where "Age>=30"`); a sample without a
//!   number fails the condition.

use crate::error::{Result, StampError};
use crate::metadata::Metadata;
use crate::text;

/// A numeric comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl Comparison {
    pub fn symbol(self) -> &'static str {
        match self {
            Comparison::Less => "<",
            Comparison::LessOrEqual => "<=",
            Comparison::Greater => ">",
            Comparison::GreaterOrEqual => ">=",
        }
    }

    fn holds(self, a: f64, b: f64) -> bool {
        match self {
            Comparison::Less => a < b,
            Comparison::LessOrEqual => a <= b,
            Comparison::Greater => a > b,
            Comparison::GreaterOrEqual => a >= b,
        }
    }
}

/// One condition on a sample's metadata.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// The value of `field` is one of `values`.
    OneOf { field: String, values: Vec<String> },
    /// The value of `field` is none of `values`.
    NoneOf { field: String, values: Vec<String> },
    /// The value of `field` is a number standing in `comparison` to `value`.
    Compare {
        field: String,
        comparison: Comparison,
        value: f64,
    },
}

impl Condition {
    pub fn field(&self) -> &str {
        match self {
            Condition::OneOf { field, .. }
            | Condition::NoneOf { field, .. }
            | Condition::Compare { field, .. } => field,
        }
    }

    /// True if a sample with this value of the field meets the condition.
    pub fn accepts(&self, value: &str) -> bool {
        let value = value.trim();
        match self {
            Condition::OneOf { values, .. } => values.iter().any(|v| v.trim() == value),
            Condition::NoneOf { values, .. } => !values.iter().any(|v| v.trim() == value),
            Condition::Compare {
                comparison,
                value: limit,
                ..
            } => text::parse_float(value)
                .is_some_and(|number| number.is_finite() && comparison.holds(number, *limit)),
        }
    }

    /// The condition as a reader would write it.
    pub fn describe(&self) -> String {
        match self {
            Condition::OneOf { field, values } if values.len() == 1 => {
                format!("{field} = {}", values[0])
            }
            Condition::OneOf { field, values } => {
                format!("{field} is one of {}", values.join(", "))
            }
            Condition::NoneOf { field, values } if values.len() == 1 => {
                format!("{field} is not {}", values[0])
            }
            Condition::NoneOf { field, values } => {
                format!("{field} is none of {}", values.join(", "))
            }
            Condition::Compare {
                field,
                comparison,
                value,
            } => format!("{field} {} {value}", comparison.symbol()),
        }
    }
}

/// Conditions that every sample of an analysis must meet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SampleSelection {
    pub conditions: Vec<Condition>,
}

/// Split `FIELD<op>VALUE` at its first operator: `<=`, `>=`, `=`, `<` or
/// `>`. Returns the field, the operator and the value.
fn split_condition(text: &str) -> Option<(&str, &str, &str)> {
    let at = text.find(['=', '<', '>'])?;
    let rest = &text[at..];
    let op_len = if rest.starts_with("<=") || rest.starts_with(">=") {
        2
    } else {
        1
    };
    Some((text[..at].trim(), &rest[..op_len], rest[op_len..].trim()))
}

impl SampleSelection {
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    /// Build a selection from the text of the command line's options.
    ///
    /// Each of `include` is `FIELD=VALUE`, or a numeric comparison
    /// `FIELD<X`, `FIELD<=X`, `FIELD>X` or `FIELD>=X`. Values given for the
    /// same field with `=` are alternatives; everything else must hold
    /// together. Each of `exclude` is `FIELD=VALUE` and leaves out the
    /// samples with that value. The text is split at its first `=`, `<` or
    /// `>`, so a value may contain any character, including those; a field
    /// name may not.
    pub fn parse(include: &[String], exclude: &[String]) -> Result<SampleSelection> {
        let mut selection = SampleSelection::default();
        let malformed = |text: &str, option: &str| {
            StampError::new(format!(
                "Cannot read the condition '{text}' of {option}: write it as FIELD=VALUE{}.",
                if option == "--where" {
                    ", FIELD<NUMBER, FIELD<=NUMBER, FIELD>NUMBER or FIELD>=NUMBER"
                } else {
                    ""
                }
            ))
        };
        for text in include {
            let (field, op, value) =
                split_condition(text).ok_or_else(|| malformed(text, "--where"))?;
            if field.is_empty() {
                return Err(malformed(text, "--where"));
            }
            if op == "=" {
                selection.add_value(field, value, true);
                continue;
            }
            let comparison = match op {
                "<" => Comparison::Less,
                "<=" => Comparison::LessOrEqual,
                ">" => Comparison::Greater,
                _ => Comparison::GreaterOrEqual,
            };
            let number = text::parse_float(value)
                .filter(|n| n.is_finite())
                .ok_or_else(|| {
                    StampError::new(format!(
                        "In the condition '{text}', '{value}' is not a number; only numbers \
                         can be compared with {op}."
                    ))
                })?;
            selection.conditions.push(Condition::Compare {
                field: field.to_string(),
                comparison,
                value: number,
            });
        }
        for text in exclude {
            match split_condition(text) {
                Some((field, "=", value)) if !field.is_empty() => {
                    selection.add_value(field, value, false)
                }
                _ => return Err(malformed(text, "--where-not")),
            }
        }
        Ok(selection)
    }

    /// Add a value to the `OneOf` (or `NoneOf`) condition of a field,
    /// creating it if need be.
    fn add_value(&mut self, field: &str, value: &str, include: bool) {
        for condition in &mut self.conditions {
            match condition {
                Condition::OneOf { field: f, values } if include && f == field => {
                    values.push(value.to_string());
                    return;
                }
                Condition::NoneOf { field: f, values } if !include && f == field => {
                    values.push(value.to_string());
                    return;
                }
                _ => {}
            }
        }
        let field = field.to_string();
        let values = vec![value.to_string()];
        self.conditions.push(if include {
            Condition::OneOf { field, values }
        } else {
            Condition::NoneOf { field, values }
        });
    }

    /// Check that every field exists and every value named for a field is
    /// one that some sample has, which catches misspellings.
    pub fn check(&self, metadata: &Metadata) -> Result<()> {
        for condition in &self.conditions {
            let field = condition.field();
            if !metadata.fields().iter().any(|f| f == field) {
                return Err(StampError::new(format!(
                    "Unknown metadata field '{field}' in the sample filter. Fields in the \
                     metadata file: {}.",
                    metadata.fields().join(", ")
                )));
            }
            let known = metadata.unique_values(field);
            if let Condition::OneOf { values, .. } | Condition::NoneOf { values, .. } = condition {
                for value in values {
                    if !known.iter().any(|k| k.trim() == value.trim()) {
                        let shown = if known.len() > 30 {
                            format!("{} distinct values", known.len())
                        } else {
                            format!("values: {}", known.join(", "))
                        };
                        return Err(StampError::new(format!(
                            "No sample has the value '{value}' in the metadata field \
                             '{field}' ({shown})."
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// True if a sample meets every condition.
    pub fn accepts(&self, metadata: &Metadata, sample: &str) -> bool {
        self.conditions.iter().all(|condition| {
            metadata
                .value(sample, condition.field())
                .is_some_and(|value| condition.accepts(value))
        })
    }

    /// The conditions as a reader would write them.
    pub fn describe(&self) -> String {
        let parts: Vec<String> = self.conditions.iter().map(Condition::describe).collect();
        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProfileTree;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn metadata() -> Metadata {
        let tree = ProfileTree::read("F\tS1\tS2\tS3\tS4\tS5\nx\t1\t2\t3\t4\t5\n").unwrap();
        let text = "Id\tHost\tOrder\tAge\n\
            S1\tPrzewalski's / E. ferus\tPerissodactyla\t3\n\
            S2\tEquivita+Tapirivita\tPerissodactyla\t10\n\
            S3\tCattle (dairy)\tArtiodactyla\t7\n\
            S4\tSheep\tArtiodactyla\tNA\n\
            S5\tA=B\tCarnivora\t1.5\n";
        Metadata::read(text, &tree).unwrap().0
    }

    fn kept(selection: &SampleSelection) -> Vec<String> {
        let meta = metadata();
        meta.samples()
            .iter()
            .filter(|s| selection.accepts(&meta, s))
            .cloned()
            .collect()
    }

    #[test]
    fn values_of_one_field_are_alternatives_and_fields_must_all_match() {
        let selection = SampleSelection::parse(
            &strings(&["Order=Perissodactyla", "Order = Artiodactyla", "Age>=5"]),
            &[],
        )
        .unwrap();
        assert_eq!(selection.conditions.len(), 2);
        assert_eq!(kept(&selection), ["S2", "S3"]);
        assert_eq!(
            selection.describe(),
            "Order is one of Perissodactyla, Artiodactyla; Age >= 5"
        );
    }

    #[test]
    fn values_may_hold_any_character() {
        let one = |text: &str| SampleSelection::parse(&strings(&[text]), &[]).unwrap();
        assert_eq!(kept(&one("Host=Przewalski's / E. ferus")), ["S1"]);
        assert_eq!(kept(&one("Host=Equivita+Tapirivita")), ["S2"]);
        assert_eq!(kept(&one("Host=Cattle (dairy)")), ["S3"]);
        // Split at the first operator, so the value may contain '='.
        assert_eq!(kept(&one("Host=A=B")), ["S5"]);
    }

    #[test]
    fn exclusions_and_comparisons() {
        let selection =
            SampleSelection::parse(&strings(&["Age<10"]), &strings(&["Order=Carnivora"])).unwrap();
        // S4 has no number, S2 is too old, S5 is excluded.
        assert_eq!(kept(&selection), ["S1", "S3"]);
        let all = SampleSelection::default();
        assert!(all.is_empty());
        assert_eq!(kept(&all).len(), 5);
        let gt = SampleSelection::parse(&strings(&["Age>3"]), &[]).unwrap();
        assert_eq!(kept(&gt), ["S2", "S3"]);
        let le = SampleSelection::parse(&strings(&["Age<=3"]), &[]).unwrap();
        assert_eq!(kept(&le), ["S1", "S5"]);
    }

    #[test]
    fn problems_are_reported() {
        let meta = metadata();
        let parse = |include: &[&str], exclude: &[&str]| {
            SampleSelection::parse(&strings(include), &strings(exclude))
        };
        assert!(parse(&["Order"], &[])
            .unwrap_err()
            .0
            .contains("FIELD=VALUE"));
        assert!(parse(&["=x"], &[]).is_err());
        assert!(parse(&[], &["Age<3"])
            .unwrap_err()
            .0
            .contains("--where-not"));
        assert!(parse(&["Age<old"], &[])
            .unwrap_err()
            .0
            .contains("'old' is not a number"));
        let err = parse(&["Ordr=Carnivora"], &[])
            .unwrap()
            .check(&meta)
            .unwrap_err();
        assert!(err.0.contains("Unknown metadata field 'Ordr'"), "{}", err.0);
        let err = parse(&["Order=Perisodactyla"], &[])
            .unwrap()
            .check(&meta)
            .unwrap_err();
        assert_eq!(
            err.0,
            "No sample has the value 'Perisodactyla' in the metadata field 'Order' \
             (values: Artiodactyla, Carnivora, Perissodactyla)."
        );
        assert!(parse(&["Order=Carnivora", "Age>1"], &["Host=Sheep"])
            .unwrap()
            .check(&meta)
            .is_ok());
    }
}
