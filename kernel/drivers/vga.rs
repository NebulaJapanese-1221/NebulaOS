// VGA text mode driver

const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const VGA_BUFFER: *mut u16 = 0xb8000 as *mut u16;

pub fn init() {
    clear_screen();
}

pub fn clear_screen() {
    unsafe {
        for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
            VGA_BUFFER.add(i).write_volatile(0x0720);
        }
    }
}

pub fn write_char(ch: u8, x: usize, y: usize, color: u8) {
    if x < VGA_WIDTH && y < VGA_HEIGHT {
        unsafe {
            VGA_BUFFER.add(y * VGA_WIDTH + x).write_volatile((color as u16) << 8 | ch as u16);
        }
    }
}

pub fn print(s: &str) {
    let vga_buffer = VGA_BUFFER;
    let mut x: usize = 0;
    let mut y: usize = 0;

    for byte in s.bytes() {
        if byte == b'\n' {
            x = 0;
            y += 1;
        } else if byte == b'\r' {
            x = 0;
        } else {
            unsafe {
                vga_buffer.add(y * VGA_WIDTH + x).write_volatile((0x07 << 8) | byte as u16);
            }
            x += 1;
        }
        if x >= VGA_WIDTH {
            x = 0;
            y += 1;
        }
        if y >= VGA_HEIGHT {
            for i in 0..((VGA_HEIGHT - 1) * VGA_WIDTH) {
                unsafe {
                    vga_buffer.add(i).write_volatile(vga_buffer.add(i + VGA_WIDTH).read_volatile());
                }
            }
            for i in ((VGA_HEIGHT - 1) * VGA_WIDTH)..(VGA_HEIGHT * VGA_WIDTH) {
                unsafe {
                    vga_buffer.add(i).write_volatile(0x0720);
                }
            }
            y = VGA_HEIGHT - 1;
        }
    }
}

pub fn draw_rect(x: u16, y: u16, w: u16, h: u16, color: u32) {
    // Text mode - no graphics
    let _ = (x, y, w, h, color);
}

pub fn fill_rect(x: u16, y: u16, w: u16, h: u16, color: u32) {
    // Text mode - no graphics
    let _ = (x, y, w, h, color);
}

pub fn get_width() -> u16 {
    VGA_WIDTH as u16
}

pub fn get_height() -> u16 {
    VGA_HEIGHT as u16
}
