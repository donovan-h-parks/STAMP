//! Adding COG functional categories to an IMG/M COG profile.

use std::collections::HashMap;

use crate::error::{Result, StampError};
use crate::text;

/// What to do with a COG that belongs to several functional categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiCodeTreatment {
    /// Keep the combined code (for example `KT`) as a category of its own.
    AsFeature,
    /// Add the COG's counts to each of its categories.
    AssignToEachCode,
}

const COG_CODES: &str = include_str!("../../data/cog_codes.tsv");
const COG_FUNCTIONS: &str = include_str!("../../data/cog_functions.txt");

/// Functional category code -> (class, category name).
fn functions() -> HashMap<&'static str, (&'static str, &'static str)> {
    let mut map = HashMap::new();
    let mut class = "";
    for line in COG_FUNCTIONS.lines() {
        let line = text::strip(line);
        if line.is_empty() {
            continue;
        }
        match line.split_once(':') {
            Some((code, name)) => {
                map.insert(code, (class, name));
            }
            None => class = line,
        }
    }
    map
}

/// COG identifier -> functional category code(s).
fn codes() -> HashMap<&'static str, &'static str> {
    COG_CODES
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .collect()
}

/// Turn an IMG/M COG profile (COG id, COG annotation, one count column per
/// sample) into a STAMP profile with the COG classes and categories as the
/// upper hierarchical levels.
pub fn append_categories(text: &str, treatment: MultiCodeTreatment) -> Result<String> {
    let functions = functions();
    let codes = codes();
    let lines = text::lines(text);
    let header = lines
        .first()
        .ok_or_else(|| StampError::new("The IMG/M profile is empty."))?;

    let mut out = String::from(
        "COG classes\tCOG category names\tCOG category codes\tCOG annotations\tCOG IDs",
    );
    for sample in text::strip(header).split('\t').skip(2) {
        out.push('\t');
        out.push_str(sample);
    }
    out.push('\n');

    for (i, line) in lines.iter().enumerate().skip(1) {
        let line = text::strip(line);
        if line.is_empty() {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').collect();
        if cells.len() < 3 {
            return Err(StampError::new(format!(
                "Line {} of the IMG/M profile does not have a COG id, an annotation and counts.",
                i + 1
            )));
        }
        let (cog_id, annotation, counts) = (cells[0], cells[1], cells[2..].join("\t"));
        let code = *codes
            .get(cog_id)
            .ok_or_else(|| StampError::new(format!("Unknown COG identifier '{cog_id}'.")))?;

        let mut write = |category_code: &str| {
            let (class, name) = functions
                .get(category_code)
                .copied()
                .unwrap_or((code, code));
            out.push_str(&[class, name, category_code, annotation, cog_id, &counts].join("\t"));
            out.push('\n');
        };
        match treatment {
            MultiCodeTreatment::AsFeature => write(code),
            MultiCodeTreatment::AssignToEachCode => {
                for (start, ch) in code.char_indices() {
                    write(&code[start..start + ch.len_utf8()]);
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_tables_are_complete() {
        assert_eq!(codes().len(), 4873);
        assert_eq!(codes()["COG0001"], "H");
        let functions = functions();
        assert_eq!(functions.len(), 25);
        assert_eq!(
            functions["J"],
            (
                "Information storage and processing",
                "Translation, ribosomal structure and biogenesis"
            )
        );
    }

    #[test]
    fn unknown_cogs_are_reported() {
        let err = append_categories(
            "id\tname\tS1\tS2\nCOG9999\tx\t1\t2\n",
            MultiCodeTreatment::AsFeature,
        );
        assert_eq!(err.unwrap_err().0, "Unknown COG identifier 'COG9999'.");
    }
}
