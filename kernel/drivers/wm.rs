use crate::drivers::framebuffer;
use crate::drivers::mouse::MouseState;
use crate::userspace::apps::calculator_gui::Calculator;
use crate::userspace::apps::editor_gui::TextEditor;
use crate::userspace::apps::gui::{Application, Button, Canvas, Event, PointerEvent, Rect};
use alloc::boxed::Box;

const WALLPAPER_TOP: u32 = 0x101C3A;
const WALLPAPER_BOTTOM: u32 = 0x243D68;
const PANEL: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0xA8B5C9;

static mut CONSOLE_INITIALIZED: bool = false;
static mut CONSOLE_X: u32 = 0;
static mut CONSOLE_Y: u32 = 0;

pub struct WindowManager {
    cursor_x: u32,
    cursor_y: u32,
    left_button_down: bool,
    active_app: Option<Box<dyn Application>>,
    calculator_shortcut: Button,
    editor_shortcut: Button,
    close_button: Button,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            cursor_x: framebuffer::get_width() / 2,
            cursor_y: framebuffer::get_height() / 2,
            left_button_down: false,
            active_app: None,
            calculator_shortcut: Button::new("CALCULATOR"),
            editor_shortcut: Button::new("TEXT EDITOR"),
            close_button: Button::new("X"),
        }
    }

    pub fn render(&self) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        if width == 0 || height == 0 {
            return;
        }

        framebuffer::clear_screen(WALLPAPER_TOP);
        for y in 0..height {
            let blend = y * 255 / height;
            let color = blend_rgb(WALLPAPER_TOP, WALLPAPER_BOTTOM, blend);
            framebuffer::fill_rect(0, y, width, 1, color);
        }

        framebuffer::fill_rect(0, 0, width, 40, PANEL);
        framebuffer::fill_rect(0, height.saturating_sub(68), width, 68, PANEL);
        framebuffer::fill_rect(22, 12, 16, 16, ACCENT);
        framebuffer::fill_rect(26, 16, 8, 8, PANEL);
        framebuffer::draw_text(48, 16, "NEBULAOS", TEXT, 2);
        framebuffer::draw_text(200, 17, "DESKTOP", MUTED, 1);
        framebuffer::draw_text(width.saturating_sub(152), 17, "SYSTEM READY", ACCENT, 1);

        let pointer = self.pointer();
        self.draw_shortcut(
            &self.calculator_shortcut,
            1,
            width / 2 - 184,
            height / 2 - 50,
            pointer,
        );
        self.draw_shortcut(
            &self.editor_shortcut,
            2,
            width / 2 + 24,
            height / 2 - 50,
            pointer,
        );
        framebuffer::draw_text(width / 2 - 118, height / 2 + 150, "SELECT AN APP TO OPEN", MUTED, 1);

        if let Some(app) = self.active_app.as_ref() {
            self.draw_window(app.as_ref(), width, height, pointer);
        }
        self.draw_cursor();
    }

    pub fn handle_key(&mut self, key: u8) -> bool {
        if key == 27 {
            if self.active_app.take().is_some() {
                return true;
            }
            return false;
        }

        let area = self.app_area();
        if let Some(app) = self.active_app.as_mut() {
            return app.handle_event(Event::Key(key), area);
        }

        match key {
            b'1' => self.open_app(Box::new(Calculator::new())),
            b'2' => self.open_app(Box::new(TextEditor::new())),
            _ => return false,
        }
        true
    }

    fn open_app(&mut self, app: Box<dyn Application>) {
        self.active_app = Some(app);
    }

    pub fn handle_mouse(&mut self, mouse: MouseState) -> bool {
        let next_x = mouse.x.max(0) as u32;
        let next_y = mouse.y.max(0) as u32;
        let moved = self.cursor_x != next_x || self.cursor_y != next_y;
        self.cursor_x = next_x;
        self.cursor_y = next_y;
        let pointer_changed = mouse.left != self.left_button_down;
        let changed = moved || pointer_changed;
        self.left_button_down = mouse.left;
        let pointer = self.pointer();

        let calculator_rect = self.calculator_shortcut_rect();
        let editor_rect = self.editor_shortcut_rect();
        let close_rect = self.close_button_rect();
        let calculator_clicked = self.calculator_shortcut.handle_pointer(calculator_rect, pointer);
        let editor_clicked = self.editor_shortcut.handle_pointer(editor_rect, pointer);
        let close_clicked = self.close_button.handle_pointer(close_rect, pointer);

        let app_area = self.app_area();
        if let Some(app) = self.active_app.as_mut() {
            if close_clicked {
                self.active_app = None;
                return true;
            }
            return changed | app.handle_event(Event::Pointer(pointer), app_area);
        }

        if calculator_clicked {
            self.open_app(Box::new(Calculator::new()));
            return true;
        }
        if editor_clicked {
            self.open_app(Box::new(TextEditor::new()));
            return true;
        }
        changed
    }

    fn draw_shortcut(
        &self,
        button: &Button,
        number: u8,
        x: u32,
        y: u32,
        pointer: PointerEvent,
    ) {
        let mut canvas = FramebufferCanvas;
        button.draw_chrome(&mut canvas, Rect { x, y, width: 176, height: 144 }, pointer);
        framebuffer::fill_rect(x + 24, y + 20, 48, 48, if number == 1 { 0xF7B955 } else { ACCENT });
        if number == 1 {
            framebuffer::fill_rect(x + 34, y + 29, 28, 4, PANEL);
            framebuffer::fill_rect(x + 34, y + 39, 28, 4, PANEL);
            framebuffer::fill_rect(x + 34, y + 49, 28, 4, PANEL);
        } else {
            framebuffer::fill_rect(x + 31, y + 27, 34, 30, PANEL);
            framebuffer::fill_rect(x + 36, y + 32, 24, 20, 0xDCEAF5);
        }
        framebuffer::draw_text(x + 24, y + 88, button_label(number), TEXT, 1);
        framebuffer::draw_text(x + 24, y + 116, if number == 1 { "KEY 1" } else { "KEY 2" }, MUTED, 1);
    }

    fn draw_window(
        &self,
        app: &dyn Application,
        screen_width: u32,
        screen_height: u32,
        pointer: PointerEvent,
    ) {
        let width = 420.min(screen_width.saturating_sub(48));
        let height = 300.min(screen_height.saturating_sub(120));
        let x = (screen_width - width) / 2;
        let y = (screen_height - height) / 2;
        framebuffer::fill_rect(x + 6, y + 8, width, height, 0x080D16);
        framebuffer::fill_rect(x, y, width, height, 0xF2F5FA);
        framebuffer::fill_rect(x, y, width, 42, PANEL);
        framebuffer::draw_text(x + 18, y + 17, app.title(), TEXT, 1);
        let mut canvas = FramebufferCanvas;
        self.close_button.render(&mut canvas, self.close_button_rect(), pointer);

        app.render(&mut canvas, self.app_area(), pointer);
    }

    fn app_area(&self) -> Rect {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        let window_width = 420.min(width.saturating_sub(48));
        let window_height = 300.min(height.saturating_sub(120));
        Rect {
            x: (width - window_width) / 2,
            y: (height - window_height) / 2 + 42,
            width: window_width,
            height: window_height.saturating_sub(42),
        }
    }

    fn close_button_rect(&self) -> Rect {
        let width = 420.min(framebuffer::get_width().saturating_sub(48));
        let height = 300.min(framebuffer::get_height().saturating_sub(120));
        let x = (framebuffer::get_width() - width) / 2;
        let y = (framebuffer::get_height() - height) / 2;
        Rect { x: x + width - 34, y: y + 10, width: 22, height: 22 }
    }

    fn calculator_shortcut_rect(&self) -> Rect {
        Rect {
            x: framebuffer::get_width() / 2 - 184,
            y: framebuffer::get_height() / 2 - 50,
            width: 176,
            height: 144,
        }
    }

    fn editor_shortcut_rect(&self) -> Rect {
        Rect {
            x: framebuffer::get_width() / 2 + 24,
            y: framebuffer::get_height() / 2 - 50,
            width: 176,
            height: 144,
        }
    }

    fn pointer(&self) -> PointerEvent {
        PointerEvent {
            x: self.cursor_x,
            y: self.cursor_y,
            left_down: self.left_button_down,
        }
    }

    fn draw_cursor(&self) {
        for row in 0..16 {
            for column in 0..(row / 2 + 1) {
                if column == 0 || column == row / 2 {
                    framebuffer::put_pixel(self.cursor_x + column, self.cursor_y + row, 0x101820);
                } else {
                    framebuffer::put_pixel(self.cursor_x + column, self.cursor_y + row, TEXT);
                }
            }
        }
    }
}

struct FramebufferCanvas;

impl Canvas for FramebufferCanvas {
    fn fill_rect(&mut self, rect: Rect, color: u32) {
        framebuffer::fill_rect(rect.x, rect.y, rect.width, rect.height, color);
    }

    fn draw_rect(&mut self, rect: Rect, color: u32) {
        framebuffer::draw_rect(rect.x, rect.y, rect.width, rect.height, color);
    }

    fn draw_text(&mut self, x: u32, y: u32, text: &str, color: u32, scale: u32) {
        framebuffer::draw_text(x, y, text, color, scale);
    }
}

fn blend_rgb(from: u32, to: u32, amount: u32) -> u32 {
    let mix = |shift: u32| {
        let first = (from >> shift) & 0xFFu32;
        let second = (to >> shift) & 0xFFu32;
        (first * (255 - amount) + second * amount) / 255
    };
    mix(16) << 16 | mix(8) << 8 | mix(0)
}

fn button_label(number: u8) -> &'static str {
    if number == 1 { "CALCULATOR" } else { "TEXT EDITOR" }
}

pub fn console_write(bytes: &[u8]) {
    let width = framebuffer::get_width();
    let height = framebuffer::get_height();
    if width == 0 || height < 180 {
        return;
    }

    unsafe {
        let left = 24;
        let top = height - 178;
        if !CONSOLE_INITIALIZED {
            framebuffer::fill_rect(left, top, (width / 2).min(620), 96, PANEL);
            framebuffer::draw_rect(left, top, (width / 2).min(620), 96, 0x516786);
            framebuffer::draw_text(left + 12, top + 10, "SYSTEM CONSOLE", ACCENT, 1);
            CONSOLE_X = left + 12;
            CONSOLE_Y = top + 30;
            CONSOLE_INITIALIZED = true;
        }

        let left = left + 12;
        let right = left + (width / 2).min(620) - 24;
        for &byte in bytes {
            if byte == b'\r' {
                continue;
            }
            if byte == b'\n' || CONSOLE_X + 6 >= right {
                CONSOLE_X = left;
                CONSOLE_Y += 12;
                if CONSOLE_Y + 8 >= top + 96 {
                    framebuffer::fill_rect(left, top + 30, (width / 2).min(620) - 24, 58, PANEL);
                    CONSOLE_Y = top + 30;
                }
                if byte == b'\n' {
                    continue;
                }
            }
            if byte.is_ascii() {
                let glyph = [byte];
                if let Ok(character) = core::str::from_utf8(&glyph) {
                    framebuffer::draw_text(CONSOLE_X, CONSOLE_Y, character, TEXT, 1);
                }
            }
            CONSOLE_X += 6;
        }
    }
}
