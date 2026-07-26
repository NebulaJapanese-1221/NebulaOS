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
#[repr(C, align(4096))]
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

/// Map a virtual page to a physical page
pub fn map_page(virt: u64, phys: u64, flags: u64) {
    // TODO: implement page table mapping for x86_64
}

/// Unmap a virtual page
pub fn unmap_page(virt: u64) {
    // TODO: implement page unmapping
}

