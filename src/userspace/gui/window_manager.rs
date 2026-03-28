use crate::drivers::mouse;
use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use crate::drivers::rtc::{self, CURRENT_DATETIME, TIME_NEEDS_UPDATE};
use crate::drivers::keyboard;
use crate::userspace::gui::{Rect, Button, Shell, font, draw_rect, LARGE_TEXT, DESKTOP_GRADIENT_START, DESKTOP_GRADIENT_END, FULL_REDRAW_REQUESTED, MOUSE_SENSITIVITY};
use crate::userspace::apps::{app::{App, AppEvent}, calculator::Calculator, editor::TextEditor, paint::Paint, settings::Settings, terminal::Terminal, task_manager::TaskManager, partition_manager::PartitionManager};
use crate::userspace::localisation;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

const MAX_WINDOWS: usize = 10;
const CURSOR_WIDTH: usize = 12;
const CURSOR_HEIGHT: usize = 17;

#[derive(Clone)]
pub struct Window {
    pub id: usize,
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,    
    pub color: u32,
    pub title: &'static str,
    pub content: WindowContent,
    pub minimized: bool,
    pub maximized: bool,
    pub restore_rect: Option<Rect>,
}

impl Window {
    pub fn rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, width: self.width, height: self.height }
    }
}

#[derive(Clone)]
pub enum WindowContent {
    App(Box<dyn App>),
    None,
} 

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton { Left, Right, #[allow(dead_code)] Middle }

#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    MouseMove { x: isize, y: isize, dx: isize, dy: isize },
    MouseButton { button: MouseButton, down: bool, x: isize, y: isize },
    Scroll { delta: isize, x: isize, y: isize },
    KeyPress { key: char },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CursorStyle { Arrow, ResizeNS, ResizeEW, ResizeNESW, ResizeNWSE }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeDirection { None, Left, Right, Top, Bottom, TopLeft, TopRight, BottomLeft, BottomRight }

pub struct WindowManager {
    pub windows: Vec<Window>,
    pub z_order: Vec<usize>,
    pub next_win_id: usize,
    pub input: InputManager,
    pub drag_win_id: Option<usize>,
    pub drag_offset_x: isize,
    pub drag_offset_y: isize,
    pub click_target_id: Option<usize>,
    pub shell: Shell,
    pub dirty_rects: Vec<Rect>,
    pub context_menu_open: bool,
    pub context_menu_x: isize,
    pub context_menu_y: isize,
    pub resize_win_id: Option<usize>,
    pub resize_direction: ResizeDirection,
    cursor_style: CursorStyle,
    pub backbuffer: Vec<u32>,
    pub drag_rect: Option<Rect>,
    pub task_switcher_open: bool,
    pub task_switcher_index: usize,
    pub last_cursor_x: isize,
    pub last_cursor_y: isize,
    pub cursor_save_buffer: Vec<u32>,
}

impl WindowManager {
    pub const fn new() -> Self {
        WindowManager {
            windows: Vec::new(), z_order: Vec::new(), next_win_id: 0,
            input: InputManager::new(), drag_win_id: None, drag_offset_x: 0, drag_offset_y: 0, click_target_id: None,
            shell: Shell { start_menu: crate::userspace::gui::shell::StartMenu::new(), taskbar_height: 40 },
            dirty_rects: Vec::new(), context_menu_open: false, context_menu_x: 0, context_menu_y: 0,
            resize_win_id: None, resize_direction: ResizeDirection::None, cursor_style: CursorStyle::Arrow,
            backbuffer: Vec::new(), drag_rect: None, task_switcher_open: false, task_switcher_index: 0,
            last_cursor_x: 400, last_cursor_y: 300,
            cursor_save_buffer: Vec::new(),
        }
    }

    pub fn add_window(&mut self, mut window: Window) {
        if self.windows.len() < MAX_WINDOWS {
            window.id = self.next_win_id; self.next_win_id += 1;
            self.z_order.push(window.id);
            self.mark_dirty(Rect { x: window.x, y: window.y, width: window.width, height: window.height });
            self.windows.push(window);
        }
    }

    pub fn mark_dirty(&mut self, rect: Rect) { self.dirty_rects.push(rect); }
    fn get_window_rect(&self, win_id: usize) -> Option<Rect> {
        self.windows.iter().find(|w| w.id == win_id).map(|w| Rect { x: w.x, y: w.y, width: w.width, height: w.height })
    }

    pub fn update(&mut self) {
        if TIME_NEEDS_UPDATE.load(Ordering::Relaxed) {
            TIME_NEEDS_UPDATE.store(false, Ordering::Relaxed);
            let new_time = rtc::read_time();
            let mut current_dt = CURRENT_DATETIME.lock();
            if *current_dt != new_time {
                *current_dt = new_time;
                if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                    let time_area_width: isize = (8 * 8) + 20;
                    self.mark_dirty(Rect { x: info.width as isize - time_area_width, y: info.height as isize - self.shell.taskbar_height as isize, width: time_area_width as usize, height: self.shell.taskbar_height });
                }
            }
        }

        let (screen_width, screen_height) = if let Some(info) = FRAMEBUFFER.lock().info.as_ref() { (info.width as isize, info.height as isize) } else { (800, 600) };
        let is_vertical = self.shell.is_vertical(screen_width as usize);
        let start_interaction_id = self.resize_win_id;
        let start_interaction_rect = start_interaction_id.and_then(|id| self.get_window_rect(id));

        if let Some(rect) = self.drag_rect { self.mark_dirty(rect); }
        self.input.update(screen_width, screen_height);
        let mut mouse_moved = false;
        let events = core::mem::take(&mut self.input.event_queue);

        for event in events {
            match event {
                InputEvent::MouseMove { x, y, dx, dy } => {
                    mouse_moved = true; self.input.mouse_x = x; self.input.mouse_y = y;
                    if let Some(id) = self.resize_win_id {
                        if let Some(win) = self.windows.iter_mut().find(|w| w.id == id) {
                            let min_w: isize = 80; let min_h: isize = 40;
                            if matches!(self.resize_direction, ResizeDirection::Right | ResizeDirection::TopRight | ResizeDirection::BottomRight) { win.width = (win.width as isize + dx).max(min_w) as usize; }
                            if matches!(self.resize_direction, ResizeDirection::Bottom | ResizeDirection::BottomLeft | ResizeDirection::BottomRight) { win.height = (win.height as isize + dy).max(min_h) as usize; }
                            if matches!(self.resize_direction, ResizeDirection::Left | ResizeDirection::TopLeft | ResizeDirection::BottomLeft) { let nw = win.width as isize - dx; if nw >= min_w { win.x += dx; win.width = nw as usize; } }
                            if matches!(self.resize_direction, ResizeDirection::Top | ResizeDirection::TopLeft | ResizeDirection::TopRight) { let nh = win.height as isize - dy; if nh >= min_h { win.y += dy; win.height = nh as usize; } }
                        }
                    } else if let Some(id) = self.drag_win_id {
                        if let Some(win) = self.windows.iter().find(|w| w.id == id) {
                            self.drag_rect = Some(Rect { x: self.input.mouse_x - self.drag_offset_x, y: self.input.mouse_y - self.drag_offset_y, width: win.width, height: win.height });
                        }
                    }
                }
                InputEvent::Scroll { delta, x, y } => {
                    let mut scroll_target = None;
                    for &win_id in self.z_order.iter().rev() {
                        if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                            if !win.minimized && win.rect().contains(x, y) { scroll_target = Some(win_id); break; }
                        }
                    }
                    if let Some(target_id) = scroll_target {
                        let mut rect_to_mark = None;
                        if let Some(win) = self.windows.iter_mut().find(|w| w.id == target_id) {
                            if let WindowContent::App(app) = &mut win.content {
                                app.handle_event(&AppEvent::Scroll { delta: -delta * 16, width: win.width, height: win.height });
                                rect_to_mark = Some(win.rect());
                            }
                        }
                        if let Some(r) = rect_to_mark { self.mark_dirty(r); }
                    }
                }
                InputEvent::MouseButton { button, down, x, y } => { self.input.mouse_x = x; self.input.mouse_y = y; self.handle_mouse_button_event(button, down, screen_width, screen_height); }
                InputEvent::KeyPress { key } => {
                    if key == '\x1F' {
                        self.shell.start_menu.toggle();
                        self.update_start_menu_filter();
                        self.mark_dirty(self.shell.start_menu.rect(screen_height, self.shell.taskbar_height, is_vertical));
                    } else if self.shell.start_menu.is_open {
                        self.handle_start_menu_keys(key, screen_height, is_vertical);
                    } else if key == '\t' && self.input.alt_pressed {
                        if !self.task_switcher_open { self.task_switcher_open = true; self.task_switcher_index = 0; self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize }); }
                        if !self.windows.is_empty() { self.task_switcher_index = (self.task_switcher_index + 1) % self.windows.len(); }
                    } else {
                        if let Some(&top_id) = self.z_order.last() {
                            let mut rect_to_mark = None;
                            if let Some(win) = self.windows.iter_mut().find(|w| w.id == top_id) {
                                if let WindowContent::App(app) = &mut win.content { app.handle_event(&AppEvent::KeyPress { key }); rect_to_mark = Some(win.rect()); }
                            }
                            if let Some(r) = rect_to_mark { self.mark_dirty(r); }
                        }
                    }
                }
            }
        }

        if self.task_switcher_open && !self.input.alt_pressed {
            self.task_switcher_open = false; self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize });
        }

        if mouse_moved {
            if let Some(rect) = start_interaction_rect { self.mark_dirty(rect); }
            if let Some(id) = start_interaction_id { if let Some(rect) = self.get_window_rect(id) { self.mark_dirty(rect); } }
            if let Some(rect) = self.drag_rect { self.mark_dirty(rect); }
            
            if self.shell.start_menu.is_open {
                self.update_start_menu_hover(screen_height, is_vertical);
            }

            // Use hardware-mimicking overlay for zero-lag response
            self.refresh_hardware_cursor();
        }
        self.update_cursor_style();
        if FULL_REDRAW_REQUESTED.load(Ordering::Relaxed) { FULL_REDRAW_REQUESTED.store(false, Ordering::Relaxed); self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize }); }
        if !self.dirty_rects.is_empty() { self.draw_dirty(); }
    }

    fn handle_start_menu_keys(&mut self, key: char, screen_height: isize, is_vertical: bool) {
        let menu_rect = self.shell.start_menu.rect(screen_height, self.shell.taskbar_height, is_vertical);
        if key == '\x08' { self.shell.start_menu.search_query.pop(); self.update_start_menu_filter(); }
        else if key == '\x1B' {
            if !self.shell.start_menu.search_query.is_empty() {
                self.shell.start_menu.search_query.clear();
                self.update_start_menu_filter();
            } else {
                self.shell.start_menu.is_open = false;
            }
        }
        else if key == '\x11' { let c = self.shell.start_menu.filtered_indices.len(); if c > 0 { self.shell.start_menu.selected_index = (self.shell.start_menu.selected_index + c - 1) % c; } }
        else if key == '\x12' { let c = self.shell.start_menu.filtered_indices.len(); if c > 0 { self.shell.start_menu.selected_index = (self.shell.start_menu.selected_index + 1) % c; } }
        else if key == '\n' { if let Some(&real_idx) = self.shell.start_menu.filtered_indices.get(self.shell.start_menu.selected_index) { self.launch_start_menu_item(real_idx, screen_height); } self.shell.start_menu.is_open = false; }
        else if key >= ' ' && key <= '~' { self.shell.start_menu.search_query.push(key); self.update_start_menu_filter(); }
        self.mark_dirty(menu_rect);
    }

    fn update_start_menu_hover(&mut self, screen_height: isize, is_vertical: bool) {
        let menu_rect = self.shell.start_menu.rect(screen_height, self.shell.taskbar_height, is_vertical);
        if !menu_rect.contains(self.input.mouse_x, self.input.mouse_y) { return; }

        let menu_x = menu_rect.x;
        let menu_y = menu_rect.y;
        let item_width = self.shell.start_menu.width - 20;
        let mut draw_y = menu_y + 45;

        for (i, _) in self.shell.start_menu.filtered_indices.iter().enumerate() {
            let btn_rect = Rect { x: menu_x + 10, y: draw_y, width: item_width, height: 30 };
            if btn_rect.contains(self.input.mouse_x, self.input.mouse_y) {
                if self.shell.start_menu.selected_index != i {
                    self.shell.start_menu.selected_index = i;
                    self.mark_dirty(menu_rect);
                }
                break;
            }
            draw_y += 35;
        }
    }

    fn update_cursor_style(&mut self) {
        let mut ns = CursorStyle::Arrow;
        let mut ns_dir = ResizeDirection::None;
        const BS: isize = 5;
        if self.resize_win_id.is_none() {
            if let Some(win) = self.z_order.last().and_then(|&id| self.windows.iter().find(|w| w.id == id)) {
                if !win.minimized && !win.maximized {
                    let (mx, my) = (self.input.mouse_x, self.input.mouse_y);
                    let (ol, or, ot, ob) = (mx >= win.x-BS && mx < win.x+BS, mx >= win.x+win.width as isize-BS && mx < win.x+win.width as isize, my >= win.y-BS && my < win.y+BS, my >= win.y+win.height as isize-BS && my < win.y+win.height as isize);
                    let (ix, iy) = (mx >= win.x && mx < win.x+win.width as isize, my >= win.y && my < win.y+win.height as isize);
                    if (ot && ol) || (ob && or) { ns = CursorStyle::ResizeNWSE; ns_dir = if ot { ResizeDirection::TopLeft } else { ResizeDirection::BottomRight }; }
                    else if (ot && or) || (ob && ol) { ns = CursorStyle::ResizeNESW; ns_dir = if ot { ResizeDirection::TopRight } else { ResizeDirection::BottomLeft }; }
                    else if (ol && iy) || (or && iy) { ns = CursorStyle::ResizeEW; ns_dir = if ol { ResizeDirection::Left } else { ResizeDirection::Right }; }
                    else if (ot && ix) || (ob && ix) { ns = CursorStyle::ResizeNS; ns_dir = if ot { ResizeDirection::Top } else { ResizeDirection::Bottom }; }
                }
            }
        }
        if self.cursor_style != ns || self.resize_direction != ns_dir {
            self.cursor_style = ns; self.resize_direction = ns_dir;
            self.refresh_hardware_cursor();
        }
    }

    fn handle_mouse_button_event(&mut self, button: MouseButton, down: bool, screen_width: isize, screen_height: isize) {
        let is_vertical = self.shell.is_vertical(screen_width as usize);
        let on_taskbar = if is_vertical {
            self.input.mouse_x < self.shell.taskbar_height as isize
        } else {
            self.input.mouse_y >= screen_height - self.shell.taskbar_height as isize
        };

        if let (MouseButton::Right, true) = (button, down) {
            if !on_taskbar {
                if self.context_menu_open { self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 }); }
                self.context_menu_open = true; self.context_menu_x = self.input.mouse_x; self.context_menu_y = self.input.mouse_y; self.shell.start_menu.is_open = false;
                self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 });
            }
        }
        if let (MouseButton::Left, true) = (button, down) {
            self.handle_left_click(screen_width, screen_height);
        } else if let (MouseButton::Left, false) = (button, down) {
            self.handle_left_release();
        }
    }

    fn handle_left_click(&mut self, screen_width: isize, screen_height: isize) {
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        let is_vertical = self.shell.is_vertical(screen_width as usize);
        let tb_thickness = self.shell.taskbar_height as isize;

        // 1. Check for click on start button
        let (start_x, start_y, start_w) = if is_vertical { (0, 0, self.shell.taskbar_height) } else { (0, screen_height.saturating_sub(tb_thickness), 120usize) };
        let start_button = Button::new(start_x, start_y, start_w, self.shell.taskbar_height, locale.start());
        if start_button.contains(self.input.mouse_x, self.input.mouse_y) {
            self.shell.start_menu.toggle();
            self.update_start_menu_filter();
            self.mark_dirty(self.shell.start_menu.rect(screen_height, self.shell.taskbar_height, is_vertical));
            return;
        }

        // 2. Taskbar Window List Click
        let mut clicked_win_id = None;
        if is_vertical {
            if self.input.mouse_x < tb_thickness && self.input.mouse_y >= 40 {
                let mut y_offset = 45;
                for win in &self.windows {
                    let btn = Button::new(2, y_offset, self.shell.taskbar_height.saturating_sub(4), 30, "");
                    if btn.contains(self.input.mouse_x, self.input.mouse_y) { clicked_win_id = Some(win.id); break; }
                    y_offset += 35;
                }
            }
        } else {
            let taskbar_y = screen_height.saturating_sub(tb_thickness);
            if self.input.mouse_y >= taskbar_y && self.input.mouse_x >= 120 {
                let mut x_offset = 120 + 10;
                for win in &self.windows {
                    let btn = Button::new(x_offset, taskbar_y + 2, 100, self.shell.taskbar_height - 4, "");
                    if btn.contains(self.input.mouse_x, self.input.mouse_y) { clicked_win_id = Some(win.id); break; }
                    x_offset += 105;
                }
            }
        }

        if let Some(win_id) = clicked_win_id {
            let (is_minimized, is_top) = {
                let win = self.windows.iter().find(|w| w.id == win_id).unwrap();
                (win.minimized, self.z_order.last() == Some(&win_id))
            };

            if let Some(rect) = self.get_window_rect(win_id) { self.mark_dirty(rect); }

            if is_minimized {
                self.windows.iter_mut().find(|w| w.id == win_id).unwrap().minimized = false;
                if let Some(pos) = self.z_order.iter().position(|&i| i == win_id) {
                    let id = self.z_order.remove(pos);
                    self.z_order.push(id);
                }
            } else if is_top {
                self.windows.iter_mut().find(|w| w.id == win_id).unwrap().minimized = true;
            } else {
                if let Some(pos) = self.z_order.iter().position(|&i| i == win_id) {
                    let id = self.z_order.remove(pos);
                    self.z_order.push(id);
                }
            }
            let tb_rect = if is_vertical { Rect { x: 0, y: 0, width: self.shell.taskbar_height, height: screen_height as usize } } else { Rect { x: 0, y: screen_height.saturating_sub(tb_thickness), width: screen_width as usize, height: self.shell.taskbar_height } };
            self.mark_dirty(tb_rect);
            return;
        }

        // 3. Start Menu Item Click
        let menu_rect = self.shell.start_menu.rect(screen_height, self.shell.taskbar_height, is_vertical);
        if self.shell.start_menu.is_open && menu_rect.contains(self.input.mouse_x, self.input.mouse_y) {
            let mut draw_y = menu_rect.y + 45;
            let menu_x = menu_rect.x;
            let app_list = self.shell.get_start_menu_data();
            
            let mut clicked_idx = None;
            for &real_idx in &self.shell.start_menu.filtered_indices {
                let btn = Button::new(menu_x + 10, draw_y, self.shell.start_menu.width - 20, 30, app_list[real_idx].0);
                if btn.contains(self.input.mouse_x, self.input.mouse_y) {
                    clicked_idx = Some(real_idx);
                    break;
                }
                draw_y += 35;
            }

            if let Some(idx) = clicked_idx {
                self.launch_start_menu_item(idx, screen_height);
                self.shell.start_menu.is_open = false;
                self.mark_dirty(menu_rect);
            }
            return;
        }

        // 4. Window Interactions (Focus, Drag, Resize)
        let mut clicked_win_id = None;
        for &win_id in self.z_order.iter().rev() {
            if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                if win.minimized { continue; }
                if win.rect().contains(self.input.mouse_x, self.input.mouse_y) {
                    clicked_win_id = Some(win_id);
                    break;
                }
            }
        }

        if let Some(win_id) = clicked_win_id {
            self.click_target_id = Some(win_id);
            
            // 1. Handle Z-Order
            if let Some(pos) = self.z_order.iter().position(|&id| id == win_id) {
                let id = self.z_order.remove(pos);
                self.z_order.push(id);
            }

            // 2. Determine if we are clicking the titlebar for dragging
            let (wx, wy, ww, maximized) = {
                let win = self.windows.iter().find(|w| w.id == win_id).unwrap();
                (win.x, win.y, win.width, win.maximized)
            };

            let title_h = (if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 }) + 6;
            if self.input.mouse_y < wy + title_h as isize {
                let close_btn = Button::new(wx + ww as isize - 20, wy + 3, 16, 16, "x");
                if close_btn.contains(self.input.mouse_x, self.input.mouse_y) {
                    self.windows.retain(|w| w.id != win_id);
                    self.z_order.retain(|&id| id != win_id);
                    self.mark_dirty(Rect { x: 0, y: screen_height - self.shell.taskbar_height as isize, width: 800, height: self.shell.taskbar_height });
                } else if !maximized {
                    self.drag_win_id = Some(win_id);
                    self.drag_offset_x = self.input.mouse_x - wx;
                    self.drag_offset_y = self.input.mouse_y - wy;
                }
            }
            
            if let Some(rect) = self.get_window_rect(win_id) { self.mark_dirty(rect); }
        }
    }

    fn handle_left_release(&mut self) {
        if self.resize_win_id.is_some() { self.resize_win_id = None; self.resize_direction = ResizeDirection::None; }
        else if let Some(win_id) = self.drag_win_id {
            if let Some(rect) = self.drag_rect {
                if let Some(old) = self.get_window_rect(win_id) { self.mark_dirty(old); }
                if let Some(win) = self.windows.iter_mut().find(|w| w.id == win_id) { win.x = rect.x; win.y = rect.y; }
                self.mark_dirty(rect);
            }
            self.drag_win_id = None; self.drag_rect = None;
        }
        self.click_target_id = None;
    }

    fn update_start_menu_filter(&mut self) {
        self.shell.start_menu.filtered_indices.clear();
        let app_list = self.shell.get_start_menu_data();
        for (i, &(name, _)) in app_list.iter().enumerate() {
            if self.shell.start_menu.search_query.is_empty() || self.contains_insensitive(name, self.shell.start_menu.search_query.as_str()) {
                self.shell.start_menu.filtered_indices.push(i);
            }
        }
        // Selection always resets to the top match when filtering for a better UX
        self.shell.start_menu.selected_index = 0;
    }
    fn contains_insensitive(&self, haystack: &str, needle: &str) -> bool { if needle.is_empty() { return true; } haystack.as_bytes().windows(needle.len()).any(|window| window.eq_ignore_ascii_case(needle.as_bytes())) }

    fn draw_dirty(&mut self) {
        if self.dirty_rects.is_empty() { return; }
        let raw_dirty = core::mem::take(&mut self.dirty_rects);
        let mut final_rects = Vec::new();
        for r in raw_dirty {
            let mut current = r; let mut i = 0;
            while i < final_rects.len() { if current.intersects(&final_rects[i]) { let other = final_rects.remove(i); current = current.union(&other); i = 0; } else { i += 1; } }
            final_rects.push(current);
        }
        let fb_info = if let Some(info) = FRAMEBUFFER.lock().info.as_ref() { framebuffer::FramebufferInfo { address: info.address, width: info.width, height: info.height, pitch: info.pitch, bpp: info.bpp } } else { return };
        if self.backbuffer.len() != fb_info.width * fb_info.height { self.backbuffer.resize(fb_info.width * fb_info.height, 0); }
        let mut local_fb = framebuffer::Framebuffer::new(); local_fb.info = Some(fb_info); local_fb.draw_buffer = Some(core::mem::take(&mut self.backbuffer));

        for dirty_rect in &final_rects {
            self.draw_desktop_bg(&mut local_fb, *dirty_rect);
            for &id in &self.z_order { if let Some(win) = self.windows.iter().find(|w| w.id == id) { if !win.minimized && dirty_rect.intersects(&win.rect()) { self.draw_window(&mut local_fb, win, *dirty_rect); } } }
            self.shell.draw(&mut local_fb, self.windows.as_slice(), self.input.mouse_x, self.input.mouse_y, *dirty_rect, self.shell.start_menu.is_open, if self.context_menu_open { Some((self.context_menu_x, self.context_menu_y)) } else { None });
            if self.task_switcher_open { self.shell.draw_task_switcher(&mut local_fb, *dirty_rect, self.z_order.as_slice(), self.windows.as_slice(), self.task_switcher_index); }
        }

        // Erase the current cursor from VRAM before blitting new pixels
        self.restore_hardware_cursor();

        unsafe { core::arch::asm!("cli") };
        {
            let fb = FRAMEBUFFER.lock();
            if let (Some(ref info), Some(ref src)) = (&fb.info, &local_fb.draw_buffer) {
                for dr in final_rects {
                    let x = dr.x.max(0) as usize; let y = dr.y.max(0) as usize;
                    let w = dr.width.min(info.width.saturating_sub(x)); let h = dr.height.min(info.height.saturating_sub(y));
                    for i in 0..h {
                        let cy = y + i;
                        unsafe {
                            let src_ptr = src.as_ptr().add(cy * info.width + x);
                            let dst_ptr = (info.address as *mut u32).add(cy * (info.pitch / 4) + x);
                            core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, w);
                        }
                    }
                }
            }
        }
        unsafe { core::arch::asm!("sti") };

        // After blitting the new frame, the hardware cursor's background is invalid.
        self.cursor_save_buffer.clear();
        self.refresh_hardware_cursor();

        self.backbuffer = local_fb.draw_buffer.take().unwrap();
    }

    fn draw_desktop_bg(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        let _info = if let Some(info) = &fb.info { info } else { return };
        for y in clip.y..(clip.y + clip.height as isize) {
            let _start_c = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
            let _end_c = DESKTOP_GRADIENT_END.load(Ordering::Relaxed);
            // Gradient math omitted for brevity...
            for x in clip.x..(clip.x + clip.width as isize) { fb.set_pixel(x as usize, y as usize, 0x00_10_20_40); }
        }
    }

    fn draw_window(&self, fb: &mut framebuffer::Framebuffer, win: &Window, clip: Rect) {
        let is_active = self.z_order.last() == Some(&win.id);
        let title_height = (if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 }) + 6;
        draw_rect(fb, win.x, win.y, win.width, win.height, win.color, Some(clip));
        draw_rect(fb, win.x, win.y, win.width, title_height, if is_active { 0x00_00_40_80 } else { 0x00_40_40_40 }, Some(clip));
        font::draw_string(fb, win.x + 6, win.y + 4, win.title, 0x00_FFFFFF, Some(clip));
        if let WindowContent::App(app) = &win.content {
            let cr = clip.intersection(&Rect { x: win.x, y: win.y + title_height as isize, width: win.width, height: win.height.saturating_sub(title_height) }).unwrap();
            fb.set_clip(cr.x as usize, cr.y as usize, cr.width, cr.height);
            app.draw(fb, win);
            fb.clear_clip();
        }
    }

    fn restore_hardware_cursor(&self) {
        let fb = FRAMEBUFFER.lock();
        self.restore_vram_cursor(&fb);
    }

    /// Internal version of restore that uses an already-locked framebuffer to avoid deadlocks.
    fn restore_vram_cursor(&self, fb: &framebuffer::Framebuffer) {
        let (width, height, addr, pitch) = if let Some(info) = fb.info.as_ref() {
            (info.width as isize, info.height as isize, info.address, info.pitch)
        } else { return };

        if !self.cursor_save_buffer.is_empty() {
            let src_ptr = self.cursor_save_buffer.as_ptr() as *const u8;
            let dst_base = addr as *mut u8;

            for i in 0..CURSOR_HEIGHT {
                let cy = self.last_cursor_y + i as isize;
                if cy < 0 || cy >= height { continue; }
                let cx = self.last_cursor_x;
                let mut draw_w = CURSOR_WIDTH as isize;
                let mut start_x = 0;
                if cx < 0 { start_x = -cx; draw_w += cx; }
                if cx + (CURSOR_WIDTH as isize) > width { draw_w = width.saturating_sub(cx).max(0); }
                if draw_w <= 0 { continue; }

                unsafe {
                    core::ptr::copy_nonoverlapping(
                        src_ptr.add((i * CURSOR_WIDTH + start_x as usize) * 4),
                        dst_base.add(cy as usize * pitch + (cx + start_x) as usize * 4),
                        draw_w as usize * 4
                    );
                }
            }
        }
    }

    fn refresh_hardware_cursor(&mut self) {
        let fb = FRAMEBUFFER.lock();
        let (width, height, addr, pitch) = if let Some(info) = fb.info.as_ref() {
            (info.width as isize, info.height as isize, info.address, info.pitch)
        } else { return };

        // 1. Restore old background to VRAM using the current lock
        self.restore_vram_cursor(&fb);

        // 2. Save new background from VRAM
        self.last_cursor_x = self.input.mouse_x;
        self.last_cursor_y = self.input.mouse_y;
        self.cursor_save_buffer.resize(CURSOR_WIDTH * CURSOR_HEIGHT, 0);
        
        let dst_ptr = self.cursor_save_buffer.as_mut_ptr() as *mut u8;
        let src_base = addr as *const u8;

        for i in 0..CURSOR_HEIGHT {
            let cy = self.last_cursor_y + i as isize;
            if cy < 0 || cy >= height { continue; }
            let cx = self.last_cursor_x;
            let mut read_w = CURSOR_WIDTH as isize;
            let mut start_x = 0;
            if cx < 0 { start_x = -cx; read_w += cx; }
            if cx + (CURSOR_WIDTH as isize) > width { read_w = width.saturating_sub(cx).max(0); }
            
            if read_w > 0 {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        src_base.add(cy as usize * pitch + (cx + start_x) as usize * 4),
                        dst_ptr.add((i * CURSOR_WIDTH + start_x as usize) * 4),
                        read_w as usize * 4
                    );
                }
            }
        }

        // 3. Draw cursor bitmap directly to VRAM
        const ARROW_BITMAP: [[u8; 12]; 17] = [
                [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                [1, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
                [1, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0],
                [1, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0],
                [1, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0],
                [1, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0],
                [1, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0],
                [1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0],
                [1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 0],
                [1, 2, 2, 1, 2, 2, 1, 0, 0, 0, 0, 0],
                [1, 2, 1, 0, 1, 2, 2, 1, 0, 0, 0, 0],
                [1, 1, 0, 0, 1, 2, 2, 1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
                [0, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
                [0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0],
        ];

        let cursor_bitmap = match self.cursor_style {
            CursorStyle::Arrow => ARROW_BITMAP,
            _ => ARROW_BITMAP, // Fallback to arrow for resize styles until bitmaps are implemented
        };

        let vram = addr as *mut u32;
        for (dy, row) in cursor_bitmap.iter().enumerate() {
            let py = self.last_cursor_y + dy as isize;
            if py < 0 || py >= height { continue; }
            for (dx, &pixel) in row.iter().enumerate() {
                if pixel == 0 { continue; }
                let px = self.last_cursor_x + dx as isize;
                if px < 0 || px >= width { continue; }
                
                let color = if pixel == 1 { 0x00_00_00_00 } else { 0x00_FF_FF_FF };
                unsafe {
                    let offset = py as usize * (pitch / 4) + px as usize;
                    *vram.add(offset) = color;
                }
            }
        }
    }

    fn launch_start_menu_item(&mut self, index: usize, _screen_height: isize) {
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();

        match index {
            0 => self.add_window(Window {
                id: 0, x: 150, y: 150, width: 400, height: 300,
                color: 0x00_1E_1E_1E, title: locale.app_text_editor(),
                content: WindowContent::App(Box::new(TextEditor::new())),
                minimized: false, maximized: false, restore_rect: None,
            }),
            1 => self.add_window(Window {
                id: 0, x: 50, y: 350, width: 200, height: 220,
                color: 0x00_20_20_20, title: locale.app_calculator(),
                content: WindowContent::App(Box::new(Calculator::new())),
                minimized: false, maximized: false, restore_rect: None,
            }),
            2 => self.add_window(Window {
                id: 0, x: 180, y: 100, width: 400, height: 300,
                color: 0x00_20_20_20, title: locale.app_paint(),
                content: WindowContent::App(Box::new(Paint::new())),
                minimized: false, maximized: false, restore_rect: None,
            }),
            3 => self.add_window(Window {
                id: 0, x: 250, y: 250, width: 300, height: 300,
                color: 0x00_40_20_40, title: locale.app_settings(),
                content: WindowContent::App(Box::new(Settings::new())),
                minimized: false, maximized: false, restore_rect: None,
            }),
            4 => self.add_window(Window {
                id: 0, x: 100, y: 100, width: 480, height: 320,
                color: 0x00_1E_1E_1E, title: locale.app_terminal(),
                content: WindowContent::App(Box::new(Terminal::new())),
                minimized: false, maximized: false, restore_rect: None,
            }),
            5 => self.add_window(Window {
                id: 0, x: 150, y: 150, width: 300, height: 400,
                color: 0x00_00_40_40, title: "Task Manager",
                content: WindowContent::App(Box::new(TaskManager::new())),
                minimized: false, maximized: false, restore_rect: None,
            }),
            6 => self.add_window(Window {
                id: 0, x: 200, y: 200, width: 400, height: 300,
                color: 0x00_00_20_40, title: "Partition Manager",
                content: WindowContent::App(Box::new(PartitionManager::new())),
                minimized: false, maximized: false, restore_rect: None,
            }),
            7 => { crate::kernel::power::reboot(); }
            8 => crate::kernel::power::shutdown(),
            _ => {}
        }
    }
}

pub struct InputManager {
    pub mouse_x: isize, pub mouse_y: isize,
    pub left_button_pressed: bool, pub right_button_pressed: bool,
    pub alt_pressed: bool, pub ctrl_pressed: bool, pub shift_pressed: bool, pub win_pressed: bool,
    pub event_queue: Vec<InputEvent>,
}

impl InputManager {
    pub const fn new() -> Self { Self { mouse_x: 400, mouse_y: 300, left_button_pressed: false, right_button_pressed: false, alt_pressed: false, ctrl_pressed: false, shift_pressed: false, win_pressed: false, event_queue: Vec::new() } }
    pub fn update(&mut self, max_w: isize, max_h: isize) {
        self.event_queue.clear();
        while let Some(packet) = mouse::get_packet() {
            let mut dx = packet.x as isize;
            let mut dy = -(packet.y as isize);

            // Cursor Acceleration Curve
            let max_delta = dx.abs().max(dy.abs());
            let accel = if max_delta > 15 { 4 } else if max_delta > 8 { 3 } else if max_delta > 3 { 2 } else { 1 };
            dx *= accel; dy *= accel;

            let sens = (MOUSE_SENSITIVITY.load(Ordering::Relaxed) * 2) as isize;
            let final_dx = (dx * sens) / 100;
            let final_dy = (dy * sens) / 100;

            self.mouse_x = (self.mouse_x + final_dx).clamp(0, max_w - 1);
            self.mouse_y = (self.mouse_y + final_dy).clamp(0, max_h - 1);

            if final_dx != 0 || final_dy != 0 {
                self.event_queue.push(InputEvent::MouseMove { x: self.mouse_x, y: self.mouse_y, dx: final_dx, dy: final_dy });
            }

            let left = (packet.buttons & 0x1) != 0;
            if left != self.left_button_pressed { self.left_button_pressed = left; self.event_queue.push(InputEvent::MouseButton { button: MouseButton::Left, down: left, x: self.mouse_x, y: self.mouse_y }); }
            
            let right = (packet.buttons & 0x2) != 0;
            if right != self.right_button_pressed { self.right_button_pressed = right; self.event_queue.push(InputEvent::MouseButton { button: MouseButton::Right, down: right, x: self.mouse_x, y: self.mouse_y }); }

            if packet.wheel != 0 { self.event_queue.push(InputEvent::Scroll { delta: packet.wheel as isize, x: self.mouse_x, y: self.mouse_y }); }
        }
        
        self.alt_pressed = keyboard::is_alt_pressed();
        self.ctrl_pressed = keyboard::is_ctrl_pressed();
        self.shift_pressed = keyboard::is_shift_pressed();
        self.win_pressed = keyboard::is_win_pressed();

        while let Some(key) = keyboard::get_char() { self.event_queue.push(InputEvent::KeyPress { key }); }
    }
}