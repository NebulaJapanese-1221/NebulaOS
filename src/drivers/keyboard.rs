use spin::Mutex;
use super::ps2;
use crate::kernel::interrupts::InterruptStackFrame;
use crate::kernel::io;
use core::arch::asm;

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
                ctrl: false,
                alt: false,
                capslock: false,
                last_scancode: 0,
                repeat_count: 0,
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
    pub ctrl: bool,
    pub alt: bool,
    pub capslock: bool,
    pub last_scancode: u8,
    pub repeat_count: u32,
}

/// The global keyboard buffer.
pub static KEY_BUFFER: Mutex<KeyBuffer> = Mutex::new(KeyBuffer::new());

/// Retreives the next char from the buffer, if any.
pub fn get_char() -> Option<char> {
    // Disable interrupts to prevent deadlock with the interrupt handler
    unsafe { asm!("cli", options(nomem, nostack)); }
    let mut kb = KEY_BUFFER.lock(); 
    let c = kb.pop();
    drop(kb); // Release lock before re-enabling interrupts
    unsafe { asm!("sti", options(nomem, nostack)); }
    c
}

/// Updates modifier state based on scancode.
pub fn update_modifiers(scancode: u8) {
    let mut kb = KEY_BUFFER.lock();
    if scancode == kb.modifiers.last_scancode {
        kb.modifiers.repeat_count += 1;
    } else {
        kb.modifiers.last_scancode = scancode;
        kb.modifiers.repeat_count = 0;
    }
    match scancode {
        0x2A => kb.modifiers.lshift = true,   // Left Shift Press
        0xAA => kb.modifiers.lshift = false,  // Left Shift Release
        0x36 => kb.modifiers.rshift = true,   // Right Shift Press
        0xB6 => kb.modifiers.rshift = false,  // Right Shift Release
        0x1D => kb.modifiers.ctrl = true,      // Ctrl Press
        0x9D => kb.modifiers.ctrl = false,     // Ctrl Release
        0x38 => kb.modifiers.alt = true,       // Alt Press
        0xB8 => kb.modifiers.alt = false,      // Alt Release
        0x3A => {                              // Capslock Press (toggle)
            if scancode < 0x80 {
                kb.modifiers.capslock = !kb.modifiers.capslock;
            }
        }
        _ => {}
    }
}

pub fn is_shift_pressed() -> bool {
    let kb = KEY_BUFFER.lock();
    kb.modifiers.lshift || kb.modifiers.rshift
}

pub fn is_capslock_enabled() -> bool {
    let kb = KEY_BUFFER.lock();
    kb.modifiers.capslock
}

pub fn is_alt_pressed() -> bool {
    let kb = KEY_BUFFER.lock();
    kb.modifiers.alt
}

pub fn is_ctrl_pressed() -> bool {
    let kb = KEY_BUFFER.lock();
    kb.modifiers.ctrl
}

static SCANCODE_SET1: [char; 128] = [
    '\0', '\x1B', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\x08', '\t',
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\n', '\0', 'a', 's',
    'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`', '\0', '\\', 'z', 'x', 'c', 'v',
    'b', 'n', 'm', ',', '.', '/', '\0', '*', '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '-', '\0', '\0', '\0', '+', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
];

static SCANCODE_SET1_SHIFTED: [char; 128] = [
    '\0', '\x1B', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '\x08', '\t',
    'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '{', '}', '\n', '\0', 'A', 'S',
    'D', 'F', 'G', 'H', 'J', 'K', 'L', ':', '"', '~', '\0', '|', 'Z', 'X', 'C', 'V',
    'B', 'N', 'M', '<', '>', '?', '\0', '*', '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '-', '\0', '\0', '\0', '+', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
];

/// Converts a PS/2 scancode (Set 1) to a character.
/// Handles a basic QWERTY layout.
pub fn scancode_to_char(scancode: u8) -> Option<char> {
    let shift = is_shift_pressed();
    let capslock = is_capslock_enabled();
    let idx = scancode as usize;

    if idx >= SCANCODE_SET1.len() {
        return None;
    }

    let c = SCANCODE_SET1[idx];
    if c == '\0' {
        return None;
    }

    let is_letter = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
    
    // If it is a letter, Shift XOR CapsLock determines case.
    // If it is NOT a letter (e.g. number or symbol), only Shift determines the alternate char.
    let use_shifted = if is_letter {
        shift ^ capslock
    } else {
        shift
    };

    if use_shifted {
        Some(SCANCODE_SET1_SHIFTED[idx])
    } else {
        Some(c)
    }
}

unsafe fn send_cmd(byte: u8) {
    while (ps2::read_status() & 0x02) != 0 {}
    ps2::write_data(byte);
}

pub fn init() {
    unsafe {
        // 1. Disable PS/2 devices to prevent them from sending data during setup
        ps2::write_command(0xAD); // Disable keyboard
        ps2::write_command(0xA7); // Disable mouse

        // 2. Flush the output buffer to discard any garbage bytes from the BIOS
        while (ps2::read_status() & ps2::STATUS_OUTPUT_BUFFER) != 0 {
            ps2::read_data();
        }

        // 3. Set the controller configuration byte
        ps2::write_command(0x20); // Get Config Byte
        let mut status = ps2::read_data();
        status |= 0x01; // Enable IRQ1 (Keyboard)
        status &= !0x10; // Enable Keyboard Clock (Clear disable bit)
        ps2::write_command(0x60); // Set Config Byte
        ps2::write_data(status);

        // 4. Send commands to the keyboard itself
        // Reset keyboard first to ensure clean state
        send_cmd(0xFF); // Reset
        ps2::read_data(); // Consume ACK

        send_cmd(0xF0); // Set Scan Code Set
        send_cmd(0x02); // Scan Code Set 2
        ps2::read_data(); // Consume ACK

        send_cmd(0xF6); // Set Defaults
        ps2::read_data(); // Consume ACK

        send_cmd(0xF4); // Enable Scanning
        ps2::read_data(); // Consume ACK

        // 5. Enable the devices
        ps2::write_command(0xAE); // Enable keyboard
        ps2::write_command(0xA8); // Enable mouse
    }
}