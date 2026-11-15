//! A terminal inside STAMP's window, for Claude Code.
//!
//! `alacritty_terminal` (the core of the Alacritty terminal) runs the
//! program in a pseudo-terminal (ConPTY on Windows), interprets what it
//! writes and keeps the screen and its scrollback; this module draws that
//! screen with egui and turns keys, pastes and the mouse into terminal
//! input. Desktop only.
//!
//! The terminal is always dark, whatever STAMP's theme: Claude Code's
//! default colours are made for a dark background.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event as TermEvent, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, CursorShape, NamedColor, Rgb};
use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};

/// Name of the font family the terminal is drawn in (see
/// [`add_to_fonts`]).
const FAMILY: &str = "stamp-terminal";
/// Size of the terminal's text, in points.
const FONT_SIZE: f32 = 13.0;
/// Lines kept above the screen.
const SCROLLBACK: usize = 10_000;
/// Lines one notch of a mouse wheel scrolls.
const LINES_PER_NOTCH: f32 = 3.0;
/// Width of the scroll bar, in points.
const SCROLL_BAR_WIDTH: f32 = 12.0;

/// Add the terminal's font family: DejaVu Sans Mono, then egui's own
/// fonts. The few symbols of Claude Code's that DejaVu Sans Mono lacks are
/// drawn as near look-alikes it has ([`drawn_as`]).
pub fn add_to_fonts(fonts: &mut egui::FontDefinitions) {
    let name = "stamp-dejavu-sans-mono".to_string();
    fonts.font_data.insert(
        name.clone(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/DejaVuSansMono.ttf"
        ))),
    );
    let mut family = vec![name];
    for fallback in ["emoji-icon-font", "NotoEmoji-Regular", "Hack"] {
        if fonts.font_data.contains_key(fallback) {
            family.push(fallback.to_string());
        }
    }
    fonts
        .families
        .insert(FontFamily::Name(FAMILY.into()), family);
}

/// The size of the terminal, as `alacritty_terminal` asks for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Size {
    columns: usize,
    lines: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

impl Size {
    fn window(self, cell: Vec2) -> WindowSize {
        WindowSize {
            num_lines: self.lines as u16,
            num_cols: self.columns as u16,
            cell_width: cell.x.round() as u16,
            cell_height: cell.y.round() as u16,
        }
    }
}

/// What the terminal tells the window, from its own thread.
#[derive(Clone)]
struct Listener {
    ctx: egui::Context,
    /// Where replies to the program go (set once the event loop exists).
    sender: Arc<Mutex<Option<EventLoopSender>>>,
    exited: Arc<AtomicBool>,
    title: Arc<Mutex<String>>,
    /// Text the program asked to put on the clipboard.
    clipboard: Arc<Mutex<Option<String>>>,
    window: Arc<Mutex<WindowSize>>,
}

impl Listener {
    fn write(&self, text: String) {
        if let Ok(sender) = self.sender.lock() {
            if let Some(sender) = sender.as_ref() {
                let _ = sender.send(Msg::Input(Cow::Owned(text.into_bytes())));
            }
        }
    }
}

impl EventListener for Listener {
    fn send_event(&self, event: TermEvent) {
        match event {
            TermEvent::Wakeup | TermEvent::MouseCursorDirty | TermEvent::CursorBlinkingChange => {}
            TermEvent::PtyWrite(text) => self.write(text),
            TermEvent::Title(title) => {
                if let Ok(mut current) = self.title.lock() {
                    *current = title;
                }
            }
            TermEvent::ResetTitle => {
                if let Ok(mut current) = self.title.lock() {
                    current.clear();
                }
            }
            TermEvent::ClipboardStore(_, text) => {
                if let Ok(mut clipboard) = self.clipboard.lock() {
                    *clipboard = Some(text);
                }
            }
            TermEvent::ColorRequest(index, format) => {
                self.write(format(palette(index).unwrap_or(BACKGROUND_RGB)));
            }
            TermEvent::TextAreaSizeRequest(format) => {
                let size = self.window.lock().map(|w| *w).ok();
                if let Some(size) = size {
                    self.write(format(size));
                }
            }
            TermEvent::Exit | TermEvent::ChildExit(_) => self.exited.store(true, Ordering::SeqCst),
            TermEvent::ClipboardLoad(..) | TermEvent::Bell => {}
        }
        self.ctx.request_repaint();
    }
}

/// A program running in a terminal, and how it is shown.
pub struct Terminal {
    term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    listener: Listener,
    size: Size,
    /// Width and height of a character cell, in points.
    cell: Vec2,
    /// What runs in it, for the panel's heading.
    pub label: String,
    /// The terminal had the keyboard last frame.
    focused: bool,
    /// Scrolling not yet a whole line, in lines.
    scrolled: f32,
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Ends the program (SIGHUP on Unix; the console is closed on
        // Windows).
        let _ = self.sender.send(Msg::Shutdown);
    }
}

impl Terminal {
    /// Start `program` with `args` in `folder`, with the extra environment
    /// variables `env`.
    pub fn spawn(
        ctx: &egui::Context,
        label: &str,
        program: &str,
        args: &[String],
        folder: &Path,
        env: HashMap<String, String>,
    ) -> std::io::Result<Terminal> {
        let mut env = env;
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        let options = tty::Options {
            shell: Some(tty::Shell::new(program.to_string(), args.to_vec())),
            working_directory: Some(folder.to_path_buf()),
            drain_on_exit: true,
            env,
            #[cfg(windows)]
            escape_args: true,
        };
        // A rough cell until the panel is drawn and measures it: fonts may
        // not be loaded yet.
        let cell = Vec2::new(8.0, 17.0);
        let size = Size {
            columns: 100,
            lines: 24,
        };
        let listener = Listener {
            ctx: ctx.clone(),
            sender: Arc::new(Mutex::new(None)),
            exited: Arc::new(AtomicBool::new(false)),
            title: Arc::new(Mutex::new(String::new())),
            clipboard: Arc::new(Mutex::new(None)),
            window: Arc::new(Mutex::new(size.window(cell))),
        };
        let pty = tty::new(&options, size.window(cell), 0)?;
        let config = Config {
            scrolling_history: SCROLLBACK,
            ..Config::default()
        };
        let term = Arc::new(FairMutex::new(Term::new(config, &size, listener.clone())));
        let event_loop = EventLoop::new(term.clone(), listener.clone(), pty, true, false)?;
        let sender = event_loop.channel();
        if let Ok(mut shared) = listener.sender.lock() {
            *shared = Some(sender.clone());
        }
        event_loop.spawn();
        Ok(Terminal {
            term,
            sender,
            listener,
            size,
            cell,
            label: label.to_string(),
            focused: false,
            scrolled: 0.0,
        })
    }

    /// Whether the program has ended.
    pub fn has_exited(&self) -> bool {
        self.listener.exited.load(Ordering::SeqCst)
    }

    /// The title the program gave the terminal, if any.
    pub fn title(&self) -> String {
        self.listener
            .title
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default()
    }

    /// Whether the terminal had the keyboard in the last frame, so that
    /// STAMP's own shortcuts leave its keys alone.
    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Send text to the program as if typed.
    pub fn write(&self, bytes: impl Into<Vec<u8>>) {
        let _ = self.sender.send(Msg::Input(Cow::Owned(bytes.into())));
    }

    /// The text on the screen, one line per row, without trailing blanks:
    /// for tests and for copying.
    pub fn screen_text(&self) -> String {
        let term = self.term.lock();
        let grid = term.grid();
        let mut lines = Vec::new();
        for line in 0..grid.screen_lines() {
            let row = &grid[Line(line as i32 - grid.display_offset() as i32)];
            let text: String = (0..grid.columns())
                .map(|c| row[Column(c)].c)
                .collect::<String>()
                .trim_end()
                .to_string();
            lines.push(text);
        }
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Fit the terminal to `columns` by `lines`.
    fn resize(&mut self, columns: usize, lines: usize) {
        let size = Size {
            columns: columns.max(2),
            lines: lines.max(1),
        };
        if size == self.size {
            return;
        }
        self.size = size;
        let window = size.window(self.cell);
        if let Ok(mut shared) = self.listener.window.lock() {
            *shared = window;
        }
        self.term.lock().resize(size);
        let _ = self.sender.send(Msg::Resize(window));
    }

    /// Draw the terminal in all the room `ui` has, and pass it the
    /// keyboard and mouse when it has the focus.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.cell = cell_size(ui.ctx());
        // A fixed id, so that the keyboard focus stays with the terminal
        // whatever is drawn before it (the heading shows the program's
        // title once it sets one).
        let (whole, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let (rect, bar) = whole.split_left_right_at_x(whole.max.x - SCROLL_BAR_WIDTH);
        let response = ui.interact(
            rect,
            egui::Id::unique("stamp-claude-code-terminal"),
            Sense::click_and_drag(),
        );
        let columns = (rect.width() / self.cell.x).floor() as usize;
        let lines = (rect.height() / self.cell.y).floor() as usize;
        self.resize(columns, lines);
        let id = response.id;

        if response.clicked() || response.drag_started() {
            response.request_focus();
        }
        let focused = response.has_focus();
        self.focused = focused;
        if focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
            self.take_input(ui);
        }
        if let Some(text) = self
            .listener
            .clipboard
            .lock()
            .ok()
            .and_then(|mut c| c.take())
        {
            ui.ctx().copy_text(text);
        }
        self.mouse(ui, &response, rect);
        self.scroll_bar(ui, bar);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
        self.paint(ui, rect, focused);
    }

    /// The cell under a screen position, and which half of it.
    fn point_at(&self, rect: Rect, pos: Pos2, display_offset: usize) -> (Point, Side) {
        let x = ((pos.x - rect.min.x) / self.cell.x).max(0.0);
        let y = ((pos.y - rect.min.y) / self.cell.y).max(0.0);
        let column = (x.floor() as usize).min(self.size.columns.saturating_sub(1));
        let row = (y.floor() as usize).min(self.size.lines.saturating_sub(1));
        let side = if x.fract() < 0.5 {
            Side::Left
        } else {
            Side::Right
        };
        let point = Point::new(Line(row as i32 - display_offset as i32), Column(column));
        (point, side)
    }

    /// Scrolling and selecting text with the mouse.
    fn mouse(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect) {
        let lines = if response.hovered() {
            self.wheel_lines(ui, self.size.lines)
        } else {
            0
        };
        let mut term = self.term.lock();
        {
            if lines != 0 {
                let pointer = response.hover_pos().unwrap_or(rect.center());
                let mode = *term.mode();
                let column = ((pointer.x - rect.min.x) / self.cell.x).max(0.0) as usize;
                let row = ((pointer.y - rect.min.y) / self.cell.y).max(0.0) as usize;
                match wheel_input(lines, mode, column.min(self.size.columns - 1), row) {
                    Some(bytes) => {
                        drop(term);
                        self.write(bytes);
                        term = self.term.lock();
                    }
                    None => term.scroll_display(Scroll::Delta(lines)),
                }
            }
        }
        let offset = term.grid().display_offset();
        if let Some(pos) = response.interact_pointer_pos() {
            let (point, side) = self.point_at(rect, pos, offset);
            if response.drag_started() {
                term.selection = Some(Selection::new(SelectionType::Simple, point, side));
            } else if response.dragged() {
                if let Some(selection) = &mut term.selection {
                    selection.update(point, side);
                }
            }
        }
        if response.clicked() {
            term.selection = None;
        }
    }

    /// Lines the mouse wheel moved this frame, upwards (into the
    /// scrollback) positive: three to a notch, as terminals scroll, and
    /// fractions of a line from touchpads kept for the next frame.
    fn wheel_lines(&mut self, ui: &egui::Ui, page: usize) -> i32 {
        let events = ui.input(|i| i.events.clone());
        for event in events {
            if let egui::Event::MouseWheel { unit, delta, .. } = event {
                self.scrolled += match unit {
                    egui::MouseWheelUnit::Line => delta.y * LINES_PER_NOTCH,
                    egui::MouseWheelUnit::Point => delta.y / self.cell.y,
                    egui::MouseWheelUnit::Page => delta.y * page as f32,
                };
            }
        }
        let lines = self.scrolled.trunc() as i32;
        self.scrolled -= lines as f32;
        lines
    }

    /// The scroll bar beside the terminal: where the screen is in the
    /// scrollback, and dragging or clicking it to move there.
    fn scroll_bar(&mut self, ui: &egui::Ui, bar: Rect) {
        let response = ui.interact(
            bar,
            egui::Id::unique("stamp-claude-code-scroll-bar"),
            Sense::click_and_drag(),
        );
        let lines = if response.hovered() {
            self.wheel_lines(ui, self.size.lines)
        } else {
            0
        };
        let mut term = self.term.lock();
        if lines != 0 {
            term.scroll_display(Scroll::Delta(lines));
        }
        let screen = term.grid().screen_lines();
        let history = term.grid().total_lines() - screen;
        let offset = term.grid().display_offset();
        if let Some(pos) = response.interact_pointer_pos() {
            if history > 0 && (response.dragged() || response.clicked()) {
                let wanted = scroll_offset_at(pos.y, bar, history, screen);
                term.scroll_display(Scroll::Delta(wanted as i32 - offset as i32));
            }
        }
        let offset = term.grid().display_offset();
        drop(term);
        let painter = ui.painter_at(bar);
        painter.rect_filled(bar, 0.0, Color32::from_gray(0x2b));
        if history > 0 {
            let thumb = scroll_thumb(bar, history, screen, offset);
            let shade = if response.dragged() || response.hovered() {
                0xb0
            } else {
                0x80
            };
            painter.rect_filled(
                thumb.shrink2(Vec2::new(2.5, 1.0)),
                3.0,
                Color32::from_gray(shade),
            );
        }
    }

    /// Turn this frame's keys and pastes into input for the program.
    fn take_input(&mut self, ui: &egui::Ui) {
        let events = ui.input(|i| i.events.clone());
        let mut term = self.term.lock();
        let mode = *term.mode();
        let mut input: Vec<u8> = Vec::new();
        for event in events {
            match event {
                egui::Event::Text(text) => input.extend_from_slice(text.as_bytes()),
                egui::Event::Paste(text) => input.extend(paste_bytes(&text, mode)),
                egui::Event::Copy => match term.selection_to_string() {
                    Some(text) if !text.is_empty() => {
                        ui.ctx().copy_text(text);
                        term.selection = None;
                    }
                    // Ctrl+C with nothing selected interrupts the program.
                    _ => input.push(0x03),
                },
                egui::Event::Cut => input.push(0x18),
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if let Some(bytes) = key_bytes(key, modifiers, mode) {
                        input.extend(bytes);
                    }
                }
                _ => {}
            }
        }
        if !input.is_empty() {
            // Typing brings the screen back from the scrollback.
            term.scroll_display(Scroll::Bottom);
            drop(term);
            self.write(input);
        }
    }

    fn paint(&self, ui: &egui::Ui, rect: Rect, focused: bool) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, color32(BACKGROUND_RGB));
        let font = FontId::new(FONT_SIZE, FontFamily::Name(FAMILY.into()));
        let term = self.term.lock();
        let content = term.renderable_content();
        let offset = content.display_offset as i32;
        let cell_rect = |line: i32, column: usize, width: usize| {
            Rect::from_min_size(
                Pos2::new(
                    rect.min.x + column as f32 * self.cell.x,
                    rect.min.y + (line + offset) as f32 * self.cell.y,
                ),
                Vec2::new(self.cell.x * width as f32, self.cell.y),
            )
        };
        let selection = content.selection;
        let mut cursor_text = None;
        for indexed in content.display_iter {
            let cell = indexed.cell;
            let point = indexed.point;
            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }
            let selected = selection.is_some_and(|s| s.contains(point));
            let (fg, bg) = cell_colors(cell.fg, cell.bg, cell.flags, content.colors, selected);
            let width = if cell.flags.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            let area = cell_rect(point.line.0, point.column.0, width);
            if bg != color32(BACKGROUND_RGB) {
                painter.rect_filled(area, 0.0, bg);
            }
            if point == content.cursor.point {
                cursor_text = Some((cell.c, fg, bg, area));
            }
            if cell.c != ' ' && cell.c != '\t' && !cell.flags.contains(Flags::HIDDEN) {
                painter.text(
                    area.left_center(),
                    egui::Align2::LEFT_CENTER,
                    drawn_as(cell.c),
                    font.clone(),
                    fg,
                );
            }
            if cell.flags.intersects(Flags::ALL_UNDERLINES) {
                let y = area.max.y - 1.5;
                painter.line_segment(
                    [Pos2::new(area.min.x, y), Pos2::new(area.max.x, y)],
                    Stroke::new(1.0, fg),
                );
            }
            if cell.flags.contains(Flags::STRIKEOUT) {
                let y = area.center().y;
                painter.line_segment(
                    [Pos2::new(area.min.x, y), Pos2::new(area.max.x, y)],
                    Stroke::new(1.0, fg),
                );
            }
        }
        // The cursor: solid when the terminal has the keyboard, an outline
        // when it does not.
        let shape = content.cursor.shape;
        if shape != CursorShape::Hidden {
            let area = cursor_text.map(|(.., area)| area).unwrap_or_else(|| {
                cell_rect(
                    content.cursor.point.line.0,
                    content.cursor.point.column.0,
                    1,
                )
            });
            let color = color32(CURSOR_RGB);
            match (focused, shape) {
                (true, CursorShape::Block) => {
                    painter.rect_filled(area, 0.0, color);
                    if let Some((c, _, bg, _)) = cursor_text {
                        if c != ' ' {
                            painter.text(
                                area.left_center(),
                                egui::Align2::LEFT_CENTER,
                                drawn_as(c),
                                font.clone(),
                                if bg == color {
                                    color32(BACKGROUND_RGB)
                                } else {
                                    bg
                                },
                            );
                        }
                    }
                }
                (true, CursorShape::Beam) => {
                    painter.rect_filled(
                        Rect::from_min_size(area.min, Vec2::new(2.0, area.height())),
                        0.0,
                        color,
                    );
                }
                (true, CursorShape::Underline) => {
                    painter.rect_filled(
                        Rect::from_min_max(Pos2::new(area.min.x, area.max.y - 2.0), area.max),
                        0.0,
                        color,
                    );
                }
                _ => {
                    painter.rect_stroke(
                        area.shrink(0.5),
                        0.0,
                        Stroke::new(1.0, color),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
    }
}

/// What the mouse wheel sends to the program for `lines` (upwards
/// positive) with the pointer over cell `column`, `row` (from 0), or `None`
/// when the terminal should scroll its own scrollback instead. A program
/// that asked for mouse reports (Claude Code's full-screen view does) gets
/// wheel buttons 64 and 65, as xterm sends them; on the alternate screen
/// without mouse reports, the arrow keys.
pub fn wheel_input(lines: i32, mode: TermMode, column: usize, row: usize) -> Option<Vec<u8>> {
    let count = lines.unsigned_abs().min(50) as usize;
    if mode.intersects(TermMode::MOUSE_MODE) {
        let button: u8 = if lines > 0 { 64 } else { 65 };
        let (x, y) = (column + 1, row + 1);
        let report = if mode.contains(TermMode::SGR_MOUSE) {
            format!("\x1b[<{button};{x};{y}M").into_bytes()
        } else if x < 223 && y < 223 {
            vec![0x1b, b'[', b'M', 32 + button, 32 + x as u8, 32 + y as u8]
        } else {
            return Some(Vec::new());
        };
        return Some(report.repeat(count));
    }
    if mode.contains(TermMode::ALT_SCREEN) && mode.contains(TermMode::ALTERNATE_SCROLL) {
        let letter = if lines > 0 { 'A' } else { 'B' };
        let arrow = if mode.contains(TermMode::APP_CURSOR) {
            format!("\x1bO{letter}")
        } else {
            format!("\x1b[{letter}")
        };
        return Some(arrow.repeat(count).into_bytes());
    }
    None
}

/// The scroll bar's thumb: the part of the scrollback (`history` lines
/// above a screen of `screen` lines) that is shown, `offset` lines up.
fn scroll_thumb(bar: Rect, history: usize, screen: usize, offset: usize) -> Rect {
    let total = (history + screen) as f32;
    let height = (bar.height() * screen as f32 / total)
        .max(16.0)
        .min(bar.height());
    let room = bar.height() - height;
    let top = bar.min.y + room * (history - offset.min(history)) as f32 / history.max(1) as f32;
    Rect::from_min_size(Pos2::new(bar.min.x, top), Vec2::new(bar.width(), height))
}

/// The scroll offset that puts the middle of the thumb at `y`.
fn scroll_offset_at(y: f32, bar: Rect, history: usize, screen: usize) -> usize {
    let thumb = scroll_thumb(bar, history, screen, 0);
    let room = (bar.height() - thumb.height()).max(1.0);
    let fraction = ((y - bar.min.y - thumb.height() / 2.0) / room).clamp(0.0, 1.0);
    // The top of the bar is the oldest line.
    ((1.0 - fraction) * history as f32).round() as usize
}

/// The character drawn for `c`: itself, or for the symbols Claude Code
/// uses that DejaVu Sans Mono lacks (egui 0.37 has no emoji fonts by
/// default), the nearest it has.
pub fn drawn_as(c: char) -> char {
    match c {
        '\u{23FA}' => '\u{25CF}', // ⏺ record → ● black circle
        '\u{23BF}' => '\u{2514}', // ⎿ → └ box drawing up and right
        '\u{23F5}' => '\u{25B6}', // ⏵ → ▶
        '\u{23F4}' => '\u{25C0}', // ⏴ → ◀
        '\u{23F6}' => '\u{25B2}', // ⏶ → ▲
        '\u{23F7}' => '\u{25BC}', // ⏷ → ▼
        '\u{23F8}' => '\u{2016}', // ⏸ pause → ‖
        '\u{23F9}' => '\u{25A0}', // ⏹ stop → ■
        '\u{203B}' => '*',        // ※
        '\u{29C9}' => '\u{25A1}', // ⧉ → □
        _ => c,
    }
}

/// Width and height of a character cell.
fn cell_size(ctx: &egui::Context) -> Vec2 {
    let font = FontId::new(FONT_SIZE, FontFamily::Name(FAMILY.into()));
    ctx.fonts_mut(|fonts| {
        let width = fonts.glyph_width(&font, 'M');
        let height = fonts.row_height(&font);
        Vec2::new(width.max(1.0), height.ceil().max(1.0))
    })
}

/// What pasting `text` sends: wrapped in the bracketed-paste markers when
/// the program asked for them, so that it takes the text as one paste.
pub fn paste_bytes(text: &str, mode: TermMode) -> Vec<u8> {
    // Line breaks are sent as a terminal sends Enter.
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if mode.contains(TermMode::BRACKETED_PASTE) {
        // The end marker inside a paste would end it early.
        let text = text.replace("\x1b[201~", "");
        format!("\x1b[200~{text}\x1b[201~").into_bytes()
    } else {
        text.into_bytes()
    }
}

/// What a key sends, as an xterm does; `None` for keys that send nothing
/// (and for typed characters, which arrive as text).
pub fn key_bytes(key: egui::Key, modifiers: egui::Modifiers, mode: TermMode) -> Option<Vec<u8>> {
    use egui::Key;
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    // xterm's modifier parameter: 1 + Shift + 2 Alt + 4 Ctrl.
    let parameter =
        1 + u8::from(modifiers.shift) + 2 * u8::from(modifiers.alt) + 4 * u8::from(modifiers.ctrl);
    let cursor = |letter: char| -> Vec<u8> {
        if parameter > 1 {
            format!("\x1b[1;{parameter}{letter}").into_bytes()
        } else if app_cursor {
            format!("\x1bO{letter}").into_bytes()
        } else {
            format!("\x1b[{letter}").into_bytes()
        }
    };
    let tilde = |number: u8| -> Vec<u8> {
        if parameter > 1 {
            format!("\x1b[{number};{parameter}~").into_bytes()
        } else {
            format!("\x1b[{number}~").into_bytes()
        }
    };
    let bytes = match key {
        // Shift+Enter or Alt+Enter: a new line without sending, as Claude
        // Code's terminal setup makes other terminals send.
        Key::Enter if modifiers.shift || modifiers.alt => b"\x1b\r".to_vec(),
        Key::Enter => b"\r".to_vec(),
        Key::Backspace if modifiers.alt => b"\x1b\x7f".to_vec(),
        Key::Backspace if modifiers.ctrl => vec![0x08],
        Key::Backspace => vec![0x7f],
        Key::Tab if modifiers.shift => b"\x1b[Z".to_vec(),
        Key::Tab => b"\t".to_vec(),
        Key::Escape => b"\x1b".to_vec(),
        Key::ArrowUp => cursor('A'),
        Key::ArrowDown => cursor('B'),
        Key::ArrowRight => cursor('C'),
        Key::ArrowLeft => cursor('D'),
        Key::Home => cursor('H'),
        Key::End => cursor('F'),
        Key::Insert => tilde(2),
        Key::Delete => tilde(3),
        Key::PageUp => tilde(5),
        Key::PageDown => tilde(6),
        Key::F1 => b"\x1bOP".to_vec(),
        Key::F2 => b"\x1bOQ".to_vec(),
        Key::F3 => b"\x1bOR".to_vec(),
        Key::F4 => b"\x1bOS".to_vec(),
        Key::F5 => tilde(15),
        Key::F6 => tilde(17),
        Key::F7 => tilde(18),
        Key::F8 => tilde(19),
        Key::F9 => tilde(20),
        Key::F10 => tilde(21),
        Key::F11 => tilde(23),
        Key::F12 => tilde(24),
        // Ctrl with a letter: the control character. Typed characters
        // without Ctrl arrive as text instead. The Command key of macOS is
        // the system's.
        _ if modifiers.ctrl && !modifiers.mac_cmd => {
            let name = key.name();
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphabetic() => {
                    vec![(c.to_ascii_lowercase() as u8) & 0x1f]
                }
                _ => match key {
                    Key::Space => vec![0],
                    Key::OpenBracket => vec![0x1b],
                    Key::Backslash => vec![0x1c],
                    Key::CloseBracket => vec![0x1d],
                    _ => return None,
                },
            }
        }
        _ => return None,
    };
    Some(bytes)
}

const FOREGROUND_RGB: Rgb = Rgb {
    r: 0xcc,
    g: 0xcc,
    b: 0xcc,
};
const BACKGROUND_RGB: Rgb = Rgb {
    r: 0x1e,
    g: 0x1e,
    b: 0x1e,
};
const CURSOR_RGB: Rgb = Rgb {
    r: 0xae,
    g: 0xaf,
    b: 0xad,
};

/// The sixteen standard colours (those of Visual Studio Code's dark
/// terminal), then bright versions.
const ANSI: [u32; 16] = [
    0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5, 0x666666,
    0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xe5e5e5,
];

fn rgb(value: u32) -> Rgb {
    Rgb {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
    }
}

/// Colour number `index` of the terminal's palette: the sixteen colours,
/// the 6×6×6 cube, the greys, then the named ones (foreground and so on).
pub fn palette(index: usize) -> Option<Rgb> {
    match index {
        0..=15 => Some(rgb(ANSI[index])),
        16..=231 => {
            let i = index - 16;
            let level = |v: usize| if v == 0 { 0 } else { (55 + v * 40) as u8 };
            Some(Rgb {
                r: level(i / 36),
                g: level((i / 6) % 6),
                b: level(i % 6),
            })
        }
        232..=255 => {
            let v = (8 + (index - 232) * 10) as u8;
            Some(Rgb { r: v, g: v, b: v })
        }
        i if i == NamedColor::Foreground as usize => Some(FOREGROUND_RGB),
        i if i == NamedColor::Background as usize => Some(BACKGROUND_RGB),
        i if i == NamedColor::Cursor as usize => Some(CURSOR_RGB),
        i if i == NamedColor::BrightForeground as usize => Some(rgb(0xffffff)),
        i if i == NamedColor::DimForeground as usize => Some(dim(FOREGROUND_RGB)),
        i if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize).contains(&i) => {
            Some(dim(rgb(ANSI[i - NamedColor::DimBlack as usize])))
        }
        _ => None,
    }
}

fn dim(color: Rgb) -> Rgb {
    Rgb {
        r: (f32::from(color.r) * 0.66) as u8,
        g: (f32::from(color.g) * 0.66) as u8,
        b: (f32::from(color.b) * 0.66) as u8,
    }
}

fn color32(color: Rgb) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

/// The colours a cell is drawn in: its own, with colours the program has
/// redefined, bold as bright, dim, inverse, and selected text inverted.
fn cell_colors(
    fg: AnsiColor,
    bg: AnsiColor,
    flags: Flags,
    colors: &alacritty_terminal::term::color::Colors,
    selected: bool,
) -> (Color32, Color32) {
    let resolve = |color: AnsiColor, bold: bool| -> Rgb {
        match color {
            AnsiColor::Spec(rgb) => rgb,
            AnsiColor::Indexed(index) => {
                let mut index = usize::from(index);
                if bold && index < 8 {
                    index += 8;
                }
                colors[index]
                    .or_else(|| palette(index))
                    .unwrap_or(FOREGROUND_RGB)
            }
            AnsiColor::Named(named) => {
                let mut index = named as usize;
                if bold && index < 8 {
                    index += 8;
                }
                colors[index]
                    .or_else(|| palette(index))
                    .unwrap_or(FOREGROUND_RGB)
            }
        }
    };
    let mut fg = resolve(fg, flags.contains(Flags::BOLD));
    let mut bg = resolve(bg, false);
    if flags.contains(Flags::DIM) {
        fg = dim(fg);
    }
    if flags.contains(Flags::INVERSE) != selected {
        std::mem::swap(&mut fg, &mut bg);
    }
    (color32(fg), color32(bg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers};

    fn keys(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
        key_bytes(key, modifiers, TermMode::empty())
    }

    #[test]
    fn keys_send_what_xterm_sends() {
        assert_eq!(keys(Key::Enter, Modifiers::NONE), Some(b"\r".to_vec()));
        assert_eq!(keys(Key::Enter, Modifiers::SHIFT), Some(b"\x1b\r".to_vec()));
        assert_eq!(keys(Key::Backspace, Modifiers::NONE), Some(vec![0x7f]));
        assert_eq!(keys(Key::Tab, Modifiers::SHIFT), Some(b"\x1b[Z".to_vec()));
        assert_eq!(
            keys(Key::ArrowUp, Modifiers::NONE),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_bytes(Key::ArrowUp, Modifiers::NONE, TermMode::APP_CURSOR),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            keys(Key::ArrowLeft, Modifiers::CTRL),
            Some(b"\x1b[1;5D".to_vec())
        );
        assert_eq!(
            keys(Key::Delete, Modifiers::NONE),
            Some(b"\x1b[3~".to_vec())
        );
        assert_eq!(keys(Key::C, Modifiers::CTRL), Some(vec![0x03]));
        assert_eq!(keys(Key::R, Modifiers::CTRL), Some(vec![0x12]));
        assert_eq!(keys(Key::Escape, Modifiers::NONE), Some(vec![0x1b]));
        // Letters without Ctrl arrive as text.
        assert_eq!(keys(Key::C, Modifiers::NONE), None);
    }

    #[test]
    fn the_wheel_goes_to_programs_that_ask_for_the_mouse() {
        assert_eq!(wheel_input(3, TermMode::empty(), 0, 0), None);
        let sgr = TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE;
        assert_eq!(
            wheel_input(2, sgr, 4, 9),
            Some(b"\x1b[<64;5;10M\x1b[<64;5;10M".to_vec())
        );
        assert_eq!(wheel_input(-1, sgr, 0, 0), Some(b"\x1b[<65;1;1M".to_vec()));
        assert_eq!(
            wheel_input(1, TermMode::MOUSE_REPORT_CLICK, 0, 0),
            Some(vec![0x1b, b'[', b'M', 96, 33, 33])
        );
        let alternate = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
        assert_eq!(
            wheel_input(-2, alternate, 0, 0),
            Some(b"\x1b[B\x1b[B".to_vec())
        );
    }

    #[test]
    fn the_scroll_bar_follows_the_scrollback() {
        let bar = Rect::from_min_size(Pos2::ZERO, Vec2::new(12.0, 100.0));
        // At the bottom, the thumb is at the bottom; at the top, the top.
        let bottom = scroll_thumb(bar, 300, 100, 0);
        assert_eq!(bottom.max.y, 100.0);
        assert_eq!(bottom.height(), 25.0);
        assert_eq!(scroll_thumb(bar, 300, 100, 300).min.y, 0.0);
        assert_eq!(scroll_offset_at(0.0, bar, 300, 100), 300);
        assert_eq!(scroll_offset_at(100.0, bar, 300, 100), 0);
    }

    #[test]
    fn pastes_are_bracketed_when_asked_for() {
        assert_eq!(paste_bytes("a\nb", TermMode::empty()), b"a\rb".to_vec());
        assert_eq!(
            paste_bytes("x\x1b[201~y", TermMode::BRACKETED_PASTE),
            b"\x1b[200~xy\x1b[201~".to_vec()
        );
    }

    #[test]
    fn palette_has_the_cube_and_greys() {
        assert_eq!(palette(16), Some(Rgb { r: 0, g: 0, b: 0 }));
        assert_eq!(
            palette(231),
            Some(Rgb {
                r: 255,
                g: 255,
                b: 255
            })
        );
        assert_eq!(palette(196), Some(Rgb { r: 255, g: 0, b: 0 }));
        assert_eq!(palette(232), Some(Rgb { r: 8, g: 8, b: 8 }));
        assert_eq!(
            palette(NamedColor::Background as usize),
            Some(BACKGROUND_RGB)
        );
    }

    #[test]
    fn bold_named_colours_are_bright_and_inverse_swaps() {
        let colors = alacritty_terminal::term::color::Colors::default();
        let (fg, _) = cell_colors(
            AnsiColor::Named(NamedColor::Red),
            AnsiColor::Named(NamedColor::Background),
            Flags::BOLD,
            &colors,
            false,
        );
        assert_eq!(fg, color32(rgb(ANSI[9])));
        let (fg, bg) = cell_colors(
            AnsiColor::Named(NamedColor::Foreground),
            AnsiColor::Named(NamedColor::Background),
            Flags::INVERSE,
            &colors,
            false,
        );
        assert_eq!((fg, bg), (color32(BACKGROUND_RGB), color32(FOREGROUND_RGB)));
    }
}
