// Global Allocator for NebulaOS
// Uses both slab and buddy allocators for efficient memory management
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;
use crate::memory::{slab::SlabAllocator, buddy::BuddyAllocator};
use spin::Mutex;

/// Combined allocator using both slab and buddy systems
pub struct CombinedAllocator {
    slab: Mutex<SlabAllocator>,
    buddy: Mutex<BuddyAllocator>,
}

impl CombinedAllocator {
    /// Create a new combined allocator
    pub const fn new() -> Self {
        CombinedAllocator {
            slab: Mutex::new(SlabAllocator::new()),
            buddy: Mutex::new(BuddyAllocator::new(12, 20, 1024 * 1024 * 16)), // 16MB initial
        }
    }

    /// Initialize the allocator with memory range
    pub fn init(&self, start: usize, size: usize) {
        // Initialize buddy allocator with the memory range
        let mut buddy = self.buddy.lock();
        // In a real implementation, we would add the memory to the buddy allocator
    }
}

unsafe impl GlobalAlloc for CombinedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // For small allocations, use slab allocator
        if layout.size() <= 4096 {
            let mut slab = self.slab.lock();
            slab.alloc(layout)
        } else {
            // For large allocations, use buddy allocator
            let mut buddy = self.buddy.lock();
            buddy.alloc(layout.size())
                .unwrap_or(ptr::null_mut())
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() <= 4096 {
            let mut slab = self.slab.lock();
            slab.dealloc(ptr, layout);
        } else {
            let mut buddy = self.buddy.lock();
            buddy.dealloc(ptr, layout.size());
        }
    }
}

/// Global allocator instance
#[global_allocator]
static ALLOCATOR: CombinedAllocator = CombinedAllocator::new();

/// Initialize the allocator
pub fn init_heap(start: usize, size: usize) {
    unsafe {
        ALLOCATOR.init(start, size);
    }
}
