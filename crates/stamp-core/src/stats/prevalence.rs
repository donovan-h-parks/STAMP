//! Prevalence and mean relative abundance of each feature, within each
//! group of samples and over all of them: the two axes of an
//! occupancy-abundance (prevalence versus mean abundance) plot.
//!
//! A feature is present in a sample when its relative abundance is above
//! zero and at least a chosen minimum (as for the prevalence of the group
//! heatmap). Its **specificity** to a group is its prevalence in the group
//! where it is most prevalent minus its highest prevalence in any other
//! group, in percentage points: 0 for a feature found as often in another
//! group, 100 for one found in every sample of one group and in no sample
//! of any other.

use crate::profile::Profile;
use crate::stats::group_summary::is_present;

/// Which samples the mean abundance of a feature is taken over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MeanOver {
    /// Every sample of the group, counting a sample without the feature as
    /// zero.
    AllSamples,
    /// Only the samples where the feature is present.
    PresentSamples,
}

impl MeanOver {
    pub fn name(self) -> &'static str {
        match self {
            MeanOver::AllSamples => "All samples",
            MeanOver::PresentSamples => "Samples where present",
        }
    }
}

/// A feature in one group of samples (or in all of them).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Occupancy {
    /// Number of samples in the group.
    pub samples: usize,
    /// Number of those in which the feature is present.
    pub present: usize,
    /// Percentage of the samples in which the feature is present; NaN for a
    /// group without samples.
    pub prevalence: f64,
    /// Mean relative abundance (%) over all the group's samples; NaN for a
    /// group without samples.
    pub mean_all: f64,
    /// Mean relative abundance (%) over the samples where the feature is
    /// present; NaN where it is present in none.
    pub mean_present: f64,
}

impl Occupancy {
    /// The mean abundance over the chosen samples.
    pub fn mean(&self, over: MeanOver) -> f64 {
        match over {
            MeanOver::AllSamples => self.mean_all,
            MeanOver::PresentSamples => self.mean_present,
        }
    }

    fn of(values: &[f64], min_abundance: f64) -> Occupancy {
        let samples = values.len();
        let mut present = 0;
        let (mut total, mut total_present) = (0.0, 0.0);
        for &value in values {
            total += value;
            if is_present(value, min_abundance) {
                present += 1;
                total_present += value;
            }
        }
        let ratio = |sum: f64, n: usize| {
            if n == 0 {
                f64::NAN
            } else {
                sum / n as f64
            }
        };
        Occupancy {
            samples,
            present,
            prevalence: ratio(100.0 * present as f64, samples),
            mean_all: ratio(total, samples),
            mean_present: ratio(total_present, present),
        }
    }
}

/// A feature in each group and over all samples.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureOccupancy {
    pub feature: String,
    /// One per group, in the order of the groups.
    pub groups: Vec<Occupancy>,
    /// Over every sample, whatever its group.
    pub overall: Occupancy,
}

impl FeatureOccupancy {
    /// The group where the feature is most prevalent (ties go to the higher
    /// mean abundance, then to the earlier group); `None` if it is present
    /// in no group.
    pub fn home_group(&self) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (g, o) in self.groups.iter().enumerate() {
            if o.present == 0 {
                continue;
            }
            let better = match best {
                None => true,
                Some(b) => {
                    let current = &self.groups[b];
                    o.prevalence > current.prevalence
                        || (o.prevalence == current.prevalence && o.mean_all > current.mean_all)
                }
            };
            if better {
                best = Some(g);
            }
        }
        best
    }

    /// Prevalence in the home group minus the highest prevalence in any
    /// other group, in percentage points (see the module documentation);
    /// 0 for a feature present nowhere. With a single group, its
    /// prevalence there.
    pub fn specificity(&self) -> f64 {
        let Some(home) = self.home_group() else {
            return 0.0;
        };
        let others = self
            .groups
            .iter()
            .enumerate()
            .filter(|&(g, o)| g != home && o.samples > 0)
            .map(|(_, o)| o.prevalence)
            .fold(0.0, f64::max);
        self.groups[home].prevalence - others
    }
}

/// Prevalence and mean abundance of a feature whose relative abundance (%)
/// in sample `i` is `values[i]`, in each of `groups` groups
/// (`sample_groups[i]` is the group of sample `i`) and over all samples.
pub fn occupancy(
    values: &[f64],
    sample_groups: &[usize],
    groups: usize,
    min_abundance: f64,
) -> (Vec<Occupancy>, Occupancy) {
    let mut members: Vec<Vec<f64>> = vec![Vec::new(); groups];
    for (&value, &group) in values.iter().zip(sample_groups) {
        if let Some(list) = members.get_mut(group) {
            list.push(value);
        }
    }
    let per_group = members
        .iter()
        .map(|list| Occupancy::of(list, min_abundance))
        .collect();
    let all: Vec<f64> = values
        .iter()
        .zip(sample_groups)
        .filter(|(_, &g)| g < groups)
        .map(|(&v, _)| v)
        .collect();
    (per_group, Occupancy::of(&all, min_abundance))
}

/// [`occupancy`] of every feature of a profile (or of those named in
/// `features`), in the profile's order.
pub fn profile_occupancy(
    profile: &Profile,
    sample_groups: &[usize],
    groups: usize,
    min_abundance: f64,
    features: Option<&[String]>,
) -> Vec<FeatureOccupancy> {
    profile
        .features()
        .iter()
        .filter(|f| features.is_none_or(|names| names.contains(&f.name)))
        .map(|f| {
            let values = profile.percentages(f);
            let (groups, overall) = occupancy(&values, sample_groups, groups, min_abundance);
            FeatureOccupancy {
                feature: f.name.clone(),
                groups,
                overall,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two groups. Group 0: 0, 2, 4 (present in 2 of 3); group 1: 0, 0.5,
    // 0.05, 9 (present in 3 of 4).
    const VALUES: [f64; 7] = [0.0, 2.0, 4.0, 0.0, 0.5, 0.05, 9.0];
    const GROUPS: [usize; 7] = [0, 0, 0, 1, 1, 1, 1];

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn by_hand() {
        let (groups, all) = occupancy(&VALUES, &GROUPS, 2, 0.0);
        assert_eq!((groups[0].samples, groups[0].present), (3, 2));
        assert!(close(groups[0].prevalence, 200.0 / 3.0));
        assert!(close(groups[0].mean_all, 2.0));
        assert!(close(groups[0].mean_present, 3.0));
        assert_eq!((groups[1].samples, groups[1].present), (4, 3));
        assert!(close(groups[1].prevalence, 75.0));
        assert!(close(groups[1].mean_all, 9.55 / 4.0));
        assert!(close(groups[1].mean_present, 9.55 / 3.0));
        assert!(close(groups[1].mean(MeanOver::PresentSamples), 9.55 / 3.0));
        assert_eq!((all.samples, all.present), (7, 5));
        assert!(close(all.prevalence, 500.0 / 7.0));
        assert!(close(all.mean_all, 15.55 / 7.0));
        assert!(close(all.mean_present, 15.55 / 5.0));
    }

    #[test]
    fn a_minimum_abundance_for_presence() {
        // At 0.1% the 0.05 no longer counts as present, but still counts
        // towards the mean over all samples.
        let (groups, _) = occupancy(&VALUES, &GROUPS, 2, 0.1);
        assert_eq!(groups[1].present, 2);
        assert!(close(groups[1].prevalence, 50.0));
        assert!(close(groups[1].mean_all, 9.55 / 4.0));
        assert!(close(groups[1].mean_present, 9.5 / 2.0));
    }

    #[test]
    fn empty_groups_and_absent_features() {
        let (groups, all) = occupancy(&[0.0, 0.0], &[0, 0], 2, 0.0);
        assert!(groups[1].prevalence.is_nan() && groups[1].mean_all.is_nan());
        assert_eq!(groups[0].prevalence, 0.0);
        assert!(groups[0].mean_present.is_nan());
        assert_eq!(all.present, 0);
        let feature = FeatureOccupancy {
            feature: "x".into(),
            groups,
            overall: all,
        };
        assert_eq!(feature.home_group(), None);
        assert_eq!(feature.specificity(), 0.0);
    }

    #[test]
    fn home_group_and_specificity() {
        let (groups, overall) = occupancy(&VALUES, &GROUPS, 2, 0.0);
        let feature = FeatureOccupancy {
            feature: "x".into(),
            groups,
            overall,
        };
        // 75% in group 1 against 66.7% in group 0.
        assert_eq!(feature.home_group(), Some(1));
        assert!(close(feature.specificity(), 75.0 - 200.0 / 3.0));
        // A specialist: in every sample of one group and none of the other.
        let (groups, overall) = occupancy(&[1.0, 2.0, 0.0, 0.0], &[0, 0, 1, 1], 2, 0.0);
        let specialist = FeatureOccupancy {
            feature: "y".into(),
            groups,
            overall,
        };
        assert_eq!(specialist.home_group(), Some(0));
        assert_eq!(specialist.specificity(), 100.0);
        // Equal prevalence: the higher mean wins, and specificity is 0.
        let (groups, overall) = occupancy(&[1.0, 0.0, 5.0, 0.0], &[0, 0, 1, 1], 2, 0.0);
        let tie = FeatureOccupancy {
            feature: "z".into(),
            groups,
            overall,
        };
        assert_eq!(tie.home_group(), Some(1));
        assert_eq!(tie.specificity(), 0.0);
    }

    /// The profile of an analysis: samples left out by the sample filter
    /// count in no group, and proportions are of the whole sample.
    #[test]
    fn every_feature_of_a_profile() {
        use crate::metadata::Metadata;
        use crate::profile_tree::ProfileTree;
        use crate::Unclassified;
        let spf = "Genus\tS1\tS2\tS3\tS4\n\
            A\t1\t0\t3\t5\n\
            B\t0\t0\t1\t5\n\
            C\t9\t10\t0\t0\n";
        let meta = "Id\tHost\tSite\nS1\tHorse\tn\nS2\tHorse\tn\nS3\tTapir\tn\nS4\tTapir\ts\n";
        let tree = ProfileTree::read(spf).unwrap();
        let (mut metadata, _) = Metadata::read(meta, &tree).unwrap();
        let selection = crate::SampleSelection::parse(&[], &["Site=s".to_string()]).unwrap();
        metadata.set_selection(selection).unwrap();
        let groups = metadata.groups("Host").unwrap();
        let profile = tree
            .multi_group_profile(&groups, None, None, "Genus", Unclassified::Retain)
            .unwrap();
        let sample_groups: Vec<usize> = profile
            .samples_in_groups()
            .iter()
            .enumerate()
            .flat_map(|(g, m)| std::iter::repeat_n(g, m.len()))
            .collect();
        let all = profile_occupancy(&profile.profile, &sample_groups, 2, 0.0, None);
        assert_eq!(all.len(), 3);
        let a = &all[0];
        assert_eq!(a.feature, "A");
        // S1: 1 of 10 = 10%, S2: 0 (Horse); S3: 3 of 4 = 75% (Tapir); S4 is
        // filtered out.
        assert_eq!(a.groups[1].samples, 1);
        assert!(close(a.groups[0].prevalence, 50.0));
        assert!(close(a.groups[0].mean_all, 5.0));
        assert!(close(a.groups[0].mean_present, 10.0));
        assert!(close(a.groups[1].mean_all, 75.0));
        assert!(close(a.overall.mean_all, 85.0 / 3.0));
        let only = profile_occupancy(
            &profile.profile,
            &sample_groups,
            2,
            0.0,
            Some(&["C".to_string()]),
        );
        assert_eq!(only.len(), 1);
        assert_eq!(only[0].home_group(), Some(0));
        assert_eq!(only[0].specificity(), 100.0);
    }
}
