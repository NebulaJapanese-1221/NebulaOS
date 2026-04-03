use crate::kernel::io;

const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

const BASE_FREQUENCY: u32 = 1193182;

/// Initializes the PIT to fire at a specific frequency.
pub fn set_frequency(frequency: u32) {
    let mut divisor = BASE_FREQUENCY / frequency;
    if divisor > 0xFFFF {
        divisor = 0xFFFF; // Max divisor
    }
    if divisor == 0 {
        divisor = 1; // Min divisor
    }

    unsafe {
        // Command byte: Channel 0, LSB/MSB, Mode 3 (Square Wave)
        io::outb(PIT_COMMAND, 0x36);
        // Send divisor
        io::outb(PIT_CHANNEL0_DATA, (divisor & 0xFF) as u8);
        io::outb(PIT_CHANNEL0_DATA, ((divisor >> 8) & 0xFF) as u8);
    }
}