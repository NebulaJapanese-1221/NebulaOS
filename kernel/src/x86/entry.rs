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
        init_idt();
        drivers::pic::pic_init(0x20, 0x28);
        drivers::keyboard::keyboard_init();
        drivers::pic::pic_enable_irq(1);
        drivers::mouse::mouse_init();
        let display = &*(0x5000 as *const BootFramebuffer);
        common::memory::memory_init_with_map(
            display.memory_map as *const common::memory::MemoryRegion,
            display.memory_region_count as usize,
            core::ptr::addr_of!(__kernel_start) as u64,
            core::ptr::addr_of!(__kernel_end) as u64,
            display.address as u64,
            (display.stride as u64) * (display.height as u64),
        );
        common::scheduler::scheduler_init();
        common::process::process_init();
        common::fs::fs_init();
        gui::gui::init(
            display.address as *mut u8,
            display.width,
            display.height,
            display.stride,
            display.bits_per_pixel,
            display.red_size,
            display.red_position,
            display.green_size,
            display.green_position,
            display.blue_size,
            display.blue_position,
        );
        gui::gui::run();
    }
}
