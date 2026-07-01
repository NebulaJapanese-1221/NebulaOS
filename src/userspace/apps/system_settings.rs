use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::{String, ToString};

// --- SystemSettingsState ---
pub struct SystemSettingsState {
    pub kernel_uptime_str: String, // Placeholder for kernel uptime
    pub selected_option: usize, // For navigating settings
}

impl SystemSettingsState {
    pub fn new() -> Self {
        Self {
            kernel_uptime_str: "N/A".to_string(), // Placeholder for kernel uptime
            selected_option: 0, // Default to first option
        }
    }

    pub fn update_info(&mut self) {
        // Fetch kernel uptime (This is a placeholder, requires a kernel syscall)
        // For now, just indicate it's being checked. A real implementation
        // would use a syscall like `get_kernel_uptime()`.
        self.kernel_uptime_str = "Checking...".to_string();
    }
}

// --- SystemSettingsApp ---
pub struct SystemSettingsApp;

impl SystemSettingsApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &SystemSettingsState) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Background for the settings content
        fb.draw_rect(x, y, w, h, 0x00202020); // Dark grey background

        // --- Settings Menu ---
        let menu_width = 100;
        let menu_x = x + 10;
        let menu_y = y + 10;
        let menu_item_height = 25;
        let menu_item_spacing = 5;

        // Menu items (only System Info — time is shown on the taskbar)
        let options = ["System Info"];
        for (i, &option) in options.iter().enumerate() {
            let item_y = menu_y + i * (menu_item_height as usize + menu_item_spacing as usize);
            let item_color = if i == state.selected_option { 0x00FFFFFF } else { 0x00AAAAAA }; // Highlight selected
            draw_string(fb, menu_x + 5, item_y + 5, option, item_color);
        }

        // --- Content Area ---
        let content_x = menu_x + menu_width + 10;
        let content_y = menu_y;
        let _content_w = w - menu_width - 20;
        let content_h = h - 20;

        // Draw a separator line
        fb.draw_rect(content_x - 5, content_y, 1, content_h, 0x00AAAAAA);

        // Display information (System Info)
        match state.selected_option {
            0 => {
                let uptime_text = &state.kernel_uptime_str;
                let text_x = content_x + 10;
                let text_y = content_y + 10;
                draw_string(fb, text_x, text_y, "Kernel Uptime:", 0x00AAAAAA);
                draw_string(fb, text_x + 5, text_y + 15, uptime_text.as_str(), 0x00FFFFFF);
            }
            _ => {}
        }
    }

    // Handle clicks to select options or interact with settings
    pub fn handle_click(state: &mut SystemSettingsState, bounds: Rect, mx: i32, my: i32) {
        let app_x = bounds.x as i32;
        let app_y = bounds.y as i32 + TITLE_BAR_HEIGHT as i32;
        let app_w = bounds.width as i32;
        let app_h = (bounds.height as i32).saturating_sub(TITLE_BAR_HEIGHT as i32);

        // Check if click is within the app area
        if mx >= app_x && mx < app_x + app_w && my >= app_y && my < app_y + app_h {
            
            // Check clicks on menu items
            let _menu_width = 100;
            let _menu_x = app_x + 10;
            let menu_y = app_y + 10;
            let menu_item_height = 25;
            let menu_item_spacing = 5;
            let options = ["System Info"]; // Must match draw function

            for (i, &_option) in options.iter().enumerate() {
                let item_y_start = menu_y + i as i32 * (menu_item_height as i32 + menu_item_spacing as i32);
                let item_y_end = item_y_start + menu_item_height as i32;

                if my >= item_y_start && my < item_y_end {
                    state.selected_option = i;
                    state.update_info(); // Update displayed info when an option is selected
                    return; // Consume the click
                }
            }
            
            // If click was not on a menu item, maybe it was on a setting control
            // For now, just refresh time if clicking anywhere in the app area
            state.update_info(); 
        }
    }

    // Handle keyboard input for navigation or changing settings
    pub fn handle_keyboard_input(state: &mut SystemSettingsState, c: char) {
        match c {
            // Navigate menu with arrow keys (or simple keys for now)
            // 'w' for up, 's' for down
            'w' | 'W' => {
                if state.selected_option > 0 {
                    state.selected_option -= 1;
                }
            }
            's' | 'S' => {
                // Only one option available, nothing to do
            }
            '\n' => { // Enter key to confirm selection or activate option
                state.update_info(); // Refresh info when Enter is pressed
            }
            _ => {} // Ignore other keys
        }
    }
}