// x86_64 (64-bit) CPU exception handlers
use core::arch::global_asm;

global_asm!(
    ".macro exception_stub name, handler",
    ".global \\name",
    "\\name:",
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
    "call \\handler",
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
    "iretq",
    ".endm",

    "exception_stub divide_by_zero_asm, divide_by_zero_handler",
    "exception_stub invalid_opcode_asm, invalid_opcode_handler",
    "exception_stub gpf_asm, gpf_handler",
    "exception_stub page_fault_asm, page_fault_handler"
);

extern "C" {
    fn divide_by_zero_asm();
    fn invalid_opcode_asm();
    fn gpf_asm();
    fn page_fault_asm();
}

pub unsafe fn init() {
    use crate::arch::x86_64::idt;
    // In x86_64, handler addresses are 64-bit, stored across two 32-bit halves
    let divide_base = divide_by_zero_asm as *const () as u64;
    let invalid_base = invalid_opcode_asm as *const () as u64;
    let gpf_base = gpf_asm as *const () as u64;
    let pf_base = page_fault_asm as *const () as u64;

    idt::set_gate(0, divide_base, 0x08, 0x8E);
    idt::set_gate(6, invalid_base, 0x08, 0x8E);
    idt::set_gate(13, gpf_base, 0x08, 0x8E);
    idt::set_gate(14, pf_base, 0x08, 0x8E);
}

#[no_mangle]
pub extern "C" fn divide_by_zero_handler(regs: &crate::syscalls::SyscallRegisters) {
    let rip = regs.rip;
    crate::serial_println!("DIVIDE BY ZERO at RIP: 0x{:016x} (User: {})", rip, regs.is_user());
    panic!("CPU EXCEPTION: Divide by Zero");
}

#[no_mangle]
pub extern "C" fn invalid_opcode_handler(regs: &crate::syscalls::SyscallRegisters) {
    let rip = regs.rip;
    crate::serial_println!("INVALID OPCODE at RIP: 0x{:016x} (User: {})", rip, regs.is_user());
    panic!("CPU EXCEPTION: Invalid Opcode");
}

#[no_mangle]
pub extern "C" fn gpf_handler(regs: &crate::syscalls::SyscallRegisters) {
    let rip = regs.rip;
    crate::serial_println!("GPF at RIP: 0x{:016x} (User: {})", rip, regs.is_user());
    panic!("CPU EXCEPTION: General Protection Fault");
}

#[no_mangle]
pub extern "C" fn page_fault_handler(regs: &crate::syscalls::SyscallRegisters) {
    let rip = regs.rip;
    crate::serial_println!("PAGE FAULT at RIP: 0x{:016x} (User: {})", rip, regs.is_user());
    panic!("CPU EXCEPTION: Page Fault");
}

