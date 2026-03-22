use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window};
use super::app::{App, AppEvent};
use alloc::boxed::Box;

#[derive(Clone, Copy, Debug)]
pub struct Calculator {
    pub value: i64,
    pub operand: i64,
    pub op: u8, // 0:None, 1:+, 2:-, 3:*, 4:/
    pub inputing: bool,
    pub error: bool,
}

impl Calculator {
    pub const fn new() -> Self {
        Calculator {
            value: 0,
            operand: 0,
            op: 0,
            inputing: false,
            error: false,
        }
    }

    pub fn process_input(&mut self, key: char) {
        if self.error && key != 'C' {
            return;
        }

        match key {
            '0'..='9' => {
                let digit = key as i64 - '0' as i64;
                if self.inputing {
                    self.value = self.value.saturating_mul(10).saturating_add(digit);
                } else {
                    self.value = digit;
                    self.inputing = true;
                }
            }
            '+' | '-' | '*' | '/' => {
                if self.inputing {
                    self.calculate();
                }
                self.op = match key {
                    '+' => 1,
                    '-' => 2,
                    '*' => 3,
                    '/' => 4,
                    _ => 0,
                };
                self.operand = self.value;
                self.inputing = false;
            }
            '=' => {
                if self.op != 0 {
                    self.calculate();
                    self.op = 0;
                    self.inputing = false;
                }
            }
            'C' => {
                self.value = 0;
                self.operand = 0;
                self.op = 0;
                self.inputing = false;
                self.error = false;
            }
            _ => {}
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
            _ => {}
        }
    }
}

impl App for Calculator {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window) {
        let start_y = win.y + 20;
        let width = win.width as isize;
        
        // 1. Draw Display
        gui::draw_rect(fb, win.x + 5, start_y + 5, win.width - 10, 30, 0x00_00_00_00, None); // Black bg
        
        // Format number to string
        let mut buffer = [0u8; 20];
        let mut val = self.value;
        
        let s = if self.error {
            "Error"
        } else {
            let len;
            if val == 0 {
                buffer[0] = b'0';
                len = 1;
            } else {
                let neg = val < 0;
                if neg { val = -val; }
                let mut i = 19;
                while val > 0 && i > 0 {
                    buffer[i] = b'0' + (val % 10) as u8;
                    val /= 10;
                    i -= 1;
                }
                if neg { buffer[i] = b'-'; i -= 1; }
                // shift to front
                for j in 0..(19 - i) { buffer[j] = buffer[i + 1 + j]; }
                len = 19 - i;
            }
            core::str::from_utf8(&buffer[0..len]).unwrap_or("Err")
        };
        
        font::draw_string(fb, win.x + width - 10 - (s.len() as isize * 8), start_y + 12, s, 0x00_00_FF_00, None);

        // 2. Draw Buttons
        let labels = ["7", "8", "9", "/", "4", "5", "6", "*", "1", "2", "3", "-", "C", "0", "=", "+"];
        for (i, label) in labels.iter().enumerate() {
            let row = i / 4;
            let col = i % 4;
            let bx = win.x + (col as isize * 40);
            let by = start_y + 40 + (row as isize * 40);
            gui::draw_rect(fb, bx + 2, by + 2, 36, 36, 0x00_40_40_40, None); // Button color
            font::draw_string(fb, bx + 15, by + 12, label, 0x00_FFFFFF, None);
        }
    }

    fn handle_event(&mut self, event: &AppEvent) {
        if let AppEvent::MouseClick { x, y, .. } = event {
            if *y < 40 { return; } // Clicked on display area
            let btn_y = (*y - 40) / 40;
            let btn_x = *x / 40;
            let key = match (btn_x, btn_y) {
                (0, 0) => '7', (1, 0) => '8', (2, 0) => '9', (3, 0) => '/',
                (0, 1) => '4', (1, 1) => '5', (2, 1) => '6', (3, 1) => '*',
                (0, 2) => '1', (1, 2) => '2', (2, 2) => '3', (3, 2) => '-',
                (0, 3) => 'C', (1, 3) => '0', (2, 3) => '=', (3, 3) => '+',
                _ => return,
            };
            self.process_input(key);
        }
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new((*self).clone())
    }
}