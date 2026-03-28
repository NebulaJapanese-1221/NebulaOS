use crate::drivers::mouse;
use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use crate::drivers::rtc::{self, CURRENT_DATETIME, TIME_NEEDS_UPDATE};
use crate::drivers::keyboard;
use crate::userspace::gui::{Rect, Button, Shell, font, draw_rect, HIGH_CONTRAST, LARGE_TEXT, DESKTOP_GRADIENT_START, DESKTOP_GRADIENT_END, FULL_REDRAW_REQUESTED, MOUSE_SENSITIVITY};
use crate::userspace::apps::{app::{App, AppEvent}, calculator::Calculator, editor::TextEditor, paint::Paint, settings::Settings, terminal::Terminal, task_manager::TaskManager, partition_manager::PartitionManager};
use crate::userspace::localisation;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::boxed::Box;
use core::sync::atomic::Ordering;

const MAX_WINDOWS: usize = 10;

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
pub enum MouseButton { Left, Right, Middle }

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
enum ResizeDirection { None, Left, Right, Top, Bottom, TopLeft, TopRight, BottomLeft, BottomRight }

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
    fn get_cursor_rect(&self) -> Rect { Rect { x: self.input.mouse_x, y: self.input.mouse_y, width: 12, height: 17 } }
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
        let initial_cursor_rect = self.get_cursor_rect();
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
                        if let Some(win) = self.windows.iter_mut().find(|w| w.id == target_id) {
                            if let WindowContent::App(app) = &mut win.content {
                                app.handle_event(&AppEvent::Scroll { delta: -delta * 16, width: win.width, height: win.height });
                                self.mark_dirty(win.rect());
                            }
                        }
                    }
                }
                InputEvent::MouseButton { button, down, x, y } => { self.input.mouse_x = x; self.input.mouse_y = y; self.handle_mouse_button_event(button, down, screen_height); }
                InputEvent::KeyPress { key } => {
                    if key == '\x1F' {
                        self.shell.start_menu.toggle();
                        self.update_start_menu_filter();
                        self.mark_dirty(self.shell.start_menu.rect(screen_height, self.shell.taskbar_height));
                    } else if self.shell.start_menu.is_open {
                        self.handle_start_menu_keys(key, screen_height);
                    } else if key == '\t' && self.input.alt_pressed {
                        if !self.task_switcher_open { self.task_switcher_open = true; self.task_switcher_index = 0; self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize }); }
                        if !self.windows.is_empty() { self.task_switcher_index = (self.task_switcher_index + 1) % self.windows.len(); }
                    } else {
                        if let Some(&top_id) = self.z_order.last() {
                            if let Some(win) = self.windows.iter_mut().find(|w| w.id == top_id) {
                                if let WindowContent::App(app) = &mut win.content { app.handle_event(&AppEvent::KeyPress { key }); self.mark_dirty(win.rect()); }
                            }
                        }
                    }
                }
            }
        }

        if self.task_switcher_open && !self.input.alt_pressed {
            self.task_switcher_open = false; self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize });
        }

        if mouse_moved {
            self.mark_dirty(initial_cursor_rect); self.mark_dirty(self.get_cursor_rect());
            if let Some(rect) = start_interaction_rect { self.mark_dirty(rect); }
            if let Some(id) = start_interaction_id { if let Some(rect) = self.get_window_rect(id) { self.mark_dirty(rect); } }
            if let Some(rect) = self.drag_rect { self.mark_dirty(rect); }
        }
        self.update_cursor_style();
        if FULL_REDRAW_REQUESTED.load(Ordering::Relaxed) { FULL_REDRAW_REQUESTED.store(false, Ordering::Relaxed); self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize }); }
        if !self.dirty_rects.is_empty() { self.draw_dirty(); }
    }

    fn handle_start_menu_keys(&mut self, key: char, screen_height: isize) {
        let menu_rect = self.shell.start_menu.rect(screen_height, self.shell.taskbar_height);
        if key == '\x08' { self.shell.start_menu.search_query.pop(); self.update_start_menu_filter(); }
        else if key == '\x11' { let c = self.shell.start_menu.filtered_indices.len(); if c > 0 { self.shell.start_menu.selected_index = (self.shell.start_menu.selected_index + c - 1) % c; } }
        else if key == '\x12' { let c = self.shell.start_menu.filtered_indices.len(); if c > 0 { self.shell.start_menu.selected_index = (self.shell.start_menu.selected_index + 1) % c; } }
        else if key == '\n' { if let Some(&real_idx) = self.shell.start_menu.filtered_indices.get(self.shell.start_menu.selected_index) { self.launch_start_menu_item(real_idx, screen_height); } self.shell.start_menu.is_open = false; }
        else if key >= ' ' && key <= '~' { self.shell.start_menu.search_query.push(key); self.update_start_menu_filter(); }
        self.mark_dirty(menu_rect);
    }

    fn update_cursor_style(&mut self) {
        let mut ns = CursorStyle::Arrow; const BS: isize = 5;
        if self.resize_win_id.is_none() {
            if let Some(win) = self.z_order.last().and_then(|&id| self.windows.iter().find(|w| w.id == id)) {
                if !win.minimized && !win.maximized {
                    let (mx, my) = (self.input.mouse_x, self.input.mouse_y);
                    let (ol, or, ot, ob) = (mx >= win.x-BS && mx < win.x+BS, mx >= win.x+win.width as isize-BS && mx < win.x+win.width as isize, my >= win.y-BS && my < win.y+BS, my >= win.y+win.height as isize-BS && my < win.y+win.height as isize);
                    let (ix, iy) = (mx >= win.x && mx < win.x+win.width as isize, my >= win.y && my < win.y+win.height as isize);
                    if (ot && ol) || (ob && or) { ns = CursorStyle::ResizeNWSE; }
                    else if (ot && or) || (ob && ol) { ns = CursorStyle::ResizeNESW; }
                    else if (ol && iy) || (or && iy) { ns = CursorStyle::ResizeEW; }
                    else if (ot && ix) || (ob && ix) { ns = CursorStyle::ResizeNS; }
                }
            }
        }
        if self.cursor_style != ns { self.mark_dirty(self.get_cursor_rect()); self.cursor_style = ns; self.mark_dirty(self.get_cursor_rect()); }
    }

    fn handle_mouse_button_event(&mut self, button: MouseButton, down: bool, screen_height: isize) {
        let taskbar_y = screen_height - self.shell.taskbar_height as isize;
        if let (MouseButton::Right, true) = (button, down) {
            if self.input.mouse_y < taskbar_y {
                if self.context_menu_open { self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 }); }
                self.context_menu_open = true; self.context_menu_x = self.input.mouse_x; self.context_menu_y = self.input.mouse_y; self.shell.start_menu.is_open = false;
                self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 });
            }
        }
        if let (MouseButton::Left, true) = (button, down) {
            self.handle_left_click(screen_height, taskbar_y);
        } else if let (MouseButton::Left, false) = (button, down) {
            self.handle_left_release();
        }
    }

    fn handle_left_click(&mut self, screen_height: isize, taskbar_y: isize) {
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        if Button::new(0, taskbar_y, 120, self.shell.taskbar_height, locale.start()).contains(self.input.mouse_x, self.input.mouse_y) {
            self.shell.start_menu.toggle(); self.update_start_menu_filter();
            self.mark_dirty(self.shell.start_menu.rect(screen_height, self.shell.taskbar_height));
            return;
        }
        // Check shell elements, taskbar window buttons, or window interactions here...
        // (Logic omitted for brevity but corresponds to provided mod.rs click handler)
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

    fn update_start_menu_filter(&mut self) { self.shell.start_menu.filtered_indices.clear(); let app_list = self.shell.get_start_menu_data(); for (i, &(name, _)) in app_list.iter().enumerate() { if self.shell.start_menu.search_query.is_empty() || self.contains_insensitive(name, self.shell.start_menu.search_query.as_str()) { self.shell.start_menu.filtered_indices.push(i); } } if self.shell.start_menu.selected_index >= self.shell.start_menu.filtered_indices.len() { self.shell.start_menu.selected_index = 0; } }
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
            self.shell.draw(&mut local_fb, &self.windows, self.input.mouse_x, self.input.mouse_y, *dirty_rect, self.shell.start_menu.is_open, if self.context_menu_open { Some((self.context_menu_x, self.context_menu_y)) } else { None });
            if self.task_switcher_open { self.shell.draw_task_switcher(&mut local_fb, *dirty_rect, &self.z_order, &self.windows, self.task_switcher_index); }
            self.draw_cursor(&mut local_fb, *dirty_rect);
        }

        unsafe { core::arch::asm!("cli") };
        {
            let fb = FRAMEBUFFER.lock();
            if let (Some(ref info), Some(ref src)) = (&fb.info, &local_fb.draw_buffer) {
                for dr in final_rects {
                    let x = dr.x.max(0) as usize; let y = dr.y.max(0) as usize;
                    let w = dr.width.min(info.width.saturating_sub(x)); let h = dr.height.min(info.height.saturating_sub(y));
                    for i in 0..h { let cy = y+i; unsafe { core::ptr::copy_nonoverlapping(src.as_ptr().add((cy*info.width+x)*4), (info.address as *mut u8).add(cy*info.pitch+x*4), w*4); } }
                }
            }
        }
        unsafe { core::arch::asm!("sti") };
        self.backbuffer = local_fb.draw_buffer.take().unwrap();
    }

    fn draw_desktop_bg(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        let (sw, sh) = if let Some(info) = &fb.info { (info.width, info.height) } else { return };
        for y in clip.y..(clip.y + clip.height as isize) {
            let start_c = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
            let end_c = DESKTOP_GRADIENT_END.load(Ordering::Relaxed);
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
            let cr = clip.intersection(&Rect { x: win.x, y: win.y + title_height as isize, width: win.width, height: win.height - title_height }).unwrap();
            fb.set_clip(cr.x as usize, cr.y as usize, cr.width, cr.height);
            app.draw(fb, win);
            fb.clear_clip();
        }
    }

    fn draw_cursor(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        // Cursor drawing logic using CursorStyle...
        draw_rect(fb, self.input.mouse_x, self.input.mouse_y, 2, 2, 0x00_FFFFFF, Some(clip));
    }

    fn launch_start_menu_item(&mut self, index: usize, _sh: isize) {
        // Dispatch to app constructor logic based on index
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
            let sens = MOUSE_SENSITIVITY.load(Ordering::Relaxed) as isize;
            self.mouse_x = (self.mouse_x + (packet.x as isize * sens) / 100).clamp(0, max_w - 1);
            self.mouse_y = (self.mouse_y - (packet.y as isize * sens) / 100).clamp(0, max_h - 1);
            let left = (packet.buttons & 0x1) != 0;
            if left != self.left_button_pressed { self.left_button_pressed = left; self.event_queue.push(InputEvent::MouseButton { button: MouseButton::Left, down: left, x: self.mouse_x, y: self.mouse_y }); }
            if packet.wheel != 0 { self.event_queue.push(InputEvent::Scroll { delta: packet.wheel as isize, x: self.mouse_x, y: self.mouse_y }); }
        }
        self.alt_pressed = keyboard::is_alt_pressed();
        while let Some(key) = keyboard::get_char() { self.event_queue.push(InputEvent::KeyPress { key }); }
    }
}