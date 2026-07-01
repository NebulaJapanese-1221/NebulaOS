use crate::framebuffer::Framebuffer;
use crate::gui::{draw_string, Window, WindowManager, AppType, TITLE_BAR_HEIGHT};

// Constants for layout and appearance
const START_MENU_WIDTH: u32 = 250;
const START_MENU_HEIGHT: u32 = 400;
const MENU_ITEM_HEIGHT: u32 = 30;
const MENU_ITEM_PADDING: u32 = 5;
const MENU_BG_COLOR: u32 = 0x001A1A1A;
const MENU_ITEM_BG_COLOR: u32 = 0x00333333;
const MENU_TEXT_COLOR: u32 = 0xFFFFFF;

// Make AppType public in window_manager.rs to include Terminal
// For now, assuming it's fixed.

pub fn draw(fb: &mut Framebuffer, y_start: u32) {
    
    let menu_y = y_start - START_MENU_HEIGHT;
    
    fb.draw_rect(0, menu_y as usize, START_MENU_WIDTH as usize, START_MENU_HEIGHT as usize, MENU_BG_COLOR);
    
    let mut current_y = menu_y + MENU_ITEM_PADDING;

    let item_area_x_start = MENU_ITEM_PADDING as usize;
    let item_area_x_end = (START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as usize;

    // Calculator Entry
    fb.draw_rect(item_area_x_start, current_y as usize, item_area_x_end, MENU_ITEM_HEIGHT as usize, MENU_ITEM_BG_COLOR);
    draw_string(fb, (MENU_ITEM_PADDING + 5) as usize, (current_y + 8) as usize, "Calculator", MENU_TEXT_COLOR);
    current_y += MENU_ITEM_HEIGHT + MENU_ITEM_PADDING;

    // Text Editor Entry
    fb.draw_rect(item_area_x_start, current_y as usize, item_area_x_end, MENU_ITEM_HEIGHT as usize, MENU_ITEM_BG_COLOR);
    draw_string(fb, (MENU_ITEM_PADDING + 5) as usize, (current_y + 8) as usize, "Text Editor", MENU_TEXT_COLOR);
    current_y += MENU_ITEM_HEIGHT + MENU_ITEM_PADDING;
    
    // Terminal Entry
    fb.draw_rect(item_area_x_start, current_y as usize, item_area_x_end, MENU_ITEM_HEIGHT as usize, MENU_ITEM_BG_COLOR);
    draw_string(fb, (MENU_ITEM_PADDING + 5) as usize, (current_y + 8) as usize, "Terminal", MENU_TEXT_COLOR);
}

pub fn handle_click(mx: i32, my: i32, height: i32, wm: &mut WindowManager, start_menu_open: &mut bool) {
    let taskbar_top = height - TITLE_BAR_HEIGHT as i32; // Top edge of the taskbar
    let menu_y = taskbar_top - START_MENU_HEIGHT as i32; // Top edge of the start menu

    // Check Start Button click
    if mx >= 5 && mx <= 75 && my >= taskbar_top + 5 && my <= taskbar_top + 35 {
        *start_menu_open = !*start_menu_open;
        return; // Don't process other clicks if start button was clicked
    }

    if !*start_menu_open {
        return; // If menu is closed, do nothing
    }

    let mut opened_app = false;

    let item_area_x_start = MENU_ITEM_PADDING as i32;
    let item_area_x_end = (MENU_ITEM_PADDING + START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as i32;
    
    let mut current_item_y = menu_y + MENU_ITEM_PADDING as i32;

    // Calculator entry
    if mx >= item_area_x_start && mx <= item_area_x_end &&
       my >= current_item_y && my <= current_item_y + MENU_ITEM_HEIGHT as i32 {
        wm.windows.push(Window::new("Calculator", 300, 200, 220, 300, AppType::Calculator));
        opened_app = true;
    }
        current_item_y += (MENU_ITEM_HEIGHT + MENU_ITEM_PADDING) as i32;

    // Text Editor entry
    if mx >= item_area_x_start && mx <= item_area_x_end &&
       my >= current_item_y && my <= current_item_y + MENU_ITEM_HEIGHT as i32 {
        wm.windows.push(Window::new("Text Editor", 350, 250, 400, 300, AppType::TextEditor));
        opened_app = true;
    }
        current_item_y += (MENU_ITEM_HEIGHT + MENU_ITEM_PADDING) as i32;
    
    // Terminal entry
    if mx >= item_area_x_start && mx <= item_area_x_end &&
       my >= current_item_y && my <= current_item_y + MENU_ITEM_HEIGHT as i32 {
        // AppType::Terminal needs to be defined in window_manager.rs
        wm.windows.push(Window::new("Terminal", 400, 300, 600, 400, AppType::Terminal));
        opened_app = true;
    }

    if opened_app {
        *start_menu_open = false; // Close menu if an app was launched
    }
}