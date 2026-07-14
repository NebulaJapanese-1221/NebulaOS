use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::rtc;

#[derive(Debug)]
pub struct TerminalState {
    pub buffer: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub should_close: bool,
    pub fs: Option<crate::fs::NebulaFS>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            buffer: String::from("> ".to_string()),
            cursor_pos: 2,
            history: Vec::new(),
            history_idx: None,
            should_close: false,
            fs: None,
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
            "cls" => {
                self.buffer.clear();
                self.buffer.push_str("> ");
                self.cursor_pos = 2;
            }
            "help" => {
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
            "echo" => {
                if let Some(text) = cmd.split_whitespace().nth(1) {
                    self.buffer.push_str(text);
                    self.buffer.push_str("\n");
                } else {
                    self.buffer.push_str("Usage: echo <text>\n");
                }
            }
            "date" => {
                let time = rtc::get_time();
                self.buffer.push_str(&format!("Current date: {}-{}-{} [Not fully implemented]\n", 2023, 1, 1));
            }
            "time" => {
                let time = rtc::get_time();
                self.buffer.push_str(&format!("Current time: {:02}:{:02}:{:02}\n", time.hour, time.minute, time.second));
            }
            "exit" => {
                self.should_close = true;
                self.buffer.push_str("Terminal closed.\n");
            }
            "ls" => {
                if let Some(fs) = &self.fs {
                    self.buffer.push_str("Listing directory contents:\n");
                    self.buffer.push_str("file1.txt\n");
                    self.buffer.push_str("file2.txt\n");
                    self.buffer.push_str("Documents\n");
                    self.buffer.push_str("Downloads\n");
                } else {
                    self.buffer.push_str("Filesystem not available\n");
                }
            }
            "cd" => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let dir = parts[1];
                    self.buffer.push_str(&format!("Changing to directory: {}\n", dir));
                } else {
                    self.buffer.push_str("Usage: cd <directory>\n");
                }
            }
            "cat" => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let filename = parts[1];
                    self.buffer.push_str(&format!("Displaying file: {}\n", filename));
                    self.buffer.push_str("File contents would be shown here.\n");
                } else {
                    self.buffer.push_str("Usage: cat <filename>\n");
                }
            }
            "touch" => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let filename = parts[1];
                    if let Some(fs) = &mut self.fs {
                        match fs.create_file(2, filename) {
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
            "mkdir" => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let dirname = parts[1];
                    if let Some(fs) = &mut self.fs {
                        match fs.create_dir(2, dirname) {
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
            "rm" => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() > 1 {
                    let filename = parts[1];
                    if let Some(fs) = &mut self.fs {
                        match fs.unlink(2, filename) {
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
            "" => {}
            _ => {
                self.buffer.push_str(&format!("Command not found: {}\n", cmd));
            }
        }

        if !cmd.trim().is_empty() && cmd.trim() != "cls" && (self.history.is_empty() || self.history.last().unwrap() != cmd) {
            self.history.push(cmd.to_string());
        }
        self.history_idx = None;

        if cmd.trim() != "cls" {
            self.buffer.push_str("> ");
            self.cursor_pos = self.buffer.len();
        }
    }

    pub fn handle_keypress(&mut self, c: char) {
        match c {
            '\n' => {
                let cmd_start_idx = self.buffer.rfind('>').map_or(0, |idx| idx + 1);
                let cmd = self.buffer.chars().skip(cmd_start_idx).collect::<String>().trim().to_string();
                self.process_command(&cmd);
            }
            '\x08' => {
                if self.cursor_pos > 0 {
                    if self.cursor_pos > self.buffer.rfind('>').map_or(0, |idx| idx + 1) {
                        self.buffer.remove(self.cursor_pos - 1);
                        self.cursor_pos -= 1;
                    }
                }
            }
            _ => {
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

        let is_cleared_state = state.buffer.starts_with("> ") && 
                                (state.buffer.len() == 2 || (state.buffer.len() > 2 && state.buffer.ends_with('\n') && state.buffer.lines().count() <= 2));
        
        if is_cleared_state {
            fb.draw_rect(x, y, w, h, 0x00000000);
        } else {
            let mut current_y = y + 5;
            let mut lines_drawn = 0;
            let max_lines = h / 12;

            for line in state.buffer.lines() {
                if lines_drawn < max_lines {
                    draw_string(fb, x + 5, current_y, line, 0x00FFFFFF);
                    current_y += 12;
                    lines_drawn += 1;
                }
            }
        }
    }

    pub fn handle_click(_state: &mut TerminalState, _bounds: Rect, _mx: i32, _my: i32) {}

    pub fn handle_keypress(state: &mut TerminalState, c: char) {
        state.handle_keypress(c);
    }
}