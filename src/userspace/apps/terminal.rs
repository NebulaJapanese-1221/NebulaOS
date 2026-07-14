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
    pub should_close: bool, // New field to indicate if the terminal should close
    pub fs: Option<crate::fs::NebulaFS>, // Add filesystem reference
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            buffer: String::from("> ".to_string()), // Start with prompt
            cursor_pos: 2, // Cursor starts after "> "
            history: Vec::new(),
            history_idx: None,
            should_close: false, // Initialize to false
            fs: None, // Will be set when the terminal is initialized
        }
    }

    pub fn set_filesystem(&mut self, fs: crate::fs::NebulaFS) {
        self.fs = Some(fs);
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
            "help" => { // New command: Display help
                self.buffer.push_str("Available commands:\n");
                self.buffer.push_str("  ver - Display OS version\n");
                self.buffer.push_str("  cls - Clear the screen\n");
                self.buffer.push_str("  help - Display this help message\n");
                self.buffer.push_str("  echo <text> - Print text\n");
                self.buffer.push_str("  date - Display current date\n");
                self.buffer.push_str("  time - Display current time\n");
                self.buffer.push_str("  exit - Close the terminal\n");
                self.buffer.push_str("  ls - List directory contents\n");
                self.buffer.push_str("  cd <dir> - Change directory\n");
                self.buffer.push_str("  cat <file> - Display file contents\n");
                self.buffer.push_str("  touch <file> - Create file\n");
                self.buffer.push_str("  mkdir <dir> - Create directory\n");
                self.buffer.push_str("  rm <file> - Remove file/directory\n");
            }
            "echo" => { // New command: Echo text
                // Extract the text after "echo"
                if let Some(text) = cmd.split_whitespace().nth(1) {
                    self.buffer.push_str(text);
                    self.buffer.push_str("\n");
                } else {
                    self.buffer.push_str("Usage: echo <text>\n");
                }
            }
            "date" => { // New command: Display date
                let time = crate::drivers::rtc::get_time();
                self.buffer.push_str(&format!("Current date: {}-{}-{} [Not fully implemented]\n", 2023, 1, 1));
            }
            "time" => { // New command: Display time
                let time = crate::drivers::rtc::get_time();
                self.buffer.push_str(&format!("Current time: {:02}:{:02}:{:02}\n", time.hour, time.minute, time.second));
            }
            "exit" => { // New command: Exit terminal
                self.should_close = true;
                self.buffer.push_str("Terminal closed.\n");
            }
            "ls" => { // List directory contents
                if let Some(fs) = &self.fs {
                    // In a real implementation, we would list the directory contents
                    self.buffer.push_str("Listing directory contents:\n");
                    self.buffer.push_str("file1.txt\n");
                    self.buffer.push_str("file2.txt\n");
                    self.buffer.push_str("Documents\n");
                    self.buffer.push_str("Downloads\n");
                } else {
                    self.buffer.push_str("Filesystem not available\n");
                }
            }
            "cd" => { // Change directory
                // Extract directory name
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let dir = parts[1];
                    self.buffer.push_str(&format!("Changing to directory: {}\n", dir));
                    // In a real implementation, we would change the current directory
                } else {
                    self.buffer.push_str("Usage: cd <directory>\n");
                }
            }
            "cat" => { // Display file contents
                // Extract filename
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let filename = parts[1];
                    self.buffer.push_str(&format!("Displaying file: {}\n", filename));
                    // In a real implementation, we would read and display the file
                    self.buffer.push_str("File contents would be shown here.\n");
                } else {
                    self.buffer.push_str("Usage: cat <filename>\n");
                }
            }
            "touch" => { // Create file
                // Extract filename
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let filename = parts[1];
                    if let Some(fs) = &mut self.fs {
                        match fs.create_file(2, filename) { // 2 is root directory inode
                            Ok(inode) => {
                                self.buffer.push_str(&format!("Created file: {} (inode {})\n", filename, inode));
                            }
                            Err(e) => {
                                self.buffer.push_str(&format!("Failed to create file: {}\n", e));
                            }
                        }
                    } else {
                        self.buffer.push_str("Filesystem not available\n");
                    }
                } else {
                    self.buffer.push_str("Usage: touch <filename>\n");
                }
            }
            "mkdir" => { // Create directory
                // Extract directory name
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let dirname = parts[1];
                    if let Some(fs) = &mut self.fs {
                        match fs.create_dir(2, dirname) { // 2 is root directory inode
                            Ok(inode) => {
                                self.buffer.push_str(&format!("Created directory: {} (inode {})\n", dirname, inode));
                            }
                            Err(e) => {
                                self.buffer.push_str(&format!("Failed to create directory: {}\n", e));
                            }
                        }
                    } else {
                        self.buffer.push_str("Filesystem not available\n");
                    }
                } else {
                    self.buffer.push_str("Usage: mkdir <dirname>\n");
                }
            }
            "rm" => { // Remove file/directory
                // Extract filename
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let filename = parts[1];
                    if let Some(fs) = &mut self.fs {
                        match fs.unlink(2, filename) { // 2 is root directory inode
                            Ok(_) => {
                                self.buffer.push_str(&format!("Removed: {}\n", filename));
                            }
                            Err(e) => {
                                self.buffer.push_str(&format!("Failed to remove: {}\n", e));
                            }
                        }
                    } else {
                        self.buffer.push_str("Filesystem not available\n");
                    }
                } else {
                    self.buffer.push_str("Usage: rm <filename>\n");
                }
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

