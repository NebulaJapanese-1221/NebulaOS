const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    }
    value
}

pub fn init() {
    for _ in 0..100_000 {
        if port_byte_in(KEYBOARD_STATUS_PORT) & 0x02 == 0 {
            port_byte_out(KEYBOARD_DATA_PORT, 0xF4);
            return;
        }
        core::hint::spin_loop();
    }
}

pub fn poll_key() -> Option<u8> {
    let status = port_byte_in(KEYBOARD_STATUS_PORT);
    if status & 0x01 == 0 || status & 0x20 != 0 {
        return None;
    }

    let scancode = port_byte_in(KEYBOARD_DATA_PORT);
    if scancode & 0x80 != 0 {
        return None;
    }
    scancode_to_ascii(scancode)
}

pub fn scancode_to_ascii(scancode: u8) -> Option<u8> {
    Some(match scancode {
        0x01 => 27,
        0x02 => b'1',
        0x03 => b'2',
        0x04 => b'3',
        0x05 => b'4',
        0x06 => b'5',
        0x07 => b'6',
        0x08 => b'7',
        0x09 => b'8',
        0x0A => b'9',
        0x0B => b'0',
        0x0E => 8,
        0x0C => b'-',
        0x0D => b'+',
        0x0F => b'\t',
        0x10 => b'q',
        0x11 => b'w',
        0x12 => b'e',
        0x13 => b'r',
        0x14 => b't',
        0x15 => b'y',
        0x16 => b'u',
        0x17 => b'i',
        0x18 => b'o',
        0x19 => b'p',
        0x1C => b'\n',
        0x1A => b'[',
        0x1B => b']',
        0x27 => b';',
        0x28 => b'\'',
        0x29 => b'`',
        0x1E => b'a',
        0x1F => b's',
        0x20 => b'd',
        0x21 => b'f',
        0x22 => b'g',
        0x23 => b'h',
        0x24 => b'j',
        0x25 => b'k',
        0x26 => b'l',
        0x2C => b'z',
        0x2D => b'x',
        0x2E => b'c',
        0x2F => b'v',
        0x30 => b'b',
        0x31 => b'n',
        0x32 => b'm',
        0x33 => b',',
        0x34 => b'.',
        0x35 => b'/',
        0x37 => b'*',
        0x2B => b'\\',
        0x39 => b' ',
        _ => return None,
    })
}
