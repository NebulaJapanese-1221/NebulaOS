use core::arch::asm;
extern crate alloc;
use core::sync::atomic::Ordering;
use core::ptr::addr_of_mut;
use spin::Mutex;

// Paging Flags
pub const FLAG_PRESENT: u32 = 1 << 0;
pub const FLAG_WRITABLE: u32 = 1 << 1;
pub const FLAG_USER: u32 = 1 << 2;
pub const FLAG_HUGE: u32 = 1 << 7; // PS bit (2MB pages in PAE mode)
pub const FLAG_COW: u32 = 1 << 9;  // Custom OS flag for Copy-on-Write

/// Page Directory Pointer Table (4 entries, each mapping 1GB)
#[repr(align(4096))]
struct Pdpt([u64; 4]);

/// Page Directory (512 entries, each mapping 2MB)
#[repr(align(4096))]
struct PageDirectory([u64; 512]);

static mut PDPT: Pdpt = Pdpt([0; 4]);
static mut PAGE_DIRECTORY: [PageDirectory; 4] = [PageDirectory([0; 512]); 4];

/// Initial Page Tables to identity map early kernel memory
#[repr(align(4096))]
struct PageTables([[u64; 512]; 64]);

static mut PAGE_TABLES: PageTables = PageTables([[0; 512]; 64]);

/// A dedicated memory pool for paging structures (Directories and Tables)
#[repr(align(4096))]
#[allow(dead_code)]
struct FramePool([u8; 512 * 4096]); // 2MB reserved pool (512 frames)

static mut FRAME_POOL: FramePool = FramePool([0; 512 * 4096]);

/// Bitmap to track frame allocation (1 bit = 1 frame). 512 frames / 8 = 64 bytes.
static FRAME_BITMAP: Mutex<[u8; 64]> = Mutex::new([0; 64]);

/// Returns true if the address resides within the managed system frame pool.
pub fn is_in_pool(addr: usize) -> bool {
    let pool_start = core::ptr::addr_of!(FRAME_POOL) as usize;
    addr >= pool_start && addr < pool_start + (512 * 4096)
}

/// Allocates a zeroed 4KB frame from the system frame pool.
pub fn allocate_frame() -> Option<*mut [u64; 512]> {
    let mut bitmap = FRAME_BITMAP.lock();
    for i in 0..64 {
        if bitmap[i] != 0xFF {
            for bit in 0..8 {
                if (bitmap[i] & (1 << bit)) == 0 {
                    bitmap[i] |= 1 << bit;
                    let idx = i * 8 + bit;
                    unsafe {
                        let ptr = (addr_of_mut!(FRAME_POOL) as *mut u8).add(idx * 4096) as *mut [u64; 512];
                        core::ptr::write_bytes(ptr, 0, 1);
                        return Some(ptr);
                    }
                }
            }
        }
    }
    None
}

/// Returns a frame to the system frame pool.
pub fn free_frame(ptr: *mut [u64; 512]) {
    unsafe {
        let addr = ptr as usize;
        let pool_start = addr_of_mut!(FRAME_POOL) as usize;
        if addr < pool_start || addr >= pool_start + (512 * 4096) {
            return; // Not in pool
        }

        let idx = (addr - pool_start) / 4096;
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;

        let mut bitmap = FRAME_BITMAP.lock();
        bitmap[byte_idx] &= !(1 << bit_idx);
        // Clear memory to prevent data leakage between processes
        core::ptr::write_bytes(ptr, 0, 1);
    }
}

/// Represents a virtual address space (a Page Directory).
pub struct VirtualAddressSpace {
    /// Pointer to the 4KB-aligned Page Directory.
    pub directory: *mut [u64; 512],
    /// If true, this VAS owns its directory and should return it to the pool (if we had a free list).
    owned: bool,
}

unsafe impl Send for VirtualAddressSpace {}
unsafe impl Sync for VirtualAddressSpace {}

impl VirtualAddressSpace {
    /// Returns the index in the Page Directory where the kernel's identity map ends.
    /// Everything at or above this index is considered user-space.
    fn kernel_boundary_idx(&self) -> usize {
        let mem_limit = crate::kernel::CONFIG.total_memory.load(Ordering::Relaxed);
        // Kernel boundary in PAE: each PDE maps 2MB. Index 512 = 1GB.
        ((mem_limit + 0x4000000).max(1024 * 1024 * 1024)) >> 21
    }
}

impl Drop for VirtualAddressSpace {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                // 1. Free all dynamically allocated page tables in this directory
                let limit_idx = self.kernel_boundary_idx();

                for i in limit_idx..512 {
                    let entry = (*self.directory)[i];
                    
                    // CRITICAL: Protection for shared kernel structures.
                    // If the entry matches the master directory, it is a shared table and must not be freed.
                    if entry == unsafe { (*addr_of!(PAGE_DIRECTORY))[0].0[i] } {
                        continue;
                    }

                    if (entry & FLAG_PRESENT) != 0 && (entry & FLAG_HUGE) == 0 {
                        let table_ptr = (entry & !0xFFF) as *mut [u64; 512];
                        if is_in_pool(table_ptr as usize) {
                            free_frame(table_ptr);
                        }
                    }
                }
                // 2. Free the directory itself
                free_frame(self.directory);
            }
        }
    }
}

impl VirtualAddressSpace {
    /// Returns a wrapper for the master kernel address space.
    pub fn kernel() -> Self {
        unsafe {
            Self {
                directory: addr_of_mut!(PAGE_DIRECTORY[0]) as *mut [u64; 512],
                owned: false,
            }
        }
    }

    /// Creates a new address space by cloning the kernel's page directory.
    /// This ensures that the kernel memory remains mapped in the new space.
    pub fn new_user() -> Option<Self> {
        unsafe {
            let ptr = allocate_frame()?;

            // Clone the current kernel page directory to share kernel-space mappings
            core::ptr::copy_nonoverlapping((*addr_of!(PAGE_DIRECTORY))[0].0.as_ptr(), ptr as *mut u64, 512);
            
            let mut vas = Self { directory: ptr, owned: true };
            let limit_idx = vas.kernel_boundary_idx();

            // Only set the User bit on entries that are NOT part of the reserved kernel space.
            // This enforces hardware-level isolation for the kernel's identity map.
            for i in limit_idx..512 {
                if ((*ptr)[i] & FLAG_PRESENT as u64) != 0 {
                    (*ptr)[i] |= FLAG_USER as u64;
                }
            }
            Some(vas)
        }
    }

    /// Switches the CPU to this address space.
    pub unsafe fn switch(&self) {
        (*addr_of_mut!(PDPT)).0[0] = (self.directory as u64) | 1; // Map first 1GB
        asm!("mov cr3, {}", in(reg) addr_of_mut!(PDPT) as u32);
    }

    /// Internal helper to retrieve a page table for a directory index, creating it if necessary.
    fn get_or_create_table(&self, pd_idx: usize) -> Option<*mut [u64; 512]> {
        unsafe {
            let entry = (*self.directory)[pd_idx];
            if (entry & FLAG_PRESENT as u64) != 0 {
                // If this is a Huge Page, we must "split" it into 4KB pages to allow
                // granular operations like unmapping a Stack Guard page.
                if (entry & FLAG_HUGE as u64) != 0 {
                    let new_table = allocate_frame()?;
                    let base_phys = entry & !0x1FFFFF; // 2MB Alignment
                    let pd_flags = (entry & 0xFFF) & !(FLAG_HUGE as u64);

                    // Populate new table for the 2MB range
                    for i in 0..512 {
                        (*new_table)[i] = base_phys + (i as u64 * 4096) | pd_flags;
                    }

                    // Replace the Huge Page entry with the new Page Table
                    (*self.directory)[pd_idx] = (new_table as u64) | pd_flags | FLAG_PRESENT as u64 | FLAG_WRITABLE as u64;
                    
                    // Flush TLB for the 2MB region
                    asm!("invlpg [{}]", in(reg) base_phys as u32);
                    
                    return Some(new_table);
                }
                return Some((entry & !0xFFF) as *mut [u64; 512]);
            }

            // Use pre-allocated tables for first 1GB range in early boot
            let ptr = if pd_idx < 64 {
                (*core::ptr::addr_of_mut!(PAGE_TABLES)).0.as_mut_ptr().add(pd_idx)
            } else {
                allocate_frame()?
            };

            // Always set USER on the Directory Entry to allow PTEs to control access.
            (*self.directory)[pd_idx] = (ptr as u64) | FLAG_PRESENT as u64 | FLAG_WRITABLE as u64 | FLAG_USER as u64;
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
            
            if (flags & FLAG_HUGE) != 0 && (current & 0x1FFFFF) == 0 && (phys & 0x1FFFFF) == 0 && (current + 0x200000 <= end) {
                unsafe { (*self.directory)[(current >> 21) & 0x1FF] = (phys as u64) | flags as u64 | FLAG_PRESENT as u64; }
                current += 0x200000;
            } else {
                self.map_page(current, phys, flags & !FLAG_HUGE);
                current += 4096;
            }
        }
    }

    /// Unmaps a specific 4KB page by clearing the Present bit.
    pub fn unmap_page(&self, vaddr: usize) {
        let vaddr = vaddr & !0xFFF;
        let pd_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;

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
        let pd_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;

        unsafe {
            if let Some(pt_ptr) = self.get_or_create_table(pd_idx) {
                let pt_entry = (*pt_ptr).as_mut_ptr().add(pt_idx);
                *pt_entry = (paddr as u64) | flags as u64 | FLAG_PRESENT as u64;
                
                // Only invalidate TLB if paging is already active
                let cr0: u32;
                asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack));
                if (cr0 & 0x80000000) != 0 {
                    asm!("invlpg [{}]", in(reg) vaddr);
                }
            }
        }
    }

    pub fn get_page_entry(&self, vaddr: usize) -> Option<u64> {
        let pd_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;

        let entry = unsafe { (*self.directory)[pd_idx] };
        // Check if present and NOT a huge page
        if (entry & FLAG_PRESENT as u64) == 0 || (entry & FLAG_HUGE as u64) != 0 { return None; }
        
        let pt_ptr = (entry & !0xFFF) as *const [u64; 512];
        Some(unsafe { (*pt_ptr)[pt_idx] })
    }
}

pub fn get_kernel_pd_ptr() -> u32 {
    unsafe { addr_of_mut!(PDPT) as u32 }
}

pub fn init(fb_info: Option<(usize, usize, usize, usize, u8)>) {
    unsafe {
        crate::serial_println!("[INFO] Paging: Building Identity Map...");
        let kernel_vas = VirtualAddressSpace::kernel();

        // 1. Enable Physical Address Extension (PAE) and PSE
        let mut cr4: u32;
        asm!("mov {}, cr4", out(reg) cr4);
        cr4 |= 0x20 | 0x10; // Bit 5: PAE, Bit 4: PSE
        asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));

        // 2. Identity map early memory using 2MB Huge Pages
        let mem_limit = crate::kernel::CONFIG.total_memory.load(Ordering::Relaxed);
        let map_size = (mem_limit + 0x4000000).max(1024 * 1024 * 1024); // RAM + 64MB margin, min 1GB
        kernel_vas.map_region(0, 0, map_size, FLAG_PRESENT | FLAG_WRITABLE | FLAG_HUGE);

        // Initialize PDPT
        (*addr_of_mut!(PDPT)).0[0] = (addr_of_mut!(PAGE_DIRECTORY[0]) as u64) | 1;

        // 3. Identity map the Framebuffer (if detected)
        if let Some((addr, _w, _h, pitch, _bpp)) = fb_info {
            let fb_addr = addr as u32;
            // Use height * pitch for the full buffer size
            let fb_size = (pitch * 1024) as u32; // Overestimate to cover common resolutions
            kernel_vas.map_region(fb_addr as usize, fb_addr as usize, fb_size as usize, FLAG_PRESENT | FLAG_WRITABLE | FLAG_HUGE);
        }

        crate::serial_println!("[INFO] Paging: Enabling MMU...");

        // 4. Load Page Directory into CR3 (Physical Address)
        kernel_vas.switch();

        // 5. Enable Paging by setting the PG bit (31) in CR0.
        // We perform a pipeline flush jump immediately after to ensure synchronization.
        let mut cr0: u32;
        asm!("mov {0}, cr0", out(reg) cr0, options(nomem, nostack));
        cr0 |= 0x80000000; // Bit 31: PG (Paging)
        
        asm!(
            "mov cr0, {0}",
            "jmp 2f",
            "2:",
            in(reg) cr0,
            options(nostack, nomem)
        );

        // 6. Now that Paging is stable, enable the WP bit (16) to enforce Read-Only pages
        cr0 |= 0x00010000; // Bit 16: WP (Write Protect)
        asm!("mov cr0, {0}", in(reg) cr0, options(nomem, nostack));
        
        crate::serial_println!("[INFO] Paging: System Active.");
    }
}