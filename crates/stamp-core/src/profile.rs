//! Profiles: the features at one hierarchical level, with their counts and
//! the counts of their parent category, for a chosen set of samples.

use std::collections::HashMap;

use crate::error::{Result, StampError};
use crate::metadata::{Groups, Metadata};
use crate::profile_tree::{ProfileTree, ENTIRE_SAMPLE};

/// What to do with features that are unclassified at the profile level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unclassified {
    /// Treat them like any other feature.
    Retain,
    /// Drop their reads entirely.
    Remove,
    /// Drop them as features but keep their reads in the parent totals, so
    /// the relative frequencies of the other features are unchanged.
    FrequencyOnly,
}

/// One feature of a profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Name of the feature at the profile level.
    pub name: String,
    /// Names from the top hierarchical level down to the profile level.
    pub hierarchy: Vec<String>,
    /// Count in each sample of the profile.
    pub counts: Vec<f64>,
    parent: usize,
}

/// Features at one hierarchical level for an ordered set of samples.
#[derive(Debug, Clone)]
pub struct Profile {
    /// Headings of the hierarchical levels down to the profile level.
    pub hierarchy_headings: Vec<String>,
    /// Samples, in the order of the count columns.
    pub sample_names: Vec<String>,
    features: Vec<Feature>,
    index: HashMap<String, usize>,
    /// Counts per sample of every parent category.
    parents: Vec<Vec<f64>>,
}

/// Relative frequency of a feature in percent; 0 when the parent is empty.
pub fn percent(count: f64, parent_count: f64) -> f64 {
    if parent_count > 0.0 {
        count * 100.0 / parent_count
    } else {
        0.0
    }
}

impl Profile {
    /// Features in order of first appearance in the profile file's hierarchy.
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    pub fn feature(&self, name: &str) -> Option<&Feature> {
        self.index.get(name).map(|&i| &self.features[i])
    }

    pub fn num_features(&self) -> usize {
        self.features.len()
    }

    /// Count of the feature's parent category in each sample.
    pub fn parent_counts(&self, feature: &Feature) -> &[f64] {
        &self.parents[feature.parent]
    }

    pub fn num_parent_categories(&self) -> usize {
        self.parents.len()
    }

    /// Number of sequences in each sample: the sum over every parent
    /// category, so the parent count when the parent is the entire sample.
    pub fn sample_totals(&self) -> Vec<f64> {
        let mut totals = vec![0.0; self.sample_names.len()];
        for parent in &self.parents {
            for (total, count) in totals.iter_mut().zip(parent) {
                *total += count;
            }
        }
        totals
    }

    /// Relative frequency (%) of a feature in each sample, against its
    /// parent category.
    pub fn percentages(&self, feature: &Feature) -> Vec<f64> {
        let parents = self.parent_counts(feature);
        feature
            .counts
            .iter()
            .zip(parents)
            .map(|(&c, &p)| percent(c, p))
            .collect()
    }
}

impl ProfileTree {
    /// Build the profile of the given samples at `profile_level`, with
    /// relative frequencies taken against `parent_level` (`None` for the
    /// entire sample).
    pub fn profile(
        &self,
        samples: &[&str],
        parent_level: Option<&str>,
        profile_level: &str,
        unclassified: Unclassified,
    ) -> Result<Profile> {
        let parent_depth = self.level_depth(parent_level)?;
        let profile_depth = self.level_depth(Some(profile_level))?;
        if profile_depth <= parent_depth {
            return Err(StampError::new(
                "The profile level must be deeper in the hierarchy than the parent level.",
            ));
        }
        let num_levels = self.hierarchy_headings().len();

        let mut columns = Vec::with_capacity(samples.len());
        for sample in samples {
            columns.push(
                self.sample_index(sample)
                    .ok_or_else(|| StampError::new(format!("Unknown sample '{sample}'.")))?,
            );
        }

        let mut features: Vec<Feature> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut parents: Vec<Vec<f64>> = Vec::new();
        let mut parent_index: HashMap<&str, usize> = HashMap::new();

        for leaf in self.leaves() {
            let node_name = self.node_name(leaf.path[profile_depth - 1]);

            let mut as_feature = true;
            if node_name.to_ascii_lowercase().contains("unclassified") {
                match unclassified {
                    Unclassified::Retain => {}
                    Unclassified::Remove => continue,
                    Unclassified::FrequencyOnly => as_feature = false,
                }
            }

            let mut feature_index = None;
            if as_feature {
                // Leaf features that an importer disambiguated with a " - #n"
                // suffix are compared without it when the parent is the whole
                // sample. Only the first of each such name is kept; the
                // others are dropped, reads included.
                let mut name = node_name;
                let mut truncated = false;
                if profile_depth == num_levels && parent_depth == 0 {
                    if let Some(pos) = node_name.rfind(" - #") {
                        name = &node_name[..pos];
                        truncated = true;
                    }
                }

                let existing = index.get(name).copied();
                if truncated && existing.is_some() {
                    continue;
                }
                let i = existing.unwrap_or_else(|| {
                    features.push(Feature {
                        name: name.to_string(),
                        hierarchy: Vec::new(),
                        counts: vec![0.0; columns.len()],
                        parent: 0,
                    });
                    index.insert(name.to_string(), features.len() - 1);
                    features.len() - 1
                });
                for (total, &col) in features[i].counts.iter_mut().zip(&columns) {
                    *total += leaf.counts[col];
                }
                feature_index = Some(i);
            }

            let parent_name = if parent_depth == 0 {
                ENTIRE_SAMPLE
            } else {
                self.node_name(leaf.path[parent_depth - 1])
            };
            let p = *parent_index.entry(parent_name).or_insert_with(|| {
                parents.push(vec![0.0; columns.len()]);
                parents.len() - 1
            });
            for (total, &col) in parents[p].iter_mut().zip(&columns) {
                *total += leaf.counts[col];
            }

            if let Some(i) = feature_index {
                features[i].parent = p;
                features[i].hierarchy = leaf.path[..profile_depth]
                    .iter()
                    .map(|&n| self.node_name(n).to_string())
                    .collect();
            }
        }

        Ok(Profile {
            hierarchy_headings: self.hierarchy_headings()[..profile_depth].to_vec(),
            sample_names: samples.iter().map(|s| s.to_string()).collect(),
            features,
            index,
            parents,
        })
    }

    /// Profile of two samples.
    pub fn sample_profile(
        &self,
        sample1: &str,
        sample2: &str,
        parent_level: Option<&str>,
        profile_level: &str,
        unclassified: Unclassified,
    ) -> Result<SampleProfile> {
        let profile = self.profile(
            &[sample1, sample2],
            parent_level,
            profile_level,
            unclassified,
        )?;
        Ok(SampleProfile { profile })
    }

    /// Profile of the active samples that have a number in a metadata
    /// field, with those numbers, for correlating features with the field.
    pub fn correlation_profile(
        &self,
        metadata: &Metadata,
        field: &str,
        parent_level: Option<&str>,
        profile_level: &str,
        unclassified: Unclassified,
    ) -> Result<CorrelationProfile> {
        if !metadata.fields().iter().any(|f| f == field) {
            return Err(StampError::new(format!(
                "Unknown metadata field '{field}'."
            )));
        }
        let mut numbered = metadata.numeric_values(field);
        numbered.sort_by(|a, b| a.0.cmp(b.0));
        let samples: Vec<&str> = numbered.iter().map(|(sample, _)| *sample).collect();
        let profile = self.profile(&samples, parent_level, profile_level, unclassified)?;
        Ok(CorrelationProfile {
            profile,
            field: field.to_string(),
            values: numbered.iter().map(|(_, value)| *value).collect(),
            without_value: metadata.active_samples().len() - numbered.len(),
        })
    }

    /// Profile of two groups of samples.
    pub fn group_profile(
        &self,
        groups: &Groups,
        group1: &str,
        group2: SecondGroup<'_>,
        parent_level: Option<&str>,
        profile_level: &str,
        unclassified: Unclassified,
    ) -> Result<GroupProfile> {
        let unknown = |g: &str| StampError::new(format!("Unknown group '{g}'."));
        let samples1 = groups
            .samples(group1)
            .ok_or_else(|| unknown(group1))?
            .to_vec();
        if group2 == SecondGroup::Named(group1) {
            return Err(StampError::new(format!(
                "Group '{group1}' cannot be compared with itself."
            )));
        }
        let (name2, samples2) = match group2 {
            SecondGroup::Named(g) => (
                g.to_string(),
                groups.samples(g).ok_or_else(|| unknown(g))?.to_vec(),
            ),
            SecondGroup::AllOtherSamples => (
                ALL_OTHER_SAMPLES.to_string(),
                groups.all_other_samples(group1),
            ),
        };

        let samples: Vec<&str> = samples1
            .iter()
            .chain(&samples2)
            .map(String::as_str)
            .collect();
        let profile = self.profile(&samples, parent_level, profile_level, unclassified)?;
        Ok(GroupProfile {
            profile,
            group_name1: group1.to_string(),
            group_name2: name2,
            size1: samples1.len(),
        })
    }

    /// Profile of several groups of samples. `active_groups` restricts the
    /// profile to the named groups, each of which must have samples (a
    /// group whose samples are all excluded would make every test
    /// degenerate); `None` uses every group, even one with no samples.
    pub fn multi_group_profile(
        &self,
        groups: &Groups,
        active_groups: Option<&[&str]>,
        parent_level: Option<&str>,
        profile_level: &str,
        unclassified: Unclassified,
    ) -> Result<MultiGroupProfile> {
        let mut group_names = Vec::new();
        let mut group_sizes = Vec::new();
        let mut samples: Vec<&str> = Vec::new();
        for name in groups.names() {
            if active_groups.is_some_and(|active| !active.contains(&name)) {
                continue;
            }
            let members = groups.samples(name).unwrap_or(&[]);
            if members.is_empty() && active_groups.is_some() {
                return Err(StampError::new(format!(
                    "Group '{name}' has no samples: they are all excluded or switched off."
                )));
            }
            group_names.push(name.to_string());
            group_sizes.push(members.len());
            samples.extend(members.iter().map(String::as_str));
        }
        if let Some(active) = active_groups {
            if let Some(missing) = active.iter().find(|a| !group_names.iter().any(|g| g == *a)) {
                return Err(StampError::new(format!("Unknown group '{missing}'.")));
            }
        }

        let profile = self.profile(&samples, parent_level, profile_level, unclassified)?;
        Ok(MultiGroupProfile {
            profile,
            group_names,
            group_sizes,
        })
    }
}

/// Name shown for the pseudo-group of all samples outside the first group.
pub const ALL_OTHER_SAMPLES: &str = "<All other samples>";

/// The second group of a two-group comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondGroup<'a> {
    Named(&'a str),
    /// Every sample that is not in the first group.
    AllOtherSamples,
}

/// A profile of two samples.
#[derive(Debug, Clone)]
pub struct SampleProfile {
    pub profile: Profile,
}

/// Counts of one feature in a two-sample profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleCounts {
    pub seq1: f64,
    pub seq2: f64,
    pub parent_seq1: f64,
    pub parent_seq2: f64,
}

impl SampleProfile {
    pub fn counts(&self, feature: &Feature) -> SampleCounts {
        let parents = self.profile.parent_counts(feature);
        SampleCounts {
            seq1: feature.counts[0],
            seq2: feature.counts[1],
            parent_seq1: parents[0],
            parent_seq2: parents[1],
        }
    }
}

impl SampleCounts {
    /// Relative frequencies (%) as reported in the results table. A parent
    /// count below one is treated as one.
    pub fn percentages(&self) -> (f64, f64) {
        (
            self.seq1 / self.parent_seq1.max(1.0) * 100.0,
            self.seq2 / self.parent_seq2.max(1.0) * 100.0,
        )
    }
}

/// A profile of samples that each have a number in a metadata field.
#[derive(Debug, Clone)]
pub struct CorrelationProfile {
    pub profile: Profile,
    /// The metadata field.
    pub field: String,
    /// The value of the field for each sample of the profile, in order.
    pub values: Vec<f64>,
    /// Active samples left out because the field holds no number for them.
    pub without_value: usize,
}

/// A profile of two groups of samples. The samples of the first group come
/// first in the profile's columns.
#[derive(Debug, Clone)]
pub struct GroupProfile {
    pub profile: Profile,
    pub group_name1: String,
    pub group_name2: String,
    size1: usize,
}

impl GroupProfile {
    pub fn samples_in_group1(&self) -> &[String] {
        &self.profile.sample_names[..self.size1]
    }

    pub fn samples_in_group2(&self) -> &[String] {
        &self.profile.sample_names[self.size1..]
    }

    /// Counts of a feature in the samples of each group.
    pub fn counts<'a>(&self, feature: &'a Feature) -> (&'a [f64], &'a [f64]) {
        feature.counts.split_at(self.size1)
    }

    /// Parent counts of a feature in the samples of each group.
    pub fn parent_counts(&self, feature: &Feature) -> (&[f64], &[f64]) {
        self.profile.parent_counts(feature).split_at(self.size1)
    }

    /// Values given for every sample of the profile, divided between the
    /// groups.
    pub fn split<'a, T>(&self, values: &'a [T]) -> (&'a [T], &'a [T]) {
        values.split_at(self.size1)
    }

    /// Relative frequencies (%) of a feature in the samples of each group.
    pub fn percentages(&self, feature: &Feature) -> (Vec<f64>, Vec<f64>) {
        let mut first = self.profile.percentages(feature);
        let second = first.split_off(self.size1);
        (first, second)
    }
}

/// A profile of several groups of samples. Columns are ordered by group.
#[derive(Debug, Clone)]
pub struct MultiGroupProfile {
    pub profile: Profile,
    /// Names of the groups, sorted.
    pub group_names: Vec<String>,
    group_sizes: Vec<usize>,
}

impl MultiGroupProfile {
    /// Samples of each group, in group order.
    pub fn samples_in_groups(&self) -> Vec<&[String]> {
        self.split(&self.profile.sample_names)
    }

    pub fn smallest_group(&self) -> usize {
        self.group_sizes.iter().copied().min().unwrap_or(0)
    }

    /// Counts of a feature in the samples of each group.
    pub fn counts<'a>(&self, feature: &'a Feature) -> Vec<&'a [f64]> {
        self.split(&feature.counts)
    }

    /// Parent counts of a feature in the samples of each group.
    pub fn parent_counts(&self, feature: &Feature) -> Vec<&[f64]> {
        self.split(self.profile.parent_counts(feature))
    }

    /// Relative frequencies (%) of a feature in the samples of each group.
    pub fn percentages(&self, feature: &Feature) -> Vec<Vec<f64>> {
        let all = self.profile.percentages(feature);
        self.split(&all).into_iter().map(<[f64]>::to_vec).collect()
    }

    /// Values given for every sample of the profile, divided between the
    /// groups.
    pub fn split<'a, T>(&self, values: &'a [T]) -> Vec<&'a [T]> {
        let mut out = Vec::with_capacity(self.group_sizes.len());
        let mut start = 0;
        for &size in &self.group_sizes {
            out.push(&values[start..start + size]);
            start += size;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::Metadata;

    const SPF: &str = "Phylum\tGenus\tS1\tS2\tS3\tS4\n\
        Firmicutes\tBacillus\t10\t20\t30\t40\n\
        Firmicutes\tunclassified\t5\t5\t5\t5\n\
        Proteobacteria\tEscherichia\t1\t2\t3\t4\n\
        Proteobacteria\tSalmonella\t4\t3\t2\t1\n";

    fn tree() -> ProfileTree {
        ProfileTree::read(SPF).unwrap()
    }

    fn names(profile: &Profile) -> Vec<&str> {
        profile.features().iter().map(|f| f.name.as_str()).collect()
    }

    #[test]
    fn leaf_level_against_entire_sample() {
        let p = tree()
            .sample_profile("S1", "S2", None, "Genus", Unclassified::Retain)
            .unwrap();
        assert_eq!(
            names(&p.profile),
            [
                "Bacillus",
                "Unclassified Firmicutes",
                "Escherichia",
                "Salmonella"
            ]
        );
        let bacillus = p.profile.feature("Bacillus").unwrap();
        assert_eq!(bacillus.hierarchy, ["Firmicutes", "Bacillus"]);
        let c = p.counts(bacillus);
        assert_eq!(
            (c.seq1, c.seq2, c.parent_seq1, c.parent_seq2),
            (10.0, 20.0, 20.0, 30.0)
        );
        assert_eq!(c.percentages().0, 50.0);
        assert_eq!(p.profile.num_parent_categories(), 1);
    }

    #[test]
    fn higher_level_sums_children() {
        let p = tree()
            .sample_profile("S3", "S4", None, "Phylum", Unclassified::Retain)
            .unwrap();
        assert_eq!(names(&p.profile), ["Firmicutes", "Proteobacteria"]);
        assert_eq!(
            p.profile.feature("Firmicutes").unwrap().counts,
            [35.0, 45.0]
        );
        assert_eq!(p.profile.hierarchy_headings, ["Phylum"]);
    }

    #[test]
    fn parent_level_sets_the_denominator() {
        let p = tree()
            .sample_profile("S1", "S2", Some("Phylum"), "Genus", Unclassified::Retain)
            .unwrap();
        let c = p.counts(p.profile.feature("Escherichia").unwrap());
        assert_eq!((c.parent_seq1, c.parent_seq2), (5.0, 5.0));
        assert_eq!(p.profile.num_parent_categories(), 2);
    }

    #[test]
    fn unclassified_treatments() {
        let t = tree();
        let removed = t
            .sample_profile("S1", "S2", None, "Genus", Unclassified::Remove)
            .unwrap();
        assert_eq!(
            names(&removed.profile),
            ["Bacillus", "Escherichia", "Salmonella"]
        );
        let bacillus = removed.profile.feature("Bacillus").unwrap();
        assert_eq!(removed.counts(bacillus).parent_seq1, 15.0);

        let freq = t
            .sample_profile("S1", "S2", None, "Genus", Unclassified::FrequencyOnly)
            .unwrap();
        assert_eq!(
            names(&freq.profile),
            ["Bacillus", "Escherichia", "Salmonella"]
        );
        let bacillus = freq.profile.feature("Bacillus").unwrap();
        assert_eq!(freq.counts(bacillus).parent_seq1, 20.0);
    }

    #[test]
    fn rejects_bad_levels_and_samples() {
        let t = tree();
        assert!(t
            .sample_profile("S1", "S2", Some("Genus"), "Phylum", Unclassified::Retain)
            .is_err());
        assert!(t
            .sample_profile("S1", "S2", Some("Genus"), "Genus", Unclassified::Retain)
            .is_err());
        assert!(t
            .sample_profile("S1", "S2", None, "Species", Unclassified::Retain)
            .is_err());
        assert!(t
            .sample_profile("S1", "S9", None, "Genus", Unclassified::Retain)
            .is_err());
    }

    #[test]
    fn disambiguated_leaf_names_keep_only_the_first() {
        let spf = "L1\tFunction\tA\tB\n\
            x\tKinase\t1\t2\n\
            y\tKinase - #1\t10\t20\n\
            y\tOther\t5\t5\n";
        let t = ProfileTree::read(spf).unwrap();
        let p = t
            .sample_profile("A", "B", None, "Function", Unclassified::Retain)
            .unwrap();
        assert_eq!(names(&p.profile), ["Kinase", "Other"]);
        let kinase = p.profile.feature("Kinase").unwrap();
        assert_eq!(kinase.counts, [1.0, 2.0]);
        // The dropped duplicate does not count towards the sample total either.
        assert_eq!(p.counts(kinase).parent_seq1, 6.0);

        // With a parent level the suffixed name is a feature in its own right.
        let p = t
            .sample_profile("A", "B", Some("L1"), "Function", Unclassified::Retain)
            .unwrap();
        assert_eq!(names(&p.profile), ["Kinase", "Kinase - #1", "Other"]);
    }

    const META: &str = "Id\tDiet\nS1\tChow\nS2\tHigh fat\nS3\tChow\nS4\tWestern\n";

    #[test]
    fn two_groups_and_all_other_samples() {
        let t = tree();
        let (meta, _) = Metadata::read(META, &t).unwrap();
        let groups = meta.groups("Diet").unwrap();

        let g = t
            .group_profile(
                &groups,
                "Chow",
                SecondGroup::Named("Western"),
                None,
                "Phylum",
                Unclassified::Retain,
            )
            .unwrap();
        assert_eq!(g.samples_in_group1(), ["S1", "S3"]);
        assert_eq!(g.samples_in_group2(), ["S4"]);
        let firmicutes = g.profile.feature("Firmicutes").unwrap();
        assert_eq!(g.counts(firmicutes), (&[15.0, 35.0][..], &[45.0][..]));
        assert_eq!(
            g.parent_counts(firmicutes),
            (&[20.0, 40.0][..], &[50.0][..])
        );
        assert_eq!(g.percentages(firmicutes), (vec![75.0, 87.5], vec![90.0]));

        let g = t
            .group_profile(
                &groups,
                "Chow",
                SecondGroup::AllOtherSamples,
                None,
                "Phylum",
                Unclassified::Retain,
            )
            .unwrap();
        assert_eq!(g.group_name2, ALL_OTHER_SAMPLES);
        assert_eq!(g.samples_in_group2(), ["S2", "S4"]);

        // A group is not compared with itself.
        let same = t.group_profile(
            &groups,
            "Chow",
            SecondGroup::Named("Chow"),
            None,
            "Phylum",
            Unclassified::Retain,
        );
        assert!(same.unwrap_err().to_string().contains("with itself"));
    }

    #[test]
    fn multiple_groups_and_active_subset() {
        let t = tree();
        let (meta, _) = Metadata::read(META, &t).unwrap();
        let groups = meta.groups("Diet").unwrap();

        let m = t
            .multi_group_profile(&groups, None, None, "Phylum", Unclassified::Retain)
            .unwrap();
        assert_eq!(m.group_names, ["Chow", "High fat", "Western"]);
        assert_eq!(m.smallest_group(), 1);
        let proteo = m.profile.feature("Proteobacteria").unwrap();
        assert_eq!(m.counts(proteo), [&[5.0, 5.0][..], &[5.0][..], &[5.0][..]]);
        assert_eq!(m.percentages(proteo)[0], [25.0, 12.5]);

        let m = t
            .multi_group_profile(
                &groups,
                Some(&["Western", "Chow"]),
                None,
                "Phylum",
                Unclassified::Retain,
            )
            .unwrap();
        assert_eq!(m.group_names, ["Chow", "Western"]);
        assert_eq!(m.samples_in_groups(), [&["S1", "S3"][..], &["S4"][..]]);
        assert!(t
            .multi_group_profile(
                &groups,
                Some(&["Nope"]),
                None,
                "Phylum",
                Unclassified::Retain
            )
            .is_err());

        // A chosen group whose samples are all switched off is refused.
        let (mut meta, _) = Metadata::read(META, &t).unwrap();
        meta.set_active("S4", false);
        let groups = meta.groups("Diet").unwrap();
        let empty = t.multi_group_profile(
            &groups,
            Some(&["Western", "Chow"]),
            None,
            "Phylum",
            Unclassified::Retain,
        );
        assert!(empty
            .unwrap_err()
            .to_string()
            .contains("'Western' has no samples"));
    }
}
