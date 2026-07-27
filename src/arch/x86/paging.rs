// x86 (32-bit) Paging implementation
// Uses 2-level page tables (Page Directory + Page Table) with 4KB pages
// Also supports 4MB pages via the PSE flag (bit 7 of PDE)

use core::arch::asm;
use core::ptr;

// ── Page table entry flags ──────────────────────────────────────────
pub const PAGE_PRESENT:      u32 = 1 << 0;
pub const PAGE_WRITABLE:     u32 = 1 << 1;
pub const PAGE_USER:         u32 = 1 << 2;
pub const PAGE_WRITE_THRU:   u32 = 1 << 3;
pub const PAGE_CACHE_DIS:    u32 = 1 << 4;
pub const PAGE_ACCESSED:     u32 = 1 << 5;
pub const PAGE_DIRTY:        u32 = 1 << 6;
pub const PAGE_SIZE_4MB:     u32 = 1 << 7;   // PS – Page Size (only in PDE)
pub const PAGE_GLOBAL:       u32 = 1 << 8;   // PGE bit (if CR4.PGE=1)

// ── Address mask ────────────────────────────────────────────────────
const ADDR_MASK: u32 = 0xFFFF_F000; // upper 20 bits

// ── Page table dimensions ───────────────────────────────────────────
const PD_ENTRIES: usize = 1024; // page directory entries
const PT_ENTRIES: usize = 1024; // page table entries
const PAGE_SIZE_4KB: u32 = 0x1000; // 4 KB

/// A single 32-bit page table/directory entry (hardware format).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageEntry {
    entry: u32,
}

impl PageEntry {
    /// Create a new entry pointing to `physical_addr` (page-aligned) with `flags`.
    #[inline]
    pub fn new(physical_addr: u32, flags: u32) -> Self {
        PageEntry {
            entry: (physical_addr & ADDR_MASK) | (flags & !ADDR_MASK),
        }
    }

    /// Is the page present in memory?
    #[inline]
    pub fn is_present(&self) -> bool {
        self.entry & PAGE_PRESENT != 0
    }

    /// Return the physical address (page-aligned) stored in this entry.
    #[inline]
    pub fn get_physical_addr(&self) -> u32 {
        self.entry & ADDR_MASK
    }

    /// Set additional flags (OR).
    #[inline]
    pub fn set_flags(&mut self, extra: u32) {
        self.entry |= extra;
    }

    /// Clear specific flags (AND NOT).
    #[inline]
    pub fn clear_flags(&mut self, mask: u32) {
        self.entry &= !mask;
    }

    /// Check whether this PDE maps a 4 MB page (PS bit).
    #[inline]
    pub fn is_4mb_page(&self) -> bool {
        self.entry & PAGE_SIZE_4MB != 0
    }
}

// ── Static page directory ──────────────────────────────────────────
/// Kernel page directory (1024 entries, 4 KB-aligned). Only ever touched
/// from init code while interrupts are disabled.
#[repr(C, align(4096))]
static mut KERNEL_PAGE_DIRECTORY: [PageEntry; PD_ENTRIES] =
    [PageEntry { entry: 0 }; PD_ENTRIES];

// ── Helper – get a free 4 KB page for a new page table ───────────
// Temporary hack: grab memory from high memory (above the kernel heap)
// so we don't break early boot.  A real OS would ask the buddy allocator.
const TEMP_PT_ALLOC_BASE: u32 = 0x200000; // 2 MB
static mut NEXT_FREE_PT: u32 = TEMP_PT_ALLOC_BASE;

unsafe fn alloc_page_table() -> u32 {
    let addr = NEXT_FREE_PT;
    NEXT_FREE_PT += PAGE_SIZE_4KB;
    // Zero the page table memory (presents as "not present" entries)
    ptr::write_bytes(addr as *mut u8, 0, PAGE_SIZE_4KB as usize);
    addr
}

// ── Public API ─────────────────────────────────────────────────────

/// Initialise 32-bit paging with a 4 MB identity map for the kernel
/// plus a 4 MB identity map for the framebuffer.
pub unsafe fn init_paging() {
    // Enable PSE (Page Size Extensions) in CR4 so we can use 4 MB pages.
    let mut cr4: u32;
    asm!("mov {}, cr4", out(reg) cr4);
    cr4 |= 1 << 4; // PSE bit
    asm!("mov cr4, {}", in(reg) cr4);

    let flags = PAGE_PRESENT | PAGE_WRITABLE;

    // ── Kernel identity map (0 – 4 MB) ──────────────────────────
    let kernel_vaddr = 0x00000000;
    let pde_idx = (kernel_vaddr >> 22) as usize; // bits 31:22
    KERNEL_PAGE_DIRECTORY[pde_idx] =
        PageEntry::new(0x00000000, flags | PAGE_SIZE_4MB);

    // ── Framebuffer identity map (0xFD00_0000 – 4 MB) ───────────
    let fb_vaddr = 0xFD000000;
    let pde_idx = (fb_vaddr >> 22) as usize;
    KERNEL_PAGE_DIRECTORY[pde_idx] =
        PageEntry::new(0xFD000000, flags | PAGE_SIZE_4MB);

    // ── Load page directory ─────────────────────────────────────
    let pd_phys = &raw const KERNEL_PAGE_DIRECTORY as *const _ as u32;
    asm!("mov cr3, {}", in(reg) pd_phys);

    // ── Enable paging: PG (bit 31) and PE (bit 0) ───────────────
    let mut cr0: u32;
    asm!("mov {}, cr0", out(reg) cr0);
    cr0 |= 1 << 31; // PG
    cr0 |= 1 << 0;  // PE
    asm!("mov cr0, {}", in(reg) cr0);
}

/// Physical address of the kernel page directory.
pub fn get_kernel_page_directory_phys_addr() -> u32 {
    (&raw const KERNEL_PAGE_DIRECTORY as *const _ as u32)
}

/// Create a user page directory (currently just returns the kernel PD).
pub fn create_user_page_directory() -> u32 {
    get_kernel_page_directory_phys_addr()
}

/// Map the 4 KB virtual page `virt` to physical page `phys` with the
/// given `flags` (at least `PAGE_PRESENT`).  If the required page table
/// does not exist yet it is allocated on the fly.
///
/// # Safety
///
/// Caller must ensure the paging structures are writable and that
/// `virt` is not currently mapped in an incompatible way.
pub unsafe fn map_page(virt: u32, phys: u32, flags: u32) {
    let pd_idx = (virt >> 22) as usize;          // bits 31:22
    let pt_idx = ((virt >> 12) & 0x3FF) as usize; // bits 21:12

    let pd = &raw mut KERNEL_PAGE_DIRECTORY;
    let pde = &mut (*pd)[pd_idx];

    if !pde.is_present() || pde.is_4mb_page() {
        // Allocate a new 4 KB page table, zero it, and wire it into the PDE.
        let pt_phys = alloc_page_table();
        *pde = PageEntry::new(pt_phys, PAGE_PRESENT | PAGE_WRITABLE);
    }

    // Pointer to the page table (the PDE gives us its physical address,
    // but we are identity-mapped so phys == virt).
    let pt_base = pde.get_physical_addr();
    let pt = &mut *(pt_base as *mut [PageEntry; PT_ENTRIES]);
    pt[pt_idx] = PageEntry::new(phys, flags);

    // Flush the TLB for this virtual page.
    asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags));
}

/// Unmap the 4 KB virtual page `virt`, making it inaccessible.
///
/// # Safety
///
/// The caller must ensure that no mappings are left dangling.
pub unsafe fn unmap_page(virt: u32) {
    let pd_idx = (virt >> 22) as usize;
    let pt_idx = ((virt >> 12) & 0x3FF) as usize;

    let pd = &raw mut KERNEL_PAGE_DIRECTORY;
    let pde = &(*pd)[pd_idx];

    if !pde.is_present() || pde.is_4mb_page() {
        // Nothing to do: there is no page table or this is a large page.
        return;
    }

    let pt_base = pde.get_physical_addr();
    let pt = &mut *(pt_base as *mut [PageEntry; PT_ENTRIES]);
    pt[pt_idx] = PageEntry { entry: 0 }; // clear the entry

    // Flush the TLB for this virtual page.
    asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags));
}

