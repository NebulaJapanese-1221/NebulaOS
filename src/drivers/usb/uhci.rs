use crate::kernel::io;
use super::UsbHostController;

/// UHCI Register Offsets
const USBCMD: u16 = 0x00;  // USB Command Register
const USBSTS: u16 = 0x02;  // USB Status Register
const USBINTR: u16 = 0x04; // USB Interrupt Enable Register
const FRNUM: u16 = 0x06;   // Frame Number Register
const FRBASEADD: u16 = 0x08; // Frame List Base Address Register
const SOFMOD: u16 = 0x0C;  // Start of Frame Modify Register
const PORTSC1: u16 = 0x10; // Port 1 Status/Control
const PORTSC2: u16 = 0x12; // Port 2 Status/Control

pub struct UhciController {
    pub io_base: u16,
}

impl UhciController {
    pub fn new(io_base: u16) -> Self {
        Self { io_base }
    }
}

impl UsbHostController for UhciController {
    fn init(&mut self) {
        crate::serial_println!("[USB] Initializing UHCI at I/O {:#x}", self.io_base);
        
        // 1. Perform a Global Reset
        self.reset();

        // 2. Clear Status Register
        unsafe { io::outw(self.io_base + USBSTS, 0x003F); }

        // 3. Set Frame List Base Address to 0 (Null) for now
        // Real implementations would allocate a 4KB aligned frame list
        unsafe { io::outl(self.io_base + FRBASEADD, 0); }

        // 4. Set Frame Number to 0
        unsafe { io::outw(self.io_base + FRNUM, 0); }

        // 5. Enable Interrupts (CRC, Resume, IOC, Short Packet)
        unsafe { io::outw(self.io_base + USBINTR, 0x000F); }

        crate::serial_println!("[USB] UHCI Controller ready.");
    }

    fn reset(&mut self) {
        unsafe {
            // Write GRESET bit to USBCMD
            io::outw(self.io_base + USBCMD, 0x0004);
            
            // Wait at least 10ms as per spec (using busy loop for now)
            for _ in 0..1_000_000 { core::hint::spin_loop(); }
            
            // Clear reset bit
            io::outw(self.io_base + USBCMD, 0x0000);
        }
    }
}