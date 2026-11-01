// Keyboard driver

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

pub fn init() {
    port_byte_out(KEYBOARD_STATUS_PORT, 0xF4);
}

pub fn key_available() -> bool {
    unsafe { port_byte_in(KEYBOARD_STATUS_PORT) & 0x01 != 0 }
}

pub fn read_scancode() -> u8 {
    unsafe { port_byte_in(KEYBOARD_DATA_PORT) }
}

pub fn scancode_to_ascii(scancode: u8) -> Option<u8> {
    match scancode {
        0x02 => Some(b'1'), 0x03 => Some(b'2'), 0x04 => Some(b'3'), 0x05 => Some(b'4'),
        0x06 => Some(b'5'), 0x07 => Some(b'6'), 0x08 => Some(b'7'), 0x09 => Some(b'8'),
        0x0A => Some(b'9'), 0x0B => Some(b'0'), 0x0C => Some(b'-'), 0x0D => Some(b'='),
        0x0E => Some(8),   // Backspace
        0x0F => Some(b'\t'), // Tab
        0x10 => Some(b'q'), 0x11 => Some(b'w'), 0x12 => Some(b'e'), 0x13 => Some(b'r'),
        0x14 => Some(b't'), 0x15 => Some(b'y'), 0x16 => Some(b'u'), 0x17 => Some(b'i'),
        0x18 => Some(b'o'), 0x19 => Some(b'p'), 0x1A => Some(b'['), 0x1B => Some(b']'),
        0x1C => Some(b'\n'), // Enter
        0x1E => Some(b'a'), 0x1F => Some(b's'), 0x20 => Some(b'd'), 0x21 => Some(b'f'),
        0x22 => Some(b'g'), 0x23 => Some(b'h'), 0x24 => Some(b'j'), 0x25 => Some(b'k'),
        0x26 => Some(b'l'), 0x27 => Some(b';'), 0x28 => Some(b'\''), 0x29 => Some(b'`'),
        0x2A => Some(0), // Left shift
        0x2B => Some(b'\\'), 0x2C => Some(b'z'), 0x2D => Some(b'x'), 0x2E => Some(b'c'),
        0x2F => Some(b'v'), 0x30 => Some(b'b'), 0x31 => Some(b'n'), 0x32 => Some(b'm'),
        0x33 => Some(b','), 0x34 => Some(b'.'), 0x35 => Some(b'/'), 0x36 => Some(0), // Right shift
        0x37 => Some(b'*'), 0x38 => Some(0), // Left Alt
        0x39 => Some(b' '),     // Space
        _ => None,
    }
}

pub fn getchar() -> u8 {
    loop {
        if key_available() {
            let scancode = read_scancode();
            if scancode & 0x80 == 0 {
                if let Some(ascii) = scancode_to_ascii(scancode) {
                    return ascii;
                }
            }
        }
    }
}

pub fn poll() {
    unsafe {
        let status = port_byte_in(0x64);
        if status & 0x01 != 0 {
            let _byte = port_byte_in(0x60);
        }
    }
}
