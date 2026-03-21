use spin::Mutex;
use super::ps2;
use crate::kernel::interrupts::InterruptStackFrame;
use crate::kernel::io;

/// A simple ring buffer for buffering key presses.
pub struct KeyBuffer {
    keys: [char; 256],
    head: usize,
    tail: usize,
    modifiers: Modifiers,
}

impl KeyBuffer {
    pub const fn new() -> Self {
        Self {
            keys: ['\0'; 256],
            head: 0,
            tail: 0,
            modifiers: Modifiers {
                lshift: false,
                rshift: false,
            },
        }
    }

    pub fn push(&mut self, c: char) {
        if (self.tail + 1) % 256 != self.head {
            self.keys[self.tail] = c;
            self.tail = (self.tail + 1) % 256;
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        if self.head == self.tail {
            None
        } else {
            let c = self.keys[self.head];
            self.head = (self.head + 1) % 256;
            Some(c)
        }
    }
}

/// Handles the keyboard interrupt logic.
pub fn handle_interrupt() {
    let status = unsafe { ps2::read_status() };
    
    // If data is available, we MUST read it to acknowledge the interrupt,
    // even if it's not for us (though that shouldn't happen with correct IRQ routing).
    if (status & ps2::STATUS_OUTPUT_BUFFER) != 0 {
        let scancode = unsafe { ps2::read_data() };

        // Only process if it IS NOT from the mouse (Bit 5 clear)
        if (status & ps2::STATUS_MOUSE_DATA) == 0 {
            update_modifiers(scancode);
            if scancode < 0x80 {
                if let Some(c) = scancode_to_char(scancode) {
                    KEY_BUFFER.lock().push(c);
                }
            }
        }
    }
}

pub extern "x86-interrupt" fn interrupt_handler(_frame: &mut InterruptStackFrame) {
    handle_interrupt();
    unsafe {
        io::outb(0x20, 0x20); // EOI for master PIC
    }
}

pub struct Modifiers {
    pub lshift: bool,
    pub rshift: bool,
}

/// The global keyboard buffer.
pub static KEY_BUFFER: Mutex<KeyBuffer> = Mutex::new(KeyBuffer::new());

/// Retreives the next char from the buffer, if any.
pub fn get_char() -> Option<char> {
    let mut kb = KEY_BUFFER.lock();
    kb.pop()
}

/// Updates modifier state based on scancode.
pub fn update_modifiers(scancode: u8) {
    let mut kb = KEY_BUFFER.lock();
    match scancode {
        0x2A => kb.modifiers.lshift = true,   // Left Shift Press
        0xAA => kb.modifiers.lshift = false,  // Left Shift Release
        0x36 => kb.modifiers.rshift = true,   // Right Shift Press
        0xB6 => kb.modifiers.rshift = false,  // Right Shift Release
        _ => {}
    }
}

pub fn is_shift_pressed() -> bool {
    let kb = KEY_BUFFER.lock();
    kb.modifiers.lshift || kb.modifiers.rshift
}

/// Converts a PS/2 scancode (Set 1) to a character.
/// Handles a basic QWERTY layout.
pub fn scancode_to_char(scancode: u8) -> Option<char> {
    let shift = is_shift_pressed();
    
    if shift {
        match scancode {
            0x02 => Some('!'), 0x03 => Some('@'), 0x04 => Some('#'), 0x05 => Some('$'),
            0x06 => Some('%'), 0x07 => Some('^'), 0x08 => Some('&'), 0x09 => Some('*'),
            0x0A => Some('('), 0x0B => Some(')'), 0x0C => Some('_'), 0x0D => Some('+'),
            0x0E => Some('\x08'), // Backspace
            0x0F => Some('\t'),
            0x10 => Some('Q'), 0x11 => Some('W'), 0x12 => Some('E'), 0x13 => Some('R'),
            0x14 => Some('T'), 0x15 => Some('Y'), 0x16 => Some('U'), 0x17 => Some('I'),
            0x18 => Some('O'), 0x19 => Some('P'), 0x1A => Some('{'), 0x1B => Some('}'),
            0x1C => Some('\n'), // Enter
            0x1E => Some('A'), 0x1F => Some('S'), 0x20 => Some('D'), 0x21 => Some('F'),
            0x22 => Some('G'), 0x23 => Some('H'), 0x24 => Some('J'), 0x25 => Some('K'),
            0x26 => Some('L'), 0x27 => Some(':'), 0x28 => Some('"'), 0x29 => Some('~'),
            0x2B => Some('|'),
            0x2C => Some('Z'), 0x2D => Some('X'), 0x2E => Some('C'), 0x2F => Some('V'),
            0x30 => Some('B'), 0x31 => Some('N'), 0x32 => Some('M'), 0x33 => Some('<'),
            0x34 => Some('>'), 0x35 => Some('?'),
            0x39 => Some(' '),
            _ => None,
        }
    } else {
        match scancode {
            0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'), 0x05 => Some('4'),
            0x06 => Some('5'), 0x07 => Some('6'), 0x08 => Some('7'), 0x09 => Some('8'),
            0x0A => Some('9'), 0x0B => Some('0'), 0x0C => Some('-'), 0x0D => Some('='),
            0x0E => Some('\x08'), // Backspace
            0x0F => Some('\t'),
            0x10 => Some('q'), 0x11 => Some('w'), 0x12 => Some('e'), 0x13 => Some('r'),
            0x14 => Some('t'), 0x15 => Some('y'), 0x16 => Some('u'), 0x17 => Some('i'),
            0x18 => Some('o'), 0x19 => Some('p'), 0x1A => Some('['), 0x1B => Some(']'),
            0x1C => Some('\n'), // Enter
            0x1E => Some('a'), 0x1F => Some('s'), 0x20 => Some('d'), 0x21 => Some('f'),
            0x22 => Some('g'), 0x23 => Some('h'), 0x24 => Some('j'), 0x25 => Some('k'),
            0x26 => Some('l'), 0x27 => Some(';'), 0x28 => Some('\''), 0x29 => Some('`'),
            0x2B => Some('\\'),
            0x2C => Some('z'), 0x2D => Some('x'), 0x2E => Some('c'), 0x2F => Some('v'),
            0x30 => Some('b'), 0x31 => Some('n'), 0x32 => Some('m'), 0x33 => Some(','),
            0x34 => Some('.'), 0x35 => Some('/'),
            0x39 => Some(' '),
            _ => None,
        }
    }
}