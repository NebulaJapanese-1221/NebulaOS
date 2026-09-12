use crate::common::stdint::*;
use crate::common::io;

static mut GDT: [GdtEntry; 7] = [GdtEntry::zero(); 7];
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

#[repr(C, packed)]
pub struct Tss {
    reserved0: u32,
    rsp: [u64; 3],
    reserved1: u64,
    ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    iomap_base: u16,
}

static mut TSS: Tss = Tss {
    reserved0: 0,
    rsp: [0, 0, 0],
    reserved1: 0,
    ist: [0, 0, 0, 0, 0, 0, 0],
    reserved2: 0,
    reserved3: 0,
    iomap_base: 0,
};

static mut IST_STACKS: [[u8; 4096]; 4] = [[0; 4096]; 4];

pub const GDT_ACCESS_PRESENT: u8 = 0x80;
pub const GDT_ACCESS_PRIV_0: u8 = 0x00;
pub const GDT_ACCESS_PRIV_3: u8 = 0x60;
pub const GDT_ACCESS_TYPE_CODE: u8 = 0x08;
pub const GDT_ACCESS_TYPE_DATA: u8 = 0x00;
pub const GDT_ACCESS_READABLE: u8 = 0x02;
pub const GDT_ACCESS_WRITABLE: u8 = 0x02;
pub const GDT_ACCESS_TSS_AVAILABLE: u8 = 0x09;
pub const GDT_FLAGS_GRANULARITY: u8 = 0x80;
pub const GDT_FLAGS_64BIT: u8 = 0x20;

#[no_mangle]
pub unsafe extern "C" fn init_gdt64() {
    GDT_PTR.limit = (core::mem::size_of::<[GdtEntry; 7]>() - 1) as u16;
    GDT_PTR.base = &mut GDT as *mut _ as u64;

    // Initialize IST stacks
    IST_STACKS[0].fill(0);  // IST1 - Double fault
    IST_STACKS[1].fill(0);  // IST2 - GPF
    IST_STACKS[2].fill(0);  // IST3 - Page fault
    IST_STACKS[3].fill(0);  // IST4 - Reserved
    
    // Set up TSS IST pointers
    TSS.ist[0] = &IST_STACKS[0] as *const _ as u64 + 4096;
    TSS.ist[1] = &IST_STACKS[1] as *const _ as u64 + 4096;
    TSS.ist[2] = &IST_STACKS[2] as *const _ as u64 + 4096;
    TSS.ist[3] = &IST_STACKS[3] as *const _ as u64 + 4096;

    set_gate(0, 0, 0, 0, 0);
    set_gate(1, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_64BIT);
    set_gate(2, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY);
    set_gate(3, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE, GDT_FLAGS_GRANULARITY | GDT_FLAGS_64BIT);
    set_gate(4, 0, 0, GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE, GDT_FLAGS_GRANULARITY);
    
    // TSS descriptor (index 5, selector 0x28)
    let tss_base = &mut TSS as *mut _ as u64;
    let tss_limit = core::mem::size_of::<Tss>() as u64 - 1;
    set_gate_tss(5, tss_base, tss_limit);

    gdt_flush64();
    
    // Load TR (Task Register)
    asm!("ltr ax", in("ax") 0x28u16, options(nomem, nostack));
}

unsafe fn set_gate(num: i32, base: u64, limit: u64, access: u8, flags: u8) {
    GDT[num as usize].base_low = (base & 0xFFFF) as u16;
    GDT[num as usize].base_mid = ((base >> 16) & 0xFF) as u8;
    GDT[num as usize].base_high = ((base >> 24) & 0xFF) as u8;
    GDT[num as usize].limit_low = (limit & 0xFFFF) as u16;
    GDT[num as usize].limit_high = ((flags & 0xF0) | ((limit >> 16) & 0x0F)) as u8;
    GDT[num as usize].access = access;
}

unsafe fn set_gate_tss(num: i32, base: u64, limit: u64) {
    GDT[num as usize].base_low = (base & 0xFFFF) as u16;
    GDT[num as usize].base_mid = ((base >> 16) & 0xFF) as u8;
    GDT[num as usize].base_high = ((base >> 24) & 0xFF) as u8;
    GDT[num as usize].limit_low = (limit & 0xFFFF) as u16;
    GDT[num as usize].limit_high = ((limit >> 16) & 0x0F) as u8;
    GDT[num as usize].access = GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TSS_AVAILABLE;
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
