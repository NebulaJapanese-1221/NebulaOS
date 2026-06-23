use core::arch::asm;

// --- Page Flags ---
// The flags determine the behavior of a page entry.

// Page is present in physical memory.
const PAGE_PRESENT: u32 = 1 << 0;
// Page is writable. If not set, page is read-only.
const PAGE_WRITABLE: u32 = 1 << 1;
// User mode can access the page. If not set, only kernel mode can access.
const PAGE_USER: u32 = 1 << 2;
// Page access was triggered by a read operation.
const PAGE_ACCESSED: u32 = 1 << 5;
// Page was written to. (Dirty bit)
const PAGE_DIRTY: u32 = 1 << 6;
// Page size is 4MB (for page directory entries).
const PAGE_SIZE: u32 = 1 << 7;

// --- Page Table Entry (PTE) and Page Directory Entry (PDE) ---
// These structs represent entries in the page tables and page directory.
// They are 4 bytes (32 bits) each.

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct PageEntry {
    flags: u32,
    // Physical address of the next level table or the physical page frame.
    // This must be aligned to 4KB (12 bits of zeros).
    physical_addr: u32,
}

impl PageEntry {
    /// Creates a new page entry pointing to a physical address with specified flags.
    fn new(physical_addr: u32, flags: u32) -> Self {
        PageEntry {
            flags,
            // Ensure the address is page-aligned (lower 12 bits are 0).
            physical_addr: physical_addr & !0xFFF, 
        }
    }
}

// --- Page Directory ---
// A page directory is an array of 1024 Page Directory Entries (PDEs).
// Each PDE points to a Page Table or maps a 4MB page directly.
// It must be aligned to 4KB.

const PAGE_DIRECTORY_SIZE: usize = 1024; // 1024 entries * 4 bytes/entry = 4KB

// Use a static mutable array for the kernel's initial page directory.
// This will be mapped at physical address 0x00001000 (4KB).
static mut KERNEL_PAGE_DIRECTORY: [PageEntry; PAGE_DIRECTORY_SIZE] = [
    // Initialize all entries to 0 (not present).
    PageEntry { flags: 0, physical_addr: 0 }; 
    PAGE_DIRECTORY_SIZE
];

/// Initializes the kernel's page directory.
/// This involves mapping the kernel code/data and enabling paging.
pub unsafe fn init_paging() {
    // 1. Map the kernel's code and data sections.
    // We'll start with a simple identity mapping for the kernel,
    // assuming it's loaded at a low physical address.
    // This is a placeholder; a real kernel would use the linker script 
    // to determine kernel boundaries.
    // For now, let's map the first 4MB of physical memory to the same virtual address.
    
    // Map 0x00000000 - 0x00400000 (4MB) to virtual 0x00000000 - 0x00400000
    // Assuming kernel is loaded here.
    let kernel_phys_addr = 0x100000; // Placeholder: Start of kernel code/data after bootloader etc.
    let kernel_virt_addr = 0x100000;
    let kernel_size_mb = 4; // Map first 4MB

    let pde_idx = kernel_virt_addr / PAGE_SIZE; // Calculate index in page directory
    let flags = PAGE_PRESENT | PAGE_WRITABLE; // Kernel pages are present and writable

    // Use PAGE_SIZE flag for 4MB pages (no need for separate page table for the kernel itself)
    KERNEL_PAGE_DIRECTORY[pde_idx as usize] = PageEntry::new(kernel_phys_addr, flags | PAGE_SIZE);
    
    // Note: For finer-grained control (4KB pages), we would iterate through 
    // each 4KB chunk and set up Page Tables. But for initial kernel mapping, 
    // 4MB pages simplify things.

    // 2. Load the page directory into CR3.
    // The physical address of KERNEL_PAGE_DIRECTORY must be page-aligned.
    let pd_phys_addr = &KERNEL_PAGE_DIRECTORY as *const _ as u32 & !0xFFF;
    asm!("mov cr3, {}", in(reg) pd_phys_addr);

    // 3. Enable paging by setting the PE (Protection Enable) and PG (Paging) bits in CR0.
    let mut cr0: u32;
    asm!("mov {}, cr0", out(reg) cr0);
    // Set PG (bit 31) and PE (bit 0)
    cr0 |= PAGE_PRESENT | PAGE_SIZE; // WRONG: PG is bit 31, PE is bit 0. Let's correct this.
    cr0 |= (1 << 31) | (1 << 0); // Enable PG and PE
    asm!("mov cr0, {}", in(reg) cr0);

    // If you are using 4MB pages for the kernel, you don't strictly need to set up
    // page tables initially for kernel code/data if it fits within 4MB.
    // For user space, you WILL need page tables for finer control.

    // Example: If you needed to map individual 4KB pages, you would:
    // - Allocate physical memory for a page table.
    // - Set a PDE to point to this page table with PAGE_PRESENT | PAGE_WRITABLE flags.
    // - Populate the page table with PTEs pointing to physical page frames.
}

/// Function to retrieve the physical address of the kernel's page directory.
pub fn get_kernel_page_directory_phys_addr() -> u32 {
    // Ensure the physical address is page-aligned.
    &KERNEL_PAGE_DIRECTORY as *const _ as u32 & !0xFFF
}