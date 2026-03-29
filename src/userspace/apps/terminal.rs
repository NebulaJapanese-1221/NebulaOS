use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::ata::AtaDrive;
use core::sync::atomic::Ordering;

#[derive(Clone)]
pub struct Terminal {
    pub history: Vec<String>,
    prompt: String,
    current_input: String,
}

impl Terminal {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            prompt: String::from("root@nebula: / # "),
            current_input: String::new(),
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        self.history.push(format!("{}{}", self.prompt, cmd));
        
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0] {
            "help" => {
                let locale_guard = crate::userspace::localisation::CURRENT_LOCALE.lock();
                let loc = locale_guard.as_ref().unwrap();
                self.history.push(format!("--- {} ---", loc.ctx_new_terminal()));
                self.history.push("Available commands:".to_string());
                self.history.push("  help      - Show this help message".to_string());
                self.history.push("  uname     - Show system information".to_string());
                self.history.push("  uptime    - Show system uptime".to_string());
                self.history.push("  clear     - Clear the screen".to_string());
                self.history.push("  ls        - List files (ATA Driver Test)".to_string());
            }
            "uname" => {
                let locale_guard = crate::userspace::localisation::CURRENT_LOCALE.lock();
                let loc = locale_guard.as_ref().unwrap();
                self.history.push(format!("{}: NebulaOS {}", loc.info_kernel(), crate::kernel::VERSION));
                self.history.push(format!("{}: i386-unknown-none", loc.info_target()));
                if let Some(brand) = crate::kernel::cpu::CPU_BRAND.lock().as_ref() {
                    self.history.push(format!("CPU: {}", brand));
                }
            }
            "uptime" => {
                 let total_seconds = crate::kernel::process::TICKS.load(Ordering::Relaxed) / 1000;
                 self.history.push(format!("Uptime: {}s", total_seconds));
            }
            "clear" => {
                self.history.clear();
            }
            "ls" => {
                let drive = AtaDrive::new(true, true); // Primary Master
                // We read the first sector (LBA 0) to verify access
                let sector = drive.read_sectors(0, 1);
                
                if sector.len() == 512 {
                    self.history.push("[ATA] Primary Master: Read Success".to_string());
                    
                    // Check for MBR signature (0x55, 0xAA at end of sector)
                    if sector[510] == 0x55 && sector[511] == 0xAA {
                         self.history.push("Filesystem: Detected (MBR)".to_string());
                         // Mock listing demonstrating 'files' were found on the disk
                         self.history.push("Contents of /:".to_string());
                         self.history.push("  <DIR>  system".to_string());
                         self.history.push("  <DIR>  userspace".to_string());
                         self.history.push("         kernel.elf  [1024 KB]".to_string());
                         self.history.push("         boot.cfg    [2 KB]".to_string());
                    } else {
                        self.history.push("Filesystem: Unknown / Unformatted".to_string());
                    }
                } else {
                    self.history.push("Error: Failed to read from ATA drive.".to_string());
                }
            }
            _ => {
                self.history.push(format!("Unknown command: {}", parts[0]));
            }
        }
    }
}

impl App for Terminal {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window) {
        // Draw background
        gui::draw_rect(fb, win.x, win.y + 20, win.width, win.height - 20, 0x00_00_00_00, None); // Black

        let font_height = 16;
        let line_spacing = 2;
        let max_lines = (win.height - 25) / (font_height + line_spacing);
        
        // Calculate visible range (Simple tail log)
        let total_lines = self.history.len() + 1;
        let start_line = if total_lines > max_lines { total_lines - max_lines } else { 0 };

        let mut y = win.y + 25;
        let x = win.x + 5;

        for i in start_line..self.history.len() {
             font::draw_string(fb, x, y, self.history[i].as_str(), 0x00_CC_CC_CC, None);
             y += (font_height + line_spacing) as isize;
        }

        let input_line = format!("{}{}_", self.prompt, self.current_input);
        font::draw_string(fb, x, y, input_line.as_str(), 0x00_FF_FF_FF, None);
    }

    fn handle_event(&mut self, event: &AppEvent) {
        if let AppEvent::KeyPress { key } = event {
            if *key == '\n' { let cmd = self.current_input.clone(); self.current_input.clear(); self.execute_command(&cmd); }
            else if *key == '\x08' { self.current_input.pop(); }
            else if !key.is_control() { self.current_input.push(*key); }
        }
    }

    fn box_clone(&self) -> Box<dyn App> { Box::new(self.clone()) }
}