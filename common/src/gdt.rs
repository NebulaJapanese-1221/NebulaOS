#![allow(unused_imports)]
use core::arch::asm;
use crate::nebula;
use crate::PACKED;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GdtEntry {
    pub limit_low: u16,
    pub base_low: u16,
    pub base_mid: u8,
    pub access: u8,
    pub limit_high: u8,
    pub base_high: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GdtPtr {
    pub limit: u16,
    pub base: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Tss {
    pub prev_tss: u16,
    pub reserved1: u16,
    pub esp0: u32,
    pub ss0: u16,
    pub reserved2: u16,
    pub esp1: u32,
    pub ss1: u16,
    pub reserved3: u16,
    pub esp2: u32,
    pub ss2: u16,
    pub reserved4: u16,
    pub cr3: u32,
    pub eip: u32,
    pub eflags: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub es: u16,
    pub reserved5: u16,
    pub cs: u16,
    pub reserved6: u16,
    pub ss: u16,
    pub reserved7: u16,
    pub ds: u16,
    pub reserved8: u16,
    pub fs: u16,
    pub reserved9: u16,
    pub gs: u16,
    pub reserved10: u16,
    pub ldt: u16,
    pub reserved11: u16,
    pub trap: u16,
    pub iomap_base: u16,
}

pub const GDT_NULL_SEL: u16 = 0x00;
pub const GDT_CODE_SEL: u16 = 0x08;
pub const GDT_DATA_SEL: u16 = 0x10;
pub const GDT_USER_CODE_SEL: u16 = 0x18;
pub const GDT_USER_DATA_SEL: u16 = 0x20;
pub const GDT_TSS_SEL: u16 = 0x28;

pub const GDT_ACCESS_PRESENT: u8 = 0x80;
pub const GDT_ACCESS_PRIV_0: u8 = 0x00;
pub const GDT_ACCESS_PRIV_3: u8 = 0x60;
pub const GDT_ACCESS_TYPE_CODE: u8 = 0x08;
pub const GDT_ACCESS_TYPE_DATA: u8 = 0x00;
pub const GDT_ACCESS_CONFORMING: u8 = 0x04;
pub const GDT_ACCESS_EXPAND_DOWN: u8 = 0x04;
pub const GDT_ACCESS_READABLE: u8 = 0x02;
pub const GDT_ACCESS_WRITABLE: u8 = 0x02;
pub const GDT_ACCESS_ACCESSED: u8 = 0x01;

pub const GDT_FLAGS_GRANULARITY: u8 = 0x80;
pub const GDT_FLAGS_32BIT: u8 = 0x40;
pub const GDT_FLAGS_64BIT: u8 = 0x20;
pub const GDT_FLAGS_AVL: u8 = 0x10;

#[cfg(target_arch = "x86")]
pub unsafe fn init_gdt() {
    // x86 GDT initialization is in kernel/src/x86/gdt.rs
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn init_gdt() {
    // x86_64 GDT initialization is in kernel/src/x86_64/gdt.rs
}

#[cfg(target_arch = "x86")]
pub unsafe fn set_gate(num: i32, base: u32, limit: u32, access: u8, flags: u8) {
    // x86 implementation in kernel/src/x86/gdt.rs
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn set_gate(num: i32, base: u64, limit: u64, access: u8, flags: u8) {
    // x86_64 implementation in kernel/src/x86_64/gdt.rs
}
