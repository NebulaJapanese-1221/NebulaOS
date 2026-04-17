use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect};
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
    mem_graph_buffer: Vec<u32>,
    cpu_graph_buffer: Vec<u32>,
    dirty: bool,
    task_snapshot: Vec<(usize, &'static str)>,
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
                mem_graph_buffer: Vec::new(),
                cpu_graph_buffer: Vec::new(),
                dirty: true,
                task_snapshot: Vec::new(),
            })),
        }
    }
}

impl App for TaskManager {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        // Draw background
        gui::draw_rect(fb, win.x, win.y + 20, win.width, win.height - 20, 0x00_00_00_00, Some(dirty_rect));

        let font_height = if gui::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
        let mut y = win.y + 20 + 5;
        let x = win.x + 5;

        // Draw Header
        font::draw_string(fb, x, y, "ID   State", 0x00_FF_FF_FF, Some(dirty_rect));
        y += font_height as isize + 5;
        
        // Draw Separator
        gui::draw_rect(fb, x, y, win.width - 10, 1, 0x00_80_80_80, Some(dirty_rect));
        y += 5;

        let state_lock = self.state.lock();
        for (id, status) in &state_lock.task_snapshot {
            let line = format!("{:<4} {}", id, status);
            font::draw_string(fb, x, y, &line, 0x00_CC_CC_CC, Some(dirty_rect));
            y += font_height as isize + 2;
        }

        // --- Draw Memory Graph ---
        y += 10;
        font::draw_string(fb, x, y, "Memory Usage (MB)", 0x00_FF_FF_FF, Some(dirty_rect));
        y += font_height as isize + 2;

        let graph_height = 100;
        let graph_width = if win.width > 20 { win.width - 20 } else { 10 };

        // Memory Graph Blit
        if dirty_rect.intersects(&Rect { x, y, width: graph_width, height: graph_height }) {
            fb.draw_bitmap(x as usize, y as usize, graph_width, graph_height, &state_lock.mem_graph_buffer);
        }

        let total_mem = crate::kernel::TOTAL_MEMORY.load(Ordering::Relaxed) / 1024 / 1024;
        let mem_scale_max = if total_mem > 0 { total_mem } else { 128 };
        let last_mem = state_lock.mem_history.last().copied().unwrap_or(0);
        font::draw_string(fb, x + 5, y + 5, &format!("{} / {} MB", last_mem, mem_scale_max), 0x00_FFFFFF, Some(dirty_rect));

        // --- Draw CPU Graph ---
        y += graph_height as isize + 15;
        if y + graph_height as isize + 20 < win.y + win.height as isize {
            font::draw_string(fb, x, y, "CPU Usage (%)", 0x00_FF_FF_FF, Some(dirty_rect));
            y += font_height as isize + 2;

            // CPU Graph Blit
            if dirty_rect.intersects(&Rect { x, y, width: graph_width, height: graph_height }) {
                fb.draw_bitmap(x as usize, y as usize, graph_width, graph_height, &state_lock.cpu_graph_buffer);
            }

            let last_cpu = state_lock.cpu_history.last().copied().unwrap_or(0);
            font::draw_string(fb, x + 5, y + 5, &format!("{} %", last_cpu), 0x00_FFFFFF, Some(dirty_rect));
        }
    }

    fn handle_event(&mut self, event: &AppEvent, win: &Window) -> Option<Rect> {
        if let AppEvent::Tick { tick_count } = event {
            let mut state = self.state.lock();
            // Update every 200ms approx
            if *tick_count > state.last_tick + 200 {
                state.last_tick = *tick_count;
                
                // Snapshot tasks safely outside the draw loop
                state.task_snapshot.clear();
                let scheduler = SCHEDULER.lock();
                for (i, task) in scheduler.tasks.iter().enumerate() {
                    let status = if i == scheduler.current_index { "Running" } else { "Ready" };
                    state.task_snapshot.push((task.id, status));
                }
                drop(scheduler);

                let used_bytes = ALLOCATOR.lock().used();
                state.mem_history.push(used_bytes / 1024 / 1024);
                let cpu_usage = crate::kernel::cpu::CPU_USAGE.load(Ordering::Relaxed);
                state.cpu_history.push(cpu_usage as usize);

                let graph_width = if win.width > 20 { win.width - 20 } else { 10 };
                let max_points = graph_width / 4;
                if state.mem_history.len() > max_points { state.mem_history.remove(0); }
                if state.cpu_history.len() > max_points { state.cpu_history.remove(0); }

                // --- Internal Redraw: Update Graph Buffers ---
                let graph_height = 100;
                let buf_size = graph_width * graph_height;
                
                // Update Memory Graph
                state.mem_graph_buffer.resize(buf_size, 0);
                state.mem_graph_buffer.fill(0xFF_12_12_12);
                let total_mem = crate::kernel::TOTAL_MEMORY.load(Ordering::Relaxed) / 1024 / 1024;
                let mem_scale_max = if total_mem > 0 { total_mem } else { 128 };

                for (i, &val) in state.mem_history.iter().enumerate() {
                    let bar_h = (val * graph_height) / mem_scale_max;
                    let bar_x = i * 4;
                    if bar_x + 3 < graph_width {
                        let start_y = graph_height.saturating_sub(bar_h);
                        for by in start_y..graph_height {
                            for bx in 0..3 {
                                state.mem_graph_buffer[by * graph_width + (bar_x + bx)] = 0xFF_00_7A_CC;
                            }
                        }
                    }
                }

                // Update CPU Graph
                state.cpu_graph_buffer.resize(buf_size, 0);
                state.cpu_graph_buffer.fill(0xFF_12_12_12);

                for (i, &val) in state.cpu_history.iter().enumerate() {
                    let safe_val = val.min(100);
                    let bar_h = (safe_val * graph_height) / 100;
                    let bar_x = i * 4;
                    let color = if safe_val < 50 { 0xFF_00_AA_00 } 
                               else if safe_val < 80 { 0xFF_AA_AA_00 } 
                               else { 0xFF_AA_00_00 };

                    if bar_x + 3 < graph_width {
                        let start_y = graph_height.saturating_sub(bar_h);
                        for by in start_y..graph_height {
                            for bx in 0..3 {
                                state.cpu_graph_buffer[by * graph_width + (bar_x + bx)] = color;
                            }
                        }
                    }
                }

                state.dirty = false;
                return Some(win.rect());
            }
        }
        None
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}
