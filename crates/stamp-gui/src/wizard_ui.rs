//! The window of the analysis wizard: the questions of `wizard.rs` as
//! cards, each with a small drawing of what it means and an "i" button for
//! a longer explanation. Every choice is always drawn; those the loaded
//! data cannot support are greyed and say why. The same window shows the
//! tests "at a glance": a table with a row for each test.

use eframe::egui;
use egui::{pos2, vec2, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};

use crate::app::StampApp;
use crate::icons;
use crate::plots::PlotKind;
use crate::wizard::{
    Availability, Choice, Comparison, Data, Design, Facts, Goal, Question, Step, GLANCE,
};

/// Size of a card.
const CARD: egui::Vec2 = vec2(236.0, 172.0);
/// Size of the description on a card, in points.
const SMALL: f32 = 11.5;
/// Height of the drawing at the top of a card.
const PICTURE: f32 = 56.0;

/// Colours of the drawings: two groups, two more, and a neutral ink.
const BLUE: Color32 = Color32::from_rgb(86, 148, 204);
const ORANGE: Color32 = Color32::from_rgb(235, 140, 60);
const GREEN: Color32 = Color32::from_rgb(110, 170, 90);
const PURPLE: Color32 = Color32::from_rgb(150, 120, 190);
const RED: Color32 = Color32::from_rgb(210, 80, 70);

pub fn show(app: &mut StampApp, ctx: &egui::Context) {
    if !app.show_wizard {
        return;
    }
    let mut open = true;
    egui::Window::new("Analysis wizard")
        .open(&mut open)
        .default_size([780.0, 720.0])
        .min_width(520.0)
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| contents(app, ui));
        });
    if !open {
        app.show_wizard = false;
    }
}

fn contents(app: &mut StampApp, ui: &mut egui::Ui) {
    let facts = app.wizard_facts();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.wizard.glance, false, "Step by step")
            .on_hover_text("A few questions that lead to a test");
        ui.selectable_value(&mut app.wizard.glance, true, "At a glance")
            .on_hover_text(
                "Every test in one table: when to use it, what it does, what it assumes",
            );
    });
    ui.separator();
    if app.wizard.glance {
        glance(app, ui, &facts);
        return;
    }
    trail(app, ui);
    ui.separator();
    let step = app.wizard.step();
    ui.add_space(4.0);
    ui.label(RichText::new(step.heading()).heading().strong());
    match step {
        Step::Result => result(app, ui, &facts),
        _ => {
            let hint = match step {
                Step::Comparison => format!(
                    "Choices that the loaded data cannot support are greyed, with the reason. \
                     The {} on a choice explains it.",
                    icons::INFO
                ),
                Step::Question => "Choose the question closest to yours.".to_string(),
                Step::Design => "This decides whether each subject is compared with itself or \
                    the groups with each other."
                    .to_string(),
                _ => "Choose the description closest to your data and your question.".to_string(),
            };
            ui.label(RichText::new(hint).weak());
            ui.add_space(4.0);
            explanation(app, ui);
            if step == Step::Comparison {
                data_summary(app, ui, &facts);
            }
            if step == Step::Data {
                subject_field(app, ui, &facts);
            }
            cards(app, ui, step, &facts);
        }
    }
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(step != Step::Comparison, egui::Button::new("Back"))
            .clicked()
        {
            app.wizard.back();
        }
        if ui
            .add_enabled(step != Step::Comparison, egui::Button::new("Start over"))
            .clicked()
        {
            app.wizard.restart();
        }
    });
}

/// Width and height of the drawing of a test in the table.
const GLANCE_PICTURE: egui::Vec2 = vec2(60.0, 40.0);
/// Width of the column that names the test.
const GLANCE_TEST: f32 = 168.0;

/// The tests at a glance: a row for each, under what it is for, with when
/// to use it, what it does and what it assumes. Tests the loaded data
/// cannot support are greyed and say why; the others lead to their
/// recommendation.
fn glance(app: &mut StampApp, ui: &mut egui::Ui, facts: &Facts) {
    ui.add_space(4.0);
    ui.label(
        RichText::new("Which test for which question?")
            .heading()
            .strong(),
    );
    ui.label(
        RichText::new(
            "Tests that the loaded data cannot support are greyed, with the reason. \"Show\" \
             explains a test further and can set it up.",
        )
        .weak(),
    );
    ui.add_space(12.0);
    if facts.samples > 0 {
        data_summary(app, ui, facts);
        ui.add_space(12.0);
    }

    let gap = 10.0;
    let column =
        ((ui.available_width() - GLANCE_PICTURE.x - GLANCE_TEST - 4.0 * gap) / 3.0).max(110.0);
    let cell = |ui: &mut egui::Ui, width: f32, add: &mut dyn FnMut(&mut egui::Ui)| {
        ui.allocate_ui_with_layout(
            vec2(width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 2.0;
                add(ui);
            },
        );
    };
    let text = |ui: &mut egui::Ui, text: RichText| {
        ui.add(egui::Label::new(text.size(SMALL)).wrap());
    };
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_exact_size(vec2(GLANCE_PICTURE.x, 0.0), Sense::hover());
        for (heading, width) in [
            ("Test", GLANCE_TEST),
            ("Use it when", column),
            ("What it does", column),
            ("What it assumes", column),
        ] {
            cell(ui, width, &mut |ui| {
                text(ui, RichText::new(heading).strong())
            });
        }
    });

    let visuals = ui.visuals().clone();
    let mut follow = None;
    for goal in Goal::ALL {
        ui.add_space(6.0);
        ui.label(
            RichText::new(goal.heading())
                .strong()
                .color(visuals.selection.stroke.color),
        );
        ui.separator();
        for row in GLANCE.iter().filter(|row| row.goal == goal) {
            let availability = row.comparison(facts);
            let available = availability.is_ok();
            let dim = |text: &str| {
                let text = RichText::new(text);
                if available {
                    text
                } else {
                    text.weak()
                }
            };
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let (picture, _) = ui.allocate_exact_size(GLANCE_PICTURE, Sense::hover());
                draw(
                    &ui.painter_at(picture),
                    picture,
                    row.picture(),
                    &Pens::new(
                        visuals.text_color(),
                        visuals.window_fill,
                        if available { 1.0 } else { 0.3 },
                    ),
                );
                cell(ui, GLANCE_TEST, &mut |ui| {
                    ui.add(egui::Label::new(dim(row.test).strong()).wrap());
                    match &availability {
                        Ok(_) => {
                            ui.add_space(6.0);
                            if ui
                                .small_button("Show")
                                .on_hover_text(
                                    "Why and when this test is recommended, its plots, and the \
                                     button that sets it up",
                                )
                                .clicked()
                            {
                                follow = Some(row);
                            }
                        }
                        Err(reason) => {
                            text(ui, RichText::new(reason).color(visuals.warn_fg_color));
                        }
                    }
                });
                for words in [row.when, row.does, row.assumes] {
                    cell(ui, column, &mut |ui| text(ui, dim(words)));
                }
            });
            ui.add_space(8.0);
        }
    }
    if let Some(row) = follow {
        app.wizard.follow(row, facts);
    }
}

/// The answers so far; clicking one returns to its question.
fn trail(app: &mut StampApp, ui: &mut egui::Ui) {
    let steps = app.wizard.steps();
    let current = app.wizard.step();
    let mut go_to = None;
    ui.horizontal_wrapped(|ui| {
        for (index, step) in steps.iter().enumerate() {
            if index > 0 {
                ui.weak("\u{203A}");
            }
            let text = match app.wizard.answer(*step) {
                Some(answer) => format!("{}. {}: {}", index + 1, step.name(), short(answer)),
                None => format!("{}. {}", index + 1, step.name()),
            };
            if *step == current {
                ui.label(RichText::new(text).strong());
            } else if ui
                .link(text)
                .on_hover_text("Return to this question")
                .clicked()
            {
                go_to = Some(*step);
            }
        }
    });
    if let Some(step) = go_to {
        app.wizard.go_to(step);
    }
}

/// A choice in a few words, for the trail of answers.
fn short(choice: Choice) -> &'static str {
    match choice {
        Choice::Design(Design::Independent) => "Different subjects",
        Choice::Design(Design::Matched) => "The same subjects",
        Choice::Data(Data::Typical) => "Most data",
        Choice::Data(Data::SmallOrExtreme) => "Small groups or extreme samples",
        Choice::Data(Data::MostlyAbsent) => "Features absent from most samples",
        Choice::Data(Data::Presence) => "Found in more samples",
        Choice::Data(Data::Dominated) => "A few features dominate",
        other => other.title(),
    }
}

/// What is loaded, and the group field, which decides what can be compared.
fn data_summary(app: &mut StampApp, ui: &mut egui::Ui, facts: &Facts) {
    if facts.samples == 0 {
        return;
    }
    let fields: Vec<String> = app
        .session
        .metadata
        .as_ref()
        .map(|m| m.fields().to_vec())
        .unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        ui.label(format!("Loaded: {} samples.", facts.samples));
        if fields.is_empty() {
            ui.weak("No metadata: only two samples can be compared.");
            return;
        }
        ui.label("Group field:");
        let mut field = facts.group_field.clone().unwrap_or_default();
        let before = field.clone();
        egui::ComboBox::from_id_salt("wizard-group-field")
            .selected_text(field.clone())
            .show_ui(ui, |ui| {
                for name in &fields {
                    ui.selectable_value(&mut field, name.clone(), name);
                }
            });
        if field != before {
            app.session.set_group_field(&field);
        }
        let groups: Vec<String> = facts
            .groups
            .iter()
            .map(|(name, size)| format!("{name} ({size})"))
            .collect();
        ui.weak(match groups.len() {
            0 => "No groups with samples.".to_string(),
            n if n > 6 => format!("{n} groups."),
            _ => format!("Groups: {}.", groups.join(", ")),
        });
    });
    ui.add_space(4.0);
}

/// The field that says which samples came from the same subject, for a
/// matched design.
fn subject_field(app: &mut StampApp, ui: &mut egui::Ui, facts: &Facts) {
    if app.wizard.design != Some(Design::Matched) {
        return;
    }
    let Some(comparison) = app.wizard.comparison else {
        return;
    };
    let fields = facts.matching_fields(comparison);
    ui.horizontal(|ui| {
        ui.label("Subjects are named by:");
        let mut field = app.wizard.subject_field.clone().unwrap_or_default();
        egui::ComboBox::from_id_salt("wizard-subject-field")
            .selected_text(field.clone())
            .show_ui(ui, |ui| {
                for (name, subjects) in fields {
                    ui.selectable_value(
                        &mut field,
                        name.clone(),
                        format!("{name} ({subjects} subjects matched)"),
                    );
                }
            });
        if Some(&field) != app.wizard.subject_field.as_ref() && !field.is_empty() {
            app.wizard.subject_field = Some(field);
        }
    });
    ui.add_space(4.0);
}

/// The cards of a step, as many to a row as fit.
fn cards(app: &mut StampApp, ui: &mut egui::Ui, step: Step, facts: &Facts) {
    let choices = app.wizard.choices(step, facts);
    let suggested = match (step, app.wizard.comparison, app.wizard.design) {
        (Step::Data, Some(comparison), Some(design)) => facts.suggested(comparison, design),
        _ => None,
    };
    let spacing = 10.0;
    let per_row = (((ui.available_width() + spacing) / (CARD.x + spacing)).floor() as usize).max(1);
    let mut chosen = None;
    let mut explain = None;
    for row in choices.chunks(per_row) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            for (choice, availability) in row {
                let note = suggested
                    .as_ref()
                    .filter(|(data, _)| Choice::Data(*data) == *choice)
                    .map(|(_, reason)| reason.as_str());
                match card(
                    ui,
                    *choice,
                    availability,
                    note,
                    app.wizard.info == Some(*choice),
                ) {
                    Clicked::Card => chosen = Some(*choice),
                    Clicked::Info => explain = Some(*choice),
                    Clicked::Nothing => {}
                }
            }
        });
        ui.add_space(spacing);
    }
    if let Some(choice) = explain {
        app.wizard.info = if app.wizard.info == Some(choice) {
            None
        } else {
            Some(choice)
        };
    }
    if let Some(choice) = chosen {
        app.wizard.choose(choice, facts);
    }
}

enum Clicked {
    Nothing,
    Card,
    Info,
}

/// One choice: its drawing, name, a line of description, and either the
/// reason it cannot be chosen or a note that the data suggest it.
fn card(
    ui: &mut egui::Ui,
    choice: Choice,
    availability: &Availability,
    note: Option<&str>,
    explained: bool,
) -> Clicked {
    let available = availability.is_available();
    let sense = if available {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(CARD, sense);
    let visuals = ui.visuals().clone();
    let hovered = available && response.hovered();
    let fill = if hovered {
        visuals.widgets.hovered.weak_bg_fill
    } else if available {
        visuals.widgets.inactive.weak_bg_fill
    } else {
        // Flat against the window, where the choices that can be made
        // stand out from it.
        visuals.window_fill
    };
    let stroke = if hovered {
        Stroke::new(1.5, visuals.selection.stroke.color)
    } else if note.is_some() && available {
        Stroke::new(1.5, GREEN)
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    ui.painter()
        .rect(rect, 8.0, fill, stroke, StrokeKind::Inside);
    if available {
        response
            .clone()
            .on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    let inner = rect.shrink(10.0);
    let picture = Rect::from_min_size(inner.min, vec2(inner.width(), PICTURE));
    let fade = if available { 1.0 } else { 0.3 };
    draw(
        &ui.painter_at(picture),
        picture,
        choice,
        &Pens::new(visuals.text_color(), fill, fade),
    );

    let text = Rect::from_min_max(pos2(inner.left(), picture.bottom() + 6.0), inner.max);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.y = 2.0;
    let label = |text: RichText| egui::Label::new(text).wrap().selectable(false);
    let dim = |text: RichText| if available { text } else { text.weak() };
    child.add(label(dim(RichText::new(choice.title()).strong())));
    match availability {
        Availability::Yes => {
            child.add(label(RichText::new(choice.summary()).size(SMALL)));
            if let Some(note) = note {
                child.add(label(RichText::new(note).size(SMALL).color(GREEN)));
            }
        }
        Availability::No(reason) => {
            child.add(label(
                RichText::new(reason)
                    .size(SMALL)
                    .color(visuals.warn_fg_color),
            ));
        }
    }

    // The "i" button, in the corner; it works for greyed choices too.
    let corner = Rect::from_min_size(
        pos2(rect.right() - 28.0, rect.top() + 6.0),
        vec2(22.0, 22.0),
    );
    let info = ui
        .put(
            corner,
            egui::Button::new(RichText::new(icons::INFO).size(16.0))
                .frame(false)
                .selected(explained),
        )
        .on_hover_text("What does this mean?");
    if info.clicked() {
        Clicked::Info
    } else if available && response.clicked() {
        Clicked::Card
    } else {
        Clicked::Nothing
    }
}

/// The longer explanation of the choice whose "i" was clicked.
fn explanation(app: &mut StampApp, ui: &mut egui::Ui) {
    let Some(choice) = app.wizard.info else {
        return;
    };
    let mut close = false;
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icons::label(icons::INFO, choice.title())).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.small_button("Close").clicked();
                });
            });
            ui.label(choice.info());
        });
    ui.add_space(6.0);
    if close {
        app.wizard.info = None;
    }
}

/// Where the answers lead: what will be set, why, the plots, and the
/// button that sets it up.
fn result(app: &mut StampApp, ui: &mut egui::Ui, facts: &Facts) {
    let Some(recommendation) = app.wizard.recommendation(facts) else {
        ui.label("The loaded data no longer support this path. Go back and choose again.");
        return;
    };
    ui.add_space(2.0);
    ui.label(
        RichText::new(&recommendation.heading)
            .size(20.0)
            .strong()
            .color(ui.visuals().selection.stroke.color),
    );
    ui.label(format!("{} analysis", recommendation.mode.name()));
    ui.add_space(6.0);

    let mut show_plot: Option<Option<PlotKind>> = None;
    if ui
        .add(
            egui::Button::new(RichText::new("Set up this analysis").strong())
                .min_size(vec2(180.0, 30.0)),
        )
        .on_hover_text(
            "Chooses these settings in the Statistics panel, runs the analysis and shows the \
             first plot",
        )
        .clicked()
    {
        show_plot = Some(None);
    }
    ui.add_space(6.0);

    if !recommendation.settings.is_empty() {
        section(ui, "What will be set");
        egui::Grid::new("wizard-settings")
            .num_columns(2)
            .spacing([12.0, 3.0])
            .show(ui, |ui| {
                for (label, value) in &recommendation.settings {
                    ui.weak(format!("{label}:"));
                    ui.label(value);
                    ui.end_row();
                }
            });
    }

    section(ui, "Why");
    ui.label(&recommendation.why);
    if let Some(caution) = recommendation.caution {
        ui.add_space(4.0);
        ui.label(RichText::new(caution).color(ui.visuals().warn_fg_color));
    }

    section(ui, "Plots to look at");
    egui::Grid::new("wizard-plots")
        .num_columns(3)
        .spacing([12.0, 3.0])
        .show(ui, |ui| {
            for (plot, purpose) in &recommendation.plots {
                if ui.small_button("Show").clicked() {
                    show_plot = Some(Some(*plot));
                }
                ui.label(RichText::new(plot.name()).strong());
                ui.label(*purpose);
                ui.end_row();
            }
        });

    if !recommendation.alternatives.is_empty() {
        section(ui, "Alternatives");
        for alternative in &recommendation.alternatives {
            ui.label(format!("\u{2022} {alternative}"));
        }
    }
    if let Some(report) = recommendation.report {
        section(ui, "What to report");
        ui.label(report);
    }
    if let Some(plot) = show_plot {
        app.apply_wizard(plot);
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(title).strong());
}

/// How a drawing is made: the colour of its lines, and how much it is
/// faded (for a choice that cannot be made).
struct Pens {
    ink: Color32,
    fade: f32,
    /// The colour of the card behind the drawing.
    background: Color32,
}

impl Pens {
    /// Pens for a drawing on `background`, with lines the colour of the
    /// text, at a strength from 0 to 1.
    fn new(text: Color32, background: Color32, fade: f32) -> Pens {
        let mut pens = Pens {
            ink: Color32::BLACK,
            fade,
            background,
        };
        pens.ink = pens.color(text);
        pens
    }

    /// A colour of the drawing, faded towards the card if the choice
    /// cannot be made. Opaque, so that shapes that overlap do not show.
    fn color(&self, color: Color32) -> Color32 {
        self.mix(color, self.fade)
    }

    /// `color` at a strength from 0 (the card's own colour) to 1.
    fn mix(&self, color: Color32, strength: f32) -> Color32 {
        let channel = |from: u8, to: u8| {
            (from as f32 + (to as f32 - from as f32) * strength.clamp(0.0, 1.0)).round() as u8
        };
        Color32::from_rgb(
            channel(self.background.r(), color.r()),
            channel(self.background.g(), color.g()),
            channel(self.background.b(), color.b()),
        )
    }

    fn line(&self) -> Stroke {
        Stroke::new(1.2, self.ink)
    }
}

/// A point of a drawing, from fractions of its rectangle.
fn at(rect: Rect, x: f32, y: f32) -> Pos2 {
    pos2(
        rect.left() + x * rect.width(),
        rect.top() + y * rect.height(),
    )
}

/// A column of dots at `x`, at the heights `ys`.
fn dots(painter: &egui::Painter, rect: Rect, x: f32, ys: &[f32], color: Color32) {
    for y in ys {
        painter.circle_filled(at(rect, x, *y), 3.6, color);
    }
}

/// The two axes of a small plot.
fn axes(painter: &egui::Painter, rect: Rect, pens: &Pens) {
    painter.line_segment([at(rect, 0.12, 0.05), at(rect, 0.12, 0.92)], pens.line());
    painter.line_segment([at(rect, 0.12, 0.92), at(rect, 0.92, 0.92)], pens.line());
}

/// A stacked bar from the bottom of the drawing, of the given shares.
fn stacked(painter: &egui::Painter, rect: Rect, x: f32, shares: &[(f32, Color32)], pens: &Pens) {
    let mut top = 0.92;
    for (share, color) in shares {
        let height = share * 0.84;
        painter.rect_filled(
            Rect::from_min_max(at(rect, x - 0.07, top - height), at(rect, x + 0.07, top)),
            0.0,
            pens.color(*color),
        );
        top -= height;
    }
}

/// The drawing of a choice.
fn draw(painter: &egui::Painter, rect: Rect, choice: Choice, pens: &Pens) {
    let (blue, orange, green, purple, red) = (
        pens.color(BLUE),
        pens.color(ORANGE),
        pens.color(GREEN),
        pens.color(PURPLE),
        pens.color(RED),
    );
    const COLUMN: [f32; 5] = [0.18, 0.34, 0.5, 0.66, 0.82];
    match choice {
        Choice::Comparison(Comparison::TwoSamples) => {
            painter.circle_filled(at(rect, 0.36, 0.5), 9.0, blue);
            painter.circle_filled(at(rect, 0.64, 0.5), 9.0, orange);
        }
        Choice::Comparison(Comparison::TwoGroups) | Choice::Design(Design::Independent) => {
            dots(painter, rect, 0.36, &COLUMN, blue);
            dots(painter, rect, 0.64, &COLUMN, orange);
        }
        Choice::Comparison(Comparison::MultipleGroups) => {
            for (x, color) in [(0.2, blue), (0.4, orange), (0.6, green), (0.8, purple)] {
                dots(painter, rect, x, &COLUMN, color);
            }
        }
        Choice::Comparison(Comparison::Variable) => {
            painter.line_segment([at(rect, 0.08, 0.7), at(rect, 0.92, 0.7)], pens.line());
            painter.line_segment([at(rect, 0.92, 0.7), at(rect, 0.87, 0.6)], pens.line());
            painter.line_segment([at(rect, 0.92, 0.7), at(rect, 0.87, 0.8)], pens.line());
            for step in 0..7 {
                let x = 0.14 + 0.11 * step as f32;
                let shade = 0.35 + 0.65 * step as f32 / 6.0;
                painter.circle_filled(at(rect, x, 0.42), 4.0, pens.mix(blue, shade));
            }
        }
        Choice::Design(Design::Matched) => {
            for (left, right) in COLUMN.iter().zip([0.3, 0.2, 0.62, 0.5, 0.86]) {
                painter.line_segment([at(rect, 0.36, *left), at(rect, 0.64, right)], pens.line());
                painter.circle_filled(at(rect, 0.36, *left), 3.6, blue);
                painter.circle_filled(at(rect, 0.64, right), 3.6, orange);
            }
        }
        Choice::Question(Question::Differ) => {
            painter.line_segment([at(rect, 0.2, 0.92), at(rect, 0.8, 0.92)], pens.line());
            painter.rect_filled(
                Rect::from_min_max(at(rect, 0.3, 0.62), at(rect, 0.44, 0.92)),
                0.0,
                blue,
            );
            painter.rect_filled(
                Rect::from_min_max(at(rect, 0.56, 0.3), at(rect, 0.7, 0.92)),
                0.0,
                orange,
            );
            // The bracket and star of a significant difference.
            painter.line_segment([at(rect, 0.37, 0.2), at(rect, 0.37, 0.12)], pens.line());
            painter.line_segment([at(rect, 0.37, 0.12), at(rect, 0.63, 0.12)], pens.line());
            painter.line_segment([at(rect, 0.63, 0.12), at(rect, 0.63, 0.2)], pens.line());
            painter.text(
                at(rect, 0.5, 0.1),
                egui::Align2::CENTER_CENTER,
                "*",
                egui::FontId::proportional(15.0),
                pens.ink,
            );
        }
        Choice::Question(Question::Follow) => {
            axes(painter, rect, pens);
            painter.line_segment(
                [at(rect, 0.18, 0.8), at(rect, 0.88, 0.18)],
                Stroke::new(1.5, orange),
            );
            for (x, y) in [
                (0.2, 0.72),
                (0.3, 0.78),
                (0.38, 0.55),
                (0.48, 0.6),
                (0.56, 0.4),
                (0.66, 0.44),
                (0.74, 0.22),
                (0.84, 0.28),
            ] {
                painter.circle_filled(at(rect, x, y), 3.2, blue);
            }
        }
        Choice::Question(Question::Together) => {
            // A small symmetric matrix of correlations.
            let values: [[f32; 4]; 4] = [
                [1.0, 0.8, -0.6, -0.3],
                [0.8, 1.0, -0.4, -0.7],
                [-0.6, -0.4, 1.0, 0.7],
                [-0.3, -0.7, 0.7, 1.0],
            ];
            let side = rect.height() / 4.0;
            let left = rect.center().x - 2.0 * side;
            for (row, cells) in values.iter().enumerate() {
                for (column, value) in cells.iter().enumerate() {
                    let color: Color32 = if *value > 0.0 { red } else { blue };
                    painter.rect_filled(
                        Rect::from_min_size(
                            pos2(left + column as f32 * side, rect.top() + row as f32 * side),
                            vec2(side - 1.0, side - 1.0),
                        ),
                        0.0,
                        pens.mix(color, value.abs()),
                    );
                }
            }
        }
        Choice::Question(Question::Communities) => {
            for (x, y) in [
                (0.22, 0.3),
                (0.3, 0.5),
                (0.2, 0.62),
                (0.36, 0.28),
                (0.34, 0.7),
            ] {
                painter.circle_filled(at(rect, x, y), 3.6, blue);
            }
            for (x, y) in [
                (0.66, 0.4),
                (0.74, 0.6),
                (0.8, 0.3),
                (0.64, 0.72),
                (0.78, 0.78),
            ] {
                painter.circle_filled(at(rect, x, y), 3.6, orange);
            }
            painter.circle_stroke(at(rect, 0.28, 0.48), 22.0, Stroke::new(1.0, blue));
            painter.circle_stroke(at(rect, 0.73, 0.56), 22.0, Stroke::new(1.0, orange));
        }
        Choice::Question(Question::Diversity) => {
            for (x, top, bottom, low, high, color) in [
                (0.36, 0.5, 0.78, 0.9, 0.38, blue),
                (0.64, 0.18, 0.5, 0.66, 0.06, orange),
            ] {
                painter.line_segment([at(rect, x, high), at(rect, x, low)], pens.line());
                painter.rect(
                    Rect::from_min_max(at(rect, x - 0.08, top), at(rect, x + 0.08, bottom)),
                    0.0,
                    color,
                    pens.line(),
                    StrokeKind::Inside,
                );
                let middle = (top + bottom) / 2.0;
                painter.line_segment(
                    [at(rect, x - 0.08, middle), at(rect, x + 0.08, middle)],
                    pens.line(),
                );
            }
        }
        Choice::Question(Question::MadeOf) => {
            stacked(
                painter,
                rect,
                0.36,
                &[(0.45, BLUE), (0.3, ORANGE), (0.15, GREEN), (0.1, PURPLE)],
                pens,
            );
            stacked(
                painter,
                rect,
                0.64,
                &[(0.2, BLUE), (0.25, ORANGE), (0.4, GREEN), (0.15, PURPLE)],
                pens,
            );
        }
        Choice::Question(Question::Depth) => {
            axes(painter, rect, pens);
            // A curve that climbs and levels off.
            let points: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let x = i as f32 / 20.0;
                    at(
                        rect,
                        0.12 + 0.78 * x,
                        0.92 - 0.76 * (1.0 - (-4.0 * x).exp()),
                    )
                })
                .collect();
            painter.add(egui::Shape::line(points, Stroke::new(1.8, blue)));
        }
        Choice::Data(Data::Typical) => {
            dots(painter, rect, 0.3, &[0.5, 0.62, 0.74, 0.86], blue);
            dots(painter, rect, 0.4, &[0.44, 0.56, 0.68, 0.8], blue);
            dots(painter, rect, 0.6, &[0.2, 0.32, 0.44, 0.56], orange);
            dots(painter, rect, 0.7, &[0.14, 0.26, 0.38, 0.5], orange);
        }
        Choice::Data(Data::SmallOrExtreme) => {
            dots(painter, rect, 0.36, &[0.7, 0.78, 0.86], blue);
            dots(painter, rect, 0.64, &[0.66, 0.76, 0.1], orange);
        }
        Choice::Data(Data::MostlyAbsent) => {
            painter.line_segment([at(rect, 0.1, 0.86), at(rect, 0.9, 0.86)], pens.line());
            for (index, x) in [0.16, 0.26, 0.36, 0.46, 0.56, 0.66, 0.76, 0.86]
                .iter()
                .enumerate()
            {
                let y = match index {
                    2 => 0.3,
                    6 => 0.5,
                    _ => 0.86,
                };
                painter.circle_filled(at(rect, *x, y), 3.6, if index < 4 { blue } else { orange });
            }
        }
        Choice::Data(Data::Presence) => {
            // Present (filled) in most samples of one group, few of the other.
            for (x, color, present) in [(0.36, blue, 4), (0.64, orange, 1)] {
                for (row, y) in COLUMN.iter().enumerate() {
                    if row < present {
                        painter.circle_filled(at(rect, x, *y), 4.0, color);
                    } else {
                        painter.circle_stroke(at(rect, x, *y), 3.6, Stroke::new(1.2, color));
                    }
                }
            }
        }
        Choice::Data(Data::Dominated) => {
            stacked(
                painter,
                rect,
                0.36,
                &[(0.8, BLUE), (0.1, ORANGE), (0.06, GREEN), (0.04, PURPLE)],
                pens,
            );
            stacked(
                painter,
                rect,
                0.64,
                &[(0.6, BLUE), (0.2, ORANGE), (0.12, GREEN), (0.08, PURPLE)],
                pens,
            );
        }
    }
}
