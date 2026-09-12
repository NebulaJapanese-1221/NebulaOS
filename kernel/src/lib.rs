#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(alloc_error_handler)]

pub mod common;
#[cfg(target_arch = "x86")]
pub mod x86;
#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    loop {}
}
