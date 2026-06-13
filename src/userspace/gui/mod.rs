use crate::framebuffer::Framebuffer;
mod font;
pub mod window_manager;
pub mod start_menu;

pub const CURSOR_WIDTH: usize = 12;
pub const CURSOR_HEIGHT: usize = 19;

pub const TITLE_BAR_HEIGHT: u32 = 25;
pub use window_manager::{Window, WindowManager, CURSOR_BITMAP, AppType};

pub fn draw_boot_screen(fb: &mut Framebuffer) {
    // Dark Background
    fb.draw_rect(0, 0, 1024, 768, 0x00000000);

    // Draw a stylized "N" logo for NebulaOS
    let center_x = 512;
    let center_y = 384;
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
    // 1. Draw Desktop (Wallpaper)
    fb.draw_rect(0, 0, 1024, 728, 0x00003366);

    // 2. Draw Taskbar (bottom)
    fb.draw_rect(0, 728, 1024, 40, 0x00333333); // Dark Gray
    // Taskbar Top border
    fb.draw_rect(0, 727, 1024, 1, 0x00444444);
    
    // 3. Draw Start Button
    fb.draw_rect(5, 733, 70, 30, 0x000078D7); // Windows Blue

    // 4. Draw Clock area
    fb.draw_rect(940, 733, 80, 30, 0x00222222);
    
    // Render Time string (HH:MM:SS)
    let start_x = 945;
    let start_y = 744;
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
            fb.draw_rect(item_x, 733, 110, 30, 0x00444444); // Button background
            draw_string(fb, item_x + 5, 744, window.title, 0xFFFFFF);
            item_x += 115;
        }
    }

    // 5. Draw Start Menu
    if start_menu_open {
        start_menu::draw(fb);
    }
}