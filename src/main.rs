#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

// Import the boot module to ensure the assembly is compiled
mod boot;

// Import the kernel module
mod kernel;

// Import the drivers module
mod drivers;

// Import the utility module
pub mod utils;

// Import the userspace module
mod userspace;

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    use crate::drivers::serial::SERIAL1;
    use core::fmt::Write;
    let _ = writeln!(SERIAL1.lock(), "Allocation error: {:?}", layout);
    panic!("allocation error: {:?}", layout)
}
