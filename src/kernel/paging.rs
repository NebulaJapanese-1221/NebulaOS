use core::arch::asm;
use core::ptr::addr_of_mut;

// --- Paging Constants ---
pub const FLAG_PRESENT: u64 = 1 << 0;
pub const FLAG_WRITABLE: u64 = 1 << 1;
pub const FLAG_USER: u64 = 1 << 2;
pub const FLAG_HUGE: u64 = 1 << 7; // 2MB Page in PAE
pub const FLAG_COW: u64 = 1 << 9;  // Custom OS flag

#[repr(align(4096))]
pub struct Pdpt(pub [u64; 4]);

#[repr(align(4096))]
pub struct PageDirectory(pub [u64; 512]);

// Kernel-global paging structures
static mut KERNEL_PDPT: Pdpt = Pdpt([0; 4]);
static mut PAGE_DIRECTORY: [PageDirectory; 4] = [const { PageDirectory([0; 512]) }; 4];

pub struct VirtualAddressSpace {
    pub pdpt: *mut Pdpt,
    pub owned: bool,
}

impl VirtualAddressSpace {
    pub fn kernel() -> Self {
        Self { pdpt: addr_of_mut!(KERNEL_PDPT), owned: false }
    }

    pub fn new_user() -> Option<Self> {
        let pdpt_ptr = allocate_frame()? as *mut Pdpt;
        unsafe {
            // Copy kernel identity mappings (first 4GB directories)
            for i in 0..4 {
                let pd_ptr = allocate_frame()? as *mut PageDirectory;
                core::ptr::copy_nonoverlapping(addr_of_mut!(PAGE_DIRECTORY[i]), pd_ptr, 1);
                (*pdpt_ptr).0[i] = (pd_ptr as u64) | FLAG_PRESENT | FLAG_USER;
            }
        }
        Some(Self { pdpt: pdpt_ptr, owned: true })
    }

    pub unsafe fn switch(&self) {
        asm!("mov cr3, {}", in(reg) self.pdpt as u32);
    }

    /// Maps a 4KB page in the current address space.
    pub fn map_page(&self, vaddr: usize, paddr: usize, flags: u64) {
        let pdpt_idx = (vaddr >> 30) & 0x3;
        let pd_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;

        unsafe {
            let pd_ptr = ((*self.pdpt).0[pdpt_idx] & !0xFFF) as *mut PageDirectory;
            let pt_ptr = self.get_or_create_table(pd_ptr, pd_idx, flags);
            if let Some(pt) = pt_ptr {
                (*pt)[pt_idx] = (paddr as u64) | flags | FLAG_PRESENT;
                asm!("invlpg [{}]", in(reg) vaddr);
            }
        }
    }

    /// Identity maps a large region of memory using 2MB pages.
    pub fn map_region(&self, vaddr: usize, paddr: usize, size: usize, flags: u64) {
        for offset in (0..size).step_by(2 * 1024 * 1024) {
            let curr_v = vaddr + offset;
            let curr_p = paddr + offset;
            let pdpt_idx = (curr_v >> 30) & 0x3;
            let pd_idx = (curr_v >> 21) & 0x1FF;
            unsafe {
                let pd_ptr = ((*self.pdpt).0[pdpt_idx] & !0xFFF) as *mut PageDirectory;
                (*pd_ptr).0[pd_idx] = (curr_p as u64) | flags | FLAG_PRESENT | FLAG_HUGE;
            }
        }
    }

    unsafe fn get_or_create_table(&self, pd: *mut PageDirectory, idx: usize, _flags: u64) -> Option<*mut [u64; 512]> {
        let entry = (*pd).0[idx];
        if (entry & FLAG_PRESENT) != 0 {
            return Some((entry & !0xFFF) as *mut [u64; 512]);
        }
        let ptr = allocate_frame()? as *mut [u64; 512];
        let user_bit = if self.owned { FLAG_USER } else { 0 };
        (*pd).0[idx] = (ptr as u64) | FLAG_PRESENT | FLAG_WRITABLE | user_bit;
        Some(ptr)
    }

    pub fn get_page_entry(&self, vaddr: usize) -> Option<u64> {
        let pdpt_idx = (vaddr >> 30) & 0x3;
        let pd_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;
        unsafe {
            let pd_ptr = ((*self.pdpt).0[pdpt_idx] & !0xFFF) as *mut PageDirectory;
            let entry = (*pd_ptr).0[pd_idx];
            if (entry & FLAG_PRESENT) == 0 { return None; }
            if (entry & FLAG_HUGE) != 0 { return Some(entry); }
            let pt_ptr = (entry & !0xFFF) as *mut [u64; 512];
            Some((*pt_ptr)[pt_idx])
        }
    }

    pub fn unmap_page(&self, vaddr: usize) {
        let pdpt_idx = (vaddr >> 30) & 0x3;
        let pd_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;
        unsafe {
            let pd_ptr = ((*self.pdpt).0[pdpt_idx] & !0xFFF) as *mut PageDirectory;
            let entry = (*pd_ptr).0[pd_idx];
            if (entry & FLAG_PRESENT) != 0 && (entry & FLAG_HUGE) == 0 {
                let pt_ptr = (entry & !0xFFF) as *mut [u64; 512];
                (*pt_ptr)[pt_idx] = 0;
                asm!("invlpg [{}]", in(reg) vaddr);
            }
        }
    }
}

/// Initializes PAE paging and sets up the kernel identity map.
pub fn init(fb_info: Option<(usize, usize, usize, usize, u8)>) {
    let kernel_vas = VirtualAddressSpace::kernel();
    unsafe {
        // 1. Link PDPT to static Page Directories
        for i in 0..4 {
            let pd_phys = addr_of_mut!(PAGE_DIRECTORY[i]) as u64;
            (*kernel_vas.pdpt).0[i] = pd_phys | FLAG_PRESENT;
        }

        // 2. Identity map first 1GB with 2MB pages
        kernel_vas.map_region(0, 0, 1024 * 1024 * 1024, FLAG_PRESENT | FLAG_WRITABLE | FLAG_HUGE);

        // 3. Identity map Framebuffer
        if let Some((addr, _w, h, pitch, _bpp)) = fb_info {
            kernel_vas.map_region(addr, addr, pitch * h, FLAG_PRESENT | FLAG_WRITABLE | FLAG_HUGE);
        }

        // 4. Enable PAE and switch CR3
        let mut cr4: u32;
        asm!("mov {}, cr4", out(reg) cr4);
        cr4 |= 0x20; // PAE bit
        asm!("mov cr4, {}", in(reg) cr4);
        kernel_vas.switch();

        // 5. Enable Paging bit in CR0
        let mut cr0: u32;
        asm!("mov {}, cr0", out(reg) cr0);
        cr0 |= 0x80000000;
        asm!("mov cr0, {}", in(reg) cr0);
    }
}

pub unsafe fn get_kernel_pd_ptr() -> u32 {
    addr_of_mut!(KERNEL_PDPT) as u32
}

/// Allocates a 4KB physical frame and zeros it.
pub fn allocate_frame() -> Option<*mut [u64; 512]> {
    unsafe {
        let layout = core::alloc::Layout::from_size_align(4096, 4096).ok()?;
        let ptr = alloc::alloc::alloc_zeroed(layout) as *mut [u64; 512];
        if ptr.is_null() { None } else { Some(ptr) }
    }
}