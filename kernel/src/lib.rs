#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![feature(linked_list_cursors)]

extern crate drivers;
extern crate gui;
extern crate lib;

pub mod common;
pub mod x86;
pub mod x86_64;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    loop {}
}
