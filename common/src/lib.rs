#![no_std]
#![allow(dead_code, unused_imports, unused_mut, unused_variables, unreachable_code, static_mut_refs)]

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
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
pub mod input;

pub use stdint::*;
