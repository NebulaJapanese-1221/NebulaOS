use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug)]
pub struct TextEditorState {
    pub content: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_offset: usize,
    pub syntax_highlighting: bool,
    pub file_path: Option<String>,
    pub modified: bool,
}

impl TextEditorState {
    pub fn new() -> Self {
        let mut content = Vec::new();
        content.push(String::from("// New File"));
        content.push(String::new());
        
        TextEditorState {
            content,
            cursor_x: 0,
            cursor_y: 0,
            scroll_offset: 0,
            syntax_highlighting: true,
            file_path: None,
            modified: false,
        }
    }
    
    pub fn insert_char(&mut self, c: char) {
        self.modified = true;
        
        if self.cursor_y >= self.content.len() {
            self.content.push(String::new());
        }
        
        let line = &mut self.content[self.cursor_y];
        line.insert(self.cursor_x, c);
        self.cursor_x += 1;
    }
    
    pub fn insert_newline(&mut self) {
        self.modified = true;
        
        if self.cursor_y >= self.content.len() {
            self.content.push(String::new());
        }
        
        let current_line = self.content[self.cursor_y].clone();
        let (left, right) = current_line.split_at(self.cursor_x);
        
        self.content[self.cursor_y] = left.to_string();
        self.content.insert(self.cursor_y + 1, right.to_string());
        
        self.cursor_y += 1;
        self.cursor_x = 0;
    }
    
    pub fn backspace(&mut self) {
        if self.cursor_x == 0 {
            if self.cursor_y > 0 {
                self.cursor_y -= 1;
                self.cursor_x = self.content[self.cursor_y].len();
                
                // Merge lines
                let current_line = self.content.remove(self.cursor_y + 1);
                self.content[self.cursor_y].push_str(&current_line);
            }
        } else {
            self.cursor_x -= 1;
            self.content[self.cursor_y].remove(self.cursor_x);
        }
        
        self.modified = true;
    }
    
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        if dy < 0 && self.cursor_y > 0 {
            self.cursor_y -= 1;
        } else if dy > 0 && self.cursor_y < self.content.len() - 1 {
            self.cursor_y += 1;
        }
        
        if dx < 0 && self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if dx > 0 {
            if self.cursor_x < self.content[self.cursor_y].len() {
                self.cursor_x += 1;
            }
        }
    }
    
    pub fn get_syntax_color(&self, line: &str) -> u32 {
        if !self.syntax_highlighting {
            return 0x00000000;
        }
        
        // Simple syntax highlighting
        if line.trim().starts_with("//") {
            0x00008800 // Green for comments
        } else if line.trim().starts_with("fn") || line.trim().starts_with("pub fn") {
            0x000000FF // Blue for functions
        } else if line.contains("let") || line.contains("mut") {
            0x00880000 // Purple for keywords
        } else if line.contains(":") {
            0x00008888 // Dark gray for types
        } else {
            0x00000000 // Black for normal text
        }
    }
}

pub struct TextEditorApp;

impl TextEditorApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &TextEditorState, is_focused: bool) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Draw background
        fb.draw_rect(x, y, w, h, 0x00FFFFFF);

        // Draw line numbers
        fb.draw_rect(x, y, 40, h, 0x00F0F0F0);
        
        // Draw content
        let mut line_y = y + 10;
        let start_line = state.scroll_offset;
        let end_line = (start_line + h / 15).min(state.content.len());
        
        for (i, line) in state.content.iter().skip(start_line).take(end_line - start_line) {
            // Draw line number
            draw_string(fb, x + 5, line_y, &format!("{:4}", start_line + i + 1), 0x00888888);
            
            // Draw line content with syntax highlighting
            let color = state.get_syntax_color(line);
            draw_string(fb, x + 45, line_y, line, color);
            
            line_y += 15;
        }
        
        // Draw cursor
        if state.cursor_y >= start_line && state.cursor_y < end_line {
            let cursor_line_y = y + 10 + (state.cursor_y - start_line) * 15;
            let cursor_x = x + 45 + state.cursor_x * 8;
            fb.draw_rect(cursor_x, cursor_line_y, 1, 12, 0x00000000);
        }
        
        // Draw status bar
        let status_y = y + h - 20;
        fb.draw_rect(x, status_y, w, 20, 0x00EEEEEE);
        
        let status_text = if let Some(path) = &state.file_path {
            format!("{}{}", path, if state.modified { " [Modified]" } else { "" })
        } else {
            String::from("Untitled")
        };
        
        draw_string(fb, x + 5, status_y + 3, &status_text, 0x00000000);
        draw_string(fb, x + w - 100, status_y + 3, &format!("Line: {}, Col: {}", state.cursor_y + 1, state.cursor_x + 1), 0x00000000);
    }

    pub fn handle_click(state: &mut TextEditorState, bounds: Rect, mx: i32, my: i32) {
        let x = bounds.x as i32;
        let y = bounds.y as i32 + TITLE_BAR_HEIGHT as i32;
        let w = bounds.width as i32;
        let h = (bounds.height as i32) - TITLE_BAR_HEIGHT as i32;
        
        // Check if click is in the content area
        if mx >= x + 45 && mx < x + w && my >= y && my < y + h - 20 {
            let line = (my - y) as usize / 15 + state.scroll_offset;
            if line < state.content.len() {
                let col = ((mx - (x + 45)) as usize) / 8;
                let max_col = state.content[line].len();
                
                state.cursor_y = line;
                state.cursor_x = col.min(max_col);
            }
        }
    }

    pub fn handle_keyboard_input(state: &mut TextEditorState, c: char) {
        match c {
            '\n' => state.insert_newline(),
            '\x08' => state.backspace(),
            _ => state.insert_char(c),
        }
    }
}