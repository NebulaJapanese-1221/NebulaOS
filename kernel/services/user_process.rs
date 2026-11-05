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

use crate::memory::{frame_allocator, paging};
use crate::services::process::{self, AppRect, UserEvent};
use crate::sync::SpinLock;

const USER_IMAGE_BASE: u32 = 0x4000_0000;
const USER_IMAGE_LIMIT: u32 = 0x4010_0000;
const USER_STACK_TOP: u32 = 0xC000_0000;
const USER_STACK_PAGES: u32 = 4;
const PAGE_SIZE: u32 = 4096;
const PT_LOAD: u32 = 1;
const PF_WRITE: u32 = 2;
const PF_EXECUTE: u32 = 1;
const ELF_HEADER_SIZE: usize = 52;
const PROGRAM_HEADER_SIZE: usize = 32;
const MAX_PROGRAM_HEADERS: usize = 16;
const KERNEL_PHYSICAL_LIMIT: usize = 0x0080_0000;
const LAST_OUTPUT_CAPACITY: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UserProcessError {
    OutOfMemory,
    InvalidElf,
    NoLoadableSegments,
    InvalidEntryPoint,
    LoadFailed,
}

struct LastOutput {
    bytes: [u8; LAST_OUTPUT_CAPACITY],
    length: usize,
}

static LAST_OUTPUT: SpinLock<LastOutput> = SpinLock::new(LastOutput {
    bytes: [0; LAST_OUTPUT_CAPACITY],
    length: 0,
});

unsafe extern "C" {
    #[link_name = "_binary_demo_elf_start"]
    static USER_APP_START: u8;
    #[link_name = "_binary_demo_elf_end"]
    static USER_APP_END: u8;
}

pub fn spawn_app(app_id: u32, content: AppRect) -> Result<u32, UserProcessError> {
    let image_start = core::ptr::addr_of!(USER_APP_START);
    let image_end = core::ptr::addr_of!(USER_APP_END);
    let Some(embedded_len) = (image_end as usize).checked_sub(image_start as usize) else {
        return Err(UserProcessError::InvalidElf);
    };
    if embedded_len == 0 {
        return Err(UserProcessError::InvalidElf);
    }
    let embedded = unsafe { core::slice::from_raw_parts(image_start, embedded_len) };
    spawn_image(embedded, app_id, content)
}

fn spawn_image(image: &[u8], app_id: u32, content: AppRect) -> Result<u32, UserProcessError> {
    let page_directory =
        paging::create_user_address_space().ok_or(UserProcessError::OutOfMemory)?;

    let entry = match load_elf(image, page_directory) {
        Ok(entry) => entry,
        Err(error) => {
            paging::destroy_user_address_space(page_directory);
            return Err(error);
        }
    };

    for page in 1..=USER_STACK_PAGES {
        if paging::map_user_page(page_directory, USER_STACK_TOP - page * PAGE_SIZE, true).is_none()
        {
            paging::destroy_user_address_space(page_directory);
            return Err(UserProcessError::OutOfMemory);
        }
    }
    let argument_stack = USER_STACK_TOP - 6 * core::mem::size_of::<u32>() as u32;
    let Some(stack_frame) = paging::user_page_frame(page_directory, USER_STACK_TOP - PAGE_SIZE)
    else {
        paging::destroy_user_address_space(page_directory);
        return Err(UserProcessError::LoadFailed);
    };
    let argument_frame = (stack_frame as usize
        + (argument_stack - (USER_STACK_TOP - PAGE_SIZE)) as usize)
        as *mut u32;
    unsafe {
        *argument_frame.add(0) = 0;
        *argument_frame.add(1) = app_id;
        *argument_frame.add(2) = content.x;
        *argument_frame.add(3) = content.y;
        *argument_frame.add(4) = content.width;
        *argument_frame.add(5) = content.height;
    }

    let Some(kernel_stack) = frame_allocator::alloc_frame_below(KERNEL_PHYSICAL_LIMIT) else {
        paging::destroy_user_address_space(page_directory);
        return Err(UserProcessError::OutOfMemory);
    };
    unsafe {
        core::ptr::write_bytes(kernel_stack as *mut u8, 0, PAGE_SIZE as usize);
    }
    let Some(pid) = process::add_user_process(
        entry,
        argument_stack,
        page_directory,
        kernel_stack,
        app_id,
        content,
    ) else {
        paging::destroy_user_address_space(page_directory);
        frame_allocator::free_frame(kernel_stack);
        return Err(UserProcessError::OutOfMemory);
    };
    crate::drivers::serial::write_string("Queued ring-3 application PID ");
    crate::drivers::serial::write_dec(pid);
    crate::drivers::serial::write_string(".\r\n");
    Ok(pid)
}

pub fn route_keyboard_event(pid: u32, key: u32) -> bool {
    process::route_event(
        pid,
        UserEvent {
            kind: 1,
            key,
            ..UserEvent::default()
        },
    )
}

pub fn route_pointer_event(pid: u32, x: u32, y: u32, left_down: bool) -> bool {
    process::route_event(
        pid,
        UserEvent {
            kind: 2,
            x,
            y,
            left_down: u32::from(left_down),
            ..UserEvent::default()
        },
    )
}

pub fn request_redraw(
    pid: u32,
    content: AppRect,
    pointer_x: u32,
    pointer_y: u32,
    left_down: bool,
) -> bool {
    process::update_content_rect(pid, content, pointer_x, pointer_y, left_down)
}

pub fn terminate(pid: u32) -> bool {
    process::terminate_process(pid)
}

pub fn record_syscall_output(bytes: &[u8]) {
    let mut output = LAST_OUTPUT.lock();
    let copy_length = bytes.len().min(LAST_OUTPUT_CAPACITY - output.length);
    let start = output.length;
    let end = start + copy_length;
    output.bytes[start..end].copy_from_slice(&bytes[..copy_length]);
    output.length = end;
}

pub fn copy_last_output(destination: &mut [u8]) -> usize {
    let output = LAST_OUTPUT.lock();
    let length = output.length.min(destination.len());
    destination[..length].copy_from_slice(&output.bytes[..length]);
    length
}

pub fn exit_from_syscall(_registers: *mut u32, _code: u32) -> bool {
    crate::graphics::wm::request_desktop_redraw();
    process::terminate_current_user_process()
}

fn load_elf(image: &[u8], page_directory: u32) -> Result<u32, UserProcessError> {
    if image.len() < ELF_HEADER_SIZE
        || &image[..4] != b"\x7fELF"
        || image[4] != 1
        || image[5] != 1
        || image[6] != 1
        || read_u16(image, 16)? != 2
        || read_u16(image, 18)? != 3
        || read_u32(image, 20)? != 1
        || read_u16(image, 40)? as usize != ELF_HEADER_SIZE
        || read_u16(image, 42)? as usize != PROGRAM_HEADER_SIZE
    {
        return Err(UserProcessError::InvalidElf);
    }

    let entry = read_u32(image, 24)?;
    let header_offset = read_u32(image, 28)? as usize;
    let header_count = read_u16(image, 44)? as usize;
    if header_count == 0 || header_count > MAX_PROGRAM_HEADERS {
        return Err(UserProcessError::InvalidElf);
    }
    let headers_len = header_count
        .checked_mul(PROGRAM_HEADER_SIZE)
        .and_then(|length| header_offset.checked_add(length))
        .filter(|end| *end <= image.len())
        .ok_or(UserProcessError::InvalidElf)?;

    let mut loadable_segments = 0;
    let mut entry_is_executable = false;
    for header_offset in (header_offset..headers_len).step_by(PROGRAM_HEADER_SIZE) {
        if read_u32(image, header_offset)? != PT_LOAD {
            continue;
        }
        let file_offset = read_u32(image, header_offset + 4)? as usize;
        let virtual_address = read_u32(image, header_offset + 8)?;
        let file_size = read_u32(image, header_offset + 16)? as usize;
        let memory_size = read_u32(image, header_offset + 20)? as usize;
        let flags = read_u32(image, header_offset + 24)?;

        if file_size > memory_size {
            return Err(UserProcessError::InvalidElf);
        }
        if memory_size == 0 {
            continue;
        }
        let file_end = file_offset
            .checked_add(file_size)
            .filter(|end| *end <= image.len())
            .ok_or(UserProcessError::InvalidElf)?;
        let segment_end = (virtual_address as usize)
            .checked_add(memory_size)
            .filter(|end| virtual_address >= USER_IMAGE_BASE && *end <= USER_IMAGE_LIMIT as usize)
            .ok_or(UserProcessError::InvalidElf)?;
        if flags & PF_EXECUTE != 0 && entry >= virtual_address && (entry as usize) < segment_end {
            entry_is_executable = true;
        }
        load_segment(
            image,
            page_directory,
            file_offset,
            virtual_address,
            file_end,
            virtual_address
                .checked_add(file_size as u32)
                .ok_or(UserProcessError::InvalidElf)?,
            segment_end as u32,
            flags & PF_WRITE != 0,
        )?;
        loadable_segments += 1;
    }

    if loadable_segments == 0 {
        return Err(UserProcessError::NoLoadableSegments);
    }
    if !entry_is_executable {
        return Err(UserProcessError::InvalidEntryPoint);
    }
    Ok(entry)
}

fn load_segment(
    image: &[u8],
    page_directory: u32,
    file_offset: usize,
    virtual_address: u32,
    file_end: usize,
    file_memory_end: u32,
    memory_end: u32,
    writable: bool,
) -> Result<(), UserProcessError> {
    let memory_start = virtual_address as usize;
    let aligned_start = virtual_address & !(PAGE_SIZE - 1);
    let aligned_end = memory_end
        .checked_add(PAGE_SIZE - 1)
        .map(|end| end & !(PAGE_SIZE - 1))
        .ok_or(UserProcessError::InvalidElf)?;

    let mut page = aligned_start;
    while page < aligned_end {
        if paging::user_page_frame(page_directory, page).is_none()
            && paging::map_user_page(page_directory, page, writable).is_none()
        {
            return Err(UserProcessError::OutOfMemory);
        }
        if writable && !paging::set_user_page_writable(page_directory, page) {
            return Err(UserProcessError::LoadFailed);
        }
        page = page
            .checked_add(PAGE_SIZE)
            .ok_or(UserProcessError::InvalidElf)?;
    }

    let file_size = file_end - file_offset;
    let mut copied = 0;
    while copied < file_size {
        let destination = memory_start + copied;
        let page_address = (destination as u32) & !(PAGE_SIZE - 1);
        let page_offset = destination & (PAGE_SIZE as usize - 1);
        let copy_len = (PAGE_SIZE as usize - page_offset).min(file_size - copied);
        let frame = paging::user_page_frame(page_directory, page_address)
            .ok_or(UserProcessError::LoadFailed)?;
        unsafe {
            core::ptr::copy_nonoverlapping(
                image.as_ptr().add(file_offset + copied),
                (frame as usize + page_offset) as *mut u8,
                copy_len,
            );
        }
        copied += copy_len;
    }
    let mut cleared = file_memory_end;
    while cleared < memory_end {
        let page_address = cleared & !(PAGE_SIZE - 1);
        let page_offset = (cleared - page_address) as usize;
        let clear_len = (PAGE_SIZE as usize - page_offset).min((memory_end - cleared) as usize);
        let frame = paging::user_page_frame(page_directory, page_address)
            .ok_or(UserProcessError::LoadFailed)?;
        unsafe {
            core::ptr::write_bytes((frame as usize + page_offset) as *mut u8, 0, clear_len);
        }
        cleared += clear_len as u32;
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, UserProcessError> {
    let value = bytes
        .get(offset..offset.checked_add(2).ok_or(UserProcessError::InvalidElf)?)
        .ok_or(UserProcessError::InvalidElf)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, UserProcessError> {
    let value = bytes
        .get(offset..offset.checked_add(4).ok_or(UserProcessError::InvalidElf)?)
        .ok_or(UserProcessError::InvalidElf)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}
