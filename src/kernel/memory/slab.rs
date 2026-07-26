use alloc::vec::Vec;
use core::ptr;

pub struct SlabCache {
    object_size: usize,
    objects_per_slab: usize,
    partial: Vec<Slab>,
    full: Vec<Slab>,
}

impl SlabCache {
    pub fn new(object_size: usize) -> Self {
        let page_size = 4096;
        let objects_per_slab = page_size / object_size;
        SlabCache { object_size, objects_per_slab, partial: Vec::new(), full: Vec::new() }
    }

    pub fn alloc(&mut self) -> *mut u8 {
        for slab in self.partial.iter_mut() {
            if let Some(ptr) = slab.alloc() {
                if slab.free_count == 0 {
                    let slab = self.partial.remove(0);
                    self.full.push(slab);
                }
                return ptr;
            }
        }
        let mut slab = Slab::new(self.object_size, self.objects_per_slab);
        let ptr = slab.alloc().expect("New slab should have space");
        if slab.free_count > 0 {
            self.partial.push(slab);
        } else {
            self.full.push(slab);
        }
        ptr
    }

    pub fn free(&mut self, ptr: *mut u8) {
        for slab in self.full.iter_mut() {
            if slab.contains(ptr) {
                slab.free(ptr);
                return;
            }
        }
        for slab in self.partial.iter_mut() {
            if slab.contains(ptr) {
                slab.free(ptr);
                return;
            }
        }
    }
}

struct Slab {
    memory: *mut u8,
    object_size: usize,
    objects_per_slab: usize,
    free_list: *mut u8,
    free_count: usize,
}

impl Slab {
    fn new(object_size: usize, objects_per_slab: usize) -> Self {
        let page_size = 4096;
        let memory = unsafe { page_alloc(page_size) };
        let mut free_list = ptr::null_mut();
        let mut current = memory;
        for _ in 0..objects_per_slab {
            let next = unsafe { current.add(object_size) };
            unsafe { ptr::write(next as *mut *mut u8, free_list) };
            free_list = current;
            current = next;
        }
        Slab { memory, object_size, objects_per_slab, free_list, free_count: objects_per_slab }
    }

    fn alloc(&mut self) -> Option<*mut u8> {
        if self.free_count == 0 { return None; }
        let obj = self.free_list;
        self.free_list = unsafe { ptr::read(self.free_list as *mut *mut u8) };
        self.free_count -= 1;
        Some(obj)
    }

    fn free(&mut self, ptr: *mut u8) {
        unsafe { ptr::write(ptr as *mut *mut u8, self.free_list); }
        self.free_list = ptr;
        self.free_count += 1;
    }

    fn contains(&self, ptr: *mut u8) -> bool {
        let start = self.memory;
        let end = unsafe { self.memory.add(self.object_size * self.objects_per_slab) };
        ptr >= start && ptr < end
    }
}

unsafe fn page_alloc(size: usize) -> *mut u8 {
    extern "C" { fn _page_alloc(size: usize) -> *mut u8; }
    _page_alloc(size)
}

pub struct SlabAllocator {
    caches: Vec<SlabCache>,
}

impl SlabAllocator {
    pub const fn new() -> Self { SlabAllocator { caches: Vec::new() } }

    fn get_cache(&mut self, size: usize) -> &mut SlabCache {
        let size = size.next_power_of_two();
        let len_before = self.caches.len();
        // Check if cache already exists using index
        let idx = self.caches.iter().position(|c| c.object_size == size);
        let idx = match idx {
            Some(i) => i,
            None => {
                let cache = SlabCache::new(size);
                self.caches.push(cache);
                len_before // new index
            }
        };
        &mut self.caches[idx]
    }

    pub fn alloc(&mut self, layout: alloc::alloc::Layout) -> *mut u8 {
        let cache = self.get_cache(layout.size());
        cache.alloc()
    }

    pub fn dealloc(&mut self, ptr: *mut u8, layout: alloc::alloc::Layout) {
        let cache = self.get_cache(layout.size());
        cache.free(ptr);
    }
}
