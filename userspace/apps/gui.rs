#[derive(Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Clone, Copy)]
pub struct PointerEvent {
    pub x: u32,
    pub y: u32,
    pub left_down: bool,
}

#[derive(Clone, Copy)]
pub enum Event {
    Key(u8),
    Pointer(PointerEvent),
}

pub trait Canvas {
    fn fill_rect(&mut self, rect: Rect, color: u32);
    fn draw_rect(&mut self, rect: Rect, color: u32);
    fn draw_text(&mut self, x: u32, y: u32, text: &str, color: u32, scale: u32);
}

pub trait Application {
    fn title(&self) -> &'static str;
    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent);
    fn handle_event(&mut self, event: Event, area: Rect) -> bool;
}

pub struct Button {
    label: &'static str,
    left_down: bool,
    armed: bool,
}

impl Button {
    pub const fn new(label: &'static str) -> Self {
        Self {
            label,
            left_down: false,
            armed: false,
        }
    }

    pub fn handle_pointer(&mut self, rect: Rect, pointer: PointerEvent) -> bool {
        let activated = if pointer.left_down && !self.left_down {
            self.armed = rect.contains(pointer.x, pointer.y);
            false
        } else if !pointer.left_down && self.left_down {
            let activated = self.armed && rect.contains(pointer.x, pointer.y);
            self.armed = false;
            activated
        } else {
            false
        };
        self.left_down = pointer.left_down;
        activated
    }

    pub fn render(&self, canvas: &mut dyn Canvas, rect: Rect, pointer: PointerEvent) {
        self.draw_chrome(canvas, rect, pointer);
        let text_width = self.label.len() as u32 * 6;
        let text_x = rect.x + rect.width.saturating_sub(text_width) / 2;
        let text_y = rect.y + rect.height.saturating_sub(8) / 2;
        canvas.draw_text(text_x, text_y, self.label, 0xF1F5F9, 1);
    }

    pub fn draw_chrome(&self, canvas: &mut dyn Canvas, rect: Rect, pointer: PointerEvent) {
        let hovered = rect.contains(pointer.x, pointer.y);
        let background = if self.armed && pointer.left_down && hovered {
            0x285A72
        } else if hovered {
            0x26374F
        } else {
            0x1B2942
        };
        canvas.fill_rect(rect, background);
        canvas.draw_rect(rect, if hovered { 0x66D9EF } else { 0x516786 });
    }
}
