use core::arch::asm;

extern "C" {
    fn init_idt();
    static __kernel_start: u8;
    static __kernel_end: u8;
}

unsafe fn serial_out(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

unsafe fn serial_in(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    value
}

fn serial_write(message: &[u8]) {
    unsafe {
        serial_out(0x3F9, 0x00);
        serial_out(0x3FB, 0x80);
        serial_out(0x3F8, 0x03);
        serial_out(0x3F9, 0x00);
        serial_out(0x3FB, 0x03);
        serial_out(0x3FA, 0xC7);
        serial_out(0x3FC, 0x0B);

        for byte in message {
            while serial_in(0x3FD) & 0x20 == 0 {}
            serial_out(0x3F8, *byte);
        }
    }
}

#[repr(C)]
struct BootFramebuffer {
    address: u32,
    width: u32,
    height: u32,
    stride: u32,
    bits_per_pixel: u32,
    red_size: u32,
    red_position: u32,
    green_size: u32,
    green_position: u32,
    blue_size: u32,
    blue_position: u32,
    memory_map: u32,
    memory_region_count: u32,
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    serial_write(b"NebulaOS x86 kernel booted\r\n");
    unsafe {
        let text_mode_only = core::ptr::read_volatile(0x5034 as *const u8) != 0;
        if text_mode_only {
            common::vga::init();
            common::vga::puts(b"NebulaOS text mode boot\n\0" as *const u8 as *const u8);
            common::shell::shell_run();
        }

        init_idt();
        drivers::pic::pic_init(0x20, 0x28);
        drivers::keyboard::keyboard_init();
        drivers::pic::pic_enable_irq(1);
        drivers::mouse::mouse_init();

        let mut display = &*(0x5000 as *const BootFramebuffer);
        let mut framebuffer = display.address as *mut u8;
        let mut width = display.width;
        let mut height = display.height;
        let mut stride = display.stride;
        let mut bits_per_pixel = display.bits_per_pixel;
        let mut red_size = display.red_size;
        let mut red_position = display.red_position;
        let mut green_size = display.green_size;
        let mut green_position = display.green_position;
        let mut blue_size = display.blue_size;
        let mut blue_position = display.blue_position;

        if display.address == 0 || display.width == 0 || display.height == 0 || display.stride == 0 {
            if drivers::vesa::vesa_init() {
                framebuffer = drivers::vesa::vesa_get_framebuffer();
                width = drivers::vesa::vesa_get_width();
                height = drivers::vesa::vesa_get_height();
                stride = drivers::vesa::vesa_get_stride();
                bits_per_pixel = drivers::vesa::vesa_get_bpp();
                red_size = 8;
                red_position = 16;
                green_size = 8;
                green_position = 8;
                blue_size = 8;
                blue_position = 0;
                display = &*(0x5000 as *const BootFramebuffer);
            }
        }

        common::memory::memory_init_with_map(
            display.memory_map as *const common::memory::MemoryRegion,
            display.memory_region_count as usize,
            core::ptr::addr_of!(__kernel_start) as u64,
            core::ptr::addr_of!(__kernel_end) as u64,
            framebuffer as u64,
            (stride as u64) * (height as u64),
        );
        common::scheduler::scheduler_init();
        common::process::process_init();
        common::fs::fs_init();
        gui::gui::init(
            framebuffer,
            width,
            height,
            stride,
            bits_per_pixel,
            red_size,
            red_position,
            green_size,
            green_position,
            blue_size,
            blue_position,
        );
        gui::gui::run();
    }
}
