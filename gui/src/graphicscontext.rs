use crate::common::vga;

pub unsafe fn init() {
    vga::puts(b"GraphicsContext: stub\n\0" as *const u8 as *const u8);
}
