#![no_std]

use crate::common::{idt, io, nebula};

extern "C" {
    fn init_gdt();
    fn init_idt();
}

#[no_mangle]
pub unsafe extern "C" fn kernel_main(magic: u32, _info: u32) {
    if magic != 0x2BADB002 {
        common::vga::init();
        common::vga::set_color(common::vga::VGA_COLOR_RED);
        common::vga::set_bg_color(common::vga::VGA_COLOR_BLACK);
        common::vga::clear();
        common::vga::puts(b"ERROR: Invalid multiboot magic!\n\0" as *const u8 as *const u8);
        loop { asm!("hlt"); }
    }

    common::vga::init();
    common::vga::set_color(common::vga::VGA_COLOR_WHITE);
    common::vga::set_bg_color(common::vga::VGA_COLOR_BLUE);
    common::vga::clear();
    common::vga::puts(b"NebulaOS x86 Kernel Booting...\n\0" as *const u8 as *const u8);

    init_gdt();
    init_idt();

    drivers::pic::pic_init(0x20, 0x28);
    drivers::pit::pit_init(1000);

    common::memory::memory_init();
    drivers::keyboard::keyboard_init();
    drivers::mouse::mouse_init();
    common::process::process_init();
    common::scheduler::scheduler_init();
    common::syscall::syscall_init();
    common::fs::fs_init();
    drivers::pci::pci_init();
    drivers::acpi::acpi_init();
    drivers::serial::serial_init(0x3F8, 115200);
    drivers::ata::ata_init();
    drivers::vesa::vesa_init();

    common::vga::puts(b"\nNebulaOS x86 Kernel Initialized\n\0" as *const u8 as *const u8);
    common::vga::puts(b"Version: 0.0.1\n\0" as *const u8 as *const u8);

    common::shell::shell_init();

    asm!("sti");
    loop {
        asm!("hlt");
    }
}
