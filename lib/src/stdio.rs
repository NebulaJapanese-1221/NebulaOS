use crate::common::vga;

pub unsafe fn putchar(c: i32) -> i32 {
    if c == '\n' as i32 {
        vga::putchar('\n');
    } else if c == '\r' as i32 {
        vga::putchar('\r');
    } else {
        vga::putchar(c as u8 as u8 as char);
    }
    c
}

pub unsafe fn puts(s: *const u8) -> i32 {
    vga::puts(s);
    0
}

pub unsafe fn printf(format: *const u8, _args: ...) -> i32 {
    vga::puts(format);
    0
}

pub unsafe fn sprintf(_buffer: *mut u8, _format: *const u8, _args: ...) -> i32 {
    0
}

pub unsafe fn snprintf(buffer: *mut u8, size: usize, format: *const u8, _args: ...) -> i32 {
    let len = crate::lib::string::strlen(format);
    if len >= size {
        size as i32
    } else {
        crate::lib::string::strcpy(buffer, format);
        len as i32
    }
}
