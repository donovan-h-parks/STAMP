//! Drawing a [`Figure`] with egui, so that plots are sharp at any zoom and
//! can respond to the pointer.

use eframe::egui::{self, epaint, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use stamp_plot::figure::{Color, Figure, HAlign, Shape};
use stamp_plot::text;

/// Name of the egui font family holding the plot font.
pub const PLOT_FONT: &str = "stamp-plot";

/// Register the plot font with egui. Call once at start-up.
pub fn install_plot_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        PLOT_FONT.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(text::FONT_DATA)),
    );
    fonts.families.insert(
        FontFamily::Name(PLOT_FONT.into()),
        vec![PLOT_FONT.to_owned()],
    );
    crate::icons::add_to_fonts(&mut fonts);
    ctx.set_fonts(fonts);
}

fn color32(color: Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

/// What the pointer did on a figure this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FigureResponse {
    /// The item under the pointer.
    pub hovered: Option<String>,
    /// The item that was clicked.
    pub clicked: Option<String>,
    /// Where the figure was drawn, and the whole area it was centred in.
    pub placement: Option<(Rect, Rect)>,
}

/// Show a figure at `scale` screen points per figure point, with tooltips for
/// its regions. The figure is centred in `space`, the room it has been given;
/// where it is larger than that room it takes the size it needs.
pub fn show(ui: &mut egui::Ui, figure: &Figure, scale: f32, space: Vec2) -> FigureResponse {
    let size = Vec2::new(figure.width as f32 * scale, figure.height as f32 * scale);
    let (area, response) = ui.allocate_exact_size(space.max(size), Sense::click());
    // Whole pixels keep the plot's lines crisp.
    let rect = Rect::from_min_size((area.min + (area.size() - size) / 2.0).round(), size);
    let mut outcome = FigureResponse {
        placement: Some((rect, area)),
        ..Default::default()
    };
    if !ui.is_rect_visible(rect) {
        return outcome;
    }
    paint(&ui.painter_at(rect), rect.min, scale, figure);

    if let Some(pointer) = response.hover_pos() {
        let local = (pointer - rect.min) / scale;
        if let Some(region) = figure.region_at(f64::from(local.x), f64::from(local.y)) {
            outcome.hovered = Some(region.item.clone());
            if response.clicked() {
                outcome.clicked = Some(region.item.clone());
            }
            if !region.tooltip.is_empty() {
                response.on_hover_text_at_pointer(region.tooltip.clone());
            }
        }
    }
    outcome
}

/// Paint a figure with its top-left corner at `origin`.
pub fn paint(painter: &egui::Painter, origin: Pos2, scale: f32, figure: &Figure) {
    let at = |x: f64, y: f64| Pos2::new(origin.x + x as f32 * scale, origin.y + y as f32 * scale);
    let stroke =
        |s: &stamp_plot::Stroke| Stroke::new((s.width as f32 * scale).max(0.5), color32(s.color));

    let page = Rect::from_min_size(
        origin,
        Vec2::new(figure.width as f32 * scale, figure.height as f32 * scale),
    );
    painter.rect_filled(page, 0.0, color32(figure.background));

    for shape in &figure.shapes {
        match shape {
            Shape::Line {
                points,
                stroke: line,
            } => {
                let points: Vec<Pos2> = points.iter().map(|&(x, y)| at(x, y)).collect();
                match line.dash {
                    Some((dash, gap)) => {
                        painter.extend(epaint::Shape::dashed_line(
                            &points,
                            stroke(line),
                            dash as f32 * scale,
                            gap as f32 * scale,
                        ));
                    }
                    None => {
                        painter.add(epaint::Shape::line(points, stroke(line)));
                    }
                }
            }
            Shape::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke: outline,
            } => {
                let rect = Rect::from_min_size(
                    at(*x, *y),
                    Vec2::new(*width as f32 * scale, *height as f32 * scale),
                );
                if let Some(fill) = fill {
                    painter.rect_filled(rect, 0.0, color32(*fill));
                }
                if let Some(outline) = outline {
                    painter.rect_stroke(rect, 0.0, stroke(outline), egui::StrokeKind::Middle);
                }
            }
            Shape::Circle {
                cx,
                cy,
                radius,
                fill,
                stroke: outline,
            } => {
                let (centre, r) = (at(*cx, *cy), *radius as f32 * scale);
                if let Some(fill) = fill {
                    painter.circle_filled(centre, r, color32(*fill));
                }
                if let Some(outline) = outline {
                    painter.circle_stroke(centre, r, stroke(outline));
                }
            }
            Shape::Polygon {
                points,
                fill,
                stroke: outline,
            } => {
                let points: Vec<Pos2> = points.iter().map(|&(x, y)| at(x, y)).collect();
                if let Some(fill) = fill {
                    // Polygons here are star-shaped about their centre, so a
                    // triangle fan fills them even when they are not convex.
                    let n = points.len() as f32;
                    let centre = Pos2::new(
                        points.iter().map(|p| p.x).sum::<f32>() / n,
                        points.iter().map(|p| p.y).sum::<f32>() / n,
                    );
                    let mut mesh = epaint::Mesh::default();
                    let fill = color32(*fill);
                    mesh.colored_vertex(centre, fill);
                    for point in &points {
                        mesh.colored_vertex(*point, fill);
                    }
                    for i in 0..points.len() as u32 {
                        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % points.len() as u32);
                    }
                    painter.add(epaint::Shape::mesh(mesh));
                }
                if let Some(outline) = outline {
                    painter.add(epaint::Shape::closed_line(points, stroke(outline)));
                }
            }
            Shape::Text {
                x,
                y,
                text: content,
                style,
            } => {
                let size = style.size as f32 * scale;
                let font = FontId::new(size, FontFamily::Name(PLOT_FONT.into()));
                let galley = painter.layout_no_wrap(content.clone(), font, color32(style.color));

                // Where the text's top-left corner sits relative to its
                // anchor, before rotation. Widths come from the same metrics
                // as the exported files; egui places the top of a row at the
                // font's ascender line.
                let width = text::width(content, style.size) as f32 * scale;
                let dx = match style.h_align {
                    HAlign::Left => 0.0,
                    HAlign::Center => -width / 2.0,
                    HAlign::Right => -width,
                };
                let ascender = 0.928 * size;
                let dy = style.baseline_offset() as f32 * scale - ascender;

                // Counter-clockwise degrees to egui's clockwise radians.
                let angle = -(style.rotation as f32).to_radians();
                let (sin, cos) = angle.sin_cos();
                let offset = Vec2::new(dx * cos - dy * sin, dx * sin + dy * cos);
                let shape =
                    epaint::TextShape::new(at(*x, *y) + offset, galley, color32(style.color))
                        .with_angle(angle);
                painter.add(shape);
            }
        }
    }
}
