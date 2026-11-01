// Paging and virtual memory management

use crate::memory::frame_allocator;

const PAGE_SIZE: usize = 4096;
const PAGE_DIRECTORY_INDEX: usize = 1024;
const PAGE_TABLE_INDEX: usize = 1024;

#[repr(C, packed)]
struct PageTableEntry {
    present: u16,
    _reserved: u16,
}

#[repr(C, align(4096))]
struct PageDirectory {
    entries: [u32; PAGE_DIRECTORY_INDEX],
}

#[repr(C, align(4096))]
struct PageTable {
    entries: [u32; PAGE_TABLE_INDEX],
}

static mut KERNEL_PAGE_DIR: Option<*mut PageDirectory> = None;

const PAGE_PRESENT: u32 = 0x00000001;
const PAGE_READ_WRITE: u32 = 0x00000002;
const PAGE_USER_SUPERVISOR: u32 = 0x00000004;
const PAGE_WRITE_THROUGH: u32 = 0x00000008;
const PAGE_CACHE_DISABLE: u32 = 0x00000010;

fn port_byte_out(_port: u16, _data: u8) {
    // not used here, kept for reference
}

pub fn init() {
    unsafe {
        // Allocate page directory and page tables from frame allocator
        let pd_phys = frame_allocator::alloc_frame().expect("no memory for page directory");
        let pt_phys = frame_allocator::alloc_frame().expect("no memory for page table");

        let pd = &mut *(pd_phys as *mut PageDirectory);
        let pt = &mut *(pt_phys as *mut PageTable);

        // Identity map first 4MB (1024 entries * 4KB)
        for i in 0..PAGE_TABLE_INDEX {
            let addr = (i * PAGE_SIZE) as u32;
            pt.entries[i] = addr | PAGE_PRESENT | PAGE_READ_WRITE;
        }

        // Page directory entry 0 points to the page table
        pd.entries[0] = pt_phys as u32 | PAGE_PRESENT | PAGE_READ_WRITE;

        // Map kernel code/data region (4MB to 8MB identity)
        let pt2_phys = frame_allocator::alloc_frame().expect("no memory for page table 2");
        let pt2 = &mut *(pt2_phys as *mut PageTable);
        for i in 0..PAGE_TABLE_INDEX {
            let addr = ((1024 + i) * PAGE_SIZE) as u32;
            pt2.entries[i] = addr | PAGE_PRESENT | PAGE_READ_WRITE;
        }
        pd.entries[1] = pt2_phys as u32 | PAGE_PRESENT | PAGE_READ_WRITE;

        // Set all other PDEs to 0 (not present)
        for i in 2..PAGE_DIRECTORY_INDEX {
            pd.entries[i] = 0;
        }

        KERNEL_PAGE_DIR = Some(pd as *mut PageDirectory);

        // Load page directory
        load_page_dir(pd as *mut PageDirectory as u32);
        enable_paging();
    }
}

extern "C" {
    fn load_page_dir(cr3: u32);
}

fn enable_paging() {
    unsafe {
        core::arch::asm!("mov eax, cr0", "or eax, 0x80000000", "mov cr0, eax", options(nostack));
    }
}

pub fn kernel_page_dir() -> u32 {
    unsafe { KERNEL_PAGE_DIR.unwrap() as u32 }
}
