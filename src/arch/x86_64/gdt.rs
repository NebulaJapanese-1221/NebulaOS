// x86_64 (64-bit) GDT implementation
// 64-bit long mode requires a different GDT layout

use core::arch::asm;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

#[repr(C, packed(2))]
struct GdtPtr {
    limit: u16,
    base: u64,
}

/// AMD64 TSS structure
#[repr(C, packed)]
pub struct Tss {
    reserved1: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    reserved2: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved3: u64,
    reserved4: u16,
    iomap_base: u16,
}

impl Tss {
    pub const fn new() -> Self {
        Tss {
            reserved1: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved2: 0,
            ist1: 0, ist2: 0, ist3: 0, ist4: 0,
            ist5: 0, ist6: 0, ist7: 0,
            reserved3: 0,
            reserved4: 0,
            iomap_base: 104,
        }
    }
}

static mut TSS: Tss = Tss::new();

// GDT with typical entries for long mode:
// Index 0: Null
// Index 1: Kernel Code (64-bit)
// Index 2: Kernel Data
// Index 3: User Code (64-bit)
// Index 4: User Data
// Index 5: TSS descriptor (16 bytes, spanning 2 entries)
static mut GDT: [GdtEntry; 7] = [GdtEntry {
    limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0,
}; 7];

static mut GDT_PTR: GdtPtr = GdtPtr { limit: 0, base: 0 };

pub unsafe fn init() {
    // 1. Null descriptor
    GDT[0] = GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 };

    // 2. Kernel Code (0x08) - 64-bit code segment
    // Access: Present, Ring 0, Code, Execute/Read
    // Granularity: Long mode, 64-bit
    GDT[1] = GdtEntry {
        limit_low: 0, base_low: 0, base_middle: 0,
        access: 0x9A, // P=1, DPL=0, S=1, Type=Code (Execute/Read)
        granularity: 0x20, // L=1 (Long mode), D/B=0
        base_high: 0,
    };

    // 3. Kernel Data (0x10)
    GDT[2] = GdtEntry {
        limit_low: 0xFFFF, base_low: 0, base_middle: 0,
        access: 0x92, // P=1, DPL=0, S=1, Type=Data (Read/Write)
        granularity: 0xCF,
        base_high: 0,
    };

    // 4. User Code (0x1B) - 64-bit code segment
    GDT[3] = GdtEntry {
        limit_low: 0, base_low: 0, base_middle: 0,
        access: 0xFA, // P=1, DPL=3, S=1, Type=Code (Execute/Read)
        granularity: 0x20, // L=1
        base_high: 0,
    };

    // 5. User Data (0x23)
    GDT[4] = GdtEntry {
        limit_low: 0xFFFF, base_low: 0, base_middle: 0,
        access: 0xF2, // P=1, DPL=3, S=1, Type=Data (Read/Write)
        granularity: 0xCF,
        base_high: 0,
    };

    // 6-7. TSS Descriptor (0x28, uses 2 entries = 16 bytes)
    let tss_base = &raw const TSS as u64;
    let tss_limit = (core::mem::size_of::<Tss>() - 1) as u32;

    // First 8 bytes of TSS descriptor
    GDT[5] = GdtEntry {
        limit_low: (tss_limit & 0xFFFF) as u16,
        base_low: (tss_base & 0xFFFF) as u16,
        base_middle: ((tss_base >> 16) & 0xFF) as u8,
        access: 0x89, // Present, Ring 0, TSS (Available)
        granularity: ((tss_limit >> 16) & 0x0F) as u8 | 0x00, // 1-byte granular
        base_high: ((tss_base >> 24) & 0xFF) as u8,
    };

    // Second 8 bytes of TSS descriptor (bits 32-63 of base and upper bits)
    // For GDT[6], we encode the upper 32 bits of the base
    // In standard format, this is stored as a separate 8-byte entry
    GDT[6] = GdtEntry {
        limit_low: ((tss_base >> 32) & 0xFFFF) as u16,
        base_low: ((tss_base >> 48) & 0xFFFF) as u16,
        base_middle: 0,
        access: 0,
        granularity: 0,
        base_high: 0,
    };

    GDT_PTR.limit = (core::mem::size_of::<[GdtEntry; 7]>() - 1) as u16;
    GDT_PTR.base = &raw const GDT as u64;

    let stack_ptr: u64;
    asm!("mov {}, rsp", out(reg) stack_ptr);
    TSS.rsp0 = stack_ptr;

    // Load GDT (64-bit version using lgdt with memory operand)
    asm!("lgdt [{}]", in(reg) &raw const GDT_PTR);

    // Reload segment registers
    asm!(
        "push 0x08",
        "lea rax, [2f + rip]",
        "push rax",
        "retfq",
        "2:",
        "mov ax, 0x10",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        out("rax") _,
        options(nostack)
    );

    // Load TSS via LTR
    asm!("ltr ax", in("ax") 0x28u16);
}

pub fn set_kernel_stack(addr: u64) {
    unsafe { TSS.rsp0 = addr; }
}

pub fn get_kernel_stack() -> u64 {
    unsafe { TSS.rsp0 }
}

