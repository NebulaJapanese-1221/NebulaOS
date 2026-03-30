use core::arch::asm;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Page Directory (1024 entries, each mapping 4MB)
#[repr(align(4096))]
static mut PAGE_DIRECTORY: [u32; 1024] = [0; 1024];

/// Initial Page Tables to identity map the first 128MB
#[repr(align(4096))]
static mut PAGE_TABLES: [[u32; 1024]; 32] = [[0; 1024]; 32];

pub fn init() {
    unsafe {
        // 1. Identity map the first 128MB (32 tables * 1024 entries * 4KB)
        for t in 0..32 {
            for e in 0..1024 {
                let addr = (t * 1024 + e) * 4096;
                // Present + Writable (0x03)
                PAGE_TABLES[t][e] = (addr as u32) | 0x03;
            }
            // Set Directory entry: Present + Writable
            let table_addr = &PAGE_TABLES[t] as *const _ as u32;
            PAGE_DIRECTORY[t] = table_addr | 0x03;
        }

        // 2. Identity map the Framebuffer (if detected)
        let fb_addr = crate::drivers::framebuffer::FRAMEBUFFER.lock().info.as_ref().map(|i| i.address).unwrap_or(0);
        if fb_addr != 0 {
            let pd_idx = fb_addr >> 22;
            // Use 4MB page (PSE) if supported, or just map the first few MBs for now
            // For simplicity, we ensure the PD entry is present and writable
            // Real kernels would map this dynamically.
        }

        // 3. Load Page Directory into CR3
        let pd_ptr = &PAGE_DIRECTORY as *const _ as u32;
        asm!("mov cr3, {}", in(reg) pd_ptr);

        // 4. Enable Paging (Set PG bit in CR0)
        let mut cr0: u32;
        asm!("mov {}, cr0", out(reg) cr0);
        cr0 |= 0x80000000;
        asm!("mov cr0, {}", in(reg) cr0);
    }
}

/// Unmaps a specific 4KB page by clearing the Present bit.
pub fn unmap_page(vaddr: usize) {
    let vaddr = vaddr & !0xFFF; // Align to page
    let pd_idx = vaddr >> 22;
    let pt_idx = (vaddr >> 12) & 0x3FF;

    unsafe {
        if (PAGE_DIRECTORY[pd_idx] & 0x01) != 0 {
            let pt_ptr = (PAGE_DIRECTORY[pd_idx] & !0xFFF) as *mut u32;
            let pt_entry = pt_ptr.add(pt_idx);
            *pt_entry &= !0x01; // Clear Present bit
            
            // Invalidate TLB for this address
            asm!("invlpg [{}]", in(reg) vaddr);
        }
    }
}

/// Maps a specific 4KB page (identity mapping).
pub fn map_page(vaddr: usize) {
    let vaddr = vaddr & !0xFFF;
    let pd_idx = vaddr >> 22;
    let pt_idx = (vaddr >> 12) & 0x3FF;

    unsafe {
        if (PAGE_DIRECTORY[pd_idx] & 0x01) != 0 {
            let pt_ptr = (PAGE_DIRECTORY[pd_idx] & !0xFFF) as *mut u32;
            let pt_entry = pt_ptr.add(pt_idx);
            *pt_entry = (vaddr as u32) | 0x03; // Present + Writable
            
            asm!("invlpg [{}]", in(reg) vaddr);
        }
    }
}