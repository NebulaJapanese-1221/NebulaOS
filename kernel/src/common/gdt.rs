use crate::common::stdint::{u32, u8};
use crate::common::nebula::{self, PACKED};

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

static mut GDT: [GdtEntry; 256] = [GdtEntry { limit_low: 0, base_low: 0, base_mid: 0, access: 0, limit_high: 0, base_high: 0 }; 256];
static mut GDT_PTR: GdtPtr = GdtPtr { limit: 0, base: 0 };
static mut TSS: Tss = Tss { prev_tss: 0, reserved1: 0, esp0: 0, ss0: 0, reserved2: 0, esp1: 0, ss1: 0, reserved3: 0, esp2: 0, ss2: 0, reserved4: 0, cr3: 0, eip: 0, eflags: 0, eax: 0, ecx: 0, edx: 0, ebx: 0, esp: 0, ebp: 0, esi: 0, edi: 0, es: 0, reserved5: 0, cs: 0, reserved6: 0, ss: 0, reserved7: 0, ds: 0, reserved8: 0, fs: 0, reserved9: 0, gs: 0, reserved10: 0, ldt: 0, reserved11: 0, trap: 0, iomap_base: 0 };

pub unsafe fn init_gdt() {
    GDT_PTR.limit = (core::mem::size_of::<[GdtEntry; 256]>() - 1) as u16;
    GDT_PTR.base = &mut GDT as *mut _ as u32;

    set_gate(0, 0, 0, 0, 0);
    set_gate(1, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    set_gate(2, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    set_gate(3, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    set_gate(4, 0, 0xFFFFFFFF, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);

    gdt_flush();
}

pub unsafe fn set_gate(num: i32, base: u32, limit: u32, access: u8, flags: u8) {
    GDT[num as usize].base_low = (base & 0xFFFF) as u16;
    GDT[num as usize].base_mid = ((base >> 16) & 0xFF) as u8;
    GDT[num as usize].base_high = ((base >> 24) & 0xFF) as u8;
    GDT[num as usize].limit_low = (limit & 0xFFFF) as u16;
    GDT[num as usize].limit_high = ((flags & 0xF0) | ((limit >> 16) & 0x0F)) as u8;
    GDT[num as usize].access = access;
}

#[naked]
pub unsafe fn gdt_flush() {
    unsafe {
        asm!("lgdt [{0}]", in(reg) &GDT_PTR, options(nomem, nostack));
        asm!("mov $0x10, %eax", out("eax") _);
        asm!("mov %eax, %ds", out("eax") _);
        asm!("mov %eax, %es", out("eax") _);
        asm!("mov %eax, %fs", out("eax") _);
        asm!("mov %eax, %gs", out("eax") _);
        asm!("mov %eax, %ss", out("eax") _);
        asm!("ljmp $0x08, $1f", out("eax") _);
        asm!("1:", options(nomem, nostack));
    }
}
