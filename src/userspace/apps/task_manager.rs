use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::format;
use crate::kernel::process::SCHEDULER;
use crate::kernel::allocator::ALLOCATOR;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use core::sync::atomic::Ordering;

struct TaskManagerState {
    last_tick: usize,
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
                last_tick: 0,
                dirty: true,
                task_snapshot: Vec::new(),
            })),
        }
    }
}

impl App for TaskManager {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if gui::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;

        // Draw background
        gui::draw_rect(fb, win.x, win.y + title_height as isize, win.width, win.height - title_height, 0x00_12_12_12, Some(dirty_rect));

        let mut y = win.y + title_height as isize + 5;
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

        // --- System Statistics ---
        y += 10;
        font::draw_string(fb, x, y, "System Statistics", 0x00_FF_FF_FF, Some(dirty_rect));
        y += font_height as isize + 8;

        let total_mem = crate::kernel::TOTAL_MEMORY.load(Ordering::Relaxed) / 1024 / 1024;
        let used_mem = ALLOCATOR.lock().used() / 1024 / 1024;
        let cpu_usage = crate::kernel::cpu::CPU_USAGE.load(Ordering::Relaxed);

        font::draw_string(fb, x, y, &format!("Mem: {} / {} MB", used_mem, total_mem), 0x00_E1_E1_E1, Some(dirty_rect));
        y += font_height as isize + 4;
        font::draw_string(fb, x, y, &format!("CPU: {}%", cpu_usage), 0x00_E1_E1_E1, Some(dirty_rect));
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
