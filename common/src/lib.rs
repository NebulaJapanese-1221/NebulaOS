#![no_std]
#![feature(asm)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

pub mod stdint;
pub mod io;
pub mod vga;
pub mod idt;
pub mod memory;
pub mod gdt;
pub mod nebula;
pub mod fs;
pub mod elf;
pub mod process;
pub mod scheduler;
pub mod syscall;
pub mod shell;

pub use stdint::*;
