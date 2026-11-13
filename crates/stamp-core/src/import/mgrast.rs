//! MG-RAST functional and taxonomic abundance tables.

use std::collections::HashMap;

use super::Rows;
use crate::error::{Result, StampError};
use crate::text;

/// A parsed MG-RAST table, ready to be written as a STAMP profile.
#[derive(Debug, Clone)]
pub struct MgRastProfile {
    /// Headings of the hierarchical levels. May be edited before conversion.
    pub headings: Vec<String>,
    header: Vec<String>,
    lines: Vec<String>,
    start_index: usize,
    data_index: usize,
}

fn invalid(reason: &str) -> StampError {
    StampError::new(format!(
        "This file does not appear to be a valid MG-RAST profile as {reason}."
    ))
}

impl MgRastProfile {
    pub fn parse(text: &str) -> Result<MgRastProfile> {
        let lines: Vec<String> = text::lines(text)
            .into_iter()
            .map(|l| text::strip(l).to_string())
            .collect();
        let header: Vec<String> = match lines.first() {
            Some(first) => first.split('\t').map(str::to_string).collect(),
            None => return Err(invalid("it is empty")),
        };

        if text::strip(&header[0]) != "metagenome" {
            return Err(invalid("it does not begin with a 'metagenome' column"));
        }
        let data_index = header
            .iter()
            .position(|h| h == "abundance")
            .ok_or_else(|| invalid("it does not contain an 'abundance' column"))?;
        let start_index = match header.get(1).map(String::as_str) {
            Some("level 1") | Some("domain") => 1,
            Some("source") => 2,
            _ => {
                return Err(invalid(
                    "the second column is not 'level 1', 'source', or 'domain'",
                ))
            }
        };
        if data_index <= start_index {
            return Err(invalid(
                "it has no hierarchy columns before the 'abundance' column",
            ));
        }

        Ok(MgRastProfile {
            headings: header[start_index..data_index].to_vec(),
            header,
            lines,
            start_index,
            data_index,
        })
    }

    /// Convert to the text of a STAMP profile.
    pub fn to_spf(&self) -> Result<String> {
        let num_levels = self.data_index - self.start_index;
        if self.headings.len() != num_levels {
            return Err(StampError::new(format!(
                "This MG-RAST profile has {num_levels} hierarchical levels, but {} headings were given.",
                self.headings.len()
            )));
        }
        let taxonomic = self.header[1] == "domain";
        let rows: Vec<Vec<&str>> = self.lines[1..]
            .iter()
            .filter(|l| !l.is_empty())
            .map(|l| l.split('\t').collect())
            .collect();

        let mut sample_names: Vec<&str> = Vec::new();
        for row in &rows {
            if !sample_names.contains(&row[0]) {
                sample_names.push(row[0]);
            }
        }

        // For each level, the parents each child name has been seen under.
        let mut parent_map: Vec<HashMap<String, Vec<String>>> = vec![HashMap::new(); num_levels];
        let mut profile: Rows<(Vec<String>, Vec<i64>)> = Rows::new();

        for row in &rows {
            if row.len() <= self.data_index {
                return Err(StampError::new(
                    "Your file does not appear to be a valid MG-RAST profile.",
                ));
            }
            let count: i64 = text::strip(row[self.data_index]).parse().map_err(|_| {
                StampError::new(format!(
                    "Your file does not appear to be a valid MG-RAST profile: '{}' is not a whole-number abundance.",
                    row[self.data_index]
                ))
            })?;

            let mut hierarchy: Vec<String> = row[self.start_index..self.data_index]
                .iter()
                .map(|s| s.to_string())
                .collect();

            // Missing levels ('-') take their name from the level above.
            for i in 1..hierarchy.len() {
                if hierarchy[i] == "-" {
                    hierarchy[i] = if taxonomic && !hierarchy[i - 1].contains("Unclassified") {
                        format!("Unclassified {}", hierarchy[i - 1])
                    } else {
                        hierarchy[i - 1].clone()
                    };
                }
            }

            // A child seen under more than one parent gets a numbered suffix
            // so that the profile forms a strict hierarchy.
            for i in 1..hierarchy.len() {
                let parent = hierarchy[..i].join("-");
                let parents = parent_map[i].entry(hierarchy[i].clone()).or_default();
                let parent_index = match parents.iter().position(|p| *p == parent) {
                    Some(index) => index,
                    None => {
                        parents.push(parent);
                        parents.len() - 1
                    }
                };
                if parent_index != 0 {
                    hierarchy[i] = format!("{} - #{parent_index}", hierarchy[i]);
                }
            }

            let sample = sample_names
                .iter()
                .position(|s| *s == row[0])
                .expect("sample was listed");
            let key = hierarchy.last().expect("at least one level").clone();
            let entry = profile.entry(&key, || (hierarchy, vec![0; sample_names.len()]));
            entry.1[sample] += count;
        }

        let mut out = String::new();
        out.push_str(&self.headings.join("\t"));
        for sample in &sample_names {
            out.push('\t');
            out.push_str(sample);
        }
        out.push('\n');
        for (_, (hierarchy, counts)) in profile.iter() {
            for level in hierarchy {
                out.push_str(level);
                out.push('\t');
            }
            let counts: Vec<String> = counts.iter().map(i64::to_string).collect();
            out.push_str(&counts.join("\t"));
            out.push('\n');
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_functional_table() {
        let text = "metagenome\tlevel 1\tlevel 2\tfunction\tabundance\tavg eValue\t\n\
            m1\tCarbs\tSugars\tKinase\t5\t-3\t\n\
            m2\tCarbs\tSugars\tKinase\t7\t-3\t\n\
            \n\
            m1\tLipids\t-\tKinase\t2\t-3\t\n\
            m1\tCarbs\tSugars\tKinase\t1\t-3\t\n";
        let profile = MgRastProfile::parse(text).unwrap();
        assert_eq!(profile.headings, ["level 1", "level 2", "function"]);
        assert_eq!(
            profile.to_spf().unwrap(),
            "level 1\tlevel 2\tfunction\tm1\tm2\n\
             Carbs\tSugars\tKinase\t6\t7\n\
             Lipids\tLipids\tKinase - #1\t2\t0\n"
        );
    }

    #[test]
    fn taxonomic_tables_mark_missing_levels_unclassified() {
        let text = "metagenome\tdomain\tphylum\tclass\tabundance\n\
            m1\tBacteria\t-\t-\t3\n\
            m2\tBacteria\tFirmicutes\tBacilli\t4\n";
        let mut profile = MgRastProfile::parse(text).unwrap();
        profile.headings = vec!["Domain".into(), "Phylum".into(), "Class".into()];
        assert_eq!(
            profile.to_spf().unwrap(),
            "Domain\tPhylum\tClass\tm1\tm2\n\
             Bacteria\tUnclassified Bacteria\tUnclassified Bacteria\t3\t0\n\
             Bacteria\tFirmicutes\tBacilli\t0\t4\n"
        );
    }

    #[test]
    fn skips_the_source_column() {
        let text = "metagenome\tsource\tlevel 1\tabundance\nm1\tSubsystems\tCarbs\t3\nm2\tSubsystems\tCarbs\t1\n";
        let spf = MgRastProfile::parse(text).unwrap().to_spf().unwrap();
        assert_eq!(spf, "level 1\tm1\tm2\nCarbs\t3\t1\n");
    }

    #[test]
    fn rejects_other_files() {
        let err = |t: &str| MgRastProfile::parse(t).unwrap_err().0;
        assert!(err("sample\tlevel 1\tabundance\n").contains("'metagenome' column"));
        assert!(err("metagenome\tlevel 1\tcount\n").contains("'abundance' column"));
        assert!(err("metagenome\tfoo\tabundance\n").contains("second column"));
        let bad = MgRastProfile::parse("metagenome\tlevel 1\tabundance\nm1\tx\tmany\n").unwrap();
        assert!(bad.to_spf().is_err());
    }
}
