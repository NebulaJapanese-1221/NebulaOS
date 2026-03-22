use alloc::string::{String, ToString};
use alloc::format;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window};
use super::app::{App, AppEvent};

#[derive(Clone)]
pub struct Terminal {
    pub history: Vec<String>,
    pub input_buffer: String,
    pub prompt: String,
}

impl Terminal {
    pub fn new() -> Self {
        let mut term = Self {
            history: Vec::new(),
            input_buffer: String::new(),
            prompt: String::from("/> "),
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
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window) {
        gui::draw_rect(fb, win.x, win.y + 20, win.width, win.height - 20, 0x00_000000, None);

        let start_x = win.x + 5;
        let mut y = win.y + 25;
        let line_height = 16;

        let max_lines = (win.height - 30) / line_height;
        // Reserve one line for the input prompt so it is always visible
        let history_lines = if max_lines > 0 { max_lines - 1 } else { 0 };
        let skip = if self.history.len() > history_lines { self.history.len() - history_lines } else { 0 };

        for line in self.history.iter().skip(skip) {
            font::draw_string(fb, start_x, y, line, 0x00_CCCCCC, None);
            y += line_height as isize;
        }

        let input_line = alloc::format!("{}{}", self.prompt, self.input_buffer);
        if y < (win.y + win.height as isize - line_height as isize) {
             font::draw_string(fb, start_x, y, input_line.as_str(), 0x00_FFFFFF, None);
             let cursor_x = start_x + font::string_width(&input_line) as isize;
             gui::draw_rect(fb, cursor_x, y, 8, line_height as usize, 0x00_FFFFFF, None);
        }
    }

    fn handle_event(&mut self, event: &AppEvent) {
        if let AppEvent::KeyPress { key } = event {
            self.process_key(*key);
        }
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}