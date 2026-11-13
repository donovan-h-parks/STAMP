//! CoMet profiles: one file per sample with lines such as
//! `PF00361 (NADH-Ubiquinone/plastoquinone ...): 5371`.

use super::{sample_name_from_file_name, NamedText, Rows};
use crate::error::{Result, StampError};
use crate::text;

/// Combine one CoMet result file per sample into a STAMP profile.
pub fn to_spf(files: &[NamedText<'_>]) -> Result<String> {
    let invalid = |file: &str| {
        StampError::new(format!(
            "{file} does not appear to be a valid CoMet profile."
        ))
    };

    let mut profile: Rows<Vec<Option<f64>>> = Rows::new();
    for (sample, file) in files.iter().enumerate() {
        for line in text::lines(file.text) {
            let line = text::strip(line);
            if line.is_empty() {
                continue;
            }
            let (Some(_), Some(colon), Some(open), Some(close)) = (
                line.find(' '),
                line.rfind(':'),
                line.find('('),
                line.rfind(')'),
            ) else {
                return Err(invalid(file.name));
            };
            if close < open {
                return Err(invalid(file.name));
            }
            // An entry without a description is named by its identifier;
            // an empty name would corrupt the row.
            let mut category = text::strip(&line[open + 1..close]);
            if category.is_empty() {
                category = text::strip(&line[..open]);
            }
            let count = text::parse_float(&line[colon + 1..]).ok_or_else(|| invalid(file.name))?;

            let counts = profile.entry(category, || vec![None; files.len()]);
            counts[sample] = Some(counts[sample].unwrap_or(0.0) + count);
        }
    }

    let mut out = String::from("Category");
    for file in files {
        out.push('\t');
        out.push_str(sample_name_from_file_name(file.name));
    }
    out.push('\n');
    for (category, counts) in profile.iter() {
        out.push_str(category);
        for count in counts {
            out.push('\t');
            match count {
                Some(value) => out.push_str(&text::format_float(*value)),
                None => out.push('0'),
            }
        }
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_files() {
        let a =
            "PF1 (Kinase (EC 1.2)): 5\nPF2 (Oxidase): 2\n\nPF9 (Kinase (EC 1.2)): 1\nGO:7 (): 4\n";
        let b = "PF2 (Oxidase): 3\r\n";
        let files = [
            NamedText {
                name: "profilePfam_Coral.fa.txt",
                text: a,
            },
            NamedText {
                name: "profilePfam_Fish.fa.txt",
                text: b,
            },
        ];
        assert_eq!(
            to_spf(&files).unwrap(),
            "Category\tCoral\tFish\nKinase (EC 1.2)\t6.0\t0\nOxidase\t2.0\t3.0\nGO:7\t4.0\t0\n"
        );
    }

    #[test]
    fn rejects_other_files() {
        let files = [NamedText {
            name: "x.txt",
            text: "not a comet line\n",
        }];
        assert_eq!(
            to_spf(&files).unwrap_err().0,
            "x.txt does not appear to be a valid CoMet profile."
        );
    }
}
