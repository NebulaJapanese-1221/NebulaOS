use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect, button::Button};
use super::app::{App, AppEvent};
use alloc::boxed::Box;

#[derive(Clone, Debug)]
pub struct TextEditor {
    pub content: String,
    pub scroll_offset: isize,
    pub cursor_pos: usize,
    pub format_menu_open: bool,
    pub history: Vec<String>,
    pub redo_history: Vec<String>,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            format_menu_open: false,
            history: Vec::with_capacity(5),
            redo_history: Vec::with_capacity(5),
        }
    }

    fn push_history(&mut self) {
        if self.history.len() >= 5 {
            self.history.remove(0);
        }
        self.history.push(self.content.clone());
        self.redo_history.clear();
    }

    fn undo(&mut self) {
        if let Some(prev_content) = self.history.pop() {
            if self.redo_history.len() >= 5 {
                self.redo_history.remove(0);
            }
            self.redo_history.push(self.content.clone());
            self.content = prev_content;
            self.cursor_pos = self.content.len().min(self.cursor_pos);
        }
    }

    fn redo(&mut self) {
        if let Some(next_content) = self.redo_history.pop() {
            if self.history.len() >= 5 {
                self.history.remove(0);
            }
            self.history.push(self.content.clone());
            self.content = next_content;
            self.cursor_pos = self.content.len().min(self.cursor_pos);
        }
    }

    pub fn process_key(&mut self, key: char) {
        self.push_history();
        match key {
            '\x08' => { // Backspace
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.content.remove(self.cursor_pos);
                }
            }
            '\n' => {
                self.content.insert(self.cursor_pos, '\n');
                self.cursor_pos += 1;
            }
            _ => {
                // Filter out non-printable characters
                if !key.is_control() {
                    self.content.insert(self.cursor_pos, key);
                    self.cursor_pos += 1;
                }
            }
        }
    }

     pub fn scroll(&mut self, delta: isize, win_height: usize) {
        let content_height = (self.content.lines().count() as isize + 1) * 16;
        let max_offset = (content_height - win_height as isize + 20).max(0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0, max_offset);
    }

    fn format_indent(&mut self) {
        self.push_history();
        let mut new_content = String::new();
        for line in self.content.lines() {
            new_content.push_str("    ");
            new_content.push_str(line);
            new_content.push('\n');
        }
        self.content = new_content;
        self.cursor_pos = self.content.len();
    }

    fn format_wrap(&mut self) {
        self.push_history();
        let limit = 40;
        let mut new_content = String::new();
        for line in self.content.lines() {
            let mut current_line_len = 0;
            for word in line.split_whitespace() {
                if current_line_len + word.len() > limit {
                    new_content.push('\n');
                    current_line_len = 0;
                } else if current_line_len > 0 {
                    new_content.push(' ');
                    current_line_len += 1;
                }
                new_content.push_str(word);
                current_line_len += word.len();
            }
            new_content.push('\n');
        }
        self.content = new_content;
        self.cursor_pos = self.content.len();
    }
}

impl App for TextEditor {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if gui::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;
        let toolbar_height = 34;

        // 1. Draw Toolbar Area
        gui::draw_rect(fb, win.x, win.y + title_height as isize, win.width, toolbar_height, 0x00_25_25_26, Some(dirty_rect));
        
        // Clear Button
        let mut btn_clear = Button::new(win.x + 5, win.y + title_height as isize + 5, 50, 24, "Clear");
        btn_clear.draw(fb, 0, 0, Some(dirty_rect));

        // Save Button (Mockup)
        let mut btn_save = Button::new(win.x + 60, win.y + title_height as isize + 5, 50, 24, "Save");
        btn_save.draw(fb, 0, 0, Some(dirty_rect));

        // Undo Button
        let mut btn_undo = Button::new(win.x + 115, win.y + title_height as isize + 5, 50, 24, "Undo");
        btn_undo.draw(fb, 0, 0, Some(dirty_rect));

        // Redo Button
        let mut btn_redo = Button::new(win.x + 170, win.y + title_height as isize + 5, 50, 24, "Redo");
        btn_redo.draw(fb, 0, 0, Some(dirty_rect));

        // Format Button
        let mut btn_format = Button::new(win.x + 225, win.y + title_height as isize + 5, 60, 24, "Format");
        btn_format.draw(fb, 0, 0, Some(dirty_rect));

        // Editor Statistics
        let word_count = self.content.split_whitespace().count();
        let stats = format!("Chars: {} | Words: {} | Cursor: {}", self.content.len(), word_count, self.cursor_pos);
        font::draw_string(fb, win.x + 295, win.y + title_height as isize + 9, &stats, 0x00_80_80_80, Some(dirty_rect));

        // 2. Draw Text Content
        let content_x = win.x + 5;
        let mut current_x = content_x;
        let text_area_y = win.y + title_height as isize + toolbar_height as isize;
        let mut current_y = text_area_y + 5 - self.scroll_offset;
        let line_height = 16;
        
        // Define clipping for the text area to prevent text bleeding into the toolbar during scroll
        let text_area = Rect {
            x: win.x,
            y: text_area_y,
            width: win.width,
            height: win.height.saturating_sub(title_height + toolbar_height),
        };
        let draw_clip = dirty_rect.intersection(&text_area).unwrap_or(dirty_rect);

        // Draw text content
        for (i, ch) in self.content.chars().enumerate() {
            // Draw blinking cursor before the character at cursor_pos
            if i == self.cursor_pos {
                gui::draw_rect(fb, current_x, current_y, 2, line_height, 0x00_FFFFFF, Some(draw_clip));
            }

            if ch == '\n' {
                current_y += line_height as isize;
                current_x = content_x;
            } else {
                let char_width = font::draw_char(fb, current_x, current_y, ch, 0x00_FFFFFF, Some(draw_clip));
                current_x += char_width as isize;
            }
        }

        // Draw cursor at the end of the text if it belongs there
        if self.cursor_pos == self.content.len() {
            gui::draw_rect(fb, current_x, current_y, 2, line_height, 0x00_FFFFFF, Some(draw_clip));
        }

        // 3. Draw Format Menu Overlay
        if self.format_menu_open {
            let menu_x = win.x + 225;
            let menu_y = win.y + title_height as isize + toolbar_height as isize;
            gui::draw_rect(fb, menu_x, menu_y, 85, 52, 0x00_1A_1A_1C, Some(dirty_rect));
            
            let mut btn_indent = Button::new(menu_x + 5, menu_y + 5, 75, 20, "Indent");
            btn_indent.draw(fb, 0, 0, Some(dirty_rect));
            
            let mut btn_wrap = Button::new(menu_x + 5, menu_y + 28, 75, 20, "Wrap");
            btn_wrap.draw(fb, 0, 0, Some(dirty_rect));
        }
    }

    fn handle_event(&mut self, event: &AppEvent, win: &Window) -> Option<Rect> {
        match event {
            AppEvent::MouseClick { x, y, .. } => {
                let toolbar_h = 34;

                if *y >= 0 && *y < toolbar_h {
                    if *x >= 5 && *x < 55 {
                        self.push_history();
                        self.content.clear();
                        self.cursor_pos = 0;
                        self.format_menu_open = false;
                        return Some(win.rect());
                    } else if *x >= 60 && *x < 110 { // Save
                        self.format_menu_open = false;
                        return Some(win.rect());
                    } else if *x >= 115 && *x < 165 { // Undo
                        self.undo();
                        self.format_menu_open = false;
                        return Some(win.rect());
                    } else if *x >= 170 && *x < 220 { // Redo
                        self.redo();
                        self.format_menu_open = false;
                        return Some(win.rect());
                    } else if *x >= 225 && *x < 285 { // Format
                        self.format_menu_open = !self.format_menu_open;
                        return Some(win.rect());
                    }
                } else if self.format_menu_open && *x >= 225 && *x < 310 && *y >= toolbar_h as isize && *y < (toolbar_h + 52) as isize {
                    if *y < (toolbar_h + 26) as isize {
                        self.format_indent();
                    } else {
                        self.format_wrap();
                    }
                    self.format_menu_open = false;
                    return Some(win.rect());
                } else if self.format_menu_open {
                    self.format_menu_open = false;
                    return Some(win.rect());
                }
            }
            AppEvent::KeyPress { key } => {
                self.format_menu_open = false;
                self.process_key(*key);
                return Some(win.rect());
            }
            AppEvent::Scroll { delta, height, .. } => {
                self.scroll(*delta, *height);
                return Some(win.rect());
            }
            _ => {}
        }
        None
    }
    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }

}