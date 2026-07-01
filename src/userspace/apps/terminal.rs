use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::{String, ToString}; // Import ToString
use alloc::vec::Vec; // Needed for history
use alloc::format;

#[derive(Debug)] // Add Debug derive for easier debugging
pub struct TerminalState {
    pub buffer: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            buffer: String::from("> ".to_string()), // Start with prompt
            cursor_pos: 2, // Cursor starts after "> "
            history: Vec::new(),
            history_idx: None,
        }
    }

    pub fn process_command(&mut self, cmd: &str) {
        match cmd.trim() {
            "ver" => {
                self.buffer.push_str("NebulaOS v0.0.1\n");
            }
            "cls" => { // New command: Clear Screen
                self.buffer.clear();
                self.buffer.push_str("> ");
                self.cursor_pos = 2; // Reset cursor position after clearing
            }
            "" => { /* Do nothing on empty input */ }
            _ => {
                // Use format! macro which needs `use alloc::format;`
                self.buffer.push_str(&format!("Command not found: {}\n", cmd));
            }
        }
        // Add command to history if it's not empty, not 'cls', and not a duplicate of the last entry
        if !cmd.trim().is_empty() && cmd.trim() != "cls" && (self.history.is_empty() || self.history.last().unwrap() != cmd) {
            self.history.push(cmd.to_string());
        }
        self.history_idx = None; // Reset history index after executing a command
        // If the command was not 'cls', we add a new prompt. If it was 'cls', the prompt is already there.
        if cmd.trim() != "cls" {
            self.buffer.push_str("> ");
            self.cursor_pos = self.buffer.len(); // Move cursor to end of new prompt
        }
    }

    pub fn handle_keypress(&mut self, c: char) {
        match c {
            '\n' => { // Enter key
                // Extract command part after the last "> "
                let cmd_start_idx = self.buffer.rfind('>').map_or(0, |idx| idx + 1); // Find last '>' and move past it
                let cmd = self.buffer.chars().skip(cmd_start_idx).collect::<String>().trim().to_string(); // Trim whitespace
                self.process_command(&cmd);
            }
            '\x08' => { // Backspace key
                if self.cursor_pos > 0 {
                    // Remove character before cursor if not at the beginning of the buffer
                    // Ensure we don't delete prompt characters "> "
                    if self.cursor_pos > self.buffer.rfind('>').map_or(0, |idx| idx + 1) { 
                        self.buffer.remove(self.cursor_pos - 1);
                        self.cursor_pos -= 1;
                    }
                }
            }
            // TODO: Add arrow key handling for history navigation (up/down)
            _ => { // Regular character
                // Insert character at current cursor position
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

        // Clear the entire window area first if 'cls' command was processed
        // Check if the buffer starts with "> " and has only the prompt and a newline, or just the prompt.
        let is_cleared_state = state.buffer.starts_with("> ") && 
                                (state.buffer.len() == 2 || (state.buffer.len() > 2 && state.buffer.ends_with('\n') && state.buffer.lines().count() <= 2));
        
        if is_cleared_state {
            fb.draw_rect(x, y, w, h, 0x00000000); // Black background
        } else {
            // If not cleared, redraw the buffer content
            let mut current_y = y + 5;
            let mut lines_drawn = 0;
            let max_lines = h / 12; // Approx max lines based on font height

            for line in state.buffer.lines() {
                if lines_drawn < max_lines {
                    draw_string(fb, x + 5, current_y, line, 0x00FFFFFF); // White text
                    current_y += 12; // Line height
                    lines_drawn += 1;
                } else {
                    // TODO: Implement scrolling if buffer exceeds screen height
                }
            }
        }
    }

    pub fn handle_click(_state: &mut TerminalState, _bounds: Rect, _mx: i32, _my: i32) {
        // Terminal doesn't have clickable UI yet; placeholder for future input handling
    }

    pub fn handle_keypress(state: &mut TerminalState, c: char) {
        state.handle_keypress(c);
    }
}