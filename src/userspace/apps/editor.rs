use alloc::string::String;
use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window};
use super::app::{App, AppEvent};
use alloc::boxed::Box;

#[derive(Clone, Debug)]
pub struct TextEditor {
    pub content: String,
    pub scroll_offset: isize,
    pub cursor_pos: usize,
    
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
        }
    }

    pub fn process_key(&mut self, key: char) {
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
}

impl App for TextEditor {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window) {
        let content_x = win.x + 5;
        let mut current_x = content_x;
        let mut current_y = win.y + 25;
        let line_height = 16;
        
        // Draw text content
        for (i, ch) in self.content.chars().enumerate() {
            // Draw blinking cursor before the character at cursor_pos
            if i == self.cursor_pos {
                // A real implementation would blink this based on a timer.
                gui::draw_rect(fb, current_x, current_y, 2, line_height, 0x00_FFFFFF, None);
            }

            if ch == '\n' {
                current_y += line_height as isize;
                current_x = content_x;
            } else {
                let char_width = font::draw_char(fb, current_x, current_y, ch, 0x00_FFFFFF, None);
                current_x += char_width as isize;
            }
        }

        // Draw cursor at the end of the text if it belongs there
        if self.cursor_pos == self.content.len() {
            gui::draw_rect(fb, current_x, current_y, 2, line_height, 0x00_FFFFFF, None);
        }
    }

    fn handle_event(&mut self, event: &AppEvent) {
        if let AppEvent::KeyPress { key } = event {
            self.process_key(*key);
        } else if let AppEvent::Scroll { delta } = event {
        }
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }

}