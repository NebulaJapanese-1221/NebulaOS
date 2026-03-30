use core::arch::asm;
extern crate alloc;
use core::alloc::Layout;
use core::sync::atomic::Ordering;
use alloc::alloc::{alloc_zeroed, dealloc};

// Paging Flags
pub const FLAG_PRESENT: u32 = 1 << 0;
pub const FLAG_WRITABLE: u32 = 1 << 1;
pub const FLAG_USER: u32 = 1 << 2;
pub const FLAG_HUGE: u32 = 1 << 7; // PS bit (4MB pages)
pub const FLAG_COW: u32 = 1 << 9;  // Custom OS flag for Copy-on-Write

/// Page Directory (1024 entries, each mapping 4MB)
#[repr(align(4096))]
struct PageDirectory([u32; 1024]);

static mut PAGE_DIRECTORY: PageDirectory = PageDirectory([0; 1024]);

/// Initial Page Tables to identity map the first 256MB
#[repr(align(4096))]
struct PageTables([[u32; 1024]; 64]);

static mut PAGE_TABLES: PageTables = PageTables([[0; 1024]; 64]);

/// Represents a virtual address space (a Page Directory).
pub struct VirtualAddressSpace {
    /// Pointer to the 4KB-aligned Page Directory.
    pub directory: *mut [u32; 1024],
}

unsafe impl Send for VirtualAddressSpace {}
unsafe impl Sync for VirtualAddressSpace {}

impl Drop for VirtualAddressSpace {
    fn drop(&mut self) {
        // Do not deallocate the static kernel directory
        unsafe {
            if self.directory as u32 != core::ptr::addr_of_mut!(PAGE_DIRECTORY).cast::<u32>() as u32 {
                let layout = Layout::from_size_align(4096, 4096).unwrap();
                dealloc(self.directory as *mut u8, layout);
            }
        }
    }
}

impl VirtualAddressSpace {
    /// Creates a new address space by cloning the kernel's page directory.
    /// This ensures that the kernel memory remains mapped in the new space.
    pub fn new_user() -> Option<Self> {
        unsafe {
            let layout = Layout::from_size_align(4096, 4096).ok()?;
            let ptr = alloc_zeroed(layout) as *mut [u32; 1024];
            if ptr.is_null() { return None; }

            // Clone the current kernel page directory to share kernel-space mappings
            core::ptr::copy_nonoverlapping((*core::ptr::addr_of!(PAGE_DIRECTORY)).0.as_ptr(), ptr as *mut u32, 1024);
            
            Some(Self { directory: ptr })
        }
    }

    /// Switches the CPU to this address space.
    pub unsafe fn switch(&self) {
        asm!("mov cr3, {}", in(reg) self.directory as u32);
    }

    /// Internal helper to retrieve a page table for a directory index, creating it if necessary.
    fn get_or_create_table(&self, pd_idx: usize) -> Option<*mut [u32; 1024]> {
        unsafe {
            let entry = (*self.directory)[pd_idx];
            if (entry & FLAG_PRESENT) != 0 {
                if (entry & FLAG_HUGE) != 0 { return None; }
                return Some((entry & !0xFFF) as *mut [u32; 1024]);
            }

            // Use pre-allocated tables for the first 256MB to avoid early heap noise
            let ptr = if pd_idx < 64 {
                (*core::ptr::addr_of_mut!(PAGE_TABLES)).0.as_mut_ptr().add(pd_idx)
            } else {
                let layout = Layout::from_size_align(4096, 4096).ok()?;
                let p = alloc_zeroed(layout) as *mut [u32; 1024];
                if p.is_null() { return None; }
                p
            };

            (*self.directory)[pd_idx] = (ptr as u32) | FLAG_PRESENT | FLAG_WRITABLE;
            Some(ptr)
        }
    }

    /// Maps a contiguous virtual memory region to a physical region in this address space.
    pub fn map_region(&self, vaddr: usize, paddr: usize, size: usize, flags: u32) {
        let start = vaddr & !0xFFF;
        let end = (vaddr + size + 4095) & !0xFFF;
        let offset = paddr.wrapping_sub(vaddr);

        let mut current = start;
        while current < end {
            let phys = current.wrapping_add(offset);
            
            if (flags & FLAG_HUGE) != 0 && (current & 0x3FFFFF) == 0 && (phys & 0x3FFFFF) == 0 && (current + 0x400000 <= end) {
                unsafe { (*self.directory)[current >> 22] = (phys as u32) | flags | FLAG_PRESENT; }
                current += 0x400000;
            } else {
                self.map_page(current, phys, flags & !FLAG_HUGE);
                current += 4096;
            }
        }
    }

    /// Unmaps a specific 4KB page by clearing the Present bit.
    pub fn unmap_page(&self, vaddr: usize) {
        let vaddr = vaddr & !0xFFF;
        let pd_idx = vaddr >> 22;
        let pt_idx = (vaddr >> 12) & 0x3FF;

        unsafe {
            if let Some(pt_ptr) = self.get_or_create_table(pd_idx) {
                let pt_entry = (*pt_ptr).as_mut_ptr().add(pt_idx);
                *pt_entry = 0;
                asm!("invlpg [{}]", in(reg) vaddr);
            }
        }
    }

    /// Maps a specific 4KB page to a physical address.
    pub fn map_page(&self, vaddr: usize, paddr: usize, flags: u32) {
        let vaddr = vaddr & !0xFFF;
        let paddr = paddr & !0xFFF;
        let pd_idx = vaddr >> 22;
        let pt_idx = (vaddr >> 12) & 0x3FF;

        unsafe {
            if let Some(pt_ptr) = self.get_or_create_table(pd_idx) {
                let pt_entry = (*pt_ptr).as_mut_ptr().add(pt_idx);
                *pt_entry = (paddr as u32) | flags | FLAG_PRESENT;
                asm!("invlpg [{}]", in(reg) vaddr);
            }
        }
    }

    /// Returns the raw Page Table Entry for a virtual address.
    pub fn get_page_entry(&self, vaddr: usize) -> Option<u32> {
        let pd_idx = vaddr >> 22;
        let pt_idx = (vaddr >> 12) & 0x3FF;

        unsafe {
            let entry = (*self.directory)[pd_idx];
            // Check if present and NOT a 4MB huge page (which has no page table)
            if (entry & FLAG_PRESENT) == 0 || (entry & FLAG_HUGE) != 0 { return None; }
            
            let pt_ptr = (entry & !0xFFF) as *const [u32; 1024];
            Some((*pt_ptr)[pt_idx])
        }
    }
}

/// Returns the physical address of the master kernel page directory.
pub fn get_kernel_pd_ptr() -> u32 {
    unsafe { &raw mut PAGE_DIRECTORY.0 as u32 }
}

pub fn init() {
    unsafe {
        let kernel_vas = VirtualAddressSpace { directory: core::ptr::addr_of_mut!(PAGE_DIRECTORY) as *mut [u32; 1024] };

        // Enable Page Size Extensions (PSE) in CR4
        let mut cr4: u32;
        asm!("mov {}, cr4", out(reg) cr4);
        cr4 |= 0x00000010; // Bit 4: PSE
        asm!("mov cr4, {}", in(reg) cr4);

        // 1. Identity map the kernel and RAM using 4MB Huge Pages.
        let mem_limit = crate::kernel::CONFIG.total_memory.load(Ordering::Relaxed);
        
        // Map at least 128MB for the kernel/drivers, up to 256MB for basic BSS compatibility
        let map_size = (mem_limit + 0x2000000).max(128 * 1024 * 1024).min(256 * 1024 * 1024);
        
        kernel_vas.map_region(0, 0, map_size, FLAG_PRESENT | FLAG_WRITABLE | FLAG_HUGE);

        // 2. Identity map the Framebuffer using 4MB pages (if detected)
        let fb_info = crate::drivers::framebuffer::FRAMEBUFFER.lock().info;
        if let Some(info) = fb_info {
            let fb_addr = info.address as u32;
            let fb_size = (info.width * info.height * (info.bpp as usize / 8)) as u32;
            kernel_vas.map_region(fb_addr as usize, fb_addr as usize, fb_size as usize, FLAG_PRESENT | FLAG_WRITABLE | FLAG_HUGE);
        }

        // 3. Load Page Directory into CR3 and Enable Paging
        kernel_vas.switch();

        let mut cr0: u32;
        asm!("mov {}, cr0", out(reg) cr0);
        cr0 |= 0x80000000; // Bit 31: PG
        asm!("mov cr0, {}", in(reg) cr0);
    }
}