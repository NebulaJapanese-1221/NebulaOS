// Slab Allocator for NebulaOS
// Efficient memory allocation for small, fixed-size objects

use alloc::vec::Vec;
use core::ptr;
use core::mem;
use core::ops::Deref;

/// Slab cache for objects of a specific size
pub struct SlabCache {
    object_size: usize,
    objects_per_slab: usize,
    slabs: Vec<Slab>,
    partial: Vec<Slab>,
    full: Vec<Slab>,
}

impl SlabCache {
    /// Create a new slab cache for objects of the given size
    pub fn new(object_size: usize) -> Self {
        // Calculate how many objects fit in a 4KB page
        let page_size = 4096;
        let objects_per_slab = page_size / object_size;
        
        SlabCache {
            object_size,
            objects_per_slab,
            slabs: Vec::new(),
            partial: Vec::new(),
            full: Vec::new(),
        }
    }
    
    /// Allocate an object from the slab cache
    pub fn alloc(&mut self) -> *mut u8 {
        // Try to get from partial slabs first
        for slab in &mut self.partial {
            if let Some(ptr) = slab.alloc() {
                // If slab becomes full, move it to full list
                if slab.free_count == 0 {
                    let slab = self.partial.remove(slab_index);
                    self.full.push(slab);
                }
                return ptr;
            }
        }
        
        // No partial slabs with free space, create a new slab
        let mut slab = Slab::new(self.object_size, self.objects_per_slab);
        let ptr = slab.alloc().expect("New slab should have space");
        
        // Add to partial list if not full
        if slab.free_count > 0 {
            self.partial.push(slab);
        } else {
            self.full.push(slab);
        }
        
        ptr
    }
    
    /// Free an object back to the slab cache
    pub fn free(&mut self, ptr: *mut u8) {
        // Find which slab this pointer belongs to
        for slab in &mut self.full {
            if slab.contains(ptr) {
                slab.free(ptr);
                // Move to partial list
                let slab = self.full.remove(slab_index);
                self.partial.push(slab);
                return;
            }
        }
        
        for slab in &mut self.partial {
            if slab.contains(ptr) {
                slab.free(ptr);
                return;
            }
        }
        
        panic!("Attempt to free pointer not in any slab");
    }
}

/// Individual slab of memory
struct Slab {
    memory: *mut u8,
    object_size: usize,
    objects_per_slab: usize,
    free_list: *mut u8,
    free_count: usize,
}

impl Slab {
    /// Create a new slab
    fn new(object_size: usize, objects_per_slab: usize) -> Self {
        // Allocate a page of memory
        let page_size = 4096;
        let memory = unsafe {
            let ptr = liballoc::alloc(page_size);
            if ptr.is_null() {
                panic!("Failed to allocate slab memory");
            }
            ptr
        };
        
        // Initialize free list
        let mut free_list = ptr::null_mut();
        let mut current = memory;
        
        // Link all objects together in the free list
        for _ in 0..objects_per_slab {
            let next = unsafe { current.add(object_size) };
            unsafe { ptr::write(next as *mut *mut u8, free_list) };
            free_list = current;
            current = next;
        }
        
        Slab {
            memory,
            object_size,
            objects_per_slab,
            free_list,
            free_count: objects_per_slab,
        }
    }
    
    /// Allocate an object from this slab
    fn alloc(&mut self) -> Option<*mut u8> {
        if self.free_count == 0 {
            return None;
        }
        
        let obj = self.free_list;
        self.free_list = unsafe { ptr::read(self.free_list as *mut *mut u8) };
        self.free_count -= 1;
        
        Some(obj)
    }
    
    /// Free an object back to this slab
    fn free(&mut self, ptr: *mut u8) {
        unsafe {
            ptr::write(ptr as *mut *mut u8, self.free_list);
        }
        self.free_list = ptr;
        self.free_count += 1;
    }
    
    /// Check if this slab contains a given pointer
    fn contains(&self, ptr: *mut u8) -> bool {
        let start = self.memory;
        let end = unsafe { self.memory.add(self.object_size * self.objects_per_slab) };
        ptr >= start && ptr < end
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        unsafe {
            liballoc::dealloc(self.memory, 4096);
        }
    }
}

/// Global slab allocator
pub struct SlabAllocator {
    caches: Vec<SlabCache>,
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub fn new() -> Self {
        SlabAllocator {
            caches: Vec::new(),
        }
    }
    
    /// Get or create a slab cache for the given size
    fn get_cache(&mut self, size: usize) -> &mut SlabCache {
        // Round up to nearest power of 2
        let mut size = size;
        size = size.next_power_of_two();
        
        // Find existing cache or create new one
        for cache in &mut self.caches {
            if cache.object_size == size {
                return cache;
            }
        }
        
        // Create new cache
        let cache = SlabCache::new(size);
        self.caches.push(cache);
        self.caches.last_mut().unwrap()
    }
    
    /// Allocate memory
    pub fn alloc(&mut self, layout: alloc::alloc::Layout) -> *mut u8 {
        let cache = self.get_cache(layout.size());
        cache.alloc()
    }
    
    /// Free memory
    pub fn dealloc(&mut self, ptr: *mut u8, layout: alloc::alloc::Layout) {
        let cache = self.get_cache(layout.size());
        cache.free(ptr);
    }
}