//! Rendering the interface to an image without a window or a GPU, for
//! automated tests and screenshots.

use std::collections::HashMap;

use eframe::egui::{self, epaint, Color32, Pos2, Rect, TextureId, Vec2};

/// An RGBA image, eight bits per channel.
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color32>,
}

impl Image {
    pub fn pixel(&self, x: usize, y: usize) -> Color32 {
        self.pixels[y * self.width + x]
    }

    /// Encode as a PNG file.
    pub fn to_png(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, self.width as u32, self.height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("PNG header");
            let data: Vec<u8> = self
                .pixels
                .iter()
                .flat_map(|p| p.to_srgba_unmultiplied())
                .collect();
            writer.write_image_data(&data).expect("PNG data");
        }
        bytes
    }
}

/// Runs an interface frame by frame and paints the result in software.
pub struct Renderer {
    pub ctx: egui::Context,
    size: Vec2,
    pixels_per_point: f32,
    textures: HashMap<TextureId, epaint::ColorImage>,
    pointer: Option<Pos2>,
    /// Whether the pointer position still has to be told to egui.
    pointer_moved: bool,
    time: f64,
}

impl Renderer {
    /// A renderer for a window of `width` by `height` points.
    pub fn new(width: f32, height: f32, pixels_per_point: f32) -> Renderer {
        let ctx = egui::Context::default();
        // Anti-aliasing is done with thin translucent triangles that a
        // one-sample-per-pixel painter turns into dotted edges.
        ctx.tessellation_options_mut(|options| options.feathering = false);
        Renderer {
            ctx,
            size: Vec2::new(width, height),
            pixels_per_point,
            textures: HashMap::new(),
            pointer: None,
            pointer_moved: false,
            time: 0.0,
        }
    }

    /// Put the pointer at a position, in points, for the following frames.
    pub fn move_pointer(&mut self, position: Pos2) {
        self.pointer = Some(position);
        self.pointer_moved = true;
    }

    fn input(&mut self, events: Vec<egui::Event>) -> egui::RawInput {
        self.time += 1.0 / 60.0;
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.pixels_per_point);
        // Report a move once, so that egui sees the pointer come to rest.
        if let (Some(position), true) = (self.pointer, self.pointer_moved) {
            input.events.insert(0, egui::Event::PointerMoved(position));
            self.pointer_moved = false;
        }
        input
    }

    /// Run one frame of `ui` and return the tessellated output.
    fn frame(
        &mut self,
        events: Vec<egui::Event>,
        ui: &mut dyn FnMut(&mut egui::Ui),
    ) -> Vec<epaint::ClippedPrimitive> {
        let input = self.input(events);
        let mut output = self.ctx.run_ui(input, |root| ui(root));
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                let egui::ImageData::Color(image) = &delta.image;
                match delta.pos {
                    None => {
                        self.textures.insert(*id, (**image).clone());
                    }
                    Some([x0, y0]) => {
                        if let Some(texture) = self.textures.get_mut(id) {
                            for y in 0..image.size[1] {
                                for x in 0..image.size[0] {
                                    texture.pixels[(y0 + y) * texture.size[0] + x0 + x] =
                                        image.pixels[y * image.size[0] + x];
                                }
                            }
                        }
                    }
                }
            }
        }
        for id in &output.textures_delta.free {
            self.textures.remove(id);
        }
        // egui insists that every texture update is acknowledged.
        output.textures_delta.clear();
        let shapes = std::mem::take(&mut output.shapes);
        self.ctx.tessellate(shapes, output.pixels_per_point)
    }

    /// Run frames of `ui` without painting, to let layout and animations
    /// settle.
    pub fn run(&mut self, frames: usize, mut ui: impl FnMut(&mut egui::Ui)) {
        for _ in 0..frames {
            self.frame(Vec::new(), &mut ui);
        }
    }

    /// Click the primary button at a position, in points.
    pub fn click(&mut self, position: Pos2, mut ui: impl FnMut(&mut egui::Ui)) {
        self.move_pointer(position);
        let button = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        self.frame(vec![button(true)], &mut ui);
        self.frame(vec![button(false)], &mut ui);
        self.frame(Vec::new(), &mut ui);
    }

    /// Press and release a key while Ctrl (Command on macOS) is held, as for
    /// a keyboard shortcut.
    pub fn press_shortcut(&mut self, key: egui::Key, mut ui: impl FnMut(&mut egui::Ui)) {
        let modifiers = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame(vec![event(true)], &mut ui);
        self.frame(vec![event(false)], &mut ui);
    }

    /// Run one frame with these input events (typed text, keys, pastes),
    /// then one without, as when a user types.
    pub fn send_events(&mut self, events: Vec<egui::Event>, mut ui: impl FnMut(&mut egui::Ui)) {
        self.frame(events, &mut ui);
        self.frame(Vec::new(), &mut ui);
    }

    /// Run one frame of `ui` and paint it.
    pub fn render(&mut self, mut ui: impl FnMut(&mut egui::Ui)) -> Image {
        let primitives = self.frame(Vec::new(), &mut ui);
        let scale = self.pixels_per_point;
        let width = (self.size.x * scale).round() as usize;
        let height = (self.size.y * scale).round() as usize;
        let mut canvas = vec![[0.0f32; 4]; width * height];

        for clipped in &primitives {
            let epaint::Primitive::Mesh(mesh) = &clipped.primitive else {
                continue;
            };
            let texture = self.textures.get(&mesh.texture_id);
            let clip = clipped.clip_rect;
            let (clip_x0, clip_y0) = (
                (clip.min.x * scale).floor().max(0.0) as i64,
                (clip.min.y * scale).floor().max(0.0) as i64,
            );
            let (clip_x1, clip_y1) = (
                ((clip.max.x * scale).ceil() as i64).min(width as i64),
                ((clip.max.y * scale).ceil() as i64).min(height as i64),
            );

            for triangle in mesh.indices.as_chunks::<3>().0 {
                let v = [
                    mesh.vertices[triangle[0] as usize],
                    mesh.vertices[triangle[1] as usize],
                    mesh.vertices[triangle[2] as usize],
                ];
                let p = v.map(|vertex| (vertex.pos.x * scale, vertex.pos.y * scale));
                let area =
                    (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
                if area.abs() < 1e-9 {
                    continue;
                }
                let min_x = (p.iter().map(|q| q.0).fold(f32::INFINITY, f32::min).floor() as i64)
                    .max(clip_x0);
                let max_x = (p
                    .iter()
                    .map(|q| q.0)
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil() as i64)
                    .min(clip_x1);
                let min_y = (p.iter().map(|q| q.1).fold(f32::INFINITY, f32::min).floor() as i64)
                    .max(clip_y0);
                let max_y = (p
                    .iter()
                    .map(|q| q.1)
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil() as i64)
                    .min(clip_y1);
                let colors = v.map(|vertex| vertex.color.to_normalized_gamma_f32());

                for y in min_y..max_y {
                    for x in min_x..max_x {
                        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                        let w0 =
                            ((p[1].0 - px) * (p[2].1 - py) - (p[2].0 - px) * (p[1].1 - py)) / area;
                        let w1 =
                            ((p[2].0 - px) * (p[0].1 - py) - (p[0].0 - px) * (p[2].1 - py)) / area;
                        let w2 = 1.0 - w0 - w1;
                        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                            continue;
                        }
                        let mut color = [0.0f32; 4];
                        for (c, slot) in color.iter_mut().enumerate() {
                            *slot = w0 * colors[0][c] + w1 * colors[1][c] + w2 * colors[2][c];
                        }
                        if let Some(texture) = texture {
                            let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                            let t = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                            let tx =
                                ((u * texture.size[0] as f32) as usize).min(texture.size[0] - 1);
                            let ty =
                                ((t * texture.size[1] as f32) as usize).min(texture.size[1] - 1);
                            let texel =
                                texture.pixels[ty * texture.size[0] + tx].to_normalized_gamma_f32();
                            for (slot, t) in color.iter_mut().zip(texel) {
                                *slot *= t;
                            }
                        }
                        // Colours are premultiplied: source over destination.
                        let destination = &mut canvas[y as usize * width + x as usize];
                        for c in 0..4 {
                            destination[c] = color[c] + destination[c] * (1.0 - color[3]);
                        }
                    }
                }
            }
        }

        let pixels = canvas
            .into_iter()
            .map(|c| {
                let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                Color32::from_rgba_premultiplied(byte(c[0]), byte(c[1]), byte(c[2]), byte(c[3]))
            })
            .collect();
        Image {
            width,
            height,
            pixels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paints_shapes_and_text() {
        let mut renderer = Renderer::new(120.0, 60.0, 1.0);
        let ui = |ui: &mut egui::Ui| {
            let painter = ui.painter();
            painter.rect_filled(
                Rect::from_min_size(Pos2::ZERO, Vec2::new(120.0, 60.0)),
                0.0,
                Color32::WHITE,
            );
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(30.0, 30.0)),
                0.0,
                Color32::RED,
            );
            painter.text(
                Pos2::new(60.0, 20.0),
                egui::Align2::LEFT_TOP,
                "Hi",
                egui::FontId::proportional(24.0),
                Color32::BLACK,
            );
        };
        renderer.run(2, ui);
        let image = renderer.render(ui);
        assert_eq!((image.width, image.height), (120, 60));
        assert_eq!(image.pixel(25, 25), Color32::RED);
        assert_eq!(image.pixel(5, 5), Color32::WHITE);
        let dark = (60..110)
            .flat_map(|x| (15..50).map(move |y| (x, y)))
            .filter(|&(x, y)| image.pixel(x, y).g() < 100)
            .count();
        assert!(dark > 30, "text was not drawn ({dark} dark pixels)");
        assert!(image.to_png().starts_with(b"\x89PNG"));
    }

    #[test]
    fn clicks_reach_widgets() {
        let mut renderer = Renderer::new(200.0, 100.0, 1.0);
        let mut clicks = 0;
        let mut position = Pos2::ZERO;
        renderer.run(2, |ui| {
            position = ui.button("Press").rect.center();
        });
        renderer.click(position, |ui| {
            if ui.button("Press").clicked() {
                clicks += 1;
            }
        });
        assert_eq!(clicks, 1);
    }
}
