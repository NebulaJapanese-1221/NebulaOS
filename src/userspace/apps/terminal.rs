use alloc::string::{String, ToString};
use alloc::format;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect};
use super::app::{App, AppEvent};
use core::sync::atomic::Ordering;
use core::cell::Cell;

#[derive(Clone)]
pub struct Terminal {
    pub history: Vec<String>,
    pub input_buffer: String,
    pub prompt: String,
    pub cursor_visible: bool,
    pub last_blink_tick: usize,
    dirty: Cell<bool>,
}

impl Terminal {
    pub fn new() -> Self {
        let mut term = Self {
            history: Vec::new(),
            input_buffer: String::new(),
            prompt: String::from("/> "),
            cursor_visible: true,
            last_blink_tick: 0,
            dirty: Cell::new(true),
        };
        term.history.push(String::from("NebulaOS Terminal"));
        term
    }

    pub fn process_key(&mut self, key: char) {
        match key {
            '\x08' => { // Backspace
                self.input_buffer.pop();
            }
            '\n' => { // Enter
                let cmd = self.input_buffer.trim().to_string();
                if !cmd.is_empty() {
                    self.history.push(format!("{}{}", self.prompt, cmd));
                    self.execute_command(&cmd);
                } else {
                    self.history.push(self.prompt.clone());
                }
                self.input_buffer.clear();
            }
            _ => {
                if !key.is_control() {
                    self.input_buffer.push(key);
                }
            }
        }
        self.dirty.set(true);
    }

    fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0] {
            "help" => {
                self.history.push("Available commands:".to_string());
                self.history.push("  clear    - Clear the screen".to_string());
                self.history.push("  ps       - List running processes".to_string());
                self.history.push("  reboot   - Reboot the system".to_string());
                self.history.push("  shutdown - Power off the system".to_string());
                self.history.push("  help     - Show this message".to_string());
                self.history.push("  uname    - Show system info (-a for all)".to_string());
                self.history.push("  ver      - Show system version".to_string());
                self.history.push("  uptime   - Show system uptime".to_string());
            }
            "clear" => {
                self.history.clear();
                self.history.push(String::from("NebulaOS Terminal"));
            }
            "ver" => {
                self.history.push(format!("NebulaOS v{}", crate::kernel::VERSION));
            }
            "uname" => {
                let arg = if parts.len() > 1 { parts[1] } else { "" };
                match arg {
                    "-a" | "--all" => { 
                         let brand_guard = crate::kernel::cpu::CPU_BRAND.lock();
                         let cpu = brand_guard.as_deref().unwrap_or("i686");
                         let cores = crate::kernel::CPU_CORES.load(core::sync::atomic::Ordering::Relaxed);
                         self.history.push(format!("NebulaOS nebula {} {} ({} cores)", crate::kernel::VERSION, cpu, cores));
                    }
                    "-s" | "--kernel-name" | "" => {
                        self.history.push("NebulaOS".to_string());
                    }
                    "-n" | "--nodename" => {
                        self.history.push("nebula".to_string());
                    }
                    "-r" | "--kernel-release" => {
                        self.history.push(format!("{}", crate::kernel::VERSION));
                    }
                    "-v" | "--kernel-version" => {
                        self.history.push("custom-build".to_string());
                    }
                    "-m" | "--machine" => {
                        self.history.push("i686".to_string());
                    }
                    "-p" | "--processor" => {
                        let brand_guard = crate::kernel::cpu::CPU_BRAND.lock();
                        self.history.push(brand_guard.as_deref().unwrap_or("unknown").to_string());
                    }
                    "--help" => {
                        self.history.push("Usage: uname [OPTION]...".to_string());
                        self.history.push("Print certain system information.".to_string());
                        self.history.push(" -a, --all                print all information".to_string());
                        self.history.push(" -s, --kernel-name        print the kernel name".to_string());
                        self.history.push(" -n, --nodename           print the network node hostname".to_string());
                        self.history.push(" -r, --kernel-release     print the kernel release".to_string());
                        self.history.push(" -v, --kernel-version     print the kernel version".to_string());
                        self.history.push(" -m, --machine            print the machine hardware name".to_string());
                        self.history.push(" -p, --processor          print the processor type".to_string());
                    }
                    _ => {
                        self.history.push(format!("uname: invalid option -- '{}'", arg));
                    }
                }
            }
            "ps" => {
                self.history.push(String::from("ID  STATUS"));
                let scheduler = crate::kernel::process::SCHEDULER.lock();
                for (i, task) in scheduler.tasks.iter().enumerate() {
                    let status = if i == scheduler.current_index { "Running" } else { "Ready" };
                    self.history.push(format!("{:<3} {}", task.id, status));
                }
            }
            "reboot" => {
                self.history.push("Rebooting...".to_string());
                crate::kernel::power::reboot();
            }
            "shutdown" => {
                self.history.push("Shutting down...".to_string());
                crate::kernel::power::shutdown();
            }
            "uptime" => {
                let ticks = crate::kernel::process::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                let seconds = ticks / 1000;
                self.history.push(format!("Uptime: {} seconds", seconds));
            }
            _ => {
                self.history.push(format!("Unknown command: {}", parts[0]));
            }
        }
    }
}

impl App for Terminal {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if gui::LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;
        let padding = 10;

        // 1. Only clear the background within the dirty region
        let content_rect = Rect { x: win.x, y: win.y + title_height as isize, width: win.width, height: win.height - title_height };
        if let Some(fill_area) = dirty_rect.intersection(&content_rect) {
            gui::draw_rect(fb, fill_area.x, fill_area.y, fill_area.width, fill_area.height, 0x00_12_12_12, None);
        }

        let start_x = win.x + padding as isize;
        let mut y = win.y + title_height as isize + padding as isize;
        let line_height = font_height;

        let max_lines = (win.height - 30) / line_height;
        // Reserve one line for the input prompt so it is always visible
        let history_lines = if max_lines > 0 { max_lines - 1 } else { 0 };
        let skip = if self.history.len() > history_lines { self.history.len() - history_lines } else { 0 };

        for line in self.history.iter().skip(skip) {
            // 2. Only draw history lines if they intersect the dirty rect
            let line_bounds = Rect { x: win.x, y, width: win.width, height: line_height };
            if dirty_rect.intersects(&line_bounds) {
                font::draw_string(fb, start_x, y, line, 0x00_A0_A0_A0, Some(dirty_rect));
            }
            y += line_height as isize;
        }

        // 3. Draw Input Line (the most frequent update while typing)
        let input_line = alloc::format!("{}{}", self.prompt, self.input_buffer);
        if y < (win.y + win.height as isize - line_height as isize) {
            let input_bounds = Rect { x: win.x, y, width: win.width, height: line_height };
            if dirty_rect.intersects(&input_bounds) {
                font::draw_string(fb, start_x, y, input_line.as_str(), 0x00_E1_E1_E1, Some(dirty_rect));
                if self.cursor_visible {
                    let cursor_x = start_x + font::string_width(&input_line) as isize;
                    // Only draw cursor if it's within the dirty area
                    gui::draw_rect(fb, cursor_x, y, 8, line_height as usize, 0x00_00_7A_CC, Some(dirty_rect));
                }
            }
        }
    }

    fn handle_event(&mut self, event: &AppEvent, win: &Window) -> Option<Rect> {
        match event {
            AppEvent::KeyPress { key } => {
            let font_height = if gui::LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
            let title_height = font_height + 10;
            let line_height = font_height;
            let max_lines = (win.height - (title_height + 10)) / line_height;
            let history_lines = if max_lines > 0 { max_lines - 1 } else { 0 };
            
            // Calculate where the input line currently is on the screen
            let displayed_history = self.history.len().min(history_lines);
            let input_y = win.y + title_height as isize + 10 + (displayed_history as isize * line_height as isize);
            
            let input_rect = Rect {
                x: win.x,
                y: input_y,
                width: win.width,
                height: line_height as usize,
            };

            self.process_key(*key);
            self.cursor_visible = true; // Ensure cursor is visible while typing
            self.last_blink_tick = 0;   // Reset blink phase

            // If we hit enter or the history just filled up, refresh the whole window
            if *key == '\n' || self.history.len() > history_lines {
                None
            } else {
                Some(input_rect)
            }
            }
            AppEvent::Tick { tick_count } => {
                if *tick_count > self.last_blink_tick + 500 {
                    self.cursor_visible = !self.cursor_visible;
                    self.last_blink_tick = *tick_count;

                    let font_height = if gui::LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                    let title_height = font_height + 10;
                    let line_height = font_height;
                    let max_lines = (win.height - (title_height + 10)) / line_height;
                    let history_lines = if max_lines > 0 { max_lines - 1 } else { 0 };
                    let displayed_history = self.history.len().min(history_lines);
                    let input_y = win.y + title_height as isize + 10 + (displayed_history as isize * line_height as isize);
                    let input_line = format!("{}{}", self.prompt, self.input_buffer);
                    let cursor_x = win.x + 10 + font::string_width(&input_line) as isize;

                    return Some(Rect { x: cursor_x, y: input_y, width: 8, height: line_height as usize });
                }
                None
            }
            _ => None
        }
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}