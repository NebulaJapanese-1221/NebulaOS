use crate::drivers::framebuffer;
use crate::userspace::gui::{self, Window, button::Button, rect::Rect};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::vec;

#[derive(Clone)]
pub struct Paint {
    buffer: Vec<u32>,
    buffer_width: usize,
    buffer_height: usize,
    color: u32,
    last_pos: Option<(isize, isize)>,
}

impl Paint {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            buffer_width: 0,
            buffer_height: 0,
            color: 0x00_FFFFFF, // Default white
            last_pos: None,
        }
    }

    fn resize_buffer(&mut self, width: usize, height: usize) {
        if self.buffer_width == width && self.buffer_height == height {
            return;
        }
        let mut new_buffer = vec![0x00_00_00_00; width * height]; // Black background
        
        // Copy old buffer if possible
        for y in 0..self.buffer_height.min(height) {
            for x in 0..self.buffer_width.min(width) {
                new_buffer[y * width + x] = self.buffer[y * self.buffer_width + x];
            }
        }
        
        self.buffer = new_buffer;
        self.buffer_width = width;
        self.buffer_height = height;
    }

    fn draw_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && x < self.buffer_width as isize && y >= 0 && y < self.buffer_height as isize {
                 self.buffer[y as usize * self.buffer_width + x as usize] = color;
                 // Simple brush thickness (2x2)
                 if x + 1 < self.buffer_width as isize { self.buffer[y as usize * self.buffer_width + (x + 1) as usize] = color; }
                 if y + 1 < self.buffer_height as isize { self.buffer[(y + 1) as usize * self.buffer_width + x as usize] = color; }
            }

            if x == x1 && y == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

impl App for Paint {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, _dirty_rect: Rect) {
        // Toolbar area
        let toolbar_height = 40;
        gui::draw_rect(fb, win.x, win.y + 20, win.width, toolbar_height, 0x00_30_30_30, None);

        // Canvas area
        let canvas_y = win.y + 20 + toolbar_height as isize;
        let canvas_height = win.height.saturating_sub(20 + toolbar_height);
        
        gui::draw_rect(fb, win.x, canvas_y, win.width, canvas_height, 0x00_00_00_00, None); // Black bg

        if self.buffer_width > 0 {
             // Blit buffer to screen
             let w = self.buffer_width.min(win.width);
             let h = self.buffer_height.min(canvas_height);
             
             for y in 0..h {
                 for x in 0..w {
                     let color = self.buffer[y * self.buffer_width + x];
                     if color != 0 { // Simple transparency
                         if let Some(info) = &fb.info {
                             let screen_x = win.x + x as isize;
                             let screen_y = canvas_y + y as isize;
                             if screen_x >= 0 && screen_x < info.width as isize && screen_y >= 0 && screen_y < info.height as isize {
                                  fb.set_pixel(screen_x as usize, screen_y as usize, color);
                             }
                         }
                     }
                 }
             }
        }

        // Draw Toolbar Buttons (Palette)
        let colors = [0x00_FFFFFF, 0x00_FF0000, 0x00_00FF00, 0x00_0000FF, 0x00_FFFF00, 0x00_00FFFF, 0x00_FF00FF, 0x00_000000];
        for (i, &c) in colors.iter().enumerate() {
            gui::draw_rect(fb, win.x + 5 + (i as isize * 25), win.y + 25, 20, 20, c, None);
            if self.color == c {
                // Highlight selected
                 gui::draw_rect(fb, win.x + 5 + (i as isize * 25), win.y + 45, 20, 2, 0x00_FFFFFF, None);
            }
        }
        
        let clear_btn = Button::new(win.x + 5 + (colors.len() as isize * 25) + 10, win.y + 25, 50, 20, "Clear");
        clear_btn.draw(fb, 0, 0, None);
    }

    fn handle_event(&mut self, event: &AppEvent) {
        match event {
             AppEvent::MouseMove { x, y, width, height } | AppEvent::MouseClick { x, y, width, height } => {
                 let toolbar_height = 40;
                 let canvas_h = height.saturating_sub(20 + toolbar_height);
                 
                 if self.buffer_width != *width || self.buffer_height != canvas_h {
                     self.resize_buffer(*width, canvas_h);
                 }

                 if *y < 20 + toolbar_height as isize {
                     // Toolbar click
                     if matches!(event, AppEvent::MouseClick { .. }) {
                         let colors = [0x00_FFFFFF, 0x00_FF0000, 0x00_00FF00, 0x00_0000FF, 0x00_FFFF00, 0x00_00FFFF, 0x00_FF00FF, 0x00_000000];
                         for (i, &c) in colors.iter().enumerate() {
                             let bx = 5 + (i as isize * 25);
                             if *x >= bx && *x < bx + 20 && *y >= 25 && *y < 45 { self.color = c; }
                         }
                         let clear_x = 5 + (colors.len() as isize * 25) + 10;
                         if *x >= clear_x && *x < clear_x + 50 && *y >= 25 && *y < 45 { self.buffer.fill(0); }
                     }
                 } else {
                     // Canvas Drawing
                     let cx = *x;
                     let cy = *y - (20 + toolbar_height as isize);
                     let start = self.last_pos.unwrap_or((cx, cy));
                     self.draw_line(start.0, start.1, cx, cy, self.color);
                     self.last_pos = if matches!(event, AppEvent::MouseMove { .. }) { Some((cx, cy)) } else { None };
                 }
             }
             _ => {}
        }
    }

    fn box_clone(&self) -> Box<dyn App> { Box::new(self.clone()) }
}