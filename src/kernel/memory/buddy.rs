use alloc::vec::Vec;

pub struct BuddyAllocator {
    min_order: usize,
    max_order: usize,
    free_lists: Vec<Vec<*mut u8>>,
    total_memory: usize,
    used_memory: usize,
}

impl BuddyAllocator {
    pub const fn new(min_order: usize, max_order: usize, initial_memory: usize) -> Self {
        let free_lists = Vec::new();
        BuddyAllocator {
            min_order,
            max_order,
            free_lists,
            total_memory: initial_memory,
            used_memory: 0,
        }
    }

    pub fn init_memory(&mut self, start: *mut u8, size: usize) {
        self.total_memory = size;
        // Ensure free_lists has enough entries up to max_order
        while self.free_lists.len() <= self.max_order {
            self.free_lists.push(Vec::new());
        }
        self.free_lists.get_mut(self.max_order).unwrap().push(start);
    }

    pub fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        let mut order = self.min_order;
        while (1 << order) < size { order += 1; }
        let mut current_order = order;
        // Ensure free_lists has entries up to current_order
        while self.free_lists.len() <= self.max_order {
            self.free_lists.push(Vec::new());
        }
        while current_order <= self.max_order {
            let lst = self.free_lists.get_mut(current_order).unwrap();
            if !lst.is_empty() {
                let block = lst.pop().unwrap();
                while current_order > order {
                    current_order -= 1;
                    let buddy = self.get_buddy(block, current_order);
                    let lst2 = self.free_lists.get_mut(current_order).unwrap();
                    lst2.push(buddy);
                }
                self.used_memory += 1 << order;
                return Some(block);
            }
            current_order += 1;
        }
        None
    }

    pub fn dealloc(&mut self, mut ptr: *mut u8, size: usize) {
        let mut order = self.min_order;
        while (1 << order) < size { order += 1; }
        self.used_memory -= 1 << order;
        let mut current_order = order;
        loop {
            let buddy = self.get_buddy(ptr, current_order);
            let mut found = false;
            let lst = self.free_lists.get_mut(current_order).unwrap();
            for i in 0..lst.len() {
                if lst[i] == buddy {
                    lst.swap_remove(i);
                    found = true;
                    break;
                }
            }
            if !found {
                lst.push(ptr);
                break;
            }
            let merged = if ptr < buddy { ptr } else { buddy };
            current_order += 1;
            if current_order > self.max_order {
                self.free_lists.get_mut(self.max_order).unwrap().push(merged);
                break;
            }
            ptr = merged;
        }
    }

    fn get_buddy(&self, block: *mut u8, order: usize) -> *mut u8 {
        let block_addr = block as usize;
        let block_size = 1 << order;
        let buddy_addr = if block_addr & block_size == 0 { block_addr + block_size } else { block_addr - block_size };
        buddy_addr as *mut u8
    }

    pub fn stats(&self) -> (usize, usize, usize) { (self.total_memory, self.used_memory, self.total_memory - self.used_memory) }
    pub fn fragmentation(&self) -> f64 {
        let mut free_blocks = 0;
        let mut total_free = 0;
        for (order, lst) in self.free_lists.iter().enumerate() {
            free_blocks += lst.len();
            total_free += lst.len() * (1 << order);
        }
        if free_blocks == 0 { 0.0 } else { (free_blocks as f64) / (total_free as f64 / (1 << self.min_order) as f64) }
    }
}
