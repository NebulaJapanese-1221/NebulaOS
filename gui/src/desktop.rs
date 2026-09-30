static mut FRAMEBUFFER: *mut u8 = core::ptr::null_mut();
static mut WIDTH: u32 = 0;
static mut HEIGHT: u32 = 0;
static mut STRIDE: u32 = 0;
static mut BYTES_PER_PIXEL: u32 = 0;
static mut RED_SIZE: u32 = 0;
static mut RED_POSITION: u32 = 0;
static mut GREEN_SIZE: u32 = 0;
static mut GREEN_POSITION: u32 = 0;
static mut BLUE_SIZE: u32 = 0;
static mut BLUE_POSITION: u32 = 0;
static mut WINDOW_X: u32 = 0;
static mut WINDOW_Y: u32 = 0;

const FONT: [[u8; 7]; 26] = [
    [14, 17, 17, 31, 17, 17, 17],
    [30, 17, 17, 30, 17, 17, 30],
    [14, 17, 16, 16, 16, 17, 14],
    [30, 17, 17, 17, 17, 17, 30],
    [31, 16, 16, 30, 16, 16, 31],
    [31, 16, 16, 30, 16, 16, 16],
    [14, 17, 16, 23, 17, 17, 15],
    [17, 17, 17, 31, 17, 17, 17],
    [14, 4, 4, 4, 4, 4, 14],
    [7, 2, 2, 2, 18, 18, 12],
    [17, 18, 20, 24, 20, 18, 17],
    [16, 16, 16, 16, 16, 16, 31],
    [17, 27, 21, 21, 17, 17, 17],
    [17, 25, 21, 19, 17, 17, 17],
    [14, 17, 17, 17, 17, 17, 14],
    [30, 17, 17, 30, 16, 16, 16],
    [14, 17, 17, 17, 21, 18, 13],
    [30, 17, 17, 30, 20, 18, 17],
    [15, 16, 16, 14, 1, 1, 30],
    [31, 4, 4, 4, 4, 4, 4],
    [17, 17, 17, 17, 17, 17, 14],
    [17, 17, 17, 17, 17, 10, 4],
    [17, 17, 17, 21, 21, 21, 10],
    [17, 17, 10, 4, 10, 17, 17],
    [17, 17, 10, 4, 4, 4, 4],
    [31, 1, 2, 4, 8, 16, 31],
];

fn color_channel(value: u32, size: u32, position: u32) -> u32 {
    let size = size.min(8);
    if size == 0 || position >= 32 {
        return 0;
    }
    let maximum = (1u32 << size) - 1;
    ((value * maximum + 127) / 255) << position
}

unsafe fn pack_color(color: u32) -> u32 {
    color_channel((color >> 16) & 0xFF, RED_SIZE, RED_POSITION)
        | color_channel((color >> 8) & 0xFF, GREEN_SIZE, GREEN_POSITION)
        | color_channel(color & 0xFF, BLUE_SIZE, BLUE_POSITION)
}

unsafe fn put_pixel(x: u32, y: u32, pixel: u32) {
    if x >= WIDTH || y >= HEIGHT {
        return;
    }
    let offset = (y * STRIDE + x * BYTES_PER_PIXEL) as usize;
    for byte in 0..BYTES_PER_PIXEL {
        FRAMEBUFFER.add(offset + byte as usize)
            .write_volatile((pixel >> (byte * 8)) as u8);
    }
}

unsafe fn fill_rect(x: u32, y: u32, width: u32, height: u32, color: u32) {
    let right = x.saturating_add(width).min(WIDTH);
    let bottom = y.saturating_add(height).min(HEIGHT);
    if x >= right || y >= bottom {
        return;
    }
    let pixel = pack_color(color);
    for row in y..bottom {
        for column in x..right {
            put_pixel(column, row, pixel);
        }
    }
}

fn glyph(character: u8) -> [u8; 7] {
    match character {
        b'A'..=b'Z' => FONT[(character - b'A') as usize],
        b':' => [0, 4, 4, 0, 4, 4, 0],
        b'-' => [0, 0, 0, 31, 0, 0, 0],
        _ => [0; 7],
    }
}

unsafe fn draw_text(x: u32, y: u32, text: &[u8], color: u32, scale: u32) {
    let pixel = pack_color(color);
    let mut cursor = x;
    for character in text {
        let rows = glyph(*character);
        for row in 0..7 {
            for column in 0..5 {
                if rows[row] & (1 << (4 - column)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            put_pixel(
                                cursor + column as u32 * scale + dx,
                                y + row as u32 * scale + dy,
                                pixel,
                            );
                        }
                    }
                }
            }
        }
        cursor = cursor.saturating_add(6 * scale);
    }
}

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
    let bytes_per_pixel = bits_per_pixel / 8;
    if framebuffer.is_null()
        || width == 0
        || height == 0
        || (bits_per_pixel != 24 && bits_per_pixel != 32)
        || stride < width.saturating_mul(bytes_per_pixel)
    {
        return;
    }

    FRAMEBUFFER = framebuffer;
    WIDTH = width;
    HEIGHT = height;
    STRIDE = stride;
    BYTES_PER_PIXEL = bytes_per_pixel;
    RED_SIZE = red_size;
    RED_POSITION = red_position;
    GREEN_SIZE = green_size;
    GREEN_POSITION = green_position;
    BLUE_SIZE = blue_size;
    BLUE_POSITION = blue_position;
    WINDOW_X = WIDTH.saturating_sub(WIDTH.saturating_sub(210).min(720)) / 2;
    WINDOW_Y = HEIGHT.saturating_sub(HEIGHT.saturating_sub(150).min(470)) / 2;
    draw_desktop(0);
}

unsafe fn draw_icon(x: u32, y: u32, label: &[u8], color: u32) {
    fill_rect(x, y, 48, 48, 0xE9F4F2);
    fill_rect(x + 4, y + 4, 40, 40, color);
    fill_rect(x + 12, y + 12, 24, 3, 0xFFFFFF);
    fill_rect(x + 12, y + 20, 18, 3, 0xFFFFFF);
    draw_text(x - 4, y + 56, label, 0xFFFFFF, 1);
}

pub unsafe fn draw_desktop(active_window: u8) {
    if FRAMEBUFFER.is_null() {
        return;
    }
    let width = WIDTH;
    let height = HEIGHT;
    for row in 0..height {
        let blend = row * 38 / height.max(1);
        fill_rect(0, row, width, 1, 0x102B42 + (blend << 8) + (blend << 16));
    }

    fill_rect(0, 0, width, 36, 0x10202C);
    fill_rect(18, 10, 16, 16, 0x45D6C5);
    fill_rect(22, 6, 8, 24, 0x45D6C5);
    draw_text(44, 13, b"NEBULA OS", 0xF4FAF9, 1);
    draw_text(width.saturating_sub(108), 13, b"GRAPHICS", 0x9AB8C1, 1);

    draw_icon(38, 66, b"SYSTEM", 0x28A99B);
    draw_icon(38, 158, b"TERMINAL", 0xD88B4A);

    let window_width = width.saturating_sub(210).min(720);
    let window_height = height.saturating_sub(150).min(470);
    let window_x = WINDOW_X.min(width.saturating_sub(window_width));
    let window_y = WINDOW_Y.min(height.saturating_sub(window_height));
    fill_rect(window_x + 5, window_y + 7, window_width, window_height, 0x07151F);
    fill_rect(window_x, window_y, window_width, window_height, 0xE8F0F1);
    fill_rect(window_x, window_y, window_width, 38, 0x173745);
    fill_rect(window_x + window_width - 36, window_y + 12, 9, 9, 0xD86C61);
    let title = match active_window {
        1 => b"NEBULA OS TERMINAL" as &[u8],
        2 => b"SYSTEM INFORMATION" as &[u8],
        _ => b"DESKTOP" as &[u8],
    };
    draw_text(window_x + 18, window_y + 15, title, 0xFFFFFF, 1);

    let sidebar_width = 154.min(window_width / 3);
    fill_rect(window_x, window_y + 38, sidebar_width, window_height - 38, 0xDCE7E8);
    fill_rect(window_x + 14, window_y + 60, sidebar_width - 28, 30, 0xBBD5D4);
    fill_rect(window_x + 25, window_y + 71, 8, 8, 0x168C82);
    draw_text(window_x + 42, window_y + 71, b"DESKTOP", 0x23434B, 1);
    fill_rect(window_x + 25, window_y + 112, 8, 8, 0xD88B4A);
    draw_text(window_x + 42, window_y + 112, b"APPS", 0x52676D, 1);

    let content_x = window_x + sidebar_width + 32;
    let content_y = window_y + 76;
    match active_window {
        1 => {
            fill_rect(content_x, content_y + 48, window_width - sidebar_width - 58, 1, 0xC1D0D1);
            draw_text(content_x, content_y, b"NEBULA OS TERMINAL", 0x173745, 2);
            draw_text(content_x, content_y + 72, b"COMMAND CONSOLE READY", 0x405A60, 1);
            fill_rect(content_x, content_y + 105, 12, 18, 0x168C82);
            draw_text(content_x + 22, content_y + 109, b"PRESS ESC TO RETURN", 0x405A60, 1);
        }
        2 => {
            draw_text(content_x, content_y, b"SYSTEM INFORMATION", 0x173745, 2);
            fill_rect(content_x, content_y + 45, window_width - sidebar_width - 58, 1, 0xC1D0D1);
            draw_text(content_x, content_y + 78, b"NEBULA OS IS RUNNING", 0x405A60, 1);
            draw_text(content_x, content_y + 110, b"X86-64 GRAPHICS MODE", 0x405A60, 1);
            draw_text(content_x, content_y + 142, b"READY", 0x168C82, 1);
        }
        _ => {
            draw_text(content_x, content_y, b"WELCOME TO NEBULA OS", 0x173745, 2);
            fill_rect(content_x, content_y + 45, window_width - sidebar_width - 58, 1, 0xC1D0D1);
            draw_text(content_x, content_y + 72, b"THE DESKTOP IS READY", 0x405A60, 1);
            draw_text(content_x, content_y + 112, b"T OPEN TERMINAL", 0x168C82, 1);
            draw_text(content_x, content_y + 142, b"S SYSTEM INFORMATION", 0x168C82, 1);
            draw_text(content_x, content_y + 172, b"ESC CLOSE WINDOW", 0x168C82, 1);
        }
    }

    fill_rect(0, height - 42, width, 42, 0x10202C);
    fill_rect(16, height - 34, 26, 26, 0x45D6C5);
    fill_rect(22, height - 39, 14, 36, 0x45D6C5);
    fill_rect(56, height - 31, 150, 20, 0x24424C);
    draw_text(68, height - 25, b"NEBULA OS", 0xFFFFFF, 1);
    draw_text(width.saturating_sub(82), height - 25, b"READY", 0x9AB8C1, 1);
}

pub unsafe fn dimensions() -> (u32, u32) {
    (WIDTH, HEIGHT)
}

pub unsafe fn icon_at(x: u32, y: u32) -> Option<u8> {
    if x < 100 && (66..132).contains(&y) {
        Some(2)
    } else if x < 120 && (158..224).contains(&y) {
        Some(1)
    } else {
        None
    }
}

pub unsafe fn close_button_at(x: u32, y: u32) -> bool {
    let width = WIDTH.saturating_sub(210).min(720);
    let height = HEIGHT.saturating_sub(150).min(470);
    let window_x = WINDOW_X.min(WIDTH.saturating_sub(width));
    let window_y = WINDOW_Y.min(HEIGHT.saturating_sub(height));
    x >= window_x + width.saturating_sub(45)
        && x < window_x + width
        && y >= window_y
        && y < window_y + 38
}

pub unsafe fn title_bar_at(x: u32, y: u32) -> bool {
    let width = WIDTH.saturating_sub(210).min(720);
    let height = HEIGHT.saturating_sub(150).min(470);
    let window_x = WINDOW_X.min(WIDTH.saturating_sub(width));
    let window_y = WINDOW_Y.min(HEIGHT.saturating_sub(height));
    x >= window_x && x < window_x + width && y >= window_y && y < window_y + 38
}

pub unsafe fn move_window(dx: i16, dy: i16) {
    let width = WIDTH.saturating_sub(210).min(720);
    let height = HEIGHT.saturating_sub(150).min(470);
    WINDOW_X = (WINDOW_X as i32 + dx as i32).clamp(0, WIDTH.saturating_sub(width) as i32) as u32;
    WINDOW_Y = (WINDOW_Y as i32 - dy as i32).clamp(0, HEIGHT.saturating_sub(height) as i32) as u32;
}

pub unsafe fn draw_cursor(x: u32, y: u32) {
    fill_rect(x, y, 3, 19, 0xFFFFFF);
    fill_rect(x, y + 16, 8, 3, 0xFFFFFF);
    fill_rect(x + 6, y + 19, 4, 3, 0xFFFFFF);
    fill_rect(x + 4, y + 12, 3, 5, 0x18272B);
}