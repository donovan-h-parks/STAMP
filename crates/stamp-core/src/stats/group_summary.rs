//! A summary of each feature within each group of samples: its mean,
//! median or largest proportion, or the percentage of the group's samples
//! in which it is present. These are the cells of the group heatmap.

use crate::metadata::Metadata;

/// What is summarised for each feature in each group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GroupStatistic {
    /// Mean relative abundance (%) of the group's samples.
    Mean,
    /// Median relative abundance (%).
    Median,
    /// Largest relative abundance (%) in any of the group's samples.
    Maximum,
    /// Percentage of the group's samples in which the feature is present.
    Prevalence,
}

impl GroupStatistic {
    pub const ALL: [GroupStatistic; 4] = [
        GroupStatistic::Mean,
        GroupStatistic::Median,
        GroupStatistic::Maximum,
        GroupStatistic::Prevalence,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GroupStatistic::Mean => "Mean",
            GroupStatistic::Median => "Median",
            GroupStatistic::Maximum => "Maximum",
            GroupStatistic::Prevalence => "Prevalence",
        }
    }

    /// Label of the colour scale.
    pub fn label(self) -> &'static str {
        match self {
            GroupStatistic::Mean => "Mean proportion (%)",
            GroupStatistic::Median => "Median proportion (%)",
            GroupStatistic::Maximum => "Maximum proportion (%)",
            GroupStatistic::Prevalence => "Prevalence (% of samples)",
        }
    }
}

/// True if a feature with this relative abundance (%) counts as present:
/// above zero and at least `min_abundance`.
pub fn is_present(value: f64, min_abundance: f64) -> bool {
    value > 0.0 && value >= min_abundance
}

/// The statistic of one feature in each group. `values` holds the feature's
/// relative abundance (%) in each sample and `sample_groups` the group of
/// each sample, an index below `groups`. A group without samples gets NaN.
/// `min_abundance` is the smallest relative abundance (%) at which the
/// feature counts as present, for [`GroupStatistic::Prevalence`].
pub fn summarise(
    values: &[f64],
    sample_groups: &[usize],
    groups: usize,
    statistic: GroupStatistic,
    min_abundance: f64,
) -> Vec<f64> {
    let mut members: Vec<Vec<f64>> = vec![Vec::new(); groups];
    for (&value, &group) in values.iter().zip(sample_groups) {
        if let Some(list) = members.get_mut(group) {
            list.push(value);
        }
    }
    members
        .iter_mut()
        .map(|list| {
            if list.is_empty() {
                return f64::NAN;
            }
            let n = list.len() as f64;
            match statistic {
                GroupStatistic::Mean => list.iter().sum::<f64>() / n,
                GroupStatistic::Median => median(list),
                GroupStatistic::Maximum => list.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                GroupStatistic::Prevalence => {
                    let present = list
                        .iter()
                        .filter(|&&v| is_present(v, min_abundance))
                        .count();
                    100.0 * present as f64 / n
                }
            }
        })
        .collect()
}

/// Median of values (reordered in place); the mean of the middle two for an
/// even number.
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

/// The most common value of a metadata field among each group's samples,
/// used to order and label the groups by a coarser field (host species by
/// host order, say). Ties go to the value that sorts first; samples without
/// a value are not counted, and a group with none gets an empty string.
pub fn majority_values(metadata: &Metadata, members: &[&[String]], field: &str) -> Vec<String> {
    members
        .iter()
        .map(|samples| {
            let mut counts: Vec<(&str, usize)> = Vec::new();
            for sample in samples.iter() {
                let Some(value) = metadata.value(sample, field) else {
                    continue;
                };
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }
                match counts.iter_mut().find(|(v, _)| *v == value) {
                    Some((_, count)) => *count += 1,
                    None => counts.push((value, 1)),
                }
            }
            counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            counts
                .first()
                .map(|(v, _)| v.to_string())
                .unwrap_or_default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_tree::ProfileTree;
    use crate::Unclassified;

    // Two groups; group 1 has an even number of samples.
    const VALUES: [f64; 7] = [0.0, 2.0, 4.0, 0.0, 1.0, 0.05, 9.0];
    const GROUPS: [usize; 7] = [0, 0, 0, 1, 1, 1, 1];

    #[test]
    fn each_statistic_by_hand() {
        let s = |statistic, min| summarise(&VALUES, &GROUPS, 2, statistic, min);
        assert_eq!(s(GroupStatistic::Mean, 0.0), [2.0, 10.05 / 4.0]);
        // Group 1 sorted: 0, 0.05, 1, 9 -> (0.05 + 1) / 2.
        assert_eq!(s(GroupStatistic::Median, 0.0), [2.0, 0.525]);
        assert_eq!(s(GroupStatistic::Maximum, 0.0), [4.0, 9.0]);
        assert_eq!(s(GroupStatistic::Prevalence, 0.0), [200.0 / 3.0, 75.0]);
        // With a minimum abundance of 0.1%, 0.05% is not present.
        assert_eq!(s(GroupStatistic::Prevalence, 0.1), [200.0 / 3.0, 50.0]);
        // A threshold of exactly a value counts it as present.
        assert_eq!(s(GroupStatistic::Prevalence, 2.0), [200.0 / 3.0, 25.0]);
    }

    #[test]
    fn empty_groups_and_all_zero_features() {
        let s = summarise(&VALUES, &GROUPS, 3, GroupStatistic::Mean, 0.0);
        assert!(s[2].is_nan());
        let zeros = summarise(&[0.0; 4], &[0, 0, 1, 1], 2, GroupStatistic::Maximum, 0.0);
        assert_eq!(zeros, [0.0, 0.0]);
        let zeros = summarise(&[0.0; 4], &[0, 0, 1, 1], 2, GroupStatistic::Prevalence, 0.0);
        assert_eq!(zeros, [0.0, 0.0]);
    }

    /// Samples switched off or filtered out are not part of the profile, so
    /// they count neither in the denominator of prevalence nor in the median
    /// or maximum.
    #[test]
    fn excluded_samples_do_not_count() {
        let spf = "F\ta1\ta2\ta3\ta4\tb1\tb2\tc1\n\
            x\t0\t10\t50\t0\t5\t0\t1\n\
            y\t10\t10\t50\t10\t5\t10\t1\n";
        let meta = "Id\tGroup\tSite\n\
            a1\tA\tn\na2\tA\tn\na3\tA\tn\na4\tA\ts\nb1\tB\tn\nb2\tB\tn\nc1\tC\tn\n";
        let tree = ProfileTree::read(spf).unwrap();
        let (mut metadata, _) = Metadata::read(meta, &tree).unwrap();
        // a3, where x is at its largest, is switched off; a4 (x absent) is
        // left out by the sample filter.
        metadata.set_active("a3", false);
        // Every sample of group C is switched off: the group stays, empty.
        metadata.set_active("c1", false);
        let selection = crate::SampleSelection::parse(&[], &["Site=s".to_string()]).unwrap();
        metadata.set_selection(selection).unwrap();
        let groups = metadata.groups("Group").unwrap();
        let profile = tree
            .multi_group_profile(&groups, None, None, "F", Unclassified::Retain)
            .unwrap();
        assert_eq!(profile.profile.sample_names, ["a1", "a2", "b1", "b2"]);
        let sample_groups: Vec<usize> = profile
            .samples_in_groups()
            .iter()
            .enumerate()
            .flat_map(|(g, m)| std::iter::repeat_n(g, m.len()))
            .collect();
        let x = profile.profile.feature("x").unwrap();
        let values = profile.profile.percentages(x);
        assert_eq!(profile.group_names, ["A", "B", "C"]);
        // a1: 0%, a2: 50%; b1: 50%, b2: 0%; C has no samples.
        let s = |statistic| summarise(&values, &sample_groups, 3, statistic, 0.0);
        for (statistic, expected) in [
            (GroupStatistic::Prevalence, 50.0),
            (GroupStatistic::Maximum, 50.0),
            (GroupStatistic::Median, 25.0),
            (GroupStatistic::Mean, 25.0),
        ] {
            let got = s(statistic);
            assert_eq!(&got[..2], [expected, expected], "{statistic:?}");
            assert!(got[2].is_nan());
        }
    }

    #[test]
    fn majority_value_of_a_field() {
        let spf = "F\ts1\ts2\ts3\ts4\ts5\n x\t1\t1\t1\t1\t1\n";
        let meta = "Id\tSpecies\tOrder\n\
            s1\tHorse\tPerissodactyla\ns2\tHorse\tPerissodactyla\ns3\tHorse\tArtiodactyla\n\
            s4\tCow\tArtiodactyla\ns5\tCow\t\n";
        let tree = ProfileTree::read(&spf.replace("\n x", "\nx")).unwrap();
        let (metadata, _) = Metadata::read(meta, &tree).unwrap();
        let horse = ["s1".to_string(), "s2".to_string(), "s3".to_string()];
        let cow = ["s4".to_string(), "s5".to_string()];
        let tie = ["s3".to_string(), "s1".to_string()];
        let none = ["s5".to_string()];
        assert_eq!(
            majority_values(&metadata, &[&horse, &cow, &tie, &none], "Order"),
            ["Perissodactyla", "Artiodactyla", "Artiodactyla", ""]
        );
    }
}
