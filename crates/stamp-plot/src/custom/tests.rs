use super::*;
use crate::figure::Shape;
use serde_json::json;

fn draw(spec: Value) -> Result<Built, String> {
    let profile = Table::from_json(
        json!([
            {"sample": "s1", "feature": "Bacteroides", "count": 40, "proportion": 40.0, "group": "Gut"},
            {"sample": "s1", "feature": "Prevotella", "count": 60, "proportion": 60.0, "group": "Gut"},
            {"sample": "s2", "feature": "Bacteroides", "count": 10, "proportion": 20.0, "group": "Gut"},
            {"sample": "s2", "feature": "Prevotella", "count": 40, "proportion": 80.0, "group": "Gut"},
            {"sample": "s3", "feature": "Bacteroides", "count": 5, "proportion": 25.0, "group": "Skin"},
            {"sample": "s3", "feature": "Prevotella", "count": 15, "proportion": 75.0, "group": "Skin"}
        ])
        .as_array()
        .unwrap(),
    )
    .unwrap();
    let samples = Table::from_json(
        json!([
            {"sample": "s1", "Site": "North"},
            {"sample": "s2", "Site": "South"},
            {"sample": "s3", "Site": "South"}
        ])
        .as_array()
        .unwrap(),
    )
    .unwrap();
    let tables = HashMap::from([
        ("profile".to_string(), Arc::new(profile)),
        ("samples".to_string(), Arc::new(samples)),
    ]);
    let colors = HashMap::from([
        ("Gut".to_string(), Color::rgb(1, 2, 3)),
        ("Skin".to_string(), Color::rgb(4, 5, 6)),
    ]);
    plot(&spec, &tables, &colors)
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

fn rects(figure: &Figure) -> Vec<(f64, f64, f64, f64, Option<Color>)> {
    figure
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Rect {
                x,
                y,
                width,
                height,
                fill,
                ..
            } => Some((*x, *y, *width, *height, *fill)),
            _ => None,
        })
        .collect()
}

fn values() -> Value {
    json!({"values": [
        {"taxon": "A", "abundance": 3, "site": "x"},
        {"taxon": "B", "abundance": 7, "site": "x"},
        {"taxon": "C", "abundance": 5, "site": "y"}
    ]})
}

#[test]
fn a_bar_chart_has_bars_axes_and_tooltips() {
    let built = draw(json!({
        "title": "Abundance",
        "data": values(),
        "mark": "bar",
        "encoding": {
            "x": {"field": "taxon", "type": "nominal"},
            "y": {"field": "abundance", "type": "quantitative"}
        }
    }))
    .unwrap();
    let f = &built.figure;
    let bars: Vec<_> = rects(f)
        .into_iter()
        .filter(|r| r.4 == Some(DEFAULT_COLOR))
        .collect();
    assert_eq!(bars.len(), 3);
    // B (7) is taller than C (5), which is taller than A (3); all stand on
    // the same base.
    assert!(bars[1].3 > bars[2].3 && bars[2].3 > bars[0].3);
    assert!((bars[0].1 + bars[0].3 - (bars[1].1 + bars[1].3)).abs() < 1e-9);
    let t = texts(f);
    for label in ["Abundance", "A", "B", "C", "taxon", "abundance", "0", "7"] {
        assert!(t.contains(&label.to_string()), "{label} missing from {t:?}");
    }
    assert_eq!(f.regions.len(), 3);
    assert!(f.regions[1].tooltip.contains("abundance: 7"));
    assert!(
        f.width > 180.0 && f.height > 200.0,
        "{} x {}",
        f.width,
        f.height
    );
    assert!(built.notes.is_empty(), "{:?}", built.notes);
}

#[test]
fn sorting_by_the_other_axis() {
    let built = draw(json!({
        "data": values(),
        "mark": "bar",
        "encoding": {
            "y": {"field": "taxon", "type": "nominal", "sort": "-x"},
            "x": {"field": "abundance", "type": "quantitative"}
        }
    }))
    .unwrap();
    // Horizontal bars, largest at the top.
    let bars: Vec<_> = rects(&built.figure)
        .into_iter()
        .filter(|r| r.4 == Some(DEFAULT_COLOR))
        .collect();
    let b = bars
        .iter()
        .find(|r| (r.2 - bars.iter().map(|r| r.2).fold(0.0, f64::max)).abs() < 1e-9)
        .unwrap();
    assert!(bars.iter().all(|r| r.1 >= b.1));
}

#[test]
fn stacked_and_grouped_bars() {
    let stacked = draw(json!({
        "data": {"name": "profile"},
        "mark": "bar",
        "encoding": {
            "x": {"field": "sample", "type": "nominal"},
            "y": {"field": "proportion", "type": "quantitative"},
            "color": {"field": "feature", "type": "nominal"}
        }
    }))
    .unwrap();
    let f = &stacked.figure;
    let bars: Vec<_> = rects(f)
        .into_iter()
        .filter(|r| r.4.is_some_and(|c| axes::GROUP_COLORS[..2].contains(&c)))
        .filter(|r| r.2 > 15.0)
        .collect();
    assert_eq!(bars.len(), 6, "{:?}", rects(f));
    // Each sample's two pieces together reach 100%: the same height.
    let total = |x: f64| {
        bars.iter()
            .filter(|r| (r.0 - x).abs() < 1e-6)
            .map(|r| r.3)
            .sum::<f64>()
    };
    assert!((total(bars[0].0) - total(bars[2].0)).abs() < 1e-6);
    assert!(texts(f).contains(&"Prevotella".to_string()));
    assert!(texts(f).contains(&"feature".to_string()));

    let grouped = draw(json!({
        "data": {"name": "profile"},
        "mark": "bar",
        "encoding": {
            "x": {"field": "sample", "type": "nominal"},
            "xOffset": {"field": "feature"},
            "y": {"field": "count", "type": "quantitative"},
            "color": {"field": "feature", "type": "nominal"}
        }
    }))
    .unwrap();
    let bars: Vec<_> = rects(&grouped.figure)
        .into_iter()
        .filter(|r| r.4.is_some_and(|c| axes::GROUP_COLORS[..2].contains(&c)))
        .filter(|r| r.2 > 5.0 && (r.2 - 14.0).abs() > 0.01)
        .collect();
    assert_eq!(bars.len(), 6);
    // Side by side, not on top of each other.
    assert!(bars[0].0 != bars[1].0);
}

#[test]
fn group_colours_are_used_for_groups() {
    let built = draw(json!({
        "data": {"name": "profile"},
        "transform": [{"filter": "datum.feature == 'Prevotella'"}],
        "mark": "point",
        "encoding": {
            "x": {"field": "count", "type": "quantitative"},
            "y": {"field": "proportion", "type": "quantitative"},
            "color": {"field": "group", "type": "nominal"}
        }
    }))
    .unwrap();
    let strokes: Vec<Color> = built
        .figure
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Circle {
                stroke: Some(st),
                fill: None,
                ..
            } => Some(st.color),
            _ => None,
        })
        .collect();
    assert!(strokes.contains(&Color::rgb(1, 2, 3)) && strokes.contains(&Color::rgb(4, 5, 6)));
}

#[test]
fn aggregates_in_the_encoding_and_lookups() {
    let built = draw(json!({
        "data": {"name": "profile"},
        "transform": [{"lookup": "sample", "from": {"data": {"name": "samples"}, "key": "sample", "fields": ["Site"]}}],
        "mark": "bar",
        "encoding": {
            "x": {"field": "Site", "type": "nominal"},
            "y": {"aggregate": "mean", "field": "count", "type": "quantitative"}
        }
    }))
    .unwrap();
    let t = texts(&built.figure);
    assert!(t.contains(&"North".to_string()) && t.contains(&"South".to_string()));
    assert!(t.contains(&"Mean of count".to_string()));
    assert!(built
        .figure
        .regions
        .iter()
        .any(|r| r.tooltip.contains("Mean of count: 50")));
}

#[test]
fn layers_share_axes() {
    let built = draw(json!({
        "data": values(),
        "layer": [
            {"mark": "bar", "encoding": {"x": {"field": "taxon"}, "y": {"field": "abundance"}}},
            {"mark": {"type": "text", "dy": -6}, "encoding": {"x": {"field": "taxon"}, "y": {"field": "abundance"}, "text": {"field": "abundance"}}},
            {"mark": {"type": "rule", "strokeDash": [4, 2], "color": "red"}, "encoding": {"y": {"datum": 4}}}
        ]
    }))
    .unwrap();
    let f = &built.figure;
    assert!(texts(f).contains(&"7".to_string()));
    let dashed = f.shapes.iter().any(|s| matches!(s, Shape::Line { stroke, .. } if stroke.dash == Some((4.0, 2.0)) && stroke.color == Color::rgb(255, 0, 0)));
    assert!(dashed);
}

#[test]
fn logarithmic_and_colour_scales() {
    let built = draw(json!({
        "data": {"values": [{"a": 0.01, "b": 1, "v": 1}, {"a": 50, "b": 2, "v": 9}, {"a": 0, "b": 3, "v": 5}]},
        "mark": "circle",
        "encoding": {
            "x": {"field": "a", "type": "quantitative", "scale": {"type": "log"}},
            "y": {"field": "b", "type": "quantitative"},
            "color": {"field": "v", "type": "quantitative", "scale": {"scheme": "blues"}}
        }
    }))
    .unwrap();
    let t = texts(&built.figure);
    assert!(t.contains(&"0.01".to_string()) && t.contains(&"100".to_string()));
    assert!(built
        .notes
        .iter()
        .any(|n| n.contains("1 marks were left out")));
}

#[test]
fn histograms_lines_areas_and_heatmaps() {
    let hist = draw(json!({
        "data": {"name": "profile"},
        "mark": "bar",
        "encoding": {
            "x": {"field": "count", "bin": {"step": 20}},
            "y": {"aggregate": "count"}
        }
    }))
    .unwrap();
    assert!(texts(&hist.figure).contains(&"Count of Records".to_string()));
    // 5 to 60 in bins of 20: 0 to 20, 20 to 40 (empty) and 40 to 60 (with
    // the largest value).
    assert_eq!(
        rects(&hist.figure)
            .iter()
            .filter(|r| r.4 == Some(DEFAULT_COLOR))
            .count(),
        2
    );

    let line = draw(json!({
        "data": {"name": "profile"},
        "mark": {"type": "line", "point": true},
        "encoding": {
            "x": {"field": "sample"},
            "y": {"field": "count"},
            "color": {"field": "feature"}
        }
    }))
    .unwrap();
    let lines = line
        .figure
        .shapes
        .iter()
        .filter(|s| matches!(s, Shape::Line { points, .. } if points.len() == 3))
        .count();
    assert_eq!(lines, 2);

    let area = draw(json!({
        "data": {"name": "profile"},
        "mark": "area",
        "encoding": {
            "x": {"field": "sample"},
            "y": {"field": "proportion", "stack": "normalize"},
            "color": {"field": "feature"}
        }
    }))
    .unwrap();
    assert_eq!(
        area.figure
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Area { .. }))
            .count(),
        2
    );

    let heat = draw(json!({
        "data": {"name": "profile"},
        "mark": "rect",
        "encoding": {
            "x": {"field": "sample"},
            "y": {"field": "feature"},
            "color": {"field": "proportion", "type": "quantitative"}
        }
    }))
    .unwrap();
    assert!(rects(&heat.figure).len() >= 6);
    assert!(heat
        .figure
        .regions
        .iter()
        .any(|r| r.item == "Prevotella" && r.tooltip.contains("proportion: 80")));
}

#[test]
fn box_plots_and_error_bars() {
    let boxes = draw(json!({
        "data": {"name": "profile"},
        "mark": "boxplot",
        "encoding": {
            "x": {"field": "feature"},
            "y": {"field": "proportion"},
            "color": {"field": "feature"}
        }
    }))
    .unwrap();
    let tips: Vec<&str> = boxes
        .figure
        .regions
        .iter()
        .map(|r| r.tooltip.as_str())
        .collect();
    assert!(
        tips.iter()
            .any(|t| t.contains("median: 75") && t.contains("samples: 3")),
        "{tips:?}"
    );

    let bars = draw(json!({
        "data": {"name": "profile"},
        "layer": [
            {"mark": "bar", "encoding": {"x": {"field": "feature"}, "y": {"aggregate": "mean", "field": "proportion"}}},
            {"mark": {"type": "errorbar", "extent": "stdev", "ticks": true}, "encoding": {"x": {"field": "feature"}, "y": {"field": "proportion"}}}
        ]
    }))
    .unwrap();
    assert!(bars
        .figure
        .regions
        .iter()
        .any(|r| r.tooltip.contains("mean ± standard deviation")));
}

#[test]
fn mistakes_are_explained() {
    let error = |spec: Value| draw(spec).err().unwrap();
    assert!(error(json!({"data": values(), "mark": "arc"})).contains("stacked bar"));
    assert!(error(json!({"data": values(), "facet": {}, "mark": "bar"})).contains("single panel"));
    assert!(error(json!({"data": {"name": "otu"}, "mark": "bar"}))
        .contains("available: profile, samples"));
    assert!(
        error(json!({"data": values(), "mark": "bar", "encoding": {"x": {"field": "nope"}}}))
            .contains("no field 'nope'")
    );
    assert!(error(json!({"data": {"url": "x.csv"}, "mark": "bar"})).contains("URL"));
    assert!(error(json!({"mark": "bar"})).contains("needs data"));
    assert!(error(json!({"data": values(), "layer": [{"mark": "bar", "transform": [{"filter": "taxon == 1"}]}]}))
        .starts_with("transform 1"));
    let noted = draw(json!({"data": values(), "mark": {"type": "bar", "wobble": 3}, "encoding": {"x": {"field": "taxon"}, "y": {"field": "abundance"}}})).unwrap();
    assert!(noted.notes.iter().any(|n| n.contains("wobble")));
}

#[test]
fn names_of_data_used() {
    let spec = json!({
        "data": {"name": "profile"},
        "transform": [{"lookup": "sample", "from": {"data": {"name": "samples"}, "key": "sample"}}],
        "layer": [{"mark": "bar"}, {"data": {"name": "results"}, "mark": "point"}]
    });
    let mut names = data_names(&spec);
    names.sort();
    assert_eq!(names, ["profile", "results", "samples"]);
}

#[test]
fn colours_from_css() {
    assert_eq!(parse_color("#f00"), Some(Color::rgb(255, 0, 0)));
    assert_eq!(parse_color("steelblue"), Some(Color::rgb(70, 130, 180)));
    assert_eq!(parse_color("rgb(1, 2, 3)"), Some(Color::rgb(1, 2, 3)));
    assert_eq!(parse_color("nonsense"), None);
}

/// Pictures of example plots, written to the folder named by
/// `STAMP_CUSTOM_PLOTS`, for looking at: `STAMP_CUSTOM_PLOTS=/some/dir
/// cargo test -p stamp-plot --release --lib custom::tests::gallery --
/// --ignored`.
#[test]
#[ignore]
fn gallery() {
    let Ok(dir) = std::env::var("STAMP_CUSTOM_PLOTS") else {
        return;
    };
    let examples = [
        (
            "bar",
            json!({"title": "Abundance", "data": values(), "mark": "bar",
            "encoding": {"x": {"field": "taxon"}, "y": {"field": "abundance"}}}),
        ),
        (
            "stacked",
            json!({"data": {"name": "profile"}, "mark": "bar",
            "encoding": {"x": {"field": "sample"}, "y": {"field": "proportion", "title": "Proportion (%)"},
                         "color": {"field": "feature"}}}),
        ),
        (
            "grouped",
            json!({"data": {"name": "profile"}, "mark": "bar",
            "encoding": {"x": {"field": "sample"}, "xOffset": {"field": "feature"},
                         "y": {"field": "count"}, "color": {"field": "feature"}}}),
        ),
        (
            "scatter",
            json!({"data": {"name": "profile"}, "mark": "point",
            "encoding": {"x": {"field": "count"}, "y": {"field": "proportion"},
                         "color": {"field": "group"}, "shape": {"field": "feature"}}}),
        ),
        (
            "heatmap",
            json!({"data": {"name": "profile"}, "mark": "rect",
            "encoding": {"x": {"field": "sample"}, "y": {"field": "feature"},
                         "color": {"field": "proportion", "type": "quantitative"}}}),
        ),
        (
            "boxplot",
            json!({"data": {"name": "profile"}, "mark": "boxplot",
            "encoding": {"x": {"field": "feature"}, "y": {"field": "proportion"}, "color": {"field": "group"}, "xOffset": {"field": "group"}}}),
        ),
        (
            "layers",
            json!({"data": values(), "layer": [
            {"mark": "bar", "encoding": {"x": {"field": "taxon"}, "y": {"field": "abundance"}}},
            {"mark": {"type": "text", "dy": -6}, "encoding": {"x": {"field": "taxon"}, "y": {"field": "abundance"}, "text": {"field": "abundance"}}},
            {"mark": {"type": "rule", "strokeDash": [4, 2], "color": "red"}, "encoding": {"y": {"datum": 4}}}]}),
        ),
        (
            "line",
            json!({"data": {"name": "profile"}, "mark": {"type": "line", "point": true},
            "encoding": {"x": {"field": "sample"}, "y": {"field": "count"}, "color": {"field": "feature"}}}),
        ),
        (
            "area",
            json!({"data": {"name": "profile"}, "mark": "area",
            "encoding": {"x": {"field": "sample"}, "y": {"field": "proportion", "stack": "normalize"}, "color": {"field": "feature"}}}),
        ),
        (
            "horizontal",
            json!({"data": {"name": "profile"}, "mark": "bar",
            "encoding": {"y": {"field": "sample", "sort": "-x"}, "x": {"aggregate": "sum", "field": "count"}, "color": {"field": "feature"}}}),
        ),
        (
            "histogram",
            json!({"data": {"name": "profile"}, "mark": "bar",
            "encoding": {"x": {"field": "count", "bin": {"step": 20}}, "y": {"aggregate": "count"}}}),
        ),
        (
            "errorbars",
            json!({"data": {"name": "profile"}, "layer": [
            {"mark": "bar", "encoding": {"x": {"field": "feature"}, "y": {"aggregate": "mean", "field": "proportion"}}},
            {"mark": {"type": "errorbar", "extent": "stdev", "ticks": true}, "encoding": {"x": {"field": "feature"}, "y": {"field": "proportion"}}}]}),
        ),
    ];
    for (name, spec) in examples {
        let built = draw(spec).unwrap_or_else(|e| panic!("{name}: {e}"));
        let png = crate::export::to_png(&built.figure, 150.0).unwrap();
        std::fs::write(format!("{dir}/{name}.png"), png).unwrap();
    }
}
