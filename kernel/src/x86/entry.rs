#![no_std]

use core::arch::asm;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    let vga = 0xB8000 as *mut u16;
    let message = b"NebulaOS x86 kernel booted (Multiboot)";

    unsafe {
        for cell in 0..(80 * 25) {
            vga.add(cell).write_volatile(0x0F20);
        }

        for (index, byte) in message.iter().enumerate() {
            vga.add(index).write_volatile(0x0F00 | *byte as u16);
        }
    }

    loop {
        unsafe { asm!("cli", "hlt", options(nomem, nostack)); }
    }
}
