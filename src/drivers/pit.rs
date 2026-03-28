use crate::kernel::io;

/// Sets the PIT Channel 0 frequency and verifies that the hardware has accepted the divisor.
/// Returns Ok(()) if the Null Count bit cleared within the timeout, Err otherwise.
pub fn set_frequency(hz: u32) -> Result<(), &'static str> {
    // The PIT frequency is 1.193182 MHz
    let divisor = 1193182 / hz;

    unsafe {
        // Command Register (0x43):
        // Bits 7-6: 00 (Select Channel 0)
        // Bits 5-4: 11 (Access mode: Lobo/Hibo)
        // Bits 3-1: 011 (Mode 3: Square Wave Generator)
        // Bit 0: 0 (Binary mode)
        // Result: 0x36
        io::outb(0x43, 0x36);
        io::wait();
        io::outb(0x40, (divisor & 0xFF) as u8);
        io::wait();
        io::outb(0x40, ((divisor >> 8) & 0xFF) as u8);

        // Verification: Wait for the 'Null Count' bit (Bit 6) to clear via Read-back command.
        // Command 0xE2: 11 (Read-back) 1 (No count) 0 (Latch status) 001 (Ch 0) 0 (Res)
        let mut timeout = 100000;
        while timeout > 0 {
            io::outb(0x43, 0xE2);
            let status = io::inb(0x40);
            
            // Bit 6 is 0 when the divisor has been loaded into the counter element.
            if (status & 0x40) == 0 {
                return Ok(());
            }
            timeout -= 1;
            core::hint::spin_loop();
        }
    }

    Err("PIT Hardware Error: Null Count bit failed to clear (Hardware unresponsive)")
}