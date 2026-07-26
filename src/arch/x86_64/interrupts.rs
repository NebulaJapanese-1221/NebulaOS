// x86_64 (64-bit) interrupt handler assembly stubs
// Uses 64-bit calling convention and register sizes
use core::arch::global_asm;

global_asm!(
    ".global mouse_handler_asm",
    "mouse_handler_asm:",
    "push rax",
    "push rcx",
    "push rdx",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "push ds",
    "push es",
    "push fs",
    "push gs",
    "call mouse_handler_rust",
    "pop gs",
    "pop fs",
    "pop es",
    "pop ds",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "iretq"
);

global_asm!(
    ".global keyboard_handler_asm",
    "keyboard_handler_asm:",
    "push rax",
    "push rcx",
    "push rdx",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "push ds",
    "push es",
    "push fs",
    "push gs",
    "call keyboard_handler_rust",
    "pop gs",
    "pop fs",
    "pop es",
    "pop ds",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "iretq"
);

global_asm!(
    ".global syscall_handler_asm",
    "syscall_handler_asm:",
    "push rax",
    "push rcx",
    "push rdx",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "push ds",
    "push es",
    "push fs",
    "push gs",
    "mov rax, rsp",
    "push rax",
    "call syscall_handler_rust",
    "add rsp, 8",
    "pop gs",
    "pop fs",
    "pop es",
    "pop ds",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "iretq"
);

global_asm!(
    ".global timer_handler_asm",
    "timer_handler_asm:",
    "push rax",
    "push rcx",
    "push rdx",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "push ds",
    "push es",
    "push fs",
    "push gs",
    "mov rax, rsp",
    "push rax",
    "call timer_handler_rust",
    "add rsp, 8",
    "pop gs",
    "pop fs",
    "pop es",
    "pop ds",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "iretq"
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
        use crate::arch::x86_64::io::outb;
        outb(0xA0, 0x20);
        outb(0x20, 0x20);
    }
}

#[no_mangle]
pub extern "C" fn keyboard_handler_rust() {
    super::super::super::keyboard::handle_keyboard_interrupt();
    unsafe {
        use crate::arch::x86_64::io::outb;
        outb(0x20, 0x20);
    }
}

#[no_mangle]
pub extern "C" fn syscall_handler_rust(regs: *mut super::super::super::syscalls::SyscallRegisters) -> u64 {
    unsafe { super::super::super::syscalls::syscall_handler_rust(&mut *regs) as u64 }
}

#[no_mangle]
pub extern "C" fn timer_handler_rust(regs: *mut super::super::super::syscalls::SyscallRegisters) -> u64 {
    unsafe {
        use crate::arch::x86_64::io::outb;
        outb(0x20, 0x20);
    }
    super::super::super::scheduler::timer_tick();
    super::super::super::scheduler::schedule(regs as u32) as u64
}

