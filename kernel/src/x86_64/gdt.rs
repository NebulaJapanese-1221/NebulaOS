use crate::common::stdint::*;
use crate::common::io;

static mut GDT: [GdtEntry; 6] = [GdtEntry::zero(); 6];
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
    pub base: u64,
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
pub const GDT_FLAGS_64BIT: u8 = 0x20;

#[no_mangle]
pub unsafe extern "C" fn init_gdt64() {
    GDT_PTR.limit = (core::mem::size_of::<[GdtEntry; 6]>() - 1) as u16;
    GDT_PTR.base = &mut GDT as *mut _ as u64;

    set_gate(0, 0, 0, 0, 0);
    set_gate(1, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_64BIT);
    set_gate(2, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY);
    set_gate(3, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_64BIT);
    set_gate(4, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY);

    gdt_flush64();
}

unsafe fn set_gate(num: i32, base: u64, limit: u64, access: u8, flags: u8) {
    GDT[num as usize].base_low = (base & 0xFFFF) as u16;
    GDT[num as usize].base_mid = ((base >> 16) & 0xFF) as u8;
    GDT[num as usize].base_high = ((base >> 24) & 0xFF) as u8;
    GDT[num as usize].limit_low = (limit & 0xFFFF) as u16;
    GDT[num as usize].limit_high = ((flags & 0xF0) | ((limit >> 16) & 0x0F)) as u8;
    GDT[num as usize].access = access;
}

#[naked]
unsafe fn gdt_flush64() {
    asm!("lgdt [{0}]", in(reg) &GDT_PTR, options(nomem, nostack));
    asm!("mov $0x10, %rax", out("rax") _);
    asm!("mov %rax, %ds", out("rax") _);
    asm!("mov %rax, %es", out("rax") _);
    asm!("mov %rax, %fs", out("rax") _);
    asm!("mov %rax, %gs", out("rax") _);
    asm!("mov %rax, %ss", out("rax") _);
    asm!("pushq $0x08");
    asm!("lea .lgdt_return(%rip), %rax", out("rax") _);
    asm!("pushq %rax");
    asm!("lretq");
    asm!(".lgdt_return:", options(nomem, nostack));
}
