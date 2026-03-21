use super::acpi;
use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::gui::font;
use core::arch::asm;
use super::io;

/// Shuts down the machine using ACPI.
pub fn shutdown() {
    // This will attempt to perform an ACPI shutdown.
    // It may not work on all hardware or emulators, but it is more portable.
    acpi::acpi_shutdown();

    // Try emulator shutdown hacks for QEMU/Bochs/VirtualBox
    unsafe {
        io::outw(0xB004, 0x2000); // Bochs / Older QEMU
        io::outw(0x604, 0x2000);  // Newer QEMU
        io::outw(0x4004, 0x3400); // VirtualBox
    }

    // If ACPI shutdown fails, fall back to a manual screen.
    unsafe { asm!("cli", options(nomem, nostack)); } // Disable interrupts

    let mut fb = FRAMEBUFFER.lock();
    // Extract dimensions to avoid borrowing issues
    let dims = fb.info.as_ref().map(|i| (i.width, i.height));

    if let Some((width, height)) = dims {
        fb.clear(0x00_00_00_00); // Black screen

        let msg = "It is now safe to turn off your computer.";
        let x = (width / 2).saturating_sub((msg.len() * 8) / 2);
        let y = height / 2;
        
        font::draw_string(&mut fb, x as isize, y as isize, msg, 0x00_FF_88_00, None); // Orange text
        fb.present();
    }

    loop {
        unsafe { asm!("hlt", options(nomem, nostack)); }
    }
}

/// Reboots the machine using the keyboard controller.
pub fn reboot() -> ! {
    unsafe {
        // Use the keyboard controller (port 0x64) to trigger a system reset.
        // This is a common and reliable method.
        let mut good = false;
        while !good {
            // Wait for the input buffer to be clear.
            if (io::inb(0x64) & 2) == 0 {
                io::outb(0x64, 0xFE); // Send the 'CPU Reset' command.
                good = true;
            }
        }
    }
    loop { unsafe { asm!("hlt") } }
}