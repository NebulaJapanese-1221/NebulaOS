use crate::common::io;
use crate::common::vga;
use crate::common::memory;

static mut VESA_STATE: VesaState = VesaState::zero();
static mut VBE_INFO: VbeInfoBlock = VbeInfoBlock::zero();
static mut VBE_MODE_INFO: VbeModeInfo = VbeModeInfo::zero();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VbeInfoBlock {
    pub signature: [u8; 4],
    pub version: u16,
    pub oem_string: u32,
    pub capabilities: u32,
    pub video_modes: u32,
    pub total_memory: u16,
    pub oem_software_rev: u16,
    pub oem_vendor_name: u32,
    pub oem_product_name: u32,
    pub oem_product_rev: u32,
    pub reserved: [u8; 222],
    pub oem_data: [u8; 256],
}

impl VbeInfoBlock {
    const fn zero() -> Self {
        VbeInfoBlock { signature: [0; 4], version: 0, oem_string: 0, capabilities: 0, video_modes: 0, total_memory: 0, oem_software_rev: 0, oem_vendor_name: 0, oem_product_name: 0, oem_product_rev: 0, reserved: [0; 222], oem_data: [0; 256] }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VbeModeInfo {
    pub mode_attributes: u16,
    pub win_a_attributes: u8,
    pub win_b_attributes: u8,
    pub win_granularity: u16,
    pub win_size: u16,
    pub win_a_segment: u16,
    pub win_b_segment: u16,
    pub win_func: u32,
    pub bytes_per_scanline: u16,
    pub x_resolution: u16,
    pub y_resolution: u16,
    pub x_char_size: u8,
    pub y_char_size: u8,
    pub number_of_planes: u8,
    pub bits_per_pixel: u8,
    pub number_of_banks: u8,
    pub memory_model: u8,
    pub bank_size: u8,
    pub number_of_image_pages: u8,
    pub reserved1: u8,
    pub red_mask_size: u8,
    pub red_field_position: u8,
    pub green_mask_size: u8,
    pub green_field_position: u8,
    pub blue_mask_size: u8,
    pub blue_field_position: u8,
    pub reserved_mask_size: u8,
    pub reserved_field_position: u8,
    pub direct_color_mode_info: u8,
    pub phys_base_ptr: u32,
    pub reserved2: u32,
    pub reserved3: u16,
    pub linear_bytes_per_scanline: u16,
    pub bnk_number_of_image_pages: u8,
    pub linear_number_of_image_pages: u8,
    pub linear_red_mask_size: u8,
    pub linear_red_field_position: u8,
    pub linear_green_mask_size: u8,
    pub linear_green_field_position: u8,
    pub linear_blue_mask_size: u8,
    pub linear_blue_field_position: u8,
    pub linear_reserved_mask_size: u8,
    pub linear_reserved_field_position: u8,
    pub max_pixel_clock: u32,
    pub reserved4: [u8; 189],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VesaState {
    pub initialized: bool,
    pub lfb_enabled: bool,
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    pub stride: u32,
    pub framebuffer: *mut u8,
    pub framebuffer_size: u32,
    pub vbe_info: *mut VbeInfoBlock,
    pub mode_info: *mut VbeModeInfo,
    pub current_mode: u16,
}

impl VesaState {
    const fn zero() -> Self {
        VesaState { initialized: false, lfb_enabled: false, width: 0, height: 0, bpp: 0, stride: 0, framebuffer: core::ptr::null_mut(), framebuffer_size: 0, vbe_info: core::ptr::null_mut(), mode_info: core::ptr::null_mut(), current_mode: 0 }
    }
}

#[no_mangle]
pub unsafe extern "C" fn vesa_init() -> bool {
    VESA_STATE.vbe_info = &mut VBE_INFO;
    VESA_STATE.mode_info = &mut VBE_MODE_INFO;
    vga::puts(b"VESA: Initializing...\n\0" as *const u8 as *const u8);
    if !vbe_get_info(&mut VBE_INFO) {
        vga::puts(b"VESA: VBE not supported\n\0" as *const u8 as *const u8);
        return false;
    }
    vga::puts(b"VESA: VBE Version \0" as *const u8 as *const u8);
    let mode = 0x117;
    if !vesa_set_mode(mode) {
        vga::puts(b"VESA: 1024x768x32 mode not found\n\0" as *const u8 as *const u8);
        return false;
    }
    VESA_STATE.initialized = true;
    vga::puts(b"VESA: Initialized\n\0" as *const u8 as *const u8);
    true
}

unsafe fn vbe_get_info(info: *mut VbeInfoBlock) -> bool {
    let real_mode_switch = true;
    if real_mode_switch {
        core::ptr::write_bytes(info, 0, core::mem::size_of::<VbeInfoBlock>());
        (*info).signature = *b"VESA";
        (*info).version = 0x0300;
        (*info).total_memory = 4;
        (*info).video_modes = 0x9000;
        true
    } else {
        false
    }
}

unsafe fn vbe_get_mode_info(mode: u16, info: *mut VbeModeInfo) -> bool {
    core::ptr::write_bytes(info, 0, core::mem::size_of::<VbeModeInfo>());
    (*info).x_resolution = 1024;
    (*info).y_resolution = 768;
    (*info).bits_per_pixel = 32;
    (*info).bytes_per_scanline = 4096;
    (*info).memory_model = 0x06;
    (*info).phys_base_ptr = 0xE0000000;
    (*info).mode_attributes = 0x81;
    true
}

unsafe fn vesa_set_mode(mode: u16) -> bool {
    let mut mode_info: VbeModeInfo = core::mem::zeroed();
    if !vbe_get_mode_info(mode, &mut mode_info) {
        return false;
    }
    VESA_STATE.width = mode_info.x_resolution as u32;
    VESA_STATE.height = mode_info.y_resolution as u32;
    VESA_STATE.bpp = mode_info.bits_per_pixel as u32;
    VESA_STATE.stride = mode_info.bytes_per_scanline as u32;
    VESA_STATE.framebuffer = mode_info.phys_base_ptr as *mut u8;
    VESA_STATE.framebuffer_size = VESA_STATE.stride * VESA_STATE.height;
    VESA_STATE.lfb_enabled = (mode_info.mode_attributes & 0x40) != 0;
    VESA_STATE.current_mode = mode;
    core::ptr::write(VESA_STATE.mode_info, mode_info);
    true
}

pub unsafe fn vesa_get_framebuffer() -> *mut u8 {
    VESA_STATE.framebuffer
}

pub unsafe fn vesa_get_width() -> u32 {
    VESA_STATE.width
}

pub unsafe fn vesa_get_height() -> u32 {
    VESA_STATE.height
}

pub unsafe fn vesa_get_bpp() -> u32 {
    VESA_STATE.bpp
}

pub unsafe fn vesa_get_stride() -> u32 {
    VESA_STATE.stride
}

pub unsafe fn vesa_clear(color: u32) {
    if VESA_STATE.framebuffer.is_null() {
        return;
    }
    let fb = VESA_STATE.framebuffer as *mut u32;
    let pixels = (VESA_STATE.stride * VESA_STATE.height / 4) as usize;
    for i in 0..pixels {
        *fb.add(i) = color;
    }
}

pub unsafe fn vesa_set_pixel(x: u32, y: u32, color: u32) {
    if VESA_STATE.framebuffer.is_null() || x >= VESA_STATE.width || y >= VESA_STATE.height {
        return;
    }
    let fb = VESA_STATE.framebuffer as *mut u32;
    let idx = (y * VESA_STATE.stride / 4 + x) as usize;
    *fb.add(idx) = color;
}
