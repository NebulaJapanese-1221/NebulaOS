//! GUI components for NebulaOS.

use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use crate::drivers::rtc::{self, CURRENT_DATETIME, TIME_NEEDS_UPDATE};
use alloc::vec::Vec;
use alloc::string::ToString;
use alloc::{format, vec};
use crate::userspace::apps::{app::{App, AppEvent}, calculator::Calculator, editor::TextEditor, paint::Paint, settings::Settings, terminal::Terminal, task_manager::TaskManager, nebula_browser::NebulaBrowser, file_manager::FileManager};
use spin::Mutex;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
pub mod rect;
use self::rect::Rect;
pub mod button;
use self::button::Button;
pub mod cursor;
pub mod icons;

use crate::userspace::localisation::{self, Localisation};

// Re-export fonts from their new location so existing references to gui::font work
pub use crate::userspace::fonts::font;

use self::cursor::CursorStyle;
const MAX_WINDOWS: usize = 10;
pub const TOP_BAR_HEIGHT: usize = 32; // Slightly taller for better readability

/// Sentinel rectangle used to signal that a window should be closed.
pub const CLOSE_SIGNAL_RECT: Rect = Rect { x: -1, y: -1, width: 0, height: 0 };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub enum InputEvent {
    MouseMove { x: isize, y: isize, dx: isize, dy: isize },
    MouseButton { button: MouseButton, down: bool, x: isize, y: isize },
    Scroll { delta: isize },
    KeyPress { key: char },
}

pub struct InputManager {
    pub mouse_x: isize,
    pub mouse_y: isize,
    pub lbutton: bool,
    pub rbutton: bool,
    pub mbutton: bool,
    pub ctrl_pressed: bool,
    pub shift_pressed: bool,
    pub alt_pressed: bool,
    pub event_queue: Vec<InputEvent>,
}

impl InputManager {
    pub const fn new() -> Self {
        Self {
            mouse_x: 400, mouse_y: 300,
            lbutton: false, rbutton: false, mbutton: false,
            ctrl_pressed: false, shift_pressed: false, alt_pressed: false,
            event_queue: Vec::new(),
        }
    }

    pub fn update(&mut self, screen_width: isize, screen_height: isize) {
        // Handle Mouse Input
        while let Some(packet) = crate::drivers::mouse::get_packet() {
            let dx = packet.x as isize;
            let dy = -packet.y as isize;

            let old_x = self.mouse_x;
            let old_y = self.mouse_y;

            self.mouse_x = (self.mouse_x + dx).clamp(0, screen_width - 1);
            self.mouse_y = (self.mouse_y + dy).clamp(0, screen_height - 1);

            // OOM Prevention: Cap the event queue. If it's too full, we drop 
            // old movements to keep the system alive.
            if self.event_queue.len() > 128 {
                self.event_queue.clear();
                return;
            }

            if old_x != self.mouse_x || old_y != self.mouse_y {
                self.event_queue.push(InputEvent::MouseMove { x: self.mouse_x, y: self.mouse_y, dx: self.mouse_x - old_x, dy: self.mouse_y - old_y });
            }

            let left_down = (packet.buttons & 1) != 0;
            if left_down != self.lbutton {
                self.lbutton = left_down;
                self.event_queue.push(InputEvent::MouseButton { button: MouseButton::Left, down: self.lbutton, x: self.mouse_x, y: self.mouse_y });
            }
            let right_down = (packet.buttons & 2) != 0;
            if right_down != self.rbutton {
                self.rbutton = right_down;
                self.event_queue.push(InputEvent::MouseButton { button: MouseButton::Right, down: self.rbutton, x: self.mouse_x, y: self.mouse_y });
            }
        }

        // Handle Keyboard Input
        while let Some(key) = crate::drivers::keyboard::get_char() {
            self.ctrl_pressed = crate::drivers::keyboard::is_ctrl_pressed();
            self.alt_pressed = crate::drivers::keyboard::is_alt_pressed();
            self.shift_pressed = crate::drivers::keyboard::is_shift_pressed();
            self.event_queue.push(InputEvent::KeyPress { key });
        }
    }
}

/// Represents an entry in a context menu.
#[derive(Clone)]
pub struct ContextMenuItem {
    pub label: alloc::string::String,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorLevel {
    Info,
    Warning,
    Moderate,
    Critical,
}

#[derive(Clone)]
pub struct MessageBox {
    pub message: alloc::string::String,
    pub icon_color: u32,
}

impl App for MessageBox {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, clip: Rect) {
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;
        
        // Icon background
        draw_rect(fb, win.x + 15, win.y + title_height as isize + 15, 32, 32, self.icon_color, Some(clip));
        draw_icon(fb, &icons::WARNING, win.x + 25, win.y + title_height as isize + 25, 0xFFFFFFFF, clip);

        // Message text
        font::draw_string(fb, win.x + 60, win.y + title_height as isize + 23, self.message.as_str(), 0xFFFFFFFF, Some(clip));

        // OK Button
        let mut ok_btn = Button::new(win.x + win.width as isize - 70, win.y + win.height as isize - 35, 60, 25, "OK");
        ok_btn.bg_color = 0x00_44_44_44;
        ok_btn.draw(fb, 0, 0, Some(clip));
    }

    fn handle_event(&mut self, event: &AppEvent, _win: &Window) -> Option<Rect> {
        if let AppEvent::MouseClick { x, y, width, height } = event {
            let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
            let title_height = font_height + 10;
            
            let ok_btn = Button::new(*width as isize - 70, *height as isize - title_height as isize - 35, 60, 25, "OK");
            if ok_btn.contains(*x, *y) {
                return Some(CLOSE_SIGNAL_RECT);
            }
        }
        None
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}

pub static PENDING_SYSTEM_ERRORS: Mutex<Vec<(ErrorLevel, &'static str, alloc::string::String)>> = Mutex::new(Vec::new());

pub fn push_system_error(level: ErrorLevel, title: &'static str, message: alloc::string::String) {
    PENDING_SYSTEM_ERRORS.lock().push((level, title, message));
}

#[derive(Clone)]
pub enum WindowContent {
    App(Box<dyn crate::userspace::apps::app::App>),
    MetadataOnly, // Used for snapshots to avoid expensive app state cloning
}

pub struct Window {
    pub id: usize,
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,
    pub title: &'static str,
    pub color: u32,
    pub content: WindowContent,
    pub minimized: bool,
    pub maximized: bool,
    pub is_system: bool,
    pub restore_rect: Option<Rect>,
}

impl Window {
    pub fn rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, width: self.width, height: self.height }
    }

    /// Returns a deep clone of the window, including the application state.
    pub fn deep_clone(&self) -> Self {
        Self {
            content: self.content.clone(),
            ..self.clone()
        }
    }
}

impl Clone for Window {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            x: self.x, y: self.y,
            width: self.width, height: self.height,
            title: self.title,
            color: self.color,
            content: WindowContent::MetadataOnly, // Shallow clone by default for performance
            minimized: self.minimized, maximized: self.maximized,
            is_system: self.is_system,
            restore_rect: self.restore_rect,
        }
    }
}

pub static DESKTOP_GRADIENT_START: AtomicU32 = AtomicU32::new(0x00_0F_11_1A); // Deep Navy
pub static DESKTOP_GRADIENT_END: AtomicU32 = AtomicU32::new(0x00_24_3B_55);   // Nebula Blue
pub static FULL_REDRAW_REQUESTED: AtomicBool = AtomicBool::new(false);
pub static WALLPAPER_MODE: AtomicU32 = AtomicU32::new(0); // 0: Gradient, 1: Solid, 2: Construction
pub static HIGH_CONTRAST: AtomicBool = AtomicBool::new(false);
pub static LARGE_TEXT: AtomicBool = AtomicBool::new(false);
pub static MOUSE_SENSITIVITY: AtomicU32 = AtomicU32::new(10); // 1.0x scale

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    TextEditor,
    Calculator,
    Paint,
    Settings,
    Terminal,
    TaskManager,
    Browser,
    FileManager,
    Reboot,
    Shutdown,
}

struct MenuEntry {
    label: alloc::string::String,
    y: isize,
    color: u32,
    action: MenuAction,
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
    input: InputManager,
    drag_win_id: Option<usize>,
    drag_offset_x: isize,
    drag_offset_y: isize,
    click_target_id: Option<usize>,
    start_menu_open: bool,
    system_menu_open: bool,
    dirty_rects: Vec<Rect>,
    context_menu_open: bool,
    context_menu_x: isize,
    context_menu_y: isize,
    resize_win_id: Option<usize>,
    resize_direction: ResizeDirection,
    cursor_style: CursorStyle,
    backbuffer: Vec<u32>,
    gradient_cache: Vec<u32>,
    task_switcher_open: bool,
    task_switcher_index: usize,
    brightness_osd_timeout: usize,
    tooltip: Option<(alloc::string::String, isize, isize)>,
    last_grad_hash: u32,
    last_power_poll: usize,
    last_click_tick: usize,
    last_click_win_id: Option<usize>,
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
            system_menu_open: false,
            dirty_rects: Vec::new(),
            context_menu_open: false,
            context_menu_x: 0,
            context_menu_y: 0,
            resize_win_id: None,
            resize_direction: ResizeDirection::None,
            cursor_style: CursorStyle::Arrow,
            backbuffer: Vec::new(),
            gradient_cache: Vec::new(),
            task_switcher_open: false,
            task_switcher_index: 0,
            brightness_osd_timeout: 0,
            tooltip: None,
            last_grad_hash: 0,
            last_power_poll: 0,
            last_click_tick: 0,
            last_click_win_id: None,
        }
    }

    pub fn spawn_error_popup(&mut self, title: &'static str, message: &str, level: ErrorLevel) {
        let (win_color, icon_color) = match level {
            ErrorLevel::Info => (0x00_1E_1E_1E, 0x00_00_78_D4),
            ErrorLevel::Warning => (0x00_2D_2D_00, 0x00_FF_D7_00),
            ErrorLevel::Moderate => (0x00_3D_1F_00, 0x00_FF_8C_00),
            ErrorLevel::Critical => (0x00_3D_00_00, 0x00_E8_11_23),
        };

        let content = MessageBox { message: message.to_string(), icon_color };
        let width = 360; let height = 100;
        let x = 220; let y = 150 + (self.windows.len() as isize * 25);

        self.add_window(Window {
            id: 0, x, y, width, height, title, color: win_color,
            content: WindowContent::App(Box::new(content)),
            minimized: false, maximized: false, is_system: true, restore_rect: None,
        });
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
        // OOM Prevention: If we have too many dirty rects, the heap might be 
        // struggling. Collapse all updates into one full screen redraw to
        // stop the Vec from allocating more memory.
        if self.dirty_rects.len() > 100 {
            self.dirty_rects.clear();
            FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
        } else if !FULL_REDRAW_REQUESTED.load(Ordering::Relaxed) {
            // Try to reserve space for 1 more rect. 
            // If it fails, we avoid the panic and fall back to a full redraw.
            if self.dirty_rects.try_reserve(1).is_err() {
                self.dirty_rects.clear();
                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
            } else {
                self.dirty_rects.push(rect);
            }
        }
    }

    fn get_menu_items(&self, l: &dyn Localisation) -> Vec<MenuEntry> {
        let menu_height = 385;
        vec![
            MenuEntry { label: l.app_text_editor().into(), y: 15, color: 0x00_C0_C0_C0, action: MenuAction::TextEditor },
            MenuEntry { label: l.app_calculator().into(), y: 55, color: 0x00_C0_C0_C0, action: MenuAction::Calculator },
            MenuEntry { label: l.app_paint().into(), y: 95, color: 0x00_C0_C0_C0, action: MenuAction::Paint },
            MenuEntry { label: l.app_settings().into(), y: 135, color: 0x00_C0_C0_C0, action: MenuAction::Settings },
            MenuEntry { label: l.app_terminal().into(), y: 175, color: 0x00_C0_C0_C0, action: MenuAction::Terminal },
            MenuEntry { label: alloc::string::String::from("Task Manager"), y: 215, color: 0x00_C0_C0_C0, action: MenuAction::TaskManager },
            MenuEntry { label: alloc::string::String::from("NebulaBrowser"), y: 255, color: 0x00_C0_C0_C0, action: MenuAction::Browser },
            MenuEntry { label: alloc::string::String::from("File Manager"), y: 295, color: 0x00_C0_C0_C0, action: MenuAction::FileManager },
            MenuEntry { label: l.btn_reboot().into(), y: menu_height - 85, color: 0x00_FF_A0_40, action: MenuAction::Reboot },
            MenuEntry { label: l.btn_shutdown().into(), y: menu_height - 45, color: 0x00_FF_60_60, action: MenuAction::Shutdown },
        ]
    }

    fn draw_wallpaper(&self, fb: &mut framebuffer::Framebuffer, dirty_rect: Rect, screen_width: isize, screen_height: isize) {
        let screen_rect = Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize };
        if !dirty_rect.intersects(&screen_rect) {
            return;
        }

        let r = dirty_rect.intersection(&screen_rect).unwrap();
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        let mode = WALLPAPER_MODE.load(Ordering::Relaxed);
        let style = GRADIENT_STYLE.load(Ordering::Relaxed);
        
        // 1. Draw Gradient/Solid Background
        if !high_contrast && mode == 2 {
            // Diagonal Construction Stripes
            if let Some(buf) = fb.draw_buffer.as_mut() {
                for y in r.y..(r.y + r.height as isize) {
                    let line_offset = y as usize * screen_width as usize;
                    for x in r.x..(r.x + r.width as isize) {
                        buf[line_offset + x as usize] = if ((x + y) / 32) % 2 == 0 { 0x00_E1_AD_01 } else { 0x00_1A_1A_1A };
                    }
                }
            }
            let msg = "NebulaOS is under construction";
            let msg_w = font::string_width(msg) as isize;
            font::draw_string(fb, (screen_width - msg_w) / 2, screen_height / 2, msg, 0x00_FFFFFF, Some(dirty_rect));
        } else {
            if let Some(buf) = fb.draw_buffer.as_mut() {
                for y in r.y..(r.y + r.height as isize) {
                    let offset = (y as usize * screen_width as usize) + r.x as usize;
                    if high_contrast {
                        buf[offset..offset + r.width].fill(0x00_000000);
                    } else if mode == 1 {
                        buf[offset..offset + r.width].fill(DESKTOP_GRADIENT_START.load(Ordering::Relaxed));
                    } else if style == 2 {
                        // Radial Gradient
                        let cx = screen_width / 2;
                        let cy = screen_height / 2;
                        let max_dist_sq = (cx * cx + cy * cy) as isize;
                        let g_start = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
                        let g_end = DESKTOP_GRADIENT_END.load(Ordering::Relaxed);
                        let dy = y - cy;
                        for x in r.x..(r.x + r.width as isize) {
                            let dx = x - cx;
                            let dist_sq = dx * dx + dy * dy;
                            buf[offset + (x - r.x) as usize] = self.interpolate_gradient(g_start, g_end, dist_sq, max_dist_sq);
                        }
                    } else if style == 0 {
                        // Vertical Linear
                        buf[offset..offset + r.width].fill(self.gradient_cache[y as usize]);
                    } else {
                        // Horizontal Linear
                        for x in 0..r.width {
                            buf[offset + x] = self.gradient_cache[r.x as usize + x];
                        }
                    }
                }
            }
        }
        
        let ver_str = format!("NebulaOS v{}", crate::kernel::VERSION);

        font::draw_string(fb, screen_width - font::string_width(ver_str.as_str()) as isize - 10, screen_height - 60, ver_str.as_str(), 0x00_70_70_70, Some(dirty_rect));

        if crate::kernel::IS_SAFE_MODE.load(Ordering::Relaxed) {
            let safe_str = "[ SAFE MODE ]";
            font::draw_string(fb, 20, screen_height - 60, safe_str, 0x00_FF_55_55, Some(dirty_rect));
        }
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

        // Process pending system errors from the kernel
        {
            let mut errors = PENDING_SYSTEM_ERRORS.lock();
            while let Some((level, title, msg)) = errors.pop() {
                self.spawn_error_popup(title, msg.as_str(), level);
            }
        }

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

        let current_tick = crate::kernel::process::TICKS.load(Ordering::Relaxed);

        // Notify non-minimized windows of system ticks
        for i in 0..self.windows.len() {
            if !self.windows[i].minimized {
                // Snapshot window info (metadata) to pass to the event handler safely
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

        let start_interaction_id = self.resize_win_id.or(self.drag_win_id);
        let start_interaction_rect = start_interaction_id.and_then(|id| self.get_window_rect(id));

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
                if let Some(idx) = self.windows.iter().position(|w| w.id == id) {
                    let min_width: isize = 80;
                    let min_height: isize = 40;
                    let mut r = self.windows[idx].rect();

                    if matches!(self.resize_direction, ResizeDirection::Right | ResizeDirection::TopRight | ResizeDirection::BottomRight) {
                        r.width = (r.width as isize + dx).max(min_width) as usize;
                    }
                    if matches!(self.resize_direction, ResizeDirection::Bottom | ResizeDirection::BottomLeft | ResizeDirection::BottomRight) {
                        r.height = (r.height as isize + dy).max(min_height) as usize;
                    }
                    if matches!(self.resize_direction, ResizeDirection::Left | ResizeDirection::TopLeft | ResizeDirection::BottomLeft) {
                        let new_width = r.width as isize - dx;
                        if new_width >= min_width { r.x += dx; r.width = new_width as usize; }
                    }
                    if matches!(self.resize_direction, ResizeDirection::Top | ResizeDirection::TopLeft | ResizeDirection::TopRight) {
                        let new_height = r.height as isize - dy;
                        if new_height >= min_height { r.y += dy; r.height = new_height as usize; }
                    }
                    let win = &mut self.windows[idx];
                    win.x = r.x; win.y = r.y; win.width = r.width; win.height = r.height;
                }
            }
            // If dragging, update window position based on the new final mouse position
            else if let Some(id) = self.drag_win_id {
                if let Some(win) = self.windows.iter_mut().find(|w| w.id == id) {
                    win.x = self.input.mouse_x - self.drag_offset_x;
                    win.y = self.input.mouse_y - self.drag_offset_y;
                }
            }
            // Handle System Menu Slider Dragging
            else if self.system_menu_open && self.input.lbutton {
                let menu_width = 180;
                let menu_x = screen_width - menu_width as isize - 10;
                let menu_y = TOP_BAR_HEIGHT as isize;
                let slider_y = menu_y + 125;
                let menu_rect = Rect { x: menu_x, y: menu_y, width: menu_width, height: 145 };

                // If mouse is within the slider area while button is held
                if self.input.mouse_y >= slider_y - 5 && self.input.mouse_y <= slider_y + 20 &&
                   self.input.mouse_x >= menu_x + 10 && self.input.mouse_x <= menu_x + 170 {
                    
                    let new_level = ((self.input.mouse_x - (menu_x + 10)).clamp(0, 160) * 100 / 160) as u8;
                    crate::drivers::brightness::set_master_brightness(new_level);
                    self.mark_dirty(menu_rect);
                }
            }
                }
                InputEvent::Scroll { delta } => {
                if self.input.mouse_y < TOP_BAR_HEIGHT as isize {
                    let rel_x = screen_width - self.input.mouse_x;
                    if rel_x >= 35 && rel_x < 80 { // Volume indicator area
                        // Direct Volume Control via Scrolling on Top Bar
                        let cur = crate::kernel::audio::MASTER_VOLUME.load(Ordering::Relaxed);
                        crate::kernel::audio::set_master_volume(if delta > 0 { cur.saturating_add(5) } else { cur.saturating_sub(5) });
                        self.mark_dirty(Rect { x: screen_width - 160, y: 0, width: 160, height: TOP_BAR_HEIGHT });
                        return;
                    } else if rel_x >= 80 && rel_x < 125 { // Brightness indicator area
                        // Direct Brightness Control via Scrolling on Top Bar
                        crate::drivers::brightness::increment_master_brightness(if delta > 0 { 5 } else { -5 });
                        self.mark_dirty(Rect { x: screen_width - 160, y: 0, width: 160, height: TOP_BAR_HEIGHT });
                        return;
                    }
                }

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

    fn update_cursor_style(&mut self, screen_width: isize, screen_height: isize) {
        let mut new_style = CursorStyle::Arrow;

        // 1. Check for interactive UI elements (Hand Cursor)
        let is_over_top_bar = self.input.mouse_y < TOP_BAR_HEIGHT as isize;
        let is_over_start_menu = self.start_menu_open && self.input.mouse_x < 210 && self.input.mouse_y >= TOP_BAR_HEIGHT as isize;
        let is_over_system_menu = self.system_menu_open && self.input.mouse_x >= (screen_width - 190) && self.input.mouse_y >= TOP_BAR_HEIGHT as isize;
        let is_over_taskbar = self.input.mouse_y >= screen_height - 40;

        // Hit area for Status Cluster (right side of top bar)
        let is_over_status = self.input.mouse_y < TOP_BAR_HEIGHT as isize && self.input.mouse_x >= (screen_width - 200);

        if is_over_top_bar || is_over_start_menu || is_over_system_menu || is_over_taskbar || is_over_status {
            new_style = CursorStyle::Hand;
        }

        // 2. Check Window buttons and resize borders
        let locale_lock = localisation::CURRENT_LOCALE.lock();
        let locale = locale_lock.as_ref();
        if let (Some(&top_win_id), Some(_l)) = (self.z_order.last(), locale) {
            if let Some(win) = self.windows.iter().find(|w| w.id == top_win_id) {
                if !win.minimized {
                    let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                    let title_height = font_height + 6;
                    
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
        if self.resize_win_id.is_none() && self.drag_win_id.is_none() {
            if let (Some(&top_win_id), Some(l)) = (self.z_order.last(), locale) {
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
                            if self.input.mouse_y >= win.y + title_height as isize {
                                if win.title == l.app_terminal() || win.title == l.app_text_editor() {
                                    new_style = CursorStyle::IBeam;
                                } else if win.title == l.app_paint() {
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

            // 1. Check if right-click is over a window
            let mut clicked_win_id = None;
            for &win_id in self.z_order.iter().rev() {
                if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                    if win.minimized || matches!(win.content, WindowContent::MetadataOnly) { continue; }
                    if win.rect().contains(self.input.mouse_x, self.input.mouse_y) {
                        clicked_win_id = Some(win_id);
                        break;
                    }
                }
            }

            if let Some(win_id) = clicked_win_id {
                // Focus and bring to front
                if let Some(pos) = self.z_order.iter().position(|&id| id == win_id) {
                    let id = self.z_order.remove(pos);
                    self.z_order.push(id);
                }

                // Send event to app
                let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                let title_height = font_height + 6;
                let win_snap = self.windows.iter().find(|w| w.id == win_id).unwrap().clone();
                
                let rel_x = self.input.mouse_x - win_snap.x;
                let rel_y = self.input.mouse_y - (win_snap.y + title_height as isize);

                if let Some(win) = self.windows.iter_mut().find(|w| w.id == win_id) {
                    if let WindowContent::App(app) = &mut win.content {
                        if let Some(dirty) = app.handle_event(&AppEvent::MouseRightClick { x: rel_x, y: rel_y, width: win.width, height: win.height }, &win_snap) {
                            self.mark_dirty(dirty);
                        }
                    }
                }
                self.context_menu_open = false;
                return;
            }

            // 2. Check if right click is on desktop (not on taskbar)
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
                    self.start_menu_open = !self.start_menu_open;
                    self.mark_dirty(Rect { x: 0, y: 0, width: 210, height: 400 });
                    return;
                }

                // Status Indicators Area Click (Cluster)
                // Adjusted to include the new network indicator
                if self.input.mouse_x >= (screen_width - 200) {
                    let rel_x = screen_width - self.input.mouse_x;
                    if rel_x < 35 { // Dropdown arrow area
                        // Clicked Dropdown area: Toggle System Menu
                        self.system_menu_open = !self.system_menu_open;
                        self.start_menu_open = false;
                        self.mark_dirty(Rect { x: screen_width - 210, y: 0, width: 210, height: 210 });
                    } else if rel_x < 80 { // Volume indicator area
                        // Clicked Volume area: Quick Mute Toggle
                        crate::kernel::audio::toggle_mute();
                        self.mark_dirty(Rect { x: screen_width - 200, y: 0, width: 200, height: TOP_BAR_HEIGHT });
                    } else if rel_x < 125 { // Brightness indicator area
                        // Clicked Brightness area: No direct action for now, but could toggle OSD
                        self.brightness_osd_timeout = 60; // Just show OSD for now
                        self.mark_dirty(Rect { x: screen_width - 200, y: 0, width: 200, height: TOP_BAR_HEIGHT });
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

            if self.system_menu_open {
                let menu_width = 180;
                let menu_x = screen_width - menu_width as isize - 10;
                let menu_y = TOP_BAR_HEIGHT as isize;
                let menu_rect = Rect { x: menu_x, y: menu_y, width: menu_width, height: 185 };
                
                if menu_rect.contains(self.input.mouse_x, self.input.mouse_y) {
                    let btn_m = Button::new(menu_x + 65, menu_y + 38, 16, 16, "-");
                    let btn_p = Button::new(menu_x + 130, menu_y + 38, 16, 16, "+");
                    let btn_mute = Button::new(menu_x + 10, menu_y + 65, 160, 25, "");
                    let btn_settings = Button::new(menu_x + 10, menu_y + 150, 160, 25, "Settings...");
                    
                    if btn_m.contains(self.input.mouse_x, self.input.mouse_y) {
                        let cur = crate::kernel::audio::MASTER_VOLUME.load(Ordering::Relaxed);
                        crate::kernel::audio::set_master_volume(cur.saturating_sub(5));
                        self.mark_dirty(menu_rect);
                    } else if btn_p.contains(self.input.mouse_x, self.input.mouse_y) {
                        let cur = crate::kernel::audio::MASTER_VOLUME.load(Ordering::Relaxed);
                        crate::kernel::audio::set_master_volume(cur.saturating_add(5));
                        self.mark_dirty(menu_rect);
                    } else if btn_mute.contains(self.input.mouse_x, self.input.mouse_y) {
                        crate::kernel::audio::toggle_mute();
                        self.mark_dirty(menu_rect);
                    } else if btn_settings.contains(self.input.mouse_x, self.input.mouse_y) {
                        self.system_menu_open = false;
                        let settings_title = locale.app_settings();
                        if !self.windows.iter().any(|w| w.title == settings_title) {
                            self.add_window(Window {
                                id: 0, x: 250, y: 150, width: 300, height: 420,
                                color: 0x00_40_20_40,
                                title: settings_title,
                                content: WindowContent::App(Box::new(Settings::new())),
                                minimized: false, maximized: false, is_system: false, restore_rect: None,
                            });
                        }
                        self.mark_dirty(Rect { x: 0, y: 0, width: screen_width as usize, height: screen_height as usize });
                    } else {
                        // Check Brightness Slider Click
                        let slider_y = menu_y + 125;
                        if self.input.mouse_y >= slider_y && self.input.mouse_y <= slider_y + 14 && self.input.mouse_x >= menu_x + 10 && self.input.mouse_x <= menu_x + 170 {
                            let new_level = ((self.input.mouse_x - (menu_x + 10)) * 100 / 160) as u8;
                            crate::drivers::brightness::set_master_brightness(new_level);
                            self.mark_dirty(menu_rect);
                        }
                    }
                    return;
                } else {
                    self.system_menu_open = false;
                    self.mark_dirty(menu_rect);
                    // Fall through to check if user clicked the toggle button again
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
                            minimized: false, maximized: false, is_system: false, restore_rect: None,
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
                    if win.is_system || matches!(win.content, WindowContent::MetadataOnly) { continue; }
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
                let rel_y = self.input.mouse_y - menu_y;
                
                self.mark_dirty(Rect { x: 0, y: menu_y, width: 200, height: 385 });

                let items = self.get_menu_items(locale.as_ref());
                let mut clicked_action = None;
                for item in items {
                    if self.input.mouse_x >= 10 && self.input.mouse_x < 190 && rel_y >= item.y && rel_y < item.y + 30 {
                        clicked_action = Some(item.action);
                        break;
                    }
                }

                if let Some(action) = clicked_action {
                    match action {
                        MenuAction::Settings => {
                    let settings_open = self.windows.iter().any(|w| w.title == locale.app_settings());
                    if !settings_open {
                        self.add_window(Window {
                            id: 0, x: 250, y: 150, width: 300, height: 420,
                            color: 0x00_40_20_40, // Dark Purple
                            title: locale.app_settings(),
                            content: WindowContent::App(Box::new(Settings::new())),
                            minimized: false, maximized: false, is_system: false, restore_rect: None,
                        });
                    }
                        }
                        MenuAction::Browser => {
                            self.add_window(Window {
                                id: 0, x: 80, y: 80, width: 600, height: 400,
                                color: 0x00_2D_2D_30,
                                title: "NebulaBrowser",
                                content: WindowContent::App(Box::new(NebulaBrowser::new())),
                                minimized: false, maximized: false, is_system: false, restore_rect: None,
                            });
                        }
                        MenuAction::TaskManager => {
                    self.add_window(Window {
                        id: 0, x: 150, y: 150, width: 300, height: 400,
                        color: 0x00_00_40_40,
                        title: "Task Manager",
                        content: WindowContent::App(Box::new(TaskManager::new())),
                        minimized: false, maximized: false, is_system: false, restore_rect: None,
                    });
                        }
                        MenuAction::FileManager => {
                            self.add_window(Window {
                                id: 0, x: 120, y: 120, width: 450, height: 350,
                                color: 0x00_25_25_26,
                                title: "File Manager",
                                content: WindowContent::App(Box::new(FileManager::new())),
                                minimized: false, maximized: false, is_system: false, restore_rect: None,
                            });
                        }
                        MenuAction::Terminal => {
                    self.add_window(Window {
                        id: 0, x: 100, y: 100, width: 480, height: 320,
                        color: 0x00_1E_1E_1E,
                        title: locale.app_terminal(),
                        content: WindowContent::App(Box::new(Terminal::new())),
                        minimized: false, maximized: false, is_system: false, restore_rect: None,
                    });
                        }
                        MenuAction::Calculator => {
                    self.add_window(Window {
                        id: 0, x: 50, y: 350, width: 200, height: 220,
                        color: 0x00_20_20_20,
                        title: locale.app_calculator(),
                        content: WindowContent::App(Box::new(Calculator::new())),
                        minimized: false, maximized: false, is_system: false, restore_rect: None,
                    });
                        }
                        MenuAction::Paint => {
                    self.add_window(Window {
                        id: 0, x: 180, y: 100, width: 400, height: 300,
                        color: 0x00_20_20_20,
                        title: locale.app_paint(),
                        content: WindowContent::App(Box::new(Paint::new())),
                        minimized: false, maximized: false, is_system: false, restore_rect: None,
                    });
                        }
                        MenuAction::TextEditor => {
                    self.add_window(Window {
                        id: 0, x: 150, y: 150, width: 400, height: 300,
                        color: 0x00_1E_1E_1E,
                        title: locale.app_text_editor(),
                        content: WindowContent::App(Box::new(TextEditor::new())),
                        minimized: false, maximized: false, is_system: false, restore_rect: None,
                    });
                        }
                        MenuAction::Reboot => { crate::kernel::power::reboot(); }
                        MenuAction::Shutdown => { crate::kernel::power::shutdown(); }
                    }
                    self.start_menu_open = false;
                }
            } else {
                // 2. Not a start button or menu click. Check for window interaction.
                let mut clicked_win_id = None;
                let mut resize_dir = ResizeDirection::None;
                const BORDER_SIZE: isize = 5;

                for &win_id in self.z_order.iter().rev() {
                    if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                        if win.minimized || matches!(win.content, WindowContent::MetadataOnly) { continue; } // Skip minimized and ghost windows

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
                            }
                        }
                    }
                } else {
                    // Clicked on desktop
                    self.click_target_id = None;
                    self.start_menu_open = false;
                }
            }
        } else if let (MouseButton::Left, false) = (button, down) {

            if let Some(_win_id) = self.resize_win_id {
                self.resize_win_id = None;
                self.resize_direction = ResizeDirection::None;
            } else if let Some(_win_id) = self.drag_win_id {
                self.drag_win_id = None;
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
                        // Take the snapshot BEFORE getting the mutable borrow of self.windows
                        let win_snap = self.windows.iter().find(|w| w.id == target_id).unwrap().clone();
                        if let Some(win) = self.windows.iter_mut().find(|w| w.id == target_id) {
                            let current_tick = crate::kernel::process::TICKS.load(Ordering::Relaxed);
                            let is_double = self.last_click_win_id == Some(target_id) && 
                                            current_tick < self.last_click_tick + 500;
                            
                            let event = if is_double {
                                AppEvent::MouseDoubleClick { x: rel_x, y: rel_y, width: win.width, height: win.height }
                            } else {
                                AppEvent::MouseClick { x: rel_x, y: rel_y, width: win.width, height: win.height }
                            };

                            if let WindowContent::App(app) = &mut win.content {
                                if let Some(dirty) = app.handle_event(&event, &win_snap) {
                                    if dirty.x == -1 && dirty.y == -1 && dirty.width == 0 {
                                        // Close Signal Received
                                        let id_to_remove = target_id;
                                        self.windows.retain(|w| w.id != id_to_remove);
                                        self.z_order.retain(|&id| id != id_to_remove);
                                        self.mark_dirty(win_snap.rect());
                                        // Redraw taskbar to update window list
                                        self.mark_dirty(Rect { x: 0, y: screen_height - 40, width: screen_width as usize, height: 40 });
                                    } else {
                                        self.mark_dirty(dirty);
                                    }
                                } else {
                                    self.mark_dirty(win_snap.rect());
                                }
                            }

                            self.last_click_tick = if is_double { 0 } else { current_tick };
                            self.last_click_win_id = if is_double { None } else { Some(target_id) };
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

        // Second pass: Proximity merging to prevent "death by a thousand cuts" 
        // especially in QEMU where each draw call has high overhead.
        if final_rects.len() > 5 {
            let mut merged_rects: Vec<Rect> = Vec::new();
            while let Some(mut r) = final_rects.pop() {
                let mut i = 0;
                while i < final_rects.len() {
                    if r.inflate(10).intersects(&final_rects[i]) { r = r.union(&final_rects.remove(i)); } 
                    else { i += 1; }
                }
                merged_rects.push(r);
            }
            final_rects = merged_rects;
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
            // Attempt to reserve the exact amount needed for the screen.
            // If this fails, we cannot draw the GUI backbuffer.
            if self.backbuffer.try_reserve_exact(buffer_size.saturating_sub(self.backbuffer.len())).is_err() {
                // Fail-safe: Push a system error if there's enough room, 
                // otherwise, this frame is dropped.
                return;
            }
            self.backbuffer.resize(buffer_size, 0);
        }

        // Optimized Gradient: Only update cache when screen size or colors change
        let g_start = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
        let g_end = DESKTOP_GRADIENT_END.load(Ordering::Relaxed);
        let mode = WALLPAPER_MODE.load(Ordering::Relaxed);
        let g_hash = g_start ^ g_end ^ mode;

        if self.gradient_cache.len() != fb_info.height || self.last_grad_hash != g_hash {
            self.gradient_cache.resize(fb_info.height, 0);
            for y in 0..fb_info.height {
                self.gradient_cache[y] = if mode == 0 { self.interpolate_gradient(g_start, g_end, y as isize, fb_info.height as isize) } else { g_start };
            }
            self.last_grad_hash = g_hash;
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
            // 1. Desktop Wallpaper and Decorative Elements
            let mut wallpaper_covered = false;
            
            // Z-Order Optimization for Wallpaper: Check if any window completely covers this dirty rect.
            for &win_id in &self.z_order {
                if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                    if !win.minimized {
                        let wr = win.rect();
                        if wr.x <= dirty_rect.x && wr.y <= dirty_rect.y &&
                           wr.x + wr.width as isize >= dirty_rect.x + dirty_rect.width as isize &&
                           wr.y + wr.height as isize >= dirty_rect.y + dirty_rect.height as isize {
                            wallpaper_covered = true;
                            break;
                        }
                    }
                }
            }

            if !wallpaper_covered {
                self.draw_wallpaper(&mut local_fb, *dirty_rect, screen_width, screen_height);
            }

            // 2. Draw Windows
            for (idx, &win_id) in self.z_order.iter().enumerate() {
                if let Some(win) = self.windows.iter().find(|w| w.id == win_id) {
                    if win.minimized || matches!(win.content, WindowContent::MetadataOnly) { continue; }
                    let win_rect = win.rect();

                    if dirty_rect.intersects(&win_rect) {
                        // Z-Order Optimization: Calculate the actual region of this window that needs drawing.
                        let overlap = dirty_rect.intersection(&win_rect).unwrap();
                        let mut is_covered = false;

                        // Check if any window ABOVE this one in the Z-order completely covers the overlapping region.
                        for &above_id in &self.z_order[idx + 1..] {
                            if let Some(above_win) = self.windows.iter().find(|w| w.id == above_id) {
                                if !above_win.minimized {
                                    let ar = above_win.rect();
                                    if ar.x <= overlap.x && ar.y <= overlap.y &&
                                       ar.x + ar.width as isize >= overlap.x + overlap.width as isize &&
                                       ar.y + ar.height as isize >= overlap.y + overlap.height as isize {
                                        is_covered = true;
                                        break;
                                    }
                                }
                            }
                        }

                        if !is_covered {
                            self.draw_window(&mut local_fb, win, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
                        }
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
            if self.start_menu_open && dirty_rect.intersects(&Rect { x: 0, y: TOP_BAR_HEIGHT as isize, width: 210, height: 385 }) {
                self.draw_start_menu_optimized(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect, locale);
            }

            if self.system_menu_open {
                self.draw_system_menu_optimized(&mut local_fb, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
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

            // 6. Draw Tooltip
            if let Some((ref text, tx, ty)) = self.tooltip {
                self.draw_tooltip(&mut local_fb, text.as_str(), tx, ty, *dirty_rect);
            }

            // 5. Draw Mouse Cursor
            cursor::draw_cursor(&mut local_fb, self.cursor_style, self.input.mouse_x, self.input.mouse_y, *dirty_rect);
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

    fn interpolate_gradient(&self, start: u32, end: u32, current_val: isize, max_val: isize) -> u32 {
        let (r1, g1, b1) = (((start >> 16) & 0xFF) as isize, ((start >> 8) & 0xFF) as isize, (start & 0xFF) as isize);
        let (r2, g2, b2) = (((end >> 16) & 0xFF) as isize, ((end >> 8) & 0xFF) as isize, (end & 0xFF) as isize);
        let rv = r1 + ((r2 - r1) * current_val) / max_val;
        let gv = g1 + ((g2 - g1) * current_val) / max_val;
        let bv = b1 + ((b2 - b1) * current_val) / max_val;
        ((rv as u32) << 16) | ((gv as u32) << 8) | (bv as u32)
    }

    fn draw_win_btn(&self, fb: &mut framebuffer::Framebuffer, x: isize, y: isize, icon: &[[u8; 12]; 12], color: u32, clip: Rect) {
        let rect = Rect { x, y, width: 16, height: 16 };
        if !clip.intersects(&rect) { return; }
        draw_rect(fb, x, y, 16, 16, color, Some(clip));
        draw_icon(fb, icon, x + 2, y + 2, 0x00_FFFFFF, clip);
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

        // 1. Draw Square Border and Background
        draw_rect(fb, win.x, win.y, win.width, win.height, border_color, Some(clip));
        // Inner body
        draw_rect(fb, win.x + 1, win.y + 1, win.width - 2, win.height - 2, body_color, Some(clip));

        // Draw square title bar
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
        self.draw_win_btn(fb, win.x + win.width as isize - 20, btn_y, &icons::CLOSE, 0x00_C0_40_40, clip);
        
        let max_icon = if win.maximized { &icons::RESTORE } else { &icons::MAXIMIZE };
        self.draw_win_btn(fb, win.x + win.width as isize - 40, btn_y, max_icon, title_color, clip);
        self.draw_win_btn(fb, win.x + win.width as isize - 60, btn_y, &icons::MINIMIZE, title_color, clip);

        // 2. Draw Content LAST (Ensures it is on top of the window background)
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
                if win.is_system || matches!(win.content, WindowContent::MetadataOnly) { continue; }
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
        
        let clock_x = (width as isize - clock_text_width as isize) / 2;
        font::draw_string(fb, clock_x, 5, clock_text, 0x00_FFFFFF, Some(clip));

        let mut cursor_x = width as isize - 15;
        
        // Dropdown Arrow
        draw_icon(fb, &icons::DROPDOWN, cursor_x - 14, 10, 0x00_AAAAAA, clip);
        cursor_x -= 20;

        // Volume Indicator
        let vol_ind_w = 45; // Icon + "100%"
        cursor_x -= vol_ind_w;
        draw_volume_indicator(fb, cursor_x, 7, clip);
        cursor_x -= 5;

        // Brightness Indicator
        let bright_ind_w = 45; // Icon + "100%"
        cursor_x -= bright_ind_w;
        draw_brightness_indicator(fb, cursor_x, 7, clip);
        cursor_x -= 5;

        // Network Indicator
        let net_ind_w = 40; // Icon + "100%"
        cursor_x -= net_ind_w;
        draw_network_indicator(fb, cursor_x, 7, clip);

        if battery_info.health > 0 {
            cursor_x -= 85; // Battery width + padding
            draw_battery_indicator(fb, cursor_x, 7, clip);
        }
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
        
        let menu_y = TOP_BAR_HEIGHT as isize; // Animation removed, fixed position
        let menu_x = 0;
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);

        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_1E_1E_1E };

        // 1. Draw Background (Flat design, no "effects")
        draw_rect(fb, menu_x, menu_y, menu_width, menu_height, bg_color, Some(clip));
        
        if let Some(l) = locale {
            let item_width = menu_width - 20;
            let items = self.get_menu_items(l.as_ref());

            for item in items {
                let item_x = menu_x + 10;
                let item_y = menu_y + item.y;
                let is_hovered = mouse_x >= item_x && mouse_x < item_x + item_width as isize && 
                                 mouse_y >= item_y && mouse_y < item_y + 30;

                // Dirty-rect optimization: Skip drawing this item if it doesn't intersect 
                // the region being refreshed.
                if !clip.intersects(&Rect { x: item_x, y: item_y, width: item_width, height: 30 }) { continue; }

                let bg = if is_hovered { 0x00_D0_D0_D0 } else { item.color };

                draw_rect(fb, item_x, item_y, item_width, 30, bg, Some(clip));
                font::draw_string(fb, item_x + 10, item_y + 7, &item.label, 0x00_000000, Some(clip));
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
            let mut displayed_count = 0;
            for &win_id in self.z_order.iter().rev() {
                let win = if let Some(w) = self.windows.iter().find(|w| w.id == win_id) { w } else { continue };
                if win.is_system || matches!(win.content, WindowContent::MetadataOnly) { continue; }

                if displayed_count >= 6 { break; } // Limit to 6 items
                
                let item_x = start_x + (displayed_count as isize * (icon_size + padding));
                let color = if displayed_count == self.task_switcher_index { 0x00_50_50_90 } else { 0x00_40_40_40 };
                
                draw_rect(fb, item_x, start_y, icon_size as usize, icon_size as usize, color, Some(clip));
                // Draw simple char as icon representation
                font::draw_char(fb, item_x + 12, start_y + 12, win.title.chars().next().unwrap_or('?'), 0x00_FFFFFF, Some(clip));
                displayed_count += 1;
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

    fn draw_system_menu_optimized(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, mut clip: Rect) {
        let width = if let Some(ref info) = fb.info { info.width } else { return };
        let menu_width = 180;
        let menu_height = 185;
        let menu_x = width as isize - menu_width as isize - 10;
        let menu_y = TOP_BAR_HEIGHT as isize;

        let visible_area = Rect { x: menu_x, y: menu_y, width: menu_width, height: menu_height };
        if let Some(c) = clip.intersection(&visible_area) {
            clip = c;
        } else { return; }

        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_1E_1E_1E };

        draw_rect(fb, menu_x, menu_y, menu_width, menu_height, bg_color, Some(clip));
        draw_rect(fb, menu_x, menu_y, menu_width, 1, 0x00_44_44_44, Some(clip)); // Top border

        font::draw_string(fb, menu_x + 10, menu_y + 10, "System Settings", 0x00_AAAAAA, Some(clip));

        // Volume Control UI
        let vol = crate::kernel::audio::MASTER_VOLUME.load(Ordering::Relaxed);
        let muted = crate::kernel::audio::IS_MUTED.load(Ordering::Relaxed);
        font::draw_string(fb, menu_x + 10, menu_y + 40, if muted { "Muted:" } else { "Volume:" }, 0x00_FFFFFF, Some(clip));
        
        let vol_text = format!("{}%", vol);
        font::draw_string(fb, menu_x + 85, menu_y + 40, &vol_text, 0x00_FFFFFF, Some(clip));

        let btn_m = Button::new(menu_x + 65, menu_y + 38, 16, 16, "-");
        let btn_p = Button::new(menu_x + 130, menu_y + 38, 16, 16, "+");
        
        btn_m.draw(fb, mouse_x, mouse_y, Some(clip));
        btn_p.draw(fb, mouse_x, mouse_y, Some(clip));

        let mut btn_mute = Button::new(menu_x + 10, menu_y + 65, 160, 25, if muted { "Unmute" } else { "Mute" });
        if muted { btn_mute.bg_color = 0x00_60_20_20; } // Subtle red hue for muted state
        btn_mute.draw(fb, mouse_x, mouse_y, Some(clip));

        // Brightness Slider UI
        let bright = crate::drivers::brightness::BRIGHTNESS_LEVEL.load(Ordering::Relaxed);
        font::draw_string(fb, menu_x + 10, menu_y + 100, "Brightness:", 0x00_FFFFFF, Some(clip));
        let bright_text = format!("{}%", bright);
        font::draw_string(fb, menu_x + 120, menu_y + 100, &bright_text, 0x00_FFFFFF, Some(clip));

        let slider_y = menu_y + 125;
        let slider_w = 160;
        // Track
        draw_rect(fb, menu_x + 10, slider_y + 6, slider_w, 2, 0x00_44_44_44, Some(clip));
        // Handle
        let handle_x = menu_x + 10 + (bright as isize * slider_w as isize / 100);
        draw_rect(fb, handle_x - 4, slider_y, 8, 14, 0x00_FF_CC_00, Some(clip));

        // Settings Shortcut Button
        let btn_settings = Button::new(menu_x + 10, menu_y + 150, 160, 25, "Settings...");
        btn_settings.draw(fb, mouse_x, mouse_y, Some(clip));
    }
}

/// Draws a volume icon and percentage on the top bar.
pub fn draw_volume_indicator(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
    let vol = crate::kernel::audio::MASTER_VOLUME.load(Ordering::Relaxed);
    let muted = crate::kernel::audio::IS_MUTED.load(Ordering::Relaxed);

    let icon_bmp = if muted { &icons::VOL_MUTE } else { &icons::VOL_HIGH };
    let icon_color = if muted { 0x00_FF_40_40 } else { 0x00_FFFFFF };
    draw_icon(fb, icon_bmp, x, y + 3, icon_color, clip);

    // Draw percentage if not muted
    if !muted {
        let s = format!("{}%", vol);
        font::draw_string(fb, x + 15, y + 5, s.as_str(), 0x00_FFFFFF, Some(clip));
    }
}

/// Draws a brightness icon and percentage on the top bar.
pub fn draw_brightness_indicator(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
    let bright = crate::drivers::brightness::BRIGHTNESS_LEVEL.load(Ordering::Relaxed);
    
    draw_icon(fb, &icons::BRIGHTNESS, x, y + 3, 0x00_FF_CC_00, clip);

    // Draw percentage
    let s = format!("{}%", bright);
    font::draw_string(fb, x + 15, y + 5, s.as_str(), 0x00_FFFFFF, Some(clip));
}

/// Draws a network signal strength icon and percentage on the top bar.
pub fn draw_network_indicator(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
    let signal = crate::kernel::net::NETWORK_SIGNAL_STRENGTH.load(Ordering::Relaxed);
    let conn_type = crate::kernel::net::CONNECTION_TYPE.load(Ordering::Relaxed);

    let icon_bmp = if conn_type == crate::kernel::net::ConnectionType::Ethernet as u8 {
        &icons::NET_ETHERNET
    } else if conn_type == crate::kernel::net::ConnectionType::Wifi as u8 {
        &icons::NET_WIFI
    } else {
        &icons::NET_WIFI // Disconnected wifi icon or similar
    };

    let icon_color = if conn_type == 0 { 0x00_FF_40_40 } else { 0x00_FFFFFF };
    draw_icon(fb, icon_bmp, x, y + 3, icon_color, clip);

    // Draw percentage
    let s = format!("{}%", signal);
    font::draw_string(fb, x + 15, y + 5, s.as_str(), 0x00_FFFFFF, Some(clip));
}

/// Draws a 12x12 bitmap icon at the specified location.
pub fn draw_icon(fb: &mut framebuffer::Framebuffer, bitmap: &[[u8; 12]; 12], x: isize, y: isize, color: u32, clip: Rect) {
    for (dy, row) in bitmap.iter().enumerate() {
        for (dx, &pixel) in row.iter().enumerate() {
            if pixel == 1 {
                let px = x + dx as isize;
                let py = y + dy as isize;
                if clip.contains(px, py) {
                    fb.set_pixel(px as usize, py as usize, color);
                }
            }
        }
    }
}

/// Generic helper to draw a context menu at a specific location.
pub fn draw_context_menu_box(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, items: &[ContextMenuItem], mouse_x: isize, mouse_y: isize, clip: Rect) {
    let item_h = 25;
    let width = 150;
    let height = items.len() * item_h + 10;
    
    // Draw Background
    draw_rect(fb, x, y, width, height, 0x00_2D_2D_30, Some(clip));
    draw_rect(fb, x, y, width, 1, 0x00_FFFFFF, Some(clip)); // Top Border

    for (i, item) in items.iter().enumerate() {
        let ix = x + 5;
        let iy = y + 5 + (i as isize * item_h as isize);
        let is_hovered = mouse_x >= x && mouse_x < x + width as isize && mouse_y >= iy && mouse_y < iy + item_h as isize;
        
        if is_hovered && item.enabled {
            draw_rect(fb, x + 2, iy, width - 4, item_h, 0x00_3E_3E_42, Some(clip));
        }
        
        let color = if item.enabled { 0x00_FFFFFF } else { 0x00_80_80_80 };
        font::draw_string(fb, ix + 5, iy + 5, item.label.as_str(), color, Some(clip));
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