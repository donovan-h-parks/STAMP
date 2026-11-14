//! Converters from other tools' output to STAMP profile files (`.spf`).
//!
//! Every converter takes file contents and returns the text of a STAMP
//! profile, which can be saved or passed straight to
//! [`ProfileTree::read`](crate::ProfileTree::read). Rows are written in order
//! of first appearance in the input.

pub mod biom;
pub mod cog;
pub mod comet;
pub mod lineage;
pub mod mgrast;
pub mod mothur;
pub mod rita;
pub mod sandpiper;
pub mod singlem;

/// The contents of an input file together with its file name, for converters
/// that derive sample names from file names.
#[derive(Debug, Clone, Copy)]
pub struct NamedText<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

/// Sample name used for a CoMet or RITA result file: the file name up to its
/// first `.`, without anything up to and including the first `_`.
/// `profilePfam_Coral.fa.txt` becomes `Coral`.
pub fn sample_name_from_file_name(file_name: &str) -> &str {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = base.split('.').next().unwrap_or(base);
    match stem.find('_') {
        Some(i) => &stem[i + 1..],
        None => stem,
    }
}

/// Rows of an output profile, keyed and kept in order of first appearance.
pub(crate) struct Rows<V> {
    order: Vec<String>,
    rows: std::collections::HashMap<String, V>,
}

impl<V> Rows<V> {
    pub(crate) fn new() -> Self {
        Rows {
            order: Vec::new(),
            rows: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn entry(&mut self, key: &str, create: impl FnOnce() -> V) -> &mut V {
        if !self.rows.contains_key(key) {
            self.order.push(key.to_string());
            self.rows.insert(key.to_string(), create());
        }
        self.rows.get_mut(key).expect("row was just inserted")
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.order.iter().map(|k| (k.as_str(), &self.rows[k]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_names_from_file_names() {
        assert_eq!(
            sample_name_from_file_name("profilePfam_Coral.fa.txt"),
            "Coral"
        );
        assert_eq!(sample_name_from_file_name("/data/run_A_1.txt"), "A_1");
        assert_eq!(sample_name_from_file_name("C:\\data\\gut.txt"), "gut");
        assert_eq!(sample_name_from_file_name("plain"), "plain");
    }
}
