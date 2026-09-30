pub unsafe fn init(
    framebuffer: *mut u8,
    width: u32,
    height: u32,
    stride: u32,
    bits_per_pixel: u32,
    red_size: u32,
    red_position: u32,
    green_size: u32,
    green_position: u32,
    blue_size: u32,
    blue_position: u32,
) {
    crate::desktop::init(
        framebuffer,
        width,
        height,
        stride,
        bits_per_pixel,
        red_size,
        red_position,
        green_size,
        green_position,
        blue_size,
        blue_position,
    );
}

pub unsafe fn run() -> ! {
    let mut window = 0;
    let (width, height) = crate::desktop::dimensions();
    let mut cursor_x = width / 2;
    let mut cursor_y = height / 2;
    let mut previous_buttons = 0;
    let mut dragging_window = false;
    crate::desktop::draw_cursor(cursor_x, cursor_y);
    core::arch::asm!("sti");
    loop {
        let event = match common::input::pop() {
            Some(event) => event,
            None => {
                core::arch::asm!("hlt", options(nomem, nostack));
                continue;
            }
        };
        let redraw = match event {
            common::input::InputEvent::KeyDown { scancode, .. } => match scancode {
                0x14 => {
                    window = if window == 1 { 0 } else { 1 };
                    true
                }
                0x1F => {
                    window = if window == 2 { 0 } else { 2 };
                    true
                }
                0x01 => {
                    window = 0;
                    true
                }
                _ => false,
            },
            common::input::InputEvent::MouseMotion { dx, dy, buttons } => {
                cursor_x = (cursor_x as i32 + dx as i32).clamp(0, width.saturating_sub(1) as i32) as u32;
                cursor_y = (cursor_y as i32 - dy as i32).clamp(0, height.saturating_sub(1) as i32) as u32;
                let left_pressed = buttons & 1 != 0;
                let left_clicked = left_pressed && previous_buttons & 1 == 0;
                if left_clicked {
                    if let Some(icon_window) = crate::desktop::icon_at(cursor_x, cursor_y) {
                        window = icon_window;
                        dragging_window = false;
                    } else if crate::desktop::close_button_at(cursor_x, cursor_y) {
                        window = 0;
                        dragging_window = false;
                    } else {
                        dragging_window = crate::desktop::title_bar_at(cursor_x, cursor_y);
                    }
                } else if left_pressed && dragging_window {
                    crate::desktop::move_window(dx, dy);
                } else if !left_pressed {
                    dragging_window = false;
                }
                previous_buttons = buttons;
                true
            }
            common::input::InputEvent::Empty => false,
        };
        if redraw {
            crate::desktop::draw_desktop(window);
            crate::desktop::draw_cursor(cursor_x, cursor_y);
        }
    }
}
