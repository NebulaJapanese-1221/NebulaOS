use crate::common::vga;

pub unsafe fn init() {
    vga::puts(b"GUI: Initializing...\n\0" as *const u8 as *const u8);
}

pub unsafe fn run() {
    vga::puts(b"GUI: Running\n\0" as *const u8 as *const u8);
}
