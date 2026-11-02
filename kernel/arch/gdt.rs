// Global Descriptor Table and Task State Segment

use core::ptr;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

#[repr(C, packed)]
pub struct GdtPtr {
    limit: u16,
    base: u32,
}

const GDT_ENTRIES: usize = 6;

static mut GDT: [GdtEntry; GDT_ENTRIES] = [GdtEntry {
    limit_low: 0,
    base_low: 0,
    base_middle: 0,
    access: 0,
    granularity: 0,
    base_high: 0,
}; GDT_ENTRIES];

static mut GDT_PTR: GdtPtr = GdtPtr { limit: 0, base: 0 };

const SEG_NULL: u16 = 0x00;
const SEG_KERNEL_CODE: u16 = 0x08;
const SEG_KERNEL_DATA: u16 = 0x10;
const SEG_TSS: u16 = 0x18;
const SEG_USER_CODE: u16 = 0x20 | 0x03;
const SEG_USER_DATA: u16 = 0x28 | 0x03;

pub(super) fn gdt_set_entry(index: usize, base: u32, limit: u32, access: u8, gran: u8) {
    unsafe {
        GDT[index].limit_low = (limit & 0xFFFF) as u16;
        GDT[index].base_low = (base & 0xFFFF) as u16;
        GDT[index].base_middle = ((base >> 16) & 0xFF) as u8;
        GDT[index].access = access;
        GDT[index].granularity = ((limit >> 16) & 0x0F) as u8 | gran;
        GDT[index].base_high = ((base >> 24) & 0xFF) as u8;
    }
}

pub fn init() {
    unsafe {
        GDT_PTR.limit = (GDT_ENTRIES * core::mem::size_of::<GdtEntry>() - 1) as u16;
        GDT_PTR.base = ptr::addr_of!(GDT) as u32;

        // Null descriptor
        gdt_set_entry((SEG_NULL >> 3) as usize, 0, 0, 0, 0);
        // Kernel code segment: base=0, limit=4GB, present, ring0, code, executable, readable
        gdt_set_entry((SEG_KERNEL_CODE >> 3) as usize, 0, 0xFFFFF, 0x9A, 0xCF);
        // Kernel data segment: base=0, limit=4GB, present, ring0, data, writable
        gdt_set_entry((SEG_KERNEL_DATA >> 3) as usize, 0, 0xFFFFF, 0x92, 0xCF);
        // User code/data segments: same flat address space, privilege level 3.
        gdt_set_entry((SEG_USER_CODE >> 3) as usize, 0, 0xFFFFF, 0xFA, 0xCF);
        gdt_set_entry((SEG_USER_DATA >> 3) as usize, 0, 0xFFFFF, 0xF2, 0xCF);

        load_gdt(ptr::addr_of!(GDT_PTR));
    }
}

extern "C" {
    fn load_gdt(ptr: *const GdtPtr);
}

pub const fn kernel_code_selector() -> u16 {
    SEG_KERNEL_CODE
}

pub const fn kernel_data_selector() -> u16 {
    SEG_KERNEL_DATA
}

pub const fn tss_selector() -> u16 {
    SEG_TSS
}

pub const fn user_code_selector() -> u16 {
    SEG_USER_CODE
}

pub const fn user_data_selector() -> u16 {
    SEG_USER_DATA
}
