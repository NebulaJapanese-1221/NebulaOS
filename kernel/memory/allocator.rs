// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

// Global heap allocator

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

use crate::memory::frame_allocator;
use crate::sync::SpinLock;

const HEAP_SIZE: usize = 64 * 1024; // 64KB initial heap
const PAGE_SIZE: usize = 4096;
const IDENTITY_MAPPED_LIMIT: usize = 0x0080_0000;
const WORD_ALIGNMENT: usize = core::mem::size_of::<usize>();

struct HeapBlock {
    size: usize,
    free: bool,
    next: *mut HeapBlock,
}

static mut HEAP_HEAD: *mut HeapBlock = null_mut();
static mut HEAP_INITIALIZED: bool = false;
static HEAP_LOCK: SpinLock<()> = SpinLock::new(());

fn align_up(addr: usize, align: usize) -> Option<usize> {
    addr.checked_add(align - 1)
        .map(|value| value & !(align - 1))
}

pub fn init() {
    unsafe {
        let pages_needed = HEAP_SIZE / PAGE_SIZE;
        let heap_start =
            frame_allocator::alloc_contiguous_frames_below(pages_needed, IDENTITY_MAPPED_LIMIT)
                .expect("out of memory for heap");
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
        let _guard = HEAP_LOCK.lock();
        if !HEAP_INITIALIZED {
            return null_mut();
        }

        let Some(size) = align_up(layout.size().max(1), WORD_ALIGNMENT) else {
            return null_mut();
        };
        let alignment = layout.align().max(core::mem::align_of::<HeapBlock>());

        let mut current = HEAP_HEAD;
        while !current.is_null() {
            let block = &*current;
            if block.free {
                let block_address = current as usize;
                let header_size = core::mem::size_of::<HeapBlock>();
                let Some(mut payload_address) = align_up(block_address + header_size, alignment)
                else {
                    return null_mut();
                };
                let mut aligned_block = payload_address - header_size;
                let prefix_size = aligned_block - block_address;
                if prefix_size != 0 && prefix_size < header_size + WORD_ALIGNMENT {
                    let Some(aligned_payload) =
                        align_up(block_address + header_size * 2 + WORD_ALIGNMENT, alignment)
                    else {
                        return null_mut();
                    };
                    payload_address = aligned_payload;
                    aligned_block = payload_address - header_size;
                }

                let block_end = block_address + header_size + block.size;
                if payload_address
                    .checked_add(size)
                    .is_none_or(|end| end > block_end)
                {
                    current = block.next;
                    continue;
                }

                if aligned_block != block_address {
                    let block_mut = &mut *(current as *mut HeapBlock);
                    let aligned_free = aligned_block as *mut HeapBlock;
                    (*aligned_free).size = block_end - aligned_block - header_size;
                    (*aligned_free).free = true;
                    (*aligned_free).next = block_mut.next;
                    block_mut.size = aligned_block - block_address - header_size;
                    block_mut.next = aligned_free;
                    current = aligned_free;
                }

                let block_mut = &mut *current;
                // Split if large enough
                if block_mut.size > size + header_size {
                    let remaining = block_mut.size - size - header_size;
                    let new_block = (current as usize + header_size + size) as *mut HeapBlock;
                    let new_block_mut = &mut *(new_block);
                    new_block_mut.size = remaining;
                    new_block_mut.free = true;
                    new_block_mut.next = block_mut.next;
                    block_mut.size = size;
                    block_mut.next = new_block;
                }
                block_mut.free = false;
                return (current as usize + header_size) as *mut u8;
            }
            current = block.next;
        }
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let _guard = HEAP_LOCK.lock();
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
static ALLOCATOR: HeapAllocator = HeapAllocator::new();
