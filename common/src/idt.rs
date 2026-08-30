#![allow(unused_imports)]
use core::arch::asm;
use crate::stdint::*;
use crate::io;

pub const IDT_MAX_DESCRIPTORS: usize = 256;
pub const IRQ0: u32 = 32;
pub const IRQ8: u32 = 40;
pub const IRQ15: u32 = 47;

pub const IDT_FLAG_PRESENT: u8 = 0x80;
pub const IDT_FLAG_INTERRUPT: u8 = 0x0E;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    pub base_low: u16,
    pub sel: u16,
    pub ist: u8,
    pub flags: u8,
    pub base_mid: u16,
    pub base_high: u32,
    pub reserved: u32,
}

#[repr(C, packed)]
pub struct IdtPtr {
    pub limit: u16,
    pub base: u32,
}

pub type InterruptHandler = unsafe extern "C" fn(*mut Registers);

#[repr(C)]
pub struct Registers {
    pub int_no: u32,
    pub error_code: u32,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

pub static mut IDT: [IdtEntry; IDT_MAX_DESCRIPTORS] = [IdtEntry { base_low: 0, sel: 0, ist: 0, flags: 0, base_mid: 0, base_high: 0, reserved: 0 }; IDT_MAX_DESCRIPTORS];
pub static mut IDT_PTR: IdtPtr = IdtPtr { limit: 0, base: 0 };
pub static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; IDT_MAX_DESCRIPTORS] = [None; IDT_MAX_DESCRIPTORS];

extern "C" {
    static mut isr_table: [unsafe extern "C" fn(); 32];
    static mut irq_table: [unsafe extern "C" fn(); 16];
}

pub unsafe fn init_idt64() {
    IDT_PTR.limit = (core::mem::size_of::<[IdtEntry; IDT_MAX_DESCRIPTORS]>() - 1) as u16;
    IDT_PTR.base = &mut IDT as *mut _ as u32;

    for i in 0..IDT_MAX_DESCRIPTORS {
        INTERRUPT_HANDLERS[i] = Some(default_interrupt_handler);
    }

    for i in 0..32 {
        set_gate64(i, isr_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }
    for i in 0..16 {
        set_gate64((IRQ0 as usize) + i, irq_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }
    for i in 48..IDT_MAX_DESCRIPTORS {
        set_gate64(i, default_interrupt_handler as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }

    idt_flush64();
}

pub unsafe fn set_gate64(num: usize, base: u64, sel: u16, ist: u8, flags: u8) {
    IDT[num].base_low = (base & 0xFFFF) as u16;
    IDT[num].sel = sel;
    IDT[num].ist = ist;
    IDT[num].flags = flags;
    IDT[num].base_mid = ((base >> 16) & 0xFFFF) as u16;
    IDT[num].base_high = ((base >> 32) & 0xFFFF) as u32;
    IDT[num].reserved = 0;
}

pub unsafe fn idt_flush64() {
    asm!("lidt [{0}]", in(reg) &IDT_PTR, options(nomem, nostack));
}

pub unsafe fn register_interrupt_handler(n: usize, handler: Option<InterruptHandler>) {
    if n < IDT_MAX_DESCRIPTORS {
        INTERRUPT_HANDLERS[n] = handler;
    }
}

pub unsafe fn interrupt_handler(regs: *mut Registers) {
    let int_no = (*regs).int_no as usize;
    if int_no < IDT_MAX_DESCRIPTORS {
        if let Some(handler) = INTERRUPT_HANDLERS[int_no] {
            handler(regs);
        }
    }
}

pub unsafe extern "C" fn default_interrupt_handler(regs: *mut Registers) {
    let int_no = (*regs).int_no;
    if int_no >= IRQ0 as u32 && int_no <= IRQ15 as u32 {
        io::outb(0x20, 0x20);
        if int_no >= IRQ8 as u32 {
            io::outb(0xA0, 0x20);
        }
    }
}
