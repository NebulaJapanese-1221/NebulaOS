use crate::idt;
use crate::serial_println;
use core::arch::global_asm;

global_asm!(
    ".macro exception_stub name, handler",
    ".global \\name",
    "\\name:",
    "pushal",
    "push ds", "push es", "push fs", "push gs",
    "mov eax, esp",
    "push eax",
    "call \\handler",
    "add esp, 4",
    "pop gs", "pop fs", "pop es", "pop ds",
    "popal",
    "iretd",
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
    // Register common CPU exception handlers
    idt::set_gate(0, divide_by_zero_asm as u32, 0x08, 0x8E);
    idt::set_gate(6, invalid_opcode_asm as u32, 0x08, 0x8E);
    idt::set_gate(13, gpf_asm as u32, 0x08, 0x8E);
    idt::set_gate(14, page_fault_asm as u32, 0x08, 0x8E);
}

#[no_mangle]
pub extern "C" fn divide_by_zero_handler(regs: &crate::syscalls::SyscallRegisters) {
    let eip = regs.eip;
    serial_println!("DIVIDE BY ZERO at EIP: 0x{:08x} (User: {})", eip, regs.is_user());
    panic!("CPU EXCEPTION: Divide by Zero");
}

#[no_mangle]
pub extern "C" fn invalid_opcode_handler(regs: &crate::syscalls::SyscallRegisters) {
    let eip = regs.eip;
    serial_println!("INVALID OPCODE at EIP: 0x{:08x} (User: {})", eip, regs.is_user());
    panic!("CPU EXCEPTION: Invalid Opcode");
}

#[no_mangle]
pub extern "C" fn gpf_handler(regs: &crate::syscalls::SyscallRegisters) {
    let eip = regs.eip;
    serial_println!("GPF at EIP: 0x{:08x} (User: {})", eip, regs.is_user());
    panic!("CPU EXCEPTION: General Protection Fault");
}

#[no_mangle]
pub extern "C" fn page_fault_handler(regs: &crate::syscalls::SyscallRegisters) {
    let eip = regs.eip;
    serial_println!("PAGE FAULT at EIP: 0x{:08x} (User: {})", eip, regs.is_user());
    panic!("CPU EXCEPTION: Page Fault");
}