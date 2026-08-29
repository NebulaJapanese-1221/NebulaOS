use crate::common::io;
use crate::common::idt;

static mut KEYBOARD_MODIFIERS: u8 = 0;
static mut KEYBOARD_CALLBACK: Option<unsafe extern "C" fn(u8, u8, u8)> = None;
static mut KEY_PRESSED: [bool; 256] = [false; 256];

#[no_mangle]
pub unsafe extern "C" fn keyboard_init() {
    KEYBOARD_MODIFIERS = 0;
    io::outb(0x64, 0xAD);
    io::outb(0x64, 0x20);
    let mode = io::inb(0x60);
    io::outb(0x64, 0x60);
    io::outb(0x60, mode & 0x7F);
    io::outb(0x64, 0xAE);
    io::outb(0x60, 0xFF);
    io::outb(0x60, 0xF4);
    io::outb(0x60, 0x00);
    idt::register_interrupt_handler(33, Some(keyboard_irq_handler));
}

pub unsafe extern "C" fn keyboard_irq_handler(_regs: *mut idt::Registers) {
    let scancode = io::inb(0x60);
    let pressed = !((scancode & 0x80) != 0);
    let code = scancode & 0x7F;
    KEY_PRESSED[code as usize] = pressed;

    match code {
        0x2A | 0x36 => {
            if pressed { KEYBOARD_MODIFIERS |= 0x01; } else { KEYBOARD_MODIFIERS &= !0x01; }
        }
        0x1D => {
            if pressed { KEYBOARD_MODIFIERS |= 0x02; } else { KEYBOARD_MODIFIERS &= !0x02; }
        }
        0x38 => {
            if pressed { KEYBOARD_MODIFIERS |= 0x04; } else { KEYBOARD_MODIFIERS &= !0x04; }
        }
        0x3A => {
            if pressed {
                KEYBOARD_MODIFIERS ^= 0x10;
                io::outb(0x60, 0xED);
                io::outb(0x60, 0x04);
            }
        }
        _ => {}
    }

    if let Some(callback) = KEYBOARD_CALLBACK {
        callback(code, KEYBOARD_MODIFIERS, pressed as u8);
    }

    io::outb(0x20, 0x20);
}

pub unsafe fn keyboard_set_handler(handler: Option<unsafe extern "C" fn(u8, u8, u8)>) {
    KEYBOARD_CALLBACK = handler;
}

pub unsafe fn keyboard_get_modifiers() -> u8 {
    KEYBOARD_MODIFIERS
}

pub unsafe fn keyboard_scancode_to_ascii(scancode: u8, modifiers: u8) -> char {
    let shifted = (modifiers & 0x01) != 0;
    match scancode {
        0x02 => if shifted { '!' } else { '1' },
        0x03 => if shifted { '@' } else { '2' },
        0x04 => if shifted { '#' } else { '3' },
        0x05 => if shifted { '$' } else { '4' },
        0x06 => if shifted { '%' } else { '5' },
        0x07 => if shifted { '^' } else { '6' },
        0x08 => if shifted { '&' } else { '7' },
        0x09 => if shifted { '*' } else { '8' },
        0x0A => if shifted { '(' } else { '9' },
        0x0B => if shifted { ')' } else { '0' },
        0x0C => if shifted { '_' } else { '-' },
        0x0D => if shifted { '+' } else { '=' },
        0x0E => '\x08',
        0x0F => '\t',
        0x10 => if shifted { 'Q' } else { 'q' },
        0x11 => if shifted { 'W' } else { 'w' },
        0x12 => if shifted { 'E' } else { 'e' },
        0x13 => if shifted { 'R' } else { 'r' },
        0x14 => if shifted { 'T' } else { 't' },
        0x15 => if shifted { 'Y' } else { 'y' },
        0x16 => if shifted { 'U' } else { 'u' },
        0x17 => if shifted { 'I' } else { 'i' },
        0x18 => if shifted { 'O' } else { 'o' },
        0x19 => if shifted { 'P' } else { 'p' },
        0x1A => if shifted { '{' } else { '[' },
        0x1B => if shifted { '}' } else { ']' },
        0x1C => '\n',
        0x1E => if shifted { 'A' } else { 'a' },
        0x1F => if shifted { 'S' } else { 's' },
        0x20 => if shifted { 'D' } else { 'd' },
        0x21 => if shifted { 'F' } else { 'f' },
        0x22 => if shifted { 'G' } else { 'g' },
        0x23 => if shifted { 'H' } else { 'h' },
        0x24 => if shifted { 'J' } else { 'j' },
        0x25 => if shifted { 'K' } else { 'k' },
        0x26 => if shifted { 'L' } else { 'l' },
        0x27 => if shifted { ':' } else { ';' },
        0x28 => if shifted { '"' } else { '\'' },
        0x29 => if shifted { '~' } else { '`' },
        0x2B => if shifted { '|' } else { '\\' },
        0x2C => if shifted { 'Z' } else { 'z' },
        0x2D => if shifted { 'X' } else { 'x' },
        0x2E => if shifted { 'C' } else { 'c' },
        0x2F => if shifted { 'V' } else { 'v' },
        0x30 => if shifted { 'B' } else { 'b' },
        0x31 => if shifted { 'N' } else { 'n' },
        0x32 => if shifted { 'M' } else { 'm' },
        0x33 => if shifted { '<' } else { ',' },
        0x34 => if shifted { '>' } else { '.' },
        0x35 => if shifted { '?' } else { '/' },
        0x39 => ' ',
        _ => '\0',
    }
}
