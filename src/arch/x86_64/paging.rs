// x86_64 (64-bit) Paging implementation
// Uses 4-level page tables (PML4, PDPT, PD, PT) with 4KB pages

use core::arch::asm;

pub const PAGE_PRESENT: u64 = 1 << 0;
pub const PAGE_WRITABLE: u64 = 1 << 1;
pub const PAGE_USER: u64 = 1 << 2;
pub const PAGE_SIZE_4KB: u64 = 0x1000;        // 4KB
pub const PAGE_SIZE_2MB: u64 = 0x200000;       // 2MB
pub const PAGE_SIZE_1GB: u64 = 0x40000000;     // 1GB

/// A 64-bit page table entry
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageEntry {
    entry: u64,
}

impl PageEntry {
    pub fn new(physical_addr: u64, flags: u64) -> Self {
        PageEntry {
            entry: (physical_addr & 0x000F_FFFF_FFFF_F000) | flags,
        }
    }

    pub fn is_present(&self) -> bool {
        self.entry & 1 != 0
    }

    pub fn get_physical_addr(&self) -> u64 {
        self.entry & 0x000F_FFFF_FFFF_F000
    }

    pub fn set_flags(&mut self, flags: u64) {
        self.entry |= flags;
    }

    pub fn clear_flags(&mut self, flags: u64) {
        self.entry &= !flags;
    }

    /// Check if this entry maps a large page (2 MB in PD, 1 GB in PDPT).
    /// The PS (Page Size) bit is bit 7.
    pub fn is_large_page(&self) -> bool {
        self.entry & (1 << 7) != 0
    }
}

const PAGE_TABLE_SIZE: usize = 512; // 512 entries per table

/// PML4 table (Level 4) - root of the 4-level page table hierarchy
#[repr(C, align(4096))]
pub struct PageMapLevel4 {
    entries: [PageEntry; PAGE_TABLE_SIZE],
}

impl PageMapLevel4 {
    pub const fn new() -> Self {
        PageMapLevel4 {
            entries: [PageEntry { entry: 0 }; PAGE_TABLE_SIZE],
        }
    }
}

/// Static PML4 table for the kernel, must be page-aligned
static mut KERNEL_PML4: PageMapLevel4 = PageMapLevel4::new();

/// Initialize 4-level paging for x86_64
pub unsafe fn init_paging() {
    // Identity map first 4MB of physical memory (kernel code/data)
    let flags = PAGE_PRESENT | PAGE_WRITABLE;

    // The kernel PML4 is already zero-initialized, we need to set up:
    // - PML4[0] -> PDP table
    // - PDP[0]  -> Page Directory
    // - PD[0]   -> Page Table (or 2MB pages)
    //
    // For simplicity, use 2MB pages for the first 4MB of kernel space.

    // Allocate page table structures at known addresses
    // In a real OS this would use the allocator, but here we use
    // static tables at fixed addresses for early boot

    // Set up PML4 entry 0 - points to a PDP table
    // For now, we'll set up a minimal identity mapping
    let pml4_idx = 0; // Identity map region
    let pdpt_phys = 0x2000; // Place PDPT at 8KB
    let pd_phys = 0x3000;   // Place PD at 12KB
    let pt_phys = 0x4000;   // Place PT at 16KB

    // Map PML4[0] -> PDPT at 0x2000
    KERNEL_PML4.entries[pml4_idx] = PageEntry::new(pdpt_phys, flags);

    // Set up PDPT[0] -> PD at 0x3000
    let pdpt = &mut *(pdpt_phys as *mut [PageEntry; 512]);
    pdpt[0] = PageEntry::new(pd_phys, flags);

    // Set up PD[0] -> PT at 0x4000
    let pd = &mut *(pd_phys as *mut [PageEntry; 512]);
    pd[0] = PageEntry::new(pt_phys, flags | (1 << 7)); // PS bit for 2MB pages
    pd[0].entry = (0x00000000) | flags | (1 << 7); // 2MB page, identity map

    // Map 0x00000000 - 0x00400000 using 2MB page at PD[0]
    // Our 2MB page covers the first 2MB

    // Load PML4 table address into CR3
    let pml4_phys = &raw const KERNEL_PML4 as *const _ as u64;
    asm!("mov cr3, {}", in(reg) pml4_phys);

    // Set PAE (Physical Address Extension) and PGE (Page Global Enable) bits
    let mut cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4);
    cr4 |= 1 << 5; // PAE
    asm!("mov cr4, {}", in(reg) cr4);

    // Enable IA-32e mode (Long Mode) by setting LME in EFER MSR
    let efer_msr: u32 = 0xC0000080;
    let efer_val: u64;
    asm!(
        "rdmsr",
        in("ecx") efer_msr,
        out("eax") efer_val,
        out("edx") _,
    );
    let efer_val = efer_val | (1 << 8); // LME bit
    let low = efer_val as u32;
    let high = (efer_val >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") efer_msr,
        in("eax") low,
        in("edx") high,
    );

    // Enable paging by setting PG and PE bits in CR0
    let mut cr0: u64;
    asm!("mov {}, cr0", out(reg) cr0);
    cr0 |= 1 << 31; // PG
    cr0 |= 1 << 0;  // PE
    asm!("mov cr0, {}", in(reg) cr0);
}

/// Get the physical address of the kernel's PML4 table
pub fn get_kernel_pml4_phys_addr() -> u64 {
    (&raw const KERNEL_PML4 as *const _ as u64) & !0xFFF
}

/// Create a user page directory (PML4 clone with kernel entries)
pub fn create_user_page_directory() -> u64 {
    // TODO: allocate new PML4 and copy kernel entries, mapping user pages
    get_kernel_pml4_phys_addr()
}

/// Map the 4 KB virtual page `virt` to physical page `phys` with the
/// given `flags` (at least `PAGE_PRESENT`).  Intermediate tables are
/// allocated on the fly if they are missing.
///
/// # Safety
///
/// The caller must ensure the page tables are writable and that
/// `virt` is not already mapped in an incompatible way.
pub unsafe fn map_page(virt: u64, phys: u64, flags: u64) {
    let pml4_idx = ((virt >> 39) & 0x1FF) as usize;
    let pdpt_idx = ((virt >> 30) & 0x1FF) as usize;
    let pd_idx   = ((virt >> 21) & 0x1FF) as usize;
    let pt_idx   = ((virt >> 12) & 0x1FF) as usize;

    let pml4 = &raw mut KERNEL_PML4;
    let pml4e = &mut (*pml4).entries[pml4_idx];
    if !pml4e.is_present() {
        let table = alloc_page_table_x86_64();
        *pml4e = PageEntry::new(table, PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER);
    }

    let pdpt = &mut *(pml4e.get_physical_addr() as *mut [PageEntry; PAGE_TABLE_SIZE]);
    let pdpte = &mut pdpt[pdpt_idx];
    if !pdpte.is_present() {
        let table = alloc_page_table_x86_64();
        *pdpte = PageEntry::new(table, PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER);
    }

    let pd = &mut *(pdpte.get_physical_addr() as *mut [PageEntry; PAGE_TABLE_SIZE]);
    let pde = &mut pd[pd_idx];
    if !pde.is_present() {
        let table = alloc_page_table_x86_64();
        *pde = PageEntry::new(table, PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER);
    }

    let pt = &mut *(pde.get_physical_addr() as *mut [PageEntry; PAGE_TABLE_SIZE]);
    pt[pt_idx] = PageEntry::new(phys, flags);

    // Flush the TLB for this virtual address.
    asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags));
}

/// Simple temporary page-table allocator (identity mapped).
static mut NEXT_PT_X86_64: u64 = 0x300000; // 3 MB – after the x86 temporary region
const PT_ALLOC_STEP: u64 = 0x1000;          // 4 KB

unsafe fn alloc_page_table_x86_64() -> u64 {
    let addr = NEXT_PT_X86_64;
    NEXT_PT_X86_64 += PT_ALLOC_STEP;
    core::ptr::write_bytes(addr as *mut u8, 0, PT_ALLOC_STEP as usize);
    addr
}

/// Unmap the 4 KB virtual page `virt`, clearing its page-table entry
/// and flushing the TLB.
///
/// # Safety
///
/// The caller must ensure no dangling references remain.
pub unsafe fn unmap_page(virt: u64) {
    let pml4_idx = ((virt >> 39) & 0x1FF) as usize;
    let pdpt_idx = ((virt >> 30) & 0x1FF) as usize;
    let pd_idx   = ((virt >> 21) & 0x1FF) as usize;
    let pt_idx   = ((virt >> 12) & 0x1FF) as usize;

    // Walk PML4
    let pml4e = KERNEL_PML4.entries[pml4_idx];
    if !pml4e.is_present() {
        return; // nothing mapped
    }

    // Walk PDPT
    let pdpt = &*(pml4e.get_physical_addr() as *const [PageEntry; PAGE_TABLE_SIZE]);
    let pdpte = pdpt[pdpt_idx];
    if !pdpte.is_present() {
        return;
    }

    // Walk PD
    let pd = &*(pdpte.get_physical_addr() as *const [PageEntry; PAGE_TABLE_SIZE]);
    let pde = pd[pd_idx];
    if !pde.is_present() {
        return;
    }

    // If this is a 2 MB or 1 GB page, we cannot unmap a single 4 KB part.
    // For now we simply skip large pages.
    if pde.is_large_page() {
        return;
    }

    // Walk PT and clear the entry
    let pt = &mut *(pde.get_physical_addr() as *mut [PageEntry; PAGE_TABLE_SIZE]);
    pt[pt_idx] = PageEntry { entry: 0 };

    // Flush the TLB for this virtual address
    asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags));
}

