//! Sandpiper condensed profiles: the JSON that
//! `https://sandpiper.qut.edu.au/api/condensed/<run>` returns, one sample
//! per file, `{"condensed": <tree>, "sample_name": "<run>"}`. Each node of
//! the tree has a `name` (without a rank prefix; the root's is empty), its
//! own coverage as `size`, and its `children`. GTDB and GlobDB taxonomies
//! have the same form. See [`lineage`](super::lineage) for how the
//! coverages become a profile.

use serde_json::Value;

use super::lineage::{strip_rank_prefix, Lineages};
use super::NamedText;
use crate::error::{Result, StampError};

/// True if a file looks like a Sandpiper condensed profile: a JSON object
/// whose first key is `condensed`, or whose start mentions
/// `condensed_depth`.
pub fn looks_like_sandpiper(bytes: &[u8]) -> bool {
    let start = &bytes[..bytes.len().min(65536)];
    let text = String::from_utf8_lossy(start);
    let text = text.trim_start_matches('\u{feff}').trim_start();
    let Some(rest) = text.strip_prefix('{') else {
        return false;
    };
    rest.trim_start().starts_with("\"condensed\"") || text.contains("\"condensed_depth\"")
}

/// Sample name of a file without one: the file name without its directory
/// and extension.
fn file_stem(name: &str) -> &str {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    base.strip_suffix(".json").unwrap_or(base)
}

/// Combine Sandpiper condensed profiles, one sample per file, into a STAMP
/// profile of the seven ranks from domain to species. Sample names come
/// from `sample_name`, or from the file name if that is missing.
pub fn to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let mut lineages = Lineages::new();
    for file in files {
        let invalid = |reason: &str| {
            StampError::new(format!(
                "{} does not appear to be a Sandpiper condensed profile: {reason}.",
                file.name
            ))
        };
        let json: Value = serde_json::from_str(file.text)
            .map_err(|e| invalid(&format!("it is not valid JSON ({e})")))?;
        let tree = json
            .get("condensed")
            .filter(|t| t.is_object())
            .ok_or_else(|| invalid("it has no \"condensed\" tree"))?;
        let sample = json
            .get("sample_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| file_stem(file.name));
        if lineages.has_sample(sample) {
            return Err(StampError::new(format!(
                "Sample '{sample}' is in more than one file."
            )));
        }
        let index = lineages.sample(sample);
        let mut path: Vec<&str> = Vec::new();
        add_node(tree, &mut path, index, &mut lineages)
            .map_err(|e| StampError::new(format!("{}: {}", file.name, e.0)))?;
    }
    lineages.to_spf()
}

/// Add the coverage of a node and its descendants.
fn add_node<'a>(
    node: &'a Value,
    path: &mut Vec<&'a str>,
    sample: usize,
    lineages: &mut Lineages,
) -> Result<()> {
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .map(strip_rank_prefix)
        .unwrap_or("");
    let named = !name.is_empty();
    if named {
        path.push(name);
    }
    let size = node.get("size").and_then(Value::as_f64).unwrap_or(0.0);
    if size.is_finite() && size > 0.0 {
        lineages.add(sample, path, size)?;
    }
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            add_node(child, path, sample, lineages)?;
        }
    }
    if named {
        path.pop();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: &str = r#"{"condensed": {"name": "", "size": 0.0, "condensed_depth": 0, "order": 0,
        "children": [
          {"name": "Bacteria", "size": 3.0, "condensed_depth": 1, "order": 0, "children": [
            {"name": "Bacillota", "size": 0.18000000715255737, "condensed_depth": 2, "order": 0,
             "children": [
              {"name": "Clostridia", "size": 0.0, "condensed_depth": 3, "order": 0, "children": [
                {"name": "Oscillospirales", "size": 0.0, "condensed_depth": 4, "order": 0,
                 "children": [
                  {"name": "Oscillospiraceae", "size": 0.0, "condensed_depth": 5, "order": 0,
                   "children": [
                    {"name": "Limivicinus", "size": 1.7000000476837158, "condensed_depth": 6,
                     "order": 0, "children": [
                      {"name": "Limivicinus sp030531505", "size": 0.5, "condensed_depth": 7,
                       "order": 0}]}]}]}]}]}]},
          {"name": "Archaea", "size": 0.44999998807907104, "condensed_depth": 1, "order": 1,
           "children": []}]},
      "sample_name": "SRR1"}"#;

    #[test]
    fn builds_seven_ranks_from_the_tree() {
        assert!(looks_like_sandpiper(RUN.as_bytes()));
        assert!(!looks_like_sandpiper(br#"{"id": "biom", "rows": []}"#));
        let other = RUN.replace("\"sample_name\": \"SRR1\"", "\"other\": 1");
        let files = [
            NamedText {
                name: "a.json",
                text: RUN,
            },
            NamedText {
                name: "/data/SRR2.json",
                text: &other,
            },
        ];
        let spf = to_spf(&files).unwrap();
        let lines: Vec<&str> = spf.lines().collect();
        assert_eq!(
            lines[0],
            "Domain\tPhylum\tClass\tOrder\tFamily\tGenus\tSpecies\tSRR1\tSRR2"
        );
        assert_eq!(
            lines[1],
            format!(
                "Bacteria{}\t3.0\t3.0",
                "\tunclassified d__Bacteria".repeat(6)
            )
        );
        assert_eq!(
            lines[2],
            format!(
                "Bacteria\tBacillota{}\t0.18\t0.18",
                "\tunclassified p__Bacillota".repeat(5)
            )
        );
        assert_eq!(
            lines[3],
            "Bacteria\tBacillota\tClostridia\tOscillospirales\tOscillospiraceae\tLimivicinus\t\
             unclassified g__Limivicinus\t1.7\t1.7"
        );
        assert_eq!(
            lines[4],
            "Bacteria\tBacillota\tClostridia\tOscillospirales\tOscillospiraceae\tLimivicinus\t\
             Limivicinus sp030531505\t0.5\t0.5"
        );
        assert_eq!(
            lines[5],
            format!(
                "Archaea{}\t0.45\t0.45",
                "\tunclassified d__Archaea".repeat(6)
            )
        );
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn reports_problems() {
        let bad = [NamedText {
            name: "x.json",
            text: "{\"rows\": []}",
        }];
        assert_eq!(
            to_spf(&bad).unwrap_err().0,
            "x.json does not appear to be a Sandpiper condensed profile: it has no \"condensed\" tree."
        );
        let twice = [
            NamedText {
                name: "a.json",
                text: RUN,
            },
            NamedText {
                name: "b.json",
                text: RUN,
            },
        ];
        assert_eq!(
            to_spf(&twice).unwrap_err().0,
            "Sample 'SRR1' is in more than one file."
        );
        assert!(to_spf(&[]).is_err());
    }
}
