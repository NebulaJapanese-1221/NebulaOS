use super::ps2;
use crate::kernel::interrupts::InterruptStackFrame;
use crate::kernel::io;
use crate::kernel::process::IrqSafeMutex; // Use IrqSafeMutex

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
                win: false,
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
    pub win: bool,
    pub last_scancode: u8,
    pub repeat_count: u32,
}

/// The global keyboard buffer.
pub static KEY_BUFFER: IrqSafeMutex<KeyBuffer> = IrqSafeMutex::new(KeyBuffer::new());

/// Retreives the next char from the buffer, if any.
pub fn get_char() -> Option<char> {
    let mut kb = KEY_BUFFER.lock(); 
    let c = kb.pop();
    drop(kb); // Release lock before re-enabling interrupts
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
        0x5B => kb.modifiers.win = true,       // Left Win Press
        0xDB => kb.modifiers.win = false,      // Left Win Release
        0x5C => kb.modifiers.win = true,       // Right Win Press
        0xDC => kb.modifiers.win = false,      // Right Win Release
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

pub fn is_win_pressed() -> bool {
    let kb = KEY_BUFFER.lock();
    kb.modifiers.win
}

#[derive(Clone, Copy)]
struct ScancodeEntry {
    normal: char,
    shifted: char,
    is_alpha: bool,
}

impl ScancodeEntry {
    const fn new(normal: char, shifted: char, is_alpha: bool) -> Self {
        Self { normal, shifted, is_alpha }
    }
}

static SCANCODE_TABLE: [ScancodeEntry; 128] = [
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\x1B', '\x1B', false), ScancodeEntry::new('1', '!', false), ScancodeEntry::new('2', '@', false),
    ScancodeEntry::new('3', '#', false), ScancodeEntry::new('4', '$', false), ScancodeEntry::new('5', '%', false), ScancodeEntry::new('6', '^', false),
    ScancodeEntry::new('7', '&', false), ScancodeEntry::new('8', '*', false), ScancodeEntry::new('9', '(', false), ScancodeEntry::new('0', ')', false),
    ScancodeEntry::new('-', '_', false), ScancodeEntry::new('=', '+', false), ScancodeEntry::new('\x08', '\x08', false), ScancodeEntry::new('\t', '\t', false), // 0x0F
    ScancodeEntry::new('q', 'Q', true), ScancodeEntry::new('w', 'W', true), ScancodeEntry::new('e', 'E', true), ScancodeEntry::new('r', 'R', true),
    ScancodeEntry::new('t', 'T', true), ScancodeEntry::new('y', 'Y', true), ScancodeEntry::new('u', 'U', true), ScancodeEntry::new('i', 'I', true),
    ScancodeEntry::new('o', 'O', true), ScancodeEntry::new('p', 'P', true), ScancodeEntry::new('[', '{', false), ScancodeEntry::new(']', '}', false),
    ScancodeEntry::new('\n', '\n', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('a', 'A', true), ScancodeEntry::new('s', 'S', true), // 0x1F
    ScancodeEntry::new('d', 'D', true), ScancodeEntry::new('f', 'F', true), ScancodeEntry::new('g', 'G', true), ScancodeEntry::new('h', 'H', true),
    ScancodeEntry::new('j', 'J', true), ScancodeEntry::new('k', 'K', true), ScancodeEntry::new('l', 'L', true), ScancodeEntry::new(';', ':', false),
    ScancodeEntry::new('\'', '"', false), ScancodeEntry::new('`', '~', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\\', '|', false),
    ScancodeEntry::new('z', 'Z', true), ScancodeEntry::new('x', 'X', true), ScancodeEntry::new('c', 'C', true), ScancodeEntry::new('v', 'V', true), // 0x2F
    ScancodeEntry::new('b', 'B', true), ScancodeEntry::new('n', 'N', true), ScancodeEntry::new('m', 'M', true), ScancodeEntry::new(',', '<', false),
    ScancodeEntry::new('.', '>', false), ScancodeEntry::new('/', '?', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('*', '*', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new(' ', ' ', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), // 0x3F
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\x11', '\x11', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('-', '-', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('+', '+', false), ScancodeEntry::new('\0', '\0', false), // 0x4F
    ScancodeEntry::new('\x12', '\x12', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\x1F', '\x1F', false),
    ScancodeEntry::new('\x1F', '\x1F', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), // 0x5F
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), // 0x6F
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false),
    ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), ScancodeEntry::new('\0', '\0', false), // 0x7F
];

/// Internal version for use in interrupt handlers to avoid deadlock
fn scancode_to_char_internal(kb: &KeyBuffer, scancode: u8) -> Option<char> {
    let idx = scancode as usize;
    if idx >= SCANCODE_TABLE.len() { return None; }

    let entry = &SCANCODE_TABLE[idx];
    let shift = kb.modifiers.lshift || kb.modifiers.rshift;
    let capslock = kb.modifiers.capslock;
    
    // Determine if we should use the shifted character table.
    // For alpha keys, Shift and CapsLock toggle each other. 
    // For symbols/numbers, only Shift matters.
    let use_shifted = shift ^ (capslock && entry.is_alpha);
    let c = if use_shifted { entry.shifted } else { entry.normal };

    if c == '\0' { None } else { Some(c) }
}

pub fn init() -> Result<(), &'static str> {
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
        ps2::send_device_command(0xFF, false)?;
        if ps2::wait_and_read()? != 0xAA { return Err("Keyboard self-test failed"); }

        ps2::send_device_command(0xF0, false)?;
        ps2::send_device_command(0x02, false)?; // Set Scan Code Set 2

        ps2::send_device_command(0xF6, false)?; // Set Defaults

        ps2::send_device_command(0xF4, false)?; // Enable Scanning

        // 5. Enable the devices
        ps2::write_command(0xAE); // Enable keyboard
        ps2::write_command(0xA8); // Enable mouse
    }
    Ok(())
}

/// Converts a PS/2 scancode (Set 1) to a character.
/// Handles a basic QWERTY layout.
pub fn scancode_to_char(scancode: u8) -> Option<char> {
    let kb = KEY_BUFFER.lock();
    let res = scancode_to_char_internal(&kb, scancode);
    res
}