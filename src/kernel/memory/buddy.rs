// Buddy System Allocator for NebulaOS
// Efficient allocation of power-of-two sized blocks

use alloc::vec::Vec;
use core::ptr;

/// Buddy system allocator
pub struct BuddyAllocator {
    min_order: usize,      // Minimum block size (2^min_order)
    max_order: usize,      // Maximum block size (2^max_order)
    free_lists: Vec<Vec<*mut u8>>, // Free lists for each order
    total_memory: usize,   // Total memory managed
    used_memory: usize,     // Memory currently in use
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    /// min_order: minimum block size (2^min_order bytes)
    /// max_order: maximum block size (2^max_order bytes)
    /// initial_memory: initial memory pool size
    pub fn new(min_order: usize, max_order: usize, initial_memory: usize) -> Self {
        let mut free_lists = Vec::with_capacity(max_order + 1);
        for _ in 0..=max_order {
            free_lists.push(Vec::new());
        }
        
        let mut allocator = BuddyAllocator {
            min_order,
            max_order,
            free_lists,
            total_memory: initial_memory,
            used_memory: 0,
        };
        
        // Initialize with all memory in the largest block
        if initial_memory > 0 {
            let block = unsafe { liballoc::alloc(initial_memory) };
            if !block.is_null() {
                allocator.free_lists[max_order].push(block);
            }
        }
        
        allocator
    }
    
    /// Allocate a block of memory
    pub fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        // Round up to nearest power of two
        let mut order = self.min_order;
        while (1 << order) < size {
            order += 1;
        }
        
        // Find first non-empty free list of this order or higher
        let mut current_order = order;
        while current_order <= self.max_order {
            if !self.free_lists[current_order].is_empty() {
                // Found a block, split it if necessary
                let block = self.free_lists[current_order].pop().unwrap();
                
                // Split the block down to the requested order
                while current_order > order {
                    current_order -= 1;
                    let buddy = self.get_buddy(block, current_order);
                    self.free_lists[current_order].push(buddy);
                }
                
                self.used_memory += 1 << order;
                return Some(block);
            }
            current_order += 1;
        }
        
        None
    }
    
    /// Free a block of memory
    pub fn dealloc(&mut self, ptr: *mut u8, size: usize) {
        // Round up to nearest power of two
        let mut order = self.min_order;
        while (1 << order) < size {
            order += 1;
        }
        
        self.used_memory -= 1 << order;
        
        // Merge with buddy if possible
        let mut current_order = order;
        loop {
            let buddy = self.get_buddy(ptr, current_order);
            
            // Check if buddy is free
            let mut found = false;
            let list = &mut self.free_lists[current_order];
            for (i, &block) in list.iter().enumerate() {
                if block == buddy {
                    list.remove(i);
                    found = true;
                    break;
                }
            }
            
            if !found {
                // Buddy is not free, add to free list
                self.free_lists[current_order].push(ptr);
                break;
            }
            
            // Merge with buddy
            let merged = if ptr < buddy { ptr } else { buddy };
            current_order += 1;
            
            // If we've reached max order, add to free list
            if current_order > self.max_order {
                self.free_lists[self.max_order].push(merged);
                break;
            }
            
            ptr = merged;
        }
    }
    
    /// Get the buddy of a block
    fn get_buddy(&self, block: *mut u8, order: usize) -> *mut u8 {
        let block_addr = block as usize;
        let block_size = 1 << order;
        let buddy_addr = if block_addr & block_size == 0 {
            block_addr + block_size
        } else {
            block_addr - block_size
        };
        buddy_addr as *mut u8
    }
    
    /// Get memory usage statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.total_memory, self.used_memory, self.total_memory - self.used_memory)
    }
    
    /// Get fragmentation metrics
    pub fn fragmentation(&self) -> f64 {
        let mut free_blocks = 0;
        let mut total_free = 0;
        
        for (order, list) in self.free_lists.iter().enumerate() {
            free_blocks += list.len();
            total_free += list.len() * (1 << order);
        }
        
        if free_blocks == 0 {
            0.0
        } else {
            (free_blocks as f64) / (total_free as f64 / (1 << self.min_order) as f64)
        }
    }
}