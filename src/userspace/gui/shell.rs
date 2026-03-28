use crate::drivers::framebuffer;
use crate::drivers::rtc::{CURRENT_DATETIME};
use crate::userspace::gui::{font, Rect, Button, HIGH_CONTRAST, draw_rect, Window};
use crate::userspace::localisation;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

/// Encapsulates Shell components like the Start Menu and Taskbar
pub struct Shell {
    pub start_menu: StartMenu,
    pub taskbar_height: usize,
}

impl Shell {
    pub fn draw(&self, fb: &mut framebuffer::Framebuffer, windows: &[Window], mouse_x: isize, mouse_y: isize, clip: Rect, start_menu_open: bool, context_menu: Option<(isize, isize)>) {
        self.draw_taskbar(fb, windows, mouse_x, mouse_y, clip, start_menu_open);
        if start_menu_open {
            self.draw_start_menu(fb, mouse_x, mouse_y, clip);
        }
        if let Some((x, y)) = context_menu {
            self.draw_context_menu(fb, x, y, mouse_x, mouse_y, clip);
        }
    }

    fn draw_taskbar(&self, fb: &mut framebuffer::Framebuffer, windows: &[Window], mouse_x: isize, mouse_y: isize, clip: Rect, start_menu_open: bool) {
        let (width, height) = if let Some(ref info) = fb.info { (info.width, info.height) } else { return };
        let taskbar_y = (height - self.taskbar_height) as isize;
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        
        let (bg_color, border_color) = if high_contrast { (0x00_00_00_00, 0x00_FF_FF_FF) } else { (0x00_28_28_28, 0x00_50_50_50) };
        
        draw_rect(fb, 0, taskbar_y, width, self.taskbar_height, bg_color, Some(clip));
        draw_rect(fb, 0, taskbar_y, width, 1, border_color, Some(clip));

        let start_button_width = 120;
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        let mut start_button = Button::new(0, taskbar_y, start_button_width, self.taskbar_height, locale.start());
        start_button.bg_color = if start_menu_open { 0x00_40_40_40 } else { 0x00_30_30_30 };
        start_button.text_color = 0x00_FF_FF_FF;
        start_button.draw(fb, mouse_x, mouse_y, Some(clip));

        let mut x_offset = start_button_width as isize + 10;
        let button_width = 100;
        for win in windows {
            let title = if font::string_width(win.title) > button_width - 8 {
                let mut current_width = 0;
                let mut end_char_idx = 0;
                for (i, c) in win.title.char_indices() {
                    let char_width = if c.is_ascii() { 8 } else { 16 };
                    if current_width + char_width > button_width - 16 { break; }
                    current_width += char_width;
                    end_char_idx = i + c.len_utf8();
                }
                alloc::format!("{}...", &win.title[..end_char_idx])
            } else { win.title.to_string() };

            let mut button = Button::new(x_offset, taskbar_y + 2, button_width, self.taskbar_height - 4, title.as_str());
            button.bg_color = if win.minimized { 0x00_30_30_30 } else { 0x00_50_50_50 };
            button.text_color = 0x00_FF_FF_FF;
            button.draw(fb, mouse_x, mouse_y, Some(clip));
            x_offset += button_width as isize + 5;
        }
        self.draw_clock_on_taskbar(fb, clip);
    }

    fn draw_clock_on_taskbar(&self, fb: &mut framebuffer::Framebuffer, clip: Rect) {
        if let Some((width, height)) = fb.info.as_ref().map(|i| (i.width, i.height)) {
            let taskbar_y = (height - self.taskbar_height) as isize;
            let bg_color = if HIGH_CONTRAST.load(Ordering::Relaxed) { 0x00_00_00_00 } else { 0x00_28_28_28 };
            let time_area_width = (8 * 8) + 20;
            let time_x_start = width as isize - time_area_width;
            draw_rect(fb, time_x_start, taskbar_y + 1, time_area_width as usize, self.taskbar_height - 1, bg_color, Some(clip));

            let time = CURRENT_DATETIME.lock();
            let mut time_str_bytes = [b' '; 8];
            time_str_bytes[0] = b'0' + (time.hour / 10); time_str_bytes[1] = b'0' + (time.hour % 10);
            time_str_bytes[2] = b':';
            time_str_bytes[3] = b'0' + (time.minute / 10); time_str_bytes[4] = b'0' + (time.minute % 10);
            time_str_bytes[5] = b':';
            time_str_bytes[6] = b'0' + (time.second / 10); time_str_bytes[7] = b'0' + (time.second % 10);
            let time_s = core::str::from_utf8(&time_str_bytes).unwrap_or("??:??:??");
            font::draw_string(fb, width as isize - (8 * 8) - 10, taskbar_y + 12, time_s, 0x00_FF_FF_FF, Some(clip));
        }
    }

    pub fn draw_start_menu(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let height = if let Some(ref info) = fb.info { info.height } else { return };
        let menu_y = (height - self.taskbar_height - self.start_menu.height) as isize;
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_C0_C0_C0 };
        let border_color = if high_contrast { 0x00_FF_FF_FF } else { 0x00_C0_C0_C0 };

        draw_rect(fb, 0, menu_y, self.start_menu.width, self.start_menu.height, bg_color, Some(clip));
        if high_contrast {
            draw_rect(fb, 0, menu_y, self.start_menu.width, 1, border_color, Some(clip));
            draw_rect(fb, 0, menu_y, 1, self.start_menu.height, border_color, Some(clip));
            draw_rect(fb, self.start_menu.width as isize - 1, menu_y, 1, self.start_menu.height, border_color, Some(clip));
            draw_rect(fb, 0, menu_y + self.start_menu.height as isize - 1, self.start_menu.width, 1, border_color, Some(clip));
        }

        let item_width = self.start_menu.width - 20;
        let search_y = menu_y + 10;
        draw_rect(fb, 10, search_y, item_width, 25, 0x00_FFFFFF, Some(clip));
        font::draw_string(fb, 15, search_y + 5, self.start_menu.search_query.as_str(), 0x00_00_00_00, Some(clip));

        let app_list = self.get_start_menu_data();
        let mut draw_y = menu_y + 45;
        for (i, &real_idx) in self.start_menu.filtered_indices.iter().enumerate() {
            let (text, color) = app_list[real_idx];
            let mut btn = Button::new(10, draw_y, item_width, 30, text);
            btn.bg_color = if i == self.start_menu.selected_index { 0x00_40_60_90 } else { color };
            btn.draw(fb, mouse_x, mouse_y, Some(clip));
            draw_y += 35;
        }
    }

    pub fn draw_context_menu(&self, fb: &mut framebuffer::Framebuffer, cx: isize, cy: isize, mouse_x: isize, mouse_y: isize, clip: Rect) {
        let width = 150; let height = 70;
        let high_contrast = HIGH_CONTRAST.load(Ordering::Relaxed);
        let bg_color = if high_contrast { 0x00_00_00_00 } else { 0x00_C0_C0_C0 };
        draw_rect(fb, cx, cy, width, height, bg_color, Some(clip));
        
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        Button::new(cx + 5, cy + 5, width - 10, 25, locale.ctx_refresh()).draw(fb, mouse_x, mouse_y, Some(clip));
        Button::new(cx + 5, cy + 35, width - 10, 25, locale.ctx_properties()).draw(fb, mouse_x, mouse_y, Some(clip));
    }

    pub fn draw_task_switcher(&self, fb: &mut framebuffer::Framebuffer, clip: Rect, z_order: &[usize], windows: &[Window], selected: usize) {
        if let Some(info) = fb.info.as_ref() {
            let width = 400; let height = 100;
            let x = (info.width / 2) as isize - (width / 2) as isize;
            let y = (info.height / 2) as isize - (height / 2) as isize;
            draw_rect(fb, x, y, width, height, 0x00_30_30_30, Some(clip));
            font::draw_string(fb, x + 10, y + 10, "Task Switcher", 0x00_FF_FF_FF, Some(clip));

            for (i, &win_id) in z_order.iter().rev().enumerate() {
                if i >= 6 { break; }
                let win = windows.iter().find(|w| w.id == win_id).unwrap();
                let item_x = x + 20 + (i as isize * 50);
                let color = if i == selected { 0x00_50_50_90 } else { 0x00_40_40_40 };
                draw_rect(fb, item_x, y + 40, 40, 40, color, Some(clip));
                font::draw_char(fb, item_x + 12, y + 52, win.title.chars().next().unwrap_or('?'), 0x00_FFFFFF, Some(clip));
            }
        }
    }

    pub fn get_start_menu_data(&self) -> Vec<(&'static str, u32)> {
        let locale_guard = localisation::CURRENT_LOCALE.lock();
        let locale = locale_guard.as_ref().unwrap();
        alloc::vec![
            (locale.app_text_editor(), 0x00_C0_C0_C0),
            (locale.app_calculator(), 0x00_C0_C0_C0),
            (locale.app_paint(), 0x00_C0_C0_C0),
            (locale.app_settings(), 0x00_C0_C0_C0),
            (locale.app_terminal(), 0x00_C0_C0_C0),
            ("Task Manager", 0x00_C0_C0_C0),
            ("Partitions", 0x00_C0_C0_C0),
            (locale.btn_reboot(), 0x00_FF_A0_40),
            (locale.btn_shutdown(), 0x00_FF_60_60),
        ]
    }
}

pub struct StartMenu {
    pub is_open: bool,
    pub selected_index: usize,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub width: usize,
    pub height: usize,
}

impl StartMenu {
    pub const fn new() -> Self {
        Self {
            is_open: false,
            selected_index: 0,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            width: 200,
            height: 385,
        }
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.search_query.clear();
            self.selected_index = 0;
        }
    }

    pub fn rect(&self, screen_height: isize, taskbar_height: usize) -> Rect {
        Rect {
            x: 0,
            y: screen_height - taskbar_height as isize - self.height as isize,
            width: self.width,
            height: self.height,
        }
    }
}