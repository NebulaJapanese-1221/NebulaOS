// Task State Segment for 32-bit protected mode

#[repr(C, packed)]
pub struct Tss {
    pub prev_task: u16,
    _reserved0: u16,
    pub esp0: u32,
    pub ss0: u16,
    _reserved1: u16,
    pub esp1: u32,
    pub ss1: u16,
    _reserved2: u16,
    pub esp2: u32,
    pub ss2: u16,
    _reserved3: u16,
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
    _reserved4: u16,
    pub cs: u16,
    _reserved5: u16,
    pub ss: u16,
    _reserved6: u16,
    pub ds: u16,
    _reserved7: u16,
    pub fs: u16,
    _reserved8: u16,
    pub gs: u16,
    _reserved9: u16,
    pub ldt: u16,
    _reserved10: u16,
    pub iopb_offset: u16,
}

static mut TSS: Tss = Tss {
    prev_task: 0,
    _reserved0: 0,
    esp0: 0,
    ss0: super::gdt::kernel_data_selector(),
    _reserved1: 0,
    esp1: 0,
    ss1: 0,
    _reserved2: 0,
    esp2: 0,
    ss2: 0,
    _reserved3: 0,
    cr3: 0,
    eip: 0,
    eflags: 0x00000202,
    eax: 0,
    ecx: 0,
    edx: 0,
    ebx: 0,
    esp: 0,
    ebp: 0,
    esi: 0,
    edi: 0,
    es: super::gdt::kernel_data_selector(),
    _reserved4: 0,
    cs: super::gdt::kernel_code_selector(),
    _reserved5: 0,
    ss: super::gdt::kernel_data_selector(),
    _reserved6: 0,
    ds: super::gdt::kernel_data_selector(),
    _reserved7: 0,
    fs: super::gdt::kernel_data_selector(),
    _reserved8: 0,
    gs: super::gdt::kernel_data_selector(),
    _reserved9: 0,
    ldt: 0,
    _reserved10: 0,
    iopb_offset: 0,
};

pub fn init(kernel_stack_top: u32) {
    unsafe {
        TSS.esp0 = kernel_stack_top;
        TSS.ss0 = super::gdt::kernel_data_selector();
        TSS.iopb_offset = core::mem::size_of::<Tss>() as u16;

        // Set up TSS descriptor in GDT
        let base = core::ptr::addr_of!(TSS) as u32;
        let limit = core::mem::size_of::<Tss>() as u32 - 1;

        // TSS descriptor: base, limit, access=0x89 (available 32-bit TSS), gran=0x00
        super::gdt::gdt_set_entry(3, base, limit, 0x89, 0x00);

        // Load TR
        load_tr(super::gdt::tss_selector());
    }
}

pub fn set_kernel_stack(esp0: u32) {
    unsafe {
        TSS.esp0 = esp0;
    }
}

pub fn tss_esp0() -> u32 {
    unsafe { TSS.esp0 }
}

extern "C" {
    fn load_tr(selector: u16);
}
