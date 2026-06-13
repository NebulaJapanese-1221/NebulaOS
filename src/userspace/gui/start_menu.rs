use crate::framebuffer::Framebuffer;
use super::{draw_string, Window, WindowManager, AppType};

pub fn draw(fb: &mut Framebuffer, ty: usize) {
    let menu_h = 400;
    let menu_y = ty - menu_h;
    
    // Menu Background
    fb.draw_rect(0, menu_y, 250, menu_h, 0x001A1A1A);
    
    // Calculator Entry
    fb.draw_rect(5, menu_y + 5, 240, 30, 0x00333333);
    draw_string(fb, 15, menu_y + 16, "Calculator", 0xFFFFFF);

    // Text Editor Entry
    fb.draw_rect(5, menu_y + 40, 240, 30, 0x00333333);
    draw_string(fb, 15, menu_y + 51, "Text Editor", 0xFFFFFF);
}

pub fn handle_click(mx: i32, my: i32, height: i32, wm: &mut WindowManager, start_menu_open: &mut bool) {
    let ty = height - 40; // Taskbar top
    let menu_y = ty - 400;

    // Check Start Button click
    if mx >= 5 && mx <= 75 && my >= ty + 5 && my <= ty + 35 {
        *start_menu_open = !*start_menu_open;
    } else if *start_menu_open && mx >= 5 && mx <= 245 && my >= menu_y + 5 && my <= menu_y + 35 {
        wm.windows.push(Window::new("Calculator", 300, 200, 220, 300, AppType::Calculator));
        *start_menu_open = false;
    } else if *start_menu_open && mx >= 5 && mx <= 245 && my >= menu_y + 40 && my <= menu_y + 70 {
        wm.windows.push(Window::new("Text Editor", 350, 250, 400, 300, AppType::TextEditor));
        *start_menu_open = false;
    } else {
        *start_menu_open = false;
    }
}