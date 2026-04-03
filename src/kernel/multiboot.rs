//! A small module for parsing the Multiboot v1 information structure.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MultibootMemoryMapEntry {
    pub size: u32,
    pub addr: u64,
    pub len: u64,
    pub type_: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootInfo {
    pub flags: u32,
    _mem_lower: u32,
    _mem_upper: u32,
    _boot_device: u32,
    _cmdline: u32,
    _mods_count: u32,
    _mods_addr: u32,
    _elf_sec: [u32; 4],
    pub mmap_length: u32,
    pub mmap_addr: u32,
    _drives_length: u32,
    _drives_addr: u32,
    _config_table: u32,
    _boot_loader_name: u32,
    _apm_table: u32,
    _vbe_control_info: u32,
    _vbe_mode_info: u32,
    _vbe_mode: u16,
    _vbe_interface_seg: u16,
    _vbe_interface_off: u16,
    _vbe_interface_len: u16,
    framebuffer_addr: u64,
    framebuffer_pitch: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    framebuffer_bpp: u8,
    framebuffer_type: u8,
}

/// Checks the multiboot info structure for valid framebuffer information.
/// Returns a tuple of (address, width, height, pitch, bpp) if available.
pub fn framebuffer_info(multiboot_info_ptr: usize) -> Option<(usize, usize, usize, usize, u8)> {
    if multiboot_info_ptr == 0 { return None; }
    let multiboot_info = unsafe { &*(multiboot_info_ptr as *const MultibootInfo) };

    // Check if framebuffer info is present (bit 12) and type is RGB (1)
    if (multiboot_info.flags & (1 << 12)) != 0 && multiboot_info.framebuffer_type == 1 {
        Some((multiboot_info.framebuffer_addr as usize, multiboot_info.framebuffer_width as usize, multiboot_info.framebuffer_height as usize, multiboot_info.framebuffer_pitch as usize, multiboot_info.framebuffer_bpp))
    } else {
        None
    }
}