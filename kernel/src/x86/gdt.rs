use crate::common::stdint::*;
use crate::common::io;
use core::arch::global_asm;

static mut GDT: [GdtEntry; 256] = [GdtEntry::zero(); 256];
static mut GDT_PTR: GdtPtr = GdtPtr::zero();

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

impl GdtEntry {
    const fn zero() -> Self {
        GdtEntry { limit_low: 0, base_low: 0, base_mid: 0, access: 0, limit_high: 0, base_high: 0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GdtPtr {
    pub limit: u16,
    pub base: u32,
}

impl GdtPtr {
    const fn zero() -> Self {
        GdtPtr { limit: 0, base: 0 }
    }
}

pub const GDT_ACCESS_PRESENT: u8 = 0x80;
pub const GDT_ACCESS_PRIV_0: u8 = 0x00;
pub const GDT_ACCESS_PRIV_3: u8 = 0x60;
pub const GDT_ACCESS_TYPE_CODE: u8 = 0x08;
pub const GDT_ACCESS_TYPE_DATA: u8 = 0x00;
pub const GDT_ACCESS_READABLE: u8 = 0x02;
pub const GDT_ACCESS_WRITABLE: u8 = 0x02;
pub const GDT_FLAGS_GRANULARITY: u8 = 0x80;
pub const GDT_FLAGS_32BIT: u8 = 0x40;

#[no_mangle]
pub unsafe extern "C" fn init_gdt() {
    GDT_PTR.limit = (core::mem::size_of::<[GdtEntry; 256]>() - 1) as u16;
    GDT_PTR.base = &raw mut GDT as *mut _ as u32;

    set_gate(0, 0, 0, 0, 0);
    set_gate(1, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    set_gate(2, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    set_gate(3, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    set_gate(4, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);

    gdt_flush();
}

unsafe fn set_gate(num: i32, base: u32, limit: u32, access: u8, flags: u8) {
    GDT[num as usize].base_low = (base & 0xFFFF) as u16;
    GDT[num as usize].base_mid = ((base >> 16) & 0xFF) as u8;
    GDT[num as usize].base_high = ((base >> 24) & 0xFF) as u8;
    GDT[num as usize].limit_low = (limit & 0xFFFF) as u16;
    GDT[num as usize].limit_high = (flags & 0xF0) | ((limit >> 16) & 0x0F) as u8;
    GDT[num as usize].access = access;
}

unsafe extern "C" {
    fn gdt_flush();
}

global_asm!(
    ".att_syntax",
    ".global gdt_flush",
    ".type gdt_flush, @function",
    "gdt_flush:",
    "lgdt [GDT_PTR]",
    "mov $0x10, %eax",
    "mov %eax, %ds",
    "mov %eax, %es",
    "mov %eax, %fs",
    "mov %eax, %gs",
    "mov %eax, %ss",
    "ljmp $0x08, $1f",
    "1:",
    "ret"
);
