use crate::drivers::framebuffer;
use crate::userspace::gui::{font, Window, button::Button, rect::Rect};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use core::sync::atomic::Ordering;
use crate::userspace::gui::{DESKTOP_GRADIENT_START, DESKTOP_GRADIENT_END, FULL_REDRAW_REQUESTED, HIGH_CONTRAST, LARGE_TEXT, MOUSE_SENSITIVITY};
use crate::userspace::localisation::{self, Language};
use crate::kernel::cpu::CPU_BRAND;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    System,
    Accessibility,
    Theme,
    Language,
    Mouse,
}

#[derive(Clone)]
pub struct Settings {
    current_tab: Tab,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            current_tab: Tab::System,
        }
    }

    fn draw_tabs(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 6;
        let bar_height = font_height + 9;
        let start_y = win.y + title_height as isize; // Start just below title bar
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        
        let tab_labels = [
            (Tab::System, locale.settings_tab_system()),
            (Tab::Accessibility, locale.settings_tab_a11y()),
            (Tab::Theme, locale.settings_tab_theme()),
            (Tab::Language, locale.settings_tab_language()),
            (Tab::Mouse, locale.settings_tab_mouse()),
        ];

        let count = tab_labels.len();
        let tab_width = win.width / count;
        
        for (i, (tab, label)) in tab_labels.iter().enumerate() {
            let x = win.x + (i * tab_width) as isize;            
            // Skip drawing tab if not in dirty rect
            if !dirty_rect.intersects(&Rect { x, y: start_y, width: tab_width, height: bar_height }) { continue; }

            let is_active = self.current_tab == *tab;
            let bg_color = if is_active { 0x00_50_50_60 } else { 0x00_30_30_30 };
            
            // Optimization: Draw tabs using fast-path rects instead of expensive rounded Buttons
            crate::userspace::gui::draw_rect(fb, x, start_y, tab_width, bar_height, bg_color, Some(win.rect()));
            let tx = x + (tab_width as isize - font::string_width(label) as isize) / 2;
            let ty = start_y + (bar_height as isize - 16) / 2;
            font::draw_string(fb, tx, ty, label, 0x00_FFFFFF, Some(win.rect()));

        if is_active {
                crate::userspace::gui::draw_rect(fb, x, start_y + bar_height as isize - 2, tab_width, 2, 0x00_00_AA_FF, None);
            }
        }
    }

    fn draw_content(&self, fb: &mut framebuffer::Framebuffer, win: &Window, _dirty_rect: Rect) {
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 6;
        let content_y = win.y + title_height as isize + (font_height as isize + 9) + 5; // Title + TabBar + Padding
        let content_x = win.x + 8;
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();

        match self.current_tab {
            Tab::System => {
                font::draw_string(fb, content_x, content_y, locale.app_system_info(), 0x00_FF_FF_FF, None);
                let v_str = format!("{} {}", locale.info_version(), crate::kernel::VERSION);
                font::draw_string(fb, content_x, content_y + (font_height as isize + 4), &v_str, 0x00_CC_CC_CC, None);
                
                // CPU Info
                let brand_guard = CPU_BRAND.lock();
                let cpu_name = brand_guard.as_deref().unwrap_or("Unknown CPU");
                let cores = crate::kernel::CPU_CORES.load(Ordering::Relaxed);
                let core_str = if cores == 1 { "Core" } else { "Cores" };
                
                // Truncate CPU string if too long for display
                let cpu_display = if cpu_name.len() > 30 { &cpu_name[..30] } else { cpu_name };
                
                font::draw_string(fb, content_x, content_y + (font_height as isize + 4) * 2, &format!("Processor: {}", cpu_display), 0x00_CC_CC_CC, None);
                font::draw_string(fb, content_x, content_y + (font_height as isize + 4) * 3, &format!("Cores: {} {}", cores, core_str), 0x00_CC_CC_CC, None);

                // Resolution
                let res_str = if let Some(info) = fb.info.as_ref() {
                    format!("{} {}x{}", locale.info_resolution(), info.width, info.height)
                } else {
                    format!("{} Unknown", locale.info_resolution())
                };
                font::draw_string(fb, content_x, content_y + (font_height as isize + 4) * 4, &res_str, 0x00_CC_CC_CC, None);

                // Memory
                let mem_bytes = crate::kernel::TOTAL_MEMORY.load(Ordering::Relaxed);
                let mem_mb = mem_bytes / 1024 / 1024;
                let mem_str = format!("{} {} MB", locale.info_memory(), mem_mb);
                font::draw_string(fb, content_x, content_y + (font_height as isize + 4) * 5, &mem_str, 0x00_CC_CC_CC, None);

                // Uptime
                let total_seconds = crate::kernel::process::TICKS.load(Ordering::Relaxed) / 1000;
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;
                let time_str = format!("{} {:02}:{:02}:{:02}", locale.info_uptime(), hours, minutes, seconds);
                font::draw_string(fb, content_x, content_y + (font_height as isize + 4) * 6, &time_str, 0x00_CC_CC_CC, None);

            },
            Tab::Accessibility => {
                font::draw_string(fb, content_x, content_y, locale.settings_tab_a11y(), 0x00_FF_FF_FF, None);
                
                let hc = HIGH_CONTRAST.load(Ordering::Relaxed);
                let lt = LARGE_TEXT.load(Ordering::Relaxed);

                let btn_hc_text = format!("{}  {}", if hc { "[x]" } else { "[ ]" }, locale.option_high_contrast());
                
                let btn_hc = Button { rect: Rect { x: content_x, y: content_y + (font_height as isize + 4), width: 280, height: (font_height + 4) }, text: btn_hc_text, bg_color: 0x00_30_30_30, text_color: 0xFFFFFF };
                let btn_lt_text = format!("{}  {}", if lt { "[x]" } else { "[ ]" }, locale.option_large_text());

                let btn_lt = Button { rect: Rect { x: content_x, y: content_y + (font_height as isize + 4) * 2 + 5, width: 280, height: (font_height + 4) }, text: btn_lt_text, bg_color: 0x00_30_30_30, text_color: 0xFFFFFF };
                btn_hc.draw(fb, 0, 0, None);
                btn_lt.draw(fb, 0, 0, None);

                // Brightness Slider
                let slider_y = content_y + (font_height as isize + 4) * 3 + 20;
                font::draw_string(fb, content_x, slider_y, locale.label_brightness(), 0x00_FF_FF_FF, None);
                
                let slider_x = content_x + 100;
                let slider_w = 150;
                let slider_h = 2;
                let level = crate::drivers::brightness::BRIGHTNESS_LEVEL.load(Ordering::Relaxed);

                // Track
                crate::userspace::gui::draw_rect(fb, slider_x, slider_y + 6, slider_w, slider_h, 0x00_80_80_80, None);
                // Handle
                let handle_x = slider_x + ((level as usize * slider_w) / 100) as isize;
                crate::userspace::gui::draw_rect(fb, handle_x - 4, slider_y, 8, 14, 0x00_FF_CC_00, None);
            },
            Tab::Theme => {
                font::draw_string(fb, content_x, content_y, locale.label_bg_color(), 0x00_FF_FF_FF, None);
                
                let start_color = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
                let r_val = (start_color >> 16) & 0xFF;
                let g_val = (start_color >> 8) & 0xFF;
                let b_val = start_color & 0xFF;

                let slider_x = content_x + 30;
                let slider_w = 150;
                let slider_h = 2; // Track height

                // Red Slider
                let ry = content_y + (font_height as isize + 9);
                font::draw_string(fb, content_x, ry - 4, "R", 0x00_FF_40_40, None);
                crate::userspace::gui::draw_rect(fb, slider_x, ry, slider_w, slider_h, 0x00_80_80_80, None);
                let rx_pos = slider_x + ((r_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, rx_pos - 2, ry - 4, 4, 10, 0x00_FF_FF_FF, None);

                // Green Slider
                let gy = content_y + (font_height as isize + 9) + 20;
                font::draw_string(fb, content_x, gy - 4, "G", 0x00_40_FF_40, None);
                crate::userspace::gui::draw_rect(fb, slider_x, gy, slider_w, slider_h, 0x00_80_80_80, None);
                let gx_pos = slider_x + ((g_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, gx_pos - 2, gy - 4, 4, 10, 0x00_FF_FF_FF, None);

                // Blue Slider
                let by = content_y + (font_height as isize + 9) + 40;
                font::draw_string(fb, content_x, by - 4, "B", 0x00_40_40_FF, None);
                crate::userspace::gui::draw_rect(fb, slider_x, by, slider_w, slider_h, 0x00_80_80_80, None);
                let bx_pos = slider_x + ((b_val as usize * slider_w) / 255) as isize;
                crate::userspace::gui::draw_rect(fb, bx_pos - 2, by - 4, 4, 10, 0x00_FF_FF_FF, None);

                // Color Preview
                let preview_y = content_y + (font_height as isize + 9) + 60;
                font::draw_string(fb, content_x, preview_y + 4, locale.label_preview(), 0x00_CC_CC_CC, None);
                crate::userspace::gui::draw_rect(fb, content_x + 70, preview_y, 40, 16, start_color, None);
                crate::userspace::gui::draw_rect(fb, content_x + 70, preview_y, 40, 1, 0xFFFFFF, None); // Border

                // Presets
                let presets_y = content_y + (font_height as isize + 9) + 90;
                font::draw_string(fb, content_x, presets_y, locale.label_presets(), 0x00_FF_FF_FF, None);
                
                let btn_w = 80;
                let btn_h = font_height + 9;
                let mut btn = Button::new(content_x, presets_y + (font_height as isize + 4), btn_w, btn_h, locale.preset_nebula());
                btn.bg_color = 0x00_20_40_80; btn.text_color = 0xFFFFFF; btn.draw(fb, 0, 0, None);

                let mut btn = Button::new(content_x + 90, presets_y + (font_height as isize + 4), btn_w, btn_h, locale.preset_sunset());
                btn.bg_color = 0x00_80_40_20; btn.text_color = 0xFFFFFF; btn.draw(fb, 0, 0, None);
            },
            Tab::Language => {
                font::draw_string(fb, content_x, content_y, locale.settings_tab_language(), 0x00_FF_FF_FF, None);

                let btn_en = Button { rect: Rect { x: content_x, y: content_y + (font_height as isize + 4), width: 120, height: font_height + 9 }, text: locale.lang_english().to_string(), bg_color: 0x00_30_30_30, text_color: 0xFFFFFF };
                let btn_ja = Button { rect: Rect { x: content_x, y: content_y + (font_height as isize + 4) * 2 + 5, width: 120, height: font_height + 9 }, text: locale.lang_japanese().to_string(), bg_color: 0x00_30_30_30, text_color: 0xFFFFFF };
                btn_en.draw(fb, 0, 0, None);
                btn_ja.draw(fb, 0, 0, None);
            },
            Tab::Mouse => {
                font::draw_string(fb, content_x, content_y, locale.settings_tab_mouse(), 0x00_FF_FF_FF, None);
                
                let slider_y = content_y + (font_height as isize + 4) + 20;
                font::draw_string(fb, content_x, slider_y, locale.label_mouse_speed(), 0x00_FF_FF_FF, None);
                
                let slider_x = content_x + 120;
                let slider_w = 150;
                let slider_h = 2;
                let sens = MOUSE_SENSITIVITY.load(Ordering::Relaxed);

                // Track
                crate::userspace::gui::draw_rect(fb, slider_x, slider_y + 6, slider_w, slider_h, 0x00_80_80_80, None);
                // Handle (Range 1 to 50, where 10 is 1.0x)
                let handle_x = slider_x + (((sens as usize - 1) * slider_w) / 49) as isize;
                crate::userspace::gui::draw_rect(fb, handle_x - 4, slider_y, 8, 14, 0x00_00_AA_FF, None);
            }
        }
    }
}

impl App for Settings {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        // Only redraw if there have been changes.
    
        self.draw_tabs(fb, win, dirty_rect);
        self.draw_content(fb, win, dirty_rect);
    }

    fn handle_event(&mut self, event: &AppEvent) {
        let font_height = if LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let bar_height = font_height + 9;
        let content_pane_y_start = bar_height as isize + 5;
        let content_pane_x_start = 8;

        match event {
            AppEvent::MouseClick { x, y, width, .. } => {
                // 1. Tab selection
                if *y >= 0 && *y < bar_height as isize {
                    let tab_width = *width / 5;
                    if *x < tab_width as isize { self.current_tab = Tab::System; }
                    else if *x < (tab_width * 2) as isize { self.current_tab = Tab::Accessibility; }
                    else if *x < (tab_width * 3) as isize { self.current_tab = Tab::Theme; }
                    else if *x < (tab_width * 4) as isize { self.current_tab = Tab::Language; }
                    else { self.current_tab = Tab::Mouse; }
                    return;
                }

                // 2. Content pane click
                if *y >= content_pane_y_start {
                    let rel_y = *y - content_pane_y_start;
                    let rel_x = *x - content_pane_x_start;

                    match self.current_tab {
                        Tab::Accessibility => {
                            let btn_hc_rect = Rect { x: 0, y: (font_height as isize + 4), width: 280, height: (font_height + 4) };
                            let btn_lt_rect = Rect { x: 0, y: (font_height as isize + 4) * 2 + 5, width: 280, height: (font_height + 4) };
                            if btn_hc_rect.contains(rel_x, rel_y) {
                                HIGH_CONTRAST.fetch_xor(true, Ordering::Relaxed);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            } else if btn_lt_rect.contains(rel_x, rel_y) {
                                LARGE_TEXT.fetch_xor(true, Ordering::Relaxed);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            } else {
                                // Check Brightness Slider
                                let slider_y = (font_height as isize + 4) * 3 + 20;
                                if rel_y >= slider_y && rel_y <= slider_y + 14 && rel_x >= 100 && rel_x <= 250 {
                                    let new_level = ((rel_x - 100) * 100 / 150) as u8;
                                    crate::drivers::brightness::set_master_brightness(new_level);
                                }
                            }
                        }
                        Tab::Language => {
                            let btn_en_rect = Rect { x: 0, y: (font_height as isize + 4), width: 120, height: font_height + 9 };
                            let btn_ja_rect = Rect { x: 0, y: (font_height as isize + 4) * 2 + 5, width: 120, height: font_height + 9 };
                            if btn_en_rect.contains(rel_x, rel_y) {
                                localisation::set_language(Language::English);
                            } else if btn_ja_rect.contains(rel_x, rel_y) {
                                localisation::set_language(Language::Japanese);
                            }
                        }
                        Tab::Theme => {
                            let presets_y = (font_height as isize + 9) + 90;
                            let btn_w = 80;
                            let btn_h = font_height + 9;
                            let btn_neb = Rect { x: 0, y: presets_y + (font_height as isize + 4), width: btn_w, height: btn_h };
                            let btn_sun = Rect { x: 90, y: presets_y + (font_height as isize + 4), width: btn_w, height: btn_h };

                            if btn_neb.contains(rel_x, rel_y) {
                                DESKTOP_GRADIENT_START.store(0x00_10_20_40, Ordering::Relaxed);
                                DESKTOP_GRADIENT_END.store(0x00_50_80_B0, Ordering::Relaxed);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            } else if btn_sun.contains(rel_x, rel_y) {
                                DESKTOP_GRADIENT_START.store(0x00_40_10_10, Ordering::Relaxed);
                                DESKTOP_GRADIENT_END.store(0x00_FF_80_40, Ordering::Relaxed);
                                FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                            }
                        }
                        Tab::Mouse => {
                            let slider_y = (font_height as isize + 4) + 20;
                            if rel_y >= slider_y && rel_y <= slider_y + 14 && rel_x >= 120 && rel_x <= 270 {
                                let new_sens = 1 + ((rel_x - 120) * 49 / 150) as u32;
                                MOUSE_SENSITIVITY.store(new_sens, Ordering::Relaxed);
                            }
                        }
                        _ => {}
                    }
                }
            }
            AppEvent::MouseMove { x, y, .. } => {
                if *y >= content_pane_y_start {
                    let rel_y = *y - content_pane_y_start;
                    let rel_x = *x - content_pane_x_start;
                    if self.current_tab == Tab::Theme {
                    let slider_x_rel = 30;
                    let slider_w = 150;

                    let r_rect = Rect { x: slider_x_rel, y: (font_height as isize + 9) - 5, width: slider_w, height: 12 };
                    let g_rect = Rect { x: slider_x_rel, y: (font_height as isize + 9) + 20 - 5, width: slider_w, height: 12 };
                    let b_rect = Rect { x: slider_x_rel, y: (font_height as isize + 9) + 40 - 5, width: slider_w, height: 12 };

                    let current_color = DESKTOP_GRADIENT_START.load(Ordering::Relaxed);
                    let mut r = (current_color >> 16) & 0xFF;
                    let mut g = (current_color >> 8) & 0xFF;
                    let mut b = current_color & 0xFF;
                    let mut changed = false;

                    if r_rect.contains(rel_x, rel_y) { r = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed = true; }
                    else if g_rect.contains(rel_x, rel_y) { g = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed = true; }
                    else if b_rect.contains(rel_x, rel_y) { b = (((rel_x - slider_x_rel) * 255) / slider_w as isize) as u32; changed = true; }

                    if changed {
                        let new_start = (r << 16) | (g << 8) | b;
                        let new_end = (r.saturating_add(0x30).min(255) << 16) | (g.saturating_add(0x30).min(255) << 8) | (b.saturating_add(0x40).min(255));
                        DESKTOP_GRADIENT_START.store(new_start, Ordering::Relaxed);
                        DESKTOP_GRADIENT_END.store(new_end, Ordering::Relaxed);
                        FULL_REDRAW_REQUESTED.store(true, Ordering::Relaxed);
                    }
                    } else if self.current_tab == Tab::Accessibility {
                        let slider_y = (font_height as isize + 4) * 3 + 20;
                        // Allow dragging if mouse is roughly near the slider height
                        if rel_y >= slider_y - 10 && rel_y <= slider_y + 24 && rel_x >= 100 && rel_x <= 250 {
                            let new_level = ((rel_x - 100) * 100 / 150) as u8;
                            crate::drivers::brightness::set_master_brightness(new_level);
                        }
                    } else if self.current_tab == Tab::Mouse {
                        let slider_y = (font_height as isize + 4) + 20;
                        if rel_y >= slider_y - 10 && rel_y <= slider_y + 24 && rel_x >= 120 && rel_x <= 270 {
                            let new_sens = 1 + ((rel_x - 120) * 49 / 150) as u32;
                            MOUSE_SENSITIVITY.store(new_sens, Ordering::Relaxed);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new((*self).clone())
    }
}