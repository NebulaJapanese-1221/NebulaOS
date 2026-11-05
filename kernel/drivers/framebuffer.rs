// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

use crate::graphics::font;
use crate::memory::paging;
use crate::sync::SpinLock;
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CursorType {
    Arrow,
    Hand,
    Text,
    Move,
    Crosshair,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CursorState {
    x: u32,
    y: u32,
    kind: CursorType,
    visible: bool,
}

const CURSOR_SIZE: u32 = 20;

static mut CURSOR: CursorState = CursorState {
    x: 0,
    y: 0,
    kind: CursorType::Arrow,
    visible: false,
};
static mut DISPLAYED_CURSOR: CursorState = CursorState {
    x: 0,
    y: 0,
    kind: CursorType::Arrow,
    visible: false,
};
static FRAMEBUFFER_LOCK: SpinLock<()> = SpinLock::new(());

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
                ptr::addr_of_mut!(CURSOR).write(CursorState {
                    x: 0,
                    y: 0,
                    kind: CursorType::Arrow,
                    visible: false,
                });
                ptr::addr_of_mut!(DISPLAYED_CURSOR).write(CursorState {
                    x: 0,
                    y: 0,
                    kind: CursorType::Arrow,
                    visible: false,
                });
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
        if bytes_per_pixel == 4 {
            framebuffer
                .pixels
                .add(offset)
                .cast::<u32>()
                .write_unaligned(pixel);
            return;
        }
        for byte in 0..bytes_per_pixel {
            framebuffer
                .pixels
                .add(offset + byte)
                .write((pixel >> (byte * 8)) as u8);
        }
    }
}

pub fn fill_rect(x: u32, y: u32, width: u32, height: u32, color: u32) {
    let _guard = FRAMEBUFFER_LOCK.lock();
    fill_rect_unlocked(x, y, width, height, color);
}

fn fill_rect_unlocked(x: u32, y: u32, width: u32, height: u32, color: u32) {
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
    let _guard = FRAMEBUFFER_LOCK.lock();
    let Some(framebuffer) = get_fb() else {
        return;
    };
    fill_rect_unlocked(0, 0, framebuffer.width, framebuffer.height, color);
}

pub fn present() {
    let _guard = FRAMEBUFFER_LOCK.lock();
    let Some(framebuffer) = get_fb() else {
        return;
    };
    unsafe {
        let mut offset = 0;
        while offset < framebuffer.length
            && (framebuffer.display_pixels.add(offset) as usize) & 3 != 0
        {
            framebuffer
                .display_pixels
                .add(offset)
                .write_volatile(framebuffer.pixels.add(offset).read());
            offset += 1;
        }
        while offset + core::mem::size_of::<u32>() <= framebuffer.length {
            let pixel_group = framebuffer
                .pixels
                .add(offset)
                .cast::<u32>()
                .read_unaligned();
            framebuffer
                .display_pixels
                .add(offset)
                .cast::<u32>()
                .write_volatile(pixel_group);
            offset += core::mem::size_of::<u32>();
        }
        while offset < framebuffer.length {
            framebuffer
                .display_pixels
                .add(offset)
                .write_volatile(framebuffer.pixels.add(offset).read());
            offset += 1;
        }
    }
    let cursor = unsafe { ptr::read_volatile(ptr::addr_of!(CURSOR)) };
    draw_cursor_on_display(
        framebuffer,
        cursor,
        0,
        0,
        framebuffer.width,
        framebuffer.height,
    );
    unsafe {
        ptr::addr_of_mut!(DISPLAYED_CURSOR).write(cursor);
    }
}

pub fn set_cursor(x: u32, y: u32, kind: CursorType) {
    let _guard = FRAMEBUFFER_LOCK.lock();
    unsafe {
        ptr::addr_of_mut!(CURSOR).write(CursorState {
            x,
            y,
            kind,
            visible: true,
        });
    }
}

pub fn present_cursor_update() {
    let _guard = FRAMEBUFFER_LOCK.lock();
    let Some(framebuffer) = get_fb() else {
        return;
    };
    let previous = unsafe { ptr::read_volatile(ptr::addr_of!(DISPLAYED_CURSOR)) };
    let current = unsafe { ptr::read_volatile(ptr::addr_of!(CURSOR)) };
    if previous == current {
        return;
    }

    if previous.visible {
        let (x, y) = cursor_origin(previous);
        present_region(framebuffer, x, y, CURSOR_SIZE, CURSOR_SIZE, current);
    }
    if current.visible {
        let (x, y) = cursor_origin(current);
        present_region(framebuffer, x, y, CURSOR_SIZE, CURSOR_SIZE, current);
    }
    unsafe {
        ptr::addr_of_mut!(DISPLAYED_CURSOR).write(current);
    }
}

pub fn present_rect(x: u32, y: u32, width: u32, height: u32) {
    let _guard = FRAMEBUFFER_LOCK.lock();
    let Some(framebuffer) = get_fb() else {
        return;
    };
    let cursor = unsafe { ptr::read_volatile(ptr::addr_of!(CURSOR)) };
    present_region(framebuffer, x, y, width, height, cursor);
}

fn present_region(
    framebuffer: Framebuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    cursor: CursorState,
) {
    let right = x.saturating_add(width).min(framebuffer.width);
    let bottom = y.saturating_add(height).min(framebuffer.height);
    let left = x.min(framebuffer.width);
    let top = y.min(framebuffer.height);
    let bytes_per_pixel = (framebuffer.bpp as usize).div_ceil(8);

    unsafe {
        for row in top..bottom {
            for column in left..right {
                let offset =
                    row as usize * framebuffer.pitch as usize + column as usize * bytes_per_pixel;
                for byte in 0..bytes_per_pixel {
                    framebuffer
                        .display_pixels
                        .add(offset + byte)
                        .write_volatile(framebuffer.pixels.add(offset + byte).read());
                }
            }
        }
    }

    draw_cursor_on_display(framebuffer, cursor, left, top, right, bottom);
}

fn draw_cursor_on_display(
    framebuffer: Framebuffer,
    cursor: CursorState,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) {
    if !cursor.visible {
        return;
    }
    let (cursor_x, cursor_y) = cursor_origin(cursor);
    let cursor_right = cursor_x.saturating_add(CURSOR_SIZE).min(right);
    let cursor_bottom = cursor_y.saturating_add(CURSOR_SIZE).min(bottom);
    let start_x = cursor_x.max(left);
    let start_y = cursor_y.max(top);
    let bytes_per_pixel = (framebuffer.bpp as usize).div_ceil(8);

    for y in start_y..cursor_bottom {
        for x in start_x..cursor_right {
            let Some(color) = cursor_pixel(cursor.kind, x - cursor_x, y - cursor_y) else {
                continue;
            };
            let offset = y as usize * framebuffer.pitch as usize + x as usize * bytes_per_pixel;
            let pixel = pack_color(framebuffer, color);
            unsafe {
                for byte in 0..bytes_per_pixel {
                    framebuffer
                        .display_pixels
                        .add(offset + byte)
                        .write_volatile((pixel >> (byte * 8)) as u8);
                }
            }
        }
    }
}

fn cursor_origin(cursor: CursorState) -> (u32, u32) {
    let (hotspot_x, hotspot_y) = match cursor.kind {
        CursorType::Arrow => (0, 0),
        CursorType::Hand => (8, 0),
        CursorType::Text | CursorType::Move | CursorType::Crosshair => (10, 10),
    };
    (
        cursor.x.saturating_sub(hotspot_x),
        cursor.y.saturating_sub(hotspot_y),
    )
}

fn cursor_pixel(kind: CursorType, x: u32, y: u32) -> Option<u32> {
    let row = cursor_bitmap(kind).get(y as usize)?;
    match row.as_bytes().get(x as usize)? {
        b'#' => Some(0x101820),
        b'w' => Some(0xF8FAFC),
        b'c' => Some(0x66D9EF),
        _ => None,
    }
}

fn cursor_bitmap(kind: CursorType) -> &'static [&'static str; CURSOR_SIZE as usize] {
    match kind {
        CursorType::Arrow => &[
            "#                   ",
            "##                  ",
            "#w#                 ",
            "#ww#                ",
            "#www#               ",
            "#wwww#              ",
            "#wwwww#             ",
            "#wwwwww#            ",
            "#wwwwwww#           ",
            "#wwwwwwww#          ",
            "#wwwwwwwww#         ",
            "#wwwwwwwwww#        ",
            "#wwwwww#            ",
            "#ww#wwww#           ",
            "##  #wwww#          ",
            "    #wwww#          ",
            "     #wwww#         ",
            "      ####          ",
            "                    ",
            "                    ",
        ],
        CursorType::Hand => &[
            "        ##          ",
            "        #w#         ",
            "        #w#         ",
            "        #w#         ",
            "        #w#         ",
            "        #w#         ",
            "   ##   #w#         ",
            "   #w#  #w#         ",
            "   #w#  #w##        ",
            "   #w#  #www#       ",
            "   #w# #wwww#       ",
            "   #w##wwwww#       ",
            "   #wwwwwwww#       ",
            "   #wwwwwwww#       ",
            "    #wwwwwww#       ",
            "     #wwwww#        ",
            "      #####         ",
            "                    ",
            "                    ",
            "                    ",
        ],
        CursorType::Text => &[
            "                    ",
            "                    ",
            "        ##          ",
            "       #ww#         ",
            "        ##          ",
            "        ##          ",
            "        ##          ",
            "        ##          ",
            "        ##          ",
            "    ##########      ",
            "    ##########      ",
            "        ##          ",
            "        ##          ",
            "        ##          ",
            "        ##          ",
            "       #ww#         ",
            "        ##          ",
            "                    ",
            "                    ",
            "                    ",
        ],
        CursorType::Move => &[
            "         ##         ",
            "        #cc#        ",
            "       #ccc#        ",
            "      #cccc#        ",
            "         ##          ",
            "         ##          ",
            "##       ##       ##",
            "#c#      ##      #c#",
            "#cc##############cc#",
            "#cccccccccccccccccc#",
            "#cc##############cc#",
            "#c#      ##      #c#",
            "##       ##       ##",
            "         ##          ",
            "         ##          ",
            "      #cccc#        ",
            "       #ccc#        ",
            "        #cc#        ",
            "         ##         ",
            "                    ",
        ],
        CursorType::Crosshair => &[
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "####################",
            "####################",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "         ##         ",
            "                    ",
        ],
    }
}

pub fn draw_rect(x: u32, y: u32, width: u32, height: u32, color: u32) {
    let _guard = FRAMEBUFFER_LOCK.lock();
    if width == 0 || height == 0 {
        return;
    }
    fill_rect_unlocked(x, y, width, 2.min(height), color);
    fill_rect_unlocked(
        x,
        y.saturating_add(height - 2.min(height)),
        width,
        2.min(height),
        color,
    );
    fill_rect_unlocked(x, y, 2.min(width), height, color);
    fill_rect_unlocked(
        x.saturating_add(width - 2.min(width)),
        y,
        2.min(width),
        height,
        color,
    );
}

pub fn draw_text(mut x: u32, y: u32, text: &str, color: u32, scale: u32) {
    let _guard = FRAMEBUFFER_LOCK.lock();
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
                        fill_rect_unlocked(
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
