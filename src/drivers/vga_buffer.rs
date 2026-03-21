#![allow(dead_code)]
use core::fmt;
use spin::Mutex;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    pub column_position: usize,
    color_code: ColorCode,
    buffer: *mut Buffer,
}

unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                unsafe { &mut *self.buffer }.chars[row][col] = ScreenChar {
                    ascii_character: byte,
                    color_code,
                };
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = unsafe { &mut *self.buffer }.chars[row][col];
                unsafe { &mut *self.buffer }.chars[row - 1][col] = character;
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            unsafe { &mut *self.buffer }.chars[row][col] = blank;
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    // --- Direct access methods for GUI ---

    pub fn set_char_at(&mut self, x: usize, y: usize, byte: u8, color: Color) {
        if x < BUFFER_WIDTH && y < BUFFER_HEIGHT {
             unsafe { &mut *self.buffer }.chars[y][x] = ScreenChar {
                 ascii_character: byte,
                 color_code: ColorCode::new(color, Color::Black),
             };
        }
    }
    
    #[allow(dead_code)]
    pub fn read_char_at(&self, x: usize, y: usize) -> (u8, u8) {
        if x < BUFFER_WIDTH && y < BUFFER_HEIGHT {
            let sc = unsafe { &*self.buffer }.chars[y][x];
            (sc.ascii_character, sc.color_code.0)
        } else {
            (b' ', 0)
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: ColorCode::new(Color::White, Color::Black),
    buffer: 0xb8000 as *mut Buffer,
});

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::drivers::vga_buffer::WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap();
        $crate::drivers::vga_buffer::WRITER.lock().write_byte(b'\n');
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::drivers::vga_buffer::WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap();
    });
}