use core::arch::asm;
extern crate alloc;
use core::sync::atomic::Ordering;
use core::ptr::{addr_of, addr_of_mut};
use spin::Mutex;

// Paging Flags
pub const FLAG_PRESENT: u64 = 1 << 0;
pub const FLAG_WRITABLE: u64 = 1 << 1;
pub const FLAG_USER: u64 = 1 << 2;
pub const FLAG_HUGE: u64 = 1 << 7; // PS bit (2MB pages in PAE mode)
pub const FLAG_COW: u64 = 1 << 9;  // Custom OS flag for Copy-on-Write
pub const FLAG_NX: u64 = 1 << 63;  // No-Execute bit

#[derive(Clone, Copy, Debug)]
/// Page Directory Pointer Table (4 entries, each mapping 1GB)
#[repr(align(4096))]
struct Pdpt([u64; 4]);
pub(crate) struct Pdpt([u64; 4]);

#[derive(Clone, Copy, Debug)]
/// Page Directory (512 entries, each mapping 2MB)
#[repr(align(4096))]
struct PageDirectory([u64; 512]);
pub(crate) struct PageDirectory([u64; 512]);

#[no_mangle]
static mut KERNEL_PDPT: Pdpt = Pdpt([0; 4]);
static mut PAGE_DIRECTORY: [PageDirectory; 4] = [const { PageDirectory([0; 512]) }; 4];

#[derive(Clone, Copy)]
/// Initial Page Tables to identity map early kernel memory
#[repr(align(4096))]
struct PageTables([[u64; 512]; 64]);

static mut PAGE_TABLES: PageTables = PageTables([[0; 512]; 64]);

#[derive(Clone, Copy)]
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
    /// Pointer to the 4KB-aligned Page Directory Pointer Table.
    pub pdpt: *mut Pdpt,
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
                // Iterate through all 4 PDPT entries
                for pdpt_idx in 0..4 {
                    let pdpt_entry = (*self.pdpt).0[pdpt_idx];
                    if (pdpt_entry & FLAG_PRESENT) != 0 {
                        let pd_ptr = (pdpt_entry & !0xFFF) as *mut PageDirectory;
                        
                        // Check if this PageDirectory is one of the kernel's static ones.
                        // If it is, we must NOT free it or its contents.
                        // We cast to *const PageDirectory to ensure pointer arithmetic advances by individual directories,
                        // and we wrap 'as usize' in parentheses to avoid ambiguity with generic arguments (<).
                        let is_kernel_static_pd = ((pd_ptr as usize) >= (addr_of!(PAGE_DIRECTORY) as usize)) &&
                                                  ((pd_ptr as usize) < (addr_of!(PAGE_DIRECTORY) as *const PageDirectory).add(4) as usize);

                        if !is_kernel_static_pd {
                            // This PageDirectory was dynamically allocated for this user VAS.
                            // We need to free its contents (PageTables and mapped frames) and then the PageDirectory itself.
                            for pde_idx in 0..512 {
                                let pde_entry = (*pd_ptr).0[pde_idx];
                                // Bitwise operators have lower precedence than comparison operators.
                                if ((pde_entry & FLAG_PRESENT) != 0) && ((pde_entry & FLAG_HUGE) == 0) {
                                    // This PDE points to a PageTable.
                                    let pt_ptr = (pde_entry & !0xFFF) as *mut [u64; 512];

                                    // Since `get_or_create_table` now always allocates new PageTables for user VAS,
                                    // we don't need to check if it's a kernel static PT.
                                    // Free all physical frames mapped by this PageTable.
                                    for pte_idx in 0..512 {
                                        let pte_entry = (*pt_ptr)[pte_idx];
                                        if (pte_entry & FLAG_PRESENT) != 0 {
                                            let paddr = pte_entry & !0xFFF;
                                            // Only free physical frames that were allocated from our pool.
                                            if is_in_pool(paddr as usize) {
                                                free_frame(paddr as *mut [u64; 512]); // Free the actual data frame
                                            }
                                        }
                                    }
                                    // Free the PageTable itself.
                                    if is_in_pool(pt_ptr as usize) {
                                        free_frame(pt_ptr);
                                    }
                                }
                            }
                            // Free the PageDirectory itself.
                            if is_in_pool(pd_ptr as usize) {
                                free_frame(pd_ptr as *mut [u64; 512]);
                            }
                        }
                    }
                }
                // 2. Free the PDPT itself
                free_frame(self.pdpt as *mut [u64; 512]);
            }
        }
    }
}

impl VirtualAddressSpace {
    /// Returns a wrapper for the master kernel address space.
    pub fn kernel() -> Self {
        unsafe {
            Self {
                pdpt: addr_of_mut!(KERNEL_PDPT) as *mut Pdpt, // Correctly points to the static KERNEL_PDPT
                owned: false,
            }
        }
    }

    /// Creates a new address space by cloning the kernel's page directory.
    /// This ensures that the kernel memory remains mapped in the new space.
    pub fn new_user() -> Option<Self> {
        unsafe {
            let pdpt_ptr = allocate_frame()? as *mut Pdpt;
            
            // Allocate 4 private Page Directories for this address space
            for i in 0..4 {
                let pd_ptr = allocate_frame()? as *mut PageDirectory;
                // Copy the kernel's identity map and initial mappings from the static directories
                core::ptr::copy_nonoverlapping(addr_of!(PAGE_DIRECTORY[i]), pd_ptr, 1);
                
                // Link the new private directory to the PDPT.
                // We set FLAG_USER so Ring 3 can traverse this path to reach user-mapped pages.
                (*pdpt_ptr).0[i] = (pd_ptr as u64) | FLAG_PRESENT | FLAG_WRITABLE | FLAG_USER;
            }
            
            Some(Self { pdpt: pdpt_ptr, owned: true })
        }
    }

    /// Switches the CPU to this address space.
    pub unsafe fn switch(&self) {
        asm!("mov cr3, {}", in(reg) self.pdpt as u32); // CR3 points to the PDPT in PAE mode
    }

    /// Internal helper to retrieve a page table for a directory index, creating it if necessary.
    fn get_or_create_table(&self, vaddr: usize) -> Option<*mut [u64; 512]> {
        unsafe {
            let pdpt_idx = (vaddr >> 30) & 0x03;
            let pd_idx = (vaddr >> 21) & 0x1FF;
            
            let pdpt_entry = (*self.pdpt).0[pdpt_idx];
            let pd_ptr = if (pdpt_entry & FLAG_PRESENT) == 0 {
                // PageDirectory not present, allocate a new one
                let new_pd = allocate_frame()? as *mut PageDirectory;
                let mut pdpt_flags = FLAG_PRESENT | FLAG_WRITABLE;
                if self.owned { pdpt_flags |= FLAG_USER; } // Set USER flag if this is a user VAS
                (*self.pdpt).0[pdpt_idx] = (new_pd as u64) | pdpt_flags;
                new_pd
            } else {
                (pdpt_entry & !0xFFF) as *mut PageDirectory
            };

            let entry = (*pd_ptr).0[pd_idx];
            if (entry & FLAG_PRESENT) != 0 {
                // If this is a Huge Page, we must "split" it into 4KB pages to allow
                // granular operations like unmapping a Stack Guard page.
                if (entry & FLAG_HUGE) != 0 {
                    let new_table = allocate_frame()?;
                    let base_phys = entry & !0x1FFFFF; // 2MB Alignment
                    let pd_flags = (entry & 0x8000000000000FFF) & !FLAG_HUGE; // Keep NX and standard flags
                    let mut pd_flags = (entry & 0x8000000000000FFF) & !FLAG_HUGE; 
                    // Ensure User mode permission is propagated if this is a user address space
                    if self.owned { pd_flags |= FLAG_USER; }

                    // Populate new table for the 2MB range
                    for i in 0..512 {
                        (*new_table)[i] = base_phys + (i as u64 * 4096) | pd_flags;
                    }

                    // Replace the Huge Page entry with the new Page Table
                    (*pd_ptr).0[pd_idx] = (new_table as u64) | pd_flags | FLAG_PRESENT | FLAG_WRITABLE;
                    
                    // Flush TLB for the 2MB region
                    asm!("invlpg [{}]", in(reg) base_phys as u32);
                    
                    return Some(new_table);
                }
                return Some((entry & !0xFFF) as *mut [u64; 512]);
            }

            // Allocate a new PageTable. For user VAS, this ensures isolation.
            // For kernel VAS, dynamic allocation is generally fine after early boot.
            let ptr = allocate_frame()?;

            // Always set USER on the Directory Entry to allow PTEs to control access.
            let mut new_pde_flags = FLAG_PRESENT | FLAG_WRITABLE;
            if self.owned { // If this is a user VAS, set the USER flag on the PDE
                new_pde_flags |= FLAG_USER;
            }
            (*pd_ptr).0[pd_idx] = (ptr as u64) | new_pde_flags; // Update the PDE in the correct PageDirectory
            Some(ptr)
        }
    }

    /// Maps a contiguous virtual memory region to a physical region in this address space.
    pub fn map_region(&self, vaddr: usize, paddr: usize, size: usize, flags: u64) {
        let start = vaddr & !0xFFF;
        let end = (vaddr + size + 4095) & !0xFFF;
        let offset = paddr.wrapping_sub(vaddr);

        let mut current = start;
        while current < end {
            let phys = current.wrapping_add(offset);
            
            if (flags & FLAG_HUGE) != 0 && (current & 0x1FFFFF) == 0 && (phys & 0x1FFFFF) == 0 && (current + 0x200000 <= end) {
                let pdpt_idx = current >> 30;
                let pde_idx = (current >> 21) & 0x1FF;

                let pdpt_entry = unsafe { (*self.pdpt).0[pdpt_idx] }; // Get the PDPT entry
                let page_directory_ptr = if (pdpt_entry & FLAG_PRESENT) == 0 { // If PageDirectory not present
                    let new_pd = allocate_frame().expect("Failed to allocate PageDirectory for huge page mapping") as *mut PageDirectory; // Allocate a new PageDirectory
                    let pdpt_flags = if self.owned { FLAG_PRESENT | FLAG_WRITABLE | FLAG_USER } else { FLAG_PRESENT | FLAG_WRITABLE };
                    unsafe { (*self.pdpt).0[pdpt_idx] = (new_pd as u64) | pdpt_flags; } // Update PDPT entry
                    new_pd
                } else {
                    (pdpt_entry & !0xFFF) as *mut PageDirectory
                };
                unsafe { (*page_directory_ptr).0[pde_idx] = (phys as u64) | flags | FLAG_PRESENT; }
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
        let pdpt_idx = vaddr >> 30;
        let pde_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;

        unsafe {
            let pdpt_entry = (*self.pdpt).0[pdpt_idx];
            if (pdpt_entry & FLAG_PRESENT) == 0 { return; }
            let page_directory_ptr = (pdpt_entry & !0xFFF) as *mut PageDirectory; // Get the PageDirectory pointer

            let pde_entry = (*page_directory_ptr).0[pde_idx]; // Get the PDE entry
            if (pde_entry & FLAG_PRESENT) == 0 || (pde_entry & FLAG_HUGE) != 0 { return; } // No PageTable or it's a huge page
            let pt_ptr = (pde_entry & !0xFFF) as *mut [u64; 512]; // Get the PageTable pointer

            // We directly access the PageTable, no need to create it if not present for unmap
                let pt_entry = (*pt_ptr).as_mut_ptr().add(pt_idx);
                *pt_entry = 0;
                asm!("invlpg [{}]", in(reg) vaddr);
        }
    }

    /// Maps a specific 4KB page to a physical address.
    pub fn map_page(&self, vaddr: usize, paddr: usize, flags: u64) {
        let vaddr = vaddr & !0xFFF;
        let paddr = paddr & !0xFFF;
        let pt_idx = (vaddr >> 12) & 0x1FF;

        unsafe {
            if let Some(pt_ptr) = self.get_or_create_table(vaddr) { // This will create the PageTable if not present
                let pt_entry = (*pt_ptr).as_mut_ptr().add(pt_idx);
                *pt_entry = (paddr as u64) | flags | FLAG_PRESENT;
                
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
        let pdpt_idx = vaddr >> 30;
        let pde_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;

        let pdpt_entry = unsafe { (*self.pdpt).0[pdpt_idx] };
        if (pdpt_entry & FLAG_PRESENT) == 0 { return None; }
        let page_directory_ptr = (pdpt_entry & !0xFFF) as *mut PageDirectory;
        
        let entry = unsafe { (*page_directory_ptr).0[pde_idx] };
        // Check if present and NOT a huge page
        if (entry & FLAG_PRESENT) == 0 || (entry & FLAG_HUGE) != 0 { return None; }
        
        let pt_ptr = (entry & !0xFFF) as *const [u64; 512];
        Some(unsafe { (*pt_ptr)[pt_idx] })
    }
}

pub fn get_kernel_pd_ptr() -> u32 {
    addr_of_mut!(KERNEL_PDPT) as u32
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

        // Enable NX support in hardware
        crate::kernel::cpu::enable_nx();

        // 1.5 Initialize PDPT entries to point to our static Page Directories FIRST.
        // This ensures map_region writes to the directories we actually intend to use.
        for i in 0..4 {
            let pd_phys = (addr_of_mut!(PAGE_DIRECTORY) as *mut PageDirectory).add(i) as u64;
            (*addr_of_mut!(KERNEL_PDPT)).0[i] = pd_phys | FLAG_PRESENT | FLAG_WRITABLE;
        }

        // 2. Identity map early memory using 2MB Huge Pages
        let mem_limit = crate::kernel::CONFIG.total_memory.load(Ordering::Relaxed);
        let map_size = (mem_limit + 0x4000000).max(1024 * 1024 * 1024); // RAM + 64MB margin, min 1GB
        kernel_vas.map_region(0, 0, map_size, FLAG_PRESENT | FLAG_WRITABLE | FLAG_HUGE);

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