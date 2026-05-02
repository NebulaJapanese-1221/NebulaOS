use core::arch::asm;
use core::ptr::addr_of_mut;

// --- Paging Constants ---
pub const FLAG_PRESENT: u64 = 1 << 0;
pub const FLAG_WRITABLE: u64 = 1 << 1;
pub const FLAG_HUGE: u64 = 1 << 7; // 2MB Page in PAE

#[repr(align(4096))]
pub struct Pdpt(pub [u64; 4]);

#[repr(align(4096))]
pub struct PageDirectory(pub [u64; 512]);

// Kernel-global paging structures
static mut KERNEL_PDPT: Pdpt = Pdpt([0; 4]);
static mut PAGE_DIRECTORY: [PageDirectory; 4] = [const { PageDirectory([0; 512]) }; 4];

pub struct VirtualAddressSpace {
    pub pdpt: *mut Pdpt,
}

impl VirtualAddressSpace {
    pub fn kernel() -> Self {
        Self { pdpt: addr_of_mut!(KERNEL_PDPT) }
    }

    pub unsafe fn switch(&self) {
        asm!("mov cr3, {}", in(reg) self.pdpt as u32);
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
}

/// Marks a range of memory as executable by clearing the NX bit (bit 63).
pub unsafe fn make_executable(addr: usize, size: usize) {
    // PAE in this kernel currently uses 2MB pages. 
    // We align to 2MB boundaries to find the corresponding PDEs.
    let start_page = addr & !((2 * 1024 * 1024) - 1);
    let end_page = (addr + size + (2 * 1024 * 1024) - 1) & !((2 * 1024 * 1024) - 1);

    for page in (start_page..end_page).step_by(2 * 1024 * 1024) {
        if let Some(pte) = get_pte_mut(page) {
            *pte &= !(1 << 63);
            asm!("invlpg [{}]", in(reg) page, options(nostack, preserves_flags));
        }
    }
}

fn get_pte_mut(virt_addr: usize) -> Option<&'static mut u64> {
    let pdpt_idx = (virt_addr >> 30) & 0x3;
    let pd_idx = (virt_addr >> 21) & 0x1FF;
    unsafe { Some(&mut PAGE_DIRECTORY[pdpt_idx].0[pd_idx]) }
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