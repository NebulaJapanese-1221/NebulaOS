use crate::io;

pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;
pub const VGA_MEMORY: usize = 0xB8000;

pub const VGA_COLOR_BLACK: u8 = 0;
pub const VGA_COLOR_BLUE: u8 = 1;
pub const VGA_COLOR_GREEN: u8 = 2;
pub const VGA_COLOR_CYAN: u8 = 3;
pub const VGA_COLOR_RED: u8 = 4;
pub const VGA_COLOR_MAGENTA: u8 = 5;
pub const VGA_COLOR_BROWN: u8 = 6;
pub const VGA_COLOR_LIGHT_GRAY: u8 = 7;
pub const VGA_COLOR_DARK_GRAY: u8 = 8;
pub const VGA_COLOR_LIGHT_BLUE: u8 = 9;
pub const VGA_COLOR_LIGHT_GREEN: u8 = 10;
pub const VGA_COLOR_LIGHT_CYAN: u8 = 11;
pub const VGA_COLOR_LIGHT_RED: u8 = 12;
pub const VGA_COLOR_LIGHT_MAGENTA: u8 = 13;
pub const VGA_COLOR_LIGHT_BROWN: u8 = 14;
pub const VGA_COLOR_YELLOW: u8 = 14;
pub const VGA_COLOR_WHITE: u8 = 15;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VgaChar {
    pub character: u8,
    pub color: u8,
}

static mut VGA_MEM: *mut VgaChar = 0xB8000 as *mut VgaChar;
static mut CURSOR_X: u8 = 0;
static mut CURSOR_Y: u8 = 0;
static mut CURRENT_COLOR: u8 = (VGA_COLOR_WHITE | (VGA_COLOR_BLACK << 4)) as u8;

pub unsafe fn init() {
    clear();
    set_cursor(0, 0);
    enable_cursor(0x0E, 0x0F);
}

pub unsafe fn clear() {
    for y in 0..VGA_HEIGHT {
        for x in 0..VGA_WIDTH {
            set_char(x as u8, y as u8, b' ', CURRENT_COLOR);
        }
    }
    set_cursor(0, 0);
}

pub unsafe fn set_char(x: u8, y: u8, c: u8, color: u8) {
    if (x as usize) < VGA_WIDTH && (y as usize) < VGA_HEIGHT {
        let idx = (y as usize) * VGA_WIDTH + (x as usize);
        (*VGA_MEM.add(idx)).character = c;
        (*VGA_MEM.add(idx)).color = color;
    }
}

pub unsafe fn scroll_up() {
    for y in 1..VGA_HEIGHT {
        for x in 0..VGA_WIDTH {
            let src = VGA_MEM.add(y * VGA_WIDTH + x);
            let dst = VGA_MEM.add((y - 1) * VGA_WIDTH + x);
            *dst = *src;
        }
    }
    for x in 0..VGA_WIDTH {
        set_char(x as u8, (VGA_HEIGHT - 1) as u8, b' ', CURRENT_COLOR);
    }
    if CURSOR_Y > 0 {
        CURSOR_Y -= 1;
    }
    set_cursor(CURSOR_X, CURSOR_Y);
}

pub unsafe fn set_cursor(x: u8, y: u8) {
    CURSOR_X = x;
    CURSOR_Y = y;
    let pos = (y as u16) * VGA_WIDTH as u16 + (x as u16);
    io::outb(0x3D4, 0x0F);
    io::outb(0x3D5, (pos & 0xFF) as u8);
    io::outb(0x3D4, 0x0E);
    io::outb(0x3D5, ((pos >> 8) & 0xFF) as u8);
}

pub unsafe fn putchar(c: char) {
    match c {
        '\n' => {
            CURSOR_X = 0;
            CURSOR_Y += 1;
            if CURSOR_Y >= VGA_HEIGHT as u8 {
                scroll_up();
                CURSOR_Y = (VGA_HEIGHT - 1) as u8;
            }
            set_cursor(CURSOR_X, CURSOR_Y);
        }
        '\r' => {
            CURSOR_X = 0;
            set_cursor(CURSOR_X, CURSOR_Y);
        }
        '\t' => {
            CURSOR_X = (CURSOR_X + 4) & !3;
            if CURSOR_X >= VGA_WIDTH as u8 {
                CURSOR_X = 0;
                CURSOR_Y += 1;
                if CURSOR_Y >= VGA_HEIGHT as u8 {
                    scroll_up();
                    CURSOR_Y = (VGA_HEIGHT - 1) as u8;
                }
            }
            set_cursor(CURSOR_X, CURSOR_Y);
        }
        '\x08' => {
            if CURSOR_X > 0 {
                CURSOR_X -= 1;
            } else if CURSOR_Y > 0 {
                CURSOR_Y -= 1;
                CURSOR_X = (VGA_WIDTH - 1) as u8;
            }
            set_char(CURSOR_X, CURSOR_Y, b' ', CURRENT_COLOR);
            set_cursor(CURSOR_X, CURSOR_Y);
        }
        _ => {
            set_char(CURSOR_X, CURSOR_Y, c as u8, CURRENT_COLOR);
            CURSOR_X += 1;
            if CURSOR_X >= VGA_WIDTH as u8 {
                CURSOR_X = 0;
                CURSOR_Y += 1;
                if CURSOR_Y >= VGA_HEIGHT as u8 {
                    scroll_up();
                    CURSOR_Y = (VGA_HEIGHT - 1) as u8;
                }
            }
            set_cursor(CURSOR_X, CURSOR_Y);
        }
    }
}

pub unsafe fn puts(s: *const u8) {
    let mut i = 0;
    while *s.add(i) != 0 {
        putchar(*s.add(i) as char);
        i += 1;
    }
}

pub unsafe fn set_color(color: u8) {
    CURRENT_COLOR = (CURRENT_COLOR & 0xF0) | (color & 0x0F);
}

pub unsafe fn set_bg_color(bg: u8) {
    CURRENT_COLOR = (CURRENT_COLOR & 0x0F) | ((bg & 0x0F) << 4);
}

pub unsafe fn enable_cursor(start: u8, end: u8) {
    io::outb(0x3D4, 0x0A);
    io::outb(0x3D5, (io::inb(0x3D5) & 0xC0) | start);
    io::outb(0x3D4, 0x0B);
    io::outb(0x3D5, (io::inb(0x3D5) & 0xE0) | end);
}

pub unsafe fn disable_cursor() {
    io::outb(0x3D4, 0x0A);
    io::outb(0x3D5, 0x20);
}

pub unsafe fn putc(x: usize, y: usize, c: u8) {
    if x < VGA_WIDTH && y < VGA_HEIGHT {
        let idx = y * VGA_WIDTH + x;
        (*VGA_MEM.add(idx)).character = c;
        (*VGA_MEM.add(idx)).color = CURRENT_COLOR;
    }
}

pub unsafe fn puts_at(x: usize, y: usize, s: *const u8) {
    let mut i = 0;
    let mut cx = x;
    let mut cy = y;
    while *s.add(i) != 0 {
        let c = *s.add(i);
        if c == b'\n' {
            cx = x;
            cy += 1;
        } else {
            putc(cx, cy, c);
            cx += 1;
            if cx >= VGA_WIDTH {
                cx = x;
                cy += 1;
            }
        }
        i += 1;
        if cy >= VGA_HEIGHT {
            break;
        }
    }
}
