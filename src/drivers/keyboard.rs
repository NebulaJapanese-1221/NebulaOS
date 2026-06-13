use crate::mouse::inb;
use crate::sync::Spinlock;

const BUFFER_SIZE: usize = 128;

pub struct KeyBuffer {
    buffer: [char; BUFFER_SIZE],
    head: usize,
    tail: usize,
}

impl KeyBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: ['\0'; BUFFER_SIZE],
            head: 0,
            tail: 0,
        }
    }

    pub fn push(&mut self, c: char) {
        let next = (self.head + 1) % BUFFER_SIZE;
        if next != self.tail {
            self.buffer[self.head] = c;
            self.head = next;
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        if self.head != self.tail {
            let c = self.buffer[self.tail];
            self.tail = (self.tail + 1) % BUFFER_SIZE;
            Some(c)
        } else {
            None
        }
    }
}

pub static KEY_BUFFER: Spinlock<KeyBuffer> = Spinlock::new(KeyBuffer::new());

pub fn handle_keyboard_interrupt() {
    unsafe {
        let scancode = inb(0x60);
        
        // Basic scancode set 1 mapping (Non-exhaustive)
        // Scancodes above 0x80 are key release events.
        if scancode < 0x80 {
            let ascii = match scancode {
                0x02 => '1', 0x03 => '2', 0x04 => '3', 0x05 => '4', 0x06 => '5',
                0x07 => '6', 0x08 => '7', 0x09 => '8', 0x0A => '9', 0x0B => '0',
                0x10 => 'q', 0x11 => 'w', 0x12 => 'e', 0x13 => 'r', 0x14 => 't',
                0x15 => 'y', 0x16 => 'u', 0x17 => 'i', 0x18 => 'o', 0x19 => 'p',
                0x1E => 'a', 0x1F => 's', 0x20 => 'd', 0x21 => 'f', 0x22 => 'g',
                0x23 => 'h', 0x24 => 'j', 0x25 => 'k', 0x26 => 'l',
                0x2C => 'z', 0x2D => 'x', 0x2E => 'c', 0x2F => 'v', 0x30 => 'b',
                0x31 => 'n', 0x32 => 'm',
                0x39 => ' ', // Spacebar
                0x1C => '\n', // Enter
                _ => '\0',
            };

            if ascii != '\0' {
                KEY_BUFFER.lock().push(ascii);
            }
        }

        // Send End of Interrupt to Master PIC
        crate::mouse::outb(0x20, 0x20);
    }
}