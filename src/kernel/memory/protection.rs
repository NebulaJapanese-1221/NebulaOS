use alloc::vec::Vec;

pub struct MemoryProtection {
    page_tables: Vec<u32>,
    current_process: usize,
}

impl MemoryProtection {
    pub fn new() -> Self { MemoryProtection { page_tables: Vec::new(), current_process: 0 } }
    pub fn create_address_space(&mut self) -> usize {
        self.page_tables.push(0);
        self.page_tables.len() - 1
    }
    pub fn switch_to(&mut self, process_id: usize) {
        if process_id >= self.page_tables.len() { return; }
        self.current_process = process_id;
    }
    pub fn map_page(&mut self, _virt: u32, _phys: u32, _flags: u64) {}
    pub fn unmap_page(&mut self, _virt: u32) {}
    pub fn protect_region(&mut self, _start: u32, _end: u32, _flags: u64) {}
    pub fn current_process(&self) -> usize { self.current_process }
    pub fn destroy_address_space(&mut self, process_id: usize) {
        if process_id >= self.page_tables.len() { return; }
        self.page_tables.remove(process_id);
        if self.current_process >= self.page_tables.len() { self.current_process = 0; }
    }
}
