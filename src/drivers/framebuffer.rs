use crate::sync::Spinlock;
use core::ptr::{null_mut, copy_nonoverlapping};

const MAX_DIRTY_RECTS: usize = 32;

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pitch: usize, // Bytes per scanline
    pub lfb: *mut u32,
    pub backbuffer: *mut u32,
    pub dirty_rects: [Rect; MAX_DIRTY_RECTS],
    pub dirty_count: usize,
}

unsafe impl Send for Framebuffer {}

pub static FRAMEBUFFER: Spinlock<Framebuffer> = Spinlock::new(Framebuffer::new());

impl Framebuffer {
    pub const fn new() -> Self {
        Self {
            width: 1024,
            height: 768,
            pitch: 1024 * 4,
            lfb: null_mut(),
            backbuffer: null_mut(),
            dirty_rects: [Rect { x: 0, y: 0, width: 0, height: 0 }; MAX_DIRTY_RECTS],
            dirty_count: 0,
        }
    }

    pub fn init(&mut self, addr: *mut u32, width: usize, height: usize, pitch: usize) {
        self.lfb = addr;
        self.width = width;
        self.height = height;
        self.pitch = pitch;
    }

    pub fn mark_dirty(&mut self, x: u32, y: u32, width: u32, height: u32) {
        if self.dirty_count < MAX_DIRTY_RECTS {
            self.dirty_rects[self.dirty_count] = Rect { x, y, width, height };
            self.dirty_count += 1;
        } else {
            // Array full: Merge new damage into the last rectangle to maintain bounds
            let last = &mut self.dirty_rects[MAX_DIRTY_RECTS - 1];
            let x1 = last.x.min(x);
            let y1 = last.y.min(y);
            let x2 = (last.x + last.width).max(x + width);
            let y2 = (last.y + last.height).max(y + height);

            last.x = x1;
            last.y = y1;
            last.width = x2 - x1;
            last.height = y2 - y1;
        }
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        self.mark_dirty(x as u32, y as u32, width as u32, height as u32);
        for i in 0..height {
            for j in 0..width {
                self.draw_pixel(x + j, y + i, color);
            }
        }
    }

    pub fn draw_bitmap(&mut self, x: usize, y: usize, width: usize, height: usize, bitmap: &[u16], color: u32) {
        self.mark_dirty(x as u32, y as u32, width as u32, height as u32);
        for i in 0..height {
            for j in 0..width {
                if (bitmap[i] & (1 << (width - 1 - j))) != 0 {
                    self.draw_pixel(x + j, y + i, color);
                }
            }
        }
    }

    pub fn present(&mut self) {
        // Ensure both buffers are valid before attempting synchronization
        if self.backbuffer.is_null() || self.lfb.is_null() { return; }

        for i in 0..self.dirty_count {
            let rect = self.dirty_rects[i];
            let pitch_pixels = self.pitch / 4;
            for row in rect.y..(rect.y + rect.height) {
                if row >= self.height as u32 { continue; }
                
                let src_offset = (row as usize * self.width) + rect.x as usize;
                let dst_offset = (row as usize * pitch_pixels) + rect.x as usize;
                let copy_len = rect.width as usize;

                unsafe {
                    copy_nonoverlapping(
                        self.backbuffer.add(src_offset),
                        self.lfb.add(dst_offset),
                        copy_len
                    );
                }
            }
        }
        self.dirty_count = 0;
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            // Prioritize backbuffer if it exists and is valid
            if !self.backbuffer.is_null() && (self.backbuffer as usize % 4 == 0) {
                unsafe { *self.backbuffer.add(y * self.width + x) = color; }
            } else if !self.lfb.is_null() && (self.lfb as usize % 4 == 0) {
                // Direct to LFB (early boot) using pitch
                unsafe { *self.lfb.add(y * (self.pitch / 4) + x) = color; }
            }
        }
    }
}