use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{null_mut};
use core::mem;
use crate::sync::Spinlock;

struct Hole {
    size: usize,
    next: Option<&'static mut Hole>,
}

impl Hole {
    fn addr(&self) -> usize {
        self as *const _ as usize
    }

    fn end_addr(&self) -> usize {
        self.addr() + self.size
    }
}

pub struct LinkedHeap {
    head: Spinlock<Hole>,
}

impl LinkedHeap {
    pub const fn empty() -> Self {
        Self {
            head: Spinlock::new(Hole {
                size: 0,
                next: None,
            }),
        }
    }

    pub unsafe fn init(&self, start: usize, size: usize) {
        let mut head = self.head.lock();
        // Ensure the start address is properly aligned for a Hole
        let aligned_start = Self::align_up(start, mem::align_of::<Hole>());
        let aligned_size = size - (aligned_start - start); // Adjust size based on alignment

        if aligned_size < mem::size_of::<Hole>() {
            // Not enough space to even create the first hole
            return; 
        }

        let ptr = aligned_start as *mut Hole;
        ptr.write(Hole {
            size: aligned_size,
            next: None,
        });
        head.next = Some(&mut *ptr);
    }

    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }
}

// --- Define the static ALLOCATOR instance ---
// This makes the heap accessible globally via `crate::allocator::ALLOCATOR`
pub static ALLOCATOR: LinkedHeap = LinkedHeap::empty();
// -------------------------------------------

unsafe impl GlobalAlloc for LinkedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut head = self.head.lock();
        let mut current = &mut *head;

        // Enforce a minimum alignment (8 bytes) to ensure hardware and metadata safety.
        // We also round the size up to a multiple of the alignment so the next hole starts aligned.
        let align = layout.align().max(8);
        // Ensure the requested size is at least enough to hold a Hole's metadata.
        let size = Self::align_up(layout.size(), align).max(mem::size_of::<Hole>());

        while let Some(ref mut hole) = current.next {
            // Align the start of the potential allocation within the current hole.
            let start = Self::align_up(hole.addr(), align);
            // Calculate the end address of the requested allocation.
            let mut end = start + size;

            // Check if the allocation fits within the current hole.
            if end <= hole.end_addr() {
                // We found a fit. Remove the allocated space from the list.
                let hole_being_split = current.next.take().unwrap(); // Take ownership of the hole to split
                let hole_end = hole_being_split.end_addr();
                let next_hole_after_split = hole_being_split.next.take();

                // If the remaining fragment after allocation is too small to store a Hole's metadata,
                // then we allocate that fragment as well to avoid losing memory.
                if hole_end > end && hole_end - end < mem::size_of::<Hole>() {
                    end = hole_end; // Allocate the entire remaining fragment
                }

                // If there's space left before the allocation (between hole start and allocation start), create a new hole.
                if start > hole_being_split.addr() {
                    let prefix_size = start - hole_being_split.addr();
                    // Write the prefix hole into the space before the allocation.
                    let prefix_hole_ptr = hole_being_split.addr() as *mut Hole;
                    prefix_hole_ptr.write(Hole {
                        size: prefix_size,
                        next: None, // This prefix hole will be linked to the next part
                    });
                    current.next = Some(&mut *prefix_hole_ptr);
                    current = current.next.as_mut().unwrap(); // Move current to the new prefix hole
                }

                // If there's space left after the allocation (between allocation end and hole end), create a new hole.
                if hole_end > end {
                    let suffix_ptr = end as *mut Hole;
                    // Write the suffix hole into the space after the allocation.
                    suffix_ptr.write(Hole {
                        size: hole_end - end,
                        next: next_hole_after_split, // Link it to the next hole in the original list
                    });
                    current.next = Some(&mut *suffix_ptr); // Link current to the new suffix hole
                } else {
                    // No space left, so just link to the hole that came after this one.
                    current.next = next_hole_after_split;
                }

                return start as *mut u8; // Return the start address of the allocated memory
            }
            // If no fit in this hole, move to the next one.
            current = current.next.as_mut().unwrap();
        }

        null_mut() // No suitable hole found
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut head = self.head.lock();
        let addr = ptr as usize;
        let align = layout.align().max(8);
        let size = Self::align_up(layout.size(), align).max(mem::size_of::<Hole>());
        
        let mut current = &mut *head;
        // Find the insertion point (sorted by address) for the new free block.
        while current.next.as_ref().map_or(false, |h| h.addr() < addr) {
            current = current.next.as_mut().unwrap();
        }

        // Create the new hole struct at the deallocated address.
        let new_hole_ptr = addr as *mut Hole;
        new_hole_ptr.write(Hole {
            size,
            next: current.next.take(), // Take the next hole from the list to link it later
        });
        current.next = Some(&mut *new_hole_ptr); // Link the previous node to the new hole

        // Try to coalesce with the next hole (if it exists and is contiguous)
        let new_hole = current.next.as_mut().unwrap(); // Get a mutable reference to the new hole
        if let Some(next_hole) = new_hole.next.take() { // Temporarily take ownership of the next hole
            if new_hole.end_addr() == next_hole.addr() { // Check for contiguity
                new_hole.size += next_hole.size; // Merge sizes
                new_hole.next = next_hole.next.take(); // Link to the hole after the merged one
            } else {
                new_hole.next = Some(next_hole); // Put the next hole back if not contiguous
            }
        }

        // Try to coalesce with the previous hole (if it exists and is contiguous)
        // We check current.size > 0 to ensure we don't try to merge with the dummy head node
        if current.size > 0 && current.end_addr() == addr {
            if let Some(new_node) = current.next.take() { // Take ownership of the hole we just created (now current.next)
                current.size += new_node.size; // Merge sizes
                current.next = new_node.next.take(); // Link to the hole after the merged one
            }
        }
    }
}