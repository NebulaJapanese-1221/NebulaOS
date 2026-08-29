use crate::common::vga;

pub unsafe fn enter_graphics() -> i32 {
    vga::puts(b"GUI: Switching to graphics mode...\n\0" as *const u8 as *const u8);
    0
}
