//! The "Select features" window: choosing by hand which features take part
//! in the plots and tables.
//!
//! Features are listed at any hierarchical level. At a level above the
//! profile level each entry stands for all the features beneath it, with
//! their number in brackets, and is moved as a whole.

use std::collections::{BTreeMap, HashMap, HashSet};

use eframe::egui;
use stamp_core::profile::Profile;

use crate::app::choose;

/// The outcome of a frame of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Open,
    Accepted,
    Cancelled,
}

/// A division of the features of a profile into selected and filtered, as
/// edited in the window.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureSelection {
    headings: Vec<String>,
    /// Names from the top level down to the feature itself, by feature.
    hierarchy: HashMap<String, Vec<String>>,
    selected: HashSet<String>,
    /// The level at which features are listed.
    level: usize,
    /// Entries the user has marked in each list, to be moved.
    marked_selected: HashSet<String>,
    marked_filtered: HashSet<String>,
}

impl FeatureSelection {
    /// Start from the features of `profile`. `selected` holds the current
    /// choice; `None` selects every feature.
    pub fn new(profile: &Profile, selected: Option<&HashSet<String>>) -> FeatureSelection {
        let hierarchy: HashMap<String, Vec<String>> = profile
            .features()
            .iter()
            .map(|f| (f.name.clone(), f.hierarchy.clone()))
            .collect();
        let selected = hierarchy
            .keys()
            .filter(|name| selected.is_none_or(|chosen| chosen.contains(*name)))
            .cloned()
            .collect();
        FeatureSelection {
            level: profile.hierarchy_headings.len().saturating_sub(1),
            headings: profile.hierarchy_headings.clone(),
            hierarchy,
            selected,
            marked_selected: HashSet::new(),
            marked_filtered: HashSet::new(),
        }
    }

    /// The selected features.
    pub fn selected(&self) -> &HashSet<String> {
        &self.selected
    }

    /// True if no feature has been filtered out.
    pub fn all_selected(&self) -> bool {
        self.selected.len() == self.hierarchy.len()
    }

    /// List features at this hierarchical level, counted from the top.
    pub fn set_level(&mut self, level: usize) {
        self.level = level.min(self.headings.len().saturating_sub(1));
        self.marked_selected.clear();
        self.marked_filtered.clear();
    }

    fn name_at_level<'a>(&'a self, feature: &'a str) -> &'a str {
        self.hierarchy
            .get(feature)
            .and_then(|names| names.get(self.level))
            .map_or(feature, String::as_str)
    }

    /// The entries of one list at the current level, in alphabetical order,
    /// each with the number of features it stands for.
    pub fn entries(&self, selected: bool) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<(String, &str), usize> = BTreeMap::new();
        for feature in self.hierarchy.keys() {
            if self.selected.contains(feature) == selected {
                let name = self.name_at_level(feature);
                *counts.entry((name.to_lowercase(), name)).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .map(|((_, name), count)| (name.to_string(), count))
            .collect()
    }

    /// Move every feature under the named entries of the current level into
    /// the selected list, or out of it.
    pub fn move_entries(&mut self, names: &HashSet<String>, select: bool) {
        let features: Vec<String> = self
            .hierarchy
            .keys()
            .filter(|feature| names.contains(self.name_at_level(feature)))
            .cloned()
            .collect();
        for feature in features {
            if select {
                self.selected.insert(feature);
            } else {
                self.selected.remove(&feature);
            }
        }
    }

    /// One of the two lists; clicking an entry marks or unmarks it.
    fn list(&mut self, ui: &mut egui::Ui, selected: bool) {
        let entries = self.entries(selected);
        let lowest = self.level + 1 == self.headings.len();
        let marked = if selected {
            &mut self.marked_selected
        } else {
            &mut self.marked_filtered
        };
        // Marks on entries that have since been moved are dropped.
        marked.retain(|name| entries.iter().any(|(entry, _)| entry == name));
        egui::ScrollArea::vertical()
            .id_salt(selected)
            .max_height(300.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (name, count) in &entries {
                    let text = if lowest {
                        name.clone()
                    } else {
                        format!("{name} [{count}]")
                    };
                    let label = egui::Button::selectable(marked.contains(name), text);
                    if ui.add(label).clicked() && !marked.remove(name) {
                        marked.insert(name.clone());
                    }
                }
            });
    }

    /// Draw the contents of the window.
    pub fn show(&mut self, ui: &mut egui::Ui) -> Outcome {
        ui.horizontal(|ui| {
            ui.label("Hierarchical level:");
            let levels: Vec<(usize, String)> = self.headings.iter().cloned().enumerate().collect();
            let mut level = self.level;
            if choose(ui, "select-features-level", &mut level, &levels) {
                self.set_level(level);
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            let column = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(egui::vec2(230.0, 330.0), column, |ui| {
                ui.set_width(230.0);
                ui.strong("Selected features:");
                self.list(ui, true);
            });
            ui.vertical(|ui| {
                ui.add_space(120.0);
                let remove = ui
                    .add_enabled(!self.marked_selected.is_empty(), egui::Button::new(">"))
                    .on_hover_text("Filter out the marked features");
                if remove.clicked() {
                    let names = std::mem::take(&mut self.marked_selected);
                    self.move_entries(&names, false);
                }
                let add = ui
                    .add_enabled(!self.marked_filtered.is_empty(), egui::Button::new("<"))
                    .on_hover_text("Select the marked features");
                if add.clicked() {
                    let names = std::mem::take(&mut self.marked_filtered);
                    self.move_entries(&names, true);
                }
            });
            let column = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(egui::vec2(230.0, 330.0), column, |ui| {
                ui.set_width(230.0);
                ui.strong("Filtered features:");
                self.list(ui, false);
            });
        });
        ui.separator();

        let mut outcome = Outcome::Open;
        ui.horizontal(|ui| {
            ui.label("Number of selected features:");
            ui.strong(self.selected.len().to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    outcome = Outcome::Cancelled;
                }
                if ui.button("OK").clicked() {
                    outcome = Outcome::Accepted;
                }
            });
        });
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stamp_core::{ProfileTree, Unclassified};

    fn profile() -> Profile {
        let spf = "Phylum\tGenus\tS1\tS2\n\
            Firmicutes\tBacillus\t1\t2\n\
            Firmicutes\tClostridium\t3\t4\n\
            Firmicutes\tunclassified\t3\t4\n\
            Proteobacteria\tEscherichia\t5\t6\n\
            unclassified\tunclassified\t7\t8\n";
        ProfileTree::read(spf)
            .unwrap()
            .sample_profile("S1", "S2", None, "Genus", Unclassified::Retain)
            .unwrap()
            .profile
    }

    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn names(entries: Vec<(String, usize)>) -> Vec<String> {
        entries.into_iter().map(|(name, _)| name).collect()
    }

    #[test]
    fn starts_with_the_current_choice() {
        let profile = profile();
        let all = FeatureSelection::new(&profile, None);
        assert!(all.all_selected());
        assert_eq!(all.entries(true).len(), profile.num_features());
        assert!(all.entries(false).is_empty());

        let some = FeatureSelection::new(&profile, Some(&set(&["Bacillus", "gone"])));
        assert_eq!(names(some.entries(true)), ["Bacillus"]);
        assert_eq!(some.entries(false).len(), profile.num_features() - 1);
        assert!(!some.all_selected());
    }

    #[test]
    fn entries_move_between_the_lists() {
        let mut selection = FeatureSelection::new(&profile(), None);
        selection.move_entries(&set(&["Clostridium", "Escherichia"]), false);
        assert_eq!(
            names(selection.entries(false)),
            ["Clostridium", "Escherichia"]
        );
        assert!(!selection.selected().contains("Clostridium"));
        selection.move_entries(&set(&["Escherichia"]), true);
        assert_eq!(names(selection.entries(false)), ["Clostridium"]);
    }

    #[test]
    fn a_higher_level_moves_everything_beneath_it() {
        let profile = profile();
        let total = profile.num_features();
        let mut selection = FeatureSelection::new(&profile, None);
        selection.set_level(0);
        let entries = selection.entries(true);
        let firmicutes = entries.iter().find(|(n, _)| n == "Firmicutes").unwrap();
        assert_eq!(firmicutes.1, 3);

        selection.move_entries(&set(&["Firmicutes"]), false);
        assert_eq!(selection.selected().len(), total - 3);
        assert_eq!(selection.entries(false), [("Firmicutes".to_string(), 3)]);

        // Back at the profile level the features are listed one by one.
        selection.set_level(1);
        assert_eq!(selection.entries(false).len(), 3);
        selection.move_entries(&set(&["Bacillus"]), true);
        assert_eq!(selection.selected().len(), total - 2);
    }
}
