use crate::drivers::framebuffer;
use crate::userspace::gui::{font, Window, button::Button, rect::Rect};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use crate::userspace::gui::{DESKTOP_GRADIENT_START, DESKTOP_GRADIENT_END, FULL_REDRAW_REQUESTED, HIGH_CONTRAST, LARGE_TEXT, MOUSE_SENSITIVITY};
use crate::userspace::gui::WALLPAPER_MODE;
use crate::userspace::localisation::{self, Language};
use crate::kernel::cpu::CPU_BRAND;
use core::cell::{Cell, RefCell};

const SIDEBAR_WIDTH: usize = 140;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    System,
    Accessibility,
    Theme,
    Language,
    Mouse,
    Power,
    Display,
    Network,
}

#[derive(Clone)]
pub struct Settings {
    current_tab: Tab,
    search_query: String,
    search_focused: bool,
    dirty: Cell<bool>,
    highlighted_index: Cell<usize>,
    // Caches to avoid redundant formatting/filtering on every mouse move
    tab_cache: RefCell<Vec<(Tab, String)>>,
    filtered_indices: RefCell<Vec<usize>>,
    sys_info_cache: RefCell<Vec<String>>,
    last_sys_update_tick: Cell<usize>,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            current_tab: Tab::System,
            search_query: String::new(),
            search_focused: false,
            dirty: Cell::new(true),
            highlighted_index: Cell::new(0),
            tab_cache: RefCell::new(Vec::new()),
            filtered_indices: RefCell::new(Vec::new()),
            sys_info_cache: RefCell::new(Vec::new()),
            last_sys_update_tick: Cell::new(0),
        }
    }

    /// Centralized source of truth for all available settings categories.
    fn get_all_tabs(&self) -> Vec<(Tab, String)> {
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        alloc::vec![
            (Tab::System, locale.settings_tab_system().into()),
            (Tab::Accessibility, locale.settings_tab_a11y().into()),
            (Tab::Theme, locale.settings_tab_theme().into()),
            (Tab::Language, locale.settings_tab_language().into()),
            (Tab::Mouse, locale.settings_tab_mouse().into()),
            (Tab::Power, String::from("Power")),
            (Tab::Display, String::from("Display")),
            (Tab::Network, String::from("Network")),
        ]
    }

    fn draw_sidebar(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;
        let start_y = win.y + title_height as isize;

        if self.dirty.get() {
            // Update local tab cache only when dirty
            let mut tabs = self.tab_cache.borrow_mut();
            tabs.clear();
            tabs.extend(self.get_all_tabs());
            
            let query = self.search_query.to_lowercase();
            let mut indices = self.filtered_indices.borrow_mut();
            indices.clear();
            for (i, (_, ref label)) in tabs.iter().enumerate() {
                if query.is_empty() || label.to_lowercase().contains(&query) {
                    indices.push(i);
                }
            }
            // Clamp highlight to new list size
            self.highlighted_index.set(self.highlighted_index.get().min(indices.len().saturating_sub(1)));
        }

        let item_height = font_height as isize + 20;
        let search_box_h = font_height as isize + 10;
        let list_start_y = start_y + search_box_h + 10;

        // Draw Sidebar Background
        let sidebar_rect = Rect { x: win.x, y: start_y, width: SIDEBAR_WIDTH, height: win.height - title_height };
        if dirty_rect.intersects(&sidebar_rect) {
            crate::userspace::gui::draw_rect(fb, win.x, start_y, SIDEBAR_WIDTH, win.height - title_height, 0x00_25_25_26, Some(dirty_rect));
            // Right separator line
            crate::userspace::gui::draw_rect(fb, win.x + SIDEBAR_WIDTH as isize - 1, start_y, 1, win.height - title_height, 0x00_3F_3F_46, Some(dirty_rect));

            // Draw Search Box
            let search_rect = Rect { x: win.x + 5, y: start_y + 5, width: SIDEBAR_WIDTH - 10, height: search_box_h as usize };
            let bg = if self.search_focused { 0x00_3E_3E_42 } else { 0x00_1E_1E_1E };
            crate::userspace::gui::draw_rect(fb, search_rect.x, search_rect.y, search_rect.width, search_rect.height, bg, Some(dirty_rect));
            
            let text = if self.search_query.is_empty() && !self.search_focused { "Search..." } else { self.search_query.as_str() };
            let color = if self.search_query.is_empty() && !self.search_focused { 0x00_80_80_80 } else { 0x00_FFFFFF };
            font::draw_string(fb, win.x + 10, start_y + 10, text, color, Some(dirty_rect));

            if self.search_focused {
                let cursor_x = win.x + 10 + font::string_width(self.search_query.as_str()) as isize;
                crate::userspace::gui::draw_rect(fb, cursor_x, start_y + 10, 2, font_height, 0x00_00_7A_CC, Some(dirty_rect));
            }

            // Draw Clear Button (X) if there is a query
            if !self.search_query.is_empty() {
                let clear_btn_x = win.x + SIDEBAR_WIDTH as isize - 22;
                font::draw_string(fb, clear_btn_x, start_y + 10, "X", 0x00_CC_66_66, Some(dirty_rect));
            }
        }

        let all_tabs = self.tab_cache.borrow();
        let indices = self.filtered_indices.borrow();
        for (i, &idx) in indices.iter().enumerate() {
            let (tab, ref label) = &all_tabs[idx];

            let item_y = list_start_y + (i as isize * item_height);
            let item_rect = Rect { x: win.x, y: item_y, width: SIDEBAR_WIDTH, height: item_height as usize };

            if !dirty_rect.intersects(&item_rect) { continue; }

            let is_active = self.current_tab == *tab;
            let is_highlighted = i == self.highlighted_index.get();
            
            if is_active {
                crate::userspace::gui::draw_rect(fb, win.x, item_y, SIDEBAR_WIDTH - 1, item_height as usize, 0x00_37_37_3D, Some(dirty_rect));
                // Selection indicator
                crate::userspace::gui::draw_rect(fb, win.x, item_y, 4, item_height as usize, 0x00_00_7A_CC, Some(dirty_rect));
            } else if is_highlighted {
                crate::userspace::gui::draw_rect(fb, win.x, item_y, SIDEBAR_WIDTH - 1, item_height as usize, 0x00_3E_3E_42, Some(dirty_rect));
            }

            let tx = win.x + 15;
            let ty = item_y + (item_height - font_height as isize) / 2;
            let text_color = if is_active { 0x00_FFFFFF } else { 0x00_CCCCCC };
            font::draw_string(fb, tx, ty, label, text_color, Some(dirty_rect));
        }
    }

    fn draw_content(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;
        let content_y = win.y + title_height as isize + 15;
        let content_x = win.x + SIDEBAR_WIDTH as isize + 20;
        let line_spacing = font_height as isize + 12;
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();

        match self.current_tab {
            Tab::System => {
                font::draw_string(fb, content_x, content_y, locale.app_system_info(), 0x00_FF_FF_FF, None);
                let v_str = format!("{} {}", locale.info_version(), crate::kernel::VERSION);
                font::draw_string(fb, content_x, content_y + line_spacing, v_str.as_str(), 0x00_CC_CC_CC, None);
                
                // CPU Info
                let brand_guard = CPU_BRAND.lock();
                let cpu_name = brand_guard.as_deref().unwrap_or("Unknown CPU");
                let cores = crate::kernel::CPU_CORES.load(Ordering::Relaxed);
                let core_str = if cores == 1 { "Core" } else { "Cores" };
                
                // Truncate CPU string if too long for display
                let cpu_display = if cpu_name.len() > 30 { &cpu_name[..30] } else { cpu_name };
                
                font::draw_string(fb, content_x, content_y + line_spacing * 2, format!("Processor: {}", cpu_display).as_str(), 0x00_CC_CC_CC, None);
                font::draw_string(fb, content_x, content_y + line_spacing * 3, &format!("Cores: {} {}", cores, core_str), 0x00_CC_CC_CC, None);

                // Resolution
                let res_str = if let Some(info) = fb.info.as_ref() {
                    format!("{} {}x{}", locale.info_resolution(), info.width, info.height)
                } else {
                    format!("{} Unknown", locale.info_resolution())
                };
                font::draw_string(fb, content_x, content_y + line_spacing * 4, res_str.as_str(), 0x00_CC_CC_CC, None);

                // Memory
                let mem_bytes = crate::kernel::TOTAL_MEMORY.load(Ordering::Relaxed);
                let mem_mb = mem_bytes / 1024 / 1024;
                let mem_str = format!("{} {} MB", locale.info_memory(), mem_mb);
                font::draw_string(fb, content_x, content_y + line_spacing * 5, mem_str.as_str(), 0x00_CC_CC_CC, None);

                // Uptime
                let total_seconds = crate::kernel::process::TICKS.load(Ordering::Relaxed) / 1000;
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;
                let time_str = format!("{} {:02}:{:02}:{:02}", locale.info_uptime(), hours, minutes, seconds);
                font::draw_string(fb, content_x, content_y + line_spacing * 6, time_str.as_str(), 0x00_CC_CC_CC, None);

            },
            Tab::Accessibility => {
                font::draw_string(fb, content_x, content_y, locale.settings_tab_a11y(), 0x00_FF_FF_FF, None);
                
                let hc = HIGH_CONTRAST.load(Ordering::Relaxed);
                let lt = LARGE_TEXT.load(Ordering::Relaxed);

                let btn_hc_text = format!("{}  {}", if hc { "[x]" } else { "[ ]" }, locale.option_high_contrast());
                
                let btn_hc = Button::new(content_x, content_y + line_spacing, 240, font_height + 14, btn_hc_text);
                let btn_lt_text = format!("{}  {}", if lt { "[x]" } else { "[ ]" }, locale.option_large_text());

                let btn_lt = Button::new(content_x, content_y + line_spacing * 2 + 10, 240, font_height + 14, btn_lt_text);
                
                // Pass window rect as clip to ensure items don't draw outside settings window
                btn_hc.draw(fb, 0, 0, Some(dirty_rect));
                btn_lt.draw(fb, 0, 0, Some(dirty_rect));

                // Brightness Slider
                let slider_y = content_y + line_spacing * 4;
                font::draw_string(fb, content_x, slider_y, locale.label_brightness(), 0x00_FF_FF_FF, Some(dirty_rect));
                
                let slider_x = content_x + 100;
                let slider_w = 150;
                let slider_h = 2;
                let level = crate::drivers::brightness::BRIGHTNESS_LEVEL.load(Ordering::Relaxed);

                // Track
                crate::userspace::gui::draw_rect(fb, slider_x, slider_y + 6, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                // Handle
                let handle_x = slider_x + ((level as usize * slider_w) / 100) as isize;
                crate::userspace::gui::draw_rect(fb, handle_x - 4, slider_y, 8, 14, 0x00_FF_CC_00, Some(dirty_rect));
            },
            Tab::Theme => {
                font::draw_string(fb, content_x, content_y, locale.label_bg_color(), 0x00_FF_FF_FF, Some(dirty_rect));
                
                let start_color = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
                let r_val = (start_color >> 16) & 0xFF;
                let g_val = (start_color >> 8) & 0xFF;
                let b_val = start_color & 0xFF;

                let slider_x = content_x + 30;
                let slider_w = 180;
                let slider_h = 2; // Track height

                // Red Slider
                let ry = content_y + (font_height as isize + 9);
                font::draw_string(fb, content_x, ry - 4, "R", 0x00_FF_40_40, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, slider_x, ry, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                let rx_pos = slider_x + ((r_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, rx_pos - 2, ry - 4, 4, 10, 0x00_FF_FF_FF, Some(dirty_rect));

                // Green Slider
                let gy = content_y + (font_height as isize + 9) + 20;
                font::draw_string(fb, content_x, gy - 4, "G", 0x00_40_FF_40, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, slider_x, gy, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                let gx_pos = slider_x + ((g_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, gx_pos - 2, gy - 4, 4, 10, 0x00_FF_FF_FF, Some(dirty_rect));

                // Blue Slider
                let by = content_y + (font_height as isize + 9) + 40;
                font::draw_string(fb, content_x, by - 4, "B", 0x00_40_40_FF, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, slider_x, by, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                let bx_pos = slider_x + ((b_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, bx_pos - 2, by - 4, 4, 10, 0x00_FF_FF_FF, Some(dirty_rect));

                // End Color Sliders
                let end_label_y = content_y + (font_height as isize + 9) + 65;
                font::draw_string(fb, content_x, end_label_y, "Gradient End", 0x00_FF_FF_FF, Some(dirty_rect));
                
                let end_color = DESKTOP_GRADIENT_END.load(Ordering::Relaxed);
                let re_val = (end_color >> 16) & 0xFF;
                let ge_val = (end_color >> 8) & 0xFF;
                let be_val = end_color & 0xFF;

                // Red End
                let rey = end_label_y + (font_height as isize + 9);
                font::draw_string(fb, content_x, rey - 4, "R", 0x00_FF_40_40, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, slider_x, rey, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                let rex_pos = slider_x + ((re_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, rex_pos - 2, rey - 4, 4, 10, 0x00_FF_FF_FF, Some(dirty_rect));

                // Green End
                let gey = rey + 20;
                font::draw_string(fb, content_x, gey - 4, "G", 0x00_40_FF_40, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, slider_x, gey, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                let gex_pos = slider_x + ((ge_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, gex_pos - 2, gey - 4, 4, 10, 0x00_FF_FF_FF, Some(dirty_rect));

                // Blue End
                let bey = gey + 20;
                font::draw_string(fb, content_x, bey - 4, "B", 0x00_40_40_FF, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, slider_x, bey, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                let bex_pos = slider_x + ((be_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, bex_pos - 2, bey - 4, 4, 10, 0x00_FF_FF_FF, Some(dirty_rect));

                // Color Preview
                let preview_y = content_y + (font_height as isize + 9) + 140;
                font::draw_string(fb, content_x, preview_y + 4, locale.label_preview(), 0x00_CC_CC_CC, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, content_x + 70, preview_y, 40, 16, start_color, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, content_x + 115, preview_y, 40, 16, end_color, Some(dirty_rect));
                crate::userspace::gui::draw_rect(fb, content_x + 70, preview_y, 40, 1, 0xFFFFFF, Some(dirty_rect)); // Border
                crate::userspace::gui::draw_rect(fb, content_x + 115, preview_y, 40, 1, 0xFFFFFF, Some(dirty_rect)); // Border

                // Presets
                let presets_y = content_y + (font_height as isize + 9) + 160;
                font::draw_string(fb, content_x, presets_y, locale.label_presets(), 0x00_FF_FF_FF, Some(dirty_rect));
                
                let btn_w = 80;
                let btn_h = font_height + 9;
                let mut btn = Button::new(content_x, presets_y + (font_height as isize + 10), btn_w, btn_h, locale.preset_nebula());
                btn.bg_color = 0x00_20_40_80; btn.text_color = 0xFFFFFF; btn.draw(fb, 0, 0, Some(dirty_rect));

                let mut btn = Button::new(content_x + 90, presets_y + (font_height as isize + 10), btn_w, btn_h, locale.preset_sunset());
                btn.bg_color = 0x00_80_40_20; btn.text_color = 0xFFFFFF; btn.draw(fb, 0, 0, Some(dirty_rect));

                let mut btn = Button::new(content_x + 180, presets_y + (font_height as isize + 10), btn_w, btn_h, locale.preset_midnight());
                btn.bg_color = 0x00_05_05_10; btn.text_color = 0xFFFFFF; btn.draw(fb, 0, 0, Some(dirty_rect));
            },
            Tab::Language => {
                font::draw_string(fb, content_x, content_y, locale.settings_tab_language(), 0x00_FF_FF_FF, Some(dirty_rect));

                let btn_en = Button::new(content_x, content_y + line_spacing, 160, font_height + 14, locale.lang_english());
                let btn_ja = Button::new(content_x, content_y + line_spacing * 2 + 10, 160, font_height + 14, locale.lang_japanese());
                
                btn_en.draw(fb, 0, 0, Some(dirty_rect));
                btn_ja.draw(fb, 0, 0, Some(dirty_rect));
            },
            Tab::Mouse => {
                font::draw_string(fb, content_x, content_y, locale.settings_tab_mouse(), 0x00_FF_FF_FF, Some(dirty_rect));
                
                let slider_y = content_y + line_spacing;
                font::draw_string(fb, content_x, slider_y, locale.label_mouse_speed(), 0x00_FF_FF_FF, Some(dirty_rect));
                
                let slider_x = content_x + 120;
                let slider_w = 150;
                let slider_h = 2;
                let sens = MOUSE_SENSITIVITY.load(Ordering::Relaxed);

                // Track
                crate::userspace::gui::draw_rect(fb, slider_x, slider_y + 6, slider_w, slider_h, 0x00_80_80_80, Some(dirty_rect));
                // Handle (Range 1 to 50, where 10 is 1.0x)
                let handle_x = slider_x + (((sens as usize - 1) * slider_w) / 49) as isize;
                crate::userspace::gui::draw_rect(fb, handle_x - 4, slider_y, 8, 14, 0x00_00_AA_FF, Some(dirty_rect));
            }
            Tab::Power => {
                font::draw_string(fb, content_x, content_y, "Power Settings", 0x00_FF_FF_FF, Some(dirty_rect));
                
                let ps_enabled = crate::kernel::cpu::POWER_SAVE.load(Ordering::Relaxed);
                let btn_text = if ps_enabled { "Power Saving: [Enabled]" } else { "Power Saving: [Disabled]" };
                let mut btn = Button::new(content_x, content_y + line_spacing, 240, font_height + 14, btn_text);
                if ps_enabled { btn.bg_color = 0x00_00_7A_CC; }
                
                btn.draw(fb, 0, 0, Some(dirty_rect));
                
                font::draw_string(fb, content_x, content_y + line_spacing * 2 + 10, "Reduces CPU wakeups by decreasing", 0x00_A0_A0_A0, Some(dirty_rect));
                font::draw_string(fb, content_x, content_y + line_spacing * 3, "timer frequency from 1000Hz to 100Hz.", 0x00_A0_A0_A0, Some(dirty_rect));
            }
            Tab::Display => {
                font::draw_string(fb, content_x, content_y, "Display Settings", 0x00_FF_FF_FF, Some(dirty_rect));
                
                let res_str = if let Some(info) = fb.info.as_ref() { format!("{}x{}", info.width, info.height) } else { "Unknown".into() };
                font::draw_string(fb, content_x, content_y + line_spacing, &format!("Resolution: {}", res_str), 0x00_CCCCCC, Some(dirty_rect));
                
                let btn_800 = Button::new(content_x, content_y + line_spacing * 2, 110, 25, "Set 800x600");
                let btn_1024 = Button::new(content_x + 120, content_y + line_spacing * 2, 110, 25, "Set 1024x768");
                btn_800.draw(fb, 0, 0, Some(dirty_rect));
                btn_1024.draw(fb, 0, 0, Some(dirty_rect));

                let mode = WALLPAPER_MODE.load(Ordering::Relaxed);
                let wall_text = match mode {
                    0 => "Wallpaper: [Gradient]",
                    1 => "Wallpaper: [Solid]",
                    _ => "Wallpaper: [Construction]",
                };
                let mut btn_wall = Button::new(content_x, content_y + line_spacing * 4, 240, 28, wall_text);
                if mode != 0 { btn_wall.bg_color = 0x00_44_44_44; }
                btn_wall.draw(fb, 0, 0, Some(dirty_rect));
            }
            Tab::Network => {
                font::draw_string(fb, content_x, content_y, "Network Connectivity", 0x00_FF_FF_FF, Some(dirty_rect));
                
                let conn_type = crate::kernel::net::CONNECTION_TYPE.load(Ordering::Relaxed);
                let status_str = match conn_type {
                    1 => "Ethernet (Connected)",
                    2 => "Wi-Fi (Connected)",
                    _ => "Disconnected",
                };
                
                font::draw_string(fb, content_x, content_y + line_spacing, &format!("Status: {}", status_str), 0x00_CCCCCC, Some(dirty_rect));
                
                if conn_type != 0 {
                    let strength = crate::kernel::net::NETWORK_SIGNAL_STRENGTH.load(Ordering::Relaxed);
                    font::draw_string(fb, content_x, content_y + line_spacing * 2, &format!("Signal: {}%", strength), 0x00_CCCCCC, Some(dirty_rect));
                    
                    let mac = crate::kernel::net::MAC_ADDRESS.lock();
                    let mac_str = format!("MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", 
                        mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]);
                    font::draw_string(fb, content_x, content_y + line_spacing * 3, &mac_str, 0x00_CCCCCC, Some(dirty_rect));
                    
                    font::draw_string(fb, content_x, content_y + line_spacing * 4, "IP Address: 192.168.1.42 (Static)", 0x00_AAAAAA, Some(dirty_rect));

                    // Network Stats
                    let rx = crate::kernel::net::RX_PACKETS.load(Ordering::Relaxed);
                    let tx = crate::kernel::net::TX_PACKETS.load(Ordering::Relaxed);
                    font::draw_string(fb, content_x, content_y + line_spacing * 6, &format!("Packets Received: {}", rx), 0x00_99FF99, Some(dirty_rect));
                    font::draw_string(fb, content_x, content_y + line_spacing * 7, &format!("Packets Sent: {}", tx), 0x00_FF9999, Some(dirty_rect));
                }
                
                let btn_y = if conn_type != 0 { line_spacing * 9 } else { line_spacing * 6 };
                let mut btn_refresh = Button::new(content_x, content_y + btn_y, 180, 25, "Rescan Hardware");
                btn_refresh.draw(fb, 0, 0, Some(dirty_rect));
            }
        }
    }
}

impl App for Settings {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        // Only redraw if there have been changes.
    
        self.draw_sidebar(fb, win, dirty_rect);
        self.draw_content(fb, win, dirty_rect);
        self.dirty.set(false);
    }

    fn handle_event(&mut self, event: &AppEvent, _win: &Window) -> Option<Rect> {
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let item_height = font_height as isize + 20;
        let content_pane_y_start = 15;
        let content_pane_x_start = SIDEBAR_WIDTH as isize + 20;
        let line_spacing = font_height as isize + 12;

        match event {
            AppEvent::Tick { tick_count } => {
                if self.current_tab == Tab::System && *tick_count > self.last_sys_update_tick.get() + 1000 {
                    self.last_sys_update_tick.set(*tick_count);
                    self.dirty.set(true);
                    return Some(_win.rect());
                }
                return None;
            }
            AppEvent::MouseClick { x, y, width: _width, .. } => {
                // 1. Sidebar selection
                if *x < SIDEBAR_WIDTH as isize {
                    let search_box_h = font_height as isize + 10;
                    if *y >= 5 && *y < 5 + search_box_h {
                        self.dirty.set(true);
                        // Check if click was on the clear button (X)
                        if !self.search_query.is_empty() && *x >= (SIDEBAR_WIDTH as isize - 25) {
                            self.search_query.clear();
                            self.search_focused = false;
                        } else {
                            self.search_focused = true;
                        }
                        return Some(_win.rect());
                    }
                    self.search_focused = false;

                    let list_start_y = search_box_h + 10;
                    if *y >= list_start_y {
                        let idx = ((*y - list_start_y) / item_height) as usize;
                        let indices = self.filtered_indices.borrow();
                        if idx < indices.len() {
                            let all_tabs = self.get_all_tabs();
                            self.current_tab = all_tabs[indices[idx]].0;
                            self.dirty.set(true);
                        }
                    }
                    return Some(_win.rect());
                }

                // 2. Content pane click
                if *y >= content_pane_y_start {
                    let rel_y = *y - content_pane_y_start;
                    let rel_x = *x - content_pane_x_start;

                    match self.current_tab {
                        Tab::Accessibility => {
                            let btn_hc_rect = Rect { x: 0, y: line_spacing, width: 240, height: (font_height + 14) };
                            let btn_lt_rect = Rect { x: 0, y: line_spacing * 2 + 10, width: 240, height: (font_height + 14) };
                            if btn_hc_rect.contains(rel_x, rel_y) {
                                HIGH_CONTRAST.fetch_xor(true, Ordering::Relaxed);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            } else if btn_lt_rect.contains(rel_x, rel_y) {
                                LARGE_TEXT.fetch_xor(true, Ordering::Relaxed);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            } else {
                                // Check Brightness Slider
                                let slider_y = line_spacing * 4;
                                if rel_y >= slider_y && rel_y <= slider_y + 14 && rel_x >= 100 && rel_x <= 250 {
                                    let new_level = ((rel_x - 100) * 100 / 150) as u8;
                                    crate::drivers::brightness::set_master_brightness(new_level);
                                }
                            }
                        }
                        Tab::Language => {
                            let btn_en_rect = Rect { x: 0, y: line_spacing, width: 160, height: font_height + 14 };
                            let btn_ja_rect = Rect { x: 0, y: line_spacing * 2 + 10, width: 160, height: font_height + 14 };
                            if btn_en_rect.contains(rel_x, rel_y) {
                                localisation::set_language(Language::English);
                            } else if btn_ja_rect.contains(rel_x, rel_y) {
                                localisation::set_language(Language::Japanese);
                            }
                            self.dirty.set(true);
                        }
                        Tab::Display => {
                            let btn_800_rect = Rect { x: 0, y: line_spacing * 2, width: 110, height: 25 };
                            let btn_1024_rect = Rect { x: 120, y: line_spacing * 2, width: 110, height: 25 };
                            let btn_wall_rect = Rect { x: 0, y: line_spacing * 4, width: 240, height: 28 };

                            if btn_800_rect.contains(rel_x, rel_y) {
                                crate::serial_println!("[Display] Resolution Change: 800x600 requested (actual)");
                                // Hardware hook placeholder: crate::drivers::framebuffer::set_mode(800, 600);
                            } else if btn_1024_rect.contains(rel_x, rel_y) {
                                crate::serial_println!("[Display] Resolution Change: 1024x768 requested (actual)");
                            } else if btn_wall_rect.contains(rel_x, rel_y) {
                                let next = (WALLPAPER_MODE.load(Ordering::Relaxed) + 1) % 3;
                                WALLPAPER_MODE.store(next, Ordering::Relaxed);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                                self.dirty.set(true);
                            }
                        }
                        Tab::Power => {
                            let btn_ps_rect = Rect { x: 0, y: line_spacing, width: 240, height: font_height + 14 };
                            if btn_ps_rect.contains(rel_x, rel_y) {
                                let new_state = !crate::kernel::cpu::POWER_SAVE.load(Ordering::Relaxed);
                                crate::kernel::cpu::POWER_SAVE.store(new_state, Ordering::Relaxed);
                                crate::drivers::pit::set_frequency(if new_state { 100 } else { 1000 });
                                self.dirty.set(true);
                            }
                        }
                        Tab::Network => {
                            let conn_type = crate::kernel::net::CONNECTION_TYPE.load(Ordering::Relaxed);
                            let btn_y = if conn_type != 0 { line_spacing * 9 } else { line_spacing * 6 };
                            let btn_refresh_rect = Rect { x: 0, y: btn_y, width: 180, height: 25 };
                            if btn_refresh_rect.contains(rel_x, rel_y) {
                                crate::kernel::pci::rescan_bus();
                            }
                        }
                        Tab::Theme => {
                            let presets_y = (font_height as isize + 9) + 160;
                            let btn_w = 80;
                            let btn_h = font_height + 9;
                            let btn_neb = Rect { x: 0, y: presets_y + (font_height as isize + 10), width: btn_w, height: btn_h };
                            let btn_sun = Rect { x: 90, y: presets_y + (font_height as isize + 10), width: btn_w, height: btn_h };
                            let btn_mid = Rect { x: 180, y: presets_y + (font_height as isize + 10), width: btn_w, height: btn_h };

                            if btn_neb.contains(rel_x, rel_y) {
                                DESKTOP_GRADIENT_START.store(0x00_10_20_40, Ordering::Relaxed);
                                DESKTOP_GRADIENT_END.store(0x00_50_80_B0, Ordering::Relaxed);
                                self.dirty.set(true);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            } else if btn_sun.contains(rel_x, rel_y) {
                                DESKTOP_GRADIENT_START.store(0x00_40_10_10, Ordering::Relaxed);
                                DESKTOP_GRADIENT_END.store(0x00_FF_80_40, Ordering::Relaxed);
                                self.dirty.set(true);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            } else if btn_mid.contains(rel_x, rel_y) {
                                DESKTOP_GRADIENT_START.store(0x00_02_02_05, Ordering::Relaxed);
                                DESKTOP_GRADIENT_END.store(0x00_10_15_25, Ordering::Relaxed);
                                self.dirty.set(true);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            }
                        }
                        Tab::Mouse => {
                            let slider_y = line_spacing;
                            if rel_y >= slider_y && rel_y <= slider_y + 14 && rel_x >= 120 && rel_x <= 270 {
                                let new_sens = 1 + ((rel_x - 120) * 49 / 150) as u32;
                                MOUSE_SENSITIVITY.store(new_sens, Ordering::Relaxed);
                                self.dirty.set(true);
                            }
                        }
                        _ => {}
                    }
                }
            }
            AppEvent::KeyPress { key } => {
                let mut handled = true;
                match *key {
                    '\u{B5}' => { // Up Arrow
                        let cur = self.highlighted_index.get();
                        if cur > 0 {
                            self.highlighted_index.set(cur - 1);
                            self.dirty.set(true);
                        }
                    }
                    '\u{B6}' => { // Down Arrow
                        let indices = self.filtered_indices.borrow();
                        let cur = self.highlighted_index.get();
                        if cur + 1 < indices.len() {
                            self.highlighted_index.set(cur + 1);
                            self.dirty.set(true);
                        }
                    }
                    '\n' => { // Enter
                        let indices = self.filtered_indices.borrow();
                        let idx = self.highlighted_index.get();
                        if idx < indices.len() {
                            let all_tabs = self.get_all_tabs();
                            self.current_tab = all_tabs[indices[idx]].0;
                            self.dirty.set(true);
                        }
                        self.search_focused = false;
                    }
                    '\x08' if self.search_focused => { self.search_query.pop(); self.dirty.set(true); }
                    '\x1B' => { self.search_focused = false; self.dirty.set(true); }
                    c if self.search_focused && !c.is_control() => { self.search_query.push(c); self.dirty.set(true); }
                    _ => { handled = false; }
                }
                if handled {
                    return Some(_win.rect());
                }
            }
            AppEvent::MouseMove { x, y, .. } => {
                if *y >= content_pane_y_start {
                    let rel_y = *y - content_pane_y_start;
                    let rel_x = *x - content_pane_x_start;
                    if self.current_tab == Tab::Theme {
                    let slider_x_rel = 30;
                    let slider_w = 180;
                    let r_start_y = font_height as isize + 9;

                    let r_rect = Rect { x: slider_x_rel, y: r_start_y - 5, width: slider_w, height: 12 };
                    let g_rect = Rect { x: slider_x_rel, y: r_start_y + 20 - 5, width: slider_w, height: 12 };
                    let b_rect = Rect { x: slider_x_rel, y: r_start_y + 40 - 5, width: slider_w, height: 12 };

                    let current_start = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
                    let mut rs = (current_start >> 16) & 0xFF;
                    let mut gs = (current_start >> 8) & 0xFF;
                    let mut bs = current_start & 0xFF;
                    let mut changed = false;

                    if r_rect.contains(rel_x, rel_y) { rs = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed = true; }
                    else if g_rect.contains(rel_x, rel_y) { gs = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed = true; }
                    else if b_rect.contains(rel_x, rel_y) { bs = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed = true; }

                    if changed {
                        DESKTOP_GRADIENT_START.store((rs << 16) | (gs << 8) | bs, Ordering::Relaxed);
                        FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                    }

                    let mut changed_end = false;
                    let r_end_y = r_start_y + 85;
                    let re_rect = Rect { x: slider_x_rel, y: r_end_y - 5, width: slider_w, height: 12 };
                    let ge_rect = Rect { x: slider_x_rel, y: r_end_y + 20 - 5, width: slider_w, height: 12 };
                    let be_rect = Rect { x: slider_x_rel, y: r_end_y + 40 - 5, width: slider_w, height: 12 };

                    let current_end = DESKTOP_GRADIENT_END.load(Ordering::Relaxed);
                    let mut re = (current_end >> 16) & 0xFF;
                    let mut ge = (current_end >> 8) & 0xFF;
                    let mut be = current_end & 0xFF;

                    if re_rect.contains(rel_x, rel_y) { re = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed_end = true; }
                    else if ge_rect.contains(rel_x, rel_y) { ge = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed_end = true; }
                    else if be_rect.contains(rel_x, rel_y) { be = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed_end = true; }

                    if changed_end {
                        DESKTOP_GRADIENT_END.store((re << 16) | (ge << 8) | be, Ordering::Relaxed);
                        FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                    }
                    } else if self.current_tab == Tab::Accessibility {
                        let slider_y = line_spacing * 4;
                        // Allow dragging if mouse is roughly near the slider height
                        if rel_y >= slider_y - 10 && rel_y <= slider_y + 24 && rel_x >= 100 && rel_x <= 250 {
                            let new_level = ((rel_x - 100) * 100 / 150) as u8;
                            crate::drivers::brightness::set_master_brightness(new_level);
                        }
                    } else if self.current_tab == Tab::Mouse {
                        let slider_y = line_spacing;
                        if rel_y >= slider_y - 10 && rel_y <= slider_y + 24 && rel_x >= 120 && rel_x <= 270 {
                            let new_sens = 1 + ((rel_x - 120) * 49 / 150) as u32;
                            MOUSE_SENSITIVITY.store(new_sens, Ordering::Relaxed);
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new((*self).clone())
    }
}