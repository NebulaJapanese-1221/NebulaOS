pub unsafe fn init() {
    crate::common::vga::puts(b"Renderer: stub\n\0" as *const u8 as *const u8);
}
