#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

#[path = "../drivers/mod.rs"]
mod drivers;
#[path = "../memory/mod.rs"]
mod memory;
#[path = "../gdt.rs"]
mod gdt;
#[path = "../tss.rs"]
mod tss;
#[path = "../idt.rs"]
mod idt;
#[path = "../../userspace/mod.rs"]
mod userspace;

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    drivers::serial::write_string("Kernel panic.\r\n");
    halt()
}

#[no_mangle]
pub extern "C" fn kernel_entry(multiboot_info: *const u8) -> ! {
    gdt::init();
    drivers::serial::init();
    drivers::serial::write_string("NebulaOS kernel started.\r\n");

    tss::init(0x90000);
    memory::init();
    drivers::paging::init();
    drivers::allocator::init();
    idt::init();
    drivers::pic::remap(0x20, 0x28);
    drivers::pit::init(100);

    let framebuffer = match drivers::framebuffer::init(multiboot_info) {
        Ok(framebuffer) => framebuffer,
        Err(error) => {
            drivers::serial::write_string("Framebuffer initialization failed: ");
            drivers::serial::write_string(match error {
                drivers::framebuffer::FramebufferError::InvalidBootInformation => "invalid boot info",
                drivers::framebuffer::FramebufferError::NoRgbFramebuffer => "RGB framebuffer missing",
                drivers::framebuffer::FramebufferError::UnsupportedPixelFormat => {
                    "unsupported pixel format"
                }
                drivers::framebuffer::FramebufferError::FramebufferTooLarge => "framebuffer too large",
                drivers::framebuffer::FramebufferError::AddressNotMapped => "framebuffer map failed",
            });
            drivers::serial::write_string("\r\n");
            halt();
        }
    };

    drivers::serial::write_string("Graphics framebuffer ready: ");
    drivers::serial::write_dec(framebuffer.width);
    drivers::serial::write_string("x");
    drivers::serial::write_dec(framebuffer.height);
    drivers::serial::write_string("\r\n");

    drivers::keyboard::init();
    if drivers::mouse::init() {
        drivers::serial::write_string("PS/2 mouse ready.\r\n");
    } else {
        drivers::serial::write_string("PS/2 mouse unavailable; keyboard navigation enabled.\r\n");
    }

    let mut desktop = drivers::wm::WindowManager::new();
    desktop.render();
    drivers::pic::unmask_irq(0);
    unsafe { asm!("sti"); }
    drivers::serial::write_string("Graphical desktop ready.\r\n");

    loop {
        let mut redraw = false;
        if let Some(key) = drivers::keyboard::poll_key() {
            redraw |= desktop.handle_key(key);
        }
        if let Some(mouse) = drivers::mouse::poll() {
            redraw |= desktop.handle_mouse(mouse);
        }
        if redraw {
            desktop.render();
        }
        unsafe { asm!("hlt", options(nomem, nostack)); }
    }
}

fn halt() -> ! {
    loop {
        unsafe { asm!("cli", "hlt", options(nomem, nostack)); }
    }
}
