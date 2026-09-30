#![no_std]

extern "C" {
    fn init_gdt64();
    fn init_idt64();
    static __kernel_start: u8;
    static __kernel_end: u8;
}

#[repr(C)]
struct BootFramebuffer64 {
    address: u64,
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
    memory_map: u64,
    memory_region_count: u32,
    reserved: u32,
}

#[no_mangle]
pub unsafe extern "C" fn kernel_main(display_info: *const BootFramebuffer64) -> ! {
    init_gdt64();
    init_idt64();

    drivers::pic::pic_init(0x20, 0x28);
    drivers::pit::pit_init(1000);
    drivers::keyboard::keyboard_init();
    drivers::pic::pic_enable_irq(1);
    drivers::mouse::mouse_init();

    let display = &*display_info;
    common::memory::memory_init_with_map(
        display.memory_map as *const common::memory::MemoryRegion,
        display.memory_region_count as usize,
        core::ptr::addr_of!(__kernel_start) as u64,
        core::ptr::addr_of!(__kernel_end) as u64,
        display.address,
        (display.stride as u64) * (display.height as u64),
    );
    common::process::process_init();
    common::scheduler::scheduler_init();
    common::syscall::syscall_init();
    common::fs::fs_init();
    drivers::pci::pci_init();
    drivers::acpi::acpi_init();
    drivers::serial::serial_init(0x3F8, 115200);
    drivers::serial::serial_puts(b"NebulaOS x86_64 kernel booted\r\n\0".as_ptr());
    drivers::ata::ata_init();

    asm!("sti");
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
    gui::gui::run()
}
