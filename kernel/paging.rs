use crate::memory::frame_allocator;
use core::ptr;

const PAGE_SIZE: usize = 4096;
const ENTRIES_PER_TABLE: usize = 1024;
const MMIO_VIRTUAL_BASE: u32 = 0xE000_0000;
const MMIO_MAX_SIZE: usize = 64 * 1024 * 1024;

#[repr(C, align(4096))]
struct PageDirectory {
    entries: [u32; ENTRIES_PER_TABLE],
}

#[repr(C, align(4096))]
struct PageTable {
    entries: [u32; ENTRIES_PER_TABLE],
}

static mut KERNEL_PAGE_DIR: *mut PageDirectory = ptr::null_mut();

const PAGE_PRESENT: u32 = 0x0000_0001;
const PAGE_READ_WRITE: u32 = 0x0000_0002;
const PAGE_CACHE_DISABLE: u32 = 0x0000_0010;
const PAGE_WRITE_THROUGH: u32 = 0x0000_0008;

pub fn init() {
    unsafe {
        let pd_phys = frame_allocator::alloc_frame().expect("no memory for page directory");
        let pt_phys = frame_allocator::alloc_frame().expect("no memory for page table");
        let pd = pd_phys as *mut PageDirectory;
        let pt = pt_phys as *mut PageTable;

        for i in 0..ENTRIES_PER_TABLE {
            (*pt).entries[i] = (i * PAGE_SIZE) as u32 | PAGE_PRESENT | PAGE_READ_WRITE;
        }
        (*pd).entries[0] = pt_phys as u32 | PAGE_PRESENT | PAGE_READ_WRITE;

        let pt2_phys = frame_allocator::alloc_frame().expect("no memory for page table");
        let pt2 = pt2_phys as *mut PageTable;
        for i in 0..ENTRIES_PER_TABLE {
            (*pt2).entries[i] =
                ((ENTRIES_PER_TABLE + i) * PAGE_SIZE) as u32 | PAGE_PRESENT | PAGE_READ_WRITE;
        }
        (*pd).entries[1] = pt2_phys as u32 | PAGE_PRESENT | PAGE_READ_WRITE;

        for i in 2..ENTRIES_PER_TABLE {
            (*pd).entries[i] = 0;
        }

        KERNEL_PAGE_DIR = pd;
        load_page_dir(pd_phys as u32);
        enable_paging();
    }
}

pub fn map_mmio(physical_start: u32, length: usize) -> Option<*mut u8> {
    if length == 0 || length > MMIO_MAX_SIZE {
        return None;
    }

    let page_offset = physical_start as usize & (PAGE_SIZE - 1);
    let physical_page = physical_start as usize & !(PAGE_SIZE - 1);
    let mapped_length = length.checked_add(page_offset)?;
    let page_count = mapped_length.checked_add(PAGE_SIZE - 1)? / PAGE_SIZE;
    let end = physical_page.checked_add(page_count.checked_mul(PAGE_SIZE)?)?;
    if end > u32::MAX as usize {
        return None;
    }

    unsafe {
        let page_directory = KERNEL_PAGE_DIR;
        if page_directory.is_null() {
            return None;
        }

        for page in 0..page_count {
            let virtual_address = MMIO_VIRTUAL_BASE as usize + page * PAGE_SIZE;
            let directory_index = virtual_address >> 22;
            let table_index = (virtual_address >> 12) & 0x3FF;
            let mut table_entry = (*page_directory).entries[directory_index];

            if table_entry & PAGE_PRESENT == 0 {
                let table_phys = frame_allocator::alloc_frame()?;
                let table = table_phys as *mut PageTable;
                ptr::write_bytes(table, 0, 1);
                table_entry = table_phys as u32 | PAGE_PRESENT | PAGE_READ_WRITE;
                (*page_directory).entries[directory_index] = table_entry;
            }

            let table = (table_entry & 0xFFFF_F000) as *mut PageTable;
            (*table).entries[table_index] = (physical_page + page * PAGE_SIZE) as u32
                | PAGE_PRESENT
                | PAGE_READ_WRITE
                | PAGE_CACHE_DISABLE
                | PAGE_WRITE_THROUGH;
        }

        let cr3: u32;
        core::arch::asm!("mov {0}, cr3", out(reg) cr3, options(nostack, preserves_flags));
        load_page_dir(cr3);
        Some((MMIO_VIRTUAL_BASE as usize + page_offset) as *mut u8)
    }
}

extern "C" {
    fn load_page_dir(cr3: u32);
}

fn enable_paging() {
    unsafe {
        core::arch::asm!(
            "mov {cr0:e}, cr0",
            "or {cr0:e}, 0x80000000",
            "mov cr0, {cr0:e}",
            cr0 = out(reg) _,
            options(nostack),
        );
    }
}

pub fn kernel_page_dir() -> u32 {
    unsafe { KERNEL_PAGE_DIR as u32 }
}
