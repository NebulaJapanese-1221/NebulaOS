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

// Syscall interface (int 0x80) and interrupt handler

use crate::services::process::{AppRect, UserEvent};
use crate::userspace::syscall::{
    AppContext, AppEvent, DeviceInfo, DrawCommand, MemoryInfo, ProcessInfo, StorageFileInfo,
    StorageRequest, SystemStats, MAX_WRITE_LENGTH, SYSCALL_DEVICE_INFO, SYSCALL_DRAW_FILL_RECT,
    SYSCALL_DRAW_RECT, SYSCALL_DRAW_TEXT, SYSCALL_EXIT, SYSCALL_GET_APP_CONTEXT, SYSCALL_GET_TICKS,
    SYSCALL_MEMORY_INFO, SYSCALL_POLL_EVENT, SYSCALL_PRINT, SYSCALL_PROCESS_INFO, SYSCALL_REBOOT,
    SYSCALL_SET_MOUSE_SENSITIVITY, SYSCALL_SET_WALLPAPER_THEME, SYSCALL_SHUTDOWN, SYSCALL_SLEEP,
    SYSCALL_STORAGE_DELETE, SYSCALL_STORAGE_FORMAT, SYSCALL_STORAGE_LIST, SYSCALL_STORAGE_READ,
    SYSCALL_STORAGE_RENAME, SYSCALL_STORAGE_WRITE, SYSCALL_SYSTEM_STATS, SYSCALL_TERMINATE_PROCESS,
    SYSCALL_YIELD,
};
use core::mem::size_of;

// Register offsets from regs_ptr (ESP after isr_common pushes)
const REG_EAX: usize = 44 / 4;
const REG_EBX: usize = 32 / 4;
const REG_ECX: usize = 40 / 4;
const REG_CS: usize = 60 / 4;
const STORAGE_MAX_DATA: usize = 512;

// This is called from isr_common in assembly
// interrupt_num: the interrupt vector
// error_code: the error code (0 if none)
// regs_ptr: pointer to saved registers on the stack
#[no_mangle]
pub extern "C" fn isr_handler_rust(interrupt_num: u32, error_code: u32, regs_ptr: *mut u32) -> u32 {
    if interrupt_num == 128 {
        return handle_syscall(regs_ptr);
    }

    if interrupt_num == 32 {
        crate::drivers::pit::pit_tick();
        crate::services::process::wake_sleeping_processes(crate::drivers::pit::ticks());
    } else if interrupt_num == 48 {
        crate::arch::lapic::end_of_interrupt();
    } else if interrupt_num < 32 {
        if regs_ptr.is_null() {
            crate::graphics::system_screen::show(
                "SYSTEM ERROR",
                "CPU exception with invalid register state.",
            );
            loop {
                unsafe {
                    core::arch::asm!("cli", "hlt", options(nomem, nostack));
                }
            }
        }
        crate::services::error::handle_exception(interrupt_num, error_code, regs_ptr);
    }

    if (32..48).contains(&interrupt_num) {
        crate::arch::pic::eoi((interrupt_num - 32) as u8);
    }

    0
}

fn handle_syscall(regs_ptr: *mut u32) -> u32 {
    if regs_ptr.is_null() || unsafe { *regs_ptr.add(REG_CS) & 3 != 3 } {
        return u32::MAX;
    }
    unsafe {
        let syscall_num = *regs_ptr.add(REG_EAX);

        let result = match syscall_num {
            SYSCALL_PRINT => {
                let address = *regs_ptr.add(REG_EBX);
                let len = *regs_ptr.add(REG_ECX) as usize;
                if len > MAX_WRITE_LENGTH
                    || !crate::memory::paging::user_range_is_mapped(address, len)
                {
                    u32::MAX
                } else {
                    let mut buffer = [0u8; MAX_WRITE_LENGTH];
                    if len > 0 {
                        core::ptr::copy_nonoverlapping(
                            address as *const u8,
                            buffer.as_mut_ptr(),
                            len,
                        );
                    }
                    crate::graphics::wm::console_write(&buffer[..len]);
                    crate::services::user_process::record_syscall_output(&buffer[..len]);
                    len as u32
                }
            }
            SYSCALL_EXIT => {
                crate::services::error::report_user_exit(*regs_ptr.add(REG_EBX));
                if crate::services::user_process::exit_from_syscall(
                    regs_ptr,
                    *regs_ptr.add(REG_EBX),
                ) {
                    0
                } else {
                    u32::MAX
                }
            }
            SYSCALL_GET_TICKS => crate::drivers::pit::ticks(),
            SYSCALL_DRAW_FILL_RECT | SYSCALL_DRAW_RECT | SYSCALL_DRAW_TEXT => {
                draw_user_command(*regs_ptr.add(REG_EBX), syscall_num)
            }
            SYSCALL_POLL_EVENT => poll_user_event(*regs_ptr.add(REG_EBX)),
            SYSCALL_GET_APP_CONTEXT => write_app_context(*regs_ptr.add(REG_EBX)),
            SYSCALL_STORAGE_LIST
            | SYSCALL_STORAGE_READ
            | SYSCALL_STORAGE_WRITE
            | SYSCALL_STORAGE_DELETE
            | SYSCALL_STORAGE_RENAME
            | SYSCALL_STORAGE_FORMAT => storage_request(*regs_ptr.add(REG_EBX), syscall_num),
            SYSCALL_SYSTEM_STATS => write_system_stats(*regs_ptr.add(REG_EBX)),
            SYSCALL_SET_WALLPAPER_THEME => {
                let value = *regs_ptr.add(REG_EBX);
                if value <= 1 {
                    crate::graphics::wm::set_dark_theme(value == 1);
                    0
                } else {
                    u32::MAX
                }
            }
            SYSCALL_SET_MOUSE_SENSITIVITY => match *regs_ptr.add(REG_EBX) {
                value @ 1..=4 => {
                    crate::drivers::mouse::set_sensitivity(value as i32);
                    0
                }
                _ => u32::MAX,
            },
            SYSCALL_PROCESS_INFO => write_process_info(*regs_ptr.add(REG_EBX)),
            SYSCALL_MEMORY_INFO => write_memory_info(*regs_ptr.add(REG_EBX)),
            SYSCALL_DEVICE_INFO => write_device_info(*regs_ptr.add(REG_EBX)),
            SYSCALL_YIELD => {
                if crate::services::process::yield_current_user_process() {
                    0
                } else {
                    u32::MAX
                }
            }
            SYSCALL_SLEEP => {
                let ticks = *regs_ptr.add(REG_EBX);
                if crate::services::process::sleep_current_user_process(
                    crate::drivers::pit::ticks(),
                    ticks,
                ) {
                    0
                } else {
                    u32::MAX
                }
            }
            SYSCALL_TERMINATE_PROCESS => {
                let pid = *regs_ptr.add(REG_EBX);
                if pid != crate::services::process::current_pid()
                    && crate::services::user_process::terminate(pid)
                {
                    0
                } else {
                    u32::MAX
                }
            }
            SYSCALL_REBOOT => crate::services::system_control::reboot(),
            SYSCALL_SHUTDOWN => crate::services::system_control::shutdown(),
            _ => u32::MAX,
        };
        *regs_ptr.add(REG_EAX) = result;
        result
    }
}

unsafe fn read_user<T: Copy>(address: u32) -> Option<T> {
    if !crate::memory::paging::user_range_is_mapped(address, size_of::<T>()) {
        return None;
    }
    Some(core::ptr::read_unaligned(address as *const T))
}

unsafe fn write_user<T: Copy>(address: u32, value: &T) -> bool {
    if !crate::memory::paging::user_range_is_mapped(address, size_of::<T>()) {
        return false;
    }
    core::ptr::write_unaligned(address as *mut T, *value);
    true
}

fn poll_user_event(address: u32) -> u32 {
    if !crate::memory::paging::user_range_is_mapped(address, size_of::<UserEvent>()) {
        return u32::MAX;
    }
    if let Some(content) = crate::services::process::take_current_draw_dirty() {
        crate::drivers::framebuffer::present_rect(
            content.x,
            content.y,
            content.width,
            content.height,
        );
    }
    let Some(event) = crate::services::process::take_current_event() else {
        crate::services::process::block_current_user_process();
        return 0;
    };
    unsafe {
        let abi_event = AppEvent {
            kind: event.kind,
            key: event.key,
            x: event.x,
            y: event.y,
            left_down: event.left_down,
        };
        if write_user(address, &abi_event) {
            1
        } else {
            u32::MAX
        }
    }
}

fn write_app_context(address: u32) -> u32 {
    let Some((app_id, rect)) = crate::services::process::current_app_context() else {
        return u32::MAX;
    };
    let context = AppContext {
        app_id,
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        dark_theme: u32::from(crate::graphics::wm::is_dark_theme()),
        mouse_sensitivity: crate::drivers::mouse::sensitivity(),
    };
    unsafe {
        if write_user(address, &context) {
            0
        } else {
            u32::MAX
        }
    }
}

fn write_system_stats(address: u32) -> u32 {
    let stats = SystemStats {
        ticks: crate::drivers::pit::ticks(),
        free_frames: crate::memory::frame_allocator::free_frame_count() as u32,
        total_frames: crate::memory::frame_allocator::total_frame_count() as u32,
    };
    unsafe {
        if write_user(address, &stats) {
            0
        } else {
            u32::MAX
        }
    }
}

fn write_process_info(address: u32) -> u32 {
    let info: ProcessInfo = crate::services::process::process_info();
    unsafe {
        if write_user(address, &info) {
            0
        } else {
            u32::MAX
        }
    }
}

fn write_memory_info(address: u32) -> u32 {
    let free_frames = crate::memory::frame_allocator::free_frame_count() as u32;
    let total_frames = crate::memory::frame_allocator::total_frame_count() as u32;
    let info = MemoryInfo {
        total_frames,
        free_frames,
        allocated_frames: total_frames.saturating_sub(free_frames),
        frame_size: 4096,
    };
    unsafe {
        if write_user(address, &info) {
            0
        } else {
            u32::MAX
        }
    }
}

fn write_device_info(address: u32) -> u32 {
    let mouse = crate::drivers::mouse::state();
    let info = DeviceInfo {
        framebuffer_width: crate::drivers::framebuffer::get_width(),
        framebuffer_height: crate::drivers::framebuffer::get_height(),
        mouse_x: mouse.x.max(0) as u32,
        mouse_y: mouse.y.max(0) as u32,
        mouse_buttons: u32::from(mouse.left)
            | (u32::from(mouse.right) << 1)
            | (u32::from(mouse.middle) << 2),
        mouse_sensitivity: crate::drivers::mouse::sensitivity(),
        mouse_available: u32::from(crate::drivers::mouse::is_available()),
        storage_available: u32::from(crate::drivers::disk::is_available()),
    };
    unsafe {
        if write_user(address, &info) {
            0
        } else {
            u32::MAX
        }
    }
}

fn draw_user_command(address: u32, syscall: u32) -> u32 {
    let command = unsafe { read_user::<DrawCommand>(address) };
    let Some(command) = command else {
        return u32::MAX;
    };
    let Some((_, area)) = crate::services::process::current_app_context() else {
        return u32::MAX;
    };
    if !rect_within(command.x, command.y, command.width, command.height, area)
        || command.width == 0
        || command.height == 0
    {
        return u32::MAX;
    }
    let x = area.x + command.x;
    let y = area.y + command.y;
    let fb_width = crate::drivers::framebuffer::get_width();
    let fb_height = crate::drivers::framebuffer::get_height();
    if x.checked_add(command.width)
        .is_none_or(|right| right > fb_width)
        || y.checked_add(command.height)
            .is_none_or(|bottom| bottom > fb_height)
    {
        return u32::MAX;
    }
    match syscall {
        SYSCALL_DRAW_FILL_RECT if command.kind == 0 => {
            crate::drivers::framebuffer::fill_rect(
                x,
                y,
                command.width,
                command.height,
                command.color,
            );
            crate::services::process::mark_current_draw_dirty();
            0
        }
        SYSCALL_DRAW_RECT if command.kind == 1 => {
            crate::drivers::framebuffer::draw_rect(
                x,
                y,
                command.width,
                command.height,
                command.color,
            );
            crate::services::process::mark_current_draw_dirty();
            0
        }
        SYSCALL_DRAW_TEXT if command.kind == 2 => {
            let length = command.text_len as usize;
            if length > MAX_WRITE_LENGTH
                || command.scale == 0
                || command.scale > 8
                || !crate::memory::paging::user_range_is_mapped(command.text_ptr, length)
            {
                return u32::MAX;
            }
            let text_width = (length as u32)
                .checked_mul(6)
                .and_then(|width| width.checked_mul(command.scale));
            let text_height = 8u32.checked_mul(command.scale);
            let (Some(text_width), Some(text_height)) = (text_width, text_height) else {
                return u32::MAX;
            };
            if text_width > command.width
                || text_height > command.height
                || x.checked_add(text_width)
                    .is_none_or(|right| right > area.x + area.width)
                || y.checked_add(text_height)
                    .is_none_or(|bottom| bottom > area.y + area.height)
                || x.checked_add(text_width)
                    .is_none_or(|right| right > fb_width)
                || y.checked_add(text_height)
                    .is_none_or(|bottom| bottom > fb_height)
            {
                return u32::MAX;
            }
            let bytes =
                unsafe { core::slice::from_raw_parts(command.text_ptr as *const u8, length) };
            let Ok(text) = core::str::from_utf8(bytes) else {
                return u32::MAX;
            };
            crate::drivers::framebuffer::draw_text(x, y, text, command.color, command.scale);
            crate::services::process::mark_current_draw_dirty();
            0
        }
        _ => u32::MAX,
    }
}

fn rect_within(x: u32, y: u32, width: u32, height: u32, area: AppRect) -> bool {
    x.checked_add(width)
        .is_some_and(|right| right <= area.width)
        && y.checked_add(height)
            .is_some_and(|bottom| bottom <= area.height)
        && area.x.checked_add(area.width).is_some()
        && area.y.checked_add(area.height).is_some()
}

fn storage_request(address: u32, syscall: u32) -> u32 {
    let Some(mut request) = (unsafe { read_user::<StorageRequest>(address) }) else {
        return u32::MAX;
    };
    let expected_op = syscall - SYSCALL_STORAGE_LIST;
    if request.op != expected_op
        || request.name_len > 15
        || request.data_len as usize > STORAGE_MAX_DATA
    {
        return u32::MAX;
    }
    let output_limit = if request.op == 0 {
        size_of::<StorageFileInfo>() * 16
    } else if request.op == 1 {
        STORAGE_MAX_DATA
    } else {
        0
    };
    if request.out_len as usize > output_limit {
        return u32::MAX;
    }
    let name_len = request.name_len as usize;
    let data_len = request.data_len as usize;
    if !crate::memory::paging::user_range_is_mapped(request.name_ptr, name_len)
        || !crate::memory::paging::user_range_is_mapped(request.data_ptr, data_len)
        || !crate::memory::paging::user_range_is_mapped(request.out_ptr, request.out_len as usize)
    {
        return u32::MAX;
    }
    let mut name = [0u8; 15];
    let mut data = [0u8; STORAGE_MAX_DATA];
    unsafe {
        if name_len != 0 {
            core::ptr::copy_nonoverlapping(
                request.name_ptr as *const u8,
                name.as_mut_ptr(),
                name_len,
            );
        }
        if data_len != 0 {
            core::ptr::copy_nonoverlapping(
                request.data_ptr as *const u8,
                data.as_mut_ptr(),
                data_len,
            );
        }
    }
    let name = &name[..name_len];
    let result: Result<usize, crate::drivers::disk::StorageError> = match request.op {
        0 => {
            let files = match crate::drivers::disk::list_files() {
                Ok(files) => files,
                Err(error) => return storage_error(error),
            };
            let capacity = request.out_len as usize / size_of::<StorageFileInfo>();
            let count = files
                .iter()
                .filter(|file| file.length > 0 || file.name[0] != 0)
                .count();
            if capacity < count {
                return u32::MAX;
            }
            let mut output_index = 0;
            for file in files
                .iter()
                .filter(|file| file.length > 0 || file.name[0] != 0)
            {
                let info = StorageFileInfo {
                    name: file.name,
                    length: file.length,
                };
                unsafe {
                    core::ptr::write_unaligned(
                        (request.out_ptr as *mut StorageFileInfo).add(output_index),
                        info,
                    );
                }
                output_index += 1;
            }
            Ok(count * size_of::<StorageFileInfo>())
        }
        1 => {
            let mut output = [0u8; STORAGE_MAX_DATA];
            match crate::drivers::disk::read_file(name, &mut output) {
                Ok(length) if length <= request.out_len as usize => {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            output.as_ptr(),
                            request.out_ptr as *mut u8,
                            length,
                        );
                    }
                    Ok(length)
                }
                Ok(_) => return u32::MAX,
                Err(error) => Err(error),
            }
        }
        2 => crate::drivers::disk::write_file(name, &data[..data_len]).map(|()| 0),
        3 => crate::drivers::disk::delete_file(name).map(|()| 0),
        4 => crate::drivers::disk::rename_file(name, &data[..data_len]).map(|()| 0),
        5 => crate::drivers::disk::format_if_blank().map(|()| 0),
        _ => return u32::MAX,
    };
    match result {
        Ok(copied) => {
            if request.op == 1 {
                request.out_len = copied as u32;
                unsafe {
                    if write_user(address, &request) {
                        0
                    } else {
                        u32::MAX
                    }
                }
            } else {
                0
            }
        }
        Err(error) => storage_error(error),
    }
}

fn storage_error(error: crate::drivers::disk::StorageError) -> u32 {
    use crate::drivers::disk::StorageError;
    match error {
        StorageError::NoDisk => 1,
        StorageError::Io => 2,
        StorageError::NotFormatted => 3,
        StorageError::NotBlank => 4,
        StorageError::InvalidName => 5,
        StorageError::FileTooLarge => 6,
        StorageError::DirectoryFull => 7,
        StorageError::NotFound => 8,
    }
}
