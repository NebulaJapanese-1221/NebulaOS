use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;
use alloc::vec::Vec;

pub struct TerminalState {
    pub buffer: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_idx: None,
        }
    }

    pub fn process_command(&mut self, cmd: &str) {
        match cmd.trim() {
            "ver" => {
                self.buffer.push_str("NebulaOS v0.0.1\n");
            }
            "" => { /* Do nothing on empty input */ }
            _ => {
                self.buffer.push_str(&format!("Command not found: {}\n", cmd));
            }
        }
        // Add command to history if it's not empty and not a duplicate of the last entry
        if !cmd.trim().is_empty() && (self.history.is_empty() || self.history.last().unwrap() != cmd) {
            self.history.push(cmd.to_string());
        }
        self.history_idx = None; // Reset history index after executing a command
        self.buffer.push_str("> ");
        self.cursor_pos = self.buffer.len();
    }

    pub fn handle_keypress(&mut self, c: char) {
        match c {
            '\n' => { // Enter key
                let cmd = self.buffer[self.buffer.rfind('>' ..= self.buffer.len()).unwrap_or(0)..].trim_start_matches('>').trim().to_string();
                self.process_command(&cmd);
            }
            '\x08' => { // Backspace key
                if self.cursor_pos > 0 {
                    self.buffer.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            }
            _ => { // Regular character
                self.buffer.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
        }
    }
}

pub struct TerminalApp;

impl TerminalApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &TerminalState) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Background
        fb.draw_rect(x, y, w, h, 0x00000000); // Black background

        // Display buffer content
        let mut current_y = y + 5;
        let mut line_start = 0;
        for (i, char) in state.buffer.char_indices() {
            if char == '\n' {
                let line = &state.buffer[line_start..i];
                draw_string(fb, x + 5, current_y, line, 0x00FFFFFF); // White text
                current_y += 12; // Line height
                line_start = i + 1;
            }
        }
        // Draw the last line (or only line if no newline)
        let last_line = &state.buffer[line_start..];
        draw_string(fb, x + 5, current_y, last_line, 0x00FFFFFF);

        // Draw prompt at the end of the buffer
        if state.buffer.is_empty() || state.buffer.ends_with('\n') {
            let prompt = "> ";
            draw_string(fb, x + 5, current_y + 12, prompt, 0x0000FF00); // Green prompt
            // Cursor should be after prompt
            state.cursor_pos = state.buffer.len() + prompt.len();
        }
    }

    pub fn handle_keypress(state: &mut TerminalState, c: char) {
        state.handle_keypress(c);
    }
}