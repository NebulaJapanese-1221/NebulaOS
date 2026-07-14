// Adaptive Replacement Cache (ARC) for NebulaFS
// Enhanced implementation inspired by ZFS's ARC

use alloc::collections::LinkedList;
use alloc::collections::btree_map::BTreeMap;
use crate::fs::dmu::BlockPointer;

/// ARC cache entry
struct ARCEntry {
    bp: BlockPointer,       // Block pointer
    data: Vec<u8>,          // Cached data
    freq: u32,             // Frequency count
    last_used: u64,        // Last used timestamp
}

/// ARC cache state
pub struct ARCCache {
    target_size: usize,     // Target cache size in bytes
    current_size: usize,    // Current cache size in bytes
    
    // Main cache lists
    mru: LinkedList<u64>,    // Most Recently Used (in cache)
    mfu: LinkedList<u64>,    // Most Frequently Used (in cache)
    
    // Ghost lists (for evicted entries)
    mru_ghost: LinkedList<u64>,
    mfu_ghost: LinkedList<u64>,
    
    // Cache storage
    cache: BTreeMap<u64, ARCEntry>,
    
    // Statistics
    hits: u64,
    misses: u64,
    evictions: u64,
    
    // Adaptive parameters
    p: f64,                // Target ratio of MFU to total cache
    c: usize,             // Total cache size
    
    // Time tracking
    time: u64,
}

impl ARCCache {
    /// Create a new ARC cache
    pub fn new(target_size: usize) -> Self {
        ARCCache {
            target_size,
            current_size: 0,
            mru: LinkedList::new(),
            mfu: LinkedList::new(),
            mru_ghost: LinkedList::new(),
            mfu_ghost: LinkedList::new(),
            cache: BTreeMap::new(),
            hits: 0,
            misses: 0,
            evictions: 0,
            p: 0.0,
            c: target_size,
            time: 0,
        }
    }

    /// Lookup a block in the cache
    pub fn lookup(&mut self, bp: &BlockPointer) -> Option<Vec<u8>> {
        self.time += 1;
        let key = self.block_key(bp);
        
        if let Some(entry) = self.cache.get(&key) {
            self.hits += 1;
            entry.freq += 1;
            entry.last_used = self.time;
            
            // Move to MRU position in the appropriate list
            if self.mru.contains(&key) {
                // Was in MRU list - move to front
                self.mru.move_to_front(key);
            } else if self.mfu.contains(&key) {
                // Was in MFU list - move to front
                self.mfu.move_to_front(key);
            }
            
            Some(entry.data.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a block into the cache
    pub fn insert(&mut self, bp: BlockPointer, data: Vec<u8>) {
        self.time += 1;
        let key = self.block_key(&bp);
        let size = data.len();
        
        // If the block is already in cache, update it
        if self.cache.contains_key(&key) {
            if let Some(entry) = self.cache.get_mut(&key) {
                entry.data = data;
                entry.freq += 1;
                entry.last_used = self.time;
                
                // Move to appropriate position
                if self.mru.contains(&key) {
                    self.mru.move_to_front(key);
                } else if self.mfu.contains(&key) {
                    self.mfu.move_to_front(key);
                }
                return;
            }
        }
        
        // Evict entries if needed to make space
        while self.current_size + size > self.target_size {
            self.evict();
        }
        
        // Insert the new entry
        let entry = ARCEntry {
            bp,
            data,
            freq: 1,
            last_used: self.time,
        };
        
        self.cache.insert(key, entry);
        
        // Adaptive replacement algorithm
        if self.mru.len() + self.mfu.len() == self.c {
            if self.mru.len() > 0 {
                self.mru_ghost.push_back(self.mru.pop_back().unwrap());
            } else {
                self.mru_ghost.push_back(self.mfu.pop_back().unwrap());
            }
        }
        
        let total_cache = self.mru.len() + self.mfu.len() + self.mru_ghost.len() + self.mfu_ghost.len();
        if total_cache == 2 * self.c {
            if (self.mru_ghost.len() > self.mfu_ghost.len() && self.mfu.len() > 0) ||
               (self.mru.len() == 0 && self.mfu.len() > 0) {
                self.mfu_ghost.push_back(self.mfu.pop_back().unwrap());
            } else {
                self.mru_ghost.push_back(self.mru.pop_back().unwrap());
            }
        }
        
        // Update target ratio p
        let delta = if self.mru_ghost.len() >= self.mfu_ghost.len() && self.mfu.len() > 0 {
            1
        } else if self.mru.len() > 0 {
            -1
        } else {
            0
        };
        
        if delta != 0 {
            self.p = (self.p * (self.c as f64 - 1.0) + delta as f64) / self.c as f64;
        }
        
        // Decide which list to add to
        let mfu_target = (self.p * self.c as f64) as usize;
        if self.mfu.len() + (if self.mru.contains(&key) { 1 } else { 0 }) < mfu_target {
            self.mfu.push_front(key);
        } else {
            self.mru.push_front(key);
        }
        
        self.current_size += size;
    }

    /// Evict an entry from the cache using adaptive policy
    fn evict(&mut self) {
        // Check if we should evict from MRU or MFU based on target ratio
        let mfu_target = (self.p * self.c as f64) as usize;
        
        if self.mru.len() > 0 && (self.mru.len() > mfu_target || self.mfu.len() < mfu_target) {
            // Evict from MRU
            if let Some(key) = self.mru.pop_back() {
                if let Some(entry) = self.cache.remove(&key) {
                    self.current_size -= entry.data.len();
                    self.evictions += 1;
                    self.mru_ghost.push_front(key);
                }
            }
        } else if self.mfu.len() > 0 {
            // Evict from MFU
            if let Some(key) = self.mfu.pop_back() {
                if let Some(entry) = self.cache.remove(&key) {
                    self.current_size -= entry.data.len();
                    self.evictions += 1;
                    self.mfu_ghost.push_front(key);
                }
            }
        }
    }

    /// Generate a unique key for a block
    fn block_key(&self, bp: &BlockPointer) -> u64 {
        // Combine vdev_id and offset to create a unique key
        (bp.vdev_id << 32) | (bp.offset as u64)
    }

    /// Get cache statistics
    pub fn stats(&self) -> (u64, u64, u64, usize) {
        (self.hits, self.misses, self.evictions, self.current_size)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.mru.clear();
        self.mfu.clear();
        self.mru_ghost.clear();
        self.mfu_ghost.clear();
        self.current_size = 0;
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
        self.p = 0.0;
        self.time = 0;
    }

    /// Get cache utilization
    pub fn utilization(&self) -> f64 {
        self.current_size as f64 / self.target_size as f64
    }

    /// Get hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses > 0 {
            self.hits as f64 / (self.hits + self.misses) as f64
        } else {
            0.0
        }
    }
}