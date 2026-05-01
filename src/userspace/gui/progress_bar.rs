use super::rect::Rect;
use super::font;
use crate::drivers::framebuffer;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProgressBarOrientation {
    Horizontal,
    Vertical,
}

pub struct ProgressBar {
    pub rect: Rect,
    pub progress: usize, // 0 to 100
    pub color: u32,
    pub bg_color: u32,
    pub orientation: ProgressBarOrientation,
    pub text: bool, // If true, displays percentage automatically
    pub text_color: u32,
}

impl ProgressBar {
    pub fn new(x: isize, y: isize, width: usize, height: usize, progress: usize, color: u32) -> Self {
        Self {
            rect: Rect { x, y, width, height },
            progress: progress.min(100),
            color,
            bg_color: 0x00_2D_2D_30, // Default Nebula Dark grey
            orientation: ProgressBarOrientation::Vertical,
            text: false,
            text_color: 0x00_FFFFFF,
        }
    }

    pub fn draw(&self, fb: &mut framebuffer::Framebuffer, clip: Option<Rect>) {
        // Enforce clipping to the bar's own rectangle
        let draw_clip = if let Some(c) = clip {
            match c.intersection(&self.rect) {
                Some(intersection) => intersection,
                None => return,
            }
        } else {
            self.rect
        };

        // Background
        super::draw_rect(fb, self.rect.x, self.rect.y, self.rect.width, self.rect.height, self.bg_color, Some(draw_clip));

        let progress = self.progress.min(100);
        match self.orientation {
            ProgressBarOrientation::Vertical => {
                let fill_h = (progress * self.rect.height) / 100;
                if fill_h > 0 {
                    super::draw_rect(fb, self.rect.x + 2, self.rect.y + (self.rect.height - fill_h) as isize, self.rect.width - 4, fill_h, self.color, Some(draw_clip));
                }
            }
            ProgressBarOrientation::Horizontal => {
                let fill_w = (progress * self.rect.width) / 100;
                if fill_w > 0 {
                    super::draw_rect(fb, self.rect.x + 2, self.rect.y + 2, fill_w.saturating_sub(4), self.rect.height - 4, self.color, Some(draw_clip));
                }
            }
        }

        // Automatically draw percentage text if enabled
        if self.text {
            let font_height = if super::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
            let text_val = alloc::format!("{}%", progress);
            let tw = font::string_width(text_val.as_str());
            
            let tx = self.rect.x + (self.rect.width as isize - tw as isize) / 2;
            let ty = self.rect.y + (self.rect.height as isize - font_height as isize) / 2;
            
            font::draw_string(fb, tx, ty, text_val.as_str(), self.text_color, Some(draw_clip));
        }
    }
}