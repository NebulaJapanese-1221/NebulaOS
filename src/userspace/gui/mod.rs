//! GUI components for NebulaOS.
//! NOTE: This module is currently being refactored to support a graphical framebuffer
//! instead of the old text-mode VGA buffer. The drawing logic is temporarily disabled.

use crate::drivers::mouse::{self, MousePacket};
use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use crate::drivers::rtc::{self, CURRENT_DATETIME, TIME_NEEDS_UPDATE};
use crate::drivers::keyboard;
use alloc::vec::Vec;
use alloc::string::ToString;
use crate::userspace::apps::{app::{App, AppEvent}, calculator::Calculator, editor::TextEditor, settings::Settings, terminal::Terminal};
use spin::Mutex;
use alloc::boxed::Box;
pub mod font;
use core::sync::atomic::Ordering;
pub mod rect;
use self::rect::Rect;
pub mod button;
use self::button::Button;

const MAX_WINDOWS: usize = 10;

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

pub struct WindowManager {
    windows: Vec<Window>,
    z_order: Vec<usize>,
    next_win_id: usize,
    mouse_x: isize,
    mouse_y: isize,
    drag_win_id: Option<usize>,
    drag_offset_x: isize,
    drag_offset_y: isize,
    left_button_pressed: bool,
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
}

impl WindowManager {
    pub const fn new() -> Self {
        WindowManager {
            windows: Vec::new(),
            z_order: Vec::new(),
            next_win_id: 0,
            mouse_x: 40,
            mouse_y: 12,
            drag_win_id: None,
            drag_offset_x: 0,
            drag_offset_y: 0,
            left_button_pressed: false,
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
        Rect { x: self.mouse_x, y: self.mouse_y, width: 12, height: 17 }
    }

    fn get_window_rect(&self, win_id: usize) -> Option<Rect> {
        self.windows.iter().find(|w| w.id == win_id).map(|w| Rect {
            x: w.x, y: w.y, width: w.width, height: w.height
        })
    }

    pub fn update(&mut self) {
        // The old redraw logic is replaced by dirty rect tracking.
        // We no longer use boolean flags to trigger a full redraw.

        // Check if time needs update
        if TIME_NEEDS_UPDATE.load(Ordering::Relaxed) {
            TIME_NEEDS_UPDATE.store(false, Ordering::Relaxed);
            // Read RTC and update global time
            let new_time = rtc::read_time();
            let mut current_dt = CURRENT_DATETIME.lock();
            if *current_dt != new_time { // Only redraw if time actually changed
                *current_dt = new_time;
                if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
                    let taskbar_height: usize = 40;
                    let time_area_width: isize = (8 * 8) + 20;
                    let rect = Rect { x: info.width as isize - time_area_width, y: info.height as isize - taskbar_height as isize, width: time_area_width as usize, height: taskbar_height };
                    self.mark_dirty(rect);
                }
            }
            drop(current_dt); // Release lock early
        }

        // Cache screen dimensions to avoid locking Framebuffer in the loop
        let (screen_width, screen_height) = if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
            (info.width as isize, info.height as isize)
        } else {
            (800, 600) // Fallback
        };

        // Mark the cursor's starting position as dirty ONCE before processing packets
        self.mark_dirty(self.get_cursor_rect());

        let start_interaction_id = self.resize_win_id.or(self.drag_win_id);
        let start_interaction_rect = start_interaction_id.and_then(|id| self.get_window_rect(id));

        let mut mouse_moved = false;

        while let Some(packet) = mouse::get_packet() {
            mouse_moved = true;
            let old_mouse_x = self.mouse_x;
            let old_mouse_y = self.mouse_y;

            self.mouse_x += packet.x as isize;
            self.mouse_y -= packet.y as isize; // PS/2 Y-axis is often inverted

            // Clamp final position to screen bounds. This prevents the "bounce" effect
            // by ensuring the cursor stops cleanly at the edge.
            self.mouse_x = self.mouse_x.clamp(0, screen_width - 1);
            self.mouse_y = self.mouse_y.clamp(0, screen_height - 1);

            // Process clicks/drags immediately to maintain synchronization with position
            self.handle_mouse_buttons(packet, screen_height);

            let dx = self.mouse_x - old_mouse_x;
            let dy = self.mouse_y - old_mouse_y;

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
                if let Some(idx) = self.windows.iter().position(|w| w.id == id) {
                    // Update position without marking dirty inside the loop
                    let win = &mut self.windows[idx];
                    win.x = self.mouse_x - self.drag_offset_x;
                    win.y = self.mouse_y - self.drag_offset_y;
                }
            }
        }
        
        // Mark the cursor's final position as dirty ONCE after processing packets
        if mouse_moved {
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
            let end_interaction_id = self.resize_win_id.or(self.drag_win_id);
            if end_interaction_id != start_interaction_id {
                if let Some(id) = end_interaction_id {
                    if let Some(rect) = self.get_window_rect(id) {
                        self.mark_dirty(rect);
                    }
                }
            }
        }

        // Update cursor style based on what's under it
        self.update_cursor_style();

        // Check for keyboard input and pass to focused window
        while let Some(key) = keyboard::get_char() {
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
                        let in_x_body = self.mouse_x >= win.x && self.mouse_x < win.x + win.width as isize;
                        let in_y_body = self.mouse_y >= win.y && self.mouse_y < win.y + win.height as isize;

                        let on_left = self.mouse_x >= win.x - BORDER_SIZE && self.mouse_x < win.x + BORDER_SIZE;
                        let on_right = self.mouse_x >= win.x + win.width as isize - BORDER_SIZE && self.mouse_x < win.x + win.width as isize;
                        let on_top = self.mouse_y >= win.y - BORDER_SIZE && self.mouse_y < win.y + BORDER_SIZE;
                        let on_bottom = self.mouse_y >= win.y + win.height as isize - BORDER_SIZE && self.mouse_y < win.y + win.height as isize;

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

    fn handle_mouse_buttons(&mut self, packet: MousePacket, screen_height: isize) {
        let left_down = (packet.buttons & 0x1) != 0;
        let right_down = (packet.buttons & 0x2) != 0;

        if right_down {
            let taskbar_height: usize = 40;
            let taskbar_y = screen_height - taskbar_height as isize;

            // Check if right click is on desktop (not on taskbar)
            if self.mouse_y < taskbar_y {
                 // Mark old menu dirty if open
                 if self.context_menu_open {
                     self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 });
                 }

                 self.context_menu_open = true;
                 self.context_menu_x = self.mouse_x;
                 self.context_menu_y = self.mouse_y;
                 self.start_menu_open = false; // Close start menu
                 
                 // Mark new menu dirty
                 self.mark_dirty(Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 });
                 
                 // Also mark start menu dirty if it was open (handled by start_menu_open logic elsewhere? No, we just toggled flag)
                 // Ideally we should mark start menu area dirty if it was open. 
                 // For simplicity, we just mark context menu. Start menu closing redraw depends on subsequent updates.
            }
        }

        if left_down && !self.left_button_pressed {
            // Click started
            self.left_button_pressed = true;
            let taskbar_height: usize = 40;
            let taskbar_y = screen_height - taskbar_height as isize;
            let start_button_width = 120;
            let start_button = Button::new(0, taskbar_y, start_button_width, taskbar_height, "Start");

            if self.context_menu_open {
                let menu_rect = Rect { x: self.context_menu_x, y: self.context_menu_y, width: 150, height: 100 };
                if menu_rect.contains(self.mouse_x, self.mouse_y) {
                    let btn1 = Button::new(self.context_menu_x + 5, self.context_menu_y + 5, 140, 25, "New Terminal");
                    let btn2 = Button::new(self.context_menu_x + 5, self.context_menu_y + 35, 140, 25, "System Info");
                    let btn3 = Button::new(self.context_menu_x + 5, self.context_menu_y + 65, 140, 25, "Properties");
                    
                    if btn1.contains(self.mouse_x, self.mouse_y) {
                        self.add_window(Window {
                            id: 0, x: 100, y: 100, width: 480, height: 320,
                            color: 0x00_1E_1E_1E,
                            title: "Terminal",
                            content: WindowContent::App(Box::new(Terminal::new())),
                            minimized: false, maximized: false, restore_rect: None,
                        });
                    } else if btn2.contains(self.mouse_x, self.mouse_y) {
                         let mut term = Terminal::new();
                         term.history.push("> uname -a".to_string());
                         term.history.push(alloc::format!("NebulaOS {} i686 NebulaFS", crate::kernel::VERSION));
                         self.add_window(Window {
                            id: 0, x: 150, y: 150, width: 480, height: 320,
                            color: 0x00_1E_1E_1E,
                            title: "System Info",
                            content: WindowContent::App(Box::new(term)),
                            minimized: false, maximized: false, restore_rect: None,
                        });
                    } else if btn3.contains(self.mouse_x, self.mouse_y) {
                         let mut term = Terminal::new();
                         term.history.push("Name: Desktop".to_string());
                         term.history.push("Type: Workspace".to_string());
                         term.history.push("Location: /home/user/Desktop".to_string());
                         self.add_window(Window {
                            id: 0, x: 200, y: 200, width: 300, height: 200,
                            color: 0x00_1E_1E_1E,
                            title: "Properties",
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
            if start_button.contains(self.mouse_x, self.mouse_y) {
                // Toggle Start Menu
                let was_open = self.start_menu_open;
                self.start_menu_open = !self.start_menu_open;
                self.drag_win_id = None;
                if was_open || self.start_menu_open {
                     let menu_height: usize = 300;
                     let menu_width: usize = 200;
                     self.mark_dirty(Rect { x: 0, y: taskbar_y - menu_height as isize, width: menu_width, height: menu_height });
                }
            } else if self.mouse_y >= taskbar_y && self.mouse_x >= start_button_width as isize {
                // Taskbar Window List Click
                // Start after the start button + padding
                let mut x_offset = start_button_width as isize + 10;
                let button_width = 100;
                let mut clicked_win_id = None;

                // First, find which window was clicked without holding a mutable borrow
                for win in &self.windows {
                    let button = Button::new(x_offset, taskbar_y + 2, button_width, taskbar_height - 4, "");
                    if button.contains(self.mouse_x, self.mouse_y) {
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

            } else if self.start_menu_open && self.mouse_x < 200 && self.mouse_y < taskbar_y{
                // --- Start Menu Item Click Logic ---
                let menu_y = screen_height - 40 - 300;
                let menu_x = 0;
                let menu_width = 200;
                let item_width = menu_width - 20;

                let editor_button = Button::new(menu_x + 10, menu_y + 15, item_width, 30, "Text Editor");
                let calc_button = Button::new(menu_x + 10, menu_y + 55, item_width, 30, "Calculator");
                let settings_button = Button::new(menu_x + 10, menu_y + 95, item_width, 30, "Settings");
                let terminal_button = Button::new(menu_x + 10, menu_y + 135, item_width, 30, "Terminal");
                let shutdown_button = Button::new(menu_x + 10, menu_y + 300 - 45, item_width, 30, "Shutdown");

                self.mark_dirty(Rect { x: 0, y: menu_y, width: 200, height: 300 }); // Mark menu dirty on click
                if settings_button.contains(self.mouse_x, self.mouse_y) {
                    // Clicked "Settings"
                    let settings_open = self.windows.iter().any(|w| w.title == "Settings");
                    if !settings_open {
                        self.add_window(Window {
                            id: 0, x: 250, y: 250, width: 200, height: 150,
                            color: 0x00_40_20_40, // Dark Purple
                            title: "Settings",
                            content: WindowContent::App(Box::new(Settings::new())),
                            minimized: false, maximized: false, restore_rect: None,
                        });
                    }
                }
                self.start_menu_open = false; // Close menu after click
                
                if terminal_button.contains(self.mouse_x, self.mouse_y) {
                    // Clicked "Terminal"
                    self.add_window(Window {
                        id: 0, x: 100, y: 100, width: 480, height: 320,
                        color: 0x00_1E_1E_1E,
                        title: "Terminal",
                        content: WindowContent::App(Box::new(Terminal::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if shutdown_button.contains(self.mouse_x, self.mouse_y) {
                    crate::kernel::power::shutdown();
                }

                if calc_button.contains(self.mouse_x, self.mouse_y) {
                    // Clicked "Calculator"
                    self.add_window(Window {
                        id: 0, x: 50, y: 350, width: 160, height: 220,
                        color: 0x00_20_20_20,
                        title: "Calculator",
                        content: WindowContent::App(Box::new(Calculator::new())),
                        minimized: false, maximized: false, restore_rect: None,
                    });
                }

                if editor_button.contains(self.mouse_x, self.mouse_y) {
                    // Clicked "Text Editor"
                    self.add_window(Window {
                        id: 0, x: 150, y: 150, width: 400, height: 300,
                        color: 0x00_1E_1E_1E, // Very dark gray
                        title: "Text Editor",
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

                        let in_x_body = self.mouse_x >= win.x && self.mouse_x < win.x + win.width as isize;
                        let in_y_body = self.mouse_y >= win.y && self.mouse_y < win.y + win.height as isize;

                        // Check for resize handles if window is not maximized
                        if !win.maximized {
                            let on_left = self.mouse_x >= win.x - BORDER_SIZE && self.mouse_x < win.x + BORDER_SIZE;
                            let on_right = self.mouse_x >= win.x + win.width as isize - BORDER_SIZE && self.mouse_x < win.x + win.width as isize;
                            let on_top = self.mouse_y >= win.y - BORDER_SIZE && self.mouse_y < win.y + BORDER_SIZE;
                            let on_bottom = self.mouse_y >= win.y + win.height as isize - BORDER_SIZE && self.mouse_y < win.y + win.height as isize;

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

                        let close_button = Button::new(win_x + win_width as isize - 20, win_y + 2, 16, 16, "x");
                        let max_button = Button::new(win_x + win_width as isize - 40, win_y + 2, 16, 16, "+");
                        let min_button = Button::new(win_x + win_width as isize - 60, win_y + 2, 16, 16, "-");

                        if self.mouse_y < win_y + 20 { // Clicked title bar
                            if close_button.contains(self.mouse_x, self.mouse_y) {
                                // Remove window
                                // The rect was already marked dirty when the window was clicked.
                                self.windows.retain(|w| w.id != win_id);
                                self.z_order.retain(|&id| id != win_id);
                                // Mark taskbar dirty to remove button
                                self.mark_dirty(Rect { x: 0, y: taskbar_y, width: 800, height: 40 });
                            } else if max_button.contains(self.mouse_x, self.mouse_y) {
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
                                        top_win.x = 0; top_win.y = 0;
                                        top_win.width = fb_info_w;
                                        top_win.height = fb_info_h - taskbar_h;
                                        top_win.maximized = true;
                                    }
                                    (old_rect, Rect { x: top_win.x, y: top_win.y, width: top_win.width, height: top_win.height })
                                };
                                self.mark_dirty(old_rect);
                                self.mark_dirty(new_rect);
                            } else if min_button.contains(self.mouse_x, self.mouse_y) {
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
                                self.drag_offset_x = self.mouse_x - top_win.x;
                                self.drag_offset_y = self.mouse_y - top_win.y;
                            }
                        }
                    }
                } else {
                    // Clicked on desktop
                    self.click_target_id = None;
                    if self.start_menu_open {
                        let menu_height: usize = 300;
                        let menu_width: usize = 200;
                        self.mark_dirty(Rect { x: 0, y: taskbar_y - menu_height as isize, width: menu_width, height: menu_height });
                        self.start_menu_open = false;
                    }
                }
            }
        } else if !left_down && self.left_button_pressed {
            // Click released
            self.left_button_pressed = false;

            if self.resize_win_id.is_some() {
                self.resize_win_id = None;
                self.resize_direction = ResizeDirection::None;
            } else if self.drag_win_id.is_none() { // Only process content click if not dragging
                if let Some(target_id) = self.click_target_id {
                    let mut event_coords = None;
                    if let Some(win) = self.windows.iter().find(|w| w.id == target_id) {
                        // Check if click is inside the window content area (below titlebar)
                        if self.mouse_x >= win.x && self.mouse_x < win.x + win.width as isize &&
                           self.mouse_y >= win.y + 20 && self.mouse_y < win.y + win.height as isize {
                             event_coords = Some((self.mouse_x - win.x, self.mouse_y - (win.y + 20)));
                        }
                    }

                    if let Some((rel_x, rel_y)) = event_coords {
                        if let Some(rect) = self.get_window_rect(target_id) {
                            self.mark_dirty(rect);
                        }
                        if let Some(win) = self.windows.iter_mut().find(|w| w.id == target_id) {
                             if let WindowContent::App(app) = &mut win.content {
                                app.handle_event(&AppEvent::MouseClick { x: rel_x, y: rel_y });
                            }
                        }
                    }
                }
            }
            self.drag_win_id = None;
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
            if let Some(fb_info) = local_fb.info.as_ref() {
                let screen_rect = Rect { x: 0, y: 0, width: fb_info.width, height: fb_info.height };
                if let Some(r) = dirty_rect.intersection(&screen_rect) {
                    let h = fb_info.height as isize;
                    for y in r.y..(r.y + r.height as isize) {
                        // Interpolate from Deep Blue (0x10, 0x20, 0x40) to Lighter Blue (0x50, 0x80, 0xB0)
                        let r_val = 0x10 + ((0x50 - 0x10) * y) / h;
                        let g_val = 0x20 + ((0x80 - 0x20) * y) / h;
                        let b_val = 0x40 + ((0xB0 - 0x40) * y) / h;
                        let color = ((r_val as u32) << 16) | ((g_val as u32) << 8) | (b_val as u32);
                        
                        for x in r.x..(r.x + r.width as isize) {
                            local_fb.set_pixel(x as usize, y as usize, color);
                        }
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
                        self.draw_window(&mut local_fb, win, self.mouse_x, self.mouse_y, *dirty_rect);
                    }
                }
            }

            // 3. Draw Taskbar
            self.draw_taskbar(&mut local_fb, self.mouse_x, self.mouse_y, *dirty_rect);

            // 4. Draw Start Menu if open
            if self.start_menu_open {
                self.draw_start_menu(&mut local_fb, self.mouse_x, self.mouse_y, *dirty_rect);
            }

            if self.context_menu_open {
                self.draw_context_menu(&mut local_fb, self.mouse_x, self.mouse_y, *dirty_rect);
            }

            // 5. Draw Mouse Cursor
            self.draw_cursor(&mut local_fb, self.mouse_x, self.mouse_y, *dirty_rect);
        }

        // STEP 2: Blit the local backbuffer to the actual Video Memory.
        // We lock the global framebuffer only for this step.
        let mut fb = FRAMEBUFFER.lock();
        
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
        let title_color = if is_active { 0x00_00_40_80 } else { 0x00_40_40_40 };
        let title_height = 22;

        // Draw main window body
        draw_rect(fb, win.x, win.y, win.width, win.height, win.color, Some(clip));
        
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
        font::draw_string(fb, win.x + 6, win.y + 3, win.title, 0x00_FFFFFF, Some(clip)); // White

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
        let bright = 0x00_FF_FF_FF;
        let dark = 0x00_40_40_40;
        draw_rect(fb, win.x, win.y, win.width, 1, bright, Some(clip)); // Top
        draw_rect(fb, win.x, win.y, 1, win.height, bright, Some(clip)); // Left
        draw_rect(fb, win.x + win.width as isize - 1, win.y, 1, win.height, dark, Some(clip)); // Right
        draw_rect(fb, win.x, win.y + win.height as isize - 1, win.width, 1, dark, Some(clip)); // Bottom
        draw_rect(fb, win.x, win.y + title_height as isize, win.width, 1, dark, Some(clip)); // Header separator
    }

    fn draw_taskbar(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Rect) {
        if let Some(ref fb_info) = fb.info {
            let taskbar_height = 40;
            let taskbar_y = (fb_info.height - taskbar_height) as isize;
            
            // Dark sleek taskbar
            draw_rect(fb, 0, taskbar_y, fb_info.width, taskbar_height, 0x00_28_28_28, Some(clip));
            draw_rect(fb, 0, taskbar_y, fb_info.width, 1, 0x00_50_50_50, Some(clip)); // Top highlight

            let start_button_width = 120;
            // Give feedback if menu is open
            let mut start_button = Button::new(0, taskbar_y, start_button_width, taskbar_height, "Start");
            start_button.bg_color = if self.start_menu_open { 0x00_40_40_40 } else { 0x00_30_30_30 };
            start_button.text_color = 0x00_FF_FF_FF;
            start_button.draw(fb, mouse_x, mouse_y, Some(clip));

            // Draw Window List
            let mut x_offset = start_button_width as isize + 10;
            let button_width = 100;
            for win in &self.windows {
                // Truncate title to fit
                let mut title = win.title;
                if title.len() > 10 { title = &win.title[..10]; }

                let mut button = Button::new(x_offset, taskbar_y + 2, button_width, taskbar_height - 4, title);
                button.bg_color = if win.minimized { 0x00_30_30_30 } else { 0x00_50_50_50 };
                button.text_color = 0x00_FF_FF_FF;
                button.draw(fb, mouse_x, mouse_y, Some(clip));
                x_offset += button_width as isize + 5;
            }

            // Draw the time on the right side
            self.draw_clock_on_taskbar(fb, clip);
        }
    }

    // New function to draw only the clock area on the taskbar
    fn draw_clock_on_taskbar(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        if let Some((width, height)) = fb.info.as_ref().map(|i| (i.width, i.height)) {
            let taskbar_height = 40;
            let taskbar_y = (height - taskbar_height) as isize;
            
            // Clear the previous time area with taskbar background color
            let time_area_width = (8 * 8) + 20; // "HH:MM:SS" is 8 chars * 8px width + padding
            let time_x_start = width as isize - time_area_width;
            draw_rect(fb, time_x_start, taskbar_y + 1, time_area_width as usize, taskbar_height - 1, 0x00_28_28_28, Some(clip));

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
        if let Some(ref fb_info) = fb.info {
            let menu_width = 200;
            let menu_height = 300;
            let taskbar_height = 40;
            let menu_x = 0;
            let menu_y = (fb_info.height - taskbar_height - menu_height) as isize;

            draw_rect(fb, menu_x, menu_y, menu_width, menu_height, 0x00_C0_C0_C0, Some(clip));

            let item_width = menu_width - 20;
            Button::new(menu_x + 10, menu_y + 15, item_width, 30, "Text Editor").draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 55, item_width, 30, "Calculator").draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 95, item_width, 30, "Settings").draw(fb, mouse_x, mouse_y, Some(clip));
            Button::new(menu_x + 10, menu_y + 135, item_width, 30, "Terminal").draw(fb, mouse_x, mouse_y, Some(clip));

            let mut shutdown_button = Button::new(menu_x + 10, menu_y + menu_height as isize - 45, item_width, 30, "Shutdown");
            shutdown_button.bg_color = 0x00_FF_60_60; // Light red
            shutdown_button.draw(fb, mouse_x, mouse_y, Some(clip));
        }
    }

    fn draw_context_menu(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let menu_x = self.context_menu_x;
        let menu_y = self.context_menu_y;
        let width = 150;
        let height = 100;

        draw_rect(fb, menu_x, menu_y, width, height, 0x00_C0_C0_C0, Some(clip));
        draw_rect(fb, menu_x, menu_y, width, 1, 0x00_FFFFFF, Some(clip));
        draw_rect(fb, menu_x, menu_y, 1, height, 0x00_FFFFFF, Some(clip));
        draw_rect(fb, menu_x + width as isize - 1, menu_y, 1, height, 0x00_40_40_40, Some(clip));
        draw_rect(fb, menu_x, menu_y + height as isize - 1, width, 1, 0x00_40_40_40, Some(clip));

        let item_width = width - 10;
        Button::new(menu_x + 5, menu_y + 5, item_width, 25, "New Terminal").draw(fb, mouse_x, mouse_y, Some(clip));
        Button::new(menu_x + 5, menu_y + 35, item_width, 25, "System Info").draw(fb, mouse_x, mouse_y, Some(clip));
        Button::new(menu_x + 5, menu_y + 65, item_width, 25, "Properties").draw(fb, mouse_x, mouse_y, Some(clip));
    }

    fn draw_cursor(&self, fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
        if let Some(fb_info) = fb.info.as_ref() {
            let width = fb_info.width as isize;
            let height = fb_info.height as isize;
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
}

pub fn draw_rect(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, width: usize, height: usize, color: u32, clip: Option<Rect>) {    
    if let Some(fb_info) = fb.info.as_ref() {
        let screen_rect = Rect { x: 0, y: 0, width: fb_info.width, height: fb_info.height };
        let mut target_rect = Rect { x, y, width, height };

        // Clip to provided clip rect
        if let Some(c) = clip {
            if let Some(clipped) = target_rect.intersection(&c) {
                target_rect = clipped;
            } else {
                return; // Completely clipped out
            }
        }

        // Use intersection to cleanly clip to screen bounds
        if let Some(clipped) = target_rect.intersection(&screen_rect) {
            let end_y = clipped.y + clipped.height as isize;
            let end_x = clipped.x + clipped.width as isize;
            for py in clipped.y .. end_y {
                for px in clipped.x .. end_x {
                    fb.set_pixel(px as usize, py as usize, color);
                }
            }
        }
    }
}

pub static WINDOW_MANAGER: Mutex<WindowManager> = Mutex::new(WindowManager::new());

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