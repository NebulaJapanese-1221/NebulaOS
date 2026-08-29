use crate::common::io;

static mut SERIAL_PORT: u16 = 0;
static mut SERIAL_INITIALIZED: bool = false;

#[no_mangle]
pub unsafe extern "C" fn serial_init(port: u16, _baud_rate: u32) {
    SERIAL_PORT = port;
    SERIAL_INITIALIZED = true;
    io::outb(port + 3, 0x80);
    io::outb(port + 0, 0x03);
    io::outb(port + 1, 0x00);
    io::outb(port + 3, 0x03);
    io::outb(port + 2, 0xC7);
    io::outb(port + 4, 0x0B);
}

pub unsafe fn serial_putchar(c: char) {
    if !SERIAL_INITIALIZED {
        return;
    }
    while (io::inb(SERIAL_PORT + 5) & 0x20) == 0 {}
    io::outb(SERIAL_PORT, c as u8);
}

pub unsafe fn serial_puts(s: *const u8) {
    let mut i = 0;
    while *s.add(i) != 0 {
        serial_putchar(*s.add(i) as char);
        i += 1;
    }
}
