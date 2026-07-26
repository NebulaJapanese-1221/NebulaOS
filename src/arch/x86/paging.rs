// x86 (32-bit) Paging implementation
// Uses 2-level page tables (Page Directory + Page Table) with 4KB pages
// Also supports 4MB pages via the PSE flag

use core::arch::asm;

pub const PAGE_PRESENT: u32 = 1 << 0;
pub const PAGE_WRITABLE: u32 = 1 << 1;
pub const PAGE_USER: u32 = 1 << 2;
pub const PAGE_SIZE: u32 = 0x0040_0000; // 4MB

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageEntry {
    flags: u32,
    physical_addr: u32,
}

impl PageEntry {
    fn new(physical_addr: u32, flags: u32) -> Self {
        PageEntry {
            flags,
            physical_addr: physical_addr & !0xFFF,
        }
    }
}

const PAGE_DIRECTORY_SIZE: usize = 1024;

static mut KERNEL_PAGE_DIRECTORY: [PageEntry; PAGE_DIRECTORY_SIZE] = [
    PageEntry { flags: 0, physical_addr: 0 };
    PAGE_DIRECTORY_SIZE
];

pub unsafe fn init_paging() {
    let kernel_phys_addr = 0x00000000;
    let kernel_virt_addr = 0x00000000;
    let pde_idx = kernel_virt_addr / PAGE_SIZE;
    let flags = PAGE_PRESENT | PAGE_WRITABLE;
    KERNEL_PAGE_DIRECTORY[pde_idx as usize] = PageEntry::new(kernel_phys_addr, flags | PAGE_SIZE);

    let framebuffer_phys_addr = 0xFD000000;
    let framebuffer_virt_addr = 0xFD000000;
    let pde_idx = framebuffer_virt_addr / PAGE_SIZE;
    KERNEL_PAGE_DIRECTORY[pde_idx as usize] = PageEntry::new(framebuffer_phys_addr, flags | PAGE_SIZE);

    let pd_phys_addr = &raw const KERNEL_PAGE_DIRECTORY as *const _ as u32 & !0xFFF;
    asm!("mov cr3, {}", in(reg) pd_phys_addr);

    let mut cr0: u32;
    asm!("mov {}, cr0", out(reg) cr0);
    cr0 |= (1 << 31) | (1 << 0);
    asm!("mov cr0, {}", in(reg) cr0);
}

pub fn get_kernel_page_directory_phys_addr() -> u32 {
    (&raw const KERNEL_PAGE_DIRECTORY as *const _ as u32) & !0xFFF
}

pub fn create_user_page_directory() -> u32 {
    get_kernel_page_directory_phys_addr()
}

pub fn map_page(virt: u32, phys: u32, flags: u32) {
    // TODO: implement page table mapping for x86
}

pub fn unmap_page(virt: u32) {
    // TODO: implement page unmapping
}

