use crate::drivers::font;
use crate::drivers::paging;
use core::ptr;

const MULTIBOOT_INFO_MAX_SIZE: usize = 1024 * 1024;
const MAX_FRAMEBUFFER_SIZE: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
    pixels: *mut u8,
    display_pixels: *mut u8,
    length: usize,
    red_position: u8,
    red_size: u8,
    green_position: u8,
    green_size: u8,
    blue_position: u8,
    blue_size: u8,
}

static mut FB: Option<Framebuffer> = None;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferError {
    InvalidBootInformation,
    NoRgbFramebuffer,
    UnsupportedPixelFormat,
    FramebufferTooLarge,
    AddressNotMapped,
    BackBufferAllocationFailed,
}

pub fn init(multiboot_info: *const u8) -> Result<Framebuffer, FramebufferError> {
    if multiboot_info.is_null() {
        return Err(FramebufferError::InvalidBootInformation);
    }

    let total_size = unsafe { read_u32(multiboot_info, 0) as usize };
    if !(16..=MULTIBOOT_INFO_MAX_SIZE).contains(&total_size) {
        return Err(FramebufferError::InvalidBootInformation);
    }

    let mut tag_offset = 8usize;
    while tag_offset
        .checked_add(8)
        .is_some_and(|end| end <= total_size)
    {
        let tag_type = unsafe { read_u32(multiboot_info, tag_offset) };
        let tag_size = unsafe { read_u32(multiboot_info, tag_offset + 4) as usize };
        if tag_size < 8
            || tag_offset
                .checked_add(tag_size)
                .is_none_or(|end| end > total_size)
        {
            return Err(FramebufferError::InvalidBootInformation);
        }

        if tag_type == 8 {
            if tag_size < 38 {
                return Err(FramebufferError::InvalidBootInformation);
            }
            let address = unsafe { read_u64(multiboot_info, tag_offset + 8) };
            let pitch = unsafe { read_u32(multiboot_info, tag_offset + 16) };
            let width = unsafe { read_u32(multiboot_info, tag_offset + 20) };
            let height = unsafe { read_u32(multiboot_info, tag_offset + 24) };
            let bpp = unsafe { ptr::read(multiboot_info.add(tag_offset + 28)) };
            let framebuffer_type = unsafe { ptr::read(multiboot_info.add(tag_offset + 29)) };

            if framebuffer_type != 1 {
                return Err(FramebufferError::NoRgbFramebuffer);
            }

            let red_position = unsafe { ptr::read(multiboot_info.add(tag_offset + 32)) };
            let red_size = unsafe { ptr::read(multiboot_info.add(tag_offset + 33)) };
            let green_position = unsafe { ptr::read(multiboot_info.add(tag_offset + 34)) };
            let green_size = unsafe { ptr::read(multiboot_info.add(tag_offset + 35)) };
            let blue_position = unsafe { ptr::read(multiboot_info.add(tag_offset + 36)) };
            let blue_size = unsafe { ptr::read(multiboot_info.add(tag_offset + 37)) };

            if address > u32::MAX as u64
                || width == 0
                || height == 0
                || !matches!(bpp, 24 | 32)
                || red_size == 0
                || green_size == 0
                || blue_size == 0
                || red_size > 8
                || green_size > 8
                || blue_size > 8
                || red_position.saturating_add(red_size) > bpp
                || green_position.saturating_add(green_size) > bpp
                || blue_position.saturating_add(blue_size) > bpp
                || pitch < width.saturating_mul((bpp as u32).div_ceil(8))
            {
                return Err(FramebufferError::UnsupportedPixelFormat);
            }

            let length = (pitch as usize)
                .checked_mul(height as usize)
                .ok_or(FramebufferError::FramebufferTooLarge)?;
            if length > MAX_FRAMEBUFFER_SIZE {
                return Err(FramebufferError::FramebufferTooLarge);
            }
            let address = address as u32;
            address
                .checked_add(length as u32)
                .ok_or(FramebufferError::FramebufferTooLarge)?;
            let pixels =
                paging::map_mmio(address, length).ok_or(FramebufferError::AddressNotMapped)?;
            let back_buffer = paging::map_kernel_buffer(length)
                .ok_or(FramebufferError::BackBufferAllocationFailed)?;

            let framebuffer = Framebuffer {
                width,
                height,
                pitch,
                bpp,
                pixels: back_buffer,
                display_pixels: pixels,
                length,
                red_position,
                red_size,
                green_position,
                green_size,
                blue_position,
                blue_size,
            };
            unsafe {
                ptr::addr_of_mut!(FB).write(Some(framebuffer));
            }
            return Ok(framebuffer);
        }

        if tag_type == 0 {
            break;
        }
        tag_offset = tag_offset
            .checked_add(
                tag_size
                    .checked_add(7)
                    .ok_or(FramebufferError::InvalidBootInformation)?
                    & !7,
            )
            .ok_or(FramebufferError::InvalidBootInformation)?;
    }

    Err(FramebufferError::NoRgbFramebuffer)
}

unsafe fn read_u32(base: *const u8, offset: usize) -> u32 {
    ptr::read_unaligned(base.add(offset).cast::<u32>())
}

unsafe fn read_u64(base: *const u8, offset: usize) -> u64 {
    ptr::read_unaligned(base.add(offset).cast::<u64>())
}

pub fn get_fb() -> Option<Framebuffer> {
    unsafe { ptr::read_volatile(ptr::addr_of!(FB)) }
}

pub fn put_pixel(x: u32, y: u32, color: u32) {
    let Some(framebuffer) = get_fb() else {
        return;
    };
    if x >= framebuffer.width || y >= framebuffer.height {
        return;
    }

    let pixel = pack_color(framebuffer, color);
    let bytes_per_pixel = (framebuffer.bpp as usize).div_ceil(8);
    let offset = y as usize * framebuffer.pitch as usize + x as usize * bytes_per_pixel;
    write_pixel(framebuffer, offset, bytes_per_pixel, pixel);
}

fn channel(value: u32, size: u8) -> u32 {
    let max_value = (1u32 << size) - 1;
    value * max_value / 255
}

fn pack_color(framebuffer: Framebuffer, color: u32) -> u32 {
    let rgb = color & 0x00FF_FFFF;
    channel((rgb >> 16) & 0xFF, framebuffer.red_size) << framebuffer.red_position
        | channel((rgb >> 8) & 0xFF, framebuffer.green_size) << framebuffer.green_position
        | channel(rgb & 0xFF, framebuffer.blue_size) << framebuffer.blue_position
}

fn write_pixel(framebuffer: Framebuffer, offset: usize, bytes_per_pixel: usize, pixel: u32) {
    unsafe {
        for byte in 0..bytes_per_pixel {
            framebuffer
                .pixels
                .add(offset + byte)
                .write_volatile((pixel >> (byte * 8)) as u8);
        }
    }
}

pub fn fill_rect(x: u32, y: u32, width: u32, height: u32, color: u32) {
    let Some(framebuffer) = get_fb() else {
        return;
    };
    let right = x.saturating_add(width).min(framebuffer.width);
    let bottom = y.saturating_add(height).min(framebuffer.height);
    let bytes_per_pixel = (framebuffer.bpp as usize).div_ceil(8);
    let pixel = pack_color(framebuffer, color);
    for row in y.min(framebuffer.height)..bottom {
        for column in x.min(framebuffer.width)..right {
            let offset =
                row as usize * framebuffer.pitch as usize + column as usize * bytes_per_pixel;
            write_pixel(framebuffer, offset, bytes_per_pixel, pixel);
        }
    }
}

pub fn clear_screen(color: u32) {
    let Some(framebuffer) = get_fb() else {
        return;
    };
    fill_rect(0, 0, framebuffer.width, framebuffer.height, color);
}

pub fn present() {
    let Some(framebuffer) = get_fb() else {
        return;
    };
    unsafe {
        for offset in 0..framebuffer.length {
            framebuffer
                .display_pixels
                .add(offset)
                .write_volatile(framebuffer.pixels.add(offset).read());
        }
    }
}

pub fn draw_rect(x: u32, y: u32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }
    fill_rect(x, y, width, 2.min(height), color);
    fill_rect(
        x,
        y.saturating_add(height - 2.min(height)),
        width,
        2.min(height),
        color,
    );
    fill_rect(x, y, 2.min(width), height, color);
    fill_rect(
        x.saturating_add(width - 2.min(width)),
        y,
        2.min(width),
        height,
        color,
    );
}

pub fn draw_text(mut x: u32, y: u32, text: &str, color: u32, scale: u32) {
    let scale = scale.max(1);
    let start_x = x;
    let Some(framebuffer) = get_fb() else {
        return;
    };

    for byte in text.bytes() {
        if byte == b'\n' {
            x = start_x;
            continue;
        }
        if byte == b' ' {
            x = x.saturating_add(6 * scale);
            continue;
        }
        if let Some(rows) = font::glyph(byte.to_ascii_uppercase()) {
            for (row, bits) in rows.iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        fill_rect(
                            x + column * scale,
                            y + row as u32 * scale,
                            scale,
                            scale,
                            color,
                        );
                    }
                }
            }
        }
        x = x.saturating_add(6 * scale);
        if x >= framebuffer.width {
            break;
        }
    }
}

pub fn get_width() -> u32 {
    get_fb().map(|fb| fb.width).unwrap_or(0)
}

pub fn get_height() -> u32 {
    get_fb().map(|fb| fb.height).unwrap_or(0)
}
