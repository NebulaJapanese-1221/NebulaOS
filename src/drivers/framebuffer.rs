//! A simple driver for a linear framebuffer.

use alloc::vec::Vec;
use spin::Mutex;
use core::ptr;

/// Holds the information about the framebuffer provided by the bootloader.
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {

    pub address: usize,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bpp: usize,
}

pub struct Framebuffer {
    pub info: Option<FramebufferInfo>,
    /// An off-screen buffer where all drawing operations take place.
    /// This is then copied to the visible VRAM by `present()`.
    pub draw_buffer: Option<Vec<u32>>,
    pub clip_rect: Option<(usize, usize, usize, usize)>,
}

impl Framebuffer {
    pub const fn new() -> Self {
        Framebuffer { info: None, draw_buffer: None, clip_rect: None }
    }

    /// Initializes the framebuffer with the information from the bootloader.
    pub fn init(&mut self, info: (usize, usize, usize, usize, u8)) {
        self.info = Some(FramebufferInfo {
            address: info.0,
           width: info.1,
            height: info.2,
            pitch: info.3,
            bpp: info.4 as usize,
        });

        if let Some(ref fb_info) = self.info {
            let buffer_size = fb_info.width * fb_info.height;
            self.draw_buffer = Some(Vec::with_capacity(buffer_size));
            if let Some(ref mut buffer) = self.draw_buffer {
                buffer.resize(buffer_size, 0);
            }
        }
    }


    /// Sets a single pixel in the off-screen draw buffer.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if let (Some(ref info), Some(ref mut buffer)) = (&self.info, &mut self.draw_buffer) {
            if let Some((cx, cy, cw, ch)) = self.clip_rect {
                if x < cx || x >= cx + cw || y < cy || y >= cy + ch {
                    return;
                }
            }
            if x < info.width && y < info.height {
                buffer[y * info.width + x] = color;
            }
        }
    }

    pub fn set_clip(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.clip_rect = Some((x, y, width, height));
    }

    pub fn clear_clip(&mut self) {
        self.clip_rect = None;
    }

    /// Clears the entire screen to a single color.
    pub fn clear(&mut self, color: u32) {
        if let Some(ref mut buffer) = self.draw_buffer {
            buffer.fill(color);
        }
    }

    /// Draws a bitmap image to the buffer.
    /// `data` is a slice of u32 pixels (ARGB/RGBA).
    /// Assumes 0x00 in the alpha channel (highest byte) is fully transparent.
    pub fn draw_bitmap(&mut self, x: usize, y: usize, width: usize, height: usize, data: &[u32]) {
        if let (Some(ref info), Some(ref mut buffer)) = (&self.info, &mut self.draw_buffer) {
            for j in 0..height {
                for i in 0..width {
                    let color = data[j * width + i];
                    // Skip transparent pixels (assuming alpha is high bits, 0 = transparent)
                    if (color >> 24) == 0 { continue; }
                    
                    let px = x + i;
                    let py = y + j;
                    
                    if px < info.width && py < info.height {
                        buffer[py * info.width + px] = color;
                    }
                }
            }
        }
    }

    /// Copies the off-screen draw buffer to the visible framebuffer memory.
    /// This makes the changes visible on screen.
    pub fn present(&self) {
        if let (Some(ref info), Some(ref buffer)) = (&self.info, &self.draw_buffer) {
            // We assume 32-bit color depth for both buffers
            if info.bpp != 32 {
                return;
            }

            let vram_ptr = info.address as *mut u8;
            let draw_buffer_ptr = buffer.as_ptr() as *const u8;
            let row_len_bytes = info.width * 4;

            if info.pitch == row_len_bytes {
                // If the framebuffer is linear, we can copy the whole thing at once.
                // This is much faster and reduces tearing.
                let total_bytes = info.width * info.height * 4;
                unsafe {
                    ptr::copy_nonoverlapping(draw_buffer_ptr, vram_ptr, total_bytes);
                }
            } else {
                // If the pitch is different from the width, we must copy row by row.
                for y in 0..info.height {
                    let src_offset = y * row_len_bytes;
                    let dst_offset = y * info.pitch;
                    unsafe {
                        ptr::copy_nonoverlapping(draw_buffer_ptr.add(src_offset), vram_ptr.add(dst_offset), row_len_bytes);
                    }
                }
            }
        }
    }

    /// Copies a specific rectangle from the backbuffer to the framebuffer memory.
    pub fn present_rect(&self, x: usize, y: usize, width: usize, height: usize) {
        if let (Some(ref info), Some(ref buffer)) = (&self.info, &self.draw_buffer) {
            if info.bpp != 32 { return; }

            let vram_ptr = info.address as *mut u8;
            let draw_buffer_ptr = buffer.as_ptr() as *const u8;

            // Prevent panic if coordinates are out of bounds
            if x >= info.width || y >= info.height { return; }
            
            // Clamp dimensions to screen bounds
            let safe_width = width.min(info.width - x);
            let safe_height = height.min(info.height - y);
            let row_len_bytes = safe_width * 4;

            for i in 0..safe_height {
                let cy = y + i;
                let src_offset = (cy * info.width + x) * 4;
                let dst_offset = cy * info.pitch + x * 4;
                unsafe {
                    ptr::copy_nonoverlapping(draw_buffer_ptr.add(src_offset), vram_ptr.add(dst_offset), row_len_bytes);
                }
            }
        }
    }
}

/// Global instance of the framebuffer driver.

/// Global instance of the framebuffer driver.
pub static FRAMEBUFFER: Mutex<Framebuffer> = Mutex::new(Framebuffer::new());