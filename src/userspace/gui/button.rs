use super::rect::Rect;
use crate::drivers::framebuffer;
use super::font;
use alloc::string::String;

#[derive(Clone)]
pub struct Button {
    pub rect: Rect,
    pub text: String,
    pub bg_color: u32,
    pub text_color: u32,
}

impl Button {
    pub fn new<S: Into<String>>(x: isize, y: isize, width: usize, height: usize, text: S) -> Self {
        Self {
            rect: Rect { x, y, width, height },
            text: text.into(),
            bg_color: 0x00_C0_C0_C0,
            text_color: 0x00_00_00_00,
        }
    }

    pub fn draw(&self, fb: &mut framebuffer::Framebuffer, mouse_x: isize, mouse_y: isize, clip: Option<Rect>) {
        let is_hovered = self.contains(mouse_x, mouse_y);
        
        // Enforce clipping to the button's own rectangle to prevent spillover
        let draw_clip = if let Some(c) = clip {
            if let Some(intersection) = c.intersection(&self.rect) {
                intersection
            } else {
                return; // Button is outside the dirty rect
            }
        } else {
            self.rect
        };

        let high_contrast = super::HIGH_CONTRAST.load(core::sync::atomic::Ordering::Relaxed);

        let draw_color = if is_hovered {
            if high_contrast {
                0x00_FF_FF_FF // Invert (White) on hover
            } else {
                // Brighten the color for hover effect by adding 0x20 to each component
                let r = ((self.bg_color >> 16) & 0xFF) as u8;
                let g = ((self.bg_color >> 8) & 0xFF) as u8;
                let b = (self.bg_color & 0xFF) as u8;

                let r_h = r.saturating_add(0x20);
                let g_h = g.saturating_add(0x20);
                let b_h = b.saturating_add(0x20);

                ((r_h as u32) << 16) | ((g_h as u32) << 8) | (b_h as u32)
            }
        } else {
            if high_contrast { 0x00_00_00_00 } else { self.bg_color }
        };
        super::draw_rect(fb, self.rect.x, self.rect.y, self.rect.width, self.rect.height, draw_color, Some(draw_clip));

        // Add 3D bevel effect
        let (bright, dark) = if high_contrast {
            (0x00_FF_FF_FF, 0x00_FF_FF_FF)
        } else {
            (0x00_FF_FF_FF, 0x00_40_40_40)
        };
        
        super::draw_rect(fb, self.rect.x, self.rect.y, self.rect.width, 1, bright, Some(draw_clip)); // Top
        super::draw_rect(fb, self.rect.x, self.rect.y, 1, self.rect.height, bright, Some(draw_clip)); // Left
        super::draw_rect(fb, self.rect.x + self.rect.width as isize - 1, self.rect.y, 1, self.rect.height, dark, Some(draw_clip)); // Right
        super::draw_rect(fb, self.rect.x, self.rect.y + self.rect.height as isize - 1, self.rect.width, 1, dark, Some(draw_clip)); // Bottom

        let font_height = if super::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
        let text_x = self.rect.x + (self.rect.width as isize - font::string_width(self.text.as_str()) as isize) / 2;
        let text_y = self.rect.y + (self.rect.height as isize - font_height) / 2;
        
        let final_text_color = if high_contrast {
            if is_hovered { 0x00_00_00_00 } else { 0x00_FF_FF_FF }
        } else {
            self.text_color
        };
        
        font::draw_string(fb, text_x, text_y, self.text.as_str(), final_text_color, Some(draw_clip));
    }

    pub fn contains(&self, x: isize, y: isize) -> bool {
        self.rect.contains(x, y)
    }
}