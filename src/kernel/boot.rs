use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::fonts::font;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use super::cpu;

pub static BOOT_ANIM_FRAME: AtomicUsize = AtomicUsize::new(0);
pub static BOOT_ANIM_RUNNING: AtomicBool = AtomicBool::new(true);
pub static BOOT_PROGRESS_DISPLAY: AtomicUsize = AtomicUsize::new(0);

/// Pre-calculated offsets for a 12-spoke loading wheel (30-degree increments).
const SPOKE_OFFSETS: [(isize, isize); 12] = [
    (0, -20), (10, -17), (17, -10), (20, 0),
    (17, 10), (10, 17), (0, 20), (-10, 17),
    (-17, 10), (-20, 0), (-17, -10), (-10, -17)
];

/// Pre-assembled u32 colors for the spinner.
const SPINNER_COLORS: [u32; 12] = [
    0x00_00_CC_FF, 0x00_00_AA_EE, 0x00_00_88_CC, 0x00_00_66_AA, 0x00_00_44_88, 0x00_00_22_66,
    0x00_10_10_30, 0x00_10_10_30, 0x00_10_10_30, 0x00_10_10_30, 0x00_10_10_30, 0x00_10_10_30,
];

pub(crate) fn draw_spinner(fb: &mut crate::drivers::framebuffer::Framebuffer, cx: isize, cy: isize) {
    let frame = if BOOT_ANIM_RUNNING.load(Ordering::Relaxed) {
        BOOT_ANIM_FRAME.fetch_add(1, Ordering::Relaxed)
    } else {
        BOOT_ANIM_FRAME.load(Ordering::Relaxed)
    };

    let head = (frame % 12) as usize;

    for i in 0..12 {
        let color = SPINNER_COLORS[(i + 12 - head) % 12];
        let (dx, dy) = SPOKE_OFFSETS[i];

        for r in (8..=20).step_by(2) {
            let px = cx + (dx * r as isize / 20);
            let py = cy + (dy * r as isize / 20);
            fb.set_pixel(px as usize, py as usize, color);
        }
    }
}

pub fn draw_boot_screen_content(fb: &mut crate::drivers::framebuffer::Framebuffer, status: &str, progress: usize) {
    let (width, height) = match fb.info.as_ref() {
        Some(info) => (info.width, info.height),
        None => return,
    };

    // Clear the center area for drawing
    crate::userspace::gui::draw_rect(fb, (width / 2) as isize - 220, (height / 2) as isize - 100, 440, 250, 0x00_050515, None);

    let title = "NebulaOS";
    let x_title = (width / 2).saturating_sub((title.len() * 8) / 2);
    let y_title = (height / 2).saturating_sub(60);

    // Title with subtle shadow
    font::draw_string(fb, x_title as isize + 2, y_title as isize + 2, title, 0x00_001133, None);
    font::draw_string(fb, x_title as isize, y_title as isize, title, 0x00_00CCFF, None);

    if crate::kernel::IS_SAFE_MODE.load(Ordering::Relaxed) {
        font::draw_string(fb, x_title as isize - 18, y_title as isize - 25, "[ SAFE MODE ]", 0x00_FFCC00, None);
    }

    draw_spinner(fb, (width / 2) as isize, (height / 2) as isize - 10);

    let x_status = (width / 2).saturating_sub((status.len() * 8) / 2);
    font::draw_string(fb, x_status as isize, (height / 2) as isize + 40, status, 0x00_888888, None);

    draw_progress_bar_internal(fb, progress, width, height);
}

pub fn draw_boot_screen(status: &str, progress: usize) {
    let mut fb = FRAMEBUFFER.lock();
    let (width, height) = match fb.info.as_ref() {
        Some(info) => (info.width, info.height),
        None => return,
    };

    draw_boot_screen_content(&mut fb, status, progress);
    fb.present_rect(width / 2 - 220, (height / 2).saturating_sub(100), 440, 250);
}

pub fn add_boot_status(status: &str, target_progress: usize) {
    let current = BOOT_PROGRESS_DISPLAY.load(Ordering::Relaxed);
    if target_progress > current {
        for p in current..=target_progress {
            BOOT_PROGRESS_DISPLAY.store(p, Ordering::Relaxed);
            draw_boot_screen(status, p);
            cpu::spin_wait_us(5000); 
        }
    } else {
        draw_boot_screen(status, target_progress);
    }
}

fn draw_progress_bar_internal(fb: &mut crate::drivers::framebuffer::Framebuffer, progress: usize, width: usize, height: usize) {
    let bar_width = 300;
    let bar_height = 4;
    let x = (width / 2).saturating_sub(bar_width / 2);
    let y = (height / 2) + 80;
    
    // Draw subtle container border
    crate::userspace::gui::draw_rect(fb, x as isize - 2, y as isize - 2, bar_width + 4, bar_height + 4, 0x00_1A1A2A, None);

    let info = if let Some(i) = fb.info.as_ref() { i } else { return };
    let buffer = if let Some(b) = fb.draw_buffer.as_mut() { b } else { return };

    // Draw background track
    for py in y..(y + bar_height) {
        let offset = py * info.width + x;
        buffer[offset..offset + bar_width].fill(0x00_0D0D1D);
    }

    // Draw fill
    let filled_width = (bar_width * progress) / 100;
    for py in y..(y + bar_height) {
        let offset = py * info.width + x;
        buffer[offset..offset + filled_width].fill(0x00_00CCFF);
    }
}