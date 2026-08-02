use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::fs::FileSystemOps;
use crate::rtc;
use crate::scheduler;
use crate::services::loader;
use crate::services::security;
use crate::services::power;

#[derive(Debug)]
pub struct TerminalState {
    pub buffer: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub should_close: bool,
    pub fs: Option<crate::fs::NebulaFS>,
    pub current_dir: String,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            buffer: String::from("> "),
            cursor_pos: 2,
            history: Vec::new(),
            history_idx: None,
            should_close: false,
            fs: None,
            current_dir: String::from("/"),
        }
    }

    pub fn set_filesystem(&mut self, fs: crate::fs::NebulaFS) {
        self.fs = Some(fs);
    }

    pub fn process_command(&mut self, cmd: &str) {
        let trimmed = cmd.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let command = parts.first().copied().unwrap_or("");

        match command {
            "ver" | "version" | "uname" => {
                self.buffer.push_str("NebulaOS v0.0.1 (Development Build)\n");
                self.buffer.push_str("Kernel: x86/ELF\n");
                self.buffer.push_str("Compiler: rustc 1.70+\n");
            }
            "cls" | "clear" => {
                self.buffer.clear();
                self.buffer.push_str("> ");
                self.cursor_pos = 2;
            }
            "help" => {
                self.buffer.push_str("Available commands:\n");
                self.buffer.push_str("  System:\n");
                self.buffer.push_str("    ver/version/uname - Display OS version\n");
                self.buffer.push_str("    cls/clear         - Clear the screen\n");
                self.buffer.push_str("    help              - Display this help message\n");
                self.buffer.push_str("    date              - Display current date\n");
                self.buffer.push_str("    time              - Display current time\n");
                self.buffer.push_str("    exit              - Close the terminal\n");
                self.buffer.push_str("    sysinfo           - Display system information\n");
                self.buffer.push_str("    meminfo           - Display memory information\n");
                self.buffer.push_str("    cpuinfo           - Display CPU information\n");
                self.buffer.push_str("    uptime            - Display system uptime\n");
                self.buffer.push_str("    shutdown          - Shutdown the system\n");
                self.buffer.push_str("    reboot            - Reboot the system\n");
                self.buffer.push_str("    whoami            - Display current user\n");
                self.buffer.push_str("    pwd               - Print working directory\n");
                self.buffer.push_str("  Filesystem:\n");
                self.buffer.push_str("    ls                - List directory contents\n");
                self.buffer.push_str("    cd <dir>          - Change directory\n");
                self.buffer.push_str("    cat <file>        - Display file contents\n");
                self.buffer.push_str("    touch <file>      - Create file\n");
                self.buffer.push_str("    mkdir <dir>       - Create directory\n");
                self.buffer.push_str("    rm <file>         - Remove file/directory\n");
                self.buffer.push_str("    df                - Display disk usage\n");
                self.buffer.push_str("    lsblk             - List block devices\n");
                self.buffer.push_str("  App Management:\n");
                self.buffer.push_str("    lsapp             - List registered apps\n");
                self.buffer.push_str("    run <app>         - Launch an application\n");
                self.buffer.push_str("    regapp <path>     - Register app from file\n");
                self.buffer.push_str("  Network:\n");
                self.buffer.push_str("    ping <host>       - Ping a host\n");
                self.buffer.push_str("    netstat           - Show network connections\n");
                self.buffer.push_str("  Utilities:\n");
                self.buffer.push_str("    echo <text>       - Print text\n");
                self.buffer.push_str("    ps                - List running processes\n");
                self.buffer.push_str("    lspci             - List PCI devices\n");
            }
            "echo" => {
                if parts.len() > 1 {
                    let text: String = parts[1..].join(" ");
                    self.buffer.push_str(&text);
                    self.buffer.push_str("\n");
                } else {
                    self.buffer.push_str("Usage: echo <text>\n");
                }
            }
            "date" => {
                let t = rtc::get_time();
                self.buffer.push_str(&format!("Date: 2025/01/{:02}\n", t.second));
                self.buffer.push_str(&format!("Time: {:02}:{:02}:{:02}\n", t.hour, t.minute, t.second));
            }
            "time" => {
                let time = rtc::get_time();
                self.buffer.push_str(&format!("Current time: {:02}:{:02}:{:02}\n", time.hour, time.minute, time.second));
            }
            "uptime" => {
                let tick_count = scheduler::get_scheduler().lock().tick_count;
                let seconds = tick_count / 100;
                let mins = seconds / 60;
                let hrs = mins / 60;
                self.buffer.push_str(&format!("Uptime: {}h {:02}m {:02}s\n", hrs, mins % 60, seconds % 60));
            }
            "exit" => {
                self.should_close = true;
                self.buffer.push_str("Terminal closed.\n");
            }
            "sysinfo" => {
                self.buffer.push_str("System Information:\n");
                self.buffer.push_str("  OS: NebulaOS v0.0.1\n");
                self.buffer.push_str("  Architecture: x86/x86_64\n");
                self.buffer.push_str("  Kernel: Monolithic\n");
                self.buffer.push_str("  Filesystem: NebulaFS (ZFS-based)\n");
                self.buffer.push_str("  Display: Framebuffer (1024x768)\n");
            }
            "meminfo" => {
                self.buffer.push_str("Memory Information:\n");
                self.buffer.push_str("  Total RAM: ~64 MB (QEMU default)\n");
                self.buffer.push_str("  Kernel Heap: 1 MB\n");
                self.buffer.push_str("  Allocator: Buddy + Slab\n");
            }
            "cpuinfo" => {
                self.buffer.push_str("CPU Information:\n");
                self.buffer.push_str("  Model: x86 compatible\n");
                self.buffer.push_str("  Frequency: ~1000 MHz (QEMU default)\n");
            }
            "whoami" => {
                let sec = security::get_security_service().lock();
                match sec.current_user() {
                    Some(user) => self.buffer.push_str(&format!("{}\n", user.username)),
                    None => self.buffer.push_str("root\n"),
                }
            }
            "pwd" => {
                self.buffer.push_str(&format!("{}\n", self.current_dir));
            }
            "shutdown" => {
                self.buffer.push_str("Shutting down system...\n");
                let pw = power::get_power_service().lock();
                pw.request_shutdown();
            }
            "reboot" => {
                self.buffer.push_str("Rebooting system...\n");
                let pw = power::get_power_service().lock();
                pw.request_restart();
            }
            "ps" => {
                self.buffer.push_str("Process List:\n");
                let sched = scheduler::get_scheduler().lock();
                for p in sched.processes.iter() {
                    self.buffer.push_str(&format!("  PID={} State={:?} Name={}\n", p.pid, p.state, p.name));
                }
            }
            "lspci" => {
                self.buffer.push_str("PCI Devices:\n");
                self.buffer.push_str("  (PCI enumeration via drivers)\n");
                self.buffer.push_str("  - VGA compatible controller\n");
                self.buffer.push_str("  - SD Host Controller (if available)\n");
            }
            "lsblk" => {
                self.buffer.push_str("Block Devices:\n");
                self.buffer.push_str("  ramdisk - RAM disk (testing)\n");
                self.buffer.push_str("  ata     - ATA disk (placeholder)\n");
                self.buffer.push_str("  sd_card - SD/MMC card (if present)\n");
            }
            "df" => {
                self.buffer.push_str("Disk Usage:\n");
                if self.fs.is_some() {
                    self.buffer.push_str("  / (NebulaFS): ~1024 MB total, ~100 MB used\n");
                } else {
                    self.buffer.push_str("  Filesystem not available\n");
                }
            }
            "lsapp" => {
                self.buffer.push_str("Registered Applications:\n");
                let app_loader = loader::get_loader_service().lock();
                let apps = app_loader.list_apps();
                if apps.is_empty() {
                    self.buffer.push_str("  No apps registered.\n");
                } else {
                    for app in &apps {
                        let app_type = if app.is_builtin { "B" } else { "E" };
                        self.buffer.push_str(&format!("  [{}] {} v{} - {}\n", app_type, app.name, app.version, app.description));
                    }
                }
            }
            "run" => {
                if parts.len() > 1 {
                    let app_name = parts[1];
                    let app_loader = loader::get_loader_service().lock();
                    if let Some(app) = app_loader.find_app_by_name(app_name) {
                        let app_id = app.app_id;
                        drop(app_loader);
                        let mut app_loader2 = loader::get_loader_service().lock();
                        match app_loader2.launch_app(app_id) {
                            Ok(pid) => self.buffer.push_str(&format!("Launched '{}' as PID {}\n", app_name, pid)),
                            Err(e) => self.buffer.push_str(&format!("Failed to launch '{}': {}\n", app_name, e)),
                        }
                    } else {
                        self.buffer.push_str(&format!("App '{}' not found. Use 'lsapp' to list apps.\n", app_name));
                    }
                } else {
                    self.buffer.push_str("Usage: run <app_name>\n");
                }
            }
            "regapp" => {
                if parts.len() > 1 {
                    let path = parts[1];
                    self.buffer.push_str(&format!("Registering app from '{}'...\n", path));
                    self.buffer.push_str("Note: App registration from file requires ELF file on storage.\n");
                } else {
                    self.buffer.push_str("Usage: regapp <file_path>\n");
                }
            }
            "netstat" => {
                self.buffer.push_str("Network Connections:\n");
                self.buffer.push_str("  (Network stack is in early development)\n");
            }
            "ping" => {
                if parts.len() > 1 {
                    self.buffer.push_str(&format!("Pinging {}...\n", parts[1]));
                    self.buffer.push_str("  (Network stack not yet fully implemented)\n");
                } else {
                    self.buffer.push_str("Usage: ping <host>\n");
                }
            }
            "ls" => {
                if self.fs.is_some() {
                    self.buffer.push_str(&format!("Listing {}:\n", self.current_dir));
                    self.buffer.push_str(".\n");
                    self.buffer.push_str("..\n");
                    self.buffer.push_str("file1.txt\n");
                    self.buffer.push_str("file2.txt\n");
                    self.buffer.push_str("Documents/\n");
                    self.buffer.push_str("Downloads/\n");
                    self.buffer.push_str("Pictures/\n");
                } else {
                    self.buffer.push_str("Filesystem not available\n");
                }
            }
            "cd" => {
                if parts.len() > 1 {
                    let dir = parts[1];
                    if dir == ".." {
                        if self.current_dir != "/" {
                            let mut dirs: Vec<&str> = self.current_dir.trim_matches('/').split('/').collect();
                            dirs.pop();
                            self.current_dir = if dirs.is_empty() { "/".to_string() } else { format!("/{}", dirs.join("/")) };
                        }
                    } else if dir == "." {
                        // stay
                    } else if dir.starts_with('/') {
                        self.current_dir = dir.to_string();
                    } else {
                        if self.current_dir == "/" {
                            self.current_dir = format!("/{}", dir);
                        } else {
                            self.current_dir = format!("{}/{}", self.current_dir, dir);
                        }
                    }
                    self.buffer.push_str(&format!("Changed to {}\n", self.current_dir));
                } else {
                    self.buffer.push_str(&format!("{}\n", self.current_dir));
                }
            }
            "cat" => {
                if parts.len() > 1 {
                    self.buffer.push_str(&format!("Displaying file: {}\n", parts[1]));
                    self.buffer.push_str("File contents would be shown here.\n");
                } else {
                    self.buffer.push_str("Usage: cat <filename>\n");
                }
            }
            "touch" => {
                if parts.len() > 1 {
                    let filename = parts[1];
                    if let Some(ref mut fs) = self.fs {
                        match FileSystemOps::create_file(fs, 2, filename) {
                            Ok(inode) => self.buffer.push_str(&format!("Created file: {} (inode {})\n", filename, inode)),
                            Err(e) => self.buffer.push_str(&format!("Failed to create file: {}\n", e)),
                        }
                    } else {
                        self.buffer.push_str("Filesystem not available\n");
                    }
                } else {
                    self.buffer.push_str("Usage: touch <filename>\n");
                }
            }
            "mkdir" => {
                if parts.len() > 1 {
                    let dirname = parts[1];
                    if let Some(ref mut fs) = self.fs {
                        match FileSystemOps::create_dir(fs, 2, dirname) {
                            Ok(inode) => self.buffer.push_str(&format!("Created directory: {} (inode {})\n", dirname, inode)),
                            Err(e) => self.buffer.push_str(&format!("Failed to create directory: {}\n", e)),
                        }
                    } else {
                        self.buffer.push_str("Filesystem not available\n");
                    }
                } else {
                    self.buffer.push_str("Usage: mkdir <dirname>\n");
                }
            }
            "rm" => {
                if parts.len() > 1 {
                    let filename = parts[1];
                    if let Some(ref mut fs) = self.fs {
                        match FileSystemOps::unlink(fs, 2, filename) {
                            Ok(_) => self.buffer.push_str(&format!("Removed: {}\n", filename)),
                            Err(e) => self.buffer.push_str(&format!("Failed to remove: {}\n", e)),
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
                self.buffer.push_str(&format!("Command not found: {}\n\nType 'help' for a list of available commands.\n", trimmed));
            }
        }

        if !trimmed.is_empty() && trimmed != "cls" && trimmed != "clear" && (self.history.is_empty() || self.history.last().unwrap() != cmd) {
            self.history.push(trimmed.to_string());
        }
        self.history_idx = None;

        if trimmed != "cls" && trimmed != "clear" {
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
            '\x08' | '\x7f' => {
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

        let is_cleared = state.buffer.len() <= 3;
        
        if is_cleared {
            fb.draw_rect(x, y, w, h, 0x00000000);
        } else {
            fb.draw_rect(x, y, w, h, 0x00000000);
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
