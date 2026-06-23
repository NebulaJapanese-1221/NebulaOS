use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, Window, WindowManager, AppType, TITLE_BAR_HEIGHT};

const START_MENU_WIDTH: u32 = 250;
const START_MENU_HEIGHT: u32 = 400;
const MENU_ITEM_HEIGHT: u32 = 30;
const MENU_ITEM_PADDING: u32 = 5;
const MENU_BG_COLOR: u32 = 0x001A1A1A;
const MENU_ITEM_BG_COLOR: u32 = 0x00333333;
const MENU_TEXT_COLOR: u32 = 0xFFFFFF;

pub fn draw(fb: &mut Framebuffer, y_start: u32, start_menu_open: bool) {
    if !start_menu_open {
        return;
    }
    
    let menu_y = y_start - START_MENU_HEIGHT;
    
    // Menu Background
    fb.draw_rect(0, menu_y, START_MENU_WIDTH as usize, START_MENU_HEIGHT as usize, MENU_BG_COLOR);
    
    let mut current_y = menu_y + MENU_ITEM_PADDING;

    // Calculator Entry
    fb.draw_rect(MENU_ITEM_PADDING as usize, current_y as usize, (START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as usize, MENU_ITEM_HEIGHT as usize, MENU_ITEM_BG_COLOR);
    draw_string(fb, (MENU_ITEM_PADDING + 5) as usize, (current_y + 8) as usize, "Calculator", MENU_TEXT_COLOR);
    current_y += MENU_ITEM_HEIGHT + MENU_ITEM_PADDING;

    // Text Editor Entry
    fb.draw_rect(MENU_ITEM_PADDING as usize, current_y as usize, (START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as usize, MENU_ITEM_HEIGHT as usize, MENU_ITEM_BG_COLOR);
    draw_string(fb, (MENU_ITEM_PADDING + 5) as usize, (current_y + 8) as usize, "Text Editor", MENU_TEXT_COLOR);
    current_y += MENU_ITEM_HEIGHT + MENU_ITEM_PADDING;
    
    // Terminal Entry
    fb.draw_rect(MENU_ITEM_PADDING as usize, current_y as usize, (START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as usize, MENU_ITEM_HEIGHT as usize, MENU_ITEM_BG_COLOR);
    draw_string(fb, (MENU_ITEM_PADDING + 5) as usize, (current_y + 8) as usize, "Terminal", MENU_TEXT_COLOR);
    current_y += MENU_ITEM_HEIGHT + MENU_ITEM_PADDING;
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

    // Check Calculator entry
    let calc_item_y_start = menu_y + MENU_ITEM_PADDING as i32;
    if mx >= MENU_ITEM_PADDING as i32 && mx <= (MENU_ITEM_PADDING + START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as i32 &&
       my >= calc_item_y_start && my <= calc_item_y_start + MENU_ITEM_HEIGHT as i32 {
        wm.windows.push(Window::new("Calculator", 300, 200, 220, 300, AppType::Calculator));
        opened_app = true;
    }

    // Check Text Editor entry
    let text_editor_item_y_start = calc_item_y_start + MENU_ITEM_HEIGHT as i32 + MENU_ITEM_PADDING as i32;
    if mx >= MENU_ITEM_PADDING as i32 && mx <= (MENU_ITEM_PADDING + START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as i32 &&
       my >= text_editor_item_y_start && my <= text_editor_item_y_start + MENU_ITEM_HEIGHT as i32 {
        wm.windows.push(Window::new("Text Editor", 350, 250, 400, 300, AppType::TextEditor));
        opened_app = true;
    }

    // Check Terminal entry
    let terminal_item_y_start = text_editor_item_y_start + MENU_ITEM_HEIGHT as i32 + MENU_ITEM_PADDING as i32;
    if mx >= MENU_ITEM_PADDING as i32 && mx <= (MENU_ITEM_PADDING + START_MENU_WIDTH - MENU_ITEM_PADDING * 2) as i32 &&
       my >= terminal_item_y_start && my <= terminal_item_y_start + MENU_ITEM_HEIGHT as i32 {
        wm.windows.push(Window::new("Terminal", 400, 300, 600, 400, AppType::Terminal));
        opened_app = true;
    }

    if opened_app {
        *start_menu_open = false; // Close menu if an app was launched
    }
}