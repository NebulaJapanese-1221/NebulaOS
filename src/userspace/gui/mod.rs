//! GUI components for NebulaOS.

pub mod widgets;
pub mod shell;
pub mod window_manager;

use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use spin::Mutex;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// Re-exports for convenience
pub use self::widgets::{rect, button};
pub use self::widgets::rect::Rect;
pub use self::widgets::button::Button;
pub use self::shell::Shell;
pub use self::window_manager::{WindowManager, Window};
pub use crate::userspace::fonts::font;

pub static DESKTOP_GRADIENT_START: AtomicU32 = AtomicU32::new(0x00_10_20_40);
pub static DESKTOP_GRADIENT_END: AtomicU32 = AtomicU32::new(0x00_50_80_B0);
pub static FULL_REDRAW_REQUESTED: AtomicBool = AtomicBool::new(false);
pub static HIGH_CONTRAST: AtomicBool = AtomicBool::new(false);
pub static LARGE_TEXT: AtomicBool = AtomicBool::new(false);
pub static MOUSE_SENSITIVITY: AtomicU32 = AtomicU32::new(100);

pub fn draw_rect(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, width: usize, height: usize, color: u32, clip: Option<Rect>) {    
    widgets::rect::draw_rect(fb, x, y, width, height, color, clip)
}

/// Draws an animated loading spinner in the center of the given area.
pub fn draw_loading_spinner(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, clip: Rect) {
    let ticks = crate::kernel::process::TICKS.load(Ordering::Relaxed);
    let frame = (ticks / 80) % 8;
    
    let bg_color = 0x00_20_20_20;
    let box_size = 80;
    draw_rect(fb, x - (box_size / 2), y - (box_size / 2), box_size as usize, box_size as usize, bg_color, Some(clip));
    // White Border
    draw_rect(fb, x - (box_size / 2), y - (box_size / 2), box_size as usize, 1, 0x00_FF_FF_FF, Some(clip));
    draw_rect(fb, x - (box_size / 2), y + (box_size / 2) - 1, box_size as usize, 1, 0x00_FF_FF_FF, Some(clip));
    draw_rect(fb, x - (box_size / 2), y - (box_size / 2), 1, box_size as usize, 0x00_FF_FF_FF, Some(clip));
    draw_rect(fb, x + (box_size / 2) - 1, y - (box_size / 2), 1, box_size as usize, 0x00_FF_FF_FF, Some(clip));

    for i in 0..8 {
        let dot_color = if i == frame as usize { 0x00_00_FF_FF } else { 0x00_60_60_60 };
        let (ox, oy) = match i {
            0 => (0, -15), 1 => (11, -11), 2 => (15, 0), 3 => (11, 11),
            4 => (0, 15), 5 => (-11, 11), 6 => (-15, 0), 7 => (-11, -11),
            _ => (0, 0)
        };
        draw_rect(fb, x + ox - 3, y + oy - 3, 6, 6, dot_color, Some(clip));
    }
    
    font::draw_string(fb, x - 35, y + 25, "Starting...", 0x00_FF_FF_FF, Some(clip));
}

/// Linearly interpolates between two colors.
pub fn interpolate_color(src: u32, dest: u32, step: u32, total_steps: u32) -> u32 {
    if total_steps == 0 { return dest; }
    let step = step.min(total_steps);

    let r_s = (src >> 16) & 0xFF;
    let g_s = (src >> 8) & 0xFF;
    let b_s = src & 0xFF;

    let r_d = (dest >> 16) & 0xFF;
    let g_d = (dest >> 8) & 0xFF;
    let b_d = dest & 0xFF;

    let r = (r_s as i32 + (r_d as i32 - r_s as i32) * step as i32 / total_steps as i32) as u32;
    let g = (g_s as i32 + (g_d as i32 - g_s as i32) * step as i32 / total_steps as i32) as u32;
    let b = (b_s as i32 + (b_d as i32 - b_s as i32) * step as i32 / total_steps as i32) as u32;

    (r << 16) | (g << 8) | b
}

/// Fades all pixels in the current draw buffer toward black based on the given step.
pub fn fade_buffer(fb: &mut framebuffer::Framebuffer, step: u32, total_steps: u32) {
    if let Some(ref mut buffer) = fb.draw_buffer {
        for pixel in buffer.iter_mut() {
            // Interpolate from black (0x000000) to the current pixel color
            *pixel = interpolate_color(0, *pixel, step, total_steps);
        }
    }
}

pub static WINDOW_MANAGER: Mutex<WindowManager> = Mutex::new(WindowManager::new());

/// Initializes the global Window Manager and sets up the backbuffer.
pub fn init() {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(info) = FRAMEBUFFER.lock().info.as_ref() {
        // Allocate backbuffer based on current resolution
        wm.backbuffer.resize(info.width * info.height, 0);
        // Mark the entire screen as dirty for the first frame
        wm.mark_dirty(Rect { x: 0, y: 0, width: info.width, height: info.height });
    }
}

/// Main GUI update loop, usually called by the kernel's shell task.
pub fn update() {
    // Acquire the lock and process events/drawing
    WINDOW_MANAGER.lock().update();
}