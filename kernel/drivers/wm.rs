// Window Manager - manages windows on the desktop

use crate::drivers::vesa;

pub struct Window {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub title: &'static str,
    pub visible: bool,
    pub focused: bool,
}

impl Window {
    pub fn new(id: u8, x: u16, y: u16, width: u16, height: u16, title: &'static str) -> Self {
        Window {
            id,
            x,
            y,
            width,
            height,
            title,
            visible: true,
            focused: false,
        }
    }

    pub fn render(&self) {
        if !self.visible {
            return;
        }

        let title_bg: u32 = 0x000080; // Dark blue title bar
        let border_color: u32 = 0x0000FF; // Blue border
        let content_color: u32 = 0x000000; // Black content

        // Draw window border
        vesa::draw_rect(self.x, self.y, self.width, self.height, border_color);

        // Draw title bar
        vesa::fill_rect(self.x + 2, self.y + 2, self.width - 4, 16, title_bg);

        // Draw title text (simple text rendering)
        let title_bytes = self.title.as_bytes();
        let mut x = self.x + 4;
        for &ch in title_bytes.iter() {
            if ch < 32 || ch > 126 {
                continue;
            }
            // Draw simple 8x8 character
            draw_char(x, self.y + 4, ch, 0xFFFFFF);
            x += 8;
        }

        // Draw close button (X in red box)
        let close_x = self.x + self.width - 20;
        let close_y = self.y + 4;
        vesa::fill_rect(close_x, close_y, 16, 16, 0xFF0000);
        draw_char(close_x + 4, close_y + 4, b'X', 0xFFFFFF);

        // Draw content area
        vesa::fill_rect(self.x + 2, self.y + 18, self.width - 4, self.height - 20, content_color);
    }
}

fn draw_char(x: u16, y: u16, ch: u8, color: u32) {
    // Simple 5x7 font - just draw a block for each character
    // In a real OS, this would use a proper font
    for dy in 0..8 {
        for dx in 0..8 {
            // Draw a simple block character
            vesa::put_pixel(x + dx, y + dy, color);
        }
    }
}

pub struct WindowManager {
    windows: [Window; 8],
    count: usize,
    focused_id: Option<u8>,
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            windows: [
                Window::new(0, 0, 0, 0, 0, ""),
                Window::new(0, 0, 0, 0, 0, ""),
                Window::new(0, 0, 0, 0, 0, ""),
                Window::new(0, 0, 0, 0, 0, ""),
                Window::new(0, 0, 0, 0, 0, ""),
                Window::new(0, 0, 0, 0, 0, ""),
                Window::new(0, 0, 0, 0, 0, ""),
                Window::new(0, 0, 0, 0, 0, ""),
            ],
            count: 0,
            focused_id: None,
        }
    }

    pub fn add_window(&mut self, id: u8, x: u16, y: u16, width: u16, height: u16, title: &'static str) {
        if self.count < 8 {
            self.windows[self.count] = Window::new(id, x, y, width, height, title);
            self.count += 1;
        }
    }

    pub fn render(&self) {
        for i in 0..self.count {
            self.windows[i].render();
        }
    }
}
