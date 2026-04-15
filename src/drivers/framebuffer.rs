//! A simple driver for a linear framebuffer.

use alloc::vec::Vec;
use spin::Mutex;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[cfg(target_arch = "x86")]
#[allow(unused_imports)]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

/// Flag indicating if the background rendering task is active.
pub static RENDER_TASK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The latency of the last VRAM blit in TSC cycles.
pub static BLIT_LATENCY: AtomicUsize = AtomicUsize::new(0);
/// Heartbeat incremented by the background rendering task.
pub static GPU_HEARTBEAT: AtomicUsize = AtomicUsize::new(0);

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
    /// An intermediate buffer that holds the frame ready for VRAM blitting.
    pub ready_buffer: Option<Vec<u32>>,
    /// The third buffer used for active VRAM blitting to prevent blocking the GUI thread.
    pub blit_buffer: Option<Vec<u32>>,
    /// A dedicated scratch buffer for large pixel operations (like fading or row processing).
    /// Using this prevents stack overflows during complex GUI operations.
    pub scratch_buffer: Option<Vec<u32>>,
    /// Flag indicating if a new frame is ready in the ready_buffer.
    pub frame_ready: AtomicBool,
    /// Bounding box of the region that needs to be blitted.
    pub dirty_x1: usize,
    pub dirty_y1: usize,
    pub dirty_x2: usize,
    pub dirty_y2: usize,
    pub clip_rect: Option<(usize, usize, usize, usize)>,
}

impl Framebuffer {
    pub const fn new() -> Self {
        Framebuffer { 
            info: None, draw_buffer: None, ready_buffer: None, blit_buffer: None, scratch_buffer: None,
            frame_ready: AtomicBool::new(false),
            dirty_x1: 0, dirty_y1: 0, dirty_x2: 0, dirty_y2: 0,
            clip_rect: None 
        }
    }

    /// Initializes the framebuffer with the information from the bootloader.
    pub fn init(&mut self, info: (usize, usize, usize, usize, u8)) {
        // Robustness: Ensure basic parameters are valid
        if info.0 == 0 || info.1 == 0 || info.2 == 0 || info.4 == 0 {
            return;
        }

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
            self.ready_buffer = Some(Vec::with_capacity(buffer_size));
            if let Some(ref mut buffer) = self.ready_buffer {
                buffer.resize(buffer_size, 0);
            }
            self.blit_buffer = Some(Vec::with_capacity(buffer_size));
            if let Some(ref mut buffer) = self.blit_buffer {
                buffer.resize(buffer_size, 0);
            }
            self.scratch_buffer = Some(Vec::with_capacity(buffer_size));
            if let Some(ref mut buffer) = self.scratch_buffer {
                buffer.resize(buffer_size, 0);
            }

            // Initialize dirty rect to whole screen for first draw
            self.dirty_x1 = 0;
            self.dirty_y1 = 0;
            self.dirty_x2 = fb_info.width;
            self.dirty_y2 = fb_info.height;
            self.frame_ready.store(true, Ordering::Release);
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
        if let (Some(info), Some(buffer)) = (self.info.as_ref(), self.draw_buffer.as_mut()) {
            for j in 0..height {
                let py = y + j;
                if py >= info.height { break; }
                
                let src_row = &data[j * width..(j + 1) * width];
                let dest_offset = py * info.width + x;
                
                // Check if row is fully opaque (common for many UI assets)
                if src_row.iter().all(|&p| (p >> 24) == 0xFF) {
                    let copy_len = width.min(info.width.saturating_sub(x));
                    buffer[dest_offset..dest_offset + copy_len].copy_from_slice(&src_row[..copy_len]);
                } else {
                    // Per-pixel alpha for complex icons
                    for i in 0..width {
                        let px = x + i;
                        if px >= info.width { break; }
                        let color = src_row[i];
                        if (color >> 24) != 0 {
                            buffer[dest_offset + i] = color;
                        }
                    }
                }
            }
        }
    }

    /// Fades the contents of the draw_buffer using the scratch_buffer as a workspace.
    /// This avoids large stack allocations that cause overflows in kernel tasks.
    pub fn apply_fade(&mut self, step: u32, max: u32) {
        if max == 0 { return; }

        // If this is the start of a fade sequence, cache the current draw_buffer into scratch.
        // This allows us to always fade from the original "source of truth".
        if step == max {
            if let (Some(ref draw), Some(ref mut scratch)) = (&self.draw_buffer, &mut self.scratch_buffer) {
                scratch.copy_from_slice(draw);
            }
        }

        if let (Some(ref scratch), Some(ref mut draw)) = (&self.scratch_buffer, &mut self.draw_buffer) {
            // Use fixed-point math to avoid division in the hot loop
            let scale = (step << 8) / max; 
            for i in 0..scratch.len() {
                let pixel = scratch[i];
                let r = (((pixel >> 16) & 0xFF) * scale) >> 8;
                let g = (((pixel >> 8) & 0xFF) * scale) >> 8;
                let b = ((pixel & 0xFF) * scale) >> 8;
                draw[i] = (pixel & 0xFF000000) | (r << 16) | (g << 8) | b;
            }
        }
    }
    /// Marks the whole screen as dirty and ready for presentation.
    pub fn present(&mut self) {
        if let Some(info) = self.info {
            self.present_rect(0, 0, info.width, info.height);
        }
    }

    /// Copies a specific rectangle from the backbuffer to the deferred ready buffer (or VRAM).
    pub fn present_rect(&mut self, x: usize, y: usize, width: usize, height: usize) {
        let (safe_width, safe_height) = if let Some(info) = self.info {
            (width.min(info.width.saturating_sub(x)), height.min(info.height.saturating_sub(y)))
        } else { return };

        if safe_width == 0 || safe_height == 0 { return; }

        // Update dirty bounds
        self.dirty_x1 = self.dirty_x1.min(x);
        self.dirty_y1 = self.dirty_y1.min(y);
        self.dirty_x2 = self.dirty_x2.max(x + safe_width);
        self.dirty_y2 = self.dirty_y2.max(y + safe_height);

        if RENDER_TASK_ACTIVE.load(Ordering::Relaxed) {
            // Deferred: Copy from draw_buffer to ready_buffer
            if let (Some(ref info), Some(ref draw), Some(ref mut ready)) = (&self.info, &self.draw_buffer, &mut self.ready_buffer) {
                for i in 0..safe_height {
                    let cy = y + i;
                    let offset = cy * info.width + x;
                    unsafe {
                        ptr::copy_nonoverlapping(draw.as_ptr().add(offset), ready.as_mut_ptr().add(offset), safe_width);
                    }
                }
                self.frame_ready.store(true, Ordering::Release);
            }
        } else {
            // Synchronous (Early boot/Panic): Blit directly to VRAM from draw_buffer
            self.blit_rect_to_vram(x, y, safe_width, safe_height, false);
        }
    }

    /// Flushes the ready buffer to the actual VRAM. Called by the background render task.
    /// Returns true if a blit actually occurred.
    pub fn blit_to_vram(&mut self) -> bool {
        if !self.frame_ready.swap(false, Ordering::Acquire) { return false; }
        
        let start_tsc = crate::kernel::cpu::read_tsc();
        GPU_HEARTBEAT.fetch_add(1, Ordering::Relaxed);

        let x = self.dirty_x1;
        let y = self.dirty_y1;
        let w = self.dirty_x2.saturating_sub(x);
        let h = self.dirty_y2.saturating_sub(y);

        if w > 0 && h > 0 {
            // Triple Buffering: Swap Ready and Blit buffers instantly
            // This allows the GUI to start the next 'present' immediately
            let mut temp = self.ready_buffer.take();
            core::mem::swap(&mut temp, &mut self.blit_buffer);
            self.ready_buffer = temp;

            // Perform the slow VRAM write using the stable blit_buffer
            if let (Some(info), Some(buffer)) = (self.info.as_ref(), self.blit_buffer.as_ref().map(|v| v.as_slice())) {
                self.blit_rect_to_vram_internal(info, buffer, x, y, w, h);
            }
        }

        // Reset dirty rect for next frame
        if let Some(info) = self.info {
            self.dirty_x1 = info.width;
            self.dirty_y1 = info.height;
            self.dirty_x2 = 0;
            self.dirty_y2 = 0;
        }

        let end_tsc = crate::kernel::cpu::read_tsc();
        BLIT_LATENCY.store((end_tsc - start_tsc) as usize, Ordering::Relaxed);

        true
    }

    /// Internal helper to copy a rectangle to VRAM from a specific source buffer.
    fn blit_rect_to_vram(&self, x: usize, y: usize, width: usize, height: usize, from_ready: bool) {
        let source = if from_ready { 
            self.ready_buffer.as_ref().map(|v| v.as_slice()) 
        } else { 
            self.draw_buffer.as_ref().map(|v| v.as_slice()) 
        };
        if let (Some(info), Some(buffer)) = (self.info.as_ref(), source) {
            self.blit_rect_to_vram_internal(info, buffer, x, y, width, height);
        }
    }

    fn blit_rect_to_vram_internal(&self, info: &FramebufferInfo, buffer: &[u32], x: usize, y: usize, width: usize, height: usize) {
        let vram_ptr = info.address as *mut u8;

        if info.bpp == 32 {
            let src_ptr = buffer.as_ptr() as *const u8;
            let row_len_bytes = width * 4;
            for i in 0..height {
                let cy = y + i;
                let src_offset = (cy * info.width + x) * 4;
                let dst_offset = cy * info.pitch + x * 4;
                unsafe {
                    ptr::copy_nonoverlapping(src_ptr.add(src_offset), vram_ptr.add(dst_offset), row_len_bytes);
                }
            }
        } else if info.bpp == 24 {
            for i in 0..height {
                let cy = y + i;
                let src_row_base = cy * info.width + x;
                let dst_row_base = cy * info.pitch + x * 3;
                for j in 0..width {
                    let pixel = buffer[src_row_base + j];
                    unsafe {
                        let p_ptr = vram_ptr.add(dst_row_base + j * 3);
                        // Standard 24-bit is BGR
                        *p_ptr = (pixel & 0xFF) as u8;           // Blue
                        *p_ptr.add(1) = ((pixel >> 8) & 0xFF) as u8;  // Green
                        *p_ptr.add(2) = ((pixel >> 16) & 0xFF) as u8; // Red
                    }
                }
            }
        } else if info.bpp == 16 {
            // 16-bit support: Typically RGB565 (5 bits Red, 6 bits Green, 5 bits Blue)
            for i in 0..height {
                let cy = y + i;
                let src_row_base = cy * info.width + x;
                let dst_row_base = cy * info.pitch + x * 2;
                for j in 0..width {
                    let pixel = buffer[src_row_base + j];
                    let r = ((pixel >> 16) & 0xFF) as u16;
                    let g = ((pixel >> 8) & 0xFF) as u16;
                    let b = (pixel & 0xFF) as u16;
                    // Format: RRRRRGGGGGGBBBBB
                    let rgb565 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
                    unsafe {
                        let p_ptr = vram_ptr.add(dst_row_base + j * 2) as *mut u16;
                        ptr::write_volatile(p_ptr, rgb565);
                    }
                }
            }
        }
    }

    /// Draws a line between two points using Bresenham's algorithm.
    pub fn draw_line(&mut self, x1: isize, y1: isize, x2: isize, y2: isize, color: u32) {
        let mut x = x1;
        let mut y = y1;
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as usize, y as usize, color);
            }
            if x == x2 && y == y2 { break; }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draws a circle using the midpoint circle algorithm.
    pub fn draw_circle(&mut self, xc: isize, yc: isize, radius: isize, color: u32) {
        let mut x = 0;
        let mut y = radius;
        let mut d = 3 - 2 * radius;

        self.draw_circle_points(xc, yc, x, y, color);
        while y >= x {
            x += 1;
            if d > 0 {
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
            self.draw_circle_points(xc, yc, x, y, color);
        }
    }

    fn draw_circle_points(&mut self, xc: isize, yc: isize, x: isize, y: isize, color: u32) {
        let pts = [(xc+x, yc+y), (xc-x, yc+y), (xc+x, yc-y), (xc-x, yc-y), (xc+y, yc+x), (xc-y, yc+x), (xc+y, yc-x), (xc-y, yc-x)];
        for (px, py) in pts {
            if px >= 0 && py >= 0 {
                self.set_pixel(px as usize, py as usize, color);
            }
        }
    }
}

/// Global instance of the framebuffer driver.

/// Global instance of the framebuffer driver.
pub static FRAMEBUFFER: Mutex<Framebuffer> = Mutex::new(Framebuffer::new());