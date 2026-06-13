use crate::mouse::outb;

/// Initialize the PIT to a specific frequency (Hz)
pub fn init(hz: u32) {
    // The PIT has an internal oscillator frequency of 1.193182 MHz
    let divisor = 1193182 / hz;

    unsafe {
        // Command byte: Channel 0, access low/high byte, square wave mode, 16-bit binary
        outb(0x43, 0x36);
        outb(0x40, (divisor & 0xFF) as u8);         // Low byte
        outb(0x40, ((divisor >> 8) & 0xFF) as u8);  // High byte
    }
}