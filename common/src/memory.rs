use crate::stdint::*;
use crate::idt;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

pub const HEAP_START: usize = 0x1000000;
pub const HEAP_SIZE: usize = 0x4000000;
pub const HEAP_END: usize = HEAP_START + HEAP_SIZE;

pub const PHYSICAL_MEMORY_START: usize = 0x100000;
pub const PHYSICAL_MEMORY_END: usize = 0xFFFFFFFF;

pub const MEMORY_TYPE_FREE: u8 = 0;
pub const MEMORY_TYPE_RESERVED: u8 = 1;
pub const MEMORY_TYPE_ACPI: u8 = 2;
pub const MEMORY_TYPE_KERNEL: u8 = 3;
pub const MEMORY_TYPE_USER: u8 = 4;

pub const PAGE_PRESENT: u8 = 0x01;
pub const PAGE_WRITABLE: u8 = 0x02;
pub const PAGE_USER: u8 = 0x04;
pub const PAGE_WRITETHROUGH: u8 = 0x08;
pub const PAGE_CACHEDIS: u8 = 0x10;
pub const PAGE_ACCESS: u8 = 0x20;
pub const PAGE_DIRTY: u8 = 0x40;
pub const PAGE_GLOBAL: u8 = 0x80;

static mut TOTAL_PHYSICAL_MEMORY: usize = 0;
static mut USED_PHYSICAL_MEMORY: usize = 0;
static mut PAGE_BITMAP: *mut u32 = core::ptr::null_mut();
static mut TOTAL_PAGES: usize = 0;
static mut PAGE_BITMAP_SIZE: usize = 0;

pub unsafe fn memory_init() {
    memory_init_physical();
    memory_init_heap();
}

pub unsafe fn memory_init_heap() {
}

pub unsafe fn memory_init_physical() {
    TOTAL_PHYSICAL_MEMORY = 128 * 1024 * 1024;
    let available = TOTAL_PHYSICAL_MEMORY - PHYSICAL_MEMORY_START;
    TOTAL_PAGES = available / PAGE_SIZE;
    PAGE_BITMAP_SIZE = (TOTAL_PAGES + 31) / 32;
    PAGE_BITMAP = 0x00200000 as *mut u32;
    core::ptr::write_bytes(PAGE_BITMAP, 0, (PAGE_BITMAP_SIZE * 4) as usize);
    let bitmap_pages = (PAGE_BITMAP_SIZE * 4 + PAGE_SIZE - 1) / PAGE_SIZE;
    for i in 0..bitmap_pages {
        let page_num = (0x00200000 / PAGE_SIZE) + i;
        if page_num < TOTAL_PAGES {
            let word = page_num / 32;
            let bit = page_num % 32;
            *PAGE_BITMAP.add(word as usize) |= 1 << bit;
        }
    }
    let reserved_pages = PHYSICAL_MEMORY_START / PAGE_SIZE;
    for i in 0..reserved_pages {
        if i < TOTAL_PAGES {
            let word = i / 32;
            let bit = i % 32;
            *PAGE_BITMAP.add(word as usize) |= 1 << bit;
        }
    }
}

pub unsafe fn alloc_page() -> *mut u8 {
    for i in 0..PAGE_BITMAP_SIZE {
        if *PAGE_BITMAP.add(i as usize) != 0xFFFFFFFF {
            for j in 0..32 {
                if (*PAGE_BITMAP.add(i as usize) & (1 << j)) == 0 {
                    let page_num = i * 32 + j;
                    *PAGE_BITMAP.add(i as usize) |= 1 << j;
                    USED_PHYSICAL_MEMORY += PAGE_SIZE;
                    return (page_num * PAGE_SIZE) as *mut u8;
                }
            }
        }
    }
    core::ptr::null_mut()
}

pub unsafe fn free_page(ptr: *mut u8) {
    let addr = ptr as usize;
    let page_num = addr / PAGE_SIZE;
    let word = page_num / 32;
    let bit = page_num % 32;
    if word < PAGE_BITMAP_SIZE {
        *PAGE_BITMAP.add(word as usize) &= !(1 << bit);
        USED_PHYSICAL_MEMORY -= PAGE_SIZE;
    }
}

pub unsafe fn get_total() -> usize { TOTAL_PHYSICAL_MEMORY }
pub unsafe fn get_used() -> usize { USED_PHYSICAL_MEMORY }
pub unsafe fn get_free() -> usize { TOTAL_PHYSICAL_MEMORY - USED_PHYSICAL_MEMORY }

pub unsafe fn page_fault_handler(_regs: *mut idt::Registers) {
    crate::nebula::kernel_panic("Page fault");
}
