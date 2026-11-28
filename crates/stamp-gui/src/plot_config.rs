//! The "Configure plot" window: the settings of the plot being shown.
//!
//! Changes apply as they are made, so their effect can be seen at once.

use eframe::egui;
use stamp_core::stats::cluster::Linkage;
use stamp_core::stats::co_occurrence::FeatureRanking;
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::diversity::{AlphaMetric, Distance, Normalisation};
use stamp_core::stats::group_summary::GroupStatistic;
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::prevalence::MeanOver;
use stamp_plot::axes::LegendPosition;
use stamp_plot::co_occurrence_plot::View;
use stamp_plot::community_plots::OrdinationMethod;
use stamp_plot::composition;
use stamp_plot::extended_error_bar::{self, SortBy};
use stamp_plot::feature_plots::{BoxLayout, BoxStyle, Field, GroupOrder as BoxGroupOrder};
use stamp_plot::group_heatmap::{ColumnOrder as GroupOrder, NumberFormat, Scale};
use stamp_plot::heatmap::{ColorMap, Order};
use stamp_plot::histogram::Inset;
use stamp_plot::ordination_plot;
use stamp_plot::prevalence_plot::View as PrevalenceView;
use stamp_plot::rank_abundance;
use stamp_plot::rarefaction;
use stamp_plot::sample_plots::BarField;
use stamp_plot::scatter::Spread;
use stamp_plot::shared_features;

use crate::app::choose;
use crate::plots::{BoxFeatures, PlotKind, PlotSettings};
use crate::session::Mode;

/// The ways a box plot shows the samples of a group.
const BOX_STYLES: [(BoxStyle, &str); 3] = [
    (BoxStyle::Box, "Box"),
    (BoxStyle::Violin, "Violin"),
    (BoxStyle::Swarm, "Swarm"),
];

/// The choices of "Labels for:": what the names beneath bars or columns
/// name.
fn label_choices() -> Vec<(composition::Labels, &'static str)> {
    composition::Labels::ALL
        .iter()
        .map(|&labels| (labels, labels.name()))
        .collect()
}

/// The rows of a settings grid. Each method adds one labelled row and
/// records whether its value changed.
struct Rows<'a> {
    ui: &'a mut egui::Ui,
    changed: bool,
    /// Tells apart controls with the same label in different groups.
    salt: &'a str,
    /// Controls added while this is set are switched off.
    disabled: bool,
}

impl Rows<'_> {
    fn number(
        &mut self,
        label: &str,
        value: &mut f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        self.text(label);
        let drag = egui::DragValue::new(value).speed(speed).range(range);
        self.changed |= self.put(drag);
        self.ui.end_row();
    }

    /// A length in inches.
    fn inches(&mut self, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>) {
        self.text(label);
        let drag = egui::DragValue::new(value)
            .speed(0.05)
            .range(range)
            .suffix(" in");
        self.changed |= self.put(drag);
        self.ui.end_row();
    }

    /// A font size in points.
    fn font(&mut self, label: &str, value: &mut f64) {
        self.text(label);
        let drag = egui::DragValue::new(value)
            .speed(0.25)
            .range(4.0..=36.0)
            .suffix(" pt");
        self.changed |= self.put(drag);
        self.ui.end_row();
    }

    fn size(&mut self, width: &mut f64, height: &mut f64) {
        self.inches("Width:", width, 2.0..=40.0);
        self.inches("Height:", height, 2.0..=40.0);
    }

    fn count(&mut self, label: &str, value: &mut usize, range: std::ops::RangeInclusive<usize>) {
        self.text(label);
        self.changed |= self.put(egui::DragValue::new(value).range(range));
        self.ui.end_row();
    }

    /// A label, faded if the controls are switched off.
    fn text(&mut self, label: &str) {
        self.ui.add_enabled(!self.disabled, egui::Label::new(label));
    }

    /// A control, switched off if the controls are. Returns true if its
    /// value changed.
    fn put(&mut self, widget: impl egui::Widget) -> bool {
        self.ui.add_enabled(!self.disabled, widget).changed()
    }

    /// Run `add` with the controls it adds switched off unless `enabled`.
    /// An option that depends on another stays in view this way.
    fn when(&mut self, enabled: bool, add: impl FnOnce(&mut Rows)) {
        let before = self.disabled;
        self.disabled |= !enabled;
        add(self);
        self.disabled = before;
    }

    fn number_if(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        self.when(enabled, |rows| rows.number(label, value, speed, range));
    }

    fn inches_if(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        self.when(enabled, |rows| rows.inches(label, value, range));
    }

    fn count_if(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut usize,
        range: std::ops::RangeInclusive<usize>,
    ) {
        self.when(enabled, |rows| rows.count(label, value, range));
    }

    fn font_if(&mut self, enabled: bool, label: &str, value: &mut f64) {
        self.when(enabled, |rows| rows.font(label, value));
    }

    fn check_if(&mut self, enabled: bool, label: &str, value: &mut bool) {
        self.when(enabled, |rows| rows.check(label, value));
    }

    fn choice_if<T: PartialEq + Clone>(
        &mut self,
        enabled: bool,
        label: &str,
        value: &mut T,
        options: &[(T, &str)],
    ) {
        self.when(enabled, |rows| rows.choice(label, value, options));
    }

    /// A line of text, such as a name.
    fn line(&mut self, label: &str, value: &mut String) {
        self.text(label);
        let edit = egui::TextEdit::singleline(value).desired_width(170.0);
        self.changed |= self.put(edit);
        self.ui.end_row();
    }

    /// Lines of text, such as names one per line.
    fn lines(&mut self, label: &str, value: &mut String) {
        self.text(label);
        let edit = egui::TextEdit::multiline(value)
            .desired_rows(3)
            .desired_width(170.0);
        self.changed |= self.put(edit);
        self.ui.end_row();
    }

    fn check(&mut self, label: &str, value: &mut bool) {
        self.changed |= self.put(egui::Checkbox::new(value, label));
        self.ui.end_row();
    }

    fn choice<T: PartialEq + Clone>(&mut self, label: &str, value: &mut T, options: &[(T, &str)]) {
        self.text(label);
        let options: Vec<(T, String)> = options
            .iter()
            .map(|(option, name)| (option.clone(), name.to_string()))
            .collect();
        let id = format!("config-{}-{label}", self.salt);
        let mut changed = false;
        self.ui.add_enabled_ui(!self.disabled, |ui| {
            changed = choose(ui, &id, value, &options);
        });
        self.changed |= changed;
        self.ui.end_row();
    }

    /// A value that is either chosen automatically or set by hand.
    fn optional(
        &mut self,
        label: &str,
        value: &mut Option<f64>,
        default: f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
    ) {
        let mut set = value.is_some();
        if self.ui.checkbox(&mut set, label).changed() {
            *value = set.then_some(default);
            self.changed = true;
        }
        match value {
            Some(number) => {
                let drag = egui::DragValue::new(number).speed(speed).range(range);
                self.changed |= self.ui.add(drag).changed();
            }
            None => {
                self.ui.weak("automatic");
            }
        }
        self.ui.end_row();
    }

    /// Lower and upper limits of an axis, or automatic limits.
    fn limits(&mut self, label: &str, value: &mut Option<(f64, f64)>, default: (f64, f64)) {
        let mut set = value.is_some();
        if self.ui.checkbox(&mut set, label).changed() {
            *value = set.then_some(default);
            self.changed = true;
        }
        match value {
            Some((low, high)) => {
                let mut changed = false;
                self.ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(low).speed(0.1)).changed();
                    ui.label("to");
                    changed |= ui.add(egui::DragValue::new(high).speed(0.1)).changed();
                });
                if *high <= *low {
                    *high = *low + 1.0;
                }
                self.changed |= changed;
            }
            None => {
                self.ui.weak("automatic");
            }
        }
        self.ui.end_row();
    }

    /// Shortening of long feature names.
    fn truncate(&mut self, value: &mut Option<usize>) {
        let mut set = value.is_some();
        if self
            .ui
            .checkbox(&mut set, "Truncate feature names to:")
            .changed()
        {
            *value = set.then_some(50);
            self.changed = true;
        }
        match value {
            Some(length) => {
                let drag = egui::DragValue::new(length)
                    .range(5..=500)
                    .suffix(" characters");
                self.changed |= self.ui.add(drag).changed();
            }
            None => {
                self.ui.weak("not truncated");
            }
        }
        self.ui.end_row();
    }

    fn legend(&mut self, value: &mut LegendPosition) {
        let positions: Vec<(LegendPosition, &str)> = LegendPosition::ALL
            .iter()
            .map(|&position| (position, position.name()))
            .collect();
        self.choice("Position:", value, &positions);
    }

    /// How samples are made comparable before their diversity is measured.
    fn normalisation(&mut self, value: &mut Normalisation) {
        let choices: Vec<(Normalisation, &str)> =
            Normalisation::ALL.iter().map(|&n| (n, n.name())).collect();
        self.choice("Samples:", value, &choices);
    }

    /// The distance between samples and how the samples are made comparable.
    fn community(&mut self, distance: &mut Distance, normalisation: &mut Normalisation) {
        let distances: Vec<(Distance, &str)> =
            Distance::ALL.iter().map(|&d| (d, d.name())).collect();
        self.choice("Distance:", distance, &distances);
        self.normalisation(normalisation);
    }

    fn field(&mut self, value: &mut Field) {
        self.choice(
            "Field to plot:",
            value,
            &[
                (Field::Proportion, "Proportions (%)"),
                (Field::SequenceCount, "Raw values"),
            ],
        );
    }

    fn order(&mut self, label: &str, value: &mut Order) {
        self.choice(
            label,
            value,
            &[
                (Order::Cluster(Linkage::Average), "Cluster: average (UPGMA)"),
                (
                    Order::Cluster(Linkage::Nearest),
                    "Cluster: nearest neighbour",
                ),
                (
                    Order::Cluster(Linkage::Furthest),
                    "Cluster: furthest neighbour",
                ),
                (Order::Cluster(Linkage::Centroid), "Cluster: centroid"),
                (Order::Cluster(Linkage::Ward), "Cluster: Ward"),
                (Order::Alphabetical, "Alphabetical"),
                (Order::MeanAbundance, "Mean abundance"),
                (Order::Original, "As in the profile"),
            ],
        );
    }
}

/// The settings window of one plot: rows of options, some gathered in
/// titled frames as STAMP v2 groups them.
struct Dialog<'a> {
    ui: &'a mut egui::Ui,
    changed: bool,
}

impl Dialog<'_> {
    fn grid(&mut self, ui_id: &str, framed: bool, add: impl FnOnce(&mut Rows)) {
        let changed = &mut self.changed;
        let contents = |ui: &mut egui::Ui| {
            if framed {
                ui.strong(ui_id);
            }
            egui::Grid::new(format!("plot-config-{ui_id}"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    let mut rows = Rows {
                        ui,
                        changed: false,
                        salt: ui_id,
                        disabled: false,
                    };
                    add(&mut rows);
                    *changed |= rows.changed;
                });
        };
        if framed {
            egui::Frame::group(self.ui.style()).show(self.ui, |ui| {
                ui.set_width(GROUP_WIDTH);
                contents(ui);
            });
        } else {
            contents(self.ui);
        }
    }

    /// Related options in a frame with a title.
    fn group(&mut self, title: &str, add: impl FnOnce(&mut Rows)) {
        self.grid(title, true, add);
    }

    fn legend(&mut self, value: &mut LegendPosition) {
        self.group("Legend position", |rows| rows.legend(value));
    }

    /// The settings of a plot of samples on the axes of an ordination by
    /// `method`. `extra` adds the groups of the data and tests, after the
    /// plots to show.
    fn ordination(
        &mut self,
        o: &mut ordination_plot::Options,
        method: OrdinationMethod,
        context: &Context,
        extra: impl FnOnce(&mut Dialog),
    ) {
        // NMDS has two axes; the axes of db-RDA and CCA are named by what
        // they are (constrained or not), so only by number here.
        let axis = match method {
            OrdinationMethod::Pcoa => "PCo",
            OrdinationMethod::Nmds => "NMDS",
            OrdinationMethod::DbRda | OrdinationMethod::Cca => "Axis ",
        };
        let panels = method != OrdinationMethod::Nmds;
        let constrained = method.constrained().is_some();
        self.group("Figure size", |rows| {
            rows.check(
                "Fixed pixels per unit distance (recommended)",
                &mut o.fixed_scale,
            );
            rows.inches("Width:", &mut o.width, 2.0..=40.0);
            // The height follows from the width while the scale is fixed.
            rows.inches_if(!o.fixed_scale, "Height:", &mut o.height, 2.0..=40.0);
        });
        self.group("Plots to show", |rows| {
            // The main panel is always drawn.
            rows.check_if(false, &format!("{axis}1 vs. {axis}2"), &mut true);
            rows.check_if(
                panels,
                &format!("{axis}3 vs. {axis}2"),
                &mut o.show_pc3_vs_pc2,
            );
            rows.check_if(
                panels,
                &format!("{axis}1 vs. {axis}3"),
                &mut o.show_pc1_vs_pc3,
            );
            rows.check_if(
                constrained,
                "Explanatory variables (arrows and centroids)",
                &mut o.show_variables,
            );
        });
        extra(self);
        self.colours_and_shapes(o, context);
        self.legend(&mut o.legend);
        self.group(FONT_SIZES, |rows| {
            rows.font("Notes above the plot:", &mut o.note_font_size);
            rows.font_if(
                constrained && o.show_variables,
                "Explanatory variables:",
                &mut o.variable_font_size,
            );
        });
        self.group(OTHER_OPTIONS, |rows| {
            rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
            rows.number("Opacity:", &mut o.opacity, 0.01, 0.05..=1.0);
            rows.check("Assign unique shapes", &mut o.unique_shapes);
            rows.check_if(
                panels && o.show_pc3_vs_pc2,
                &format!("Rotate {axis}3 labels"),
                &mut o.rotate_pc3_labels,
            );
        });
    }

    /// The explanatory variables of db-RDA and CCA: a tick box for each
    /// metadata field. None ticked stands for the field that defines the
    /// groups.
    fn explanatory_variables(&mut self, chosen: &mut Vec<String>, context: &Context) {
        let changed = &mut self.changed;
        egui::Frame::group(self.ui.style()).show(self.ui, |ui| {
            ui.set_width(GROUP_WIDTH);
            ui.strong("Explanatory variables");
            if context.fields.is_empty() {
                ui.weak("Load metadata to choose its fields.");
                return;
            }
            let default = context.group_field.as_deref();
            for field in &context.fields {
                let mut on = if chosen.is_empty() {
                    Some(field.as_str()) == default
                } else {
                    chosen.contains(field)
                };
                let numeric = context.numeric_fields.contains(field);
                let label = if numeric {
                    format!("{field} (number)")
                } else {
                    field.clone()
                };
                if ui.checkbox(&mut on, label).changed() {
                    // Ticking or clearing one field starts from what is
                    // shown, the group field included.
                    if chosen.is_empty() {
                        chosen.extend(default.map(str::to_string));
                    }
                    if on {
                        if !chosen.contains(field) {
                            chosen.push(field.clone());
                        }
                    } else {
                        chosen.retain(|f| f != field);
                    }
                    // Kept in the order of the metadata.
                    chosen.sort_by_key(|f| context.fields.iter().position(|g| g == f));
                    *changed = true;
                }
            }
            if chosen.is_empty() {
                ui.weak("None ticked: the field that defines the groups.");
            }
        });
    }

    /// What colours and shapes the samples of an ordination plot: the
    /// groups, a metadata field (numeric fields marked), or the abundance
    /// of a feature; and a field for the shapes.
    fn colours_and_shapes(&mut self, o: &mut ordination_plot::Options, context: &Context) {
        self.styles(
            Styles {
                color_by: &mut o.color_by,
                shape_by: &mut o.shape_by,
                color_map: &mut o.color_map,
                color_scale: &mut o.color_scale,
                draw_order: &mut o.draw_order,
            },
            "Groups",
            context,
            |_, _| {},
        );
    }

    /// The "Colours and shapes" group of the ordination plots and the map.
    /// `no_shape` names the shapes without a field; `extra` adds rows
    /// after the colour scale, told whether the colours are a scale.
    fn styles(
        &mut self,
        o: Styles,
        no_shape: &str,
        context: &Context,
        extra: impl FnOnce(&mut Rows, bool),
    ) {
        use stamp_plot::ordination_style::{ColorScale, ColorSource, DrawOrder};
        self.group("Colours and shapes", |rows| {
            let numeric = |field: &str| context.numeric_fields.iter().any(|f| f == field);
            let typed = match &*o.color_by {
                ColorSource::Feature(name) => name.clone(),
                _ => String::new(),
            };
            // A field drawn by category is chosen as the field, with the
            // box below ticked.
            let mut as_categories = matches!(*o.color_by, ColorSource::Category(_));
            let mut shown = match &*o.color_by {
                ColorSource::Category(field) => ColorSource::Field(field.clone()),
                other => other.clone(),
            };
            let mut options: Vec<(ColorSource, String)> = vec![
                (ColorSource::Group, "Groups".to_string()),
                (ColorSource::SelectedFeature, "Selected feature".to_string()),
                (
                    ColorSource::Feature(typed.clone()),
                    "Feature by name".to_string(),
                ),
            ];
            for field in &context.fields {
                let name = if numeric(field) {
                    format!("{field} (numeric)")
                } else {
                    field.clone()
                };
                options.push((ColorSource::Field(field.clone()), name));
            }
            let named: Vec<(ColorSource, &str)> = options
                .iter()
                .map(|(source, name)| (source.clone(), name.as_str()))
                .collect();
            rows.choice("Colour by:", &mut shown, &named);
            let mut name = typed;
            let by_name = matches!(shown, ColorSource::Feature(_));
            rows.when(by_name, |rows| rows.line("Feature:", &mut name));
            if by_name {
                shown = ColorSource::Feature(name);
            }
            let numeric_field = matches!(&shown, ColorSource::Field(f) if numeric(f));
            rows.check_if(
                numeric_field,
                "Treat the numbers as categories",
                &mut as_categories,
            );
            *o.color_by = match shown {
                ColorSource::Field(field) if numeric_field && as_categories => {
                    ColorSource::Category(field)
                }
                other => other,
            };
            let scale = matches!(
                *o.color_by,
                ColorSource::Feature(_) | ColorSource::SelectedFeature
            ) || (numeric_field && !as_categories);
            let maps: Vec<(ColorMap, &str)> =
                ColorMap::ALL.iter().map(|&m| (m, m.name())).collect();
            rows.choice_if(scale, "Colour map:", o.color_map, &maps);
            let scales: Vec<(ColorScale, &str)> =
                ColorScale::ALL.iter().map(|&s| (s, s.name())).collect();
            rows.choice_if(scale, "Colour scale:", o.color_scale, &scales);
            let orders: Vec<(DrawOrder, &str)> =
                DrawOrder::ALL.iter().map(|&d| (d, d.name())).collect();
            rows.choice_if(scale, "Drawing order:", o.draw_order, &orders);
            extra(rows, scale);

            let mut shapes: Vec<(Option<String>, &str)> = vec![(None, no_shape)];
            shapes.extend(context.fields.iter().map(|f| (Some(f.clone()), f.as_str())));
            rows.choice("Shape by:", o.shape_by, &shapes);
        });
    }

    /// The map of sample locations.
    fn map(&mut self, settings: &mut PlotSettings, context: &Context) {
        use stamp_plot::geo::{Projection, Resolution};
        use stamp_plot::map_plot::{Extent, Region, SiteValue, SizeSource};
        let o = &mut settings.map;
        self.group("Figure size", |rows| {
            rows.inches("Width:", &mut o.width, 2.0..=40.0);
        });
        let (lat_field, lon_field) = (&mut settings.map_lat_field, &mut settings.map_lon_field);
        self.group("Sample locations", |rows| {
            for (label, value, found) in [
                ("Latitude field:", lat_field, &context.lat_field),
                ("Longitude field:", lon_field, &context.lon_field),
            ] {
                let automatic = match found {
                    Some(field) if value.is_none() => format!("Automatic ({field})"),
                    _ => "Automatic".to_string(),
                };
                let mut fields: Vec<(Option<String>, &str)> = vec![(None, automatic.as_str())];
                fields.extend(context.fields.iter().map(|f| (Some(f.clone()), f.as_str())));
                rows.choice(label, value, &fields);
            }
        });
        self.group("Map", |rows| {
            let projections: Vec<(Projection, &str)> =
                Projection::ALL.iter().map(|&p| (p, p.name())).collect();
            rows.choice("Projection:", &mut o.projection, &projections);
            // A custom area keeps its numbers while another is chosen.
            let custom = match o.extent {
                Extent::Custom { .. } => o.extent,
                _ => Extent::Custom {
                    west: -30.0,
                    east: 60.0,
                    south: -40.0,
                    north: 40.0,
                },
            };
            let mut areas: Vec<(Extent, &str)> = vec![
                (Extent::Auto, Extent::Auto.name()),
                (Extent::World, Extent::World.name()),
            ];
            areas.extend(Region::ALL.iter().map(|&r| (Extent::Region(r), r.name())));
            areas.push((custom, custom.name()));
            rows.choice("Area:", &mut o.extent, &areas);
            if let Extent::Custom {
                west,
                east,
                south,
                north,
            } = &mut o.extent
            {
                rows.text("Longitudes:");
                let mut changed = false;
                rows.ui.horizontal(|ui| {
                    changed |= ui
                        .add(egui::DragValue::new(west).speed(0.5).range(-180.0..=179.0))
                        .changed();
                    ui.label("to");
                    changed |= ui
                        .add(egui::DragValue::new(east).speed(0.5).range(-179.0..=180.0))
                        .changed();
                });
                rows.ui.end_row();
                rows.text("Latitudes:");
                rows.ui.horizontal(|ui| {
                    changed |= ui
                        .add(egui::DragValue::new(south).speed(0.5).range(-90.0..=89.0))
                        .changed();
                    ui.label("to");
                    changed |= ui
                        .add(egui::DragValue::new(north).speed(0.5).range(-89.0..=90.0))
                        .changed();
                });
                rows.ui.end_row();
                if *east <= *west {
                    *east = (*west + 1.0).min(180.0);
                }
                if *north <= *south {
                    *north = (*south + 1.0).min(90.0);
                }
                rows.changed |= changed;
            }
            let details: Vec<(Resolution, &str)> =
                Resolution::ALL.iter().map(|&r| (r, r.name())).collect();
            rows.choice("Detail:", &mut o.resolution, &details);
            rows.check("Colour the sea", &mut o.ocean);
            rows.check("Outline countries and coasts", &mut o.borders);
            rows.check("Lines of longitude and latitude", &mut o.graticule);
        });
        let site_value = &mut o.site_value;
        self.styles(
            Styles {
                color_by: &mut o.color_by,
                shape_by: &mut o.shape_by,
                color_map: &mut o.color_map,
                color_scale: &mut o.color_scale,
                draw_order: &mut o.draw_order,
            },
            "None (circles)",
            context,
            |rows, scale| {
                let values: Vec<(SiteValue, &str)> =
                    SiteValue::ALL.iter().map(|&v| (v, v.name())).collect();
                rows.choice_if(scale, "Value of a site:", site_value, &values);
            },
        );
        self.group("Markers", |rows| {
            rows.check("One marker per site", &mut o.aggregate);
            let mut precision = (o.site_precision > 0.0).then_some(o.site_precision);
            rows.when(o.aggregate || o.jitter, |rows| {
                rows.optional(
                    "Site size (degrees):",
                    &mut precision,
                    0.1,
                    0.01,
                    0.001..=10.0,
                );
            });
            o.site_precision = precision.unwrap_or(0.0);
            rows.check_if(!o.aggregate, "Spread the samples of a site", &mut o.jitter);
            let typed = match &o.size_by {
                SizeSource::Feature(name) => name.clone(),
                _ => String::new(),
            };
            let mut sizes: Vec<(SizeSource, String)> = vec![
                (SizeSource::Count, "Number of samples".to_string()),
                (SizeSource::Fixed, "Same size".to_string()),
                (SizeSource::SelectedFeature, "Selected feature".to_string()),
                (
                    SizeSource::Feature(typed.clone()),
                    "Feature by name".to_string(),
                ),
            ];
            sizes.extend(
                context
                    .numeric_fields
                    .iter()
                    .map(|f| (SizeSource::Field(f.clone()), f.clone())),
            );
            let named: Vec<(SizeSource, &str)> =
                sizes.iter().map(|(s, n)| (s.clone(), n.as_str())).collect();
            rows.choice("Size by:", &mut o.size_by, &named);
            let mut name = typed;
            let by_name = matches!(o.size_by, SizeSource::Feature(_));
            rows.when(by_name, |rows| rows.line("Feature:", &mut name));
            if by_name {
                o.size_by = SizeSource::Feature(name);
            }
            let sized = o.size_by != SizeSource::Fixed;
            rows.number(
                if sized {
                    "Smallest marker:"
                } else {
                    "Marker size:"
                },
                &mut o.marker_size,
                1.0,
                1.0..=2000.0,
            );
            rows.number_if(
                sized,
                "Largest marker:",
                &mut o.max_marker_size,
                1.0,
                1.0..=5000.0,
            );
            rows.check_if(sized, "Logarithmic sizes", &mut o.size_log);
            rows.number("Opacity:", &mut o.opacity, 0.01, 0.05..=1.0);
        });
        self.group(OTHER_OPTIONS, |rows| {
            rows.check("Show legend", &mut o.legend);
        });
        self.group(FONT_SIZES, |rows| {
            rows.font("Notes above the map:", &mut o.note_font_size);
        });
    }

    /// The extended error bar plot and, with `only_significant` (its
    /// setting, and the coverage of the post-hoc intervals if known), the
    /// post-hoc plot, which lays its pairs of groups out the same way.
    fn error_bar(
        &mut self,
        options: &mut extended_error_bar::Options,
        mode: Mode,
        sequence_counts: Option<&mut bool>,
        only_significant: Option<(&mut bool, Option<f64>)>,
    ) {
        let groups = mode != Mode::TwoSamples;
        let post_hoc = only_significant.is_some();
        self.group("Sorting", |rows| {
            let labels = if post_hoc {
                "Group labels"
            } else {
                "Feature labels"
            };
            rows.choice(
                "Field:",
                &mut options.sort_by,
                &[
                    (SortBy::PValues, "p-values"),
                    (SortBy::EffectSizes, "Effect sizes"),
                    (SortBy::FeatureLabels, labels),
                ],
            );
        });
        if !post_hoc {
            self.group("Multiple comparisons", |rows| {
                rows.check(
                    "Show corrected p-values",
                    &mut options.show_corrected_p_values,
                );
            });
        }
        self.group("Figure size", |rows| {
            rows.inches("Width:", &mut options.width, 2.0..=40.0);
            rows.inches("Row height:", &mut options.row_height, 0.05..=2.0);
        });
        self.group("Error bar x-axis limits", |rows| {
            rows.limits(
                "Use custom limits:",
                &mut options.effect_limits,
                (-10.0, 10.0),
            );
        });
        self.group("Figure elements", |rows| {
            rows.check("Show bar subplot", &mut options.show_bars);
            if let Some(counts) = sequence_counts {
                rows.choice(
                    "Bar subplot shows:",
                    counts,
                    &[(false, "Proportions (%)"), (true, "Raw values")],
                );
            }
            if groups {
                rows.check("Show std. dev. on bar subplot", &mut options.show_std_dev);
                rows.number_if(
                    options.show_std_dev,
                    "End cap size:",
                    &mut options.end_cap_size,
                    0.1,
                    0.0..=20.0,
                );
            }
            rows.check("Show p-value labels", &mut options.show_p_values);
            rows.number("Marker size:", &mut options.marker_size, 1.0, 1.0..=500.0);
        });
        if let Some((only_significant, coverage)) = only_significant {
            self.group("Filter", |rows| {
                let level = coverage.map_or("1 - post-hoc coverage".to_string(), |coverage| {
                    stamp_plot::post_hoc::level_text(coverage)
                });
                rows.check(
                    &format!("Show only pairs of groups that differ (p \u{2264} {level})"),
                    only_significant,
                );
            });
        } else {
            self.group("Legend position", |rows| {
                rows.choice(
                    "Position:",
                    &mut options.legend,
                    &[
                        (extended_error_bar::Legend::None, "None"),
                        (extended_error_bar::Legend::UpperLeft, "Upper left"),
                        (extended_error_bar::Legend::UpperCenter, "Upper centre"),
                        (extended_error_bar::Legend::UpperRight, "Upper right"),
                        (extended_error_bar::Legend::LowerLeft, "Lower left"),
                        (extended_error_bar::Legend::LowerCenter, "Lower centre"),
                        (extended_error_bar::Legend::LowerRight, "Lower right"),
                    ],
                );
            });
        }
        self.group(FONT_SIZES, |rows| {
            let names = if post_hoc {
                "Group labels and p-values:"
            } else {
                "Feature names and p-values:"
            };
            rows.font(names, &mut options.label_font_size);
            rows.font("Titles:", &mut options.title_font_size);
        });
        self.group(OTHER_OPTIONS, |rows| {
            rows.truncate(&mut options.truncate_labels);
        });
    }
}

/// Title of the frame, placed last, for the options that belong to no
/// other group.
const OTHER_OPTIONS: &str = "Other options";
/// Title of the frame of the font sizes particular to a plot. Those of tick
/// labels, axis labels and legends are in the preferences.
const FONT_SIZES: &str = "Font sizes";

/// Width of the frames around groups of options, so that they line up.
const GROUP_WIDTH: f32 = 350.0;

/// What the settings of some plots offer to choose from: the fields of the
/// metadata (and which of them hold numbers) and the groups of the
/// analysis.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub fields: Vec<String>,
    pub groups: Vec<String>,
    pub numeric_fields: Vec<String>,
    /// The field that defines the groups of the analysis, if any.
    pub group_field: Option<String>,
    /// The coverage of the post-hoc confidence intervals, whose complement
    /// is the significance level of the post-hoc plot.
    pub post_hoc_coverage: Option<f64>,
    /// The fields that look like a latitude and a longitude.
    pub lat_field: Option<String>,
    pub lon_field: Option<String>,
}

/// The settings of colours and shapes shared by the ordination plots and
/// the map.
struct Styles<'a> {
    color_by: &'a mut stamp_plot::ordination_style::ColorSource,
    shape_by: &'a mut Option<String>,
    color_map: &'a mut ColorMap,
    color_scale: &'a mut stamp_plot::ordination_style::ColorScale,
    draw_order: &'a mut stamp_plot::ordination_style::DrawOrder,
}

/// The controls for the settings of one plot. Returns true if a setting
/// changed.
pub fn show(
    ui: &mut egui::Ui,
    mode: Mode,
    kind: PlotKind,
    settings: &mut PlotSettings,
    context: &Context,
) -> bool {
    let mut dialog = Dialog { ui, changed: false };
    match kind {
        PlotKind::ExtendedErrorBar => dialog.error_bar(
            &mut settings.error_bar,
            mode,
            (mode == Mode::TwoSamples).then_some(&mut settings.error_bar_sequence_counts),
            None,
        ),
        PlotKind::PostHoc => dialog.error_bar(
            &mut settings.post_hoc,
            mode,
            None,
            Some((
                &mut settings.post_hoc_only_significant,
                context.post_hoc_coverage,
            )),
        ),
        PlotKind::Bar if mode == Mode::TwoSamples => {
            let o = &mut settings.stat_bar;
            dialog.group("Figure size", |rows| {
                rows.inches("Width:", &mut o.width, 2.0..=40.0);
                rows.inches("Row height:", &mut o.row_height, 0.05..=2.0);
            });
            dialog.legend(&mut o.legend);
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.choice(
                    "Field to plot:",
                    &mut o.field,
                    &[
                        (BarField::Proportions, "Proportions (%)"),
                        (BarField::Sequences, "Raw values"),
                        (BarField::ParentSequences, "Number of parent sequences"),
                        (BarField::PValues, "p-values"),
                        (BarField::CorrectedPValues, "Corrected p-values"),
                        (BarField::EffectSize, "Effect size"),
                    ],
                );
                rows.check("Sort values", &mut o.sort);
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::Bar => {
            let o = &mut settings.bar;
            let field = &mut settings.feature_field;
            dialog.group("Figure size", |rows| {
                rows.inches("Height:", &mut o.height, 2.0..=40.0);
                rows.inches("Column width:", &mut o.column_width, 0.05..=2.0);
                rows.number("Bar width (%):", &mut o.bar_width_percent, 1.0, 5.0..=100.0);
            });
            dialog.legend(&mut o.legend);
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Title and p-value:", &mut o.title_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.field(field);
                rows.choice("Labels for:", &mut o.labels, &label_choices());
                rows.check("Logarithmic axis (log10)", &mut o.log_scale);
                rows.check("Show average of each group", &mut o.show_averages);
                rows.check("Show p-value", &mut o.show_p_value);
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::Box => {
            let o = &mut settings.box_plot;
            let field = &mut settings.feature_field;
            let which = &mut settings.box_features;
            let count = &mut settings.box_feature_count;
            let list = &mut settings.box_feature_list;
            let order_field = &mut settings.box_category_field;
            dialog.group("Features", |rows| {
                rows.choice(
                    "Show:",
                    which,
                    &[
                        (BoxFeatures::Selected, "The selected feature"),
                        (
                            BoxFeatures::MostSignificant,
                            "The most significant features",
                        ),
                        (BoxFeatures::Listed, "The features listed below"),
                    ],
                );
                let listed = *which == BoxFeatures::Listed;
                rows.count_if(
                    *which == BoxFeatures::MostSignificant,
                    "Number of features:",
                    count,
                    1..=50,
                );
                rows.when(listed, |rows| {
                    rows.lines("Features (one per line):", list);
                    rows.text("");
                    rows.ui.add_enabled(
                        listed,
                        egui::Label::new(
                            egui::RichText::new(
                                "Ctrl+click features in the feature table\nto add or remove them.",
                            )
                            .weak(),
                        ),
                    );
                    rows.ui.end_row();
                });
            });
            dialog.group("Layout", |rows| {
                rows.choice(
                    "Several features:",
                    &mut o.layout,
                    &[
                        (BoxLayout::Grouped, "A box per group at each feature"),
                        (BoxLayout::Panels, "One panel per feature"),
                    ],
                );
                rows.check_if(
                    o.layout == BoxLayout::Panels,
                    "Same value axis in every panel",
                    &mut o.shared_axis,
                );
                rows.check("Horizontal boxes", &mut o.horizontal);
                rows.check("Logarithmic axis (log10)", &mut o.log_scale);
            });
            dialog.group("Groups", |rows| {
                rows.choice(
                    "Order:",
                    &mut o.group_order,
                    &[
                        (BoxGroupOrder::AsGiven, "As in the analysis"),
                        (BoxGroupOrder::Alphabetical, "Alphabetical"),
                        (BoxGroupOrder::Median, "Highest median first"),
                        (BoxGroupOrder::Category, "By a metadata field"),
                    ],
                );
                let mut fields: Vec<(Option<String>, &str)> = vec![(None, "None")];
                fields.extend(context.fields.iter().map(|f| (Some(f.clone()), f.as_str())));
                rows.choice_if(
                    o.group_order == BoxGroupOrder::Category,
                    "Field:",
                    order_field,
                    &fields,
                );
                rows.check("Show number of samples", &mut o.show_counts);
            });
            dialog.group("Samples", |rows| {
                rows.choice("Plot style:", &mut o.style, &BOX_STYLES);
                // A swarm is every sample as a point already.
                rows.check_if(
                    o.style != BoxStyle::Swarm,
                    "Show every sample as a point",
                    &mut o.show_points,
                );
                rows.check_if(
                    mode == Mode::TwoGroups,
                    "Join paired samples with lines (paired tests)",
                    &mut o.show_pairs,
                );
                rows.number_if(
                    o.show_points || o.show_pairs || o.style == BoxStyle::Swarm,
                    "Point size:",
                    &mut o.point_size,
                    0.1,
                    0.5..=10.0,
                );
            });
            dialog.group("Figure size", |rows| {
                rows.inches("Height:", &mut o.height, 2.0..=40.0);
                rows.inches("Width:", &mut o.width, 2.0..=40.0);
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Titles and p-values:", &mut o.title_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.field(field);
                rows.check("Show average of each group", &mut o.show_averages);
                rows.check("Show p-value", &mut o.show_p_value);
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::PrevalenceAbundance => {
            let o = &mut settings.prevalence;
            let min_abundance = &mut settings.prevalence_min_abundance;
            let only_active = &mut settings.prevalence_only_active;
            let selected = &mut settings.prevalence_highlight_selected;
            let listed = &mut settings.prevalence_highlight;
            dialog.group("Figure size", |rows| {
                rows.size(&mut o.width, &mut o.height);
            });
            dialog.group("Values", |rows| {
                rows.number(
                    "Present at or above (%):",
                    min_abundance,
                    0.001,
                    0.0..=100.0,
                );
                rows.choice(
                    "Mean abundance over:",
                    &mut o.mean_over,
                    &[
                        (MeanOver::AllSamples, "All samples of the group"),
                        (MeanOver::PresentSamples, "Samples where present"),
                    ],
                );
                rows.check("Plot only active features", only_active);
            });
            dialog.group("View", |rows| {
                let views: Vec<(PrevalenceView, &str)> =
                    PrevalenceView::ALL.iter().map(|&v| (v, v.name())).collect();
                rows.choice("Show:", &mut o.view, &views);
                let mut groups: Vec<(Option<String>, &str)> = vec![(None, "Every group")];
                groups.extend(context.groups.iter().map(|g| (Some(g.clone()), g.as_str())));
                rows.choice_if(
                    o.view == PrevalenceView::Combined,
                    "Group:",
                    &mut o.group,
                    &groups,
                );
                rows.number_if(
                    o.view == PrevalenceView::Specificity,
                    "Specific at (percentage points):",
                    &mut o.specificity,
                    0.5,
                    0.0..=100.0,
                );
            });
            dialog.group("Highlighted features", |rows| {
                rows.check("Highlight the selected feature", selected);
                rows.lines("Also highlight (one per line):", listed);
                rows.check("Name highlighted features", &mut o.label_highlighted);
            });
            dialog.group("Points", |rows| {
                rows.number("Point size:", &mut o.point_size, 0.1, 0.5..=20.0);
                rows.number("Opacity:", &mut o.opacity, 0.01, 0.02..=1.0);
            });
            dialog.legend(&mut o.legend);
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Panel titles:", &mut o.title_font_size);
                rows.font("Names of highlighted features:", &mut o.label_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::ProfileBar => {
            let o = &mut settings.profile_bar;
            let only_active = &mut settings.profile_bar_only_active;
            dialog.group("Figure size", |rows| {
                rows.inches("Height:", &mut o.height, 2.0..=40.0);
                rows.inches("Column width:", &mut o.column_width, 0.05..=2.0);
                rows.number(
                    "Bar width (%):",
                    &mut o.bar_width_percent,
                    1.0,
                    10.0..=100.0,
                );
            });
            dialog.group("Confidence intervals", |rows| {
                rows.check("Show CIs", &mut o.show_cis);
                rows.number_if(
                    o.show_cis,
                    "End cap size:",
                    &mut o.end_cap_size,
                    0.1,
                    0.0..=20.0,
                );
            });
            dialog.group("Mark significant features", |rows| {
                rows.check("Mark features with a p-value <=", &mut o.show_p_values);
                rows.number_if(
                    o.show_p_values,
                    "p-value:",
                    &mut o.p_value_threshold,
                    0.001,
                    0.0..=1.0,
                );
            });
            dialog.legend(&mut o.legend);
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.choice(
                    "Field to plot:",
                    &mut o.sequence_counts,
                    &[(false, "Proportions (%)"), (true, "Raw values")],
                );
                rows.count(
                    "Maximum # features to show:",
                    &mut o.features_to_show,
                    1..=1000,
                );
                rows.check("Only show active features", only_active);
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::Scatter => {
            let o = &mut settings.scatter;
            let spread = &mut settings.group_spread;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Histogram", |rows| {
                rows.check("Show histograms", &mut o.show_histograms);
                rows.count_if(o.show_histograms, "Bins:", &mut o.bins, 2..=500);
                rows.inches_if(
                    o.show_histograms,
                    "Height:",
                    &mut o.histogram_size,
                    0.1..=5.0,
                );
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("R\u{b2} value:", &mut o.note_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                if mode == Mode::TwoSamples {
                    rows.check("Show CIs", &mut o.show_spread);
                } else {
                    rows.check("Show spread of each group", &mut o.show_spread);
                    rows.choice_if(
                        o.show_spread,
                        "Spread:",
                        spread,
                        &[
                            (Spread::StandardDeviation, "1 standard deviation"),
                            (Spread::TwoStandardDeviations, "2 standard deviations"),
                            (Spread::Quartiles, "25th and 75th percentiles"),
                            (Spread::Percentiles9And91, "9th and 91st percentiles"),
                            (Spread::Percentiles2And98, "2nd and 98th percentiles"),
                            (Spread::MinimumAndMaximum, "Minimum and maximum"),
                        ],
                    );
                }
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
                rows.check("Show R\u{b2} value", &mut o.show_r2);
            });
        }
        PlotKind::SequenceHistogram => {
            let o = &mut settings.sequence_histogram;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Histogram properties", |rows| {
                rows.optional("Custom bin width:", &mut o.bin_width, 10.0, 1.0, 1.0..=1e9);
                rows.check("Show y-axis as log scale", &mut o.log_scale);
            });
            dialog.group("X-axis properties", |rows| {
                rows.limits("Custom x-axis extents:", &mut o.x_limits, (0.0, 1000.0));
            });
            dialog.legend(&mut o.legend);
        }
        PlotKind::PValueHistogram => {
            let o = &mut settings.p_value_histogram;
            let corrected = &mut settings.p_value_histogram_corrected;
            dialog.group("Main figure size", |rows| {
                rows.size(&mut o.width, &mut o.height)
            });
            dialog.group("Main histogram properties", |rows| {
                rows.number("Bin width:", &mut o.bin_width, 0.001, 0.001..=0.5);
                rows.check("Show y-axis as log scale", &mut o.log_scale);
            });
            // The inset keeps its settings while it is switched off.
            let mut shown = o.inset.is_some();
            let defaults = || {
                PlotSettings::default()
                    .p_value_histogram
                    .inset
                    .unwrap_or(Inset {
                        width_percent: 60.0,
                        height_percent: 60.0,
                        bin_width: 0.002,
                        x_limit: 0.05,
                        log_scale: false,
                    })
            };
            let mut inset = o.inset.clone().unwrap_or_else(defaults);
            dialog.group("Inset figure size", |rows| {
                rows.check("Show inset", &mut shown);
                rows.number_if(
                    shown,
                    "Width (%):",
                    &mut inset.width_percent,
                    1.0,
                    10.0..=90.0,
                );
                rows.number_if(
                    shown,
                    "Height (%):",
                    &mut inset.height_percent,
                    1.0,
                    10.0..=90.0,
                );
            });
            dialog.group("Inset histogram properties", |rows| {
                rows.number_if(
                    shown,
                    "Bin width:",
                    &mut inset.bin_width,
                    0.0005,
                    0.0001..=0.5,
                );
                rows.number_if(
                    shown,
                    "x-axis limit:",
                    &mut inset.x_limit,
                    0.005,
                    0.001..=1.0,
                );
                rows.check_if(shown, "Show y-axis as log scale", &mut inset.log_scale);
            });
            o.inset = shown.then_some(inset);
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.choice(
                    "Field to plot:",
                    corrected,
                    &[(false, "p-values"), (true, "p-values (corrected)")],
                );
            });
        }
        PlotKind::MultipleComparison => {
            let o = &mut settings.multiple_comparison;
            dialog.group("Main figure size", |rows| {
                rows.size(&mut o.width, &mut o.height)
            });
            dialog.group("Histogram plot", |rows| {
                rows.number("Bin width:", &mut o.bin_width, 0.001, 0.001..=0.5);
                rows.number("x-axis limit:", &mut o.histogram_limit, 0.01, 0.01..=1.0);
                rows.check("Show y-axis as log scale", &mut o.log_scale);
            });
            dialog.group("Correction plot", |rows| {
                rows.number("x-axis limit:", &mut o.correction_limit, 0.01, 0.01..=1.0);
            });
            dialog.group("Significant features plot", |rows| {
                rows.number("x-axis limit:", &mut o.significant_limit, 0.01, 0.01..=1.0);
            });
        }
        PlotKind::Volcano => {
            let o = &mut settings.volcano;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Significant features", |rows| {
                rows.choice(
                    "Field to plot:",
                    &mut o.corrected,
                    &[(false, "p-values"), (true, "p-values (corrected)")],
                );
                rows.optional(
                    "Significance level:",
                    &mut o.significance,
                    0.05,
                    0.001,
                    0.0..=1.0,
                );
                rows.optional(
                    "Minimum effect size:",
                    &mut o.min_effect,
                    1.0,
                    0.05,
                    0.0..=f64::MAX,
                );
            });
            // Several groups have no two sides to name.
            if mode != Mode::MultipleGroups {
                dialog.legend(&mut o.legend);
            }
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
            });
        }
        PlotKind::CorrelationScatter => {
            let o = &mut settings.correlation;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Correlation", |rows| {
                rows.check("Show fitted line (least squares)", &mut o.show_line);
                rows.check("Show correlation and p-value", &mut o.show_statistics);
            });
            dialog.legend(&mut o.legend);
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Title:", &mut o.title_font_size);
                rows.font("Correlation and p-value:", &mut o.note_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
                rows.check("Assign unique shapes", &mut o.unique_shapes);
            });
        }
        PlotKind::AlphaScatter => {
            let o = &mut settings.correlation;
            let (metric, normalisation) = (
                &mut settings.alpha_metric,
                &mut settings.community_normalisation,
            );
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Diversity", |rows| {
                let metrics: Vec<(AlphaMetric, &str)> =
                    AlphaMetric::ALL.iter().map(|&m| (m, m.name())).collect();
                rows.choice("Measure:", metric, &metrics);
                rows.normalisation(normalisation);
            });
            dialog.group("Correlation", |rows| {
                rows.check("Show fitted line (least squares)", &mut o.show_line);
                rows.check("Show correlation and p-value", &mut o.show_statistics);
            });
            dialog.legend(&mut o.legend);
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Title:", &mut o.title_font_size);
                rows.font("Correlation and p-value:", &mut o.note_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
                rows.check("Assign unique shapes", &mut o.unique_shapes);
            });
        }
        PlotKind::Composition => {
            let o = &mut settings.composition;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Bars", |rows| {
                // Two samples have no groups to average over.
                if mode != Mode::TwoSamples {
                    rows.choice(
                        "Bars for:",
                        &mut o.bars,
                        &[
                            (composition::Bars::Samples, "Each sample"),
                            (composition::Bars::Groups, "Each group (mean)"),
                        ],
                    );
                }
                rows.count("Features to show:", &mut o.features_to_show, 1..=50);
                rows.check("Show Other", &mut o.show_other);
                rows.check("Show Unclassified", &mut o.show_unclassified);
                // How the bars of each sample are arranged within their
                // groups (two samples are each their own group).
                let each_sample = o.bars == composition::Bars::Samples;
                let orders: Vec<(composition::SampleOrder, &str)> = composition::SampleOrder::ALL
                    .iter()
                    .map(|&order| (order, order.name()))
                    .collect();
                rows.choice_if(
                    each_sample && mode != Mode::TwoSamples,
                    "Sort samples by:",
                    &mut o.sample_order,
                    &orders,
                );
                if o.sample_order == composition::SampleOrder::MetadataField {
                    let mut fields: Vec<(Option<String>, &str)> = vec![(None, "(choose a field)")];
                    fields.extend(context.fields.iter().map(|f| (Some(f.clone()), f.as_str())));
                    rows.choice_if(
                        each_sample && mode != Mode::TwoSamples,
                        "Metadata field:",
                        &mut o.sort_field,
                        &fields,
                    );
                }
                rows.number_if(
                    each_sample,
                    "Space between groups (pt):",
                    &mut o.group_space,
                    0.5,
                    0.0..=72.0,
                );
                rows.choice("Labels for:", &mut o.labels, &label_choices());
                rows.when(o.labels != composition::Labels::None, |rows| {
                    rows.optional(
                        "Label angle (°):",
                        &mut o.label_angle,
                        30.0,
                        1.0,
                        0.0..=90.0,
                    );
                });
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.check("Show legend", &mut o.show_legend);
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::Rarefaction => {
            let o = &mut settings.rarefaction;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Curves", |rows| {
                // Two samples have no groups to pool or to count samples in.
                if mode != Mode::TwoSamples {
                    let kinds: Vec<(rarefaction::Curves, &str)> = rarefaction::Curves::ALL
                        .iter()
                        .map(|&kind| (kind, kind.name()))
                        .collect();
                    rows.choice("Curves for:", &mut o.curves, &kinds);
                }
                let by_samples =
                    mode != Mode::TwoSamples && o.curves == rarefaction::Curves::GroupsAccumulated;
                rows.count_if(!by_samples, "Sampling points:", &mut o.points, 2..=1000);
                rows.check(
                    "Truncate to the smallest sample",
                    &mut o.truncate_to_smallest,
                );
                rows.check("Show Chao estimates", &mut o.show_chao1);
            });
            dialog.legend(&mut o.legend);
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Names at the curve ends:", &mut o.label_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.check("Label curves", &mut o.label_samples);
            });
        }
        PlotKind::Map => dialog.map(settings, context),
        PlotKind::Custom => {
            dialog.ui.label(
                "The assistant drew this plot. Ask it to change the plot; the font sizes \
                 below apply to it as to every plot.",
            );
        }
        PlotKind::Ordination => {
            let method = settings.ordination_method;
            let methods: Vec<(OrdinationMethod, &str)> = OrdinationMethod::ALL
                .iter()
                .map(|&m| (m, m.long_name()))
                .collect();
            let variables = &mut settings.ordination_variables;
            dialog.group("Method", |rows| {
                rows.choice("Method:", &mut settings.ordination_method, &methods);
            });
            if method.constrained().is_some() {
                dialog.explanatory_variables(variables, context);
            }
            let (distance, normalisation, permutations, show_tests) = (
                &mut settings.community_distance,
                &mut settings.community_normalisation,
                &mut settings.community_permutations,
                &mut settings.ordination_show_tests,
            );
            dialog.ordination(&mut settings.ordination, method, context, |dialog| {
                dialog.group("Distances between samples", |rows| {
                    rows.when(method.uses_distance(), |rows| {
                        let distances: Vec<(Distance, &str)> =
                            Distance::ALL.iter().map(|&d| (d, d.name())).collect();
                        rows.choice("Distance:", distance, &distances);
                    });
                    rows.normalisation(normalisation);
                    let tests = match method {
                        OrdinationMethod::Pcoa | OrdinationMethod::Nmds => "Show community tests",
                        _ => "Show permutation tests",
                    };
                    rows.check_if(method != OrdinationMethod::Nmds, tests, show_tests);
                    rows.count_if(
                        *show_tests && method != OrdinationMethod::Nmds,
                        "Permutations:",
                        permutations,
                        99..=99_999,
                    );
                });
            });
        }
        PlotKind::Mantel => {
            let o = &mut settings.mantel;
            let (distance, normalisation, permutations) = (
                &mut settings.community_distance,
                &mut settings.community_normalisation,
                &mut settings.community_permutations,
            );
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Distances between samples", |rows| {
                rows.community(distance, normalisation);
                rows.count("Permutations:", permutations, 99..=99_999);
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Title:", &mut o.title_font_size);
                rows.font("Mantel test:", &mut o.note_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.check("Show fitted line (least squares)", &mut o.show_line);
                rows.check("Show Mantel test", &mut o.show_statistics);
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
            });
        }
        PlotKind::Ancom => {
            let o = &mut settings.ancom;
            let (test, alpha) = (&mut settings.ancom_test, &mut settings.ancom_alpha);
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Log-ratio tests", |rows| {
                let tests: Vec<(MultiGroupTest, &str)> =
                    MultiGroupTest::ALL.iter().map(|&t| (t, t.name())).collect();
                rows.choice("Test:", test, &tests);
                rows.number("Significance level:", alpha, 0.001, 0.0001..=0.5);
            });
            dialog.legend(&mut o.legend);
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Summary:", &mut o.note_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.number("Marker size:", &mut o.marker_size, 1.0, 1.0..=500.0);
            });
        }
        PlotKind::SharedFeatures => {
            let o = &mut settings.shared;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Shared features", |rows| {
                rows.choice(
                    "Style:",
                    &mut o.style,
                    &[
                        (shared_features::Style::UpSet, "UpSet"),
                        (shared_features::Style::Venn, "Venn (two or three groups)"),
                    ],
                );
                // Two samples are groups of one sample each.
                let groups = mode != Mode::TwoSamples;
                rows.check_if(groups, "Presence filter", &mut o.presence_filter);
                rows.number_if(
                    groups && o.presence_filter,
                    "Present if in at least (% of samples):",
                    &mut o.min_prevalence,
                    1.0,
                    0.0..=100.0,
                );
                rows.check_if(
                    groups && o.presence_filter,
                    "Absence filter",
                    &mut o.absence_filter,
                );
                rows.number_if(
                    groups && o.presence_filter && o.absence_filter,
                    "Absent if in less than (% of samples):",
                    &mut o.absence_threshold,
                    1.0,
                    0.0..=100.0,
                );
                rows.count_if(
                    o.style == shared_features::Style::UpSet,
                    "Combinations shown:",
                    &mut o.max_intersections,
                    1..=200,
                );
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Group names and counts:", &mut o.label_font_size);
                rows.font_if(
                    o.style == shared_features::Style::Venn,
                    "Counts in the Venn diagram:",
                    &mut o.count_font_size,
                );
                rows.font("Note above the plot:", &mut o.note_font_size);
            });
        }
        PlotKind::RankAbundance => {
            let o = &mut settings.rank_abundance;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Curves", |rows| {
                if mode != Mode::TwoSamples {
                    rows.choice(
                        "Curves for:",
                        &mut o.curves,
                        &[
                            (rank_abundance::Curves::Samples, "Each sample"),
                            (rank_abundance::Curves::Groups, "Each group (mean)"),
                        ],
                    );
                }
                rows.check("Logarithmic proportions", &mut o.log_scale);
            });
            dialog.legend(&mut o.legend);
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.number("Line width:", &mut o.line_width, 0.1, 0.25..=10.0);
            });
        }
        PlotKind::AlphaDiversity => {
            let o = &mut settings.alpha_box;
            let (metric, normalisation) = (
                &mut settings.alpha_metric,
                &mut settings.community_normalisation,
            );
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Diversity", |rows| {
                let metrics: Vec<(AlphaMetric, &str)> =
                    AlphaMetric::ALL.iter().map(|&m| (m, m.name())).collect();
                rows.choice("Measure:", metric, &metrics);
                rows.normalisation(normalisation);
            });
            dialog.group("Samples", |rows| {
                rows.choice("Plot style:", &mut o.style, &BOX_STYLES);
                rows.check_if(
                    o.style != BoxStyle::Swarm,
                    "Show every sample as a point",
                    &mut o.show_points,
                );
                rows.number_if(
                    o.show_points || o.style == BoxStyle::Swarm,
                    "Point size:",
                    &mut o.point_size,
                    0.1,
                    0.5..=10.0,
                );
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Title and p-value:", &mut o.title_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.check("Show average of each group", &mut o.show_averages);
                rows.check("Show p-value", &mut o.show_p_value);
            });
        }
        PlotKind::DistanceBox => {
            let o = &mut settings.distance_box;
            let (distance, normalisation) = (
                &mut settings.community_distance,
                &mut settings.community_normalisation,
            );
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Distances between samples", |rows| {
                rows.community(distance, normalisation);
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Title:", &mut o.title_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.check("Show average of each group", &mut o.show_averages);
            });
        }
        PlotKind::GroupHeatmap => {
            let o = &mut settings.group_heatmap;
            let summary = &mut settings.group_heatmap_summary;
            let only_active = &mut settings.group_heatmap_only_active;
            let field = &mut settings.group_heatmap_category_field;
            dialog.group("Figure size", |rows| {
                rows.check("Fit the figure to its cells", &mut o.fit_to_cells);
                rows.inches_if(o.fit_to_cells, "Cell width:", &mut o.cell_width, 0.02..=2.0);
                rows.inches_if(
                    o.fit_to_cells,
                    "Cell height:",
                    &mut o.cell_height,
                    0.02..=2.0,
                );
                rows.inches_if(!o.fit_to_cells, "Width:", &mut o.width, 2.0..=40.0);
                rows.inches_if(!o.fit_to_cells, "Height:", &mut o.height, 2.0..=40.0);
            });
            dialog.group("Values", |rows| {
                let statistics: Vec<(GroupStatistic, &str)> = vec![
                    (GroupStatistic::Mean, "Mean proportion (%)"),
                    (GroupStatistic::Median, "Median proportion (%)"),
                    (GroupStatistic::Maximum, "Maximum proportion (%)"),
                    (GroupStatistic::Prevalence, "Prevalence (% of samples)"),
                ];
                rows.choice("Statistic:", &mut summary.statistic, &statistics);
                rows.number_if(
                    summary.statistic == GroupStatistic::Prevalence,
                    "Present at or above (%):",
                    &mut summary.min_abundance,
                    0.01,
                    0.0..=100.0,
                );
                rows.check("Plot only active features", only_active);
                let mut capped = summary.max_features.is_some();
                if rows
                    .ui
                    .checkbox(&mut capped, "Most abundant features:")
                    .changed()
                {
                    summary.max_features = capped.then_some(50);
                    rows.changed = true;
                }
                match &mut summary.max_features {
                    Some(n) => {
                        rows.changed |= rows
                            .ui
                            .add(egui::DragValue::new(n).range(1..=5000))
                            .changed();
                    }
                    None => {
                        rows.ui.weak("all");
                    }
                }
                rows.ui.end_row();
            });
            dialog.group("Colours", |rows| {
                rows.choice(
                    "Scale:",
                    &mut o.scale,
                    &[
                        (Scale::Linear, "Linear"),
                        (Scale::Log10, "Logarithmic (log10)"),
                    ],
                );
                rows.number_if(
                    o.scale == Scale::Log10,
                    "Smallest value shown:",
                    &mut o.log_floor,
                    0.0001,
                    0.000001..=10.0,
                );
                let maps: Vec<(ColorMap, &str)> =
                    ColorMap::ALL.iter().map(|&m| (m, m.name())).collect();
                rows.choice("Colour map:", &mut o.color_map, &maps);
                rows.check("Show zeros in grey", &mut o.grey_zeros);
                rows.check("Show colour scale", &mut o.show_color_scale);
            });
            dialog.group("Sort rows", |rows| {
                rows.order("Method:", &mut o.row_order);
                let clustered = matches!(o.row_order, Order::Cluster(_));
                rows.check_if(clustered, "Show dendrogram", &mut o.show_row_dendrogram);
            });
            dialog.group("Sort columns", |rows| {
                let mut fields: Vec<(Option<String>, &str)> = vec![(None, "None")];
                fields.extend(context.fields.iter().map(|f| (Some(f.clone()), f.as_str())));
                rows.choice("Label groups by field:", field, &fields);
                rows.choice(
                    "Method:",
                    &mut o.column_order,
                    &[
                        (
                            GroupOrder::Cluster(Linkage::Average),
                            "Cluster: average (UPGMA)",
                        ),
                        (
                            GroupOrder::Cluster(Linkage::Nearest),
                            "Cluster: nearest neighbour",
                        ),
                        (
                            GroupOrder::Cluster(Linkage::Furthest),
                            "Cluster: furthest neighbour",
                        ),
                        (GroupOrder::Cluster(Linkage::Centroid), "Cluster: centroid"),
                        (GroupOrder::Cluster(Linkage::Ward), "Cluster: Ward"),
                        (GroupOrder::Category, "By the field above"),
                        (GroupOrder::Alphabetical, "Alphabetical"),
                        (GroupOrder::Original, "As in the analysis"),
                    ],
                );
                let clustered = matches!(o.column_order, GroupOrder::Cluster(_));
                rows.check_if(clustered, "Show dendrogram", &mut o.show_column_dendrogram);
                rows.inches_if(
                    o.show_row_dendrogram || o.show_column_dendrogram,
                    "Dendrogram size:",
                    &mut o.dendrogram_size,
                    0.1..=5.0,
                );
                rows.check("Show number of samples", &mut o.show_group_sizes);
            });
            dialog.group("Values in the cells", |rows| {
                rows.check("Show values", &mut o.show_values);
                rows.choice_if(
                    o.show_values,
                    "Format:",
                    &mut o.number_format,
                    &[
                        (NumberFormat::Decimals, "Decimal places"),
                        (NumberFormat::Significant, "Significant digits"),
                    ],
                );
                rows.count_if(o.show_values, "Digits:", &mut o.digits, 0..=6);
                rows.font_if(o.show_values, "Font size:", &mut o.value_font_size);
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Feature and group names:", &mut o.label_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::CoOccurrence => {
            let co = &mut settings.co_occurrence;
            let o = &mut settings.co_occurrence_options;
            let features = &mut settings.co_occurrence_features;
            let group = &mut settings.co_occurrence_group;
            let pair = &mut settings.co_occurrence_pair;
            dialog.group("Correlation", |rows| {
                rows.choice(
                    "Show:",
                    &mut co.view,
                    &[
                        (View::Matrix, "Every pair of features"),
                        (View::OneFeature, "The selected feature against all others"),
                        (
                            View::Pair,
                            "The selected feature against one other (scatter plot)",
                        ),
                    ],
                );
                rows.choice(
                    "Measure:",
                    &mut co.method,
                    &[
                        (CorrelationMethod::Spearman, "Spearman rank correlation"),
                        (CorrelationMethod::Pearson, "Pearson correlation"),
                    ],
                );
                rows.choice(
                    "Values:",
                    &mut co.clr,
                    &[(true, "Centred log ratios"), (false, "Proportions (%)")],
                );
                if mode != Mode::Correlation {
                    let mut groups: Vec<(Option<String>, &str)> =
                        vec![(None, "All samples of the analysis")];
                    groups.extend(context.groups.iter().map(|g| (Some(g.clone()), g.as_str())));
                    rows.choice("Samples:", group, &groups);
                }
            });
            dialog.group("Features", |rows| {
                let matrix = co.view == View::Matrix;
                rows.choice_if(
                    matrix,
                    "Rank features by:",
                    &mut co.ranking,
                    &[
                        (FeatureRanking::MeanAbundance, "Mean abundance"),
                        (FeatureRanking::Prevalence, "Prevalence"),
                    ],
                );
                rows.count_if(matrix, "Number of features:", &mut co.top, 0..=500);
                rows.when(matrix, |rows| {
                    rows.lines("Always include (one per line):", features)
                });
                rows.when(co.view == View::Pair, |rows| {
                    rows.lines(
                        "Against (a feature's name; empty for the most strongly correlated):",
                        pair,
                    )
                });
                rows.count_if(
                    co.view == View::OneFeature,
                    "Correlations shown:",
                    &mut co.partner_count,
                    1..=500,
                );
                rows.number(
                    "Present in at least (% of samples):",
                    &mut co.min_prevalence,
                    0.5,
                    0.0..=100.0,
                );
            });
            dialog.group("Significance", |rows| {
                rows.check(
                    "Mark significant correlations (*)",
                    &mut co.mark_significant,
                );
                rows.number_if(
                    co.mark_significant,
                    "q-value below (Benjamini-Hochberg):",
                    &mut co.q_threshold,
                    0.005,
                    0.0001..=1.0,
                );
                rows.number_if(
                    co.mark_significant,
                    "Correlation at least (|r|):",
                    &mut co.mark_min_coefficient,
                    0.01,
                    0.0..=1.0,
                );
            });
            dialog.group("Figure", |rows| {
                rows.check("Fit the figure to its cells", &mut o.fit_to_cells);
                rows.inches_if(o.fit_to_cells, "Cell size:", &mut o.cell_size, 0.02..=2.0);
                rows.inches_if(!o.fit_to_cells, "Width:", &mut o.width, 2.0..=40.0);
                rows.inches_if(!o.fit_to_cells, "Height:", &mut o.height, 2.0..=40.0);
                let matrix = co.view == View::Matrix;
                rows.choice_if(
                    matrix,
                    "Order:",
                    &mut o.order,
                    &[
                        (Order::Cluster(Linkage::Average), "Cluster: average (UPGMA)"),
                        (
                            Order::Cluster(Linkage::Furthest),
                            "Cluster: furthest neighbour",
                        ),
                        (Order::Cluster(Linkage::Ward), "Cluster: Ward"),
                        (Order::Original, "As chosen"),
                    ],
                );
                rows.check_if(
                    matrix && matches!(o.order, Order::Cluster(_)),
                    "Show dendrogram",
                    &mut o.show_dendrogram,
                );
                rows.check_if(matrix, "Show values in the cells", &mut o.show_values);
                let maps: Vec<(ColorMap, &str)> =
                    ColorMap::ALL.iter().map(|&m| (m, m.name())).collect();
                rows.choice("Colour map:", &mut o.color_map, &maps);
                rows.check("Reverse colours (red for positive)", &mut o.reverse_colors);
                rows.check_if(matrix, "Show colour scale", &mut o.show_color_scale);
            });
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Feature names:", &mut o.label_font_size);
                rows.font_if(
                    co.view == View::Matrix && o.show_values,
                    "Values in the cells:",
                    &mut o.value_font_size,
                );
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.truncate(&mut o.truncate_labels);
            });
        }
        PlotKind::Heatmap => {
            let o = &mut settings.heatmap;
            let counts = &mut settings.heatmap_sequence_counts;
            let only_active = &mut settings.heatmap_only_active;
            dialog.group("Figure size", |rows| rows.size(&mut o.width, &mut o.height));
            dialog.group("Sort rows", |rows| {
                rows.order("Method:", &mut o.row_order);
                let clustered = matches!(o.row_order, Order::Cluster(_));
                rows.check_if(clustered, "Show dendrogram", &mut o.show_row_dendrogram);
                let tree = clustered && o.show_row_dendrogram;
                rows.inches_if(
                    tree,
                    "Width (inches):",
                    &mut o.row_dendrogram_width,
                    0.1..=10.0,
                );
                rows.check_if(tree, "Colour clusters", &mut o.show_row_clusters);
                rows.number_if(
                    tree && o.show_row_clusters,
                    "Dendrogram clustering threshold:",
                    &mut o.row_cluster_threshold,
                    0.01,
                    0.0..=1.0,
                );
            });
            dialog.group("Sort columns", |rows| {
                rows.order("Method:", &mut o.column_order);
                let clustered = matches!(o.column_order, Order::Cluster(_));
                rows.check_if(clustered, "Show dendrogram", &mut o.show_column_dendrogram);
                rows.inches_if(
                    clustered && o.show_column_dendrogram,
                    "Height (inches):",
                    &mut o.column_dendrogram_height,
                    0.1..=10.0,
                );
            });
            dialog.legend(&mut o.legend);
            dialog.group(FONT_SIZES, |rows| {
                rows.font("Feature and sample names:", &mut o.label_font_size);
            });
            dialog.group(OTHER_OPTIONS, |rows| {
                rows.choice(
                    "Field to plot:",
                    counts,
                    &[(false, "Proportions (%)"), (true, "Raw values")],
                );
                rows.check("Plot only active features", only_active);
                let maps: Vec<(ColorMap, &str)> =
                    ColorMap::ALL.iter().map(|&m| (m, m.name())).collect();
                rows.choice("Colour map:", &mut o.color_map, &maps);
                rows.check("Show colour scale", &mut o.show_color_scale);
                rows.choice("Labels for:", &mut o.labels, &label_choices());
                rows.truncate(&mut o.truncate_labels);
            });
        }
    }
    dialog.changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_plot_has_settings_and_can_be_reset() {
        let ctx = egui::Context::default();
        for mode in Mode::ALL {
            for &kind in PlotKind::available(mode) {
                let mut settings = PlotSettings::default();
                let mut drawn = false;
                let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let before = ui.min_rect();
                    assert!(!show(ui, mode, kind, &mut settings, &Context::default()));
                    drawn = ui.min_rect().height() > before.height() + 40.0;
                });
                assert!(drawn, "{mode:?} {kind:?} has no settings");
                // Nothing changes without the user doing something.
                assert_eq!(settings, PlotSettings::default());

                settings.error_bar.width = 12.0;
                settings.post_hoc.width = 12.0;
                settings.reset(mode, kind);
                match kind {
                    PlotKind::ExtendedErrorBar => assert_eq!(settings.error_bar.width, 7.0),
                    PlotKind::PostHoc => assert_eq!(settings.post_hoc.width, 7.0),
                    _ => assert_eq!(settings.error_bar.width, 12.0),
                }
            }
        }
    }
}
