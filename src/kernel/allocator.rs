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
        let ptr = start as *mut Hole;
        ptr.write(Hole {
            size,
            next: None,
        });
        head.next = Some(&mut *ptr);
    }

    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }
}

unsafe impl GlobalAlloc for LinkedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut head = self.head.lock();
        let mut current = &mut *head;

        // Enforce a minimum alignment (8 bytes) to ensure hardware and metadata safety.
        // We also round the size up to a multiple of the alignment so the next hole starts aligned.
        let align = layout.align().max(8);
        let size = Self::align_up(layout.size(), align).max(mem::size_of::<Hole>());

        while let Some(ref mut hole) = current.next {
            let start = Self::align_up(hole.addr(), align);
            let mut end = start + size;

            if end <= hole.end_addr() {
                // We found a fit. Remove the hole from the list.
                let hole = current.next.take().unwrap();
                let hole_end = hole.end_addr();
                let next_hole = hole.next.take();

                // If the remaining fragment is too small to store a Hole, include it in this allocation
                if hole_end > end && hole_end - end < mem::size_of::<Hole>() {
                    end = hole_end;
                }

                // If there's space left before the allocation, create a hole.
                if start > hole.addr() {
                    let prefix_size = start - hole.addr();
                    hole.size = prefix_size;
                    hole.next = None; // Re-linked below
                    current.next = Some(hole);
                    current = current.next.as_mut().unwrap();
                }

                // If there's space left after the allocation, create a hole.
                if hole_end > end {
                    let suffix_ptr = end as *mut Hole;
                    suffix_ptr.write(Hole {
                        size: hole_end - end,
                        next: next_hole,
                    });
                    current.next = Some(&mut *suffix_ptr);
                } else {
                    current.next = next_hole;
                }

                return start as *mut u8;
            }
            current = current.next.as_mut().unwrap();
        }

        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut head = self.head.lock();
        let addr = ptr as usize;
        let align = layout.align().max(8);
        let size = Self::align_up(layout.size(), align).max(mem::size_of::<Hole>());
        
        let mut current = &mut *head;
        // Find the insertion point (sorted by address)
        while current.next.as_ref().map_or(false, |h| h.addr() < addr) {
            current = current.next.as_mut().unwrap();
        }

        // Create the new hole
        let new_hole_ptr = addr as *mut Hole;
        new_hole_ptr.write(Hole {
            size,
            next: current.next.take(),
        });
        current.next = Some(&mut *new_hole_ptr);

        // Coalesce with next: if this hole ends where the next begins, merge them
        let new_hole = current.next.as_mut().unwrap();
        if let Some(next_hole) = new_hole.next.take() {
            if new_hole.end_addr() == next_hole.addr() {
                new_hole.size += next_hole.size;
                new_hole.next = next_hole.next.take();
            } else {
                new_hole.next = Some(next_hole);
            }
        }

        // Coalesce with previous: if the previous hole ends where this one begins, merge them
        // We check current.size > 0 to ensure we don't try to merge with the dummy head node
        if current.size > 0 && current.end_addr() == addr {
            if let Some(new_node) = current.next.take() {
                current.size += new_node.size;
                current.next = new_node.next.take();
            }
        }
    }
}