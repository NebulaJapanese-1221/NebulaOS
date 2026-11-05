// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

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
#[path = "../sync.rs"]
mod sync;
#[path = "../userspace.rs"]
mod userspace;

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    services::error::handle_panic(info)
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
    drivers::pci::init();
    drivers::rtc::init();
    drivers::audio::init();
    arch::smp::init(multiboot_info);

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
        if drivers::mouse::interrupts_enabled() {
            arch::pic::unmask_irq(12);
        }
    } else {
        drivers::serial::write_string("PS/2 mouse unavailable; keyboard navigation enabled.\r\n");
    }
    boot_screen.update(graphics::boot_screen::BootStage::InputReady);

    arch::pic::unmask_irq(0);
    unsafe {
        asm!("sti");
    }
    arch::smp::init(multiboot_info);
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
            desktop.refresh_periodic_apps();
        }
        redraw |= graphics::wm::take_theme_changed();
        redraw |= graphics::wm::take_desktop_redraw_request();
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
