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

pub struct TextField {
    pub rect: Rect,
    pub focused: bool,
}

impl TextField {
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            focused: false,
        }
    }

    pub fn render(&self, canvas: &mut dyn Canvas, text: &str) {
        canvas.fill_rect(self.rect, 0xE2E8F0);
        canvas.draw_rect(self.rect, if self.focused { 0x1596B5 } else { 0x516786 });
        canvas.draw_text(self.rect.x + 7, self.rect.y + 8, text, 0x111827, 1);
        if self.focused {
            let caret_x = self.rect.x + 7 + text.len() as u32 * 6;
            canvas.fill_rect(
                Rect {
                    x: caret_x,
                    y: self.rect.y + 6,
                    width: 1,
                    height: self.rect.height.saturating_sub(12),
                },
                0x111827,
            );
        }
    }

    pub fn handle_pointer(&mut self, pointer: PointerEvent) -> bool {
        if pointer.left_down && !self.rect.contains(pointer.x, pointer.y) && self.focused {
            self.focused = false;
            true
        } else if pointer.left_down && self.rect.contains(pointer.x, pointer.y) && !self.focused {
            self.focused = true;
            true
        } else {
            false
        }
    }
}

pub struct Checkbox {
    pub checked: bool,
    left_down: bool,
    armed: bool,
}

impl Checkbox {
    pub const fn new(checked: bool) -> Self {
        Self {
            checked,
            left_down: false,
            armed: false,
        }
    }

    pub fn render(&self, canvas: &mut dyn Canvas, rect: Rect, label: &str) {
        canvas.fill_rect(rect, 0xE2E8F0);
        canvas.draw_rect(rect, 0x516786);
        if self.checked {
            canvas.fill_rect(
                Rect {
                    x: rect.x + 4,
                    y: rect.y + 4,
                    width: rect.width.saturating_sub(8),
                    height: rect.height.saturating_sub(8),
                },
                0x1596B5,
            );
        }
        canvas.draw_text(rect.x + rect.width + 8, rect.y + 4, label, 0xF1F5F9, 1);
    }

    pub fn handle_pointer(&mut self, rect: Rect, pointer: PointerEvent) -> bool {
        let toggled = pointer.left_down == false
            && self.left_down
            && self.armed
            && rect.contains(pointer.x, pointer.y);
        if pointer.left_down && !self.left_down {
            self.armed = rect.contains(pointer.x, pointer.y);
        } else if !pointer.left_down {
            self.armed = false;
        }
        self.left_down = pointer.left_down;
        if toggled {
            self.checked = !self.checked;
        }
        toggled
    }
}

pub struct Menu {
    pub selected: usize,
    left_down: bool,
    armed: Option<usize>,
}

impl Menu {
    pub const fn new(selected: usize) -> Self {
        Self {
            selected,
            left_down: false,
            armed: None,
        }
    }

    pub fn render(&self, canvas: &mut dyn Canvas, rect: Rect, items: &[&str]) {
        for (index, item) in items.iter().enumerate() {
            let item_rect = Rect {
                x: rect.x,
                y: rect.y + index as u32 * 28,
                width: rect.width,
                height: 24,
            };
            canvas.fill_rect(
                item_rect,
                if self.selected == index {
                    0x285A72
                } else {
                    0x1B2942
                },
            );
            canvas.draw_text(item_rect.x + 8, item_rect.y + 8, item, 0xF1F5F9, 1);
        }
    }

    pub fn handle_pointer(&mut self, rect: Rect, items: &[&str], pointer: PointerEvent) -> bool {
        let row = if rect.contains(pointer.x, pointer.y) {
            Some(((pointer.y - rect.y) / 28) as usize).filter(|index| *index < items.len())
        } else {
            None
        };
        let selected =
            !pointer.left_down && self.left_down && self.armed.is_some() && self.armed == row;
        if pointer.left_down && !self.left_down {
            self.armed = row;
        } else if !pointer.left_down {
            self.armed = None;
        }
        self.left_down = pointer.left_down;
        if selected {
            if let Some(index) = row {
                self.selected = index;
                return true;
            }
        }
        false
    }
}

pub struct ScrollArea {
    pub offset: u32,
}

pub struct FocusGroup {
    selected: usize,
}

impl FocusGroup {
    pub const fn new(selected: usize) -> Self {
        Self { selected }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn next(&mut self, count: usize) {
        if count > 0 {
            self.selected = (self.selected + 1) % count;
        }
    }
}

impl ScrollArea {
    pub const fn new() -> Self {
        Self { offset: 0 }
    }

    pub fn render(&self, canvas: &mut dyn Canvas, rect: Rect, content_height: u32) {
        canvas.fill_rect(rect, 0xE2E8F0);
        let track_height = rect.height;
        let thumb_height = if content_height == 0 {
            track_height
        } else {
            (track_height.saturating_mul(track_height) / content_height)
                .max(16)
                .min(track_height)
        };
        let max_offset = content_height.saturating_sub(track_height);
        let thumb_y = if max_offset == 0 {
            rect.y
        } else {
            rect.y + (track_height - thumb_height).saturating_mul(self.offset) / max_offset
        };
        canvas.fill_rect(
            Rect {
                x: rect.x + rect.width.saturating_sub(8),
                y: thumb_y,
                width: 6,
                height: thumb_height,
            },
            0x1596B5,
        );
    }

    pub fn handle_key(&mut self, key: u8, content_height: u32, viewport_height: u32) -> bool {
        let max_offset = content_height.saturating_sub(viewport_height);
        let next = match key {
            b'[' => self.offset.saturating_sub(24),
            b']' => self.offset.saturating_add(24).min(max_offset),
            _ => return false,
        };
        let changed = next != self.offset;
        self.offset = next;
        changed
    }
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
