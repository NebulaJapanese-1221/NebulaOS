use core::arch::asm;

#[repr(C, packed)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

#[repr(C, packed)]
struct GdtPtr {
    limit: u16,
    base: u32,
}

#[repr(C, packed)]
pub struct Tss {
    prev_tss: u32,
    esp0: u32, ss0: u32,
    esp1: u32, ss1: u32,
    esp2: u32, ss2: u32,
    cr3: u32, eip: u32, eflags: u32,
    eax: u32, ecx: u32, edx: u32, ebx: u32, esp: u32, ebp: u32, esi: u32, edi: u32,
    es: u32, cs: u32, ss: u32, ds: u32, fs: u32, gs: u32,
    ldt: u32, trap: u16, iomap_base: u16,
}

static mut TSS: Tss = Tss {
    prev_tss: 0, esp0: 0, ss0: 0x10, // ss0 is Kernel Data
    esp1: 0, ss1: 0, esp2: 0, ss2: 0,
    cr3: 0, eip: 0, eflags: 0, eax: 0, ecx: 0, edx: 0, ebx: 0, esp: 0, ebp: 0, esi: 0, edi: 0,
    es: 0, cs: 0, ss: 0, ds: 0, fs: 0, gs: 0, ldt: 0, trap: 0, iomap_base: 104,
};

static mut GDT: [GdtEntry; 6] = [
    GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 },
    GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 },
    GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 },
    GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 },
    GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 },
    GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 },
];

static mut GDT_PTR: GdtPtr = GdtPtr { limit: 0, base: 0 };

pub fn init() {
    unsafe {
        // 1. Null Descriptor
        GDT[0] = GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 };

        // 2. Kernel Code Segment (0x08): Base 0, Limit 0xFFFFF, Access 0x9A (P=1, DPL=0, S=1, Type=Code), Granularity 0xCF (G=1, DB=1, Limit_High=0xF)
        GDT[1] = GdtEntry { 
            limit_low: 0xFFFF, base_low: 0, base_middle: 0, 
            access: 0x9A, granularity: 0xCF, base_high: 0 
        };

        // 3. Kernel Data Segment (0x10): Base 0, Limit 0xFFFFF, Access 0x92 (P=1, DPL=0, S=1, Type=Data), Granularity 0xCF
        GDT[2] = GdtEntry { 
            limit_low: 0xFFFF, base_low: 0, base_middle: 0, 
            access: 0x92, granularity: 0xCF, base_high: 0 
        };

        // 4. User Code Segment (Index 3, Selector 0x1B): Base 0, Limit 0xFFFFF, Access 0xFA (P=1, DPL=3, S=1, Type=Code), Granularity 0xCF
        GDT[3] = GdtEntry { 
            limit_low: 0xFFFF, base_low: 0, base_middle: 0, 
            access: 0xFA, granularity: 0xCF, base_high: 0 
        };

        // 5. User Data Segment (Index 4, Selector 0x23): Base 0, Limit 0xFFFFF, Access 0xF2 (P=1, DPL=3, S=1, Type=Data), Granularity 0xCF
        GDT[4] = GdtEntry { 
            limit_low: 0xFFFF, base_low: 0, base_middle: 0, 
            access: 0xF2, granularity: 0xCF, base_high: 0 
        };

        // 6. TSS Descriptor (Index 5, Selector 0x28): Access 0x89 (P=1, DPL=0, Type=32-bit TSS available)
        let tss_base = &raw const TSS as u32;
        let tss_limit = (core::mem::size_of::<Tss>() - 1) as u32;
        GDT[5] = GdtEntry {
            limit_low: (tss_limit & 0xFFFF) as u16,
            base_low: (tss_base & 0xFFFF) as u16,
            base_middle: ((tss_base >> 16) & 0xFF) as u8,
            access: 0x89,
            granularity: ((tss_limit >> 16) & 0x0F) as u8,
            base_high: ((tss_base >> 24) & 0xFF) as u8,
        };

        GDT_PTR.limit = (core::mem::size_of::<[GdtEntry; 6]>() - 1) as u16;
        GDT_PTR.base = &raw const GDT as u32;

        // Initialize initial kernel stack for interrupts
        let stack_ptr: u32;
        asm!("mov {}, esp", out(reg) stack_ptr);
        TSS.esp0 = stack_ptr;

        // Load the GDT
        asm!("lgdt [{}]", in(reg) &raw const GDT_PTR);

        // Reload segment registers. We use a far return (retf) to update CS.
        asm!(
            "push 0x08",        // Push the new code segment selector
            "lea eax, [2f]",    // Get address of the next label
            "push eax",         // Push target instruction pointer
            "retf",             // Far return: pops IP then CS
            "2:",               // Target label
            "mov ax, 0x10",     // Update data segment registers
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            out("eax") _,       // Clobber eax
            options(nostack)
        );

        // Load the Task Register (Selector 0x28)
        asm!("ltr ax", in("ax") 0x28u16);
    }
}

#[allow(dead_code)]
pub fn set_kernel_stack(addr: u32) {
    unsafe {
        TSS.esp0 = addr;
    }
}