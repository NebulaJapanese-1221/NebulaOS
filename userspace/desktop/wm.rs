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

// Userspace Window Manager

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use crate::desktop::syscalls::{get_ticks, yield_now, sleep};
use crate::desktop::framebuffer::{fill_rect, draw_rect, draw_text, present, get_width, get_height, set_cursor, mark_dirty, CursorType};
use crate::desktop::input::{poll_key, poll, MouseState};
use crate::desktop::app_launcher::{spawn_app, request_redraw, terminate, UserProcessError, process_exists, AppRect};

#[derive(Clone, Copy, PartialEq, Eq)]
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
const MAX_DESKTOPS: usize = 4;
const VERSION: &str = "1.0.0";

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppKind {
    Calculator,
    Clock,
    TextEditor,
    FileManager,
    Settings,
    Paint,
    SystemMonitor,
    Terminal,
}

impl AppKind {
    fn from_menu_index(index: usize) -> Option<Self> {
        Some(match index {
            0 => Self::Calculator,
            1 => Self::Clock,
            2 => Self::TextEditor,
            3 => Self::FileManager,
            4 => Self::Settings,
            5 => Self::Paint,
            6 => Self::SystemMonitor,
            7 => Self::Terminal,
            _ => return None,
        })
    }

    fn title(self) -> &'static str {
        match self {
            Self::Calculator => "CALCULATOR",
            Self::Clock => "CLOCK",
            Self::TextEditor => "TEXT EDITOR",
            Self::FileManager => "FILE MANAGER",
            Self::Settings => "SETTINGS",
            Self::Paint => "PAINT",
            Self::SystemMonitor => "SYSTEM MONITOR",
            Self::Terminal => "TERMINAL",
        }
    }
}

#[derive(Clone, Copy)]
enum Capture {
    None,
    Drag(usize),
    Resize(usize),
    Close(usize),
    Minimize(usize),
    Maximize(usize),
    StartMenu,
    StartApp(usize),
    TaskbarWindow(usize),
    DismissStart,
    SnapPreview(usize, SnapPosition),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SnapPosition {
    None,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

struct Notification {
    id: u32,
    title: String,
    message: String,
    timestamp: u64,
    expires_at: u64,
}

struct Window {
    pid: u32,
    kind: AppKind,
    bounds: Rect,
    restore_bounds: Rect,
    minimized: bool,
    maximized: bool,
    desktop: usize,
}

pub struct WindowManager {
    cursor_x: u32,
    cursor_y: u32,
    cursor_type: CursorType,
    left_button_down: bool,
    alt_pressed: bool,
    super_pressed: bool,
    windows: Vec<Window>,
    focused: Option<usize>,
    capture: Capture,
    drag_offset: (u32, u32),
    start_menu_open: bool,
    current_desktop: usize,
    snap_preview: Option<(usize, SnapPosition)>,
    notifications: Vec<Notification>,
    next_notification_id: u32,
    show_notification_center: bool,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            cursor_x: get_width() / 2,
            cursor_y: get_height() / 2,
            cursor_type: CursorType::Arrow,
            left_button_down: false,
            alt_pressed: false,
            super_pressed: false,
            windows: Vec::new(),
            focused: None,
            capture: Capture::None,
            drag_offset: (0, 0),
            start_menu_open: false,
            current_desktop: 0,
            snap_preview: None,
            notifications: Vec::new(),
            next_notification_id: 1,
            show_notification_center: false,
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.handle_events();
            self.render();
            yield_now().ok();
            sleep(1).ok();
        }
    }

    fn handle_events(&mut self) {
        // Handle keyboard
        if let Some(key) = poll_key() {
            self.handle_key(key);
        }

        // Handle mouse
        if let Some(mouse) = poll() {
            self.handle_mouse(mouse);
        }
    }

    fn handle_key(&mut self, key: u8) {
        // Track Alt key state
        if key == 0x38 {
            self.alt_pressed = true;
            return;
        }
        if key == 0xB8 {
            self.alt_pressed = false;
            return;
        }
        
        // Track Super (Win) key state
        if key == 0x5B || key == 0x5C {
            self.super_pressed = true;
            return;
        }
        if key == 0xDB || key == 0xDC {
            self.super_pressed = false;
            return;
        }

        // Alt+Tab: cycle focus between non-minimized windows on current desktop
        if self.alt_pressed && key == 0x0F {
            let non_minimized: Vec<usize> = self
                .windows
                .iter()
                .enumerate()
                .filter(|(_, w)| !w.minimized && w.desktop == self.current_desktop)
                .map(|(i, _)| i)
                .collect();
            if non_minimized.len() > 1 {
                let current = self.focused.unwrap_or(0);
                let pos = non_minimized.iter().position(|&i| i == current);
                let next = match pos {
                    Some(p) => non_minimized[(p + 1) % non_minimized.len()],
                    None => non_minimized[0],
                };
                self.focus_window(next);
            }
            return;
        }

        // Alt+M: minimize focused window
        if self.alt_pressed && key == 0x32 {
            if let Some(index) = self.focused {
                if index < self.windows.len() && !self.windows[index].minimized {
                    self.windows[index].minimized = true;
                    self.focused = self.windows.iter().rposition(|w| !w.minimized && w.desktop == self.current_desktop);
                }
            }
            return;
        }

        // Alt+R: restore minimized window (restore most recently minimized)
        if self.alt_pressed && key == 0x13 {
            if let Some(index) = self.windows.iter().rposition(|w| w.minimized) {
                self.windows[index].minimized = false;
                self.focus_window(index);
            }
            return;
        }
        
        // Super+Arrow: Window snapping
        if self.super_pressed {
            match key {
                0x4B => { self.snap_focused_window(SnapPosition::Left); return; }  // Left
                0x4D => { self.snap_focused_window(SnapPosition::Right); return; } // Right
                0x48 => { self.snap_focused_window(SnapPosition::Top); return; }   // Up
                0x50 => { self.snap_focused_window(SnapPosition::Bottom); return; }// Down
                0x31..=0x34 => { // Super+1/2/3/4 for desktop switching
                    let desk = (key - 0x31) as usize;
                    if desk < MAX_DESKTOPS {
                        self.switch_desktop(desk);
                    }
                    return;
                }
                0x35 => { // Super+N for notification center
                    self.show_notification_center = !self.show_notification_center;
                    return;
                }
                _ => {}
            }
        }
        
        // Ctrl+Alt+Left/Right for desktop switching
        if self.alt_pressed && key == 0x4B { // Left
            if self.current_desktop > 0 {
                self.switch_desktop(self.current_desktop - 1);
            }
            return;
        }
        if self.alt_pressed && key == 0x4D { // Right
            if self.current_desktop < MAX_DESKTOPS - 1 {
                self.switch_desktop(self.current_desktop + 1);
            }
            return;
        }

        if key == 27 && self.start_menu_open {
            self.start_menu_open = false;
            return;
        }
        if key == b'\t' && !self.alt_pressed && self.focused.is_none() {
            self.start_menu_open = !self.start_menu_open;
            return;
        }
        if key == 27 {
            if let Some(index) = self.focused {
                if index < self.windows.len() {
                    let _ = terminate(self.windows[index].pid);
                    self.windows.remove(index);
                    self.focused = self.windows.iter().rposition(|window| !window.minimized && window.desktop == self.current_desktop);
                    self.capture = Capture::None;
                    return;
                }
            }
            return;
        }
        if let Some(index) = self.focused {
            // Route key to focused app
            return;
        }
        if let Some(app_index) = match key {
            b'1'..=b'8' => Some((key - b'1') as usize),
            _ => None,
        } {
            self.open_app(app_index);
            return;
        }
    }

    fn switch_desktop(&mut self, desktop: usize) {
        if desktop >= MAX_DESKTOPS || desktop == self.current_desktop {
            return;
        }
        self.current_desktop = desktop;
        self.focused = self.windows.iter().rposition(|w| !w.minimized && w.desktop == desktop);
        self.start_menu_open = false;
        self.show_notification_center = false;
    }
    
    fn snap_focused_window(&mut self, position: SnapPosition) {
        let Some(index) = self.focused else { return };
        if index >= self.windows.len() { return; }
        if self.windows[index].minimized { return; }
        
        let width = get_width();
        let height = get_height();
        let (x, y, w, h) = match position {
            SnapPosition::Left => (8, 44, width.saturating_sub(16) / 2, height.saturating_sub(92)),
            SnapPosition::Right => (width.saturating_sub(16) / 2 + 8, 44, width.saturating_sub(16) / 2, height.saturating_sub(92)),
            SnapPosition::Top => (8, 44, width.saturating_sub(16), height.saturating_sub(92) / 2),
            SnapPosition::Bottom => (8, 44 + height.saturating_sub(92) / 2, width.saturating_sub(16), height.saturating_sub(92) / 2),
            _ => return,
        };
        
        if !self.windows[index].maximized {
            self.windows[index].restore_bounds = self.windows[index].bounds;
        }
        self.windows[index].bounds = Rect { x, y, width: w, height: h };
        self.windows[index].maximized = false;
    }

    fn open_app(&mut self, index: usize) {
        let Some(kind) = AppKind::from_menu_index(index) else {
            return;
        };
        let width = get_width();
        let height = get_height();
        let offset = (self.windows.len() as u32 * 28) % 140;
        let bounds = Rect {
            x: (width.saturating_sub(460) / 2 + offset).min(width.saturating_sub(440)),
            y: (height.saturating_sub(340) / 2 + offset).min(height.saturating_sub(320)),
            width: 420.min(width.saturating_sub(24)),
            height: 300.min(height.saturating_sub(80)),
        };
        let app_id = index as u32;
        let area = app_area(bounds);
        let content = AppRect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        };
        match spawn_app(app_id, content) {
            Ok(pid) => {
                self.windows.push(Window {
                    pid,
                    kind,
                    bounds,
                    restore_bounds: bounds,
                    minimized: false,
                    maximized: false,
                    desktop: self.current_desktop,
                });
                self.focused = Some(self.windows.len() - 1);
            }
            Err(_) => {
                // Handle error
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseState) {
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
        
        // Check for drag-to-edge snap preview
        if was_down && mouse.left {
            if let Capture::Drag(index) = self.capture {
                if index < self.windows.len() {
                    let snap_pos = self.get_snap_position(x, y);
                    self.snap_preview = if snap_pos != SnapPosition::None {
                        Some((index, snap_pos))
                    } else {
                        None
                    };
                }
            }
        } else {
            self.snap_preview = None;
        }

        if !was_down && mouse.left {
            self.capture = self.capture_press(pointer);
        } else if was_down && mouse.left {
            if let Capture::Drag(index) = self.capture {
                if let Some(window) = self.windows.get_mut(index) {
                    let old_bounds = window.bounds;
                    let fb_w = get_width();
                    let fb_h = get_height();
                    window.bounds.x = x
                        .saturating_sub(self.drag_offset.0)
                        .min(fb_w.saturating_sub(window.bounds.width));
                    window.bounds.y = y
                        .saturating_sub(self.drag_offset.1)
                        .max(40)
                        .min(fb_h.saturating_sub(window.bounds.height + 44));
                    if old_bounds != window.bounds {
                        // Redraw dirty regions
                        mark_dirty(old_bounds.x, old_bounds.y, old_bounds.width, old_bounds.height);
                        mark_dirty(window.bounds.x, window.bounds.y, window.bounds.width, window.bounds.height);
                    }
                }
            }
        }
        if let Capture::Resize(index) = self.capture {
            if let Some(window) = self.windows.get_mut(index) {
                let old_bounds = window.bounds;
                let fb_w = get_width();
                let fb_h = get_height();
                let new_width = x
                    .saturating_sub(window.bounds.x)
                    .max(200)
                    .min(fb_w.saturating_sub(window.bounds.x));
                let new_height = y
                    .saturating_sub(window.bounds.y)
                    .max(150)
                    .min(fb_h.saturating_sub(window.bounds.y));
                let new_bounds = Rect {
                    x: window.bounds.x,
                    y: window.bounds.y,
                    width: new_width,
                    height: new_height,
                };
                window.bounds = new_bounds;
                if old_bounds != new_bounds {
                    if old_bounds.width > new_bounds.width {
                        mark_dirty(new_bounds.x + new_bounds.width, old_bounds.y, old_bounds.width - new_bounds.width, old_bounds.height);
                    }
                    if old_bounds.height > new_bounds.height {
                        mark_dirty(old_bounds.x, new_bounds.y + new_bounds.height, old_bounds.width, old_bounds.height - new_bounds.height);
                    }
                    mark_dirty(old_bounds.x, old_bounds.y, old_bounds.width.max(new_bounds.width), old_bounds.height.max(new_bounds.height));
                }
            }
        }

        let mut redraw = button_changed || previous_hover != self.hover_target_at(x, y);
        if was_down && !mouse.left {
            // Apply snap on release
            if let Capture::Drag(index) = self.capture {
                if let Some(pos) = self.snap_preview {
                    if pos.0 == index {
                        self.apply_snap(index, pos.1);
                    }
                }
            }
            redraw |= self.capture_release(pointer);
            self.capture = Capture::None;
            self.snap_preview = None;
        }
        
        let pointer_over_app = self.focused.is_some_and(|index| {
            self.windows.get(index).is_some_and(|window| {
                !window.minimized
                    && window.desktop == self.current_desktop
                    && (app_area(window.bounds).contains(x, y)
                        || app_area(window.bounds).contains(previous_x, previous_y))
            })
        });
        if !self.start_menu_open && !self.show_notification_center && (button_changed || (moved && (mouse.left || pointer_over_app)))
        {
            if let Some(index) = self.focused {
                if index < self.windows.len() && !self.windows[index].minimized && self.windows[index].desktop == self.current_desktop {
                    // Route pointer event to app
                }
            }
        }
        if matches!(self.capture, Capture::Drag(_)) && moved {
            // Request redraw
        }
        let next_cursor_type = self.cursor_type_at(x, y);
        let cursor_changed = moved || self.cursor_type != next_cursor_type;
        self.cursor_type = next_cursor_type;
        if cursor_changed {
            let _ = set_cursor(x, y, next_cursor_type);
        }
    }
    
    fn get_snap_position(&self, x: u32, y: u32) -> SnapPosition {
        let width = get_width();
        let height = get_height();
        let edge_threshold = 20u32;
        
        let at_left = x <= edge_threshold;
        let at_right = x >= width.saturating_sub(edge_threshold);
        let at_top = y <= 40 + edge_threshold;
        let at_bottom = y >= height.saturating_sub(44 + edge_threshold);
        
        if at_left && at_top { return SnapPosition::TopLeft; }
        if at_right && at_top { return SnapPosition::TopRight; }
        if at_left && at_bottom { return SnapPosition::BottomLeft; }
        if at_right && at_bottom { return SnapPosition::BottomRight; }
        if at_left { return SnapPosition::Left; }
        if at_right { return SnapPosition::Right; }
        if at_top { return SnapPosition::Top; }
        if at_bottom { return SnapPosition::Bottom; }
        SnapPosition::None
    }
    
    fn apply_snap(&mut self, index: usize, position: SnapPosition) {
        if index >= self.windows.len() { return; }
        let width = get_width();
        let height = get_height();
        let (x, y, w, h) = match position {
            SnapPosition::Left => (8, 44, width.saturating_sub(16) / 2, height.saturating_sub(92)),
            SnapPosition::Right => (width.saturating_sub(16) / 2 + 8, 44, width.saturating_sub(16) / 2, height.saturating_sub(92)),
            SnapPosition::Top => (8, 44, width.saturating_sub(16), height.saturating_sub(92) / 2),
            SnapPosition::Bottom => (8, 44 + height.saturating_sub(92) / 2, width.saturating_sub(16), height.saturating_sub(92) / 2),
            SnapPosition::TopLeft => (8, 44, width.saturating_sub(16) / 2, height.saturating_sub(92) / 2),
            SnapPosition::TopRight => (width.saturating_sub(16) / 2 + 8, 44, width.saturating_sub(16) / 2, height.saturating_sub(92) / 2),
            SnapPosition::BottomLeft => (8, 44 + height.saturating_sub(92) / 2, width.saturating_sub(16) / 2, height.saturating_sub(92) / 2),
            SnapPosition::BottomRight => (width.saturating_sub(16) / 2 + 8, 44 + height.saturating_sub(92) / 2, width.saturating_sub(16) / 2, height.saturating_sub(92) / 2),
            _ => return,
        };
        
        if !self.windows[index].maximized {
            self.windows[index].restore_bounds = self.windows[index].bounds;
        }
        self.windows[index].bounds = Rect { x, y, width: w, height: h };
        self.windows[index].maximized = false;
    }

    fn render(&mut self) {
        self.prune_exited_windows();
        let width = get_width();
        let height = get_height();
        if width == 0 || height == 0 {
            return;
        }
        let (wallpaper_top, wallpaper_bottom) = (WALLPAPER_TOP, WALLPAPER_BOTTOM);
        fill_rect(0, 0, width, height, wallpaper_top).ok();
        for y in 0..height {
            fill_rect(0, y, width, 1, blend_rgb(wallpaper_top, wallpaper_bottom, y * 255 / height)).ok();
        }
        
        // Draw windows for current desktop
        let pointer = self.pointer();
        for index in 0..self.windows.len() {
            if !self.windows[index].minimized && self.windows[index].desktop == self.current_desktop {
                self.draw_window(index, pointer);
            }
        }
        
        // Draw snap preview
        if let Some((index, position)) = self.snap_preview {
            if index < self.windows.len() {
                self.draw_snap_preview(position);
            }
        }
        
        self.draw_taskbar(pointer);
        if self.start_menu_open {
            self.draw_start_menu(pointer);
        }
        if self.show_notification_center {
            self.draw_notification_center(pointer);
        }
        self.draw_notifications(pointer);
        
        let _ = set_cursor(self.cursor_x, self.cursor_y, self.cursor_type);
        present().ok();
    }
    
    fn draw_taskbar(&self, pointer: PointerEvent) {
        let width = get_width();
        let height = get_height();
        let taskbar_y = height.saturating_sub(44);
        
        fill_rect(0, taskbar_y, width, 44, PANEL).ok();
        
        // Start button
        let start_rect = start_button_rect(height);
        draw_window_button(start_rect, "START", 0x285A72, pointer).ok();
        
        // Desktop switcher
        let desktop_start_x = 120;
        for d in 0..MAX_DESKTOPS {
            let desk_rect = Rect {
                x: desktop_start_x + d as u32 * 36,
                y: taskbar_y + 6,
                width: 30,
                height: 30,
            };
            let is_current = d == self.current_desktop;
            let has_windows = self.windows.iter().any(|w| w.desktop == d && !w.minimized);
            let bg_color = if is_current { 0x285A72 } else if has_windows { 0x26374F } else { 0x1E2D3D };
            fill_rect(desk_rect.x, desk_rect.y, desk_rect.width, desk_rect.height, bg_color).ok();
            draw_rect(desk_rect.x, desk_rect.y, desk_rect.width, desk_rect.height, 
                if is_current { ACCENT } else { 0x516786 }).ok();
            let label = (b'1' + d as u8) as char;
            draw_text(desk_rect.x + 10, desk_rect.y + 9, &label.to_string(), TEXT, 1).ok();
        }
        
        // Window buttons
        let mut x = 280;
        for (index, window) in self.windows.iter().enumerate() {
            if window.desktop != self.current_desktop { continue; }
            if x + 156 > width.saturating_sub(200) {
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
            ).ok();
            x += 156;
        }
        
        // System tray (right side)
        let tray_x = width.saturating_sub(180);
        self.draw_system_tray(tray_x, taskbar_y, pointer);
        
        // Version
        draw_text(
            width.saturating_sub(132),
            taskbar_y + 18,
            VERSION,
            ACCENT,
            1,
        ).ok();
    }
    
    fn draw_system_tray(&self, x: u32, taskbar_y: u32, pointer: PointerEvent) {
        let mut icon_x = x;
        
        let net_rect = Rect { x: icon_x, y: taskbar_y + 7, width: 30, height: 30 };
        let net_hover = net_rect.contains(pointer.x, pointer.y);
        fill_rect(net_rect.x, net_rect.y, net_rect.width, net_rect.height, 
            if net_hover { 0x1596B5 } else { 0x26374F }).ok();
        draw_text(net_rect.x + 6, net_rect.y + 9, "NET", MUTED, 1).ok();
        icon_x += 36;
        
        let vol_rect = Rect { x: icon_x, y: taskbar_y + 7, width: 30, height: 30 };
        let vol_hover = vol_rect.contains(pointer.x, pointer.y);
        fill_rect(vol_rect.x, vol_rect.y, vol_rect.width, vol_rect.height, 
            if vol_hover { 0x1596B5 } else { 0x26374F }).ok();
        draw_text(vol_rect.x + 6, vol_rect.y + 9, "VOL", MUTED, 1).ok();
        icon_x += 36;
        
        let bat_rect = Rect { x: icon_x, y: taskbar_y + 7, width: 30, height: 30 };
        let bat_hover = bat_rect.contains(pointer.x, pointer.y);
        fill_rect(bat_rect.x, bat_rect.y, bat_rect.width, bat_rect.height, 
            if bat_hover { 0x1596B5 } else { 0x26374F }).ok();
        draw_text(bat_rect.x + 6, bat_rect.y + 9, "BAT", MUTED, 1).ok();
        icon_x += 36;
        
        let time_rect = Rect { x: icon_x, y: taskbar_y + 7, width: 70, height: 30 };
        fill_rect(time_rect.x, time_rect.y, time_rect.width, time_rect.height, 0x26374F).ok();
        draw_text(time_rect.x + 5, time_rect.y + 9, "12:34", TEXT, 1).ok();
    }
    
    fn draw_start_menu(&self, pointer: PointerEvent) {
        let rect = start_menu_rect(get_height());
        fill_rect(rect.x + 5, rect.y + 5, rect.width, rect.height, 0x080D16).ok();
        fill_rect(rect.x, rect.y, rect.width, rect.height, PANEL).ok();
        draw_rect(rect.x, rect.y, rect.width, rect.height, 0x516786).ok();
        draw_text(rect.x + 16, rect.y + 14, "NEBULAOS", TEXT, 1).ok();
        draw_text(rect.x + 16, rect.y + 30, "VERSION", MUTED, 1).ok();
        draw_text(rect.x + 76, rect.y + 30, VERSION, ACCENT, 1).ok();
        for index in 0..APP_COUNT {
            let row = start_menu_item_rect(rect, index);
            if let Some(kind) = AppKind::from_menu_index(index) {
                draw_window_button(row, kind.title(), 0x26374F, pointer).ok();
            }
        }
    }
    
    fn draw_snap_preview(&self, position: SnapPosition) {
        let width = get_width();
        let height = get_height();
        let (x, y, w, h) = match position {
            SnapPosition::Left => (0, 40, width / 2, height - 84),
            SnapPosition::Right => (width / 2, 40, width / 2, height - 84),
            SnapPosition::Top => (0, 40, width, (height - 84) / 2),
            SnapPosition::Bottom => (0, 40 + (height - 84) / 2, width, (height - 84) / 2),
            SnapPosition::TopLeft => (0, 40, width / 2, (height - 84) / 2),
            SnapPosition::TopRight => (width / 2, 40, width / 2, (height - 84) / 2),
            SnapPosition::BottomLeft => (0, 40 + (height - 84) / 2, width / 2, (height - 84) / 2),
            SnapPosition::BottomRight => (width / 2, 40 + (height - 84) / 2, width / 2, (height - 84) / 2),
            _ => return,
        };
        draw_rect(x, y, w, h, ACCENT).ok();
        draw_rect(x + 2, y + 2, w.saturating_sub(4), h.saturating_sub(4), ACCENT).ok();
    }
    
    fn draw_notifications(&self, pointer: PointerEvent) {
        let width = get_width();
        let height = get_height();
        let now = get_ticks() as u64;
        
        let mut y = 50;
        for notification in &self.notifications {
            if notification.expires_at < now { continue; }
            let text_width = (notification.title.len() + notification.message.len()) as u32 * 6 + 40;
            let notif_width = text_width.min(width - 40).max(200);
            let notif_x = width - notif_width - 20;
            let notif_rect = Rect { x: notif_x, y, width: notif_width, height: 60 };
            
            if notification.expires_at < now { continue; }
            
            fill_rect(notif_rect.x + 3, notif_rect.y + 3, notif_rect.width, notif_rect.height, 0x080D16).ok();
            fill_rect(notif_rect.x, notif_rect.y, notif_rect.width, notif_rect.height, PANEL).ok();
            draw_rect(notif_rect.x, notif_rect.y, notif_rect.width, notif_rect.height, ACCENT).ok();
            draw_text(notif_rect.x + 10, notif_rect.y + 8, &notification.title, TEXT, 1).ok();
            draw_text(notif_rect.x + 10, notif_rect.y + 28, &notification.message, MUTED, 1).ok();
            
            let close_rect = Rect { x: notif_rect.x + notif_width - 24, y: notif_rect.y + 4, width: 20, height: 20 };
            let close_hover = close_rect.contains(pointer.x, pointer.y);
            fill_rect(close_rect.x, close_rect.y, close_rect.width, close_rect.height, 
                if close_hover { 0xE45B67 } else { 0x26374F }).ok();
            draw_text(close_rect.x + 5, close_rect.y + 4, "x", TEXT, 1).ok();
            
            y += 70;
        }
    }
    
    fn draw_notification_center(&self, pointer: PointerEvent) {
        let width = get_width();
        let height = get_height();
        let panel_w = 350;
        let panel_x = width - panel_w - 10;
        let panel_y = 40;
        let panel_h = height - 84;
        
        fill_rect(panel_x + 5, panel_y + 5, panel_w, panel_h, 0x080D16).ok();
        fill_rect(panel_x, panel_y, panel_w, panel_h, PANEL).ok();
        draw_rect(panel_x, panel_y, panel_w, panel_h, 0x516786).ok();
        draw_text(panel_x + 16, panel_y + 14, "NOTIFICATIONS", TEXT, 1).ok();
        
        let now = get_ticks() as u64;
        let mut y = panel_y + 50;
        let mut has_notifications = false;
        
        for notification in &self.notifications {
            if notification.expires_at < now { continue; }
            has_notifications = true;
            let notif_rect = Rect { x: panel_x + 10, y, width: panel_w - 20, height: 70 };
            
            fill_rect(notif_rect.x, notif_rect.y, notif_rect.width, notif_rect.height, 0x1E2D3D).ok();
            draw_rect(notif_rect.x, notif_rect.y, notif_rect.width, notif_rect.height, 0x516786).ok();
            draw_text(notif_rect.x + 10, notif_rect.y + 8, &notification.title, TEXT, 1).ok();
            draw_text(notif_rect.x + 10, notif_rect.y + 28, &notification.message, MUTED, 1).ok();
            
            let time_str = format!("{}s ago", (now - notification.timestamp) / 100);
            draw_text(notif_rect.x + 10, notif_rect.y + 48, &time_str, MUTED, 1).ok();
            
            y += 80;
        }
        
        if !has_notifications {
            draw_text(panel_x + 16, panel_y + 60, "No notifications", MUTED, 1).ok();
        }
        
        let close_rect = Rect { x: panel_x + panel_w - 40, y: panel_y + 8, width: 30, height: 30 };
        let close_hover = close_rect.contains(pointer.x, pointer.y);
        fill_rect(close_rect.x, close_rect.y, close_rect.width, close_rect.height, 
            if close_hover { 0xE45B67 } else { 0x26374F }).ok();
        draw_text(close_rect.x + 8, close_rect.y + 8, "x", TEXT, 1).ok();
    }
    
    fn draw_window(&self, index: usize, pointer: PointerEvent) {
        let window = &self.windows[index];
        let rect = window.bounds;
        let is_focused = self.focused == Some(index);
        fill_rect(rect.x + 6, rect.y + 8, rect.width, rect.height, 0x080D16).ok();
        fill_rect(rect.x, rect.y, rect.width, rect.height, 0xF2F5FA).ok();
        fill_rect(
            rect.x,
            rect.y,
            rect.width,
            42,
            if is_focused { PANEL } else { 0x263247 },
        ).ok();
        draw_text(rect.x + 14, rect.y + 17, window.kind.title(), TEXT, 1).ok();
        draw_window_button(window_button_rect(rect, 0), "X", 0xE45B67, pointer).ok();
        draw_window_button(window_button_rect(rect, 1), "_", 0x42546D, pointer).ok();
        draw_window_button(
            window_button_rect(rect, 2),
            if window.maximized { "o" } else { "[]" },
            0x42546D,
            pointer,
        ).ok();
        let area = app_area(rect);
        fill_rect(area.x, area.y, area.width, area.height, 0xF2F5FA).ok();
        if is_focused {
            draw_rect(rect.x, rect.y, rect.width, rect.height, ACCENT).ok();
        }
        let resize_rect = resize_handle_rect(rect);
        fill_rect(resize_rect.x, resize_rect.y, resize_rect.width, resize_rect.height, 0x8A9BA8).ok();
        draw_rect(resize_rect.x, resize_rect.y, resize_rect.width, resize_rect.height, 0x516786).ok();
        draw_text(
            resize_rect.x + 3,
            resize_rect.y + 3,
            "⟲",
            TEXT,
            1,
        ).ok();
    }

    fn draw_wallpaper_rect(&self, x: u32, y: u32, width: u32, height: u32) {
        let fb_width = get_width();
        let fb_height = get_height();
        if fb_width == 0 || fb_height == 0 {
            return;
        }
        let (wallpaper_top, wallpaper_bottom) = (WALLPAPER_TOP, WALLPAPER_BOTTOM);
        let top = y.min(fb_height);
        let bottom = y.saturating_add(height).min(fb_height);
        let left = x.min(fb_width);
        let right = x.saturating_add(width).min(fb_width);
        if right <= left || bottom <= top {
            return;
        }
        for row in top..bottom {
            let color = blend_rgb(wallpaper_top, wallpaper_bottom, row * 255 / fb_height.max(1));
            fill_rect(left, row, right.saturating_sub(left), 1, color).ok();
        }
    }
    
    fn hover_target_at(&self, x: u32, y: u32) -> Option<(u8, usize, usize)> {
        if start_button_rect(get_height()).contains(x, y) {
            return Some((0, 0, 0));
        }
        if self.start_menu_open {
            let menu = start_menu_rect(get_height());
            for index in 0..APP_COUNT {
                if start_menu_item_rect(menu, index).contains(x, y) {
                    return Some((1, index, 0));
                }
            }
            return None;
        }

        let taskbar_y = get_height().saturating_sub(44);
        if y >= taskbar_y {
            let mut taskbar_x = 132;
            for index in 0..self.windows.len() {
                if taskbar_x + 156 > get_width().saturating_sub(150) {
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
            if resize_handle_rect(window.bounds).contains(x, y) {
                return Some((3, index, 3));
            }
            break;
        }
        None
    }
    
    fn cursor_type_at(&self, x: u32, y: u32) -> CursorType {
        if start_button_rect(get_height()).contains(x, y) {
            return CursorType::Hand;
        }
        if self.start_menu_open {
            let menu = start_menu_rect(get_height());
            if (0..APP_COUNT).any(|index| start_menu_item_rect(menu, index).contains(x, y)) {
                return CursorType::Hand;
            }
            return CursorType::Arrow;
        }

        let taskbar_y = get_height().saturating_sub(44);
        if y >= taskbar_y {
            let mut taskbar_x = 132;
            for _ in &self.windows {
                if taskbar_x + 156 > get_width().saturating_sub(150) {
                    break;
                }
                let rect = Rect {
                    x: taskbar_x,
                    y: taskbar_y + 8,
                    width: 148,
                    height: 28,
                };
                if rect.contains(x, y) {
                    return CursorType::Hand;
                }
                taskbar_x += 156;
            }
            return CursorType::Arrow;
        }

        for window in self.windows.iter().rev() {
            if window.minimized || !window.bounds.contains(x, y) {
                continue;
            }
            if (0..3).any(|button| window_button_rect(window.bounds, button).contains(x, y)) {
                return CursorType::Hand;
            }
            if resize_handle_rect(window.bounds).contains(x, y) {
                return CursorType::Move;
            }
            if y < window.bounds.y + 42 {
                return CursorType::Move;
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
                        CursorType::Hand
                    } else if editor.contains(x, y) {
                        CursorType::Text
                    } else {
                        CursorType::Arrow
                    }
                }
                AppKind::Terminal => CursorType::Text,
                AppKind::Paint => CursorType::Crosshair,
                _ => CursorType::Hand,
            };
        }
        CursorType::Arrow
    }
    
    fn capture_press(&mut self, pointer: PointerEvent) -> Capture {
        if self.start_menu_open {
            let menu = start_menu_rect(get_height());
            if start_button_rect(get_height()).contains(pointer.x, pointer.y) {
                return Capture::StartMenu;
            }
            for index in 0..APP_COUNT {
                if start_menu_item_rect(menu, index).contains(pointer.x, pointer.y) {
                    return Capture::StartApp(index);
                }
            }
            return Capture::DismissStart;
        }
        let taskbar_y = get_height().saturating_sub(44);
        if pointer.y >= taskbar_y {
            if start_button_rect(get_height()).contains(pointer.x, pointer.y) {
                return Capture::StartMenu;
            }
            let mut taskbar_x = 132;
            for index in 0..self.windows.len() {
                if taskbar_x + 156 > get_width().saturating_sub(150) {
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
            let resize_rect = resize_handle_rect(bounds);
            if resize_rect.contains(pointer.x, pointer.y) {
                return Capture::Resize(focused);
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
                if start_button_rect(get_height()).contains(pointer.x, pointer.y) {
                    self.start_menu_open = !self.start_menu_open;
                    return true;
                }
            }
            Capture::StartApp(index) => {
                let menu = start_menu_rect(get_height());
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
                let taskbar_y = get_height().saturating_sub(44);
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
                    let _ = terminate(self.windows[index].pid);
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
                            self.focused =
                                self.windows.iter().rposition(|window| !window.minimized);
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
                            width: get_width().saturating_sub(16),
                            height: get_height().saturating_sub(92),
                        };
                        window.maximized = true;
                    }
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
    }

    fn prune_exited_windows(&mut self) {
        let focused_pid = self
            .focused
            .and_then(|index| self.windows.get(index))
            .map(|window| window.pid);
        let old_len = self.windows.len();
        self.windows.retain(|window| process_exists(window.pid));
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

fn resize_handle_rect(bounds: Rect) -> Rect {
    let handle_size = 16u32;
    Rect {
        x: bounds.x + bounds.width.saturating_sub(handle_size),
        y: bounds.y + bounds.height.saturating_sub(handle_size),
        width: handle_size,
        height: handle_size,
    }
}

fn draw_window_button(rect: Rect, label: &str, color: u32, pointer: PointerEvent) -> Result<(), u32> {
    let hover = rect.contains(pointer.x, pointer.y);
    fill_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        if hover { 0x1596B5 } else { color },
    )?;
    draw_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        if hover { ACCENT } else { 0x516786 },
    )?;
    draw_text(
        rect.x + 5,
        rect.y + rect.height.saturating_sub(8) / 2,
        label,
        TEXT,
        1,
    )?;
    Ok(())
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