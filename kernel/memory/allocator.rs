// Global heap allocator

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, null_mut};

use crate::memory::frame_allocator;

const HEAP_SIZE: usize = 64 * 1024; // 64KB initial heap
const HEAP_START: usize = 0x00300000;

struct HeapBlock {
    size: usize,
    free: bool,
    next: *mut HeapBlock,
}

static mut HEAP_HEAD: *mut HeapBlock = null_mut();
static mut HEAP_INITIALIZED: bool = false;

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub fn init() {
    unsafe {
        // Allocate pages for the heap
        let pages_needed = HEAP_SIZE / 4096;
        let mut addr = HEAP_START;
        for _ in 0..pages_needed {
            let frame = frame_allocator::alloc_frame().expect("out of memory for heap");
            // Identity map - heap is in higher half, but for now identity map
            // In a full paging setup we'd map virtual to physical
            let _ = frame;
            addr += 4096;
        }

        let heap_start = HEAP_START;
        let block = &mut *(heap_start as *mut HeapBlock);
        block.size = HEAP_SIZE - core::mem::size_of::<HeapBlock>();
        block.free = true;
        block.next = null_mut();
        HEAP_HEAD = heap_start as *mut HeapBlock;
        HEAP_INITIALIZED = true;
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    let _ = layout;
    loop {}
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !HEAP_INITIALIZED {
            return null_mut();
        }

        let size = align_up(layout.size(), core::mem::size_of::<usize>());
        let align = layout.align();

        let mut current = HEAP_HEAD;
        while !current.is_null() {
            let block = &*current;
            if block.free && block.size >= size {
                let block_mut = &mut *(current as *mut HeapBlock);
                // Split if large enough
                if block.size > size + core::mem::size_of::<HeapBlock>() {
                    let remaining = block.size - size - core::mem::size_of::<HeapBlock>();
                    let new_block = (current as usize + core::mem::size_of::<HeapBlock>() + size)
                        as *mut HeapBlock;
                    let new_block_mut = &mut *(new_block);
                    new_block_mut.size = remaining;
                    new_block_mut.free = true;
                    new_block_mut.next = block_mut.next;
                    block_mut.size = size;
                    block_mut.next = new_block;
                }
                block_mut.free = false;
                return (current as usize + core::mem::size_of::<HeapBlock>()) as *mut u8;
            }
            current = block.next;
        }
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() || !HEAP_INITIALIZED {
            return;
        }

        let block_ptr = (ptr as usize - core::mem::size_of::<HeapBlock>()) as *mut HeapBlock;
        let block = &mut *block_ptr;
        block.free = true;

        // Coalesce with next free block
        if !block.next.is_null() {
            let next = &*block.next;
            if next.free {
                block.size += core::mem::size_of::<HeapBlock>() + next.size;
                block.next = next.next;
            }
        }
    }
}

pub struct HeapAllocator;

impl HeapAllocator {
    pub const fn new() -> Self {
        HeapAllocator
    }
}

#[global_allocator]
static ALLOCATOR: HeapAllocator = HeapAllocator;
