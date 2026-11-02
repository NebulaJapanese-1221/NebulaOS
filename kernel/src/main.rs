#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

#[path = "../arch/mod.rs"]
mod arch;
#[path = "../drivers/mod.rs"]
mod drivers;
#[path = "../graphics/mod.rs"]
mod graphics;
#[path = "../memory/mod.rs"]
mod memory;
#[path = "../services/mod.rs"]
mod services;
#[path = "../userspace.rs"]
mod userspace;

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    drivers::serial::write_string("Kernel panic.\r\n");
    let mut output = [0; 128];
    let length = services::user_process::copy_last_output(&mut output);
    if length != 0 {
        drivers::serial::write_string("Captured user output: ");
        for byte in &output[..length] {
            drivers::serial::write_byte(*byte);
        }
        drivers::serial::write_string("\r\n");
    }
    halt()
}

#[no_mangle]
pub extern "C" fn kernel_entry(multiboot_info: *const u8) -> ! {
    arch::gdt::init();
    drivers::serial::init();
    drivers::serial::write_string("NebulaOS kernel started.\r\n");

    arch::tss::init(0x90000);
    memory::init();
    memory::paging::init();
    services::process::init();
    memory::allocator::init();
    arch::idt::init();
    arch::pic::remap(0x20, 0x28);
    arch::pic::mask_irq(0);
    drivers::pit::init(100);

    let framebuffer = match drivers::framebuffer::init(multiboot_info) {
        Ok(framebuffer) => framebuffer,
        Err(error) => {
            drivers::serial::write_string("Framebuffer initialization failed: ");
            drivers::serial::write_string(match error {
                drivers::framebuffer::FramebufferError::InvalidBootInformation => {
                    "invalid boot info"
                }
                drivers::framebuffer::FramebufferError::NoRgbFramebuffer => {
                    "RGB framebuffer missing"
                }
                drivers::framebuffer::FramebufferError::UnsupportedPixelFormat => {
                    "unsupported pixel format"
                }
                drivers::framebuffer::FramebufferError::FramebufferTooLarge => {
                    "framebuffer too large"
                }
                drivers::framebuffer::FramebufferError::AddressNotMapped => {
                    "framebuffer map failed"
                }
                drivers::framebuffer::FramebufferError::BackBufferAllocationFailed => {
                    "framebuffer back buffer allocation failed"
                }
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
    let mut boot_screen = graphics::boot_screen::BootScreen::new();
    match drivers::disk::init() {
        Ok(()) => drivers::serial::write_string(if drivers::disk::is_formatted() {
            "IDE storage ready; NebulaFS mounted.\r\n"
        } else {
            "IDE storage ready; disk needs explicit formatting.\r\n"
        }),
        Err(drivers::disk::StorageError::NoDisk) => drivers::serial::write_string(
            "No IDE disk detected; file manager storage unavailable.\r\n",
        ),
        Err(_) => drivers::serial::write_string("IDE storage initialization failed.\r\n"),
    }
    boot_screen.update(graphics::boot_screen::BootStage::StorageReady);

    drivers::keyboard::init();
    if drivers::mouse::init() {
        drivers::serial::write_string("PS/2 mouse ready.\r\n");
    } else {
        drivers::serial::write_string("PS/2 mouse unavailable; keyboard navigation enabled.\r\n");
    }
    boot_screen.update(graphics::boot_screen::BootStage::InputReady);

    arch::pic::unmask_irq(0);
    unsafe {
        asm!("sti");
    }
    let mut desktop = graphics::wm::WindowManager::new();
    boot_screen.update(graphics::boot_screen::BootStage::DesktopReady);
    boot_screen.wait_until_minimum_duration();
    desktop.render();
    let mut last_periodic_redraw = drivers::pit::ticks();
    drivers::serial::write_string("Graphical desktop ready.\r\n");

    loop {
        let mut redraw = false;
        let ticks = drivers::pit::ticks();
        if ticks.wrapping_sub(last_periodic_redraw) >= 100 {
            last_periodic_redraw = ticks;
            redraw = true;
        }
        redraw |= graphics::wm::take_theme_changed();
        if let Some(key) = drivers::keyboard::poll_key() {
            redraw |= desktop.handle_key(key);
        }
        if let Some(mouse) = drivers::mouse::poll() {
            redraw |= desktop.handle_mouse(mouse);
        }
        if redraw {
            desktop.render();
        }
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}

fn halt() -> ! {
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}
