use super::acpi;
use super::cpu;
use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::fonts::font;
use core::arch::asm;
use super::io;

/// Shuts down the machine using ACPI.
pub fn shutdown() {
    // Disable interrupts to prevent other tasks from drawing over the screen
    unsafe { asm!("cli", options(nomem, nostack)); }

    // Animate "Shutting Down" screen
    for _ in 0..30 { // Reduced frame count for a snappier exit
        let mut fb = FRAMEBUFFER.lock();
        if let Some(info) = fb.info.as_ref() {
            let width = info.width;
            let height = info.height;
            
            // Clear the animation area to prevent smearing
            let clear_color = 0x00_050515; // Matching the boot screen background
            crate::userspace::gui::draw_rect(&mut fb, (width / 2) as isize - 150, (height / 2) as isize - 60, 300, 120, clear_color, None);

            let msg = "NebulaOS is powering off...";
            let x = (width / 2).saturating_sub((msg.len() * 8) / 2);
            font::draw_string(&mut fb, x as isize, (height / 2) as isize - 40, msg, 0x00_AAAAAA, None);
            
            // Draw the spinner animation
            crate::kernel::boot::draw_spinner(&mut fb, (width / 2) as isize, (height / 2) as isize);
            // Optimized blit
            fb.present_rect(width / 2 - 150, height / 2 - 60, 300, 120);
        }
        drop(fb);
        cpu::spin_wait_ms(10);
    }

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
        asm!("cli", options(nomem, nostack)); // Disable interrupts
        
        // Draw "Rebooting" screen
        {
            let mut fb = FRAMEBUFFER.lock();
            // Extract dimensions first to avoid holding an immutable borrow while calling clear() (mutable borrow)
            let dims = fb.info.as_ref().map(|i| (i.width, i.height));
            if let Some((width, height)) = dims {
                fb.clear(0x00_00_00_00);
                let msg = "Rebooting...";
                let x = (width / 2).saturating_sub((msg.len() * 8) / 2);
                let y = height / 2;
                font::draw_string(&mut fb, x as isize, y as isize, msg, 0x00_FFFFFF, None);
                fb.present();
            }
        }

        // Method 1: PCI Reset (Port 0xCF9) - Very reliable on modern systems
        io::outb(0xCF9, 0x06);
        cpu::spin_wait_ms(1);

        // Method 2: Keyboard Controller Reset (Port 0x64)
        for _ in 0..10 {
            while (io::inb(0x64) & 2) != 0 { asm!("nop") }
            io::outb(0x64, 0xFE);
            cpu::spin_wait_ms(10);
        }

        // Method 3: Triple Fault (Fallback)
        let idt_ptr = crate::kernel::interrupts::IdtEntry::new(0, 0, 0);
        asm!("lidt [{}]", in(reg) &idt_ptr);
        asm!("int 3");
    }
    loop { unsafe { asm!("hlt") } }
}