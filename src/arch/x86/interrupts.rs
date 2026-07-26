// x86 (32-bit) interrupt handler assembly stubs
use core::arch::global_asm;

global_asm!(
    ".global mouse_handler_asm",
    "mouse_handler_asm:",
    "pushal",
    "push ds", "push es", "push fs", "push gs",
    "call mouse_handler_rust",
    "pop gs", "pop fs", "pop es", "pop ds",
    "popal",
    "iretd"
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
    "add esp, 4",
    "pop gs", "pop fs", "pop es", "pop ds",
    "popal",
    "iretd"
);

global_asm!(
    ".global timer_handler_asm",
    "timer_handler_asm:",
    "pushal",
    "push ds", "push es", "push fs", "push gs",
    "mov eax, esp",
    "push eax",
    "call timer_handler_rust",
    "add esp, 4",
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
    super::super::super::mouse::handle_mouse_interrupt();
    unsafe {
        use crate::arch::x86::io::outb;
        outb(0xA0, 0x20);
        outb(0x20, 0x20);
    }
}

#[no_mangle]
pub extern "C" fn keyboard_handler_rust() {
    super::super::super::keyboard::handle_keyboard_interrupt();
    unsafe {
        use crate::arch::x86::io::outb;
        outb(0x20, 0x20);
    }
}

#[no_mangle]
pub extern "C" fn syscall_handler_rust(regs: *mut super::super::super::syscalls::SyscallRegisters) -> u32 {
    unsafe { super::super::super::syscalls::syscall_handler_rust(&mut *regs) }
}

#[no_mangle]
pub extern "C" fn timer_handler_rust(regs: *mut super::super::super::syscalls::SyscallRegisters) -> u32 {
    unsafe {
        use crate::arch::x86::io::outb;
        outb(0x20, 0x20);
    }
    super::super::super::scheduler::timer_tick();
    super::super::super::scheduler::schedule(regs as u32)
}

