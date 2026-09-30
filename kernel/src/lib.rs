#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![allow(dead_code, unused_imports, unused_mut, unused_variables, unused_attributes, suspicious_runtime_symbol_definitions, static_mut_refs)]

extern crate gui;

pub mod common;
#[cfg(target_arch = "x86")]
pub mod x86;
#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn memset(destination: *mut u8, value: i32, count: usize) -> *mut u8 {
    for offset in 0..count {
        core::ptr::write_volatile(destination.add(offset), value as u8);
    }
    destination
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(destination: *mut u8, source: *const u8, count: usize) -> *mut u8 {
    for offset in 0..count {
        let value = core::ptr::read_volatile(source.add(offset));
        core::ptr::write_volatile(destination.add(offset), value);
    }
    destination
}
