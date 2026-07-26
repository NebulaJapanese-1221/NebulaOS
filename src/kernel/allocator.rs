// Global Allocator for NebulaOS
// Uses both slab and buddy allocators for efficient memory management
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::memory::{slab::SlabAllocator, buddy::BuddyAllocator};
use crate::sync::Spinlock;

/// Combined allocator using both slab and buddy systems
pub struct CombinedAllocator {
    initialized: AtomicBool,
    slab: Spinlock<SlabAllocator>,
    buddy: Spinlock<BuddyAllocator>,
}

unsafe impl Sync for CombinedAllocator {}
unsafe impl Send for CombinedAllocator {}

impl CombinedAllocator {
    /// Create a new combined allocator (lazily initialized)
    pub const fn new() -> Self {
        CombinedAllocator {
            initialized: AtomicBool::new(false),
            slab: Spinlock::new(SlabAllocator::new()),
            buddy: Spinlock::new(BuddyAllocator::new(12, 20, 1024 * 1024 * 16)), // 16MB initial
        }
    }

    fn ensure_init(&self) {
        if !self.initialized.load(Ordering::SeqCst) {
            self.initialized.store(true, Ordering::SeqCst);
        }
    }

    /// Initialize the allocator with memory range
pub fn init(&self, _start: usize, _size: usize) {
        self.ensure_init();
        // Initialize buddy allocator with the memory range
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
pub static ALLOCATOR: CombinedAllocator = CombinedAllocator::new();

/// Initialize the allocator
pub fn init_heap(start: usize, size: usize) {
    ALLOCATOR.init(start, size);
}

/// Allocate a page of memory (page-aligned, 4KB)
/// Used by the slab allocator for backing pages
#[no_mangle]
pub unsafe extern "C" fn _page_alloc(size: usize) -> *mut u8 {
    // Allocate from the global buddy allocator
    let layout = alloc::alloc::Layout::from_size_align(size, 4096).unwrap();
    let mut buddy = ALLOCATOR.buddy.lock();
    buddy.alloc(layout.size()).unwrap_or(core::ptr::null_mut())
}
