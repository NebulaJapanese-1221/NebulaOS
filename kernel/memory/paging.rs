use crate::memory::frame_allocator;
use core::ptr;

const PAGE_SIZE: usize = 4096;
const ENTRIES_PER_TABLE: usize = 1024;
const KERNEL_BUFFER_VIRTUAL_BASE: u32 = 0xD000_0000;
const MMIO_VIRTUAL_BASE: u32 = 0xE000_0000;
const MMIO_MAX_SIZE: usize = 64 * 1024 * 1024;
const USER_PHYSICAL_LIMIT: usize = 0x0080_0000;
const USER_ADDRESS_MIN: u32 = 0x4000_0000;
const USER_ADDRESS_MAX: u32 = 0xC000_0000;

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
const PAGE_USER: u32 = 0x0000_0004;

pub fn create_user_address_space() -> Option<u32> {
    unsafe {
        let kernel_directory = KERNEL_PAGE_DIR;
        if kernel_directory.is_null() {
            return None;
        }
        let directory_frame = frame_allocator::alloc_frame_below(USER_PHYSICAL_LIMIT)?;
        let directory = directory_frame as *mut PageDirectory;
        ptr::write_bytes(directory, 0, 1);
        for index in 0..ENTRIES_PER_TABLE {
            (*directory).entries[index] = (*kernel_directory).entries[index] & !PAGE_USER;
        }
        Some(directory_frame as u32)
    }
}

pub fn map_user_page(directory_address: u32, virtual_address: u32, writable: bool) -> Option<u32> {
    if virtual_address < USER_ADDRESS_MIN || virtual_address >= USER_ADDRESS_MAX {
        return None;
    }
    unsafe {
        let directory = directory_address as *mut PageDirectory;
        if directory.is_null() {
            return None;
        }
        let directory_index = (virtual_address as usize) >> 22;
        let table_index = (virtual_address as usize >> 12) & 0x3FF;
        let mut directory_entry = (*directory).entries[directory_index];
        if directory_entry & PAGE_PRESENT == 0 {
            let table_frame = frame_allocator::alloc_frame_below(USER_PHYSICAL_LIMIT)?;
            let table = table_frame as *mut PageTable;
            ptr::write_bytes(table, 0, 1);
            directory_entry = table_frame as u32 | PAGE_PRESENT | PAGE_READ_WRITE | PAGE_USER;
            (*directory).entries[directory_index] = directory_entry;
        } else if directory_entry & PAGE_USER == 0 {
            return None;
        }

        let table = (directory_entry & 0xFFFF_F000) as *mut PageTable;
        let entry = &mut (*table).entries[table_index];
        if *entry & PAGE_PRESENT != 0 {
            return None;
        }
        let frame = frame_allocator::alloc_frame_below(USER_PHYSICAL_LIMIT)?;
        ptr::write_bytes(frame as *mut u8, 0, PAGE_SIZE);
        *entry = frame as u32 | PAGE_PRESENT | PAGE_USER;
        if writable {
            *entry |= PAGE_READ_WRITE;
        }
        Some(frame as u32)
    }
}

pub fn user_page_frame(directory_address: u32, virtual_address: u32) -> Option<u32> {
    if virtual_address < USER_ADDRESS_MIN || virtual_address >= USER_ADDRESS_MAX {
        return None;
    }
    unsafe {
        let directory = directory_address as *const PageDirectory;
        let directory_entry = (*directory).entries[(virtual_address as usize) >> 22];
        if directory_entry & (PAGE_PRESENT | PAGE_USER) != (PAGE_PRESENT | PAGE_USER) {
            return None;
        }
        let table = (directory_entry & 0xFFFF_F000) as *const PageTable;
        let page_entry = (*table).entries[(virtual_address as usize >> 12) & 0x3FF];
        if page_entry & (PAGE_PRESENT | PAGE_USER) != (PAGE_PRESENT | PAGE_USER) {
            return None;
        }
        Some(page_entry & 0xFFFF_F000)
    }
}

pub fn set_user_page_writable(directory_address: u32, virtual_address: u32) -> bool {
    if virtual_address < USER_ADDRESS_MIN || virtual_address >= USER_ADDRESS_MAX {
        return false;
    }
    unsafe {
        let directory = directory_address as *mut PageDirectory;
        let directory_entry = (*directory).entries[(virtual_address as usize) >> 22];
        if directory_entry & (PAGE_PRESENT | PAGE_USER) != (PAGE_PRESENT | PAGE_USER) {
            return false;
        }
        let table = (directory_entry & 0xFFFF_F000) as *mut PageTable;
        let page_entry = &mut (*table).entries[(virtual_address as usize >> 12) & 0x3FF];
        if *page_entry & (PAGE_PRESENT | PAGE_USER) != (PAGE_PRESENT | PAGE_USER) {
            return false;
        }
        *page_entry |= PAGE_READ_WRITE;
        true
    }
}

pub fn user_range_is_mapped(address: u32, length: usize) -> bool {
    if length == 0 {
        return true;
    }
    let Some(end) = (address as usize).checked_add(length) else {
        return false;
    };
    if address < USER_ADDRESS_MIN || end > USER_ADDRESS_MAX as usize {
        return false;
    }
    let current_directory: u32;
    unsafe {
        core::arch::asm!("mov {0}, cr3", out(reg) current_directory, options(nostack, preserves_flags));
    }
    let mut page = address & !(PAGE_SIZE as u32 - 1);
    while (page as usize) < end {
        if user_page_frame(current_directory, page).is_none() {
            return false;
        }
        page = page.saturating_add(PAGE_SIZE as u32);
    }
    true
}

pub fn destroy_user_address_space(directory_address: u32) {
    unsafe {
        let directory = directory_address as *mut PageDirectory;
        for directory_index in 0..ENTRIES_PER_TABLE {
            let directory_entry = (*directory).entries[directory_index];
            if directory_entry & (PAGE_PRESENT | PAGE_USER) != (PAGE_PRESENT | PAGE_USER) {
                continue;
            }
            let table_frame = (directory_entry & 0xFFFF_F000) as usize;
            let table = table_frame as *mut PageTable;
            for page_entry in (*table).entries {
                if page_entry & (PAGE_PRESENT | PAGE_USER) == (PAGE_PRESENT | PAGE_USER) {
                    frame_allocator::free_frame((page_entry & 0xFFFF_F000) as usize);
                }
            }
            frame_allocator::free_frame(table_frame);
        }
        frame_allocator::free_frame(directory_address as usize);
    }
}

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

pub fn map_kernel_buffer(length: usize) -> Option<*mut u8> {
    if length == 0 || length > MMIO_MAX_SIZE {
        return None;
    }

    let page_count = length.checked_add(PAGE_SIZE - 1)? / PAGE_SIZE;
    let end =
        (KERNEL_BUFFER_VIRTUAL_BASE as usize).checked_add(page_count.checked_mul(PAGE_SIZE)?)?;
    if end > MMIO_VIRTUAL_BASE as usize {
        return None;
    }

    unsafe {
        let page_directory = KERNEL_PAGE_DIR;
        if page_directory.is_null() {
            return None;
        }

        let mut mapped_pages = 0;
        while mapped_pages < page_count {
            let virtual_address = KERNEL_BUFFER_VIRTUAL_BASE as usize + mapped_pages * PAGE_SIZE;
            let directory_index = virtual_address >> 22;
            let table_index = (virtual_address >> 12) & 0x3FF;
            let mut table_entry = (*page_directory).entries[directory_index];

            if table_entry & PAGE_PRESENT == 0 {
                let Some(table_phys) = frame_allocator::alloc_frame() else {
                    unmap_kernel_buffer_pages(mapped_pages);
                    return None;
                };
                let table = table_phys as *mut PageTable;
                ptr::write_bytes(table, 0, 1);
                table_entry = table_phys as u32 | PAGE_PRESENT | PAGE_READ_WRITE;
                (*page_directory).entries[directory_index] = table_entry;
            }

            let table = (table_entry & 0xFFFF_F000) as *mut PageTable;
            if (*table).entries[table_index] & PAGE_PRESENT != 0 {
                unmap_kernel_buffer_pages(mapped_pages);
                return None;
            }
            let Some(frame) = frame_allocator::alloc_frame() else {
                unmap_kernel_buffer_pages(mapped_pages);
                return None;
            };
            (*table).entries[table_index] = frame as u32 | PAGE_PRESENT | PAGE_READ_WRITE;
            mapped_pages += 1;
        }

        let cr3: u32;
        core::arch::asm!("mov {0}, cr3", out(reg) cr3, options(nostack, preserves_flags));
        load_page_dir(cr3);
        let buffer = KERNEL_BUFFER_VIRTUAL_BASE as *mut u8;
        ptr::write_bytes(buffer, 0, length);
        Some(buffer)
    }
}

unsafe fn unmap_kernel_buffer_pages(page_count: usize) {
    let page_directory = KERNEL_PAGE_DIR;
    for page in 0..page_count {
        let virtual_address = KERNEL_BUFFER_VIRTUAL_BASE as usize + page * PAGE_SIZE;
        let directory_entry = (*page_directory).entries[virtual_address >> 22];
        if directory_entry & PAGE_PRESENT == 0 {
            continue;
        }
        let table = (directory_entry & 0xFFFF_F000) as *mut PageTable;
        let page_entry = &mut (*table).entries[(virtual_address >> 12) & 0x3FF];
        if *page_entry & PAGE_PRESENT != 0 {
            frame_allocator::free_frame((*page_entry & 0xFFFF_F000) as usize);
            *page_entry = 0;
        }
    }
    let cr3: u32;
    core::arch::asm!("mov {0}, cr3", out(reg) cr3, options(nostack, preserves_flags));
    load_page_dir(cr3);
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
