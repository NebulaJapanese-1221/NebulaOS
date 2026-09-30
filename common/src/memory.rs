use crate::idt;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);
pub const MEMORY_TYPE_FREE: u32 = 1;
pub const MAX_PHYSICAL_PAGES: usize = 1 << 20;
pub const HEAP_SIZE: usize = 1024 * 1024;

const BITMAP_WORDS: usize = MAX_PHYSICAL_PAGES / 32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryRegion {
    pub base: u64,
    pub length: u64,
    pub kind: u32,
    pub reserved: u32,
}

static mut PAGE_BITMAP: [u32; BITMAP_WORDS] = [u32::MAX; BITMAP_WORDS];
static mut ALLOCATED_BITMAP: [u32; BITMAP_WORDS] = [0; BITMAP_WORDS];
static mut TOTAL_PHYSICAL_MEMORY: usize = 0;
static mut USED_PHYSICAL_MEMORY: usize = 0;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut HEAP_HEAD: *mut HeapBlock = core::ptr::null_mut();

#[repr(C)]
struct HeapBlock {
    size: usize,
    free: bool,
    next: *mut HeapBlock,
}

#[repr(C)]
struct HeapAllocation {
    block: *mut HeapBlock,
}

pub unsafe fn memory_init() {
    memory_init_with_map(core::ptr::null(), 0, 0, 0, 0, 0);
}

pub unsafe fn memory_init_with_map(
    regions: *const MemoryRegion,
    count: usize,
    kernel_start: u64,
    kernel_end: u64,
    framebuffer_start: u64,
    framebuffer_length: u64,
) {
    for word in 0..BITMAP_WORDS {
        PAGE_BITMAP[word] = u32::MAX;
        ALLOCATED_BITMAP[word] = 0;
    }
    TOTAL_PHYSICAL_MEMORY = 0;
    USED_PHYSICAL_MEMORY = 0;

    if regions.is_null() || count == 0 {
        memory_init_heap();
        return;
    }

    for index in 0..count {
        let region = &*regions.add(index);
        if region.kind != MEMORY_TYPE_FREE || region.length == 0 {
            continue;
        }
        let end = region
            .base
            .saturating_add(region.length)
            .min(MAX_PHYSICAL_PAGES as u64 * PAGE_SIZE as u64);
        let first_page = region.base.saturating_add((PAGE_SIZE - 1) as u64) / PAGE_SIZE as u64;
        let last_page = end / PAGE_SIZE as u64;
        for page in first_page..last_page {
            let page = page as usize;
            let word = page / 32;
            let bit = page % 32;
            if PAGE_BITMAP[word] & (1 << bit) != 0 {
                PAGE_BITMAP[word] &= !(1 << bit);
                TOTAL_PHYSICAL_MEMORY += PAGE_SIZE;
            }
        }
    }

    reserve_range(0, 0x100000);
    reserve_range(kernel_start, kernel_end.saturating_sub(kernel_start));
    reserve_range(framebuffer_start, framebuffer_length);
    memory_init_heap();
}

unsafe fn reserve_range(start: u64, length: u64) {
    if length == 0 {
        return;
    }
    let end = start
        .saturating_add(length)
        .min(MAX_PHYSICAL_PAGES as u64 * PAGE_SIZE as u64);
    let first_page = start / PAGE_SIZE as u64;
    let last_page = end.saturating_add((PAGE_SIZE - 1) as u64) / PAGE_SIZE as u64;
    for page in first_page..last_page {
        let page = page as usize;
        let word = page / 32;
        let bit = page % 32;
        if PAGE_BITMAP[word] & (1 << bit) == 0 {
            PAGE_BITMAP[word] |= 1 << bit;
            TOTAL_PHYSICAL_MEMORY = TOTAL_PHYSICAL_MEMORY.saturating_sub(PAGE_SIZE);
        }
    }
}

pub unsafe fn alloc_page() -> *mut u8 {
    for word in 0..BITMAP_WORDS {
        let available = !PAGE_BITMAP[word];
        if available == 0 {
            continue;
        }
        let bit = available.trailing_zeros() as usize;
        let page = word * 32 + bit;
        PAGE_BITMAP[word] |= 1 << bit;
        ALLOCATED_BITMAP[word] |= 1 << bit;
        USED_PHYSICAL_MEMORY += PAGE_SIZE;
        return (page * PAGE_SIZE) as *mut u8;
    }
    core::ptr::null_mut()
}

pub unsafe fn free_page(ptr: *mut u8) -> bool {
    let address = ptr as usize;
    if address == 0 || address & (PAGE_SIZE - 1) != 0 {
        return false;
    }
    let page = address / PAGE_SIZE;
    if page >= MAX_PHYSICAL_PAGES {
        return false;
    }
    let word = page / 32;
    let bit = page % 32;
    let mask = 1 << bit;
    if ALLOCATED_BITMAP[word] & mask == 0 {
        return false;
    }
    ALLOCATED_BITMAP[word] &= !mask;
    PAGE_BITMAP[word] &= !mask;
    USED_PHYSICAL_MEMORY -= PAGE_SIZE;
    true
}

pub unsafe fn memory_init_heap() {
    let heap_start = core::ptr::addr_of_mut!(HEAP) as *mut u8 as usize;
    let alignment = core::mem::align_of::<HeapBlock>();
    let aligned_start = align_up(heap_start, alignment);
    let lost = aligned_start - heap_start;
    let first = aligned_start as *mut HeapBlock;
    core::ptr::write(
        first,
        HeapBlock {
            size: HEAP_SIZE - lost - core::mem::size_of::<HeapBlock>(),
            free: true,
            next: core::ptr::null_mut(),
        },
    );
    HEAP_HEAD = first;
}

pub unsafe fn heap_alloc(size: usize, alignment: usize) -> *mut u8 {
    if size == 0 || alignment == 0 || !alignment.is_power_of_two() {
        return core::ptr::null_mut();
    }
    let alignment = alignment.max(core::mem::align_of::<HeapAllocation>());
    let allocation_header = core::mem::size_of::<HeapAllocation>();
    let mut block = HEAP_HEAD;
    while !block.is_null() {
        if (*block).free {
            let payload_start = block as usize + core::mem::size_of::<HeapBlock>();
            let user_start = align_up(payload_start + allocation_header, alignment);
            let required_end = user_start.checked_add(size);
            let block_end = payload_start.checked_add((*block).size);
            if let (Some(required_end), Some(block_end)) = (required_end, block_end) {
                if required_end <= block_end {
                    let split_at = align_up(required_end, core::mem::align_of::<HeapBlock>());
                    if split_at + core::mem::size_of::<HeapBlock>() < block_end {
                        let old_next = (*block).next;
                        let next = split_at as *mut HeapBlock;
                        core::ptr::write(
                            next,
                            HeapBlock {
                                size: block_end - split_at - core::mem::size_of::<HeapBlock>(),
                                free: true,
                                next: old_next,
                            },
                        );
                        (*block).size = split_at - payload_start;
                        (*block).next = next;
                    }
                    (*block).free = false;
                    let header = (user_start - allocation_header) as *mut HeapAllocation;
                    core::ptr::write(header, HeapAllocation { block });
                    return user_start as *mut u8;
                }
            }
        }
        block = (*block).next;
    }
    core::ptr::null_mut()
}

pub unsafe fn heap_free(ptr: *mut u8) -> bool {
    if ptr.is_null() {
        return false;
    }
    let heap_start = core::ptr::addr_of_mut!(HEAP) as *mut u8 as usize;
    let heap_end = heap_start + HEAP_SIZE;
    let address = ptr as usize;
    let header_size = core::mem::size_of::<HeapAllocation>();
    if address < heap_start + header_size || address >= heap_end {
        return false;
    }
    let allocation = (address - header_size) as *mut HeapAllocation;
    let block = (*allocation).block;
    let block_address = block as usize;
    if block.is_null()
        || block_address < heap_start
        || block_address + core::mem::size_of::<HeapBlock>() > heap_end
        || (*block).free
    {
        return false;
    }
    (*block).free = true;
    let mut current = HEAP_HEAD;
    while !current.is_null() {
        let next = (*current).next;
        if !next.is_null()
            && (*current).free
            && current as usize + core::mem::size_of::<HeapBlock>() + (*current).size == next as usize
            && (*next).free
        {
            (*current).size += core::mem::size_of::<HeapBlock>() + (*next).size;
            (*current).next = (*next).next;
        } else {
            current = next;
        }
    }
    true
}

fn align_up(value: usize, alignment: usize) -> usize {
    value.saturating_add(alignment - 1) & !(alignment - 1)
}

pub unsafe fn get_total() -> usize { TOTAL_PHYSICAL_MEMORY }
pub unsafe fn get_used() -> usize { USED_PHYSICAL_MEMORY }
pub unsafe fn get_free() -> usize { TOTAL_PHYSICAL_MEMORY.saturating_sub(USED_PHYSICAL_MEMORY) }

pub unsafe fn page_fault_handler(_regs: *mut idt::Registers) {
    crate::nebula::kernel_panic("Page fault");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_reserves_and_reuses_memory() {
        let regions = [
            MemoryRegion { base: 0, length: 8 * 1024 * 1024, kind: MEMORY_TYPE_FREE, reserved: 0 },
            MemoryRegion { base: 8 * 1024 * 1024, length: 1024 * 1024, kind: 2, reserved: 0 },
        ];
        unsafe {
            memory_init_with_map(regions.as_ptr(), regions.len(), 0x100000, 0x180000, 0x700000, 0x100000);
            assert!(get_total() > 0);
            let page = alloc_page();
            assert!(!page.is_null());
            assert!(page as usize >= 0x180000);
            assert!(free_page(page));
            assert!(!free_page(page));
            let first = heap_alloc(128, 16);
            let second = heap_alloc(256, 64);
            assert!(!first.is_null() && !second.is_null());
            assert_eq!(second as usize & 63, 0);
            assert!(heap_free(first));
            assert!(heap_free(second));
            assert!(!heap_free(second));
        }
    }
}
