// System Monitor Application for NebulaOS
// Displays CPU, memory, and process information

use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct SystemMonitorState {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub processes: Vec<ProcessInfo>,
    pub update_timer: usize,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: usize,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: usize,
    pub state: String,
}

impl SystemMonitorState {
    pub fn new() -> Self {
        // Generate some dummy data
        let mut processes = Vec::new();
        processes.push(ProcessInfo {
            pid: 1,
            name: String::from("init"),
            cpu_usage: 0.5,
            memory_usage: 1024,
            state: String::from("Running"),
        });
        processes.push(ProcessInfo {
            pid: 2,
            name: String::from("shell"),
            cpu_usage: 1.2,
            memory_usage: 2048,
            state: String::from("Running"),
        });
        processes.push(ProcessInfo {
            pid: 3,
            name: String::from("terminal"),
            cpu_usage: 0.8,
            memory_usage: 1536,
            state: String::from("Running"),
        });
        
        SystemMonitorState {
            cpu_usage: 15.5,
            memory_usage: 30.2,
            processes,
            update_timer: 0,
        }
    }
    
    pub fn update(&mut self) {
        self.update_timer += 1;
        
        // Simulate changing values
        if self.update_timer % 10 == 0 {
            self.cpu_usage = 10.0 + (self.update_timer % 20) as f32 * 0.8;
            self.memory_usage = 25.0 + (self.update_timer % 15) as f32 * 1.0;
            
            // Update process info
            for process in &mut self.processes {
                process.cpu_usage = 0.1 + (self.update_timer % 10) as f32 * 0.2;
            }
        }
    }
}

pub struct SystemMonitorApp;

impl SystemMonitorApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &mut SystemMonitorState) {
        state.update();
        
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Draw background
        fb.draw_rect(x, y, w, h, 0x00F0F0F0);

        // Draw CPU usage gauge
        Self::draw_gauge(fb, x + 10, y + 10, w / 3 - 20, 100, "CPU Usage", state.cpu_usage, 0x0000FF00);
        
        // Draw Memory usage gauge
        Self::draw_gauge(fb, x + w / 3 + 10, y + 10, w / 3 - 20, 100, "Memory Usage", state.memory_usage, 0x000000FF);
        
        // Draw Process List
        fb.draw_rect(x + 10, y + 120, w - 20, h - 130, 0x00FFFFFF);
        draw_string(fb, x + 15, y + 125, "Processes:", 0x00000000);
        
        // Draw process table headers
        draw_string(fb, x + 15, y + 145, "PID", 0x00000000);
        draw_string(fb, x + 60, y + 145, "Name", 0x00000000);
        draw_string(fb, x + 180, y + 145, "CPU%", 0x00000000);
        draw_string(fb, x + 240, y + 145, "Memory", 0x00000000);
        draw_string(fb, x + 320, y + 145, "State", 0x00000000);
        
        // Draw process list
        let mut line_y = y + 165;
        for process in &state.processes {
            draw_string(fb, x + 15, line_y, &format!("{}", process.pid), 0x00000000);
            draw_string(fb, x + 60, line_y, &process.name, 0x00000000);
            draw_string(fb, x + 180, line_y, &format!("{:.1}", process.cpu_usage), 0x00000000);
            draw_string(fb, x + 240, line_y, &format!("{} KB", process.memory_usage), 0x00000000);
            draw_string(fb, x + 320, line_y, &process.state, 0x00000000);
            line_y += 15;
        }
    }
    
    fn draw_gauge(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, label: &str, value: f32, color: u32) {
        // Draw gauge background
        fb.draw_rect(x, y, w, h, 0x00FFFFFF);
        fb.draw_rect(x, y, w, 1, 0x00000000);
        
        // Draw label
        draw_string(fb, x + 5, y + 5, label, 0x00000000);
        
        // Draw value
        draw_string(fb, x + 5, y + 20, &format!("{:.1}%", value), 0x00000000);
        
        // Draw gauge bar
        let bar_width = (w - 10) as f32 * (value / 100.0);
        fb.draw_rect(x + 5, y + 40, bar_width as usize, 20, color);
        
        // Draw gauge outline
        fb.draw_rect(x + 5, y + 40, w - 10, 20, 0x00000000, true);
    }

    pub fn handle_click(_state: &mut SystemMonitorState, _bounds: Rect, _mx: i32, _my: i32) {
        // Handle clicks on processes or other interactive elements
    }

    pub fn handle_keyboard_input(_state: &mut SystemMonitorState, _c: char) {
        // Handle keyboard input for process management
    }
}