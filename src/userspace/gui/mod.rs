//! GUI components for NebulaOS.
//! NOTE: This module is currently being refactored to support a graphical framebuffer
//! instead of the old text-mode VGA buffer. The drawing logic is temporarily disabled.

use crate::drivers::mouse;
use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use crate::drivers::rtc::{self, CURRENT_DATETIME, TIME_NEEDS_UPDATE};
use crate::drivers::keyboard;
use alloc::vec::Vec;
use alloc::string::ToString;
use alloc::format;
use crate::userspace::apps::{app::{App, AppEvent}, calculator::Calculator, editor::TextEditor, paint::Paint, settings::Settings, terminal::Terminal, task_manager::TaskManager};
use spin::Mutex;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
pub mod rect;
use self::rect::Rect;
pub mod button;
use self::button::Button;
use crate::userspace::localisation;

// Re-export fonts from their new location so existing references to gui::font work
pub use crate::userspace::fonts::font;

const MAX_WINDOWS: usize = 10;
const TOP_BAR_HEIGHT: usize = 26;

pub static DESKTOP_GRADIENT_START: AtomicU32 = AtomicU32::new(0x00_10_20_40);
pub static DESKTOP_GRADIENT_END: AtomicU32 = AtomicU32::new(0x00_50_80_B0);
pub static FULL_REDRAW_REQUESTED: AtomicBool = AtomicBool::new(false);
pub static HIGH_CONTRAST: AtomicBool = AtomicBool::new(false);
pub static LARGE_TEXT: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct Window {
    pub id: usize,
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,    
    pub color: u32, // Now an RGB color value
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
enum CursorStyle {
    Arrow,
    ResizeNS, // North-South
    ResizeEW, // East-West
    ResizeNESW, // North-East South-West
    ResizeNWSE, // North-West South-East
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResizeDirection {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    MouseMove { x: isize, y: isize, dx: isize, dy: isize },
    MouseButton { button: MouseButton, down: bool, x: isize, y: isize },
    Scroll { delta: isize },
    KeyPress { key: char },
}

pub struct InputManager {
    pub mouse_x: isize,
    pub mouse_y: isize,
    pub left_button_pressed: bool,
    pub right_button_pressed: bool,
    pub alt_pressed: bool,
    pub ctrl_pressed: bool,
    pub shift_pressed: bool,
    pub event_queue: Vec<InputEvent>,
}

impl InputManager {
    pub const fn new() -> Self {
        Self {
            mouse_x: 40,
            mouse_y: 12,
            left_button_pressed: false,
            right_button_pressed: false,
            alt_pressed: false,
            ctrl_pressed: false,
            shift_pressed: false,
            event_queue: Vec::new(),
        }
    }

    pub fn update(&mut self, max_w: isize, max_h: isize) {
        self.event_queue.clear();

        while let Some(packet) = mouse::get_packet() {
            let dx = packet.x as isize;
            let dy = -(packet.y as isize); // PS/2 Y-axis is inverted
            self.mouse_x = (self.mouse_x + dx).clamp(0, max_w - 1);
            self.mouse_y = (self.mouse_y + dy).clamp(0, max_h - 1);

            if dx != 0 || dy != 0 {
                self.event_queue.push(InputEvent::MouseMove { x: self.mouse_x, y: self.mouse_y, dx, dy });
            }

            let left = (packet.buttons & 0x1) != 0;
            let right = (packet.buttons & 0x2) != 0;

            if left != self.left_button_pressed {
                self.left_button_pressed = left;
                self.event_queue.push(InputEvent::MouseButton { button: MouseButton::Left, down: left, x: self.mouse_x, y: self.mouse_y });
            }
            if right != self.right_button_pressed {
                self.right_button_pressed = right;
                self.event_queue.push(InputEvent::MouseButton { button: MouseButton::Right, down: right, x: self.mouse_x, y: self.mouse_y });
            }

            if packet.wheel != 0 {
                self.event_queue.push(InputEvent::Scroll { delta: packet.wheel as isize });
            }
        }

        // Poll modifier states from the keyboard driver
        self.alt_pressed = keyboard::is_alt_pressed(); 
        self.ctrl_pressed = keyboard::is_ctrl_pressed();
        self.shift_pressed = keyboard::is_shift_pressed();

        while let Some(key) = keyboard::get_char() {
            // Interrupts are handled inside get_char, so this is safe to loop
            self.event_queue.push(InputEvent::KeyPress { key });
        }
    }
}

pub struct WindowManager {
    windows: Vec<Window>,
    z_order: Vec<usize>,
    next_win_id: usize,
    input: InputManager,
    drag_win_id: Option<usize>,
    drag_offset_x: isize,
    drag_offset_y: isize,
    click_target_id: Option<usize>,
    start_menu_open: bool,
    dirty_rects: Vec<Rect>,
    context_menu_open: bool,
    context_menu_x: isize,
    context_menu_y: isize,
    resize_win_id: Option<usize>,
    resize_direction: ResizeDirection,
    cursor_style: CursorStyle,
    backbuffer: Vec<u32>,
    drag_rect: Option<Rect>,
    task_switcher_open: bool,
    task_switcher_index: usize,
    osd_timeout: usize,
    brightness_osd_timeout: usize,
    tooltip: Option<(alloc::string::String, isize, isize)>,
    volume_popup_open: bool,
    dragging_volume_slider: bool,
}

impl WindowManager {
    pub const fn new() -> Self {
        WindowManager {
            windows: Vec::new(),
            z_order: Vec::new(),
            next_win_id: 0,
            input: InputManager::new(), // Now creates Vec, so non-const
            drag_win_id: None,
            drag_offset_x: 0,
            drag_offset_y: 0,
            click_target_id: None,
            start_menu_open: false,
            dirty_rects: Vec::new(),
            context_menu_open: false,
            context_menu_x: 0,
            context_menu_y: 0,
            resize_win_id: None,
            resize_direction: ResizeDirection::None,
            cursor_style: CursorStyle::Arrow,
            backbuffer: Vec::new(),
            drag_rect: None,
            task_switcher_open: false,
            task_switcher_index: 0,
            osd_timeout: 0,
            brightness_osd_timeout: 0,
            tooltip: None,
            volume_popup_open: false,
            dragging_volume_slider: false,
        }
    }

    pub fn add_window(&mut self, mut window: Window) {
        if self.windows.len() < MAX_WINDOWS {
            window.id = self.next_win_id;
            self.next_win_id += 1;
            self.z_order.push(window.id);
            self.mark_dirty(Rect { x: window.x, y: window.y, width: window.width, height: window.height });
            self.windows.push(window);
        }
    }

    fn mark_dirty(&mut self, rect: Rect) {
        self.dirty_rects.push(rect);
    }

    fn get_cursor_rect(&self) -> Rect {
        Rect { x: self.input.mouse_x, y: self.input.mouse_y, width: 12, height: 17 }
    }

    fn get_window_rect(&self, win_id: usize) -> Option<Rect> {
        self.windows.iter().find(|w| w.id == win_id).map(|w| Rect {
            x: w.x, y: w.y, width: w.width, height: w.height
        })
    }

    pub fn update(&mut self) {
        // The old redraw logic is replaced by dirty rect tracking.
        // We no longer use boolean flags to trigger a full redraw.

        // Synchronize hardware power status (Battery/Thermal)
        crate::kernel::acpi::update_power_status();

        // Check if time needs update
        if TIME_NEEDS_UPDATE.load(Ordering::Relaxed) {
            TIME_NEEDS_UPDATE.store(false, Ordering::Relaxed);
            // Read RTC and update global time
            let new_time = rtc::read_time();
            let mut current_dt = CURRENT_DATETIME.lock();
            if *current_dt != new_time { // Only redraw if time actually changed
                *current_dt = new_time;
                if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                    let clock_width = (19 * 8) + 10;
                    let rect = Rect { x: info.width as isize - clock_width as isize - 5, y: 0, width: clock_width, height: TOP_BAR_HEIGHT };
                    self.mark_dirty(rect);
                }
            }
            drop(current_dt); // Release lock early
        }

        // Handle OSD timeout
        if self.osd_timeout > 0 {
            self.osd_timeout -= 1;
            if self.osd_timeout == 0 {
                if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                    let osd_w = 200; let osd_h = 60;
                    self.mark_dirty(Rect {
                        x: (info.width as isize - osd_w) / 2,
                        y: (info.height as isize - osd_h) / 2,
                        width: osd_w as usize, height: osd_h as usize
                    });
                }
            }
        }

        // Handle Brightness OSD timeout
        if self.brightness_osd_timeout > 0 {
            self.brightness_osd_timeout -= 1;
            if self.brightness_osd_timeout == 0 {
                if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                    let osd_w = 200; let osd_h = 60;
                    self.mark_dirty(Rect {
                        x: (info.width as isize - osd_w) / 2,
                        y: (info.height as isize - osd_h) / 2,
                        width: osd_w as usize, height: osd_h as usize
                    });
                }
            }
        }
        // Cache screen dimensions to avoid locking Framebuffer in the loop
        let (screen_width, screen_height) = if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
            (info.width as isize, info.height as isize)
        } else {
            (800, 600) // Fallback
        };

        // Tooltip logic for Battery Indicator
        let mut new_tooltip = None;
        let top_bar_y = 0;
        let bat_x = screen_width - 700; // Even wider spacing to prevent collisions
        let bat_hit_rect = Rect { x: bat_x, y: top_bar_y, width: 180, height: TOP_BAR_HEIGHT as usize };

        if bat_hit_rect.contains(self.input.mouse_x, self.input.mouse_y) {
            let info = crate::drivers::battery::BATTERY.lock().get_info();
            let text = format!("Health: {}%  Cycles: {}", info.health, info.cycle_count);
            new_tooltip = Some((text, bat_x, TOP_BAR_HEIGHT as isize));
        }

        if self.tooltip != new_tooltip {
            if let Some((_, tx, ty)) = &self.tooltip { self.mark_dirty(Rect { x: *tx, y: *ty, width: 220, height: 25 }); }
            if let Some((_, tx, ty)) = &new_tooltip { self.mark_dirty(Rect { x: *tx, y: *ty, width: 250, height: 25 }); }
            self.tooltip = new_tooltip;
        };

        // Mark the cursor's starting position as dirty ONCE before processing packets
        let initial_cursor_rect = self.get_cursor_rect();

        let start_interaction_id = self.resize_win_id;
        let start_interaction_rect = start_interaction_id.and_then(|id| self.get_window_rect(id));

        if let Some(rect) = self.drag_rect {
            self.mark_dirty(rect);
        }

        // Poll all drivers and buffer events
        self.input.update(screen_width, screen_height);

        let mut mouse_moved = false;
        
        // Process buffered events
        let events = core::mem::take(&mut self.input.event_queue);
        for event in events {
            match event {
                InputEvent::MouseMove { x, y, dx, dy } => {
                    mouse_moved = true;
                    // Update "current" input state for other methods that might rely on it
                    // (though we try to use x,y from event)
                    self.input.mouse_x = x;
                    self.input.mouse_y = y;

                    if let Some(id) = self.resize_win_id {
                if let Some(win) = self.windows.iter_mut().find(|w| w.id == id) {

                    let min_width: isize = 80;
                    let min_height: isize = 40;

                    if self.resize_direction == ResizeDirection::Right || self.resize_direction == ResizeDirection::TopRight || self.resize_direction == ResizeDirection::BottomRight {
                        let new_width = (win.width as isize + dx).max(min_width);
                        win.width = new_width as usize;
                    }
                    if self.resize_direction == ResizeDirection::Bottom || self.resize_direction == ResizeDirection::BottomLeft || self.resize_direction == ResizeDirection::BottomRight {
                        let new_height = (win.height as isize + dy).max(min_height);
                        win.height = new_height as usize;
                    }
                    if self.resize_direction == ResizeDirection::Left || self.resize_direction == ResizeDirection::TopLeft || self.resize_direction == ResizeDirection::BottomLeft {
                        let new_width = win.width as isize - dx;
                        if new_width >= min_width { win.x += dx; win.width = new_width as usize; }
                    }
                    if self.resize_direction == ResizeDirection::Top || self.resize_direction == ResizeDirection::TopLeft || self.resize_direction == ResizeDirection::TopRight {
                        let new_height = win.height as isize - dy;
                        if new_height >= min_height { win.y += dy; win.height = new_height as usize; }
                    }
                }
            }
            // If dragging, update window position based on the new final mouse position
            else if let Some(id) = self.drag_win_id {
                if let Some(win) = self.windows.iter().find(|w| w.id == id) {
                    // Don't move window, update drag_rect instead
                    let new_x = self.input.mouse_x - self.drag_offset_x;
                            let new_y = self.input.mouse_y - self.drag_offset_y;
                    self.drag_rect = Some(Rect { x: new_x, y: new_y, width: win.width, height: win.height });
                }
            }
            // Volume Popup Interaction (Hover/Dragging)
            else if self.volume_popup_open {
                let width = screen_width as usize;
                let vol_x = width as isize - 450;
                let popup_rect = Rect { x: vol_x - 10, y: TOP_BAR_HEIGHT as isize, width: 80, height: 150 };
                
                if popup_rect.contains(x, y) {
                    self.mark_dirty(popup_rect);
                }

                if self.dragging_volume_slider && self.input.left_button_pressed {
                    let bar_y_start = TOP_BAR_HEIGHT as isize + 30;
                    let bar_height = 100;
                    let rel_y = (y - bar_y_start).clamp(0, bar_height as isize);
                    let new_percent = 100 - (rel_y * 100 / bar_height as isize) as u8;
                    crate::drivers::speaker::SPEAKER.lock().set_volume(new_percent);
                    self.mark_dirty(popup_rect);
                }
            }
                }
                InputEvent::Scroll { delta } => {
                if let Some(target_id) = self.click_target_id {
                    if let Some(idx) = self.windows.iter().position(|w| w.id == target_id) {
                        let width = self.windows[idx].width;
                        let height = self.windows[idx].height;
                        if let WindowContent::App(app) = &mut self.windows[idx].content {
                                    app.handle_event(&AppEvent::Scroll { delta: delta * 3, width, height });
                            if let Some(rect) = self.get_window_rect(target_id) {
                                self.mark_dirty(rect);
                            }
                        }
                    }
                }
            }
                InputEvent::MouseButton { button, down, x, y } => {
                    // Sync state for handle method
                    self.input.mouse_x = x;
                    self.input.mouse_y = y;
                    self.handle_mouse_button_event(button, down, screen_width, screen_height);
                }
                InputEvent::KeyPress { key } => {
                    // Check for Alt+Tab (simulated logic as we rely on get_char)
                    // In a real scenario, we'd check self.input.alt_pressed && key == '\t'
                    // For this refactor, we'll assume a specific key combo or if modifiers worked.
                    
                    // Temporary: using F1 as "Alt+Tab" for demonstration if modifiers aren't fully hooked up in driver
                    // Or strictly use modifiers if the driver supported it.
                    // Let's implement the logic assuming we can detect the "switch" intent.
                    
                    // Basic Window Switching Logic
                    if key == '\t' && self.input.alt_pressed {
                        if !self.task_switcher_open {
                            self.task_switcher_open = true;
                            self.task_switcher_index = 0; // Start at current
                            // Mark screen dirty to draw switcher overlay
                            self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize });
                        }

                        // Cycle to next window
                        if !self.windows.is_empty() {
                            self.task_switcher_index = (self.task_switcher_index + 1) % self.windows.len();
                        }
                    } else if (key == 'm' || key == 'M') && self.input.ctrl_pressed {
                        // Global Shortcut: Ctrl+M to toggle mute
                        crate::drivers::speaker::SPEAKER.lock().toggle_mute();
                        self.osd_timeout = 60;
                        if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                             let osd_w = 200; let osd_h = 60;
                             self.mark_dirty(Rect { x: (info.width as isize - osd_w)/2, y: (info.height as isize - osd_h)/2, width: osd_w as usize, height: osd_h as usize });
                             self.mark_dirty(Rect { x: 0, y: info.height as isize - 40, width: info.width, height: 40 });
                        }
                    } else if (key == '=' || key == '+') && self.input.ctrl_pressed {
                        // Global Shortcut: Ctrl+Plus to increase volume
                        crate::drivers::speaker::SPEAKER.lock().increment_volume(5);
                        self.osd_timeout = 60;
                        if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                             let osd_w = 200; let osd_h = 60;
                             self.mark_dirty(Rect { x: (info.width as isize - osd_w)/2, y: (info.height as isize - osd_h)/2, width: osd_w as usize, height: osd_h as usize });
                             self.mark_dirty(Rect { x: 0, y: info.height as isize - 40, width: info.width, height: 40 });
                        }
                    } else if (key == '-' || key == '_') && self.input.ctrl_pressed {
                        // Global Shortcut: Ctrl+Minus to decrease volume
                        crate::drivers::speaker::SPEAKER.lock().increment_volume(-5);
                        self.osd_timeout = 60;
                        if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                             let osd_w = 200; let osd_h = 60;
                             self.mark_dirty(Rect { x: (info.width as isize - osd_w)/2, y: (info.height as isize - osd_h)/2, width: osd_w as usize, height: osd_h as usize });
                             self.mark_dirty(Rect { x: 0, y: info.height as isize - 40, width: info.width, height: 40 });
                        }
                    } else if key == '\u{B0}' || key == '\u{B1}' || key == '\u{B2}' {
                        // Hardware Multimedia Keys (Commonly mapped in driver)
                        let mut speaker = crate::drivers::speaker::SPEAKER.lock();
                        match key {
                            '\u{B0}' => speaker.increment_volume(5),  // Vol Up
                            '\u{B1}' => speaker.increment_volume(-5), // Vol Down
                            '\u{B2}' => speaker.toggle_mute(),        // Mute
                            _ => {}
                        }
                        self.osd_timeout = 60;
                        if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                             let osd_w = 200; let osd_h = 60;
                             self.mark_dirty(Rect { x: (info.width as isize - osd_w)/2, y: (info.height as isize - osd_h)/2, width: osd_w as usize, height: osd_h as usize });
                             self.mark_dirty(Rect { x: 0, y: info.height as isize - 40, width: info.width, height: 40 });
                        }
                    } else if key == '\u{B3}' { // Brightness Down
                        crate::drivers::brightness::increment_master_brightness(-5);
                        self.brightness_osd_timeout = 60;
                        if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                             let osd_w = 200; let osd_h = 60;
                             self.mark_dirty(Rect { x: (info.width as isize - osd_w)/2, y: (info.height as isize - osd_h)/2, width: osd_w as usize, height: osd_h as usize });
                             // No taskbar update for brightness
                        }
                    } else if key == '\u{B4}' { // Brightness Up
                        crate::drivers::brightness::increment_master_brightness(5);
                        self.brightness_osd_timeout = 60;
                        if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                             let osd_w = 200; let osd_h = 60;
                             self.mark_dirty(Rect { x: (info.width as isize - osd_w)/2, y: (info.height as isize - osd_h)/2, width: osd_w as usize, height: osd_h as usize });
                             // No taskbar update for brightness
                        }
                    } else if (key == 'b' || key == 'B') && self.input.ctrl_pressed {
                        // Global Shortcut: Ctrl+B to toggle brightness OSD (or cycle modes)
                        self.brightness_osd_timeout = 60; // Just show OSD for now
                        if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                             let osd_w = 200; let osd_h = 60;
                             self.mark_dirty(Rect { x: (info.width as isize - osd_w)/2, y: (info.height as isize - osd_h)/2, width: osd_w as usize, height: osd_h as usize });
                        }
                    } else {
                        if self.task_switcher_open && !self.input.alt_pressed {
                            // Alt released, switch to selected window
                            self.task_switcher_open = false;
                            
                            if !self.windows.is_empty() {
                                // The Z-order is usually sorted back-to-front. 
                                // We want to pick the window at the visual index.
                                // Simple implementation: rotate z-order to bring selected to front
                                // Here we just pick based on the z-order list reversed (Top down)
                                if self.task_switcher_index < self.z_order.len() {
                                    // Find win_id at that index from the top
                                    let win_id = self.z_order[self.z_order.len() - 1 - self.task_switcher_index];
                                    
                                    // Move to top
                                    if let Some(pos) = self.z_order.iter().position(|&id| id == win_id) {
                                        self.z_order.remove(pos);
                                        self.z_order.push(win_id);
                                    }
                                    // Restore if minimized
                                    if let Some(win) = self.windows.iter_mut().find(|w| w.id == win_id) {
                                        win.minimized = false;
                                    }
                                }
                            }
                             self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize });
                        } else {
                            // Normal key handling
                             if let Some(&top_win_id) = self.z_order.last() {
                                if let Some(rect) = self.get_window_rect(top_win_id) {
                                    self.mark_dirty(rect);
                                }

                                if let Some(win) = self.windows.iter_mut().find(|w| w.id == top_win_id) {
                                    if let WindowContent::App(app) = &mut win.content {
                                        app.handle_event(&AppEvent::KeyPress { key });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Handle Alt release if not caught in KeyPress (e.g. if loop finished)
        if self.task_switcher_open && !self.input.alt_pressed {
             self.task_switcher_open = false;
             // Finalize switch logic handled above or here
             self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize });
        }

        // Mark the cursor's final position as dirty ONCE after processing packets
        if mouse_moved {
            self.mark_dirty(initial_cursor_rect);
            self.mark_dirty(self.get_cursor_rect());

            // Handle window dirty rects efficiently
            // 1. Mark the position where the window STARTED this frame (to clear it)
            if let Some(rect) = start_interaction_rect {
                self.mark_dirty(rect);
            }
            
            // 2. Mark the position where the initial window ENDED up (to draw it new)
            if let Some(id) = start_interaction_id {
                if let Some(rect) = self.get_window_rect(id) {
                    self.mark_dirty(rect);
                }
            }

            // 3. If we switched to dragging a DIFFERENT window mid-loop (rare), mark it too
            let end_interaction_id = self.resize_win_id;
            if end_interaction_id != start_interaction_id {
                if let Some(id) = end_interaction_id {
                    if let Some(rect) = self.get_window_rect(id) {
                        self.mark_dirty(rect);
                    }
                }
            }

            if let Some(rect) = self.drag_rect {
                self.mark_dirty(rect);
            }

            // Send MouseMove event to the active window if we are clicking/dragging on it
            if let Some(target_id) = self.click_target_id {
                if self.drag_win_id.is_none() && self.resize_win_id.is_none() {
                    if let Some(idx) = self.windows.iter().position(|w| w.id == target_id) {
                        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                        let title_height = font_height + 6;
                        let (win_x, win_y, win_w, win_h) = {
                            let w = &self.windows[idx];
                            (w.x, w.y, w.width, w.height)
                        };
                        
                        if let WindowContent::App(app) = &mut self.windows[idx].content {
                            let rel_x = self.input.mouse_x - win_x;
                            let rel_y = self.input.mouse_y - (win_y + title_height as isize);
                            if rel_x >= 0 && rel_x < win_w as isize && rel_y >= 0 && rel_y < win_h.saturating_sub(title_height) as isize {
                                app.handle_event(&AppEvent::MouseMove { x: rel_x, y: rel_y, width: win_w, height: win_h });
                            }
                        }
                    }
                }
            }
        }
      
        // Update cursor style based on what's under it
        self.update_cursor_style();

        if FULL_REDRAW_REQUESTED.load(Ordering::Relaxed) {
            FULL_REDRAW_REQUESTED.store(false, Ordering::Relaxed);
            if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                self.mark_dirty(Rect { x: 0, y: 0, width: info.width, height: info.height });
            }
        }

        if !self.dirty_rects.is_empty() {
            self.draw_dirty();
        }
    }

    fn update_cursor_style(&mut self) {
        let mut new_style = CursorStyle::Arrow;
        const BORDER_SIZE: isize = 5;

        // Check top-most window first for hover effect, but only if not currently resizing
        if self.resize_win_id.is_none() {
            if let Some(&top_win_id) = self.z_order.last() {
                if let Some(win) = self.windows.iter().find(|w| w.id == top_win_id) {
                    if !win.minimized && !win.maximized {
                        let in_x_body = self.input.mouse_x >= win.x && self.input.mouse_x < win.x + win.width as isize;
                        let in_y_body = self.input.mouse_y >= win.y && self.input.mouse_y < win.y + win.height as isize;

                        let on_left = self.input.mouse_x >= win.x - BORDER_SIZE && self.input.mouse_x < win.x + BORDER_SIZE;
                        let on_right = self.input.mouse_x >= win.x + win.width as isize - BORDER_SIZE && self.input.mouse_x < win.x + win.width as isize;
                        let on_top = self.input.mouse_y >= win.y - BORDER_SIZE && self.input.mouse_y < win.y + BORDER_SIZE;
                        let on_bottom = self.input.mouse_y >= win.y + win.height as isize - BORDER_SIZE && self.input.mouse_y < win.y + win.height as isize;

                        if (on_top && on_left) || (on_bottom && on_right) {
                            new_style = CursorStyle::ResizeNWSE;
                        } else if (on_top && on_right) || (on_bottom && on_left) {
                            new_style = CursorStyle::ResizeNESW;
                        } else if (on_left && in_y_body) || (on_right && in_y_body) {
                            new_style = CursorStyle::ResizeEW;
                        } else if (on_top && in_x_body) || (on_bottom && in_y_body) {
                            new_style = CursorStyle::ResizeNS;
                        }
                    }
                }
            }
        }
        
        if self.cursor_style != new_style {
            self.mark_dirty(self.get_cursor_rect());
            self.cursor_style = new_style;
            self.mark_dirty(self.get_cursor_rect());
        }
    }

    fn handle_mouse_button_event(&mut self, button: MouseButton, down: bool, screen_width: isize, screen_height: isize) {
        if let (MouseButton::Right, true) = (button, down) {
            let taskbar_height: usize = 40;
            let taskbar_y = screen_height - taskbar_height as isize;

            // Check if right click is on desktop (not on taskbar)
            if self.input.mouse_y < taskbar_y {
                 // Mark old menu dirty if open
                 if self.context_menu_open {
                     self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 });
                 }

                 self.context_menu_open = true;
                 self.context_menu_x = self.input.mouse_x;
                 self.context_menu_y = self.input.mouse_y;
                 self.start_menu_open = false; // Close start menu
                 
                 // Mark new menu dirty
                 self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 });
                 
                 // Also mark start menu dirty if it was open (handled by start_menu_open logic elsewhere? No, we just toggled flag)
                 // Ideally we should mark start menu area dirty if it was open. 
                 // For simplicity, we just mark context menu. Start menu closing redraw depends on subsequent updates.
            }
        }

        if let (MouseButton::Left, true) = (button, down) {
            let taskbar_height: usize = 40;
            let taskbar_y = screen_height - taskbar_height as isize;
            let start_button_width = 120;
            let locale_guard = localisation::CURRENT_LOCALE.lock();
            let locale = locale_guard.as_ref().unwrap();

            let width = screen_width as usize;
            let vol_x = width as isize - 450;
            let popup_rect = Rect { x: vol_x - 10, y: TOP_BAR_HEIGHT as isize, width: 80, height: 150 };

            // Close volume popup if clicking outside
            if self.volume_popup_open && !popup_rect.contains(self.input.mouse_x, self.input.mouse_y) && self.input.mouse_y >= TOP_BAR_HEIGHT as isize {
                self.volume_popup_open = false;
                self.mark_dirty(popup_rect);
                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
            } else if self.volume_popup_open {
                if popup_rect.contains(self.input.mouse_x, self.input.mouse_y) {
                    // Handle button click inside popup
                    let mute_btn_rect = Rect { x: vol_x - 5, y: TOP_BAR_HEIGHT as isize + 5, width: 70, height: 22 };
                    if mute_btn_rect.contains(self.input.mouse_x, self.input.mouse_y) {
                        crate::drivers::speaker::SPEAKER.lock().toggle_mute();
                        self.mark_dirty(popup_rect);
                        return;
                    }
                    
                    // Start dragging if clicking the slider area (below the mute button)
                    if self.input.mouse_y >= TOP_BAR_HEIGHT as isize + 30 {
                        self.dragging_volume_slider = true;
                        // Trigger first update immediately on click
                    }
                }
            }

            // 0. Check for clicks on the TOP BAR
            if self.input.mouse_y < TOP_BAR_HEIGHT as isize {
                // Nebula Menu Click (Updated width)
                if self.input.mouse_x < 80 {
                    self.start_menu_open = !self.start_menu_open;
                    self.volume_popup_open = false;
                    self.mark_dirty(Rect { x: 0, y: 0, width: 200, height: 400 });
                    return;
                }
                
                // Volume Area Click (Toggles the popup)
                if self.input.mouse_x >= vol_x - 40 && self.input.mouse_x < vol_x + 60 {
                    self.volume_popup_open = !self.volume_popup_open;
                    self.start_menu_open = false;
                    self.mark_dirty(popup_rect);
                    if !self.volume_popup_open {
                        FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                    }
                    return;
                }

                // Clock Click: Refresh Desktop
                let clock_x = width as isize - 160;
                if self.input.mouse_x >= clock_x {
                    FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                    return;
                }
            }

            let start_button = Button::new(0, taskbar_y, start_button_width, taskbar_height, locale.start());

            if self.context_menu_open {
                let menu_rect = Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 };
                if menu_rect.contains(self.input.mouse_x, self.input.mouse_y) {
                    let btn_refresh = Button::new(self.context_menu_x + 5, self.context_menu_y + 5, 140, 25, locale.ctx_refresh());
                    let btn_props = Button::new(self.context_menu_x + 5, self.context_menu_y + 35, 140, 25, locale.ctx_properties());
                    
                    if btn_refresh.contains(self.input.mouse_x, self.input.mouse_y) {
                        FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                    } else if btn_props.contains(self.input.mouse_x, self.input.mouse_y) {
                         let mut term = Terminal::new();
                         term.history.push("Name: Desktop".to_string());
                         term.history.push("Type: Workspace".to_string());
                         term.history.push("Location: /home/user/Desktop".to_string());
                         self.add_window(Window {
                            id: 0, x: 200, y: 200, width: 300, height: 200,
                            color: 0x00_1E_1E_1E,
                            title: locale.ctx_properties(),
                            content: WindowContent::App(Box::new(term)),
                            minimized: false, maximized: false, restore_rect: None,
                        });
                    }
                    self.context_menu_open = false;
                    self.mark_dirty(menu_rect);
                    return;
                } else {
                    self.context_menu_open = false;
                    self.mark_dirty(menu_rect);
                }
            }

            // 1. Check for click on start button
            if start_button.contains(self.input.mouse_x, self.input.mouse_y) {
                // Toggle Start Menu
                let was_open = self.start_menu_open;
                self.start_menu_open = !self.start_menu_open;
                self.drag_win_id = None;
                if was_open || self.start_menu_open {
                     let menu_height: usize = 345; // Increased slightly to fit items better
                     let menu_width: usize = 200;
                     self.mark_dirty(Rect { x: 0, y: taskbar_y - menu_height as isize, width: menu_width, height: menu_height });
                }
            } else if self.input.mouse_y >= taskbar_y && self.input.mouse_x >= start_button_width as isize {
                // Taskbar Window List Click
                // Start after the start button + padding
                let mut x_offset = start_button_width as isize + 10;
                let button_width = 100;
                let mut clicked_win_id = None;

                // First, find which window was clicked without holding a mutable borrow
                for win in &self.windows {
                    let button = Button::new(x_offset, taskbar_y + 2, button_width, taskbar_height - 4, "");
                    if button.contains(self.input.mouse_x, self.input.mouse_y) {
                        clicked_win_id = Some(win.id);
                        break;
                    }
                    x_offset += (button_width + 5) as isize;
                }

                // Now, act on the found window ID
                if let Some(win_id) = clicked_win_id {
                    let (is_minimized, is_top) = {
                        let win = self.windows.iter().find(|w| w.id == win_id).unwrap();
                        (win.minimized, self.z_order.last() == Some(&win_id))
                    };

                    if let Some(rect) = self.get_window_rect(win_id) {
                        self.mark_dirty(rect);
                    }

                    if is_minimized {
                        // Restore and bring to front
                        self.windows.iter_mut().find(|w| w.id == win_id).unwrap().minimized = false;
                        if let Some(pos) = self.z_order.iter().position(|&i| i == win_id) {
                            self.z_order.remove(pos);
                            self.z_order.push(win_id);
                        }
                    } else {
                        if is_top {
                            // Minimize
                            self.windows.iter_mut().find(|w| w.id == win_id).unwrap().minimized = true;
                        } else {
                            // Bring to front
                            if let Some(pos) = self.z_order.iter().position(|&i| i == win_id) {
                                self.z_order.remove(pos);
                                self.z_order.push(win_id);
                            }
                        }
                    }
                    self.mark_dirty(Rect { x: 0, y: taskbar_y, width: 800, height: 40 });
                }

            } else if self.start_menu_open && self.input.mouse_x < 200 && self.input.mouse_y >= TOP_BAR_HEIGHT as isize {
                // --- Nebula Menu Item Click Logic ---
                let menu_y = TOP_BAR_HEIGHT as isize;
                let menu_x = 0;
                let menu_width = 200;
                let item_width = menu_width - 20;

                let editor_button = Button::new(menu_x + 10, menu_y + 15, item_width, 30, locale.app_text_editor());
                let calc_button = Button::new(menu_x + 10, menu_y + 55, item_width, 30, locale.app_calculator());
                let paint_button = Button::new(menu_x + 10, menu_y + 95, item_width, 30, locale.app_paint());
                let settings_button = Button::new(menu_x + 10, menu_y + 135, item_width, 30, locale.app_settings());
                let terminal_button = Button::new(menu_x + 10, menu_y + 175, item_width, 30, locale.app_terminal());
                let taskmgr_button = Button::new(menu_x + 10, menu_y + 215, item_width, 30, "Task Manager");
                let shutdown_button = Button::new(menu_x + 10, menu_y + 345 - 45, item_width, 30, locale.btn_shutdown());
                let reboot_button = Button::new(menu_x + 10, menu_y + 345 - 85, item_width, 30, locale.btn_reboot());

                self.mark_dirty(Rect { x: 0, y: menu_y, width: 200, height: 345 });
                if settings_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    // Clicked "Settings"
                    let settings_open = self.windows.iter().any(|w| w.title == locale.app_settings());
                    if !settings_open {
                        self.add_window(Window {
                            id: 0, x: 250, y: 250, width: 300, height: 300,
                            color: 0x00_40_20_40, // Dark Purple
                            title: locale.app_settings(),
                            content: WindowContent::App(Box::new(Settings::new())),
                            minimized: false, maximized: false, restore_rect: None,
                        });
                    }
                }
                self.start_menu_open = false; // Close menu after click
                
                if taskmgr_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    // Clicked "Task Manager"
                    self.add_window(Window {
                        id: 0, x: 150, y: 150, width: 300, height: 400,
                        color: 0x00_00_40_40,
                        title: "Task Manager",
                        content: WindowContent::App(Box::new(TaskManager::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if terminal_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    // Clicked "Terminal"
                    self.add_window(Window {
                        id: 0, x: 100, y: 100, width: 480, height: 320,
                        color: 0x00_1E_1E_1E,
                        title: locale.app_terminal(),
                        content: WindowContent::App(Box::new(Terminal::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if shutdown_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    crate::kernel::power::shutdown();
                }

                if reboot_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    crate::kernel::power::reboot();
                }

                if calc_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    // Clicked "Calculator"
                    self.add_window(Window {
                        id: 0, x: 50, y: 350, width: 200, height: 220,
                        color: 0x00_20_20_20,
                        title: locale.app_calculator(),
                        content: WindowContent::App(Box::new(Calculator::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if paint_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    // Clicked "Paint"
                    self.add_window(Window {
                        id: 0, x: 180, y: 100, width: 400, height: 300,
                        color: 0x00_20_20_20,
                        title: locale.app_paint(),
                        content: WindowContent::App(Box::new(Paint::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if editor_button.contains(self.input.mouse_x, self.input.mouse_y) {
                    // Clicked "Text Editor"
                    self.add_window(Window {
                        id: 0, x: 150, y: 150, width: 400, height: 300,
                        color: 0x00_1E_1E_1E, // Very dark gray
                        title: locale.app_text_editor(),
                        content: WindowContent::App(Box::new(TextEditor::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }
            } else {
                // 2. Not a start button or menu click. Check for window interaction.
                let mut clicked_win_id = None;
                let mut resize_dir = ResizeDirection::None;
                const BORDER_SIZE: isize = 5;

                for &win_id in self.z_order.iter().rev() {
                    if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                        if win.minimized { continue; } // Skip minimized windows

                        let in_x_body = self.input.mouse_x >= win.x && self.input.mouse_x < win.x + win.width as isize;
                        let in_y_body = self.input.mouse_y >= win.y && self.input.mouse_y < win.y + win.height as isize;

                        // Check for resize handles if window is not maximized
                        if !win.maximized {
                            let on_left = self.input.mouse_x >= win.x - BORDER_SIZE && self.input.mouse_x < win.x + BORDER_SIZE;
                            let on_right = self.input.mouse_x >= win.x + win.width as isize - BORDER_SIZE && self.input.mouse_x < win.x + win.width as isize;
                            let on_top = self.input.mouse_y >= win.y - BORDER_SIZE && self.input.mouse_y < win.y + BORDER_SIZE;
                            let on_bottom = self.input.mouse_y >= win.y + win.height as isize - BORDER_SIZE && self.input.mouse_y < win.y + win.height as isize;

                            if on_top && on_left { resize_dir = ResizeDirection::TopLeft; }
                            else if on_top && on_right { resize_dir = ResizeDirection::TopRight; }
                            else if on_bottom && on_left { resize_dir = ResizeDirection::BottomLeft; }
                            else if on_bottom && on_right { resize_dir = ResizeDirection::BottomRight; }
                            else if on_left && in_y_body { resize_dir = ResizeDirection::Left; }
                            else if on_right && in_y_body { resize_dir = ResizeDirection::Right; }
                            else if on_top && in_x_body { resize_dir = ResizeDirection::Top; }
                            else if on_bottom && in_y_body { resize_dir = ResizeDirection::Bottom; }
                        }

                        if resize_dir != ResizeDirection::None || (in_x_body && in_y_body) {
                            clicked_win_id = Some(win_id);
                            break;
                        }
                    }
                }

                if let Some(win_id) = clicked_win_id {
                    self.click_target_id = Some(win_id);

                    if let Some(rect) = self.get_window_rect(win_id) {
                        self.mark_dirty(rect);
                    }

                    // Bring window to front of Z-order
                    if let Some(z_index) = self.z_order.iter().position(|&id| id == win_id) {
                        let id = self.z_order.remove(z_index);
                        self.z_order.push(id);
                    }

                    if resize_dir != ResizeDirection::None {
                        self.resize_win_id = Some(win_id);
                        self.resize_direction = resize_dir;
                        self.drag_win_id = None;
                    } else {
                        // Now that it's at the front, check for drag/close.
                        // Get window properties immutably first to avoid borrow conflicts.
                        let (win_x, win_y, win_width, win_maximized) = {
                            let win = self.windows.iter().find(|w| w.id == win_id).unwrap();
                            (win.x, win.y, win.width, win.maximized)
                        };

                        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                        let title_height = font_height + 6;
                        let close_button = Button::new(win_x + win_width as isize - 20, win_y + 3, 16, 16, "x");
                        let max_button = Button::new(win_x + win_width as isize - 40, win_y + 3, 16, 16, "+");
                        let min_button = Button::new(win_x + win_width as isize - 60, win_y + 3, 16, 16, "-");

                        if self.input.mouse_y < win_y + title_height as isize { // Clicked title bar
                            if close_button.contains(self.input.mouse_x, self.input.mouse_y) {
                                // Remove window
                                // The rect was already marked dirty when the window was clicked.
                                self.windows.retain(|w| w.id != win_id);
                                self.z_order.retain(|&id| id != win_id);
                                // Mark taskbar dirty to remove button
                                self.mark_dirty(Rect { x: 0, y: taskbar_y, width: 800, height: 40 });
                            } else if max_button.contains(self.input.mouse_x, self.input.mouse_y) {
                                // Maximize Toggle
                                let fb_info_w = FRAMEBUFFER.lock().info.as_ref().map(|i| i.width).unwrap_or(800);
                                let fb_info_h = FRAMEBUFFER.lock().info.as_ref().map(|i| i.height).unwrap_or(600);
                                let taskbar_h = 40;

                                let (old_rect, new_rect) = {
                                    let top_win = self.windows.iter_mut().find(|w| w.id == win_id).unwrap();
                                    let old_rect = Rect { x: top_win.x, y: top_win.y, width: top_win.width, height: top_win.height };
                                    if top_win.maximized {
                                        if let Some(rect) = top_win.restore_rect {
                                            top_win.x = rect.x; top_win.y = rect.y;
                                            top_win.width = rect.width; top_win.height = rect.height;
                                        }
                                        top_win.maximized = false;
                                    } else {
                                        top_win.restore_rect = Some(old_rect);
                                        top_win.x = 0; 
                                        top_win.y = TOP_BAR_HEIGHT as isize;
                                        top_win.width = fb_info_w;
                                        top_win.height = fb_info_h - taskbar_h - TOP_BAR_HEIGHT;
                                        top_win.maximized = true;
                                    }
                                    (old_rect, Rect { x: top_win.x, y: top_win.y, width: top_win.width, height: top_win.height })
                                };
                                self.mark_dirty(old_rect);
                                self.mark_dirty(new_rect);
                            } else if min_button.contains(self.input.mouse_x, self.input.mouse_y) {
                                // Minimize
                                let old_rect = {
                                    let top_win = self.windows.iter_mut().find(|w| w.id == win_id).unwrap();
                                    top_win.minimized = true;
                                    Rect { x: top_win.x, y: top_win.y, width: top_win.width, height: top_win.height }
                                };
                                self.mark_dirty(old_rect);
                                self.mark_dirty(Rect { x: 0, y: taskbar_y, width: 800, height: 40 }); // Update taskbar
                            } else if !win_maximized { // Start drag (only if not maximized)
                                let top_win = self.windows.iter().find(|w| w.id == win_id).unwrap();
                                self.drag_win_id = Some(win_id);
                                self.drag_offset_x = self.input.mouse_x - top_win.x;
                                self.drag_offset_y = self.input.mouse_y - top_win.y;
                                self.drag_rect = Some(Rect { x: top_win.x, y: top_win.y, width: top_win.width, height: top_win.height });
                            }
                        }
                    }
                } else {
                    // Clicked on desktop
                    self.click_target_id = None;
                    if self.start_menu_open {
                        let menu_height: usize = 345;
                        let menu_width: usize = 200;
                        self.mark_dirty(Rect { x: 0, y: taskbar_y - menu_height as isize, width: menu_width, height: menu_height });
                        self.start_menu_open = false;
                    }
                }
            }
        } else if let (MouseButton::Left, false) = (button, down) {

            if self.resize_win_id.is_some() {
                self.resize_win_id = None;
                self.resize_direction = ResizeDirection::None;
            } else if let Some(win_id) = self.drag_win_id {
                // Drag finished - Commit move
                if let Some(rect) = self.drag_rect {
                    if let Some(old_rect) = self.get_window_rect(win_id) {
                        self.mark_dirty(old_rect);
                    }
                    if let Some(win) = self.windows.iter_mut().find(|w| w.id == win_id) {
                        win.x = rect.x;
                        win.y = rect.y;
                    }
                    self.mark_dirty(rect);
                }
                self.drag_win_id = None;
                self.drag_rect = None;
            } else if self.dragging_volume_slider {
                self.dragging_volume_slider = false;
            } else { // Only process content click if not dragging
                if let Some(target_id) = self.click_target_id {
                    let mut event_coords = None;
                    if let Some(win) = self.windows.iter().find(|w| w.id == target_id) {
                        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                        let title_height = font_height + 6;
                        // Check if click is inside the window content area (below titlebar)
                        if self.input.mouse_x >= win.x && self.input.mouse_x < win.x + win.width as isize &&
                           self.input.mouse_y >= win.y + title_height as isize && self.input.mouse_y < win.y + win.height as isize {
                             event_coords = Some((self.input.mouse_x - win.x, self.input.mouse_y - (win.y + title_height as isize)));
                        }
                    }

                    if let Some((rel_x, rel_y)) = event_coords {
                        if let Some(rect) = self.get_window_rect(target_id) {
                            self.mark_dirty(rect);
                        }
                        if let Some(win) = self.windows.iter_mut().find(|w| w.id == target_id) {
                             if let WindowContent::App(app) = &mut win.content {
                                app.handle_event(&AppEvent::MouseClick { x: rel_x, y: rel_y, width: win.width, height: win.height });
                            }
                        }
                    }
                }
            }
            self.click_target_id = None;
        }
    }

    fn draw_dirty(&mut self) {
        if self.dirty_rects.is_empty() { return; }

        let raw_dirty_rects = core::mem::take(&mut self.dirty_rects);
        
        // Smart Merging: Only merge rectangles that actually intersect.
        // This prevents creating massive redraw areas for distant updates (preventing lag),
        // while combining overlapping updates like cursor movement (preventing flicker).
        let mut final_rects: Vec<Rect> = Vec::new();
        for r in raw_dirty_rects {
            let mut current = r;
            // Check against all currently accepted rects for overlaps
            let mut i = 0;
            while i < final_rects.len() {
                if current.intersects(&final_rects[i]) {
                    // If overlap, merge them and remove the old one from the list
                    let other = final_rects.remove(i);
                    current = current.union(&other);
                    // Restart scan to ensure we catch all transitive overlaps with the new larger rect
                    i = 0; 
                } else {
                    i += 1;
                }
            }
            final_rects.push(current);
        }

        // Acquire screen info to set up local buffer
        let fb_info = if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
            // Manually clone the info since FramebufferInfo doesn't derive Clone
            framebuffer::FramebufferInfo {
                address: info.address,
                width: info.width,
                height: info.height,
                pitch: info.pitch,
                bpp: info.bpp,
            }
        } else {
            return;
        };

        // Ensure local backbuffer is sized correctly
        let buffer_size = fb_info.width * fb_info.height;
        if self.backbuffer.len() != buffer_size {
            self.backbuffer.resize(buffer_size, 0);
        }

        // Create a temporary Framebuffer wrapping our local backbuffer.
        // This allows us to use existing draw functions without locking the global driver.
        let mut local_fb = framebuffer::Framebuffer::new();
        local_fb.info = Some(fb_info);
        // We temporarily move our buffer into the struct
        local_fb.draw_buffer = Some(core::mem::take(&mut self.backbuffer));

        // STEP 1: Draw everything to the local backbuffer.
        // No VRAM writes happen here, so no flickering can occur.
        for dirty_rect in &final_rects {
            // 1. Draw Desktop Background (Clear the region)
            // Extract info to avoid holding an immutable borrow on local_fb while calling set_pixel (mutable borrow)
            let (screen_width, screen_height) = if let Some(info) = &local_fb.info { (info.width, info.height) } else { (0, 0) };
            
            if screen_width > 0 {
                let screen_rect = Rect { x: 0, y: 0, width: screen_width, height: screen_height };
                if let Some(r) = dirty_rect.intersection(&screen_rect) {
                    let h = screen_height as isize;
                    for y in r.y..(r.y + r.height as isize) {
                        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
                        let (start_c, end_c) = if high_contrast {
                            (0x00_00_00_00, 0x00_00_00_00) // Solid black in high contrast
                        } else {
                            (DESKTOP_GRADIENT_START.load(Ordering::Relaxed), DESKTOP_GRADIENT_END.load(Ordering::Relaxed))
                        };

                        let (r1, g1, b1) = (((start_c >> 16) & 0xFF) as isize, ((start_c >> 8) & 0xFF) as isize, (start_c & 0xFF) as isize);
                        let (r2, g2, b2) = (((end_c >> 16) & 0xFF) as isize, ((end_c >> 8) & 0xFF) as isize, (end_c & 0xFF) as isize);

                        let r_val = r1 + ((r2 - r1) * y) / h;
                        let g_val = g1 + ((g2 - g1) * y) / h;
                        let b_val = b1 + ((b2 - b1) * y) / h;
                        
                        let color = ((r_val as u32) << 16) | ((g_val as u32) << 8) | (b_val as u32);
                        
                        // Optimized: Use draw_rect (slice::fill) instead of pixel-by-pixel loop
                        draw_rect(&mut local_fb, r.x, y, r.width, 1, color, Some(*dirty_rect));
                    }
                    
                    // Draw version string on desktop (bottom right, above taskbar)
                    let version = format!("NebulaOS {}", crate::kernel::VERSION);
                    let v_w = font::string_width(&version);
                    let v_x = screen_width as isize - v_w as isize - 8;
                    let v_y = screen_height as isize - 40 - 20; // 40 taskbar, 20 padding
                    
                    if dirty_rect.intersects(&Rect { x: v_x, y: v_y, width: v_w, height: 16 }) {
                        font::draw_string(&mut local_fb, v_x, v_y, &version, 0x00_FFFFFF, Some(*dirty_rect));
                    }
                }
            } else {
                draw_rect(&mut local_fb, dirty_rect.x, dirty_rect.y, dirty_rect.width, dirty_rect.height, 0x00_5A_9D_A5, Some(*dirty_rect));
            }

            // 2. Draw Windows
            for &win_id in &self.z_order {
                if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                    if win.minimized { continue; }
                    let win_rect = Rect { x: win.x, y: win.y, width: win.width, height: win.height };
                    if dirty_rect.intersects(&win_rect) {
                        self.draw_window(&mut local_fb, win, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
                    }
                }
            }

            // 3. Draw Top Bar (System Status & Nebula Menu) - Duplicate call removed
            self.draw_top_bar(&mut local_fb, *dirty_rect);

            // 4. Draw Taskbar (Window Management)
            self.draw_taskbar(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect);

            // 5. Draw Nebula Menu if open
            if self.start_menu_open {
                self.draw_start_menu(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
            }

            if self.context_menu_open {
                self.draw_context_menu(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
            }

            // Draw Task Switcher
            if self.task_switcher_open {
                self.draw_task_switcher(&mut local_fb, *dirty_rect);
            }

            // Draw Volume OSD
            if self.osd_timeout > 0 {
                self.draw_osd(&mut local_fb, *dirty_rect);
            }

            // Draw Brightness OSD
            if self.brightness_osd_timeout > 0 {
                self.draw_brightness_osd(&mut local_fb, *dirty_rect);
            }

            // 5.5 Draw Volume Popup
            if self.volume_popup_open {
                self.draw_volume_popup(&mut local_fb, *dirty_rect);
            }

            // Draw Drag Overlay
            if let Some(rect) = self.drag_rect {
                let border_color = 0x00_FF_FF_FF; // White outline
                draw_rect(&mut local_fb, rect.x, rect.y, rect.width, 2, border_color, Some(*dirty_rect)); // Top
                draw_rect(&mut local_fb, rect.x, rect.y + rect.height as isize - 2, rect.width, 2, border_color, Some(*dirty_rect)); // Bottom
                draw_rect(&mut local_fb, rect.x, rect.y, 2, rect.height, border_color, Some(*dirty_rect)); // Left
                draw_rect(&mut local_fb, rect.x + rect.width as isize - 2, rect.y, 2, rect.height, border_color, Some(*dirty_rect)); // Right
            }

            // 6. Draw Tooltip
            if let Some((ref text, tx, ty)) = self.tooltip {
                self.draw_tooltip(&mut local_fb, text.as_str(), tx, ty, *dirty_rect);
            }

            // 5. Draw Mouse Cursor
            self.draw_cursor(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
        }

        // STEP 2: Blit the local backbuffer to the actual Video Memory.
        // We lock the global framebuffer only for this step.
        let fb = FRAMEBUFFER.lock();
        
        // We can't use fb.present_rect() because that copies from fb.draw_buffer.
        // We need to copy from local_fb.draw_buffer to VRAM.
        if let (Some(ref info), Some(ref src_buffer)) = (&fb.info, &local_fb.draw_buffer) {
             let vram_ptr = info.address as *mut u8;
             let src_ptr = src_buffer.as_ptr() as *const u8;

             for dirty_rect in final_rects {
                // Clamp dimensions
                let x = dirty_rect.x.max(0) as usize;
                let y = dirty_rect.y.max(0) as usize;
                let width = dirty_rect.width.min(info.width.saturating_sub(x));
                let height = dirty_rect.height.min(info.height.saturating_sub(y));
                
                if width == 0 || height == 0 { continue; }

                let row_len_bytes = width * 4;
                for i in 0..height {
                    let cy = y + i;
                    let offset = (cy * info.width + x) * 4; // Source is linear
                    let dst_offset = cy * info.pitch + x * 4; // Dest uses pitch
                    unsafe {
                        core::ptr::copy_nonoverlapping(src_ptr.add(offset), vram_ptr.add(dst_offset), row_len_bytes);
                    }
                }
            }
        }

        // Restore buffer ownership to self for next frame
        if let Some(buf) = local_fb.draw_buffer.take() {
            self.backbuffer = buf;
        }
    }

    fn draw_window(&self, fb: &mut framebuffer::Framebuffer, win: &Window, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let is_active = self.z_order.last() == Some(&win.id);
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        
        let (title_color, body_color, border_bright, border_dark, title_text_color) = if high_contrast {
            (if is_active { 0x00_00_00_FF } else { 0x00_00_00_00 }, 0x00_00_00_00, 0x00_FF_FF_FF, 0x00_FF_FF_FF, 0x00_FF_FF_FF)
        } else {
            (if is_active { 0x00_00_40_80 } else { 0x00_40_40_40 }, win.color, 0x00_FF_FF_FF, 0x00_40_40_40, 0x00_FF_FF_FF)
        };
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 6;

        // Draw main window body
        draw_rect(fb, win.x, win.y, win.width, win.height, body_color, Some(clip));
        
        // Draw Content
        let content_rect = Rect {
            x: win.x,
            y: win.y + title_height as isize,
            width: win.width,
            height: win.height.saturating_sub(title_height),
        };
        
        let screen_rect = fb.info.as_ref().map(|i| Rect { x: 0, y: 0, width: i.width, height: i.height }).unwrap_or(Rect { x: 0, y: 0, width: 0, height: 0 });

        if let Some(r) = clip.intersection(&content_rect).and_then(|r| r.intersection(&screen_rect)) {
            fb.set_clip(r.x as usize, r.y as usize, r.width, r.height);
            if let WindowContent::App(app) = &win.content {
                app.draw(fb, win);
            }
            fb.clear_clip();
        }

        // Draw title bar on top
        draw_rect(fb, win.x, win.y, win.width, title_height, title_color, Some(clip));
        // Draw title text
        // Clip text to title bar area to prevent spillover (especially with Large Text)
        let title_rect = Rect { x: win.x, y: win.y, width: win.width, height: title_height };
        
        if let Some(title_clip) = clip.intersection(&title_rect) {
            // Vertically center text in title bar
            let text_y = win.y + (title_height as isize - font_height as isize) / 2;
            font::draw_string(fb, win.x + 6, text_y, win.title, title_text_color, Some(title_clip));
        }

        // Window control buttons
        let btn_y = win.y + 3;
        let mut close_button = Button::new(win.x + win.width as isize - 20, btn_y, 16, 16, "x");
        close_button.bg_color = 0x00_C0_40_40;
        close_button.text_color = 0x00_FFFFFF;
        close_button.draw(fb, mouse_x, mouse_y, Some(clip));

        let mut max_button = Button::new(win.x + win.width as isize - 40, btn_y, 16, 16, "+");
        max_button.bg_color = title_color;
        max_button.text_color = 0x00_FFFFFF;
        max_button.draw(fb, mouse_x, mouse_y, Some(clip));

        let mut min_button = Button::new(win.x + win.width as isize - 60, btn_y, 16, 16, "-");
        min_button.bg_color = title_color;
        min_button.text_color = 0x00_FFFFFF;
        min_button.draw(fb, mouse_x, mouse_y, Some(clip));

        // Draw Bevel Frame (On top of everything)
        draw_rect(fb, win.x, win.y, win.width, 1, border_bright, Some(clip)); // Top
        draw_rect(fb, win.x, win.y, 1, win.height, border_bright, Some(clip)); // Left
        draw_rect(fb, win.x + win.width as isize - 1, win.y, 1, win.height, border_dark, Some(clip)); // Right
        draw_rect(fb, win.x, win.y + win.height as isize - 1, win.width, 1, border_dark, Some(clip)); // Bottom
        draw_rect(fb, win.x, win.y + title_height as isize, win.width, 1, border_dark, Some(clip)); // Header separator
    }

    fn draw_taskbar(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let (width, height) = if let Some(ref info) = fb.info {
            (info.width, info.height)
        } else {
            return;
        };

            let taskbar_height = 40;
            let taskbar_y = (height - taskbar_height) as isize;
            let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
            
            let (bg_color, border_color) = if high_contrast {
                (0x00_00_00_00, 0x00_FF_FF_FF)
            } else {
                (0x00_28_28_28, 0x00_50_50_50)
            };
            
            // Dark sleek taskbar
            draw_rect(fb, 0, taskbar_y, width, taskbar_height, bg_color, Some(clip));
            draw_rect(fb, 0, taskbar_y, width, 1, border_color, Some(clip)); // Top highlight

            // Draw Window List
            let mut x_offset = 10;
            let button_width = 100;
            for win in &self.windows {
                // Truncate title to fit
                let title = if font::string_width(win.title) > button_width - 8 {
                    let mut current_width = 0;
                    let mut end_char_idx = 0;
                    for (i, c) in win.title.char_indices() {
                        let char_width = if c.is_ascii() { 8 } else { 16 };
                        if current_width + char_width > button_width - 16 { // -16 for "..."
                            break;
                        }
                        current_width += char_width;
                        end_char_idx = i + c.len_utf8();
                    }
                    alloc::format!("{}...", &win.title[..end_char_idx])
                } else {
                    win.title.to_string()
                };

                let mut button = Button::new(x_offset, taskbar_y + 2, button_width, taskbar_height - 4, title);
                button.bg_color = if win.minimized { 0x00_30_30_30 } else { 0x00_50_50_50 };
                button.text_color = 0x00_FF_FF_FF;
                button.draw(fb, mouse_x, mouse_y, Some(clip));
                x_offset += button_width as isize + 5;
            }
    }

    fn draw_top_bar(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        let (width, _) = if let Some(ref info) = fb.info {
            (info.width, info.height)
        } else {
            return;
        };

        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_1A_1A_1A };
        let border_color = if high_contrast { 0x00_FF_FF_FF } else { 0x00_33_33_33 };

        // Semi-transparent look (by being slightly lighter than black)
        draw_rect(fb, 0, 0, width, TOP_BAR_HEIGHT, bg_color, Some(clip));
        draw_rect(fb, 0, TOP_BAR_HEIGHT as isize - 1, width, 1, border_color, Some(clip));

        // Brand/OS Label
        font::draw_string(fb, 10, 5, "Nebula", 0x00_00_AA_FF, Some(clip));

        // Right-aligned status icons
        let clock_width = (19 * 8) + 10; // "YYYY-MM-DD HH:MM:SS" is 19 chars
        let bat_x = width as isize - 335;
        let vol_x = width as isize - 225;
        let clock_x = width as isize - clock_width as isize - 5;

        draw_battery_indicator(fb, bat_x, 7, clip);
        draw_volume_control(fb, vol_x, 7, clip);
        
        // Draw clock
        let time = CURRENT_DATETIME.lock();
        let datetime_s = format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", time.year, time.month, time.day, time.hour, time.minute, time.second);
        font::draw_string(fb, clock_x, 5, datetime_s.as_str(), 0x00_FFFFFF, Some(clip));
    }

    // New function to draw only the clock area on the taskbar
    fn draw_clock_on_taskbar(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        if let Some((width, height)) = fb.info.as_ref().map(|i| (i.width, i.height)) {
            let taskbar_height = 40;
            let taskbar_y = (height - taskbar_height) as isize;
            let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
            let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_28_28_28 };
            
            // Clear the previous time area with taskbar background color
            let time_area_width = (8 * 8) + 20; // "HH:MM:SS" is 8 chars * 8px width + padding
            let time_x_start = width as isize - time_area_width;
            draw_rect(fb, time_x_start, taskbar_y + 1, time_area_width as usize, taskbar_height - 1, bg_color, Some(clip));

            let time = CURRENT_DATETIME.lock(); // Read from global current time
            let mut time_str_bytes = [b' '; 8];
            time_str_bytes[0] = b'0' + (time.hour / 10);
            time_str_bytes[1] = b'0' + (time.hour % 10);
            time_str_bytes[2] = b':';
            time_str_bytes[3] = b'0' + (time.minute / 10);
            time_str_bytes[4] = b'0' + (time.minute % 10);
            time_str_bytes[5] = b':';
            time_str_bytes[6] = b'0' + (time.second / 10);
            time_str_bytes[7] = b'0' + (time.second % 10);
            let time_s = core::str::from_utf8(&time_str_bytes).unwrap_or("??:??:??");

            let time_x = width as isize - (8 * 8) - 10; // 8 chars * 8px width - 10px padding
            font::draw_string(fb, time_x, taskbar_y + 12, time_s, 0x00_FF_FF_FF, Some(clip)); // White text
        }
    }

    fn draw_start_menu(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Rect) {
        if fb.info.is_none() { return; }

            let menu_width = 200;
            let menu_height = 345;
            let menu_y = TOP_BAR_HEIGHT as isize; // Drop DOWN from top bar
            let menu_x = 0;
            let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
            let radius = 12isize;
            
            let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_C0_C0_C0 };
            let border_color = if high_contrast { 0x00_FF_FF_FF } else { 0x00_40_40_40 };

            // 1. Draw Filled Rounded Background
            // We draw three rectangles to form the core "plus" shape of the menu
            draw_rect(fb, menu_x, menu_y + radius, menu_width, (menu_height as isize - 2 * radius) as usize, bg_color, Some(clip));
            draw_rect(fb, menu_x + radius, menu_y, (menu_width as isize - 2 * radius) as usize, radius as usize, bg_color, Some(clip));
            draw_rect(fb, menu_x + radius, menu_y + menu_height as isize - radius, (menu_width as isize - 2 * radius) as usize, radius as usize, bg_color, Some(clip));

            // Fill the four corners by drawing concentric arcs using the draw_circle primitive
            for r in 0..radius {
                // Top-Left quadrant
                if let Some(c) = clip.intersection(&Rect { x: menu_x, y: menu_y, width: radius as usize, height: radius as usize }) {
                    fb.set_clip(c.x as usize, c.y as usize, c.width, c.height);
                    fb.draw_circle(menu_x + radius, menu_y + radius, r, bg_color);
                }
                // Top-Right quadrant
                if let Some(c) = clip.intersection(&Rect { x: menu_x + menu_width as isize - radius, y: menu_y, width: radius as usize, height: radius as usize }) {
                    fb.set_clip(c.x as usize, c.y as usize, c.width, c.height);
                    fb.draw_circle(menu_x + menu_width as isize - radius - 1, menu_y + radius, r, bg_color);
                }
                // Bottom-Left
                if let Some(c) = clip.intersection(&Rect { x: menu_x, y: menu_y + menu_height as isize - radius, width: radius as usize, height: radius as usize }) {
                    fb.set_clip(c.x as usize, c.y as usize, c.width, c.height);
                    fb.draw_circle(menu_x + radius, menu_y + menu_height as isize - radius - 1, r, bg_color);
                }
                // Bottom-Right
                if let Some(c) = clip.intersection(&Rect { x: menu_x + menu_width as isize - radius, y: menu_y + menu_height as isize - radius, width: radius as usize, height: radius as usize }) {
                    fb.set_clip(c.x as usize, c.y as usize, c.width, c.height);
                    fb.draw_circle(menu_x + menu_width as isize - radius - 1, menu_y + menu_height as isize - radius - 1, r, bg_color);
                }
            }
            fb.clear_clip();

            // 2. Draw Rounded Outline
            // Straight edges
            fb.draw_line(menu_x + radius, menu_y, menu_x + menu_width as isize - radius, menu_y, border_color);
            fb.draw_line(menu_x + radius, menu_y + menu_height as isize - 1, menu_x + menu_width as isize - radius, menu_y + menu_height as isize - 1, border_color);
            fb.draw_line(menu_x, menu_y + radius, menu_x, menu_y + menu_height as isize - radius, border_color);
            fb.draw_line(menu_x + menu_width as isize - 1, menu_y + radius, menu_x + menu_width as isize - 1, menu_y + menu_height as isize - radius, border_color);

            // Corner arcs (using the same quadrant-clipping technique)
            let corner_coords = [
                (menu_x + radius, menu_y + radius, menu_x, menu_y), // TL
                (menu_x + menu_width as isize - radius - 1, menu_y + radius, menu_x + menu_width as isize - radius, menu_y), // TR
                (menu_x + radius, menu_y + menu_height as isize - radius - 1, menu_x, menu_y + menu_height as isize - radius), // BL
                (menu_x + menu_width as isize - radius - 1, menu_y + menu_height as isize - radius - 1, menu_x + menu_width as isize - radius, menu_y + menu_height as isize - radius) // BR
            ];

            for (cx, cy, cl_x, cl_y) in corner_coords {
                if let Some(c) = clip.intersection(&Rect { x: cl_x, y: cl_y, width: radius as usize, height: radius as usize }) {
                    fb.set_clip(c.x as usize, c.y as usize, c.width, c.height);
                    fb.draw_circle(cx, cy, radius, border_color);
                }
            }
            fb.clear_clip();

            let locale_guard = localisation::CURRENT_LOCALE.lock();
            let locale = locale_guard.as_ref().unwrap();
            let item_width = menu_width - 20;
            Button::new(menu_x + 10, menu_y + 15, item_width, 30, locale.app_text_editor()).draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 55, item_width, 30, locale.app_calculator()).draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 95, item_width, 30, locale.app_paint()).draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 135, item_width, 30, locale.app_settings()).draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 175, item_width, 30, locale.app_terminal()).draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 215, item_width, 30, "Task Manager").draw(fb, mouse_x, mouse_y, Some(clip));

            let mut reboot_button = Button::new(menu_x + 10, menu_y + menu_height as isize - 85, item_width, 30, locale.btn_reboot());
            reboot_button.bg_color = 0x00_FF_A0_40; // Orange
            reboot_button.draw(fb, mouse_x, mouse_y, Some(clip));

            let mut shutdown_button = Button::new(menu_x + 10, menu_y + menu_height as isize - 45, item_width, 30, locale.btn_shutdown());
            shutdown_button.bg_color = 0x00_FF_60_60; // Light red
            shutdown_button.draw(fb, mouse_x, mouse_y, Some(clip));
    }

    fn draw_tooltip(&self, fb: &mut framebuffer::Framebuffer, text: &str, x: isize, y: isize, clip: Rect) {
        let w = font::string_width(text) + 10;
        let h = 20;
        let rect = Rect { x, y, width: w, height: h };
        
        if !clip.intersects(&rect) { return; }

        draw_rect(fb, x, y, w, h, 0x00_FFFFE1, Some(clip)); // Classic Tooltip Yellow
        draw_rect(fb, x, y, w, 1, 0x00_000000, Some(clip)); // Border
        draw_rect(fb, x, y, 1, h, 0x00_000000, Some(clip));
        draw_rect(fb, x + w as isize - 1, y, 1, h, 0x00_000000, Some(clip));
        draw_rect(fb, x, y + h as isize - 1, w, 1, 0x00_000000, Some(clip));

        font::draw_string(fb, x + 5, y + 2, text, 0x00_000000, Some(clip));
    }

    fn draw_context_menu(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let menu_x = self.context_menu_x;
        let menu_y = self.context_menu_y;
        let width = 150;
        let height = 70; // Reduced height since items removed
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        
        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_C0_C0_C0 };
        let light = if high_contrast { 0x00_FF_FF_FF } else { 0x00_FFFFFF };
        let dark = if high_contrast { 0x00_FF_FF_FF } else { 0x00_40_40_40 };

        draw_rect(fb, menu_x, menu_y, width, height, bg_color, Some(clip));
        draw_rect(fb, menu_x, menu_y, width, 1, light, Some(clip));
        draw_rect(fb, menu_x, menu_y, 1, height, light, Some(clip));
        draw_rect(fb, menu_x + width as isize - 1, menu_y, 1, height, dark, Some(clip));
        draw_rect(fb, menu_x, menu_y + height as isize - 1, width, 1, dark, Some(clip));

        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        let item_width = width - 10;
        Button::new(menu_x + 5, menu_y + 5, item_width, 25, locale.ctx_refresh()).draw(fb, mouse_x, mouse_y, Some(clip));
        Button::new(menu_x + 5, menu_y + 35, item_width, 25, locale.ctx_properties()).draw(fb, mouse_x, mouse_y, Some(clip));
    }

    fn draw_task_switcher(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        if let Some(info) = fb.info.as_ref() {
            let width = 400;
            let height = 100;
            let x = (info.width / 2) - (width / 2);
            let y = (info.height / 2) - (height / 2);

            // Draw background
            draw_rect(fb, x as isize, y as isize, width, height, 0x00_30_30_30, Some(clip));
            draw_rect(fb, x as isize, y as isize, width, 1, 0x00_FF_FF_FF, Some(clip));
            draw_rect(fb, x as isize, y as isize, 1, height, 0x00_FF_FF_FF, Some(clip));
            draw_rect(fb, x as isize + width as isize - 1, y as isize, 1, height, 0x00_00_00_00, Some(clip));
            draw_rect(fb, x as isize, y as isize + height as isize - 1, width, 1, 0x00_00_00_00, Some(clip));

            font::draw_string(fb, x as isize + 10, y as isize + 10, "Task Switcher", 0x00_FF_FF_FF, Some(clip));

            // Draw icons/list
            let start_x = x as isize + 20;
            let start_y = y as isize + 40;
            let icon_size = 40;
            let padding = 10;

            // We iterate z_order backwards (Top to Bottom) to show most recent first
            for (i, &win_id) in self.z_order.iter().rev().enumerate() {
                if i >= 6 { break; } // Limit to 6 items for now
                let win = self.windows.iter().find(|w| w.id == win_id).unwrap();
                
                let item_x = start_x + (i as isize * (icon_size + padding));
                let color = if i == self.task_switcher_index { 0x00_50_50_90 } else { 0x00_40_40_40 };
                
                draw_rect(fb, item_x, start_y, icon_size as usize, icon_size as usize, color, Some(clip));
                // Draw simple char as icon representation
                font::draw_char(fb, item_x + 12, start_y + 12, win.title.chars().next().unwrap_or('?'), 0x00_FFFFFF, Some(clip));
            }
        }
    }

    fn draw_osd(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        if let Some(info) = fb.info.as_ref() {
            let osd_w = 200;
            let osd_h = 60;
            let x = (info.width as isize - osd_w) / 2;
            let y = (info.height as isize - osd_h) / 2;
            let osd_rect = Rect { x, y, width: osd_w as usize, height: osd_h as usize };

            if !clip.intersects(&osd_rect) { return; }

            let speaker = crate::drivers::speaker::SPEAKER.lock();
            let vol = speaker.master_volume;
            let muted = speaker.muted;
            drop(speaker);

            let percent = 100 - (vol as u32 * 100 / 63);
            
            // Draw Background with border
            draw_rect(fb, x, y, osd_w as usize, osd_h as usize, 0x00_20_20_20, Some(clip));
            draw_rect(fb, x, y, osd_w as usize, 1, 0x00_FFFFFF, Some(clip));

            let label = if muted { "MUTE" } else { "VOLUME" };
            font::draw_string(fb, x + 10, y + 10, label, 0x00_FFFFFF, Some(clip));
            
            let bar_color = if muted { 0x00_44_44_44 } else { 0x00_00_FF_00 };
            draw_rect(fb, x + 10, y + 35, 180 * percent as usize / 100, 15, bar_color, Some(clip));
        }
    }

    fn draw_volume_popup(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        let width = fb.info.as_ref().map(|i| i.width).unwrap_or(800);
        let vol_x = width as isize - 450;
        let x = vol_x - 10;
        let y = TOP_BAR_HEIGHT as isize;
        let w = 80;
        let h = 150;
        let popup_rect = Rect { x, y, width: w, height: h };

        if !clip.intersects(&popup_rect) { return; }

        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_20_20_20 };
        let border_color = if high_contrast { 0x00_FF_FF_FF } else { 0x00_40_40_40 };

        draw_rect(fb, x, y, w, h, bg_color, Some(clip));
        // Border
        draw_rect(fb, x, y, w, 1, 0x00_FFFFFF, Some(clip)); // Top
        draw_rect(fb, x, y, 1, h, 0x00_FFFFFF, Some(clip)); // Left
        draw_rect(fb, x + w as isize - 1, y, 1, h, border_color, Some(clip)); // Right
        draw_rect(fb, x, y + h as isize - 1, w, 1, border_color, Some(clip)); // Bottom

        let speaker = crate::drivers::speaker::SPEAKER.lock();
        let vol = speaker.master_volume;
        let is_muted = speaker.muted;
        let percent = 100 - (vol as u32 * 100 / 63);
        drop(speaker);

        // Vertical Bar
        let bar_x = x + 30;
        let bar_y = y + 30;
        let bar_w = 20;
        let bar_h = 100;
        
        draw_rect(fb, bar_x, bar_y, bar_w, bar_h, 0x00_10_10_10, Some(clip)); // BG
        let fill_h = (bar_h * percent as usize) / 100;
        let color = if is_muted { 0x00_44_44_44 } else { 0x00_00_AA_00 };
        let fill_top_y = bar_y + (bar_h - fill_h) as isize;
        draw_rect(fb, bar_x, fill_top_y, bar_w, fill_h, color, Some(clip));

        // Draw Volume Handle (Slider Thumb)
        let handle_size: isize = 12;
        let handle_x = bar_x + (bar_w as isize / 2) - (handle_size / 2);
        let handle_y = fill_top_y - (handle_size / 2);
        
        let handle_rect = Rect { x: handle_x, y: handle_y, width: handle_size as usize, height: handle_size as usize };
        let is_hovering = handle_rect.contains(self.input.mouse_x, self.input.mouse_y);
        let color_main = if is_hovering { 0x00_FFFF00 } else { 0x00_FFFFFF };
        let color_inner = if is_hovering { 0x00_AAAA00 } else { 0x00_AAAAAA };

        draw_rect(fb, handle_x, handle_y, handle_size as usize, handle_size as usize, color_main, Some(clip));
        draw_rect(fb, handle_x + 2, handle_y + 2, (handle_size - 4) as usize, (handle_size - 4) as usize, color_inner, Some(clip));

        let mute_label = if is_muted { "Unmute" } else { "Mute" };
        let mut mute_btn = Button::new(x + 5, y + 5, 70, 22, mute_label);
        mute_btn.bg_color = if is_muted { 0x00_C0_40_40 } else { 0x00_60_60_60 };
        mute_btn.text_color = 0x00_FFFFFF;
        mute_btn.draw(fb, self.input.mouse_x, self.input.mouse_y, Some(clip));

        let p_str = format!("{}%", percent);
        font::draw_string(fb, x + 25, y + 135, p_str.as_str(), 0x00_CCCCCC, Some(clip));
    }

    fn draw_brightness_osd(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        if let Some(info) = fb.info.as_ref() {
            let osd_w = 200;
            let osd_h = 60;
            let x = (info.width as isize - osd_w) / 2;
            let y = (info.height as isize - osd_h) / 2;
            let osd_rect = Rect { x, y, width: osd_w as usize, height: osd_h as usize };

            if !clip.intersects(&osd_rect) { return; }

            let brightness_level = crate::drivers::brightness::BRIGHTNESS_LEVEL.load(Ordering::Relaxed);
            
            // Draw Background with border
            draw_rect(fb, x, y, osd_w as usize, osd_h as usize, 0x00_20_20_20, Some(clip));
            draw_rect(fb, x, y, osd_w as usize, 1, 0x00_FFFFFF, Some(clip));

            let label = "BRIGHTNESS";
            font::draw_string(fb, x + 10, y + 10, label, 0x00_FFFFFF, Some(clip));
            
            let bar_color = 0x00_FF_CC_00; // Orange/Yellow for brightness
            draw_rect(fb, x + 10, y + 35, 180 * brightness_level as usize / 100, 15, bar_color, Some(clip));
        }
    }

    fn draw_cursor(&self, fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
        let (width, height) = if let Some(ref info) = fb.info {
            (info.width as isize, info.height as isize)
        } else {
            return;
        };

            // Standard Arrow Cursor Bitmap (12x17)
            // 0 = Transparent, 1 = Black Border, 2 = White Fill
            let cursor_bitmap = match self.cursor_style {
                CursorStyle::Arrow => [
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
                ],
                CursorStyle::ResizeNS => [
                    [0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 2, 2, 2, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 2, 2, 2, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                ],
                CursorStyle::ResizeEW => [
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0],
                    [0, 1, 2, 1, 0, 0, 0, 0, 0, 1, 2, 1],
                    [1, 2, 2, 2, 1, 1, 1, 1, 1, 2, 2, 2],
                    [0, 1, 2, 1, 0, 0, 0, 0, 0, 1, 2, 1],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                ],
                CursorStyle::ResizeNWSE => [
                    [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 1, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 1],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                ],
                CursorStyle::ResizeNESW => [
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 1],
                    [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 1, 0],
                    [0, 0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0],
                    [0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0],
                    [0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0],
                    [0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
                    [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                ],
            };

            for (dy, row) in cursor_bitmap.iter().enumerate() {
                for (dx, &pixel) in row.iter().enumerate() {
                    if pixel == 0 { continue; }

                    let color = if pixel == 1 { 0x00_00_00_00 } else { 0x00_FF_FF_FF };
                    let px = x + dx as isize;
                    let py = y + dy as isize;

                    if clip.contains(px, py) && px >= 0 && py >= 0 && px < width && py < height {
                        fb.set_pixel(px as usize, py as usize, color);
                    }
                }
            }
    }
}

/// Draws a volume level indicator on the screen.
pub fn draw_volume_control(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
    let speaker = crate::drivers::speaker::SPEAKER.lock();
    let vol = speaker.master_volume;
    let is_muted = speaker.muted;
    drop(speaker);

    // Remap hardware attenuation (0-63, where 0 is max) to 0-100% fill
    let percent = 100 - (vol as u32 * 100 / 63);
    
    let width = 60;
    let height = 12;
    
    draw_rect(fb, x, y, width, height, 0x00_10_10_10, Some(clip)); // Background
    let fill_w = (width * percent as usize) / 100;

    if is_muted {
        draw_rect(fb, x, y, fill_w, height, 0x00_44_44_44, Some(clip)); // Gray Fill when muted
        font::draw_string(fb, x - 45, y - 2, "MUTE", 0x00_FF_00_00, Some(clip)); // Red Mute Icon
    } else {
        draw_rect(fb, x, y, fill_w, height, 0x00_00_AA_00, Some(clip)); // Green Fill
        font::draw_string(fb, x - 35, y - 2, "VOL", 0x00_FFFFFF, Some(clip));
    }
}

/// Draws a battery icon and percentage on the taskbar.
pub fn draw_battery_indicator(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
    let info = crate::drivers::battery::BATTERY.lock().get_info();
    
    let width = 30;
    let height = 12;
    
    // Draw Battery Body
    draw_rect(fb, x, y, width, height, 0x00_10_10_10, Some(clip)); // Outline/BG
    draw_rect(fb, x + width as isize, y + 3, 2, 6, 0x00_CCCCCC, Some(clip)); // Positive terminal nub

    // Determine color based on level
    let color = match info.percentage {
        0..=20 => 0x00_FF_00_00, // Red
        21..=50 => 0x00_FF_AA_00, // Orange
        _ => 0x00_00_AA_00,      // Green
    };

    let fill_w = (width * info.percentage as usize) / 100;
    draw_rect(fb, x, y, fill_w, height, color, Some(clip));

    // Draw charging indicator (simple '+')
    if info.state == crate::drivers::battery::BatteryState::Charging {
        font::draw_char(fb, x + 11, y - 2, '+', 0x00_FFFFFF, Some(clip));
    }

    // Only draw percentage on taskbar by default. Extended info is in tooltip.
    let s = format!("{}%", info.percentage);
    font::draw_string(fb, x + 40, y - 2, s.as_str(), 0x00_FFFFFF, Some(clip));
}

pub fn draw_rect(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, width: usize, height: usize, color: u32, clip: Option<Rect>) {    
    if let (Some(info), Some(buffer)) = (fb.info.as_ref(), fb.draw_buffer.as_mut()) {
        let screen_rect = Rect { x: 0, y: 0, width: info.width, height: info.height };
        let mut target_rect = Rect { x, y, width, height };

        if let Some(c) = clip {
            if let Some(clipped) = target_rect.intersection(&c) {
                target_rect = clipped;
            } else {
                return;
            }
        }

        if let Some(clipped) = target_rect.intersection(&screen_rect) {
            // Optimized: Calculate pointer offsets once and use a faster loop
            let start_x = clipped.x as usize;
            let start_y = clipped.y as usize;
            let end_y = start_y + clipped.height;
            
            for py in start_y..end_y {
                let offset = py * info.width + start_x;
                buffer[offset..offset + clipped.width].fill(color);
            }
        }
    }
}

pub static WINDOW_MANAGER: Mutex<WindowManager> = Mutex::new(WindowManager::new()); // Must be non-const because of Vec::new in InputManager? No, Vec::new is const.

pub fn init() {
    let mut wm = WINDOW_MANAGER.lock();
    
    // Initial draw
    if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
        wm.mark_dirty(Rect { x: 0, y: 0, width: info.width, height: info.height });
    }
    wm.draw_dirty();

}

pub fn update() {
    WINDOW_MANAGER.lock().update();
}