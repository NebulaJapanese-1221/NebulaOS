use core::arch::global_asm;

global_asm!(
    ".global mouse_handler_asm",
    "mouse_handler_asm:",
    "pushal",                  // Save all registers
    "push ds", "push es", "push fs", "push gs",
    "call mouse_handler_rust", // Call the actual logic
    "pop gs", "pop fs", "pop es", "pop ds",
    "popal",                   // Restore registers
    "iretd"                    // Interrupt Return (32-bit)
);

global_asm!(
    ".global keyboard_handler_asm",
    "keyboard_handler_asm:",
    "pushal",
    "push ds", "push es", "push fs", "push gs",
    "call keyboard_handler_rust",
    "pop gs", "pop fs", "pop es", "pop ds",
    "popal",
    "iretd"
);

global_asm!(
    ".global syscall_handler_asm",
    "syscall_handler_asm:",
    "pushal",
    "push ds", "push es", "push fs", "push gs",
    "mov eax, esp",
    "push eax",
    "call syscall_handler_rust",
    "mov esp, eax",            // Switch to the returned stack pointer
    "pop gs", "pop fs", "pop es", "pop ds",
    "popal",
    "iretd"
);

global_asm!(
    ".global timer_handler_asm",
    "timer_handler_asm:",
    "pushal",
    "push ds", "push es", "push fs", "push gs",
    "mov eax, esp", "push eax",
    "call timer_handler_rust",
    "mov esp, eax",            // Switch to the returned stack pointer
    "pop gs", "pop fs", "pop es", "pop ds",
    "popal",
    "iretd"
);

extern "C" {
    pub fn mouse_handler_asm();
    pub fn keyboard_handler_asm();
    pub fn syscall_handler_asm();
    pub fn timer_handler_asm();
}

#[no_mangle]
pub extern "C" fn mouse_handler_rust() {
    super::mouse::handle_mouse_interrupt();
    unsafe {
        // Global EOI for IRQ 12
        super::ps2::outb(0xA0, 0x20);
        super::ps2::outb(0x20, 0x20);
    }
}

#[no_mangle]
pub extern "C" fn keyboard_handler_rust() {
    super::keyboard::handle_keyboard_interrupt();
    // Send EOI to Master PIC for IRQ 1
    unsafe {
        super::ps2::outb(0x20, 0x20);
    }
}

#[no_mangle]
pub extern "C" fn syscall_handler_rust(regs: *mut super::syscalls::SyscallRegisters) -> u32 {
    unsafe { super::syscalls::syscall_handler_rust(&mut *regs) }
}

#[no_mangle]
pub extern "C" fn timer_handler_rust(regs: *mut super::syscalls::SyscallRegisters) -> u32 {
    // Send EOI to Master PIC for IRQ 0
    unsafe {
        super::ps2::outb(0x20, 0x20);
    }
    super::scheduler::timer_tick();
    super::scheduler::schedule(regs as u32)
}