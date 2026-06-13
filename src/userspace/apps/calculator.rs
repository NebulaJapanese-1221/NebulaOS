use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;

pub struct CalculatorState {
    pub display: String,
    pub last_val: i32,
    pub op: Option<char>,
    pub clear_on_next: bool,
}

impl CalculatorState {
    pub fn new() -> Self {
        Self {
            display: String::from("0"),
            last_val: 0,
            op: None,
            clear_on_next: false,
        }
    }
}

pub struct CalculatorApp;

impl CalculatorApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &CalculatorState) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Calculator Display Area
        fb.draw_rect(x + 10, y + 10, w - 20, 35, 0x00FFFFFF);
        let text_offset = state.display.len() * 8;
        draw_string(fb, x + w - text_offset - 15, y + 20, state.display.as_str(), 0x00000000);

        // Buttons Grid Layout
        let labels = [
            "7", "8", "9", "/",
            "4", "5", "6", "*",
            "1", "2", "3", "-",
            "0", "C", "=", "+"
        ];

        let padding = 8;
        let cell_w = (w - padding * 2) / 4;
        let cell_h = (h - 60) / 4;

        for r in 0..4 {
            for c in 0..4 {
                let bx = x + padding + c * cell_w;
                let by = y + 55 + r * cell_h;
                
                // Draw Button background
                fb.draw_rect(bx + 2, by + 2, cell_w - 4, cell_h - 4, 0x00D0D0D0);
                
                // Draw Button Label
                draw_string(fb, bx + cell_w / 2 - 4, by + cell_h / 2 - 4, labels[r * 4 + c], 0x00000000);
            }
        }
    }

    pub fn handle_click(state: &mut CalculatorState, bounds: Rect, mx: i32, my: i32) {
        let x = bounds.x as i32;
        let y = bounds.y as i32 + TITLE_BAR_HEIGHT as i32;
        let w = bounds.width as i32;
        let h = (bounds.height as i32).saturating_sub(TITLE_BAR_HEIGHT as i32);

        let padding = 8;
        let cell_w = (w - padding * 2) / 4;
        let cell_h = (h - 60) / 4;

        let rel_x = mx - x - padding;
        let rel_y = my - y - 55;

        // Check if the click is within the button grid area
        if rel_x >= 0 && rel_x < (w - padding * 2) && rel_y >= 0 && rel_y < (h - 60) {
            let col = rel_x / cell_w;
            let row = rel_y / cell_h;
            
            let labels = [
                "7", "8", "9", "/",
                "4", "5", "6", "*",
                "1", "2", "3", "-",
                "0", "C", "=", "+"
            ];
            
            if let Some(label) = labels.get((row * 4 + col) as usize) {
                match *label {
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        if state.clear_on_next {
                            state.display.clear();
                            state.clear_on_next = false;
                        }
                        if state.display == "0" { state.display.clear(); }
                        state.display.push_str(*label);
                    }
                    "+" | "-" => {
                        state.last_val = state.display.parse().unwrap_or(0);
                        state.op = Some(label.chars().next().unwrap());
                        state.clear_on_next = true;
                    }
                    "=" => {
                        let current_val: i32 = state.display.parse().unwrap_or(0);
                        let result = match state.op {
                            Some('+') => state.last_val + current_val,
                            Some('-') => state.last_val - current_val,
                            _ => current_val,
                        };
                        state.display = alloc::format!("{}", result);
                        state.op = None;
                        state.clear_on_next = true;
                    }
                    "C" => {
                        state.display = String::from("0");
                        state.last_val = 0;
                        state.op = None;
                        state.clear_on_next = false;
                    }
                    "*" | "/" => {
                        // Multiplication/Division placeholders
                        crate::serial_println!("Op {} not yet implemented", label);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn handle_keyboard_input(state: &mut CalculatorState, c: char) {
        match c {
            '0'..='9' => {
                if state.clear_on_next {
                    state.display.clear();
                    state.clear_on_next = false;
                }
                if state.display == "0" { state.display.clear(); }
                state.display.push(c);
            }
            '+' | '-' => {
                state.last_val = state.display.parse().unwrap_or(0);
                state.op = Some(c);
                state.clear_on_next = true;
            }
            '=' | '\n' => { // Enter key acts as '='
                let current_val: i32 = state.display.parse().unwrap_or(0);
                let result = match state.op {
                    Some('+') => state.last_val + current_val,
                    Some('-') => state.last_val - current_val,
                    _ => current_val,
                };
                state.display = alloc::format!("{}", result);
                state.op = None;
                state.clear_on_next = true;
            }
            'c' | 'C' => {
                state.display = String::from("0");
                state.last_val = 0;
                state.op = None;
                state.clear_on_next = false;
            }
            '*' | '/' => {
                crate::serial_println!("Op {} not yet implemented", c);
            }
            _ => {}
        }
    }
}