#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0, White = 15, // Adding a few defaults
    Blue = 1, Green = 2, Red = 4,
}

#[allow(dead_code)]
pub struct VgaWriter {
    cursor_x: usize,
    cursor_y: usize,
    color: u8,
    buffer: *mut u8,
}

#[allow(dead_code)]
impl VgaWriter {
    pub const fn new(foreground: Color, background: Color) -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            color: (background as u8) << 4 | (foreground as u8),
            buffer: 0xb8000 as *mut u8,
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        if byte == b'\n' {
            self.cursor_x = 0;
            self.cursor_y += 1;
            return;
        }

        let offset = (self.cursor_y * 80 + self.cursor_x) * 2;
        unsafe {
            *self.buffer.offset(offset as isize) = byte;
            *self.buffer.offset(offset as isize + 1) = self.color;
        }

        self.cursor_x += 1;
        if self.cursor_x >= 80 {
            self.cursor_x = 0;
            self.cursor_y += 1;
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}