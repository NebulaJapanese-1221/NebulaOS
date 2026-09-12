use common::vga;

pub unsafe fn init() {
    vga::puts(b"Renderer: stub\n\0" as *const u8 as *const u8);
}
