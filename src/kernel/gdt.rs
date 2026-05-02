use core::mem::size_of;
use core::arch::asm;

pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const TSS_SELECTOR: u16 = 0x28;
pub const DOUBLE_FAULT_TSS_SELECTOR: u16 = 0x30;

#[repr(C, packed)]
pub struct GdtEntry {
    pub limit_low: u16,
    pub base_low: u16,
    pub base_middle: u8,
    pub access: u8,
    pub granularity: u8,
    pub base_high: u8,
}

impl GdtEntry {
    pub const fn new(base: u32, limit: u32, access: u8, granularity: u8) -> Self {
        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            granularity: (granularity & 0xF0) | ((limit >> 16) & 0x0F) as u8,
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }
}

/// A 16-byte system descriptor used in x86_64 for TSS and LDT.
/// This occupies two slots in the GDT.
#[repr(C, packed)]
pub struct GdtSystemEntry {
    pub low: GdtEntry,
    pub base_upper32: u32,
    pub reserved: u32,
}

impl GdtSystemEntry {
    pub const fn new(base: u64, limit: u32, access: u8, granularity: u8) -> Self {
        Self {
            low: GdtEntry::new(base as u32, limit, access, granularity),
            base_upper32: (base >> 32) as u32,
            reserved: 0,
        }
    }
}

#[repr(C, packed)]
pub struct GdtPtr {
    pub limit: u16,
    pub base: usize, // Scales to 32-bit or 64-bit automatically
}

#[cfg(target_arch = "x86_64")]
#[repr(C, packed)]
pub struct TaskStateSegment {
    pub reserved0: u32,
    pub rsp: [u64; 3],
    pub reserved1: u64,
    pub ist: [u64; 7],
    pub reserved2: u64,
    pub reserved3: u16,
    pub iomap_base: u16,
}

#[cfg(target_arch = "x86")]
#[repr(C, packed)]
pub struct TaskStateSegment {
    pub prev_tss: u32,
    pub esp0: u32,
    pub ss0: u32,
    pub esp1: u32,
    pub ss1: u32,
    pub esp2: u32,
    pub ss2: u32,
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
    pub es: u32,
    pub cs: u32,
    pub ss: u32,
    pub ds: u32,
    pub fs: u32,
    pub gs: u32,
    pub ldt: u32,
    pub trap: u16,
    pub iomap_base: u16,
}

#[cfg(target_arch = "x86_64")]
impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved0: 0, rsp: [0; 3], reserved1: 0,
            ist: [0; 7], reserved2: 0, reserved3: 0,
            iomap_base: size_of::<TaskStateSegment>() as u16,
        }
    }
}

#[cfg(target_arch = "x86")]
impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            prev_tss: 0,
            esp0: 0,
            ss0: 0x10, // Kernel Data Segment
            esp1: 0, ss1: 0, esp2: 0, ss2: 0,
            cr3: 0, eip: 0, eflags: 0,
            eax: 0, ecx: 0, edx: 0, ebx: 0,
            esp: 0, ebp: 0, esi: 0, edi: 0,
            es: 0, cs: 0, ss: 0, ds: 0, fs: 0, gs: 0,
            ldt: 0, trap: 0, iomap_base: 0,
        }
    }
}

pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

// A separate, dedicated stack for the double fault handler
static mut DOUBLE_FAULT_STACK: [u8; 4096] = [0; 4096];

// A separate TSS for the double fault handler
static mut DOUBLE_FAULT_TSS: TaskStateSegment = TaskStateSegment::new();

static mut GDT: [GdtEntry; 7] = [
    // 0x00: Null
    GdtEntry::new(0, 0, 0, 0),
    // 0x08: Kernel Code (Base=0, Limit=4GB, Access=0x9A, Gran=0xCF)
    GdtEntry::new(0, 0xFFFFF, 0x9A, 0xCF),
    // 0x10: Kernel Data (Base=0, Limit=4GB, Access=0x92, Gran=0xCF)
    GdtEntry::new(0, 0xFFFFF, 0x92, 0xCF),
    // 0x18: User Code (Base=0, Limit=4GB, Access=0xFA, Gran=0xCF)
    GdtEntry::new(0, 0xFFFFF, 0xFA, 0xCF),
    // 0x20: User Data (Base=0, Limit=4GB, Access=0xF2, Gran=0xCF)
    GdtEntry::new(0, 0xFFFFF, 0xF2, 0xCF),
    // 0x28: TSS (to be filled in init)
    GdtEntry::new(0, 0, 0, 0),
    // 0x30: Double Fault TSS (to be filled in init)
    GdtEntry::new(0, 0, 0, 0),
];

extern "C" {
    fn double_fault_task_handler();
}

pub fn init() {
    unsafe {
        // --- Main TSS setup ---
        let tss_base = core::ptr::addr_of!(TSS) as u32;
        let tss_limit = size_of::<TaskStateSegment>() as u32 - 1;
        // 0x89 = Present, Ring0, 32-bit TSS Available
        GDT[5] = GdtEntry::new(tss_base, tss_limit, 0x89, 0x00);

        // --- Double Fault TSS setup ---
        let df_tss_base = core::ptr::addr_of!(DOUBLE_FAULT_TSS) as u32;
        let df_tss_limit = size_of::<TaskStateSegment>() as u32 - 1;
        GDT[6] = GdtEntry::new(df_tss_base, df_tss_limit, 0x89, 0x00);

        // Populate the Double Fault TSS
        DOUBLE_FAULT_TSS.eip = double_fault_task_handler as *const () as u32;
        DOUBLE_FAULT_TSS.esp = (core::ptr::addr_of!(DOUBLE_FAULT_STACK) as usize + 4096) as u32;
        DOUBLE_FAULT_TSS.ss0 = KERNEL_DATA_SELECTOR as u32;
        DOUBLE_FAULT_TSS.cs = KERNEL_CODE_SELECTOR as u32;
        DOUBLE_FAULT_TSS.ds = KERNEL_DATA_SELECTOR as u32;
        DOUBLE_FAULT_TSS.es = KERNEL_DATA_SELECTOR as u32;
        DOUBLE_FAULT_TSS.fs = KERNEL_DATA_SELECTOR as u32;
        DOUBLE_FAULT_TSS.gs = KERNEL_DATA_SELECTOR as u32;
        DOUBLE_FAULT_TSS.ss = KERNEL_DATA_SELECTOR as u32;

        let gdt_ptr = GdtPtr {
            limit: (size_of::<[GdtEntry; 7]>() - 1) as u16,
            base: core::ptr::addr_of!(GDT) as usize,
        };

        asm!("lgdt [{}]", in(reg) &gdt_ptr, options(readonly, nostack, preserves_flags));
        
        // Reload CS and Data Segments
        // 0x08 is Kernel Code, 0x10 is Kernel Data
        asm!(
            "push {code_sel}",
            "lea eax, [2f]",
            "push eax",
            "retf",
            "2:",
            "mov ax, {data_sel}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            code_sel = const KERNEL_CODE_SELECTOR,
            data_sel = const KERNEL_DATA_SELECTOR,
            out("eax") _,
        );

        // Load Task Register
        asm!("ltr ax", in("ax") TSS_SELECTOR);
    }
}

/// Updates the ESP0 stack pointer in the TSS.
/// This determines the kernel stack used when an interrupt occurs in Ring 3.
pub fn set_interrupt_stack(esp0: u32) {
    unsafe { TSS.esp0 = esp0; }
}