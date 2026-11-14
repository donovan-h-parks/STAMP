//! Correlations between features (co-occurrence): a symmetric heatmap of
//! the correlation of every pair of the chosen features, or the
//! correlations of one feature with every other as ranked bars.

use stamp_core::stats::cluster::Linkage;
use stamp_core::stats::co_occurrence::{
    co_occurrence, partners, top_features, CoOccurrence, FeatureRanking, Partner,
};
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::results::clr_values;
use stamp_core::Profile;

use crate::axes::{fonts, INCH, NOTE_SIZE};
use crate::figure::{Color, Figure, HAlign, Stroke, TextStyle, VAlign};
use crate::group_heatmap::{color_bar, color_bar_height, text_color_on};
use crate::heatmap::{arrange, dendrogram, ColorMap, Order};
use crate::text;

/// What is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum View {
    /// Every pair of the chosen features.
    Matrix,
    /// One feature against every other.
    OneFeature,
}

/// What is correlated, and which features.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub method: CorrelationMethod,
    /// Correlate centred log ratios instead of proportions.
    pub clr: bool,
    pub ranking: FeatureRanking,
    /// Number of features chosen by `ranking`.
    pub top: usize,
    /// Features always included, whatever their rank.
    pub forced: Vec<String>,
    pub view: View,
    /// With [`View::OneFeature`], the number of strongest correlations shown.
    pub partner_count: usize,
    /// Mark correlations whose q-value is below `q_threshold` and whose
    /// coefficient is at least `mark_min_coefficient` from zero.
    pub mark_significant: bool,
    pub q_threshold: f64,
    pub mark_min_coefficient: f64,
    /// Leave out features present (above zero) in fewer than this
    /// percentage of the samples, unless chosen by name.
    pub min_prevalence: f64,
}

impl Settings {
    /// True if a correlation is marked as significant.
    pub fn marks(&self, coefficient: f64, q: f64) -> bool {
        self.mark_significant
            && q < self.q_threshold
            && coefficient.abs() >= self.mark_min_coefficient
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            method: CorrelationMethod::Spearman,
            clr: false,
            ranking: FeatureRanking::MeanAbundance,
            top: 30,
            forced: Vec::new(),
            view: View::Matrix,
            partner_count: 30,
            mark_significant: true,
            q_threshold: 0.05,
            mark_min_coefficient: 0.0,
            min_prevalence: 0.0,
        }
    }
}

/// The correlations of the chosen features in some samples of a profile,
/// and how they were obtained.
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub result: CoOccurrence,
    pub clr: bool,
    /// How the features were chosen, for a note on the plot.
    pub selection: String,
}

/// Each feature's values in the chosen samples (`columns`, all if `None`):
/// proportions (%), for ranking, and the values to correlate (the same, or
/// centred log ratios over all the profile's features).
fn feature_values(
    profile: &Profile,
    columns: Option<&[usize]>,
    clr: bool,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let pick = |row: Vec<f64>| match columns {
        Some(columns) => columns.iter().map(|&c| row[c]).collect(),
        None => row,
    };
    let proportions: Vec<Vec<f64>> = profile
        .features()
        .iter()
        .map(|f| pick(profile.percentages(f)))
        .collect();
    let values = if clr {
        clr_values(profile).into_iter().map(pick).collect()
    } else {
        proportions.clone()
    };
    (proportions, values)
}

/// True if a feature is present (above zero) in at least `min_prevalence`
/// percent of the samples.
fn prevalent(proportions: &[f64], min_prevalence: f64) -> bool {
    if min_prevalence <= 0.0 {
        return true;
    }
    let present = proportions.iter().filter(|&&v| v > 0.0).count();
    100.0 * present as f64 >= min_prevalence * proportions.len() as f64
}

/// Indices of named features, or an error naming one that is unknown.
fn indices(profile: &Profile, names: &[String]) -> Result<Vec<usize>, String> {
    names
        .iter()
        .map(|name| {
            profile
                .features()
                .iter()
                .position(|f| f.name == *name)
                .ok_or_else(|| format!("Unknown feature '{name}'"))
        })
        .collect()
}

impl Data {
    /// Correlate the features `settings` chooses, in the samples of the
    /// profile at `columns` (every sample if `None`).
    pub fn compute(
        profile: &Profile,
        columns: Option<&[usize]>,
        settings: &Settings,
    ) -> Result<Data, String> {
        let samples = columns.map_or(profile.sample_names.len(), <[usize]>::len);
        if samples < 3 {
            return Err("Correlations need at least three samples".to_string());
        }
        let forced = indices(profile, &settings.forced)?;
        let (proportions, values) = feature_values(profile, columns, settings.clr);
        // Only features prevalent enough compete for the top places.
        let eligible: Vec<usize> = (0..proportions.len())
            .filter(|&i| prevalent(&proportions[i], settings.min_prevalence))
            .collect();
        let candidates: Vec<Vec<f64>> = eligible.iter().map(|&i| proportions[i].clone()).collect();
        let mut chosen: Vec<usize> = top_features(&candidates, settings.ranking, settings.top, &[])
            .into_iter()
            .map(|k| eligible[k])
            .collect();
        for index in forced {
            if !chosen.contains(&index) {
                chosen.push(index);
            }
        }
        if chosen.len() < 2 {
            return Err("Correlations need at least two features".to_string());
        }
        let names: Vec<String> = chosen
            .iter()
            .map(|&i| profile.features()[i].name.clone())
            .collect();
        let rows: Vec<Vec<f64>> = chosen.iter().map(|&i| values[i].clone()).collect();
        let added = chosen.len() - settings.top.min(eligible.len()).min(chosen.len());
        let mut selection = format!(
            "{} features of highest {}",
            chosen.len() - added,
            match settings.ranking {
                FeatureRanking::MeanAbundance => "mean abundance",
                FeatureRanking::Prevalence => "prevalence",
            }
        );
        if added > 0 {
            selection.push_str(&format!(" and {added} chosen"));
        }
        Ok(Data {
            result: co_occurrence(names, &rows, settings.method),
            clr: settings.clr,
            selection,
        })
    }
}

/// Correlations of one feature with every other feature of the profile,
/// strongest first.
pub fn compute_partners(
    profile: &Profile,
    columns: Option<&[usize]>,
    feature: &str,
    settings: &Settings,
) -> Result<Vec<Partner>, String> {
    let samples = columns.map_or(profile.sample_names.len(), <[usize]>::len);
    if samples < 3 {
        return Err("Correlations need at least three samples".to_string());
    }
    let target = indices(profile, &[feature.to_string()])?[0];
    let (proportions, values) = feature_values(profile, columns, settings.clr);
    let others: Vec<(String, &[f64])> = profile
        .features()
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != target && prevalent(&proportions[*i], settings.min_prevalence))
        .map(|(i, f)| (f.name.clone(), values[i].as_slice()))
        .collect();
    Ok(partners(&values[target], &others, settings.method))
}

/// Label of a coefficient: "Spearman rho of CLR values".
fn coefficient_label(method: CorrelationMethod, clr: bool) -> String {
    let base = match method {
        CorrelationMethod::Pearson => "Pearson r",
        CorrelationMethod::Spearman => "Spearman rho",
    };
    if clr {
        format!("{base} (centred log ratios)")
    } else {
        format!("{base} (proportions)")
    }
}

/// Appearance of the plots. Lengths are in inches.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub width: f64,
    pub height: f64,
    /// Size the figure to give every cell `cell_size` (or every bar
    /// `cell_size` of height).
    pub fit_to_cells: bool,
    pub cell_size: f64,
    /// Order of the features: clustered, or as chosen.
    pub order: Order,
    pub show_dendrogram: bool,
    pub dendrogram_size: f64,
    pub color_map: ColorMap,
    /// Run the colour map from +1 down to -1, so that the red end of a map
    /// from red to blue marks positive correlations.
    pub reverse_colors: bool,
    /// Write each correlation in its cell.
    pub show_values: bool,
    pub value_font_size: f64,
    pub show_color_scale: bool,
    pub truncate_labels: Option<usize>,
    pub label_font_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: 7.0,
            height: 7.0,
            fit_to_cells: true,
            cell_size: 0.16,
            order: Order::Cluster(Linkage::Average),
            show_dendrogram: true,
            dendrogram_size: 0.6,
            color_map: ColorMap::RedToBlue,
            reverse_colors: true,
            show_values: false,
            value_font_size: 5.0,
            show_color_scale: true,
            truncate_labels: Some(40),
            label_font_size: NOTE_SIZE,
        }
    }
}

impl Options {
    /// Colour of a correlation from -1 to 1.
    fn color(&self, r: f64) -> Color {
        let fraction = (r + 1.0) / 2.0;
        self.color_map.color(if self.reverse_colors {
            1.0 - fraction
        } else {
            fraction
        })
    }

    /// Colour of a position along the colour scale (0 at -1).
    fn bar_color(&self, fraction: f64) -> Color {
        self.color(fraction * 2.0 - 1.0)
    }
}

const DIAGONAL_COLOR: Color = Color::rgb(0xe8, 0xe8, 0xe8);

fn shorten(name: &str, length: Option<usize>) -> String {
    match length {
        Some(length) => text::truncate(name, length),
        None => name.to_string(),
    }
}

fn significance_note(settings: &Settings, tests: usize) -> String {
    let strength = if settings.mark_min_coefficient > 0.0 {
        format!(
            " and |{}| \u{2265} {}",
            settings.method.coefficient_name(),
            settings.mark_min_coefficient
        )
    } else {
        String::new()
    };
    format!(
        "* q < {}{strength} (Benjamini-Hochberg over {tests} correlations)",
        settings.q_threshold
    )
}

/// The heatmap of every pair of features.
pub fn matrix_plot(data: &Data, settings: &Settings, options: &Options) -> Figure {
    let result = &data.result;
    let n = result.features.len();
    if n < 2 {
        return Figure::message("Correlations need at least two features");
    }
    let items: Vec<Vec<f64>> = result
        .coefficients
        .iter()
        .map(|r| {
            r.iter()
                .map(|&v| if v.is_finite() { v } else { 0.0 })
                .collect()
        })
        .collect();
    let order_kind = match options.order {
        Order::MeanAbundance => Order::Original,
        other => other,
    };
    let (order, tree) = arrange(&items, &result.features, order_kind);
    let tree = tree.filter(|_| options.show_dendrogram);
    let labels: Vec<String> = order
        .iter()
        .map(|&i| shorten(&result.features[i], options.truncate_labels))
        .collect();
    let size = options.label_font_size;
    let label_width = labels
        .iter()
        .map(|l| text::width(l, size))
        .fold(0.0, f64::max);

    let margin = 0.15 * INCH;
    let dendrogram_space = if tree.is_some() {
        options.dendrogram_size * INCH
    } else {
        0.0
    };
    let tests = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .filter(|&(i, j)| result.q_values[i][j].is_finite())
        .count();
    let notes: Vec<String> = {
        let mut notes = vec![format!("{} samples; {}", result.samples, data.selection)];
        if settings.mark_significant {
            notes.push(significance_note(settings, tests));
        }
        notes
    };
    let notes_height = notes.len() as f64 * NOTE_SIZE * 1.4;
    let scale_height = if options.show_color_scale {
        color_bar_height() + 0.12 * INCH
    } else {
        0.0
    };
    let (left, top) = (margin + dendrogram_space, margin + dendrogram_space);
    let below = 4.0 + label_width + 0.12 * INCH + scale_height + notes_height + margin;
    let right_space = 4.0 + label_width + margin;
    let (cell, mut figure) = if options.fit_to_cells {
        let cell = options.cell_size * INCH;
        let width = (left + cell * n as f64 + right_space).max(3.5 * INCH);
        let height = top + cell * n as f64 + below;
        (cell, Figure::new(width, height))
    } else {
        let figure = Figure::with_size_inches(options.width, options.height);
        let cell = ((figure.width - left - right_space) / n as f64)
            .min((figure.height - top - below) / n as f64);
        if cell * (n as f64) < 0.5 * INCH {
            return Figure::message(
                "The figure is too small for this heatmap; enlarge it or fit it to its cells",
            );
        }
        (cell, figure)
    };
    let (right, bottom) = (left + cell * n as f64, top + cell * n as f64);

    let mark_style =
        TextStyle::new((cell * 0.9).clamp(4.0, 10.0)).align(HAlign::Center, VAlign::Middle);
    let value_style = TextStyle::new(options.value_font_size).align(HAlign::Center, VAlign::Middle);
    for (r, &i) in order.iter().enumerate() {
        for (c, &j) in order.iter().enumerate() {
            let value = result.coefficients[i][j];
            let (x, y) = (left + cell * c as f64, top + cell * r as f64);
            let fill = if i == j {
                DIAGONAL_COLOR
            } else if value.is_finite() {
                options.color(value)
            } else {
                Color::WHITE
            };
            figure.rect(x, y, cell + 0.3, cell + 0.3, Some(fill), None);
            if i == j || !value.is_finite() {
                continue;
            }
            let q = result.q_values[i][j];
            if options.show_values {
                figure.text(
                    x + cell / 2.0,
                    y + cell / 2.0,
                    format!("{value:.2}"),
                    value_style.color(text_color_on(fill)),
                );
            } else if settings.marks(value, q) {
                figure.text(
                    x + cell / 2.0,
                    y + cell * 0.62,
                    "*",
                    mark_style.color(text_color_on(fill)),
                );
            }
            figure.region(
                x,
                y,
                cell,
                cell,
                &result.features[i],
                format!(
                    "{} and {}\n{}: {value:.3}\np = {}, q = {}",
                    result.features[i],
                    result.features[j],
                    result.method.coefficient_name(),
                    crate::group_heatmap::format_significant(result.p_values[i][j], 3),
                    crate::group_heatmap::format_significant(q, 3),
                ),
            );
        }
    }
    figure.rect(
        left,
        top,
        right - left,
        bottom - top,
        None,
        Some(Stroke::solid(Color::BLACK, 0.5)),
    );

    let style = TextStyle::new(size);
    if cell >= size * 0.8 {
        for (k, label) in labels.iter().enumerate() {
            let middle = cell * (k as f64 + 0.5);
            figure.text(
                right + 4.0,
                top + middle,
                label.clone(),
                style.align(HAlign::Left, VAlign::Middle),
            );
            figure.text(
                left + middle,
                bottom + 4.0,
                label.clone(),
                style.align(HAlign::Right, VAlign::Middle).rotated(90.0),
            );
        }
    }
    if let Some(tree) = &tree {
        let max = tree.max_height().max(f64::MIN_POSITIVE);
        let leaf_y = |position: usize| top + cell * (position as f64 + 0.5);
        let base = left - 2.0;
        let across = |h: f64| base - (base - margin) * h / max;
        dendrogram(&mut figure, tree, &leaf_y, &across, false);
        let leaf_x = |position: usize| left + cell * (position as f64 + 0.5);
        let base = top - 2.0;
        let up = |h: f64| base - (base - margin) * h / max;
        dendrogram(&mut figure, tree, &leaf_x, &up, true);
    }

    let mut y = bottom + 4.0 + label_width + 0.12 * INCH;
    if options.show_color_scale {
        let ticks: Vec<(f64, String)> = [-1.0, -0.5, 0.0, 0.5, 1.0]
            .iter()
            .map(|&v| (v, crate::axis::format_tick(v, if v == 0.0 { 0 } else { 1 })))
            .collect();
        let width = (2.0 * INCH).min(figure.width - 2.0 * margin);
        color_bar(
            &mut figure,
            margin,
            y,
            width,
            &|f| options.bar_color(f),
            (-1.0, 1.0),
            &ticks,
            &coefficient_label(result.method, data.clr),
        );
        y += scale_height;
    }
    for note in notes {
        figure.text(
            margin,
            y,
            note,
            TextStyle::new(NOTE_SIZE).align(HAlign::Left, VAlign::Top),
        );
        y += NOTE_SIZE * 1.4;
    }
    figure
}

/// The correlations of one feature with the others, as bars from zero,
/// strongest first.
#[allow(clippy::too_many_arguments)]
pub fn partner_plot(
    feature: &str,
    found: &[Partner],
    samples: usize,
    clr: bool,
    settings: &Settings,
    options: &Options,
) -> Figure {
    let shown: Vec<&Partner> = found.iter().take(settings.partner_count.max(1)).collect();
    if shown.is_empty() {
        return Figure::message(&format!(
            "'{feature}' does not vary in these samples, or no other feature does"
        ));
    }
    let size = options.label_font_size;
    let labels: Vec<String> = shown
        .iter()
        .map(|p| shorten(&p.feature, options.truncate_labels))
        .collect();
    let label_width = labels
        .iter()
        .map(|l| text::width(l, size))
        .fold(0.0, f64::max);
    let margin = 0.15 * INCH;
    let title_height = fonts().axis_label * 1.6;
    let notes = {
        let mut notes = vec![format!(
            "{samples} samples; the {} strongest of {} correlations",
            shown.len(),
            found.len()
        )];
        if settings.mark_significant {
            notes.push(significance_note(settings, found.len()));
        }
        notes
    };
    let axis_height = fonts().tick * 1.4 + 4.0 + fonts().axis_label * 1.5;
    let below = axis_height + notes.len() as f64 * NOTE_SIZE * 1.4 + margin;
    let left = margin + label_width + 6.0;
    let top = margin + title_height;
    let (row, mut figure) = if options.fit_to_cells {
        let row = options.cell_size * INCH;
        let width = options.width.max(3.5) * INCH;
        (
            row,
            Figure::new(width, top + row * shown.len() as f64 + below),
        )
    } else {
        let figure = Figure::with_size_inches(options.width, options.height);
        let row = (figure.height - top - below) / shown.len() as f64;
        if row < 2.0 {
            return Figure::message("The figure is too small for this many correlations");
        }
        (row, figure)
    };
    let right = figure.width - margin - 0.3 * INCH;
    if right - left < INCH {
        return Figure::message("The figure is too narrow for these feature names");
    }
    let bottom = top + row * shown.len() as f64;
    let x_of = |r: f64| left + (right - left) * (r + 1.0) / 2.0;

    figure.text(
        (left + right) / 2.0,
        margin,
        format!("Correlations with {feature}"),
        crate::axes::axis_label_style().align(HAlign::Center, VAlign::Top),
    );
    // Grid lines at the ticks.
    let grey = Stroke::solid(Color::rgb(0xcc, 0xcc, 0xcc), 0.5);
    for tick in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let x = x_of(tick);
        let stroke = if tick == 0.0 {
            Stroke::solid(Color::BLACK, 0.75)
        } else {
            grey
        };
        figure.line(&[(x, top), (x, bottom)], stroke);
        figure.line(
            &[(x, bottom), (x, bottom + 3.0)],
            Stroke::solid(Color::BLACK, 0.5),
        );
        figure.text(
            x,
            bottom + 4.0,
            crate::axis::format_tick(tick, if tick == 0.0 { 0 } else { 1 }),
            crate::axes::tick_style().align(HAlign::Center, VAlign::Top),
        );
    }
    figure.line(
        &[(left, bottom), (right, bottom)],
        Stroke::solid(Color::BLACK, 0.5),
    );
    figure.text(
        (left + right) / 2.0,
        bottom + 4.0 + fonts().tick * 1.4,
        coefficient_label(settings.method, clr),
        crate::axes::axis_label_style().align(HAlign::Center, VAlign::Top),
    );
    let style = TextStyle::new(size);
    for (k, (partner, label)) in shown.iter().zip(&labels).enumerate() {
        let y = top + row * k as f64;
        let (x0, x1) = (x_of(0.0), x_of(partner.coefficient));
        let fill = options.color(partner.coefficient);
        figure.rect(
            x0.min(x1),
            y + row * 0.15,
            (x1 - x0).abs(),
            row * 0.7,
            Some(fill),
            Some(Stroke::solid(Color::rgb(0x55, 0x55, 0x55), 0.4)),
        );
        figure.text(
            left - 6.0,
            y + row / 2.0,
            label.clone(),
            style.align(HAlign::Right, VAlign::Middle),
        );
        if settings.marks(partner.coefficient, partner.q_value) {
            let (x, align) = if partner.coefficient >= 0.0 {
                (x1 + 2.0, HAlign::Left)
            } else {
                (x1 - 2.0, HAlign::Right)
            };
            figure.text(
                x,
                y + row * 0.62,
                "*",
                TextStyle::new((row * 0.9).clamp(5.0, 10.0)).align(align, VAlign::Middle),
            );
        }
        figure.region(
            left,
            y,
            right - left,
            row,
            &partner.feature,
            format!(
                "{}\n{}: {:.3}\np = {}, q = {}",
                partner.feature,
                settings.method.coefficient_name(),
                partner.coefficient,
                crate::group_heatmap::format_significant(partner.p_value, 3),
                crate::group_heatmap::format_significant(partner.q_value, 3),
            ),
        );
    }
    let mut y = bottom + axis_height;
    for note in notes {
        figure.text(
            margin,
            y,
            note,
            TextStyle::new(NOTE_SIZE).align(HAlign::Left, VAlign::Top),
        );
        y += NOTE_SIZE * 1.4;
    }
    figure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Shape;

    fn profile() -> Profile {
        let spf = "F\ts1\ts2\ts3\ts4\ts5\ts6\n\
            a\t10\t20\t30\t40\t50\t60\n\
            b\t11\t19\t33\t38\t52\t61\n\
            c\t60\t50\t40\t30\t20\t10\n\
            d\t0\t0\t0\t0\t0\t0\n\
            e\t1\t0\t2\t0\t1\t0\n";
        let tree = stamp_core::ProfileTree::read(spf).unwrap();
        let samples = ["s1", "s2", "s3", "s4", "s5", "s6"];
        tree.profile(&samples, None, "F", stamp_core::Unclassified::Retain)
            .unwrap()
    }

    fn texts(figure: &Figure) -> Vec<String> {
        figure
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn features_are_chosen_and_correlated() {
        let settings = Settings {
            top: 3,
            ..Settings::default()
        };
        let data = Data::compute(&profile(), None, &settings).unwrap();
        let mut names = data.result.features.clone();
        names.sort();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(data.result.samples, 6);
        let at = |name: &str| data.result.features.iter().position(|f| f == name).unwrap();
        assert!(data.result.coefficients[at("a")][at("c")] < -0.9);
        assert_eq!(data.selection, "3 features of highest mean abundance");
        // A chosen feature is added; an unknown one is an error.
        let forced = Settings {
            top: 2,
            forced: vec!["e".into()],
            ..settings.clone()
        };
        let data = Data::compute(&profile(), None, &forced).unwrap();
        assert_eq!(data.result.features.len(), 3);
        assert_eq!(data.result.features[2], "e");
        assert_eq!(
            data.selection,
            "2 features of highest mean abundance and 1 chosen"
        );
        let unknown = Settings {
            forced: vec!["zz".into()],
            ..settings.clone()
        };
        assert_eq!(
            Data::compute(&profile(), None, &unknown).unwrap_err(),
            "Unknown feature 'zz'"
        );
        // e is present in half of the samples: left out at 60%.
        let common = Settings {
            top: 5,
            min_prevalence: 60.0,
            ..settings.clone()
        };
        let data = Data::compute(&profile(), None, &common).unwrap();
        assert!(!data.result.features.contains(&"e".to_string()));
        assert!(!data.result.features.contains(&"d".to_string()));
        // Only some samples.
        let data = Data::compute(&profile(), Some(&[0, 1, 2, 3]), &settings).unwrap();
        assert_eq!(data.result.samples, 4);
        assert!(Data::compute(&profile(), Some(&[0, 1]), &settings).is_err());
        // Centred log ratios differ from proportions.
        let clr = Settings {
            clr: true,
            method: CorrelationMethod::Pearson,
            ..settings.clone()
        };
        let plain = Settings {
            method: CorrelationMethod::Pearson,
            ..settings
        };
        let (a, b) = (
            Data::compute(&profile(), None, &clr).unwrap(),
            Data::compute(&profile(), None, &plain).unwrap(),
        );
        assert_ne!(a.result.coefficients[0][1], b.result.coefficients[0][1]);
    }

    #[test]
    fn the_matrix_is_symmetric_and_marks_significance() {
        let settings = Settings {
            top: 5,
            ..Settings::default()
        };
        let data = Data::compute(&profile(), None, &settings).unwrap();
        let figure = matrix_plot(&data, &settings, &Options::default());
        // 25 cells; d does not vary, so its pairs have no region.
        assert_eq!(figure.regions.len(), 20 - 8);
        let shown = texts(&figure);
        assert!(shown.iter().any(|t| t == "*"));
        assert!(shown.contains(&"Spearman rho (proportions)".to_string()));
        assert!(shown.iter().any(|t| t.starts_with("* q < 0.05")));
        assert!(shown.iter().any(|t| t.starts_with("6 samples;")));
        // Rows and columns follow the same order: each label twice.
        for name in ["a", "b", "c", "d", "e"] {
            assert_eq!(shown.iter().filter(|t| *t == name).count(), 2, "{name}");
        }
        let unmarked = Settings {
            mark_significant: false,
            ..settings.clone()
        };
        assert!(!texts(&matrix_plot(&data, &unmarked, &Options::default()))
            .iter()
            .any(|t| t == "*"));
        // Only strong correlations marked.
        let strong = Settings {
            mark_min_coefficient: 1.5,
            ..settings.clone()
        };
        let figure = matrix_plot(&data, &strong, &Options::default());
        let stars = |figure: &Figure| texts(figure).iter().filter(|t| *t == "*").count();
        assert_eq!(stars(&figure), 0);
        assert!(stars(&matrix_plot(&data, &settings, &Options::default())) > 0);
        assert!(texts(&figure)
            .iter()
            .any(|t| t.contains("|rho| \u{2265} 1.5")));
        let values = Options {
            show_values: true,
            ..Options::default()
        };
        assert!(texts(&matrix_plot(&data, &settings, &values))
            .iter()
            .any(|t| t == "-1.00"));
    }

    #[test]
    fn colours_run_from_blue_to_red() {
        let options = Options::default();
        let negative = options.color(-1.0);
        let positive = options.color(1.0);
        assert!(positive.r > positive.b && negative.b > negative.r);
        assert_eq!(options.color(0.0), Color::rgb(247, 247, 247));
    }

    #[test]
    fn partners_of_one_feature() {
        let settings = Settings::default();
        let found = compute_partners(&profile(), None, "a", &settings).unwrap();
        // d does not vary and is left out.
        let names: Vec<&str> = found.iter().map(|p| p.feature.as_str()).collect();
        assert_eq!(names.len(), 3);
        assert!(!names.contains(&"d"));
        let figure = partner_plot("a", &found, 6, false, &settings, &Options::default());
        let shown = texts(&figure);
        assert!(shown.contains(&"Correlations with a".to_string()));
        assert_eq!(figure.regions.len(), 3);
        assert!(compute_partners(&profile(), None, "nope", &settings).is_err());
        let flat = partner_plot("d", &[], 6, false, &settings, &Options::default());
        assert!(texts(&flat)[0].contains("does not vary"));
    }
}
