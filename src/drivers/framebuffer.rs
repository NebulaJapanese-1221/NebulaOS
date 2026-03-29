//! A simple driver for a linear framebuffer.

use alloc::vec::Vec;
use spin::Mutex;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Flag indicating if the background rendering task is active.
pub static RENDER_TASK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The current measured frames per second.
pub static FPS: AtomicUsize = AtomicUsize::new(0);
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
            info: None, draw_buffer: None, ready_buffer: None,
            frame_ready: AtomicBool::new(false),
            dirty_x1: 0, dirty_y1: 0, dirty_x2: 0, dirty_y2: 0,
            clip_rect: None 
        }
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
            self.ready_buffer = Some(Vec::with_capacity(buffer_size));
            if let Some(ref mut buffer) = self.ready_buffer {
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

        let x = self.dirty_x1;
        let y = self.dirty_y1;
        let w = self.dirty_x2.saturating_sub(x);
        let h = self.dirty_y2.saturating_sub(y);

        if w > 0 && h > 0 {
            self.blit_rect_to_vram(x, y, w, h, true);
        }

        // Reset dirty rect for next frame
        if let Some(info) = self.info {
            self.dirty_x1 = info.width;
            self.dirty_y1 = info.height;
            self.dirty_x2 = 0;
            self.dirty_y2 = 0;
        }

        true
    }

    /// Internal helper to copy a rectangle to VRAM.
    fn blit_rect_to_vram(&self, x: usize, y: usize, width: usize, height: usize, from_ready: bool) {
        let source = if from_ready { &self.ready_buffer } else { &self.draw_buffer };
        if let (Some(ref info), Some(ref buffer)) = (&self.info, source) {
            if info.bpp != 32 { return; }
            let vram_ptr = info.address as *mut u8;
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
        }
    }
}

/// Global instance of the framebuffer driver.

/// Global instance of the framebuffer driver.
pub static FRAMEBUFFER: Mutex<Framebuffer> = Mutex::new(Framebuffer::new());