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
use crate::userspace::localisation::Localisation;

// Re-export fonts from their new location so existing references to gui::font work
pub use crate::userspace::fonts::font;

const MAX_WINDOWS: usize = 10;
pub const TOP_BAR_HEIGHT: usize = 32; // Slightly taller for better readability

pub static DESKTOP_GRADIENT_START: AtomicU32 = AtomicU32::new(0x00_0F_11_1A); // Deep Navy
pub static DESKTOP_GRADIENT_END: AtomicU32 = AtomicU32::new(0x00_24_3B_55);   // Nebula Blue
pub static FULL_REDRAW_REQUESTED: AtomicBool = AtomicBool::new(false);
pub static HIGH_CONTRAST: AtomicBool = AtomicBool::new(false);
pub static LARGE_TEXT: AtomicBool = AtomicBool::new(false);
pub static MOUSE_SENSITIVITY: AtomicU32 = AtomicU32::new(10); // 1.0x scale

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
    Hand,
    IBeam,
    Crosshair,
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
    pub fractional_x: isize,
    pub fractional_y: isize,
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
            fractional_x: 0,
            fractional_y: 0,
            event_queue: Vec::new(),
        }
    }

    pub fn update(&mut self, max_w: isize, max_h: isize) {
        self.event_queue.clear();

        while let Some(packet) = mouse::get_packet() {
            let sens = MOUSE_SENSITIVITY.load(Ordering::Relaxed) as isize;
            
            // Calculate total accumulated movement (current packet + previous remainder)
            let tx = (packet.x as isize * sens) + self.fractional_x;
            let ty = (-(packet.y as isize) * sens) + self.fractional_y;
            
            // Extract whole pixels to move
            let dx = tx / 10;
            let dy = ty / 10;
            
            // Store the remainder for the next packet
            self.fractional_x = tx % 10;
            self.fractional_y = ty % 10;
            
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
    gradient_cache: Vec<u32>,
    drag_rect: Option<Rect>,
    task_switcher_open: bool,
    task_switcher_index: usize,
    brightness_osd_timeout: usize,
    tooltip: Option<(alloc::string::String, isize, isize)>,
    menu_anim_progress: usize, // 0 to 100
    menu_anim_target: usize,   // 0 or 100
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
            gradient_cache: Vec::new(),
            drag_rect: None,
            task_switcher_open: false,
            task_switcher_index: 0,
            brightness_osd_timeout: 0,
            tooltip: None,
            menu_anim_progress: 0,
            menu_anim_target: 0,
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

    fn mark_dirty_outline(&mut self, rect: Rect) {
        const BW: usize = 4; // Border width to refresh (padded for the 2px outline)
        self.mark_dirty(Rect { x: rect.x - 2, y: rect.y - 2, width: rect.width + 4, height: BW }); // Top
        self.mark_dirty(Rect { x: rect.x - 2, y: rect.y + rect.height as isize - 2, width: rect.width + 4, height: BW }); // Bottom
        self.mark_dirty(Rect { x: rect.x - 2, y: rect.y - 2, width: BW, height: rect.height + 4 }); // Left
        self.mark_dirty(Rect { x: rect.x + rect.width as isize - 2, y: rect.y - 2, width: BW, height: rect.height + 4 }); // Right
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
                    // Mark the entire right status area dirty to accommodate potential layout shifts
                    let rect = Rect { x: info.width as isize - 250, y: 0, width: 250, height: TOP_BAR_HEIGHT };
                    self.mark_dirty(rect);
                }
            }
            drop(current_dt); // Release lock early
        }

        // Handle Start Menu animation (Slide effect)
        if self.menu_anim_progress != self.menu_anim_target {
            let menu_h = 345;
            let start_y = TOP_BAR_HEIGHT as isize;
            // Calculate position before update to track what needs cleaning
            let old_y = start_y - (menu_h as isize * (100 - self.menu_anim_progress) as isize / 100);

            if self.menu_anim_progress < self.menu_anim_target {
                self.menu_anim_progress = (self.menu_anim_progress + 10).min(self.menu_anim_target);
            } else {
                self.menu_anim_progress = self.menu_anim_progress.saturating_sub(10).max(self.menu_anim_target);
            }
            self.start_menu_open = self.menu_anim_progress > 0;

            let new_y = start_y - (menu_h as isize * (100 - self.menu_anim_progress) as isize / 100);
            let r1 = Rect { x: 0, y: old_y, width: 210, height: menu_h };
            let r2 = Rect { x: 0, y: new_y, width: 210, height: menu_h };
            self.mark_dirty(r1.union(&r2));
        }

        // Check for brightness changes (to trigger OSD popup)
        if crate::drivers::brightness::BRIGHTNESS_UPDATED.swap(false, Ordering::Relaxed) {
            self.brightness_osd_timeout = 60;
            if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                 let osd_w = 200; let osd_h = 60;
                 self.mark_dirty(Rect { 
                     x: (info.width as isize - osd_w)/2, 
                     y: (info.height as isize - osd_h)/2, 
                     width: osd_w as usize, height: osd_h as usize 
                 });
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

        // Notify non-minimized windows of system ticks for periodic state updates (like blinking cursors)
        let current_tick = crate::kernel::process::TICKS.load(Ordering::Relaxed);
        for i in 0..self.windows.len() {
            if !self.windows[i].minimized {
                // Snapshot window info to pass to the event handler safely
                let win_info = self.windows[i].clone();
                if let WindowContent::App(app) = &mut self.windows[i].content {
                    if let Some(dirty_rect) = app.handle_event(&AppEvent::Tick { tick_count: current_tick }, &win_info) {
                        self.mark_dirty(dirty_rect);
                    }
                }
            }
        }

        // Tooltip logic for Battery Indicator
        let mut new_tooltip = None;
        let top_bar_y = 0;

        let battery_info = crate::drivers::battery::BATTERY.lock().get_info();
        
        // Calculate dynamic hitboxes based on presence
        let mut current_hit_x = screen_width - 10;

        if battery_info.health > 0 {
            let bat_x = current_hit_x - 75;
            let bat_hit_rect = Rect { x: bat_x, y: top_bar_y, width: 75, height: TOP_BAR_HEIGHT as usize };
            if bat_hit_rect.contains(self.input.mouse_x, self.input.mouse_y) {
                let text = format!("Health: {}%  Cycles: {}", battery_info.health, battery_info.cycle_count);
                new_tooltip = Some((text, bat_x - 120, TOP_BAR_HEIGHT as isize));
            }
            current_hit_x -= 85; // Move next hitbox to the left of battery
        }

        let clock_x = current_hit_x - 70;
        let clock_hit_rect = Rect { x: clock_x, y: top_bar_y, width: 70, height: TOP_BAR_HEIGHT as usize };

        if new_tooltip.is_none() && clock_hit_rect.contains(self.input.mouse_x, self.input.mouse_y) {
            let time = CURRENT_DATETIME.lock();
            let text = format!("Today: {:04}/{:02}/{:02} (UTC)", time.year, time.month, time.day);
            new_tooltip = Some((text, clock_x - 120, TOP_BAR_HEIGHT as isize));
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
                self.mark_dirty_outline(rect);
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
                if let Some(win) = self.windows.iter().find(|w| w.id == id) {
                    let mut r = self.drag_rect.unwrap_or(win.rect());
                    let min_width: isize = 80;
                    let min_height: isize = 40;

                    if self.resize_direction == ResizeDirection::Right || self.resize_direction == ResizeDirection::TopRight || self.resize_direction == ResizeDirection::BottomRight {
                        r.width = (r.width as isize + dx).max(min_width) as usize;
                    }
                    if self.resize_direction == ResizeDirection::Bottom || self.resize_direction == ResizeDirection::BottomLeft || self.resize_direction == ResizeDirection::BottomRight {
                        r.height = (r.height as isize + dy).max(min_height) as usize;
                    }
                    if self.resize_direction == ResizeDirection::Left || self.resize_direction == ResizeDirection::TopLeft || self.resize_direction == ResizeDirection::BottomLeft {
                        let new_width = r.width as isize - dx;
                        if new_width >= min_width { r.x += dx; r.width = new_width as usize; }
                    }
                    if self.resize_direction == ResizeDirection::Top || self.resize_direction == ResizeDirection::TopLeft || self.resize_direction == ResizeDirection::TopRight {
                        let new_height = r.height as isize - dy;
                        if new_height >= min_height { r.y += dy; r.height = new_height as usize; }
                    }
                    self.drag_rect = Some(r);
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
                }
                InputEvent::Scroll { delta } => {
                if let Some(target_id) = self.click_target_id {
                    if let Some(idx) = self.windows.iter().position(|w| w.id == target_id) {
                        let win = self.windows[idx].clone();
                        if let WindowContent::App(app) = &mut self.windows[idx].content {
                            let dirty = app.handle_event(&AppEvent::Scroll { delta: delta * 3, width: win.width, height: win.height }, &win);
                            self.mark_dirty(dirty.unwrap_or_else(|| win.rect()));
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
                                let win = self.windows.iter().find(|w| w.id == top_win_id).unwrap().clone();
                                let win_mut = self.windows.iter_mut().find(|w| w.id == top_win_id).unwrap();
                                if let WindowContent::App(app) = &mut win_mut.content {
                                    let dirty = app.handle_event(&AppEvent::KeyPress { key }, &win);
                                    self.mark_dirty(dirty.unwrap_or_else(|| win.rect()));
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
            self.mark_dirty_outline(rect);
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
                        let win_snap = self.windows[idx].clone();
                        
                        if let WindowContent::App(app) = &mut self.windows[idx].content {
                            let rel_x = self.input.mouse_x - win_x;
                            let rel_y = self.input.mouse_y - (win_y + title_height as isize);
                            if rel_x >= 0 && rel_x < win_w as isize && rel_y >= 0 && rel_y < win_h.saturating_sub(title_height) as isize {
                                let dirty = app.handle_event(&AppEvent::MouseMove { x: rel_x, y: rel_y, width: win_w, height: win_h }, &win_snap);
                                if let Some(r) = dirty { self.mark_dirty(r); }
                            }
                        }
                    }
                }
            }
        }
      
        // Update cursor style based on what's under it
        self.update_cursor_style(screen_width, screen_height);

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

    fn update_cursor_style(&mut self, _screen_width: isize, screen_height: isize) {
        let mut new_style = CursorStyle::Arrow;

        // 1. Check for interactive UI elements (Hand Cursor)
        let is_over_top_bar = self.input.mouse_y < TOP_BAR_HEIGHT as isize;
        let is_over_start_menu = self.start_menu_open && self.input.mouse_x < 210 && self.input.mouse_y >= TOP_BAR_HEIGHT as isize;
        let is_over_taskbar = self.input.mouse_y >= screen_height - 40;

        if is_over_top_bar || is_over_start_menu || is_over_taskbar {
            new_style = CursorStyle::Hand;
        }

        // 2. Check Window buttons and resize borders
        if let Some(&top_win_id) = self.z_order.last() {
            if let Some(win) = self.windows.iter().find(|w| w.id == top_win_id) {
                if !win.minimized {
                    let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                    let title_height = font_height + 6;
                    
                    // Check if over window control buttons (Close, Max, Min)
                    if self.input.mouse_y >= win.y && self.input.mouse_y < win.y + title_height as isize {
                        if self.input.mouse_x >= win.x + win.width as isize - 70 {
                            new_style = CursorStyle::Hand;
                        }
                    }
                }
            }
        }

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
                        } else if in_x_body && in_y_body {
                            let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                            let title_height = font_height + 6;

                            // If inside window but not on border/buttons, check for text apps
                            if self.input.mouse_y >= win.y + title_height as isize {
                                let locale_lock = localisation::CURRENT_LOCALE.lock();
                                if let Some(locale) = locale_lock.as_ref() {
                                    if win.title == locale.app_terminal() || win.title == locale.app_text_editor() {
                                        new_style = CursorStyle::IBeam;
                                    } else if win.title == locale.app_paint() {
                                        let toolbar_height = 40;
                                        if self.input.mouse_y >= win.y + title_height as isize + toolbar_height {
                                            new_style = CursorStyle::Crosshair;
                                        }
                                    }
                                }
                            }
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
            
            // FIX: Clone the Arc and drop the lock immediately to prevent deadlocks
            // when calling into apps (like Settings) that also need the locale.
            let locale_arc = localisation::CURRENT_LOCALE.lock().clone();
            let locale = locale_arc.as_ref().expect("Locale not initialized");

            let width = screen_width as usize;

            // 0. Check for clicks on the TOP BAR
            if self.input.mouse_y < TOP_BAR_HEIGHT as isize {
                // Start Menu Click
                let btn_width = font::string_width(locale.start()) + 20;
                if self.input.mouse_x >= 2 && self.input.mouse_x < (btn_width as isize + 2) {
                    self.menu_anim_target = if self.menu_anim_target == 0 { 100 } else { 0 };
                    self.mark_dirty(Rect { x: 0, y: 0, width: 210, height: 400 });
                    return;
                }

                // Clock Click: Refresh Desktop
                let clock_x = width as isize - 160;
                if self.input.mouse_x >= clock_x {
                    FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                    return;
                }
            }

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

            // 1. Taskbar interactions
            if self.input.mouse_y >= taskbar_y && self.input.mouse_x >= start_button_width as isize {
                // Taskbar Window List Click
                // Start after the start button + padding
                let mut x_offset = 10;
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

            } else if self.menu_anim_progress == 100 && self.input.mouse_x < 200 && self.input.mouse_y >= TOP_BAR_HEIGHT as isize {
                // --- Nebula Menu Item Click Logic ---
                // Optimization: Use coordinate math to detect clicks instead of allocating 
                // multiple Button and String objects, which prevents system crashes/hangs.
                let menu_y = TOP_BAR_HEIGHT as isize;
                let rel_y = self.input.mouse_y - menu_y;
                let item_idx = if rel_y >= 15 && rel_y < 245 && (rel_y - 15) % 40 < 30 {
                    Some((rel_y - 15) / 40)
                } else { None };

                self.mark_dirty(Rect { x: 0, y: menu_y, width: 200, height: 350 });

                if item_idx == Some(3) { // Settings
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
                self.menu_anim_target = 0; // Trigger slide-up after click
                
                if item_idx == Some(5) { // Task Manager
                    self.add_window(Window {
                        id: 0, x: 150, y: 150, width: 300, height: 400,
                        color: 0x00_00_40_40,
                        title: "Task Manager",
                        content: WindowContent::App(Box::new(TaskManager::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if item_idx == Some(4) { // Terminal
                    self.add_window(Window {
                        id: 0, x: 100, y: 100, width: 480, height: 320,
                        color: 0x00_1E_1E_1E,
                        title: locale.app_terminal(),
                        content: WindowContent::App(Box::new(Terminal::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if rel_y >= 300 && rel_y < 330 { // Shutdown
                    crate::kernel::power::shutdown();
                }

                if rel_y >= 260 && rel_y < 290 { // Reboot
                    crate::kernel::power::reboot();
                }

                if item_idx == Some(1) { // Calculator
                    self.add_window(Window {
                        id: 0, x: 50, y: 350, width: 200, height: 220,
                        color: 0x00_20_20_20,
                        title: locale.app_calculator(),
                        content: WindowContent::App(Box::new(Calculator::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if item_idx == Some(2) { // Paint
                    self.add_window(Window {
                        id: 0, x: 180, y: 100, width: 400, height: 300,
                        color: 0x00_20_20_20,
                        title: locale.app_paint(),
                        content: WindowContent::App(Box::new(Paint::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if item_idx == Some(0) { // Text Editor
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
                        self.drag_rect = self.get_window_rect(win_id);
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
                    if self.menu_anim_target == 100 {
                        // Trigger slide-up animation when clicking desktop
                        self.menu_anim_target = 0;
                    }
                }
            }
        } else if let (MouseButton::Left, false) = (button, down) {

            if let Some(win_id) = self.resize_win_id {
                if let Some(rect) = self.drag_rect {
                    if let Some(old_rect) = self.get_window_rect(win_id) {
                        self.mark_dirty(old_rect);
                    }
                    if let Some(win) = self.windows.iter_mut().find(|w| w.id == win_id) {
                        win.x = rect.x; win.y = rect.y;
                        win.width = rect.width; win.height = rect.height;
                    }
                    self.mark_dirty(rect);
                }
                self.resize_win_id = None;
                self.resize_direction = ResizeDirection::None;
                self.drag_rect = None;
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
                        let win_snap = self.windows.iter().find(|w| w.id == target_id).unwrap().clone();
                        if let Some(win) = self.windows.iter_mut().find(|w| w.id == target_id) {
                             if let WindowContent::App(app) = &mut win.content {
                                let dirty = app.handle_event(&AppEvent::MouseClick { x: rel_x, y: rel_y, width: win.width, height: win.height }, &win_snap);
                                self.mark_dirty(dirty.unwrap_or_else(|| win_snap.rect()));
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

        // Performance Safety: Limit raw rects to process to prevent O(N^2) merge hangs
        let mut raw_dirty_rects = core::mem::take(&mut self.dirty_rects);
        if raw_dirty_rects.len() > 50 {
            raw_dirty_rects.truncate(50);
        }
        
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

        // Ensure gradient cache is initialized (Fixes crash if draw_dirty is called before update)
        if self.gradient_cache.len() != fb_info.height {
            self.gradient_cache.resize(fb_info.height, 0);
            let start_c = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
            let end_c = DESKTOP_GRADIENT_END.load(Ordering::Relaxed);
            for y in 0..fb_info.height {
                self.gradient_cache[y] = self.interpolate_gradient(start_c, end_c, y as isize, fb_info.height as isize);
            }
        }

        // Create a temporary Framebuffer wrapping our local backbuffer.
        // This allows us to use existing draw functions without locking the global driver.
        let mut local_fb = framebuffer::Framebuffer::new();
        local_fb.info = Some(fb_info);
        // We temporarily move our buffer into the struct
        local_fb.draw_buffer = Some(core::mem::take(&mut self.backbuffer));

        // STEP 1: Draw everything to the local backbuffer.
        // No VRAM writes happen here, so no flickering can occur.
        let (screen_width, screen_height) = if let Some(info) = &local_fb.info { (info.width as isize, info.height as isize) } else { (0, 0) };
        let screen_rect = Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize };

        // Performance optimization: Cache shared UI context once per frame 
        // to avoid redundant mutex locking and string formatting inside the hot dirty-rect loop.
        let locale_arc = localisation::CURRENT_LOCALE.lock().clone();
        let locale = locale_arc.as_ref();

        let clock_str = {
            let t = CURRENT_DATETIME.lock();
            format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second)
        };

        let top_bar_rect = Rect { x: 0, y: 0, width: screen_width as usize, height: TOP_BAR_HEIGHT };
        let taskbar_rect = Rect { x: 0, y: screen_height - 40, width: screen_width as usize, height: 40 };

        for dirty_rect in &final_rects {
            // Occlusion Culling: If a maximized window is on top, don't draw the background.
            let background_occluded = self.windows.iter()
                .rev()
                .any(|w| w.maximized && !w.minimized && w.rect().intersects(dirty_rect));

            if !background_occluded && dirty_rect.intersects(&screen_rect) {
                let r = dirty_rect.intersection(&screen_rect).unwrap();
                let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
                
                // Optimized Gradient: Pre-calculate colors per scanline
                for y in r.y..(r.y + r.height as isize) {
                    let color = if high_contrast { 0 } else { self.gradient_cache[y as usize] };
                    if let Some(buf) = local_fb.draw_buffer.as_mut() {
                        let offset = (y as usize * screen_width as usize) + r.x as usize;
                        buf[offset..offset + r.width].fill(color);
                    }
                }
            }

            // 2. Draw Windows with subtle shadows
            for &win_id in &self.z_order {
                if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                    if win.minimized { continue; }
                    let win_rect = win.rect();
                    
                    // Draw simple 1px shadow for "Advanced" look
                    if !win.maximized && dirty_rect.intersects(&Rect { x: win.x + 2, y: win.y + 2, width: win.width, height: win.height }) {
                        draw_rect(&mut local_fb, win.x + 2, win.y + 2, win.width, win.height, 0x00_101010, Some(*dirty_rect));
                    }

                    if dirty_rect.intersects(&win_rect) {
                        self.draw_window(&mut local_fb, win, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
                    }
                }
            }

            // 3. Top Bar (System Status & Nebula Menu) - Only draw if rect intersects the top region
            if dirty_rect.intersects(&top_bar_rect) {
                self.draw_top_bar_optimized(&mut local_fb, *dirty_rect, locale, &clock_str);
            }

            // 4. Taskbar (Window Management) - Only draw if rect intersects the bottom region
            if dirty_rect.intersects(&taskbar_rect) {
                self.draw_taskbar(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
            }

            // 5. Draw Nebula Menu if open
            if self.start_menu_open && dirty_rect.intersects(&Rect { x: 0, y: TOP_BAR_HEIGHT as isize, width: 210, height: 350 }) {
                self.draw_start_menu_optimized(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect, locale);
            }

            if self.context_menu_open {
                self.draw_context_menu_optimized(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect, locale);
            }

            // Draw Task Switcher
            if self.task_switcher_open {
                self.draw_task_switcher(&mut local_fb, *dirty_rect);
            }

            // Draw Brightness OSD
            if self.brightness_osd_timeout > 0 {
                self.draw_brightness_osd(&mut local_fb, *dirty_rect);
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


        // STEP 2: Offload to Framebuffer Driver
        // We copy our local drawing to the driver's draw_buffer and use deferred presentation
        // to fix lag caused by blocking on slow VRAM hardware.
        let mut fb = FRAMEBUFFER.lock();
        
        let fb_info = fb.info;
        if let (Some(info), Some(src_buf)) = (fb_info, local_fb.draw_buffer.as_ref()) {
            // Step 2a: Copy pixels to the driver's backbuffer. 
            // We scope the mutable borrow of dest_buf so it's released before we call present_rect.
            if let Some(dest_buf) = fb.draw_buffer.as_mut() {
                for dirty_rect in &final_rects {
                    let x = dirty_rect.x.max(0) as usize;
                    let y = dirty_rect.y.max(0) as usize;
                    let width = dirty_rect.width.min(info.width.saturating_sub(x));
                    let height = dirty_rect.height.min(info.height.saturating_sub(y));

                    if width == 0 || height == 0 { continue; }

                    for i in 0..height {
                        let offset = (y + i) * info.width + x;
                        dest_buf[offset..offset + width].copy_from_slice(&src_buf[offset..offset + width]);
                    }
                }
            }

            // Step 2b: Inform the driver that these regions are ready to be blitted to VRAM.
            for dirty_rect in final_rects {
                let x = dirty_rect.x.max(0) as usize;
                let y = dirty_rect.y.max(0) as usize;
                let width = dirty_rect.width.min(info.width.saturating_sub(x));
                let height = dirty_rect.height.min(info.height.saturating_sub(y));

                if width > 0 && height > 0 {
                    fb.present_rect(x, y, width, height);
                }
            }
        }
        // Restore buffer ownership to self for next frame
        if let Some(buf) = local_fb.draw_buffer.take() {
            self.backbuffer = buf;
        }
    }

    fn interpolate_gradient(&self, start: u32, end: u32, y: isize, height: isize) -> u32 {
        let (r1, g1, b1) = (((start >> 16) & 0xFF) as isize, ((start >> 8) & 0xFF) as isize, (start & 0xFF) as isize);
        let (r2, g2, b2) = (((end >> 16) & 0xFF) as isize, ((end >> 8) & 0xFF) as isize, (end & 0xFF) as isize);
        let rv = r1 + ((r2 - r1) * y) / height;
        let gv = g1 + ((g2 - g1) * y) / height;
        let bv = b1 + ((b2 - b1) * y) / height;
        ((rv as u32) << 16) | ((gv as u32) << 8) | (bv as u32)
    }

    fn draw_win_btn(&self, fb: &mut framebuffer::Framebuffer, x: isize, y: isize, text: &str, color: u32, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let rect = Rect { x, y, width: 16, height: 16 };
        if !clip.intersects(&rect) { return; }
        let is_hovered = rect.contains(mouse_x, mouse_y);
        let bg = if is_hovered { 0x00_00_50_A0 } else { color };
        draw_rect(fb, x, y, 16, 16, bg, Some(clip));
        font::draw_string(fb, x + 4, y + 1, text, 0x00_FFFFFF, Some(clip));
    }

    fn draw_window(&self, fb: &mut framebuffer::Framebuffer, win: &Window, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let is_active = self.z_order.last() == Some(&win.id);
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        
        let (title_color, body_color, border_color, title_text_color) = if high_contrast {
            (if is_active { 0x00_00_00_FF } else { 0x00_00_00_00 }, 0x00_00_00_00, 0x00_FF_FF_FF, 0x00_FF_FF_FF)
        } else {
            (if is_active { 0x00_1E_1E_1E } else { 0x00_25_25_26 }, win.color, 0x00_3F_3F_46, 0x00_D0_D0_D0)
        };
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;

        // Draw main window body with rounded corners
        draw_rounded_rect(fb, win.x, win.y, win.width, win.height, 8, body_color, Some(clip));
        
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
                app.draw(fb, win, r);
            }
            fb.clear_clip();
        }

        // Draw title bar with rounded top corners
        draw_rounded_rect(fb, win.x, win.y, win.width, title_height, 8, title_color, Some(clip));
        // Cover the bottom rounding of the title bar to make it flush with the body
        draw_rect(fb, win.x, win.y + (title_height / 2) as isize, win.width, title_height / 2, title_color, Some(clip));

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
        // Optimized button drawing to avoid expensive heap allocations for simple window controls
        self.draw_win_btn(fb, win.x + win.width as isize - 20, btn_y, "x", 0x00_C0_40_40, mouse_x, mouse_y, clip);
        self.draw_win_btn(fb, win.x + win.width as isize - 40, btn_y, "+", title_color, mouse_x, mouse_y, clip);
        self.draw_win_btn(fb, win.x + win.width as isize - 60, btn_y, "-", title_color, mouse_x, mouse_y, clip);

        // Draw Border
        draw_rounded_rect(fb, win.x, win.y, win.width, win.height, 8, border_color, Some(clip));
        // Redraw body over border to keep only 1px edge
        draw_rounded_rect(fb, win.x + 1, win.y + 1, win.width - 2, win.height - 2, 7, body_color, Some(clip));
    }

    fn draw_taskbar(&self, fb: &mut framebuffer::Framebuffer, _mouse_x: isize, _mouse_y: isize, clip: Rect) {
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
                (0x00_12_12_12, 0x00_2A_2A_2A)
            };
            
            // Dark sleek taskbar
            draw_rect(fb, 0, taskbar_y, width, taskbar_height, bg_color, Some(clip));
            draw_rect(fb, 0, taskbar_y, width, 1, border_color, Some(clip)); // Top highlight

            // Draw Window List
            let mut x_offset = 10;
            let button_width = 100;
            for win in &self.windows {
                if x_offset + button_width as isize > width as isize - 10 { break; } // Prevent off-screen spill
                
                // Truncate title to fit
                let title = if font::string_width(win.title) > button_width - 8 {
                    "Window..." // Static fallback for extreme speed
                } else {
                    win.title
                };

                let bg = if win.minimized { 0x00_30_30_30 } else { 0x00_50_50_50 };
                draw_rect(fb, x_offset, taskbar_y + 2, button_width, taskbar_height - 4, bg, Some(clip));
                font::draw_string(fb, x_offset + 4, taskbar_y + 12, title, 0x00_FFFFFF, Some(clip));
                
                x_offset += button_width as isize + 5;
            }
    }

    fn draw_top_bar_optimized(&self, fb: &mut framebuffer::Framebuffer, clip: Rect, locale: Option<&alloc::sync::Arc<dyn Localisation>>, clock_text: &str) {
        let width = if let Some(ref info) = fb.info { info.width } else { return };

        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_1A_1A_1A };
        let border_color = if high_contrast { 0x00_FF_FF_FF } else { 0x00_33_33_33 };

        // Semi-transparent look (by being slightly lighter than black)
        draw_rect(fb, 0, 0, width, TOP_BAR_HEIGHT, bg_color, Some(clip));
        draw_rect(fb, 0, TOP_BAR_HEIGHT as isize - 1, width, 1, border_color, Some(clip));

        // Start Menu Button
        if let Some(l) = locale {
            let start_text = l.start();
            let btn_width = font::string_width(start_text) + 20;
            let mut start_btn = Button::new(2, 2, btn_width, TOP_BAR_HEIGHT - 4, start_text);
            if !high_contrast { start_btn.bg_color = 0x00_44_44_44; } 
            start_btn.text_color = 0x00_FFFFFF;
            start_btn.draw(fb, self.input.mouse_x, self.input.mouse_y, Some(clip));
        }

        // Right side status icons - Dynamic Positioning
        let battery_info = crate::drivers::battery::BATTERY.lock().get_info();
        let clock_text_width = font::string_width(clock_text) + 10;
        
        let mut cursor_x = width as isize - 10;

        if battery_info.health > 0 {
            cursor_x -= 75; // Battery indicator width
            draw_battery_indicator(fb, cursor_x, 7, clip);
            cursor_x -= 10; // Spacing between battery and clock
        }

        cursor_x -= clock_text_width as isize;
        font::draw_string(fb, cursor_x, 5, clock_text, 0x00_FFFFFF, Some(clip));
    }

    fn draw_start_menu_optimized(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, mut clip: Rect, locale: Option<&alloc::sync::Arc<dyn Localisation>>) {
        if fb.info.is_none() { return; }

        // Ensure the menu is clipped to appear "under" the top bar
        let menu_visible_area = Rect { x: 0, y: TOP_BAR_HEIGHT as isize, width: 210, height: 1000 };
        if let Some(c) = clip.intersection(&menu_visible_area) {
            clip = c;
        } else { return; }

        let menu_width = 200;
        let menu_height = 345;
        
        // Calculate sliding Y position
        let target_y = TOP_BAR_HEIGHT as isize;
        let menu_y = target_y - (menu_height as isize * (100 - self.menu_anim_progress) as isize / 100);
        let menu_x = 0;
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);

        // Fading effect: Interpolate background color based on progress
        let target_bg = if high_contrast { 0x00_00_00_00 } else { 0x00_1E_1E_1E };
        let bg_color = if high_contrast {
            target_bg
        } else {
            let desktop_bg = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
            self.interpolate_gradient(desktop_bg, target_bg, self.menu_anim_progress as isize, 100)
        };

        // 1. Draw Background (Flat design, no "effects")
        draw_rect(fb, menu_x, menu_y, menu_width, menu_height, bg_color, Some(clip));

        // 2. Outlines removed to simplify UI as requested
        
        if let Some(l) = locale {
            let item_width = menu_width - 20;
            let labels = [
                (l.app_text_editor(), 15, 0x00_C0_C0_C0),
                (l.app_calculator(), 55, 0x00_C0_C0_C0),
                (l.app_paint(), 95, 0x00_C0_C0_C0),
                (l.app_settings(), 135, 0x00_C0_C0_C0),
                (l.app_terminal(), 175, 0x00_C0_C0_C0),
                ("Task Manager", 215, 0x00_C0_C0_C0),
                (l.btn_reboot(), menu_height as isize - 85, 0x00_FF_A0_40),
                (l.btn_shutdown(), menu_height as isize - 45, 0x00_FF_60_60),
            ];

            let light = 0x00_E0_E0_E0;
            let shadow = 0x00_40_40_40;

            for (text, y_off, color) in labels {
                let item_x = menu_x + 10;
                let item_y = menu_y + y_off;
                let is_hovered = mouse_x >= item_x && mouse_x < item_x + item_width as isize && 
                                 mouse_y >= item_y && mouse_y < item_y + 30;

                // Dirty-rect optimization: Skip drawing this item if it doesn't intersect 
                // the region being refreshed.
                if !clip.intersects(&Rect { x: item_x, y: item_y, width: item_width, height: 30 }) { continue; }

                let bg = if is_hovered { 0x00_D0_D0_D0 } else { color };

                draw_rect(fb, item_x, item_y, item_width, 30, bg, Some(clip));
                // Draw Bevel
                draw_rect(fb, item_x, item_y, item_width, 1, light, Some(clip)); // Top
                draw_rect(fb, item_x, item_y, 1, 30, light, Some(clip)); // Left
                draw_rect(fb, item_x + item_width as isize - 1, item_y, 1, 30, shadow, Some(clip)); // Right
                draw_rect(fb, item_x, item_y + 29, item_width, 1, shadow, Some(clip)); // Bottom

                font::draw_string(fb, item_x + 10, item_y + 7, text, 0x00_000000, Some(clip));
            }
        }
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

    fn draw_context_menu_optimized(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Rect, locale: Option<&alloc::sync::Arc<dyn Localisation>>) {
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

        if let Some(l) = locale {
            let item_width = width - 10;
            Button::new(menu_x + 5, menu_y + 5, item_width, 25, l.ctx_refresh()).draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 5, menu_y + 35, item_width, 25, l.ctx_properties()).draw(fb, mouse_x, mouse_y, Some(clip));
        }
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

        let (mut draw_x, mut draw_y) = (x, y);
        if self.cursor_style == CursorStyle::Crosshair {
            // Center the crosshair on the mouse coordinates
            draw_x -= 6;
            draw_y -= 8;
        }

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
                CursorStyle::Hand => [
                    [0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 2, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 2, 2, 1, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 2, 2, 1, 0, 1, 1, 1, 0, 0],
                    [0, 0, 1, 2, 2, 1, 1, 2, 2, 2, 1, 0],
                    [0, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 1],
                    [1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1],
                    [1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1],
                    [1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1],
                    [1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1],
                    [1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1],
                    [0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0],
                    [0, 0, 1, 2, 2, 2, 2, 2, 2, 1, 0, 0],
                    [0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                ],
                CursorStyle::IBeam => [
                    [1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                ],
                CursorStyle::Crosshair => [
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1],
                    [1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
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
                    let px = draw_x + dx as isize;
                    let py = draw_y + dy as isize;

                    if clip.contains(px, py) && px >= 0 && py >= 0 && px < width && py < height {
                        fb.set_pixel(px as usize, py as usize, color);
                    }
                }
            }
    }
}

/// Draws a battery icon and percentage on the taskbar.
pub fn draw_battery_indicator(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
    let info = crate::drivers::battery::BATTERY.lock().get_info();
    if info.health == 0 { return; }
    
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
            
            // Final boundary check to ensure we don't index outside the physical framebuffer dimensions
            if end_y > info.height || start_x + clipped.width > info.width {
                return;
            }

            for py in start_y..end_y {
                let offset = py * info.width + start_x;
                if let Some(line) = buffer.get_mut(offset..offset + clipped.width) {
                    line.fill(color);
                }
            }
        }
    }
}

/// Draws a rectangle with rounded corners using quadrant clipping.
pub fn draw_rounded_rect(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, width: usize, height: usize, radius: isize, color: u32, clip: Option<Rect>) {
    if radius <= 0 {
        draw_rect(fb, x, y, width, height, color, clip);
        return;
    }

    let clip_rect = clip.unwrap_or(Rect { x: -10000, y: -10000, width: 20000, height: 20000 });

    // Core body (plus-shape optimization to avoid corner overlap)
    draw_rect(fb, x, y + radius, width, (height as isize - 2 * radius) as usize, color, Some(clip_rect));
    draw_rect(fb, x + radius, y, (width as isize - 2 * radius) as usize, radius as usize, color, Some(clip_rect));
    draw_rect(fb, x + radius, y + height as isize - radius, (width as isize - 2 * radius) as usize, radius as usize, color, Some(clip_rect));

    // Optimized Scanline Corner Filling
    let r2 = radius * radius;
    let centers = [
        (x + radius, y + radius, -1, -1), // TL
        (x + width as isize - radius - 1, y + radius, 1, -1), // TR
        (x + radius, y + height as isize - radius - 1, -1, 1), // BL
        (x + width as isize - radius - 1, y + height as isize - radius - 1, 1, 1), // BR
    ];

    for (cx, cy, dx_sign, dy_sign) in centers {
        for dy in 0..radius {
            let mut max_dx = 0;
            for dx in 0..radius {
                if dx * dx + dy * dy <= r2 { max_dx = dx; } else { break; }
            }
            let py = cy + dy * dy_sign;
            let sx = if dx_sign < 0 { cx - max_dx } else { cx };
            draw_rect(fb, sx, py, max_dx as usize + 1, 1, color, Some(clip_rect));
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