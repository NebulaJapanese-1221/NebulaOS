use crate::drivers::framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn contains(&self, px: isize, py: isize) -> bool {
        px >= self.x &&
        px < self.x + self.width as isize &&
        py >= self.y &&
        py < self.y + self.height as isize
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width as isize &&
        self.x + self.width as isize > other.x &&
        self.y < other.y + other.height as isize &&
        self.y + self.height as isize > other.y
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width as isize).max(other.x + other.width as isize);
        let bottom = (self.y + self.height as isize).max(other.y + other.height as isize);
        Rect {
            x,
            y,
            width: (right - x) as usize,
            height: (bottom - y) as usize,
        }
    }
    
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width as isize).min(other.x + other.width as isize);
        let bottom = (self.y + self.height as isize).min(other.y + other.height as isize);
        if right > x && bottom > y {
            Some(Rect { x, y, width: (right - x) as usize, height: (bottom - y) as usize })
        } else {
            None
        }
    }
}

pub fn draw_rect(fb: &mut framebuffer::Framebuffer, x: isize, y: isize, width: usize, height: usize, color: u32, clip: Option<Rect>) {    
    if let Some(fb_info) = fb.info.as_ref() {
        let screen_rect = Rect { x: 0, y: 0, width: fb_info.width, height: fb_info.height };
        let mut target_rect = Rect { x, y, width, height };

        if let Some(c) = clip {
            if let Some(clipped) = target_rect.intersection(&c) { target_rect = clipped; }
            else { return; }
        }

        if let Some(clipped) = target_rect.intersection(&screen_rect) {
            let end_y = clipped.y + clipped.height as isize;
            let end_x = clipped.x + clipped.width as isize;
            for py in clipped.y .. end_y {
                for px in clipped.x .. end_x { fb.set_pixel(px as usize, py as usize, color); }
            }
        }
    }
}