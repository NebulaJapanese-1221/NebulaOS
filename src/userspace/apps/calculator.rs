use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(PartialEq)]
pub enum CalcMode {
    Standard,
    Scientific,
    Graphing,
}

pub struct CalculatorState {
    pub display: String,
    pub last_val: i32,
    pub op: Option<char>,
    pub clear_on_next: bool,
    pub mode: CalcMode,
    pub graph_func: String, // Simple expression for graphing, e.g., "x*x"
}

impl CalculatorState {
    pub fn new() -> Self {
        Self {
            display: String::from("0"),
            last_val: 0,
            op: None,
            clear_on_next: false,
            mode: CalcMode::Standard,
            graph_func: String::from("x"),
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

        // --- Mode Header ---
        let mode_text = match state.mode {
            CalcMode::Standard => "STD",
            CalcMode::Scientific => "SCI",
            CalcMode::Graphing => "GRPH",
        };
        draw_string(fb, x + 5, y + 2, mode_text, 0x00AAAAAA);

        // Display Area
        fb.draw_rect(x + 10, y + 10, w - 20, 35, 0x00FFFFFF);
        let text_offset = state.display.len() * 8;
        draw_string(fb, x + w - text_offset - 15, y + 20, state.display.as_str(), 0x00000000);

        match state.mode {
            CalcMode::Standard | CalcMode::Scientific => {
                Self::draw_buttons(fb, x, y, w, h, state);
            }
            CalcMode::Graphing => {
                Self::draw_graph(fb, x, y, w, h, state);
            }
        }
    }

    fn draw_buttons(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, state: &CalculatorState) {
        let labels = match state.mode {
            CalcMode::Standard => [
                "7", "8", "9", "/",
                "4", "5", "6", "*",
                "1", "2", "3", "-",
                "0", "C", "=", "+"
            ],
            CalcMode::Scientific => [
                "sin", "cos", "tan", "sqrt",
                "7", "8", "9", "/",
                "4", "5", "6", "*",
                "1", "2", "3", "-",
                "0", "C", "=", "+"
            ],
            _ => [],
        };

        let padding = 8;
        let cols = if state.mode == CalcMode::Scientific { 4 } else { 4 };
        let rows = if state.mode == CalcMode::Scientific { 5 } else { 4 };
        
        let cell_w = (w - padding * 2) / cols;
        let cell_h = (h - 60) / rows;

        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                if let Some(label) = labels.get(idx) {
                    let bx = x + padding + c * cell_w;
                    let by = y + 55 + r * cell_h;
                    fb.draw_rect(bx + 2, by + 2, cell_w - 4, cell_h - 4, 0x00D0D0D0);
                    draw_string(fb, bx + cell_w / 2 - 4, by + cell_h / 2 - 4, *label, 0x00000000);
                }
            }
        }
    }

    fn draw_graph(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, state: &CalculatorState) {
        let graph_x = x + 20;
        let graph_y = y + 60;
        let graph_w = w - 40;
        let graph_h = h - 80;

        // Draw Graph Background
        fb.draw_rect(graph_x, graph_y, graph_w, graph_h, 0x00111111);
        
        // Draw Axes
        let center_x = graph_x + graph_w / 2;
        let center_y = graph_y + graph_h / 2;
        
        // X-Axis
        fb.draw_rect(graph_x, center_y, graph_w, 1, 0x00FFFFFF);
        // Y-Axis
        fb.draw_rect(center_x, graph_y, 1, graph_h, 0x00FFFFFF);

        // Plot Function (Very basic plotting: y = x^2 or y = x)
        for px in 0..graph_w {
            let world_x = (px as f32 - (graph_w / 2) as f32) / 20.0;
            
            // Simple Parser simulation: if "x*x" -> parabola, if "x" -> line
            let world_y = if state.graph_func.contains("*") {
                world_x * world_x
            } else {
                world_x
            };

            let py = center_y as i32 - (world_y * 20.0) as i32;
            if py >= graph_y as i32 && py < (graph_y + graph_h) as i32 {
                fb.draw_rect(graph_x + px, py as usize, 1, 1, 0x0000FF00);
            }
        }
        draw_string(fb, x + 10, y + 50, &format!("f(x)={}", state.graph_func), 0x00FFFFFF);
    }

    pub fn handle_click(state: &mut CalculatorState, bounds: Rect, mx: i32, my: i32) {
        let x = bounds.x as i32;
        let y = bounds.y as i32 + TITLE_BAR_HEIGHT as i32;
        let w = bounds.width as i32;
        let h = (bounds.height as i32).saturating_sub(TITLE_BAR_HEIGHT as i32);

        // Mode Switching buttons (Top small buttons)
        if my > y && my < y + 20 {
            if mx > x && mx < x + 40 { state.mode = CalcMode::Standard; return; }
            if mx > x + 45 && mx < x + 85 { state.mode = CalcMode::Scientific; return; }
            if mx > x + 90 && mx < x + 130 { state.mode = CalcMode::Graphing; return; }
        }

        if state.mode == CalcMode::Graphing { return; }

        let padding = 8;
        let cols = if state.mode == CalcMode::Scientific { 4 } else { 4 };
        let rows = if state.mode == CalcMode::Scientific { 5 } else { 4 };
        let cell_w = (w - padding * 2) / cols;
        let cell_h = (h - 60) / rows;

        let rel_x = mx - x - padding;
        let rel_y = my - y - 55;

        if rel_x >= 0 && rel_x < (w - padding * 2) && rel_y >= 0 && rel_y < (h - 60) {
            let col = rel_x / cell_w;
            let row = rel_y / cell_h;
            
            let labels = match state.mode {
                CalcMode::Standard => [
                    "7", "8", "9", "/",
                    "4", "5", "6", "*",
                    "1", "2", "3", "-",
                    "0", "C", "=", "+"
                ],
                CalcMode::Scientific => [
                    "sin", "cos", "tan", "sqrt",
                    "7", "8", "9", "/",
                    "4", "5", "6", "*",
                    "1", "2", "3", "-",
                    "0", "C", "=", "+"
                ],
                _ => [],
            };
            
            if let Some(label) = labels.get((row * cols + col) as usize) {
                Self::process_input(state, *label);
            }
        }
    }

    fn process_input(state: &mut CalculatorState, label: &str) {
        match label {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                if state.clear_on_next {
                    state.display.clear();
                    state.clear_on_next = false;
                }
                if state.display == "0" { state.display.clear(); }
                state.display.push_str(label);
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
            "sin" | "cos" | "tan" | "sqrt" => {
                let val: f32 = state.display.parse().unwrap_or(0.0);
                let res = match label {
                    "sin" => val.to_radians().sin(),
                    "cos" => val.to_radians().cos(),
                    "tan" => val.to_radians().tan(),
                    "sqrt" => val.sqrt(),
                    _ => 0.0,
                };
                state.display = alloc::format!("{:.2}", res);
                state.clear_on_next = true;
            }
            _ => {}
        }
    }

    pub fn handle_keyboard_input(state: &mut CalculatorState, c: char) {
        if c == 'm' { // Toggle mode
            state.mode = match state.mode {
                CalcMode::Standard => CalcMode::Scientific,
                CalcMode::Scientific => CalcMode::Graphing,
                CalcMode::Graphing => CalcMode::Standard,
            };
        } else if state.mode != CalcMode::Graphing {
            // Map keyboard characters to buttons
            let label = match c {
                '0'..='9' => c.to_string(),
                '+' => "+".to_string(),
                '-' => "-".to_string(),
                '=' | '\n' => "=".to_string(),
                'c' | 'C' => "C".to_string(),
                _ => return,
            };
            Self::process_input(state, &label);
        }
    }
}