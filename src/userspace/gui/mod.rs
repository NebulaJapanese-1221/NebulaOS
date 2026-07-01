use crate::framebuffer::Framebuffer;
mod font;
pub mod window_manager;
pub mod start_menu;

pub const CURSOR_WIDTH: usize = 12;
pub const CURSOR_HEIGHT: usize = 19;

pub const TASKBAR_HEIGHT: u32 = 40;
pub const TITLE_BAR_HEIGHT: u32 = 25;
pub use window_manager::{Window, WindowManager, CURSOR_BITMAP, AppType};

pub fn draw_boot_screen(fb: &mut Framebuffer) {
    // Dark Background
    fb.draw_rect(0, 0, fb.width, fb.height, 0x00000000);

    // Draw a stylized "N" logo for NebulaOS
    let center_x = fb.width / 2;
    let center_y = fb.height / 2;
    fb.draw_rect(center_x - 40, center_y - 50, 20, 100, 0x000078D7); // Left bar
    fb.draw_rect(center_x + 20, center_y - 50, 20, 100, 0x000078D7); // Right bar
    // Diagonal (simplified as three steps)
    fb.draw_rect(center_x - 20, center_y - 30, 20, 30, 0x000078D7);
    fb.draw_rect(center_x, center_y - 10, 20, 30, 0x000078D7);
    fb.draw_rect(center_x + 10, center_y + 10, 10, 20, 0x000078D7);

    fb.present();
}

fn draw_digit(fb: &mut Framebuffer, x: usize, y: usize, digit_idx: usize, color: u32) {
    if digit_idx >= font::FONT_BASIC.len() { return; }
    let glyph = font::FONT_BASIC[digit_idx];
    for row in 0..8 {
        for col in 0..8 {
            if (glyph[row] & (0x80 >> col)) != 0 {
                fb.draw_pixel(x + col, y + row, color);
            }
        }
    }
}

pub fn draw_large_digit(fb: &mut Framebuffer, x: usize, y: usize, digit_idx: usize, color: u32, scale: usize) {
    if digit_idx >= font::FONT_BASIC.len() { return; }
    let glyph = font::FONT_BASIC[digit_idx];
    // Mark the whole digit area as dirty once to avoid excessive merging math in draw_rect
    fb.mark_dirty(x as u32, y as u32, (8 * scale) as u32, (8 * scale) as u32);
    for row in 0..8 {
        for col in 0..8 {
            if (glyph[row] & (0x80 >> col)) != 0 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        fb.draw_pixel(x + (col * scale) + dx, y + (row * scale) + dy, color);
                    }
                }
            }
        }
    }
}

pub fn draw_large_string(fb: &mut Framebuffer, x: usize, y: usize, s: &str, color: u32, scale: usize) {
    for (i, c) in s.chars().enumerate() {
        let idx = match c {
            '0'..='9' => (c as usize) - ('0' as usize),
            ':' => 10,
            'A'..='Z' => (c as usize) - ('A' as usize) + 11,
            'a'..='z' => (c as usize) - ('a' as usize) + 37,
            '.' => 63,
            _ => 65, // Default to space
        };
        draw_large_digit(fb, x + (i * 8 * scale), y, idx, color, scale);
    }
}

pub fn draw_string(fb: &mut Framebuffer, x: usize, y: usize, s: &str, color: u32) {
    for (i, c) in s.chars().enumerate() {
        let idx = match c {
            '0'..='9' => (c as usize) - ('0' as usize),
            ':' => 10,
            'A'..='Z' => (c as usize) - ('A' as usize) + 11,
            'a'..='z' => (c as usize) - ('a' as usize) + 37,
            '.' => 63,
            '/' => 64,
            ' ' => 65,
            '+' => 66,
            '-' => 67,
            '=' => 68,
            _ => continue,
        };
        draw_digit(fb, x + (i * 8), y, idx, color);
    }
}

pub fn render_ui(fb: &mut Framebuffer, start_menu_open: bool, h: u8, m: u8, s: u8, windows: &[Window]) {
    let sw = fb.width;
    let sh = fb.height;
    let ty = sh - TASKBAR_HEIGHT as usize;

    // 1. Draw Desktop (Wallpaper)
    fb.draw_rect(0, 0, sw, ty, 0x00003366);

    // 2. Draw Taskbar (bottom)
    fb.draw_rect(0, ty, sw, TASKBAR_HEIGHT as usize, 0x00333333); // Dark Gray
    // Taskbar Top border
    fb.draw_rect(0, ty - 1, sw, 1, 0x00444444);
    
    // 3. Draw Start Button
    fb.draw_rect(5, ty + 5, 70, 30, 0x000078D7); // Windows Blue

    // 4. Draw Clock area
    fb.draw_rect(sw - 84, ty + 5, 80, 30, 0x00222222);
    
    // Render Time string (HH:MM:SS)
    let start_x = sw - 79;
    let start_y = ty + 16;
    draw_digit(fb, start_x,      start_y, (h / 10) as usize, 0xFFFFFF);
    draw_digit(fb, start_x + 8,  start_y, (h % 10) as usize, 0xFFFFFF);
    draw_digit(fb, start_x + 16, start_y, 10, 0xFFFFFF); // :
    draw_digit(fb, start_x + 24, start_y, (m / 10) as usize, 0xFFFFFF);
    draw_digit(fb, start_x + 32, start_y, (m % 10) as usize, 0xFFFFFF);
    draw_digit(fb, start_x + 40, start_y, 10, 0xFFFFFF); // :
    draw_digit(fb, start_x + 48, start_y, (s / 10) as usize, 0xFFFFFF);
    draw_digit(fb, start_x + 56, start_y, (s % 10) as usize, 0xFFFFFF);

    // 4.5 Draw Taskbar Items for minimized windows
    let mut item_x = 80;
    for window in windows {
        if window.is_minimized {
            fb.draw_rect(item_x, ty + 5, 110, 30, 0x00444444); // Button background
            draw_string(fb, item_x + 5, ty + 16, window.title, 0xFFFFFF);
            item_x += 115;
        }
    }

    // 5. Draw Start Menu
    if start_menu_open {
        start_menu::draw(fb, ty as u32);
    }
}