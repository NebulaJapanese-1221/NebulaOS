use crate::drivers::framebuffer;
use crate::drivers::mouse::MouseState;
use crate::userspace::apps::calculator_gui::Calculator;
use crate::userspace::apps::editor_gui::TextEditor;
use crate::userspace::apps::files_gui::FileManager;
use crate::userspace::apps::gui::{Application, Button, Canvas, Event, PointerEvent, Rect};
use crate::userspace::apps::monitor_gui::SystemMonitor;
use crate::userspace::apps::paint_gui::Paint;
use crate::userspace::apps::settings_gui::Settings;
use alloc::boxed::Box;
use alloc::vec::Vec;

const WALLPAPER_TOP: u32 = 0x101C3A;
const WALLPAPER_BOTTOM: u32 = 0x243D68;
const PANEL: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0xA8B5C9;
const APP_COUNT: usize = 6;
static mut DARK_THEME: bool = true;

pub fn set_dark_theme(enabled: bool) {
    unsafe { DARK_THEME = enabled; }
}

struct Window {
    app: Box<dyn Application>,
    bounds: Rect,
    restore_bounds: Rect,
    minimized: bool,
    maximized: bool,
}

#[derive(Clone, Copy)]
enum Capture {
    None,
    Launch(usize),
    Drag(usize),
    Close(usize),
    Minimize(usize),
    Maximize(usize),
    Restore(usize),
}

pub struct WindowManager {
    cursor_x: u32,
    cursor_y: u32,
    left_button_down: bool,
    windows: Vec<Window>,
    focused: Option<usize>,
    capture: Capture,
    drag_offset: (u32, u32),
    shortcuts: [Button; APP_COUNT],
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            cursor_x: framebuffer::get_width() / 2,
            cursor_y: framebuffer::get_height() / 2,
            left_button_down: false,
            windows: Vec::new(),
            focused: None,
            capture: Capture::None,
            drag_offset: (0, 0),
            shortcuts: [
                Button::new("CALCULATOR"), Button::new("TEXT EDITOR"),
                Button::new("FILE MANAGER"), Button::new("SETTINGS"),
                Button::new("PAINT"), Button::new("SYSTEM MONITOR"),
            ],
        }
    }

    pub fn render(&self) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        if width == 0 || height == 0 { return; }
        let (wallpaper_top, wallpaper_bottom) = unsafe {
            if DARK_THEME {
                (WALLPAPER_TOP, WALLPAPER_BOTTOM)
            } else {
                (0x526783, 0x9DB6C9)
            }
        };
        framebuffer::clear_screen(wallpaper_top);
        for y in 0..height {
            framebuffer::fill_rect(0, y, width, 1, blend_rgb(
                wallpaper_top, wallpaper_bottom, y * 255 / height,
            ));
        }
        framebuffer::fill_rect(0, 0, width, 40, PANEL);
        framebuffer::fill_rect(0, height.saturating_sub(42), width, 42, PANEL);
        framebuffer::fill_rect(22, 12, 16, 16, ACCENT);
        framebuffer::fill_rect(26, 16, 8, 8, PANEL);
        framebuffer::draw_text(48, 16, "NEBULAOS", TEXT, 2);
        framebuffer::draw_text(200, 17, "DESKTOP", MUTED, 1);
        framebuffer::draw_text(width.saturating_sub(152), 17, "SYSTEM READY", ACCENT, 1);

        let pointer = self.pointer();
        self.draw_shortcuts(pointer);
        for index in 0..self.windows.len() {
            if !self.windows[index].minimized {
                self.draw_window(index, pointer);
            }
        }
        self.draw_minimized_apps(pointer);
        self.draw_cursor();
    }

    fn draw_shortcuts(&self, pointer: PointerEvent) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        let card_width = 176;
        let card_height = 116;
        let gap = 16;
        let grid_width = 3 * card_width + 2 * gap;
        let origin_x = width.saturating_sub(grid_width) / 2;
        let origin_y = (height / 2).saturating_sub(132);
        for index in 0..APP_COUNT {
            let rect = shortcut_rect(index, origin_x, origin_y, card_width, card_height, gap);
            let mut canvas = FramebufferCanvas;
            self.shortcuts[index].draw_chrome(&mut canvas, rect, pointer);
            framebuffer::fill_rect(rect.x + 18, rect.y + 18, 38, 38, app_color(index));
            framebuffer::draw_text(rect.x + 18, rect.y + 75, app_name(index), TEXT, 1);
            framebuffer::draw_text(rect.x + 18, rect.y + 96, "OPEN APP", MUTED, 1);
        }
    }

    fn draw_window(&self, index: usize, pointer: PointerEvent) {
        let window = &self.windows[index];
        let rect = window.bounds;
        let is_focused = self.focused == Some(index);
        framebuffer::fill_rect(rect.x + 6, rect.y + 8, rect.width, rect.height, 0x080D16);
        framebuffer::fill_rect(rect.x, rect.y, rect.width, rect.height, 0xF2F5FA);
        framebuffer::fill_rect(rect.x, rect.y, rect.width, 42, if is_focused { PANEL } else { 0x263247 });
        framebuffer::draw_text(rect.x + 14, rect.y + 17, window.app.title(), TEXT, 1);
        let mut canvas = FramebufferCanvas;
        draw_window_button(&mut canvas, window_button_rect(rect, 0), "X", 0xE45B67, pointer);
        draw_window_button(&mut canvas, window_button_rect(rect, 1), "_", 0x42546D, pointer);
        draw_window_button(&mut canvas, window_button_rect(rect, 2), if window.maximized { "o" } else { "[]" }, 0x42546D, pointer);
        let area = app_area(rect);
        window.app.render(&mut canvas, area, pointer);
        if is_focused {
            framebuffer::draw_rect(rect.x, rect.y, rect.width, rect.height, ACCENT);
        }
    }

    fn draw_minimized_apps(&self, pointer: PointerEvent) {
        let height = framebuffer::get_height();
        let mut x = 16;
        for window in &self.windows {
            if !window.minimized { continue; }
            let rect = Rect { x, y: height.saturating_sub(34), width: 148, height: 24 };
            let mut canvas = FramebufferCanvas;
            draw_window_button(&mut canvas, rect, window.app.title(), 0x26374F, pointer);
            x += 156;
        }
    }

    pub fn handle_key(&mut self, key: u8) -> bool {
        if key == 27 {
            if let Some(index) = self.focused {
                if index < self.windows.len() {
                    self.windows.remove(index);
                    self.focused = self.windows.iter().rposition(|window| !window.minimized);
                    self.capture = Capture::None;
                    return true;
                }
            }
            return false;
        }
        if let Some(index) = self.focused {
            let window = &mut self.windows[index];
            return window.app.handle_event(Event::Key(key), app_area(window.bounds));
        }
        if let Some(app_index) = match key { b'1'..=b'6' => Some((key - b'1') as usize), _ => None } {
            self.open_app(app_index);
            return true;
        }
        false
    }

    fn open_app(&mut self, index: usize) {
        let Some(app) = make_app(index) else { return };
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        let offset = (self.windows.len() as u32 * 28) % 140;
        let bounds = Rect {
            x: (width.saturating_sub(460) / 2 + offset).min(width.saturating_sub(440)),
            y: (height.saturating_sub(340) / 2 + offset).min(height.saturating_sub(320)),
            width: 420.min(width.saturating_sub(24)),
            height: 300.min(height.saturating_sub(80)),
        };
        self.windows.push(Window {
            app, bounds, restore_bounds: bounds, minimized: false, maximized: false,
        });
        self.focused = Some(self.windows.len() - 1);
    }

    pub fn handle_mouse(&mut self, mouse: MouseState) -> bool {
        let x = mouse.x.max(0) as u32;
        let y = mouse.y.max(0) as u32;
        let moved = self.cursor_x != x || self.cursor_y != y;
        self.cursor_x = x;
        self.cursor_y = y;
        let was_down = self.left_button_down;
        self.left_button_down = mouse.left;
        let pointer = self.pointer();
        if !was_down && mouse.left {
            self.capture = self.capture_press(pointer);
        } else if was_down && mouse.left {
            if let Capture::Drag(index) = self.capture {
                if let Some(window) = self.windows.get_mut(index) {
                    window.bounds.x = x.saturating_sub(self.drag_offset.0);
                    window.bounds.y = y.saturating_sub(self.drag_offset.1).max(40);
                }
            }
        }

        let mut changed = moved || was_down != mouse.left;
        if was_down && !mouse.left {
            changed |= self.capture_release(pointer);
            self.capture = Capture::None;
        }
        if let Some(index) = self.focused {
            if index < self.windows.len() && !self.windows[index].minimized {
                let area = app_area(self.windows[index].bounds);
                changed |= self.windows[index].app.handle_event(Event::Pointer(pointer), area);
            }
        }
        changed
    }

    fn capture_press(&mut self, pointer: PointerEvent) -> Capture {
        for index in (0..self.windows.len()).rev() {
            if self.windows[index].minimized { continue; }
            let bounds = self.windows[index].bounds;
            if !bounds.contains(pointer.x, pointer.y) { continue; }
            self.focus_window(index);
            let focused = self.windows.len() - 1;
            if window_button_rect(bounds, 0).contains(pointer.x, pointer.y) { return Capture::Close(focused); }
            if window_button_rect(bounds, 1).contains(pointer.x, pointer.y) { return Capture::Minimize(focused); }
            if window_button_rect(bounds, 2).contains(pointer.x, pointer.y) { return Capture::Maximize(focused); }
            if pointer.y < bounds.y + 42 {
                self.drag_offset = (pointer.x - bounds.x, pointer.y - bounds.y);
                return Capture::Drag(focused);
            }
            return Capture::None;
        }
        let mut minimized_slot = 0;
        for index in 0..self.windows.len() {
            if self.windows[index].minimized {
                let rect = Rect {
                    x: 16 + minimized_slot * 156,
                    y: framebuffer::get_height().saturating_sub(34),
                    width: 148,
                    height: 24,
                };
                if rect.contains(pointer.x, pointer.y) {
                    self.focus_window(index);
                    return Capture::Restore(self.windows.len() - 1);
                }
                minimized_slot += 1;
            }
        }
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        let origin_x = width.saturating_sub(3 * 176 + 2 * 16) / 2;
        let origin_y = (height / 2).saturating_sub(132);
        for index in 0..APP_COUNT {
            if shortcut_rect(index, origin_x, origin_y, 176, 116, 16).contains(pointer.x, pointer.y) {
                return Capture::Launch(index);
            }
        }
        Capture::None
    }

    fn capture_release(&mut self, pointer: PointerEvent) -> bool {
        match self.capture {
            Capture::Launch(index) => {
                let width = framebuffer::get_width();
                let height = framebuffer::get_height();
                let origin_x = width.saturating_sub(3 * 176 + 2 * 16) / 2;
                let origin_y = (height / 2).saturating_sub(132);
                if shortcut_rect(index, origin_x, origin_y, 176, 116, 16).contains(pointer.x, pointer.y) {
                    self.open_app(index);
                    return true;
                }
            }
            Capture::Close(index) => {
                if index < self.windows.len() && window_button_rect(self.windows[index].bounds, 0).contains(pointer.x, pointer.y) {
                    self.windows.remove(index);
                    self.focused = self.windows.iter().rposition(|window| !window.minimized);
                    return true;
                }
            }
            Capture::Minimize(index) => {
                if let Some(window) = self.windows.get_mut(index) {
                    if window_button_rect(window.bounds, 1).contains(pointer.x, pointer.y) {
                        window.minimized = true;
                        if self.focused == Some(index) {
                            self.focused = self.windows.iter().rposition(|window| !window.minimized);
                        }
                        return true;
                    }
                }
            }
            Capture::Maximize(index) => {
                if let Some(window) = self.windows.get_mut(index) {
                    if window.maximized {
                        window.bounds = window.restore_bounds;
                        window.maximized = false;
                    } else {
                        window.restore_bounds = window.bounds;
                        window.bounds = Rect {
                            x: 8, y: 44,
                            width: framebuffer::get_width().saturating_sub(16),
                            height: framebuffer::get_height().saturating_sub(92),
                        };
                        window.maximized = true;
                    }
                    return true;
                }
            }
            Capture::Restore(index) => {
                if let Some(window) = self.windows.get_mut(index) {
                    window.minimized = false;
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn focus_window(&mut self, index: usize) {
        if index >= self.windows.len() { return; }
        let window = self.windows.remove(index);
        self.windows.push(window);
        self.focused = Some(self.windows.len() - 1);
    }

    fn pointer(&self) -> PointerEvent {
        PointerEvent { x: self.cursor_x, y: self.cursor_y, left_down: self.left_button_down }
    }

    fn draw_cursor(&self) {
        for row in 0..16 {
            for column in 0..(row / 2 + 1) {
                framebuffer::put_pixel(self.cursor_x + column, self.cursor_y + row, if column == 0 || column == row / 2 { 0x101820 } else { TEXT });
            }
        }
    }
}

fn make_app(index: usize) -> Option<Box<dyn Application>> {
    Some(match index {
        0 => Box::new(Calculator::new()),
        1 => Box::new(TextEditor::new()),
        2 => Box::new(FileManager::new()),
        3 => Box::new(Settings::new()),
        4 => Box::new(Paint::new()),
        5 => Box::new(SystemMonitor::new()),
        _ => return None,
    })
}

struct FramebufferCanvas;

impl Canvas for FramebufferCanvas {
    fn fill_rect(&mut self, rect: Rect, color: u32) { framebuffer::fill_rect(rect.x, rect.y, rect.width, rect.height, color); }
    fn draw_rect(&mut self, rect: Rect, color: u32) { framebuffer::draw_rect(rect.x, rect.y, rect.width, rect.height, color); }
    fn draw_text(&mut self, x: u32, y: u32, text: &str, color: u32, scale: u32) { framebuffer::draw_text(x, y, text, color, scale); }
}

fn app_area(bounds: Rect) -> Rect {
    Rect { x: bounds.x, y: bounds.y + 42, width: bounds.width, height: bounds.height.saturating_sub(42) }
}

fn window_button_rect(bounds: Rect, button: usize) -> Rect {
    let x = bounds.x + bounds.width.saturating_sub(34 + button as u32 * 30);
    Rect { x, y: bounds.y + 10, width: 22, height: 22 }
}

fn draw_window_button(canvas: &mut dyn Canvas, rect: Rect, label: &str, color: u32, pointer: PointerEvent) {
    let hover = rect.contains(pointer.x, pointer.y);
    canvas.fill_rect(rect, if hover { 0x1596B5 } else { color });
    canvas.draw_rect(rect, if hover { ACCENT } else { 0x516786 });
    canvas.draw_text(rect.x + 5, rect.y + rect.height.saturating_sub(8) / 2, label, TEXT, 1);
}

fn shortcut_rect(index: usize, x: u32, y: u32, width: u32, height: u32, gap: u32) -> Rect {
    Rect { x: x + (index % 3) as u32 * (width + gap), y: y + (index / 3) as u32 * (height + gap), width, height }
}

fn app_name(index: usize) -> &'static str {
    match index { 0 => "CALCULATOR", 1 => "TEXT EDITOR", 2 => "FILE MANAGER", 3 => "SETTINGS", 4 => "PAINT", _ => "SYSTEM MONITOR" }
}

fn app_color(index: usize) -> u32 {
    match index { 0 => 0xF7B955, 1 => ACCENT, 2 => 0x8ED081, 3 => 0xB29DDB, 4 => 0xE45B67, _ => 0x66D9EF }
}

fn blend_rgb(from: u32, to: u32, amount: u32) -> u32 {
    let mix = |shift: u32| {
        let first = (from >> shift) & 0xFF;
        let second = (to >> shift) & 0xFF;
        (first * (255 - amount) + second * amount) / 255
    };
    mix(16) << 16 | mix(8) << 8 | mix(0)
}

static mut CONSOLE_INITIALIZED: bool = false;
static mut CONSOLE_X: u32 = 0;
static mut CONSOLE_Y: u32 = 0;

pub fn console_write(bytes: &[u8]) {
    let width = framebuffer::get_width();
    let height = framebuffer::get_height();
    if width == 0 || height < 180 { return; }
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
            if byte == b'\r' { continue; }
            if byte == b'\n' || CONSOLE_X + 6 >= right {
                CONSOLE_X = left;
                CONSOLE_Y += 12;
                if CONSOLE_Y + 8 >= top + 96 {
                    framebuffer::fill_rect(left, top + 30, (width / 2).min(620) - 24, 58, PANEL);
                    CONSOLE_Y = top + 30;
                }
                if byte == b'\n' { continue; }
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
