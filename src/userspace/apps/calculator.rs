use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::{String, ToString}; // Import ToString
use alloc::format;
use libm;


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
            display: "0".to_string(),
            last_val: 0,
            op: None,
            clear_on_next: false,
            mode: CalcMode::Standard,
            graph_func: "x".to_string(),
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
        let text_offset = state.display.len() * 8; // Assuming 8 pixels per character width
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
        // Scientific mode has 20 buttons (4x5 grid), Standard has 16 (4x4 grid).
        // Let's use 4 columns for all.
        let labels: &[&str] = match state.mode {
            CalcMode::Standard => &[
                "7", "8", "9", "/",
                "4", "5", "6", "*",
                "1", "2", "3", "-",
                "0", "C", "=", "+"
            ],
            CalcMode::Scientific => &[
                "sin", "cos", "tan", "sqrt", // Row 1 (4 buttons)
                "7", "8", "9", "/",          // Row 2 (4 buttons)
                "4", "5", "6", "*",          // Row 3 (4 buttons)
                "1", "2", "3", "-",          // Row 4 (4 buttons)
                "0", "C", "=", "+"           // Row 5 (4 buttons)
            ],
            _ => &[], // Should not happen for draw_buttons
        };

        let padding = 8;
        let cols = 4;
        let rows = match state.mode {
            CalcMode::Standard => 4,
            CalcMode::Scientific => 5,
            _ => 0,
        };
        
        let cell_w = (w - padding * 2) / cols;
        let cell_h = (h - 60) / rows; // 60 for display area and padding

        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                if let Some(label) = labels.get(idx) {
                    let bx = x + padding + c * cell_w;
                    let by = y + 55 + r * cell_h; // 55 for display + padding
                    
                    // Draw Button background
                    fb.draw_rect(bx + 2, by + 2, cell_w - 4, cell_h - 4, 0x00D0D0D0);
                    
                    // Draw Button Label - Centered
                    let label_len = label.len();
                    let text_x = bx + cell_w / 2 - (label_len * 4); // Approx centering (8 pixels/char / 2)
                    let text_y = by + cell_h / 2 - 4; // Approx centering (8 pixels height / 2)
                    draw_string(fb, text_x, text_y, label, 0x00000000);
                }
            }
        }
    }

    fn draw_graph(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, state: &CalculatorState) {
        let graph_area_y_offset = 55; // Space for display and mode text
        let graph_x = x + 20;
        let graph_y = y + graph_area_y_offset;
        let graph_w = w - 40;
        let graph_h = h - graph_area_y_offset - 20; // Account for bottom padding

        // Draw Graph Background
        fb.draw_rect(graph_x, graph_y, graph_w, graph_h, 0x00111111);
        
        // Draw Axes
        let center_x = graph_x + graph_w / 2;
        let center_y = graph_y + graph_h / 2;
        
        // X-Axis
        fb.draw_rect(graph_x, center_y, graph_w, 1, 0x00FFFFFF);
        // Y-Axis
        fb.draw_rect(center_x, graph_y, 1, graph_h, 0x00FFFFFF);

        // Plot Function (Very basic plotting)
        // Scale factors for mapping world coordinates to pixel coordinates
        let x_scale = 20.0; 
        let y_scale = 20.0;

        for px in 0..graph_w {
            // Map pixel x to world x
            let world_x = (px as f32 - (graph_w / 2) as f32) / x_scale;
            
            // Evaluate function based on graph_func string
            let world_y = if state.graph_func.contains("x*x") { // Simple parabola check
                world_x * world_x
            } else if state.graph_func.contains("x") { // Simple line check
                world_x
            } else {
                0.0 // Default if function is unknown
            };

            // Map world y to pixel y
            let py = center_y as i32 - (world_y * y_scale) as i32;
            
            // Draw the point if it's within the graph area bounds
            if py >= graph_y as i32 && py < (graph_y + graph_h) as i32 {
                fb.draw_rect(graph_x + px, py as usize, 1, 1, 0x0000FF00); // Plot in blue
            }
        }
        // Display the function string
        draw_string(fb, x + 10, y + graph_area_y_offset as usize, &format!("f(x)={}", state.graph_func), 0x00FFFFFF);
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

        // If in graphing mode, clicking outside the buttons doesn't do anything for now.
        if state.mode == CalcMode::Graphing { return; }

        let padding = 8;
        let cols = 4;
        let rows = match state.mode {
            CalcMode::Standard => 4,
            CalcMode::Scientific => 5,
            _ => 0,
        };
        let cell_w = (w - padding * 2) / cols;
        let cell_h = (h - 60) / rows; // 60 for display area and padding

        let rel_x = mx - x - padding;
        let rel_y = my - y - 55; // 55 for display and top padding

        // Check if the click is within the button grid area
        if rel_x >= 0 && rel_x < (w - padding * 2) && rel_y >= 0 && rel_y < (h - 60) {
            let col = rel_x / cell_w;
            let row = rel_y / cell_h;
            
            let labels: &[&str] = match state.mode {
                CalcMode::Standard => &[
                    "7", "8", "9", "/",
                    "4", "5", "6", "*",
                    "1", "2", "3", "-",
                    "0", "C", "=", "+"
                ],
                CalcMode::Scientific => &[
                    "sin", "cos", "tan", "sqrt", // Row 1 (4 buttons)
                    "7", "8", "9", "/",          // Row 2 (4 buttons)
                    "4", "5", "6", "*",          // Row 3 (4 buttons)
                    "1", "2", "3", "-",          // Row 4 (4 buttons)
                    "0", "C", "=", "+"           // Row 5 (4 buttons)
                ],
                _ => &[], // Should not happen for handle_click when mode is not Graphing
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
            "+" | "-" => { // Basic arithmetic operations
                state.last_val = state.display.parse().unwrap_or(0);
                state.op = Some(label.chars().next().unwrap());
                state.clear_on_next = true;
            }
            "=" => { // Equals button
                let current_val: i32 = state.display.parse().unwrap_or(0);
                let result = match state.op {
                    Some('+') => state.last_val + current_val,
                    Some('-') => state.last_val - current_val,
                    // TODO: Implement other operations (*, /) and handle potential errors
                    _ => current_val, // If no operation, result is the current value
                };
                state.display = result.to_string();
                state.op = None;
                state.clear_on_next = true;
            }
            "C" => { // Clear button
                state.display = "0".to_string();
                state.last_val = 0;
                state.op = None;
                state.clear_on_next = false;
            }
            // Scientific functions
            "sin" | "cos" | "tan" | "sqrt" => {
                let val: f32 = state.display.parse().unwrap_or(0.0);
                let res = match label {
                    "sin" => libm::sinf(val.to_radians()),
                    "cos" => libm::cosf(val.to_radians()),
                    "tan" => libm::tanf(val.to_radians()),
                    "sqrt" => libm::sqrtf(val),
                    _ => 0.0, // Should not happen
                };
                // Format result to a reasonable precision
                state.display = format!("{:.2}", res); 
                state.clear_on_next = true;
            }
            // Handle other operations like '*' and '/' if needed
            _ => {} // Ignore unknown labels
        }
    }

    pub fn handle_keyboard_input(state: &mut CalculatorState, c: char) {
        if c == 'm' { // Toggle mode
            state.mode = match state.mode {
                CalcMode::Standard => CalcMode::Scientific,
                CalcMode::Scientific => CalcMode::Graphing,
                CalcMode::Graphing => CalcMode::Standard,
            };
        } else if state.mode != CalcMode::Graphing { // Only process input if not in graphing mode
            // Map keyboard characters to calculator buttons
            let label = match c {
                '0'..='9' => c.to_string(),
                '+' => "+".to_string(),
                '-' => "-".to_string(),
                '=' | '\n' => "=".to_string(), // Enter key acts as '='
                'c' | 'C' => "C".to_string(),
                '*' => "*".to_string(), // Add multiplication
                '/' => "/".to_string(), // Add division
                // Add sin, cos, tan, sqrt mappings if desired
                // 's' => "sin".to_string(), 
                _ => return, // Ignore other keys
            };
            Self::process_input(state, label.as_str());
        }
    }
}