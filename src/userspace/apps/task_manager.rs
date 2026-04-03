use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::format;
use crate::kernel::process::{SCHEDULER, TICKS};
use crate::kernel::allocator::ALLOCATOR;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use core::sync::atomic::Ordering;

struct TaskManagerState {
    mem_history: Vec<usize>,
    cpu_history: Vec<usize>,
    last_tick: usize,
    graph_buffer: Vec<u32>,
}

#[derive(Clone)]
pub struct TaskManager {
    state: Arc<Mutex<TaskManagerState>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TaskManagerState {
                mem_history: Vec::new(),
                cpu_history: Vec::new(),
                last_tick: 0,
                graph_buffer: Vec::new(),
            })),
        }
    }
}

impl App for TaskManager {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window) {
        // Draw background
        gui::draw_rect(fb, win.x, win.y + 20, win.width, win.height - 20, 0x00_00_00_00, None);

        let font_height = if gui::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
        let mut y = win.y + 20 + 5;
        let x = win.x + 5;

        // Draw Header
        font::draw_string(fb, x, y, "ID   State", 0x00_FF_FF_FF, None);
        y += font_height as isize + 5;
        
        // Draw Separator
        gui::draw_rect(fb, x, y, win.width - 10, 1, 0x00_80_80_80, None);
        y += 5;

        let scheduler = SCHEDULER.lock();
        for (i, task) in scheduler.tasks.iter().enumerate() {
            let state = if i == scheduler.current_index { "Running" } else { "Ready" };
            let line = format!("{:<4} {}", task.id, state);
            font::draw_string(fb, x, y, &line, 0x00_CC_CC_CC, None);
            y += font_height as isize + 2;
        }

        // --- Draw Memory Graph ---
        y += 10;
        font::draw_string(fb, x, y, "Memory Usage (MB)", 0x00_FF_FF_FF, None);
        y += font_height as isize + 2;

        let graph_height = 100;
        let graph_width = if win.width > 20 { win.width - 20 } else { 10 };

        // Update logic
        let mut state = self.state.lock();
        let current_tick = TICKS.load(Ordering::Relaxed);

        // Update every 200ms approx (200 ticks at 1000Hz)
        if current_tick > state.last_tick + 200 {
            state.last_tick = current_tick;
            let used_bytes = ALLOCATOR.lock().used();
            state.mem_history.push(used_bytes / 1024 / 1024);

            // Read CPU usage from kernel stats
            let cpu_usage = crate::kernel::cpu::CPU_USAGE.load(Ordering::Relaxed);
            state.cpu_history.push(cpu_usage as usize);

            // Limit history size to fit graph (4px per bar)
            let max_points = graph_width / 4;
            if state.mem_history.len() > max_points {
                state.mem_history.remove(0);
            }
            if state.cpu_history.len() > max_points {
                state.cpu_history.remove(0);
            }
        }

        // Draw bars
        let total_mem = crate::kernel::TOTAL_MEMORY.load(Ordering::Relaxed) / 1024 / 1024;
        let mem_scale_max = if total_mem > 0 { total_mem } else { 128 };

        // Resize and clear local buffer
        let buf_size = graph_width * graph_height;
        if state.graph_buffer.len() != buf_size {
            state.graph_buffer.resize(buf_size, 0);
        }
        state.graph_buffer.fill(0xFF_20_20_20); // Opaque dark gray background

        let mem_history = state.mem_history.clone();
        for (i, &val) in mem_history.iter().enumerate() {
            let bar_h = (val * graph_height) / mem_scale_max;
            let bar_x = i * 4;
            
            if bar_x + 3 < graph_width {
                let start_y = graph_height.saturating_sub(bar_h);
                for by in start_y..graph_height {
                    for bx in 0..3 {
                        state.graph_buffer[by * graph_width + (bar_x + bx)] = 0xFF_00_AA_00;
                    }
                }
            }
        }
        // Blit memory graph
        fb.draw_bitmap(x as usize, y as usize, graph_width, graph_height, &state.graph_buffer);

        let last_mem = state.mem_history.last().copied().unwrap_or(0);
        font::draw_string(fb, x + 5, y + 5, &format!("{} / {} MB", last_mem, mem_scale_max), 0x00_FFFFFF, None);

        // --- Draw CPU Graph ---
        y += graph_height as isize + 15;
        if y + graph_height as isize + 20 < win.y + win.height as isize {
            font::draw_string(fb, x, y, "CPU Usage (%)", 0x00_FF_FF_FF, None);
            y += font_height as isize + 2;

            // Reuse buffer for CPU graph
            state.graph_buffer.fill(0xFF_20_20_20);

            let cpu_history = state.cpu_history.clone();
            for (i, &val) in cpu_history.iter().enumerate() {
                // Clamp to 100%
                let safe_val = if val > 100 { 100 } else { val };
                let bar_h = (safe_val * graph_height) / 100;
                let bar_x = i * 4;

                // Color gradient based on load (Green -> Yellow -> Red)
                let color = if safe_val < 50 { 0xFF_00_AA_00 } 
                           else if safe_val < 80 { 0xFF_AA_AA_00 } 
                           else { 0xFF_AA_00_00 };

                if bar_x + 3 < graph_width {
                    let start_y = graph_height.saturating_sub(bar_h);
                    for by in start_y..graph_height {
                        for bx in 0..3 {
                            state.graph_buffer[by * graph_width + (bar_x + bx)] = color;
                        }
                    }
                }
            }
            
            // Blit CPU graph
            fb.draw_bitmap(x as usize, y as usize, graph_width, graph_height, &state.graph_buffer);

            let last_cpu = state.cpu_history.last().copied().unwrap_or(0);
            font::draw_string(fb, x + 5, y + 5, &format!("{} %", last_cpu), 0x00_FFFFFF, None);
        }
    }

    fn handle_event(&mut self, _event: &AppEvent) {
        // No interaction required for now
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}
