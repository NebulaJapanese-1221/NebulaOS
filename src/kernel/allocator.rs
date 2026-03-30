//! A simple heap allocator implementation.

use super::multiboot::{self, MultibootMemoryMapEntry};
use linked_list_allocator::LockedHeap;
use spin::Mutex;

extern "C" {
    // Symbol defined in the linker script.
    static _end: u8;
}

/// Represents a region of memory intentionally left unmapped to catch overflows.
#[derive(Debug, Clone, Copy)]
pub struct GuardZone {
    pub start: usize,
    pub end: usize,
}

/// Global list of guard zones for the exception handler to reference.
pub static HEAP_GUARDS: Mutex<[Option<GuardZone>; 8]> = Mutex::new([None; 8]);

/// Finds a suitable region of memory for the heap from the multiboot memory map.
///
/// # Safety
/// The caller must ensure that the multiboot info pointer is valid.
pub fn find_heap_region(multiboot_info_ptr: usize) -> Option<(usize, usize)> {
    if multiboot_info_ptr == 0 { return None; }
    let multiboot_info = unsafe { &*(multiboot_info_ptr as *const multiboot::MultibootInfo) };

    // Check if the memory map is present (bit 6)
    // NOTE(clarification): This check ensures that a memory map was actually provided by the bootloader
    if multiboot_info.flags & (1 << 6) == 0 {
        return None;
    }

    let mmap_start = multiboot_info.mmap_addr as usize;
    let mmap_end = mmap_start + multiboot_info.mmap_length as usize;
    let mut current_addr = mmap_start;

    // Get the address of the `_end` symbol, which marks the end of the kernel image.
    let kernel_end = core::ptr::addr_of!(_end) as usize;
    // Align the kernel end address to the next page boundary to be safe.
    let kernel_end_aligned = ((kernel_end + 4095) / 4096) * 4096;

    let mut best_region: Option<(usize, usize)> = None;

    while current_addr < mmap_end {
        let entry = unsafe { &*(current_addr as *const MultibootMemoryMapEntry) };
        
        // We are looking for a region of type 1 (available RAM).
        if entry.type_ == 1 { // Available RAM
            let region_start = entry.addr as usize;
            let region_end = region_start + entry.len as usize;

            // Check if this region is after the kernel and has a usable size.
            if region_end > kernel_end_aligned {
                let heap_start = region_start.max(kernel_end_aligned);
                let heap_size = region_end - heap_start;

                // If the region is large enough (e.g., > 1MB), we split it to insert a guard page.
                // This places a 4KB "Unmapped" gap right at the start of the heap to catch
                // negative offsets, and another after the first 512KB.
                if heap_size > 1024 * 1024 {
                    // We use a fixed array to avoid heap allocations before the allocator is initialized.
                    let mut g_lock = HEAP_GUARDS.lock();
                    g_lock[0] = Some(GuardZone { start: heap_start, end: heap_start + 4096 });
                    
                    let adjusted_start = heap_start + 4096;
                    let heap_end = region_end - 4096;
                    g_lock[1] = Some(GuardZone { start: heap_end, end: region_end });
                    drop(g_lock);

                    let usable_size = heap_end - adjusted_start;

                    if usable_size > best_region.map_or(0, |r| r.1) {
                        best_region = Some((adjusted_start, usable_size));
                    }
                } else if heap_size > best_region.map_or(0, |r| r.1) {
                     best_region = Some((heap_start, heap_size));
                }
            }
        }

        // Move to the next entry. The size field includes the size of the size field itself.
        current_addr += entry.size as usize + 4;
    }

    best_region
}

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();