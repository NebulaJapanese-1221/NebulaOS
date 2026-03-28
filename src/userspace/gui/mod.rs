//! GUI components for NebulaOS.

pub mod widgets;
pub mod shell;
pub mod window_manager;

use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use spin::Mutex;
use core::sync::atomic::{AtomicBool, AtomicU32};

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