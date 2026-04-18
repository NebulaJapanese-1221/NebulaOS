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

    /// Returns a new rectangle expanded by the given amount on all sides.
    pub fn inflate(&self, amount: isize) -> Rect {
        Rect {
            x: self.x - amount,
            y: self.y - amount,
            width: (self.width as isize + amount * 2).max(0) as usize,
            height: (self.height as isize + amount * 2).max(0) as usize,
        }
    }
}