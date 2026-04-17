use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, button::Button, rect::Rect};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use core::cell::{Cell, RefCell};
use alloc::string::String;

#[derive(Clone, Debug)]
pub struct Calculator {
    pub value: i64,
    pub operand: i64,
    pub op: u8, // 0:None, 1:+, 2:-, 3:*, 4:/, 5:%, 6:^
    pub inputing: bool,
    pub error: bool,
    pub scientific: bool,
    dirty: Cell<bool>,
    display_cache: RefCell<String>,
}

impl Calculator {
    pub fn new() -> Self {
        Calculator {
            value: 0,
            operand: 0,
            op: 0,
            inputing: false,
            error: false,
            scientific: false,
            dirty: Cell::new(true),
            display_cache: RefCell::new(String::new()),
        }
    }

    pub fn process_input(&mut self, key: char) {
        if self.error && key != 'C' && key != 'M' {
            return;
        }

        let changed = match key {
            '0'..='9' => {
                let digit = key as i64 - '0' as i64;
                if self.inputing {
                    self.value = self.value.saturating_mul(10).saturating_add(digit);
                } else {
                    self.value = digit;
                    self.inputing = true;
                }
                true
            }
            '+' | '-' | '*' | '/' | '%' | '^' => {
                if self.inputing {
                    self.calculate();
                }
                self.op = match key {
                    '+' => 1,
                    '-' => 2,
                    '*' => 3,
                    '/' => 4,
                    '%' => 5,
                    '^' => 6,
                    _ => 0,
                };
                self.operand = self.value;
                self.inputing = false;
                true
            }
            '=' => {
                if self.op != 0 {
                    self.calculate();
                    self.op = 0;
                    self.inputing = false;
                }
                true
            }
            'C' => {
                self.value = 0;
                self.operand = 0;
                self.op = 0;
                self.inputing = false;
                self.error = false;
                true
            }
            'M' => {
                self.scientific = !self.scientific;
                true
            }
            's' => { // Square
                self.value = self.value.saturating_mul(self.value);
                self.inputing = false;
                true
            }
            '!' => { // Factorial
                if self.value < 0 || self.value > 20 {
                    self.error = true;
                } else {
                    let mut res: i64 = 1;
                    for i in 1..=self.value {
                        res = res.saturating_mul(i);
                    }
                    self.value = res;
                }
                self.inputing = false;
                true
            }
            _ => false,
        };

        if changed {
            self.dirty.set(true);
        }
    }

    fn calculate(&mut self) {
        match self.op {
            1 => self.value = self.operand.saturating_add(self.value),
            2 => self.value = self.operand.saturating_sub(self.value),
            3 => self.value = self.operand.saturating_mul(self.value),
            4 => {
                if self.value != 0 {
                    self.value = self.operand / self.value;
                } else {
                    self.value = 0;
                    self.error = true;
                }
            }
            5 => { // Modulo
                if self.value != 0 {
                    self.value = self.operand % self.value;
                } else {
                    self.error = true;
                }
            }
            6 => { // Power
                if self.value < 0 {
                    self.error = true; // No negative exponents for integers
                } else {
                    // Basic integer power
                    let mut base = self.operand;
                    let mut exp = self.value;
                    let mut res: i64 = 1;
                    while exp > 0 {
                        if exp % 2 == 1 {
                            res = res.saturating_mul(base);
                        }
                        base = base.saturating_mul(base);
                        exp /= 2;
                    }
                    self.value = res;
                }
            }
            _ => {}
        }
    }
}

impl App for Calculator {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if gui::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
        let content_y = win.y + font_height as isize + 6;
        let width = win.width as isize;
        
        // 1. Draw Display
        gui::draw_rect(fb, win.x + 5, content_y + 5, win.width - 10, 30, 0x00_00_00_00, Some(dirty_rect)); // Black bg
        
        // Draw Mode Button inside display area (left side) using Button struct
        let mode_label = if self.scientific { "Sci" } else { "Std" };
        let mut mode_btn = Button::new(win.x + 7, content_y + 7, 30, 26, mode_label);
        mode_btn.bg_color = 0x00_60_60_60;
        mode_btn.text_color = 0x00_FF_FF_FF;
        mode_btn.draw(fb, 0, 0, Some(dirty_rect));

        if self.dirty.get() {
            let s: String = if self.error {
                String::from("Error")
            } else {
                alloc::format!("{}", self.value)
            };
            *self.display_cache.borrow_mut() = s;
            self.dirty.set(false);
        }

        let display_text = self.display_cache.borrow();
        let text_width = font::string_width(&**display_text);
        font::draw_string(fb, win.x + width - 10 - text_width as isize, content_y + 12, &**display_text, 0x00_00_FF_00, Some(dirty_rect));

        // 2. Draw Buttons
        let labels: &[&str] = if self.scientific {
            &[
                "7", "8", "9", "/", "Mod",
                "4", "5", "6", "*", "^",
                "1", "2", "3", "-", "Sq",
                "C", "0", "=", "+", "!"
            ]
        } else {
            &[
                "7", "8", "9", "/",
                "4", "5", "6", "*",
                "1", "2", "3", "-",
                "C", "0", "=", "+"
            ]
        };

        let cols = if self.scientific { 5 } else { 4 };
        let btn_size = 40;
        // Center buttons if width is larger than grid
        let grid_width = cols as isize * btn_size;
        let offset_x = (width - grid_width) / 2;

        for (i, label) in labels.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let bx = win.x + offset_x + (col as isize * btn_size);
            let by = content_y + 40 + (row as isize * btn_size);
            gui::draw_rect(fb, bx + 2, by + 2, 36, 36, 0x00_40_40_40, Some(dirty_rect)); // Button color
            font::draw_string(fb, bx + 15, by + 12, label, 0x00_FFFFFF, Some(dirty_rect));
        }
    }

    fn handle_event(&mut self, event: &AppEvent, _win: &Window) -> Option<Rect> {
        if let AppEvent::MouseClick { x, y, .. } = event {
            // Check for Mode Button Click
            if *y >= 7 && *y <= 33 && *x >= 7 && *x < 37 {
                self.process_input('M');
                return None;
            }

            if *y < 40 { return None; } // Clicked on display area
            
            let cols = if self.scientific { 5 } else { 4 };
            let btn_size = 40;
            
            // Adjust x for centered layout
            // Note: In handle_event, x is relative to window content area (left edge = 0)
            // The Window width is available in the event if needed, but App trait MouseClick has width.
            // We receive width in the event pattern match.
            let win_width = if let AppEvent::MouseClick { width, .. } = event { *width as isize } else { 200 };
            let grid_width = cols as isize * btn_size;
            let offset_x = (win_width - grid_width) / 2;
            
            if *x < offset_x { return None; }
            
            let btn_x = (*x - offset_x) / btn_size;
            let btn_y = (*y - 40) / btn_size;
            
            if btn_x >= cols as isize { return None; }

            let key = if self.scientific {
                match (btn_x, btn_y) {
                    (0, 0) => '7', (1, 0) => '8', (2, 0) => '9', (3, 0) => '/', (4, 0) => '%',
                    (0, 1) => '4', (1, 1) => '5', (2, 1) => '6', (3, 1) => '*', (4, 1) => '^',
                    (0, 2) => '1', (1, 2) => '2', (2, 2) => '3', (3, 2) => '-', (4, 2) => 's',
                    (0, 3) => 'C', (1, 3) => '0', (2, 3) => '=', (3, 3) => '+', (4, 3) => '!',
                    _ => return None,
                }
            } else {
                match (btn_x, btn_y) {
                    (0, 0) => '7', (1, 0) => '8', (2, 0) => '9', (3, 0) => '/',
                    (0, 1) => '4', (1, 1) => '5', (2, 1) => '6', (3, 1) => '*',
                    (0, 2) => '1', (1, 2) => '2', (2, 2) => '3', (3, 2) => '-',
                    (0, 3) => 'C', (1, 3) => '0', (2, 3) => '=', (3, 3) => '+',
                    _ => return None,
                }
            };
            self.process_input(key);
        }
        None
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new((*self).clone())
    }
}