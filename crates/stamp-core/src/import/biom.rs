//! BIOM tables (JSON, format version 1.0), plain or gzip-compressed.

use std::io::Read;

use serde_json::Value;

use crate::error::{Result, StampError};
use crate::text;

/// Observation metadata fields offered by default, with the analysis that
/// produces them.
pub const COMMON_METADATA_FIELDS: [(&str, &str); 3] = [
    ("COG_Category", "PICRUSt COG analysis"),
    ("KEGG_Pathways", "PICRUSt KEGG analysis"),
    ("taxonomy", "QIIME OTU table"),
];

fn invalid(reason: impl std::fmt::Display) -> StampError {
    StampError::new(format!(
        "This file does not appear to be a valid BIOM table: {reason}."
    ))
}

/// Convert a BIOM table to a STAMP profile.
///
/// With `metadata_field` set, that observation metadata field (for example
/// `taxonomy`) supplies the hierarchy, followed by the observation ids. With
/// `None`, the observation ids are the only level.
pub fn to_spf(bytes: &[u8], metadata_field: Option<&str>) -> Result<String> {
    let decompressed;
    let bytes = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut buffer = Vec::new();
        flate2::read::MultiGzDecoder::new(bytes)
            .read_to_end(&mut buffer)
            .map_err(|e| invalid(format_args!("it could not be decompressed ({e})")))?;
        decompressed = buffer;
        &decompressed[..]
    } else {
        bytes
    };

    if bytes.starts_with(b"\x89HDF") {
        return Err(StampError::new(
            "This is a BIOM 2.x (HDF5) table, which is not supported. Convert it to the JSON \
             format first, for example with 'biom convert --to-json'.",
        ));
    }

    let table: Value =
        serde_json::from_str(&text::decode(bytes)).map_err(|e| invalid(format_args!("{e}")))?;
    let rows = table["rows"]
        .as_array()
        .ok_or_else(|| invalid("it has no 'rows'"))?;
    let columns = table["columns"]
        .as_array()
        .ok_or_else(|| invalid("it has no 'columns'"))?;

    let id = |entry: &Value| -> Result<String> {
        match &entry["id"] {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            _ => Err(invalid("a row or column has no 'id'")),
        }
    };

    // Hierarchy from the chosen metadata field, one list of levels per row.
    let mut lineages: Vec<Vec<String>> = vec![Vec::new(); rows.len()];
    if let Some(field) = metadata_field {
        let not_found = || StampError::new(format!("'{field}' was not found in the BIOM table."));
        for (lineage, row) in lineages.iter_mut().zip(rows) {
            *lineage = match &row["metadata"][field] {
                Value::Array(levels) => levels
                    .iter()
                    .map(|level| match level {
                        Value::String(s) => Ok(s.clone()),
                        Value::Array(_) => Err(StampError::new(format!(
                            "'{field}' lists more than one hierarchy per observation, which \
                             cannot be used as the hierarchy of a profile."
                        ))),
                        other => Ok(other.to_string()),
                    })
                    .collect::<Result<_>>()?,
                // A single string is split on ';', as QIIME writes taxonomy.
                Value::String(s) => s.split(';').map(|l| text::strip(l).to_string()).collect(),
                _ => return Err(not_found()),
            };
        }
    }
    let num_levels = lineages.iter().map(Vec::len).max().unwrap_or(0);

    // Counts as a dense matrix.
    let mut matrix = vec![vec![0.0f64; columns.len()]; rows.len()];
    let data = table["data"]
        .as_array()
        .ok_or_else(|| invalid("it has no 'data'"))?;
    let number = |v: &Value| {
        v.as_f64()
            .ok_or_else(|| invalid("'data' holds a non-numeric value"))
    };
    if table["matrix_type"].as_str() == Some("dense") {
        if data.len() != rows.len() {
            return Err(invalid("'data' does not have one entry per row"));
        }
        for (target, source) in matrix.iter_mut().zip(data) {
            let source = source
                .as_array()
                .ok_or_else(|| invalid("'data' is not a matrix"))?;
            if source.len() != columns.len() {
                return Err(invalid("'data' does not have one value per column"));
            }
            for (cell, value) in target.iter_mut().zip(source) {
                *cell = number(value)?;
            }
        }
    } else {
        for entry in data {
            let bad = || invalid("'data' is not a list of [row, column, value] entries");
            let entry = entry.as_array().filter(|e| e.len() == 3).ok_or_else(bad)?;
            let r = entry[0].as_u64().ok_or_else(bad)? as usize;
            let c = entry[1].as_u64().ok_or_else(bad)? as usize;
            if r >= rows.len() || c >= columns.len() {
                return Err(invalid(
                    "'data' refers to a row or column that does not exist",
                ));
            }
            matrix[r][c] = number(&entry[2])?;
        }
    }

    let mut out = String::new();
    let mut header: Vec<String> = (1..=num_levels).map(|i| format!("Level_{i}")).collect();
    header.push("Observation Ids".to_string());
    for column in columns {
        header.push(id(column)?);
    }
    out.push_str(&header.join("\t"));
    out.push('\n');

    for ((row, lineage), counts) in rows.iter().zip(lineages).zip(&matrix) {
        let mut cells = lineage;
        cells.resize(num_levels, "unclassified".to_string());
        // A purely numeric id would be mistaken for a sample column.
        let observation = id(row)?;
        cells.push(if text::is_number(&observation) {
            format!("ID{observation}")
        } else {
            observation
        });
        cells.extend(counts.iter().map(|&c| text::format_float(c)));
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPARSE: &str = r#"{
        "format": "Biological Observation Matrix 1.0.0", "matrix_type": "sparse",
        "shape": [3, 2],
        "rows": [
            {"id": "OTU_a", "metadata": {"taxonomy": ["k__Bacteria", "p__Firmicutes"]}},
            {"id": "42", "metadata": {"taxonomy": ["k__Bacteria"]}},
            {"id": "OTU_c", "metadata": {"taxonomy": "k__Archaea; p__Euryarchaeota"}}
        ],
        "columns": [{"id": "S1", "metadata": null}, {"id": "S2", "metadata": null}],
        "data": [[0, 0, 5], [0, 1, 1.5], [2, 1, 7]]
    }"#;

    #[test]
    fn observation_ids_only() {
        assert_eq!(
            to_spf(SPARSE.as_bytes(), None).unwrap(),
            "Observation Ids\tS1\tS2\nOTU_a\t5.0\t1.5\nID42\t0.0\t0.0\nOTU_c\t0.0\t7.0\n"
        );
    }

    #[test]
    fn metadata_field_gives_the_hierarchy() {
        assert_eq!(
            to_spf(SPARSE.as_bytes(), Some("taxonomy")).unwrap(),
            "Level_1\tLevel_2\tObservation Ids\tS1\tS2\n\
             k__Bacteria\tp__Firmicutes\tOTU_a\t5.0\t1.5\n\
             k__Bacteria\tunclassified\tID42\t0.0\t0.0\n\
             k__Archaea\tp__Euryarchaeota\tOTU_c\t0.0\t7.0\n"
        );
        let err = to_spf(SPARSE.as_bytes(), Some("KEGG_Pathways")).unwrap_err();
        assert_eq!(err.0, "'KEGG_Pathways' was not found in the BIOM table.");
    }

    #[test]
    fn dense_and_gzip_tables() {
        let dense = r#"{"matrix_type": "dense", "rows": [{"id": "a"}, {"id": "b"}],
            "columns": [{"id": "S1"}, {"id": "S2"}], "data": [[1, 2], [3, 4]]}"#;
        let expected = "Observation Ids\tS1\tS2\na\t1.0\t2.0\nb\t3.0\t4.0\n";
        assert_eq!(to_spf(dense.as_bytes(), None).unwrap(), expected);

        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(dense.as_bytes()).unwrap();
        assert_eq!(to_spf(&encoder.finish().unwrap(), None).unwrap(), expected);
    }

    #[test]
    fn rejects_other_files() {
        assert!(to_spf(b"\x89HDF\r\n\x1a\n....", None)
            .unwrap_err()
            .0
            .contains("HDF5"));
        assert!(to_spf(b"not json", None)
            .unwrap_err()
            .0
            .contains("valid BIOM table"));
        assert!(to_spf(b"{}", None).unwrap_err().0.contains("no 'rows'"));
        let nested = r#"{"rows": [{"id": "a", "metadata": {"KEGG_Pathways": [["x", "y"]]}}],
            "columns": [{"id": "S1"}, {"id": "S2"}], "data": []}"#;
        assert!(to_spf(nested.as_bytes(), Some("KEGG_Pathways"))
            .unwrap_err()
            .0
            .contains("more than one hierarchy"));
    }
}
