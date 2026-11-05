// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

use crate::drivers::framebuffer;
use crate::drivers::mouse::MouseState;
use crate::sync::SpinLock;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Clone, Copy)]
struct PointerEvent {
    x: u32,
    y: u32,
    left_down: bool,
}

const WALLPAPER_TOP: u32 = 0x101C3A;
const WALLPAPER_BOTTOM: u32 = 0x243D68;
const PANEL: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0xA8B5C9;
const APP_COUNT: usize = 8;
const VERSION: &str = env!("CARGO_PKG_VERSION");
static DARK_THEME: AtomicBool = AtomicBool::new(true);
static THEME_CHANGED: AtomicBool = AtomicBool::new(false);
static DESKTOP_REDRAW_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_desktop_redraw() {
    DESKTOP_REDRAW_REQUESTED.store(true, Ordering::Release);
}

pub fn take_desktop_redraw_request() -> bool {
    DESKTOP_REDRAW_REQUESTED.swap(false, Ordering::AcqRel)
}

pub fn set_dark_theme(enabled: bool) {
    if DARK_THEME.swap(enabled, Ordering::AcqRel) != enabled {
        THEME_CHANGED.store(true, Ordering::Release);
    }
}

pub fn take_theme_changed() -> bool {
    THEME_CHANGED.swap(false, Ordering::AcqRel)
}

pub fn is_dark_theme() -> bool {
    DARK_THEME.load(Ordering::Acquire)
}

struct Window {
    pid: u32,
    kind: AppKind,
    bounds: Rect,
    restore_bounds: Rect,
    minimized: bool,
    maximized: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppKind {
    Calculator,
    TextEditor,
    FileManager,
    Settings,
    Paint,
    SystemMonitor,
    UserDemo,
    Terminal,
}

impl AppKind {
    fn from_menu_index(index: usize) -> Option<Self> {
        Some(match index {
            0 => Self::Calculator,
            1 => Self::TextEditor,
            2 => Self::FileManager,
            3 => Self::Settings,
            4 => Self::Paint,
            5 => Self::SystemMonitor,
            6 => Self::UserDemo,
            7 => Self::Terminal,
            _ => return None,
        })
    }

    fn title(self) -> &'static str {
        match self {
            Self::Calculator => "CALCULATOR",
            Self::TextEditor => "TEXT EDITOR",
            Self::FileManager => "FILE MANAGER",
            Self::Settings => "SETTINGS",
            Self::Paint => "PAINT",
            Self::SystemMonitor => "SYSTEM MONITOR",
            Self::UserDemo => "RING-3 DEMO",
            Self::Terminal => "TERMINAL",
        }
    }
}

#[derive(Clone, Copy)]
enum Capture {
    None,
    Drag(usize),
    Close(usize),
    Minimize(usize),
    Maximize(usize),
    StartMenu,
    StartApp(usize),
    TaskbarWindow(usize),
    DismissStart,
}

pub struct WindowManager {
    cursor_x: u32,
    cursor_y: u32,
    cursor_type: framebuffer::CursorType,
    left_button_down: bool,
    windows: Vec<Window>,
    focused: Option<usize>,
    capture: Capture,
    drag_offset: (u32, u32),
    start_menu_open: bool,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            cursor_x: framebuffer::get_width() / 2,
            cursor_y: framebuffer::get_height() / 2,
            cursor_type: framebuffer::CursorType::Arrow,
            left_button_down: false,
            windows: Vec::new(),
            focused: None,
            capture: Capture::None,
            drag_offset: (0, 0),
            start_menu_open: false,
        }
    }

    pub fn render(&mut self) {
        self.prune_exited_windows();
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        if width == 0 || height == 0 {
            return;
        }
        let (wallpaper_top, wallpaper_bottom) = if DARK_THEME.load(Ordering::Acquire) {
            (WALLPAPER_TOP, WALLPAPER_BOTTOM)
        } else {
            (0x526783, 0x9DB6C9)
        };
        framebuffer::clear_screen(wallpaper_top);
        for y in 0..height {
            framebuffer::fill_rect(
                0,
                y,
                width,
                1,
                blend_rgb(wallpaper_top, wallpaper_bottom, y * 255 / height),
            );
        }
        framebuffer::fill_rect(0, 0, width, 40, PANEL);
        framebuffer::fill_rect(0, height.saturating_sub(44), width, 44, PANEL);
        framebuffer::fill_rect(22, 12, 16, 16, ACCENT);
        framebuffer::fill_rect(26, 16, 8, 8, PANEL);
        framebuffer::draw_text(48, 16, "NEBULAOS", TEXT, 2);
        framebuffer::draw_text(200, 17, "VERSION ", MUTED, 1);
        framebuffer::draw_text(254, 17, VERSION, ACCENT, 1);
        framebuffer::draw_text(width.saturating_sub(152), 17, "SYSTEM READY", ACCENT, 1);

        let pointer = self.pointer();
        for index in 0..self.windows.len() {
            if !self.windows[index].minimized {
                self.draw_window(index, pointer);
            }
        }
        self.draw_taskbar(pointer);
        if self.start_menu_open {
            self.draw_start_menu(pointer);
        }
        framebuffer::set_cursor(self.cursor_x, self.cursor_y, self.cursor_type);
        framebuffer::present();
        for window in &self.windows {
            if !window.minimized {
                let area = app_area(window.bounds);
                let _ = crate::services::user_process::request_redraw(
                    window.pid,
                    crate::services::process::AppRect {
                        x: area.x,
                        y: area.y,
                        width: area.width,
                        height: area.height,
                    },
                    pointer.x,
                    pointer.y,
                    pointer.left_down,
                );
            }
        }
    }

    pub fn refresh_periodic_apps(&self) {
        let pointer = self.pointer();
        for window in &self.windows {
            if window.kind == AppKind::SystemMonitor && !window.minimized {
                let area = app_area(window.bounds);
                let _ = crate::services::user_process::request_redraw(
                    window.pid,
                    crate::services::process::AppRect {
                        x: area.x,
                        y: area.y,
                        width: area.width,
                        height: area.height,
                    },
                    pointer.x,
                    pointer.y,
                    pointer.left_down,
                );
            }
        }
    }

    fn draw_taskbar(&self, pointer: PointerEvent) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        let taskbar_y = height.saturating_sub(44);
        let start_rect = start_button_rect(height);
        draw_window_button(start_rect, "START", 0x285A72, pointer);
        let mut x = 132;
        for (index, window) in self.windows.iter().enumerate() {
            if x + 156 > width.saturating_sub(150) {
                break;
            }
            let rect = Rect {
                x,
                y: taskbar_y + 8,
                width: 148,
                height: 28,
            };
            draw_window_button(
                rect,
                window.kind.title(),
                if self.focused == Some(index) {
                    0x285A72
                } else {
                    0x26374F
                },
                pointer,
            );
            x += 156;
        }
        framebuffer::draw_text(
            width.saturating_sub(132),
            taskbar_y + 18,
            VERSION,
            ACCENT,
            1,
        );
    }

    fn draw_start_menu(&self, pointer: PointerEvent) {
        let rect = start_menu_rect(framebuffer::get_height());
        framebuffer::fill_rect(rect.x + 5, rect.y + 5, rect.width, rect.height, 0x080D16);
        framebuffer::fill_rect(rect.x, rect.y, rect.width, rect.height, PANEL);
        framebuffer::draw_rect(rect.x, rect.y, rect.width, rect.height, 0x516786);
        framebuffer::draw_text(rect.x + 16, rect.y + 14, "NEBULAOS", TEXT, 1);
        framebuffer::draw_text(rect.x + 16, rect.y + 30, "VERSION", MUTED, 1);
        framebuffer::draw_text(rect.x + 76, rect.y + 30, VERSION, ACCENT, 1);
        for index in 0..APP_COUNT {
            let row = start_menu_item_rect(rect, index);
            if let Some(kind) = AppKind::from_menu_index(index) {
                draw_window_button(row, kind.title(), 0x26374F, pointer);
            }
        }
    }

    fn draw_window(&self, index: usize, pointer: PointerEvent) {
        let window = &self.windows[index];
        let rect = window.bounds;
        let is_focused = self.focused == Some(index);
        framebuffer::fill_rect(rect.x + 6, rect.y + 8, rect.width, rect.height, 0x080D16);
        framebuffer::fill_rect(rect.x, rect.y, rect.width, rect.height, 0xF2F5FA);
        framebuffer::fill_rect(
            rect.x,
            rect.y,
            rect.width,
            42,
            if is_focused { PANEL } else { 0x263247 },
        );
        framebuffer::draw_text(rect.x + 14, rect.y + 17, window.kind.title(), TEXT, 1);
        draw_window_button(window_button_rect(rect, 0), "X", 0xE45B67, pointer);
        draw_window_button(window_button_rect(rect, 1), "_", 0x42546D, pointer);
        draw_window_button(
            window_button_rect(rect, 2),
            if window.maximized { "o" } else { "[]" },
            0x42546D,
            pointer,
        );
        let area = app_area(rect);
        framebuffer::fill_rect(area.x, area.y, area.width, area.height, 0xF2F5FA);
        if is_focused {
            framebuffer::draw_rect(rect.x, rect.y, rect.width, rect.height, ACCENT);
        }
    }

    pub fn handle_key(&mut self, key: u8) -> bool {
        self.prune_exited_windows();
        if key == 27 && self.start_menu_open {
            self.start_menu_open = false;
            return true;
        }
        if key == b'\t' && self.focused.is_none() {
            self.start_menu_open = !self.start_menu_open;
            return true;
        }
        if key == 27 {
            if let Some(index) = self.focused {
                if index < self.windows.len() {
                    crate::services::user_process::terminate(self.windows[index].pid);
                    self.windows.remove(index);
                    self.focused = self.windows.iter().rposition(|window| !window.minimized);
                    self.capture = Capture::None;
                    self.request_focused_redraw();
                    return true;
                }
            }
            return false;
        }
        if let Some(index) = self.focused {
            let _ = crate::services::user_process::route_keyboard_event(
                self.windows[index].pid,
                key as u32,
            );
            return true;
        }
        if let Some(app_index) = match key {
            b'1'..=b'8' => Some((key - b'1') as usize),
            _ => None,
        } {
            self.open_app(app_index);
            return true;
        }
        false
    }

    fn open_app(&mut self, index: usize) {
        let Some(kind) = AppKind::from_menu_index(index) else {
            return;
        };
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        let offset = (self.windows.len() as u32 * 28) % 140;
        let bounds = Rect {
            x: (width.saturating_sub(460) / 2 + offset).min(width.saturating_sub(440)),
            y: (height.saturating_sub(340) / 2 + offset).min(height.saturating_sub(320)),
            width: 420.min(width.saturating_sub(24)),
            height: 300.min(height.saturating_sub(80)),
        };
        let app_id = index as u32;
        let area = app_area(bounds);
        let content = crate::services::process::AppRect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        };
        let pid = match crate::services::user_process::spawn_app(app_id, content) {
            Ok(pid) => pid,
            Err(error) => {
                crate::drivers::serial::write_string("Unable to launch ");
                crate::drivers::serial::write_string(kind.title());
                crate::drivers::serial::write_string(": ");
                crate::drivers::serial::write_string(match error {
                    crate::services::user_process::UserProcessError::OutOfMemory => "out of memory",
                    crate::services::user_process::UserProcessError::InvalidElf => "invalid ELF",
                    crate::services::user_process::UserProcessError::NoLoadableSegments => {
                        "no loadable ELF segments"
                    }
                    crate::services::user_process::UserProcessError::InvalidEntryPoint => {
                        "invalid ELF entry point"
                    }
                    crate::services::user_process::UserProcessError::LoadFailed => {
                        "ELF load failed"
                    }
                });
                crate::drivers::serial::write_string(".\r\n");
                return;
            }
        };
        self.windows.push(Window {
            pid,
            kind,
            bounds,
            restore_bounds: bounds,
            minimized: false,
            maximized: false,
        });
        self.focused = Some(self.windows.len() - 1);
        self.request_focused_redraw();
    }

    pub fn handle_mouse(&mut self, mouse: MouseState) -> bool {
        self.prune_exited_windows();
        let x = mouse.x.max(0) as u32;
        let y = mouse.y.max(0) as u32;
        let previous_x = self.cursor_x;
        let previous_y = self.cursor_y;
        let previous_hover = self.hover_target_at(previous_x, previous_y);
        let moved = self.cursor_x != x || self.cursor_y != y;
        self.cursor_x = x;
        self.cursor_y = y;
        let was_down = self.left_button_down;
        self.left_button_down = mouse.left;
        let button_changed = was_down != mouse.left;
        let pointer = self.pointer();
        if !was_down && mouse.left {
            self.capture = self.capture_press(pointer);
        } else if was_down && mouse.left {
            if let Capture::Drag(index) = self.capture {
                if let Some(window) = self.windows.get_mut(index) {
                    window.bounds.x = x
                        .saturating_sub(self.drag_offset.0)
                        .min(framebuffer::get_width().saturating_sub(window.bounds.width));
                    window.bounds.y = y
                        .saturating_sub(self.drag_offset.1)
                        .max(40)
                        .min(framebuffer::get_height().saturating_sub(window.bounds.height + 44));
                }
            }
        }

        let mut redraw = button_changed || previous_hover != self.hover_target_at(x, y);
        if was_down && !mouse.left {
            redraw |= self.capture_release(pointer);
            self.capture = Capture::None;
        }
        let pointer_over_app = self.focused.is_some_and(|index| {
            self.windows.get(index).is_some_and(|window| {
                !window.minimized
                    && (app_area(window.bounds).contains(x, y)
                        || app_area(window.bounds).contains(previous_x, previous_y))
            })
        });
        if !self.start_menu_open && (button_changed || (moved && (mouse.left || pointer_over_app)))
        {
            if let Some(index) = self.focused {
                if index < self.windows.len() && !self.windows[index].minimized {
                    let _ = crate::services::user_process::route_pointer_event(
                        self.windows[index].pid,
                        pointer.x,
                        pointer.y,
                        pointer.left_down,
                    );
                }
            }
        }
        if matches!(self.capture, Capture::Drag(_)) && moved {
            self.request_focused_redraw();
            redraw = true;
        }
        let next_cursor_type = self.cursor_type_at(x, y);
        let cursor_changed = moved || self.cursor_type != next_cursor_type;
        self.cursor_type = next_cursor_type;
        if cursor_changed {
            framebuffer::set_cursor(x, y, next_cursor_type);
            if !redraw {
                framebuffer::present_cursor_update();
            }
        }
        redraw
    }

    fn hover_target_at(&self, x: u32, y: u32) -> Option<(u8, usize, usize)> {
        if start_button_rect(framebuffer::get_height()).contains(x, y) {
            return Some((0, 0, 0));
        }
        if self.start_menu_open {
            let menu = start_menu_rect(framebuffer::get_height());
            for index in 0..APP_COUNT {
                if start_menu_item_rect(menu, index).contains(x, y) {
                    return Some((1, index, 0));
                }
            }
            return None;
        }

        let taskbar_y = framebuffer::get_height().saturating_sub(44);
        if y >= taskbar_y {
            let mut taskbar_x = 132;
            for index in 0..self.windows.len() {
                if taskbar_x + 156 > framebuffer::get_width().saturating_sub(150) {
                    break;
                }
                let rect = Rect {
                    x: taskbar_x,
                    y: taskbar_y + 8,
                    width: 148,
                    height: 28,
                };
                if rect.contains(x, y) {
                    return Some((2, index, 0));
                }
                taskbar_x += 156;
            }
            return None;
        }

        for (index, window) in self.windows.iter().enumerate().rev() {
            if window.minimized || !window.bounds.contains(x, y) {
                continue;
            }
            for button in 0..3 {
                if window_button_rect(window.bounds, button).contains(x, y) {
                    return Some((3, index, button));
                }
            }
            break;
        }
        None
    }

    fn cursor_type_at(&self, x: u32, y: u32) -> framebuffer::CursorType {
        if start_button_rect(framebuffer::get_height()).contains(x, y) {
            return framebuffer::CursorType::Hand;
        }
        if self.start_menu_open {
            let menu = start_menu_rect(framebuffer::get_height());
            if (0..APP_COUNT).any(|index| start_menu_item_rect(menu, index).contains(x, y)) {
                return framebuffer::CursorType::Hand;
            }
            return framebuffer::CursorType::Arrow;
        }

        let taskbar_y = framebuffer::get_height().saturating_sub(44);
        if y >= taskbar_y {
            let mut taskbar_x = 132;
            for _ in &self.windows {
                if taskbar_x + 156 > framebuffer::get_width().saturating_sub(150) {
                    break;
                }
                let rect = Rect {
                    x: taskbar_x,
                    y: taskbar_y + 8,
                    width: 148,
                    height: 28,
                };
                if rect.contains(x, y) {
                    return framebuffer::CursorType::Hand;
                }
                taskbar_x += 156;
            }
            return framebuffer::CursorType::Arrow;
        }

        for window in self.windows.iter().rev() {
            if window.minimized || !window.bounds.contains(x, y) {
                continue;
            }
            if (0..3).any(|button| window_button_rect(window.bounds, button).contains(x, y)) {
                return framebuffer::CursorType::Hand;
            }
            if y < window.bounds.y + 42 {
                return framebuffer::CursorType::Move;
            }
            return match window.kind {
                AppKind::TextEditor => {
                    let area = app_area(window.bounds);
                    let clear_button = Rect {
                        x: area.x + area.width.saturating_sub(104),
                        y: area.y + area.height.saturating_sub(40),
                        width: 82,
                        height: 28,
                    };
                    let editor = Rect {
                        x: area.x + 22,
                        y: area.y + 18,
                        width: area.width.saturating_sub(44),
                        height: area.height.saturating_sub(76),
                    };
                    if clear_button.contains(x, y) {
                        framebuffer::CursorType::Hand
                    } else if editor.contains(x, y) {
                        framebuffer::CursorType::Text
                    } else {
                        framebuffer::CursorType::Arrow
                    }
                }
                AppKind::Terminal => framebuffer::CursorType::Text,
                AppKind::Paint => framebuffer::CursorType::Crosshair,
                _ => framebuffer::CursorType::Hand,
            };
        }
        framebuffer::CursorType::Arrow
    }

    fn capture_press(&mut self, pointer: PointerEvent) -> Capture {
        if self.start_menu_open {
            let menu = start_menu_rect(framebuffer::get_height());
            if start_button_rect(framebuffer::get_height()).contains(pointer.x, pointer.y) {
                return Capture::StartMenu;
            }
            for index in 0..APP_COUNT {
                if start_menu_item_rect(menu, index).contains(pointer.x, pointer.y) {
                    return Capture::StartApp(index);
                }
            }
            return Capture::DismissStart;
        }
        let taskbar_y = framebuffer::get_height().saturating_sub(44);
        if pointer.y >= taskbar_y {
            if start_button_rect(framebuffer::get_height()).contains(pointer.x, pointer.y) {
                return Capture::StartMenu;
            }
            let mut taskbar_x = 132;
            for index in 0..self.windows.len() {
                if taskbar_x + 156 > framebuffer::get_width().saturating_sub(150) {
                    break;
                }
                let rect = Rect {
                    x: taskbar_x,
                    y: taskbar_y + 8,
                    width: 148,
                    height: 28,
                };
                if rect.contains(pointer.x, pointer.y) {
                    return Capture::TaskbarWindow(index);
                }
                taskbar_x += 156;
            }
            return Capture::None;
        }
        for index in (0..self.windows.len()).rev() {
            if self.windows[index].minimized {
                continue;
            }
            let bounds = self.windows[index].bounds;
            if !bounds.contains(pointer.x, pointer.y) {
                continue;
            }
            self.focus_window(index);
            let focused = self.windows.len() - 1;
            if window_button_rect(bounds, 0).contains(pointer.x, pointer.y) {
                return Capture::Close(focused);
            }
            if window_button_rect(bounds, 1).contains(pointer.x, pointer.y) {
                return Capture::Minimize(focused);
            }
            if window_button_rect(bounds, 2).contains(pointer.x, pointer.y) {
                return Capture::Maximize(focused);
            }
            if pointer.y < bounds.y + 42 {
                self.drag_offset = (pointer.x - bounds.x, pointer.y - bounds.y);
                return Capture::Drag(focused);
            }
            return Capture::None;
        }
        Capture::None
    }

    fn capture_release(&mut self, pointer: PointerEvent) -> bool {
        match self.capture {
            Capture::StartMenu => {
                if start_button_rect(framebuffer::get_height()).contains(pointer.x, pointer.y) {
                    self.start_menu_open = !self.start_menu_open;
                    return true;
                }
            }
            Capture::StartApp(index) => {
                let menu = start_menu_rect(framebuffer::get_height());
                if start_menu_item_rect(menu, index).contains(pointer.x, pointer.y) {
                    self.open_app(index);
                    self.start_menu_open = false;
                    return true;
                }
            }
            Capture::DismissStart => {
                self.start_menu_open = false;
                return true;
            }
            Capture::TaskbarWindow(index) => {
                let taskbar_y = framebuffer::get_height().saturating_sub(44);
                let rect = Rect {
                    x: 132 + index as u32 * 156,
                    y: taskbar_y + 8,
                    width: 148,
                    height: 28,
                };
                if rect.contains(pointer.x, pointer.y) {
                    if let Some(window) = self.windows.get_mut(index) {
                        if window.minimized {
                            window.minimized = false;
                        }
                        self.focus_window(index);
                        return true;
                    }
                }
            }
            Capture::Close(index) => {
                if index < self.windows.len()
                    && window_button_rect(self.windows[index].bounds, 0)
                        .contains(pointer.x, pointer.y)
                {
                    crate::services::user_process::terminate(self.windows[index].pid);
                    self.windows.remove(index);
                    self.focused = self.windows.iter().rposition(|window| !window.minimized);
                    self.request_focused_redraw();
                    return true;
                }
            }
            Capture::Minimize(index) => {
                if let Some(window) = self.windows.get_mut(index) {
                    if window_button_rect(window.bounds, 1).contains(pointer.x, pointer.y) {
                        window.minimized = true;
                        if self.focused == Some(index) {
                            self.focused =
                                self.windows.iter().rposition(|window| !window.minimized);
                            self.request_focused_redraw();
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
                            x: 8,
                            y: 44,
                            width: framebuffer::get_width().saturating_sub(16),
                            height: framebuffer::get_height().saturating_sub(92),
                        };
                        window.maximized = true;
                    }
                    self.request_focused_redraw();
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn focus_window(&mut self, index: usize) {
        if index >= self.windows.len() {
            return;
        }
        let window = self.windows.remove(index);
        self.windows.push(window);
        self.focused = Some(self.windows.len() - 1);
        self.request_focused_redraw();
    }

    fn request_focused_redraw(&self) {
        let Some(index) = self.focused else { return };
        let Some(window) = self.windows.get(index) else {
            return;
        };
        let area = app_area(window.bounds);
        let _ = crate::services::user_process::request_redraw(
            window.pid,
            crate::services::process::AppRect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height,
            },
            self.cursor_x,
            self.cursor_y,
            self.left_button_down,
        );
    }

    fn prune_exited_windows(&mut self) {
        let focused_pid = self
            .focused
            .and_then(|index| self.windows.get(index))
            .map(|window| window.pid);
        let old_len = self.windows.len();
        self.windows
            .retain(|window| crate::services::process::process_exists(window.pid));
        if self.windows.len() == old_len {
            return;
        }
        self.focused = focused_pid
            .and_then(|pid| self.windows.iter().position(|window| window.pid == pid))
            .or_else(|| self.windows.iter().rposition(|window| !window.minimized));
        self.capture = Capture::None;
    }

    fn pointer(&self) -> PointerEvent {
        PointerEvent {
            x: self.cursor_x,
            y: self.cursor_y,
            left_down: self.left_button_down,
        }
    }
}

fn app_area(bounds: Rect) -> Rect {
    Rect {
        x: bounds.x,
        y: bounds.y + 42,
        width: bounds.width,
        height: bounds.height.saturating_sub(42),
    }
}

fn window_button_rect(bounds: Rect, button: usize) -> Rect {
    let x = bounds.x + bounds.width.saturating_sub(34 + button as u32 * 30);
    Rect {
        x,
        y: bounds.y + 10,
        width: 22,
        height: 22,
    }
}

fn draw_window_button(rect: Rect, label: &str, color: u32, pointer: PointerEvent) {
    let hover = rect.contains(pointer.x, pointer.y);
    framebuffer::fill_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        if hover { 0x1596B5 } else { color },
    );
    framebuffer::draw_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        if hover { ACCENT } else { 0x516786 },
    );
    framebuffer::draw_text(
        rect.x + 5,
        rect.y + rect.height.saturating_sub(8) / 2,
        label,
        TEXT,
        1,
    );
}

fn start_button_rect(height: u32) -> Rect {
    Rect {
        x: 10,
        y: height.saturating_sub(38),
        width: 104,
        height: 30,
    }
}

fn start_menu_rect(height: u32) -> Rect {
    Rect {
        x: 10,
        y: height.saturating_sub(44 + 292),
        width: 250,
        height: 292,
    }
}

fn start_menu_item_rect(menu: Rect, index: usize) -> Rect {
    Rect {
        x: menu.x + 10,
        y: menu.y + 52 + index as u32 * 29,
        width: menu.width - 20,
        height: 26,
    }
}

fn blend_rgb(from: u32, to: u32, amount: u32) -> u32 {
    let mix = |shift: u32| {
        let first = (from >> shift) & 0xFF;
        let second = (to >> shift) & 0xFF;
        (first * (255 - amount) + second * amount) / 255
    };
    mix(16) << 16 | mix(8) << 8 | mix(0)
}

struct ConsoleState {
    initialized: bool,
    x: u32,
    y: u32,
}

static CONSOLE: SpinLock<ConsoleState> = SpinLock::new(ConsoleState {
    initialized: false,
    x: 0,
    y: 0,
});

pub fn console_write(bytes: &[u8]) {
    let width = framebuffer::get_width();
    let height = framebuffer::get_height();
    if width == 0 || height < 180 {
        return;
    }
    let mut console = CONSOLE.lock();
    let left = 24;
    let top = height - 178;
    if !console.initialized {
        framebuffer::fill_rect(left, top, (width / 2).min(620), 96, PANEL);
        framebuffer::draw_rect(left, top, (width / 2).min(620), 96, 0x516786);
        framebuffer::draw_text(left + 12, top + 10, "SYSTEM CONSOLE", ACCENT, 1);
        console.x = left + 12;
        console.y = top + 30;
        console.initialized = true;
    }
    let left = left + 12;
    let right = left + (width / 2).min(620) - 24;
    for &byte in bytes {
        if byte == b'\r' {
            continue;
        }
        if byte == b'\n' || console.x + 6 >= right {
            console.x = left;
            console.y += 12;
            if console.y + 8 >= top + 96 {
                framebuffer::fill_rect(left, top + 30, (width / 2).min(620) - 24, 58, PANEL);
                console.y = top + 30;
            }
            if byte == b'\n' {
                continue;
            }
        }
        if byte.is_ascii() {
            let glyph = [byte];
            if let Ok(character) = core::str::from_utf8(&glyph) {
                framebuffer::draw_text(console.x, console.y, character, TEXT, 1);
            }
        }
        console.x += 6;
    }
}
