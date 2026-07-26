use core::panic::PanicInfo;
use crate::serial_println;
use crate::framebuffer::FRAMEBUFFER;
use crate::gui;
use core::arch::asm;

/// Helper to draw hex values on the panic screen
fn draw_hex(fb: &mut crate::framebuffer::Framebuffer, x: usize, y: usize, val: u32, color: u32) {
    let hex = b"0123456789ABCDEF";
    let mut buf = [b'0', b'x', 0, 0, 0, 0, 0, 0, 0, 0];
    for i in 0..8 {
        buf[9 - i] = hex[((val >> (i * 4)) & 0xF) as usize];
    }
    if let Ok(s) = core::str::from_utf8(&buf) {
        gui::draw_string(fb, x, y, s, color);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Log to serial as a fallback
    serial_println!("\n--- KERNEL PANIC ---");
    serial_println!("{}", info);

    // Walk the stack for serial logging
    serial_println!("Stack Trace:");
    #[cfg(target_arch = "x86")]
    let mut frame_ptr: *const usize;
    #[cfg(target_arch = "x86_64")]
    let mut frame_ptr: *const usize;
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!("mov {}, ebp", out(reg) frame_ptr);
        #[cfg(target_arch = "x86_64")]
        asm!("mov {}, rbp", out(reg) frame_ptr);
    }

    let mut depth = 0;
    while !frame_ptr.is_null() && depth < 10 {
        unsafe {
            let ret_addr = *frame_ptr.offset(1);
            serial_println!("  [{}] 0x{:08x}", depth, ret_addr as usize);
            
            let next = *frame_ptr as *const usize;
            if next <= frame_ptr || (next as usize) < 0x1000 { break; }
            frame_ptr = next;
        }
        depth += 1;
    }

    // 2. Attempt to show a graphical panic screen
    unsafe {
        FRAMEBUFFER.force_unlock();
    }
    
    let mut fb = FRAMEBUFFER.lock();
    // Blue background
    fb.draw_rect(0, 0, 1024, 768, 0x000000AA);
    fb.mark_dirty(0, 0, 1024, 768); // Force full screen update for panic
    
    gui::draw_string(&mut fb, 400, 300, "KERNEL PANIC", 0xFFFFFF);
    gui::draw_string(&mut fb, 100, 350, "A critical error has occurred and the system was halted.", 0xCCCCCC);
    
    // Display stack trace on screen
    gui::draw_string(&mut fb, 100, 400, "Stack Trace:", 0xFFFFFF);
    let mut trace_fp: *const usize;
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!("mov {}, ebp", out(reg) trace_fp);
        #[cfg(target_arch = "x86_64")]
        asm!("mov {}, rbp", out(reg) trace_fp);
    }
    
    for i in 0..8 {
        unsafe {
            if trace_fp.is_null() { break; }
            let ret_addr = *trace_fp.offset(1);
            draw_hex(&mut fb, 120, 420 + (i * 15), ret_addr as u32, 0xDDDDDD);
            
            let next_fp = *trace_fp as *const usize;
            if next_fp <= trace_fp || (next_fp as usize) < 0x1000 { break; }
            trace_fp = next_fp;
        }
    }
    
    fb.present();

    loop {}
}