use core::panic::PanicInfo;
use core::arch::asm; 
use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::fonts::font;
use super::exceptions;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {    
    // Disable interrupts to prevent further issues during panic
    unsafe { core::arch::asm!("cli") };

    crate::serial_println!("\nKERNEL PANIC\n{}", info);
    unsafe { exceptions::print_stack_trace(); }

    // Draw to screen
    let mut fb = FRAMEBUFFER.lock();
    fb.clear(0x00_CC0000); // Red (RSOD)
    
    font::draw_string(&mut fb, 30, 30, ":(", 0xFFFFFFFF, None);
    font::draw_string(&mut fb, 30, 60, "NebulaOS ran into a problem and needs to restart.", 0xFFFFFFFF, None);

    let mut writer = exceptions::PanicWriter::new(&mut fb, 30, 90);
    use core::fmt::Write;
    let _ = writeln!(writer, "Stop Code: KERNEL_PANIC");
    let _ = writeln!(writer, "Details: {}", info);
    let _ = writeln!(writer, "\nTechnical Information:\n----------------------");
    unsafe { exceptions::print_stack_trace_to(&mut writer); }
    
    // Draw support QR Code
    exceptions::draw_qr_code(&mut fb, 30, 430);
    
    fb.present();

    loop {
        unsafe { asm!("cli; hlt", options(nomem, nostack)); }
    }
}