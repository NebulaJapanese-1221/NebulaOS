use crate::framebuffer::Framebuffer;
use super::{draw_string, Window, WindowManager, AppType};

pub fn draw(fb: &mut Framebuffer) {
    // Menu Background
    fb.draw_rect(0, 328, 250, 400, 0x001A1A1A);
    // Calculator Entry
    fb.draw_rect(5, 333, 240, 30, 0x00333333);
    draw_string(fb, 15, 344, "Calculator", 0xFFFFFF);

    // Text Editor Entry
    fb.draw_rect(5, 368, 240, 30, 0x00333333);
    draw_string(fb, 15, 379, "Text Editor", 0xFFFFFF);
}

pub fn handle_click(mx: i32, my: i32, wm: &mut WindowManager, start_menu_open: &mut bool) {
    // Check Start Button click
    if mx >= 5 && mx <= 75 && my >= 733 && my <= 763 {
        *start_menu_open = !*start_menu_open;
    } else if *start_menu_open && mx >= 5 && mx <= 245 && my >= 333 && my <= 363 {
        wm.windows.push(Window::new("Calculator", 300, 200, 220, 300, AppType::Calculator));
        *start_menu_open = false;
    } else if *start_menu_open && mx >= 5 && mx <= 245 && my >= 368 && my <= 398 {
        wm.windows.push(Window::new("Text Editor", 350, 250, 400, 300, AppType::TextEditor));
        *start_menu_open = false;
    } else {
        *start_menu_open = false;
    }
}