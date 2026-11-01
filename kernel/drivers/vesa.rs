// VESA VBE driver for graphics mode

#[repr(C, packed)]
struct VbeInfoBlock {
    signature: [u8; 4],
    version: u16,
    oem_string_ptr: *const u8,
    capabilities: [u8; 4],
    video_mode_ptr: *const u16,
    total_memory: u16,
    software_rev: u16,
    vendor_name_ptr: *const u8,
    product_name_ptr: *const u8,
    product_rev_ptr: *const u8,
    reserved: [u8; 222],
}

#[repr(C, packed)]
#[derive(Clone)]
struct ModeInfoBlock {
    attributes: u16,
    win_a: u8,
    win_b: u8,
    win_granularity: u16,
    win_size: u16,
    win_a_segment: u16,
    win_b_segment: u16,
    win_func_ptr: *const u8,
    pitch: u16,
    width: u16,
    height: u16,
    char_width: u8,
    char_height: u8,
    planes: u8,
    bits_per_pixel: u8,
    banks: u8,
    memory_model: u8,
    bank_size: u8,
    image_pages: u8,
    reserved0: u8,
    red_mask: u8,
    red_position: u8,
    green_mask: u8,
    green_position: u8,
    blue_mask: u8,
    blue_position: u8,
    rsvd_mask: u8,
    rsvd_position: u8,
    direct_color_attributes: u8,
    physbase: u32,
    reserved1: [u8; 212],
}

#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub width: u16,
    pub height: u16,
    pub pitch: u16,
    pub bpp: u8,
    pub physbase: u32,
    pub initialized: bool,
}

#[derive(Clone)]
pub struct FramebufferInfo {
    pub width: u16,
    pub height: u16,
    pub pitch: u16,
    pub bpp: u8,
    pub physbase: u32,
}

static mut FB: Option<Framebuffer> = None;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

pub fn init() -> bool {
    false
}

pub fn get_fb() -> Option<Framebuffer> {
    unsafe { FB.clone() }
}

pub fn put_pixel(x: u16, y: u16, color: u32) {
    let fb = unsafe { FB.as_ref() };
    if let Some(fb) = fb {
        if x < fb.width && y < fb.height {
            let offset = (y as usize * fb.pitch as usize / 4) + (x as usize);
            let addr = (fb.physbase as usize + offset * 4) as *mut u32;
            unsafe {
                addr.write_volatile(color);
            }
        }
    }
}

pub fn fill_rect(x: u16, y: u16, w: u16, h: u16, color: u32) {
    for y in y..(y + h) {
        for x in x..(x + w) {
            put_pixel(x, y, color);
        }
    }
}

pub fn clear_screen(color: u32) {
    let fb = unsafe { FB.as_ref() };
    if let Some(fb) = fb {
        fill_rect(0, 0, fb.width, fb.height, color);
    }
}

pub fn draw_rect(x: u16, y: u16, w: u16, h: u16, color: u32) {
    fill_rect(x, y, w, 2, color);
    fill_rect(x, y + h - 2, w, 2, color);
    fill_rect(x, y, 2, h, color);
    fill_rect(x + w - 2, y, 2, h, color);
}

pub fn get_width() -> u16 {
    unsafe { FB.as_ref().map(|fb| fb.width).unwrap_or(0) }
}

pub fn get_height() -> u16 {
    unsafe { FB.as_ref().map(|fb| fb.height).unwrap_or(0) }
}
