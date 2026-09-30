#![no_std]

use core::arch::asm;

unsafe fn serial_out(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

unsafe fn serial_in(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    value
}

fn serial_write(message: &[u8]) {
    unsafe {
        serial_out(0x3F9, 0x00);
        serial_out(0x3FB, 0x80);
        serial_out(0x3F8, 0x03);
        serial_out(0x3F9, 0x00);
        serial_out(0x3FB, 0x03);
        serial_out(0x3FA, 0xC7);
        serial_out(0x3FC, 0x0B);

        for byte in message {
            while serial_in(0x3FD) & 0x20 == 0 {}
            serial_out(0x3F8, *byte);
        }
    }
}

const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const VGA_BUFFER: *mut u16 = 0xB8000 as *mut u16;

unsafe fn fill_row(row: usize, attribute: u8) {
    for column in 0..VGA_WIDTH {
        VGA_BUFFER.add(row * VGA_WIDTH + column)
            .write_volatile(((attribute as u16) << 8) | b' ' as u16);
    }
}

unsafe fn write_text(row: usize, column: usize, text: &[u8], attribute: u8) {
    for (offset, byte) in text.iter().enumerate() {
        let x = column + offset;
        if x >= VGA_WIDTH || row >= VGA_HEIGHT {
            break;
        }
        VGA_BUFFER.add(row * VGA_WIDTH + x)
            .write_volatile(((attribute as u16) << 8) | *byte as u16);
    }
}

unsafe fn draw_desktop(window: u8) {
    for row in 0..VGA_HEIGHT {
        fill_row(row, 0x1F);
    }

    fill_row(0, 0x70);
    write_text(0, 2, b"NEBULA OS", 0x70);
    write_text(0, 16, b"Desktop", 0x70);
    write_text(0, 65, b"x86 BIOS", 0x70);

    write_text(3, 3, b"[T] Terminal", 0x1F);
    write_text(5, 3, b"[S] System", 0x1F);
    write_text(7, 3, b"[?] Help", 0x1F);

    for row in 4..19 {
        for column in 17..65 {
            VGA_BUFFER.add(row * VGA_WIDTH + column)
                .write_volatile(((0x70u16) << 8) | b' ' as u16);
        }
    }
    fill_row(4, 0x17);
    write_text(4, 19, b"NebulaOS", 0x17);

    match window {
        1 => {
            write_text(6, 19, b"Terminal", 0x70);
            write_text(8, 19, b"NebulaOS command console", 0x70);
            write_text(10, 19, b"The kernel is running in 32-bit mode.", 0x70);
            write_text(12, 19, b"Type T to return to the desktop.", 0x70);
            write_text(14, 19, b"nebula> _", 0x70);
        }
        2 => {
            write_text(6, 19, b"System", 0x70);
            write_text(8, 19, b"NebulaOS x86", 0x70);
            write_text(10, 19, b"Booted with NebulaBoot BIOS loader.", 0x70);
            write_text(12, 19, b"Memory and device services are not yet", 0x70);
            write_text(13, 19, b"available in this x86 desktop build.", 0x70);
            write_text(15, 19, b"Press Esc to close this window.", 0x70);
        }
        _ => {
            write_text(7, 19, b"Welcome to NebulaOS", 0x70);
            write_text(9, 19, b"The desktop is ready.", 0x70);
            write_text(11, 19, b"T  Open terminal", 0x70);
            write_text(12, 19, b"S  System information", 0x70);
            write_text(14, 19, b"Esc  Close a window", 0x70);
        }
    }

    fill_row(24, 0x70);
    write_text(24, 2, b"Nebula", 0x70);
    write_text(24, 13, b"T: Terminal   S: System", 0x70);
    write_text(24, 66, b"Ready", 0x70);
}

unsafe fn read_key() -> Option<u8> {
    let status: u8;
    asm!("in al, dx", in("dx") 0x64u16, out("al") status, options(nomem, nostack, preserves_flags));
    if status & 1 == 0 {
        return None;
    }

    let scan_code: u8;
    asm!("in al, dx", in("dx") 0x60u16, out("al") scan_code, options(nomem, nostack, preserves_flags));
    if scan_code & 0x80 == 0 {
        Some(scan_code)
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    serial_write(b"NebulaOS x86 kernel booted\r\n");
    unsafe {
        let mut window = 0;
        draw_desktop(window);
        loop {
            match read_key() {
                Some(0x14) => window = if window == 1 { 0 } else { 1 },
                Some(0x1F) => window = if window == 2 { 0 } else { 2 },
                Some(0x01) => window = 0,
                _ => continue,
            }
            draw_desktop(window);
        }
    }
}
