//! BIOM tables: the JSON format (version 1.0) and the HDF5 format (version
//! 2.x), plain or gzip-compressed.

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

/// The contents of a BIOM table, whichever format it came in.
struct Table {
    rows: Vec<String>,
    columns: Vec<String>,
    /// The hierarchy of each row from the chosen metadata field; empty lists
    /// if none was chosen.
    lineages: Vec<Vec<String>>,
    /// `matrix[row][column]`
    matrix: Vec<Vec<f64>>,
}

const HDF5_MAGIC: &[u8] = b"\x89HDF";

/// The bytes of a table, decompressed if they were gzip-compressed.
fn decompress(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok(bytes.into());
    }
    let mut buffer = Vec::new();
    flate2::read::MultiGzDecoder::new(bytes)
        .read_to_end(&mut buffer)
        .map_err(|e| invalid(format_args!("it could not be decompressed ({e})")))?;
    Ok(buffer.into())
}

/// True if a file looks like a BIOM table: HDF5 (format 2.x), JSON (format
/// 1.0) or either of them gzip-compressed.
pub fn looks_like_biom(bytes: &[u8]) -> bool {
    let Ok(bytes) = decompress(bytes) else {
        return false;
    };
    bytes.starts_with(HDF5_MAGIC)
        || bytes
            .iter()
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|b| *b == b'{')
}

/// The observation metadata fields of a table that could supply a hierarchy,
/// such as `taxonomy`, in alphabetical order.
pub fn metadata_fields(bytes: &[u8]) -> Result<Vec<String>> {
    let bytes = decompress(bytes)?;
    let mut fields: Vec<String> = if bytes.starts_with(HDF5_MAGIC) {
        // A table without metadata may have no such group at all.
        open_hdf5(&bytes)?
            .group("observation/metadata")
            .and_then(|group| group.datasets())
            .unwrap_or_default()
    } else {
        let table = parse_json(&bytes)?;
        let mut fields = Vec::new();
        for row in table["rows"].as_array().into_iter().flatten() {
            if let Value::Object(metadata) = &row["metadata"] {
                fields.extend(metadata.keys().cloned());
            }
        }
        fields
    };
    fields.sort();
    fields.dedup();
    Ok(fields)
}

/// Convert a BIOM table to a STAMP profile. Both the JSON format (1.0) and
/// the HDF5 format (2.x) are read, plain or gzip-compressed.
///
/// With `metadata_field` set, that observation metadata field (for example
/// `taxonomy`) supplies the hierarchy, followed by the observation ids. With
/// `None`, the observation ids are the only level.
pub fn to_spf(bytes: &[u8], metadata_field: Option<&str>) -> Result<String> {
    let bytes = decompress(bytes)?;
    let table = if bytes.starts_with(HDF5_MAGIC) {
        read_hdf5(&bytes, metadata_field)?
    } else {
        read_json(&bytes, metadata_field)?
    };
    Ok(write_spf(table))
}

fn not_found(field: &str) -> StampError {
    StampError::new(format!("'{field}' was not found in the BIOM table."))
}

fn open_hdf5(bytes: &[u8]) -> Result<hdf5_pure::File> {
    hdf5_pure::File::from_bytes(bytes.to_vec()).map_err(|e| invalid(format_args!("{e}")))
}

/// Read a table in the HDF5 format (BIOM 2.x): ids as strings, counts as a
/// compressed sparse matrix with one run of entries per observation, and
/// each metadata field as a dataset of its own.
fn read_hdf5(bytes: &[u8], metadata_field: Option<&str>) -> Result<Table> {
    let file = open_hdf5(bytes)?;
    let unreadable =
        |path: &str, e: hdf5_pure::Error| invalid(format_args!("'{path}' could not be read ({e})"));
    let strings = |path: &str| -> Result<Vec<String>> {
        file.dataset(path)
            .and_then(|dataset| dataset.read_string())
            .map_err(|e| unreadable(path, e))
    };
    let integers = |path: &str| -> Result<Vec<i64>> {
        file.dataset(path)
            .and_then(|dataset| dataset.read_i64())
            .map_err(|e| unreadable(path, e))
    };
    let rows = strings("observation/ids")?;
    let columns = strings("sample/ids")?;
    let data = file
        .dataset("observation/matrix/data")
        .and_then(|dataset| dataset.read_f64())
        .map_err(|e| unreadable("observation/matrix/data", e))?;
    let indices = integers("observation/matrix/indices")?;
    let starts = integers("observation/matrix/indptr")?;

    let inconsistent = || invalid("its matrix does not match its rows and columns");
    if starts.len() != rows.len() + 1
        || indices.len() != data.len()
        || starts.first() != Some(&0)
        || starts.last() != Some(&(data.len() as i64))
        || starts.windows(2).any(|pair| pair[0] > pair[1])
    {
        return Err(inconsistent());
    }
    let mut matrix = vec![vec![0.0f64; columns.len()]; rows.len()];
    for (row, counts) in matrix.iter_mut().enumerate() {
        for entry in starts[row] as usize..starts[row + 1] as usize {
            let column = usize::try_from(indices[entry]).map_err(|_| inconsistent())?;
            *counts.get_mut(column).ok_or_else(inconsistent)? = data[entry];
        }
    }

    let mut lineages: Vec<Vec<String>> = vec![Vec::new(); rows.len()];
    if let Some(field) = metadata_field {
        let path = format!("observation/metadata/{field}");
        let dataset = file.dataset(&path).map_err(|_| not_found(field))?;
        let shape = dataset.shape().map_err(|e| unreadable(&path, e))?;
        let values = dataset.read_string().map_err(|e| unreadable(&path, e))?;
        let per_row = match shape.as_slice() {
            // One string per observation, split on ';' as QIIME writes it.
            [n] if *n as usize == rows.len() => 1,
            // A row of levels per observation, padded with empty strings.
            [n, levels] if *n as usize == rows.len() => *levels as usize,
            _ => {
                return Err(invalid(format_args!(
                    "'{field}' does not have one entry per row"
                )))
            }
        };
        if values.len() != rows.len() * per_row {
            return Err(invalid(format_args!(
                "'{field}' does not have one entry per row"
            )));
        }
        for (row, lineage) in lineages.iter_mut().enumerate() {
            let levels = &values[row * per_row..(row + 1) * per_row];
            *lineage = if shape.len() == 1 {
                levels[0]
                    .split(';')
                    .map(|level| text::strip(level).to_string())
                    .filter(|level| !level.is_empty())
                    .collect()
            } else {
                levels
                    .iter()
                    .map(|level| text::strip(level).to_string())
                    .filter(|level| !level.is_empty())
                    .collect()
            };
        }
    }
    Ok(Table {
        rows,
        columns,
        lineages,
        matrix,
    })
}

fn parse_json(bytes: &[u8]) -> Result<Value> {
    serde_json::from_str(&text::decode(bytes)).map_err(|e| invalid(format_args!("{e}")))
}

/// Read a table in the JSON format (BIOM 1.0).
fn read_json(bytes: &[u8], metadata_field: Option<&str>) -> Result<Table> {
    let table = parse_json(bytes)?;
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
                _ => return Err(not_found(field)),
            };
        }
    }

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
    Ok(Table {
        rows: rows.iter().map(id).collect::<Result<_>>()?,
        columns: columns.iter().map(id).collect::<Result<_>>()?,
        lineages,
        matrix,
    })
}

/// The table as the text of a STAMP profile.
fn write_spf(table: Table) -> String {
    let num_levels = table.lineages.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = String::new();
    let mut header: Vec<String> = (1..=num_levels).map(|i| format!("Level_{i}")).collect();
    header.push("Observation Ids".to_string());
    header.extend(table.columns);
    out.push_str(&header.join("\t"));
    out.push('\n');

    for ((observation, lineage), counts) in table
        .rows
        .into_iter()
        .zip(table.lineages)
        .zip(&table.matrix)
    {
        let mut cells = lineage;
        cells.resize(num_levels, "unclassified".to_string());
        // A purely numeric id would be mistaken for a sample column.
        cells.push(if text::is_number(&observation) {
            format!("ID{observation}")
        } else {
            observation
        });
        cells.extend(counts.iter().map(|&c| text::format_float(c)));
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    out
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
            .contains("valid BIOM table"));
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

    #[test]
    fn hdf5_tables_give_the_same_profile_as_json() {
        // The same table written by biom-format 2.1.18 in both formats.
        let json = include_bytes!("../../tests/data/otu_table_v1.biom");
        let hdf5 = include_bytes!("../../tests/data/otu_table_v2.biom");
        assert!(looks_like_biom(json) && looks_like_biom(hdf5));
        assert!(!looks_like_biom(b"Phylum\tS1\tS2\n"));
        assert_eq!(metadata_fields(json).unwrap(), ["taxonomy"]);
        assert_eq!(metadata_fields(hdf5).unwrap(), ["taxonomy"]);

        let plain = to_spf(hdf5, None).unwrap();
        assert_eq!(plain, to_spf(json, None).unwrap());
        assert_eq!(
            plain,
            "Observation Ids\tS1\tS2\tS3\tS4\n\
             OTU_1\t10.0\t0.0\t3.0\t7.0\n\
             OTU_2\t0.0\t5.0\t0.0\t2.0\n\
             OTU_3\t8.0\t8.0\t1.0\t0.0\n\
             ID4\t0.0\t0.0\t12.0\t4.0\n\
             OTU_5\t1.0\t2.0\t3.0\t4.5\n"
        );
        let with_taxonomy = to_spf(hdf5, Some("taxonomy")).unwrap();
        assert_eq!(with_taxonomy, to_spf(json, Some("taxonomy")).unwrap());
        assert!(with_taxonomy.starts_with("Level_1\tLevel_2\tLevel_3\tObservation Ids\tS1"));
        assert!(with_taxonomy.contains("k__Archaea\tp__Euryarchaeota\tc__Methanobacteria\tID4\t"));
        assert_eq!(
            to_spf(hdf5, Some("KEGG_Pathways")).unwrap_err().0,
            "'KEGG_Pathways' was not found in the BIOM table."
        );

        // Without metadata, and compressed.
        let bare = include_bytes!("../../tests/data/plain_v2.biom");
        assert!(metadata_fields(bare).unwrap().is_empty());
        assert_eq!(to_spf(bare, None).unwrap(), plain);
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(hdf5).unwrap();
        assert_eq!(to_spf(&encoder.finish().unwrap(), None).unwrap(), plain);
    }

    #[test]
    fn a_larger_hdf5_table_matches_its_json_original() {
        // An example table of this repository, converted by biom-format.
        let hdf5 = include_bytes!("../../tests/data/predicted_metagenomes.L3.v2.biom");
        let json = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/BIOM-test/predicted_metagenomes.L3.biom"
        ))
        .unwrap();
        let (from_hdf5, from_json) = (to_spf(hdf5, None).unwrap(), to_spf(&json, None).unwrap());
        assert_eq!(from_hdf5.lines().count(), 334);
        assert_eq!(from_hdf5, from_json);
    }
}
