#![no_std]
#![no_main]

use core::panic::PanicInfo;

// NOTE: This file contains the NebulaOS Bootloader Entry Point draft.
// Per project organization, this should be moved to src/boot/main.rs.
// To replace GRUB and be Multiboot compliant, the final version must:
// 1. Switch to Protected Mode (32-bit).
// 2. parse the Multiboot header of the Kernel (nebula_os).
// 3. Load the Kernel into memory.
// 4. Populate the Multiboot Information Structure.
// 5. Jump to the Kernel entry point.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // TODO: Implement Stage 1 and Stage 2 loading logic.
    // This is where the custom bootloader logic goes.
    
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}