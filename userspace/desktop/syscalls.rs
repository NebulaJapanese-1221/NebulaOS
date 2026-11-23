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

// Userspace syscall wrappers

use core::arch::asm;

pub const SYSCALL_EXIT: u32 = 0;
pub const SYSCALL_PRINT: u32 = 1;
pub const SYSCALL_GET_TICKS: u32 = 2;
pub const SYSCALL_DRAW_FILL_RECT: u32 = 3;
pub const SYSCALL_DRAW_RECT: u32 = 4;
pub const SYSCALL_DRAW_TEXT: u32 = 5;
pub const SYSCALL_POLL_EVENT: u32 = 6;
pub const SYSCALL_GET_APP_CONTEXT: u32 = 7;
pub const SYSCALL_STORAGE_LIST: u32 = 8;
pub const SYSCALL_STORAGE_READ: u32 = 9;
pub const SYSCALL_STORAGE_WRITE: u32 = 10;
pub const SYSCALL_STORAGE_DELETE: u32 = 11;
pub const SYSCALL_STORAGE_RENAME: u32 = 12;
pub const SYSCALL_STORAGE_FORMAT: u32 = 13;
pub const SYSCALL_SYSTEM_STATS: u32 = 14;
pub const SYSCALL_SET_WALLPAPER_THEME: u32 = 15;
pub const SYSCALL_SET_MOUSE_SENSITIVITY: u32 = 16;
pub const SYSCALL_PROCESS_INFO: u32 = 17;
pub const SYSCALL_MEMORY_INFO: u32 = 18;
pub const SYSCALL_DEVICE_INFO: u32 = 19;
pub const SYSCALL_PCI_COUNT: u32 = 20;
pub const SYSCALL_PCI_DEVICE: u32 = 21;
pub const SYSCALL_GET_HOSTNAME: u32 = 22;
pub const SYSCALL_YIELD: u32 = 23;
pub const SYSCALL_NICE: u32 = 24;
pub const SYSCALL_SLEEP: u32 = 25;
pub const SYSCALL_TERMINATE_PROCESS: u32 = 26;
pub const SYSCALL_REBOOT: u32 = 27;
pub const SYSCALL_SHUTDOWN: u32 = 28;
pub const SYSCALL_CLIPBOARD_SET: u32 = 29;
pub const SYSCALL_CLIPBOARD_GET: u32 = 30;
pub const SYSCALL_SPAWN_APP: u32 = 31;
pub const SYSCALL_REQUEST_REDRAW: u32 = 32;
pub const SYSCALL_GET_KEY: u32 = 33;
pub const SYSCALL_GET_MOUSE: u32 = 34;
pub const SYSCALL_PRESENT: u32 = 35;
pub const SYSCALL_PRESENT_RECT: u32 = 36;
pub const SYSCALL_GET_CURSOR: u32 = 37;
pub const SYSCALL_SET_CURSOR: u32 = 38;
pub const SYSCALL_GET_WIDTH: u32 = 39;
pub const SYSCALL_GET_HEIGHT: u32 = 40;

pub const CLIPBOARD_MAX: usize = 256;
pub const MAX_WRITE_LENGTH: usize = 1024;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DrawCommand {
    pub kind: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub color: u32,
    pub text_ptr: u32,
    pub text_len: u32,
    pub scale: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AppRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AppContext {
    pub app_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub dark_theme: u32,
    pub mouse_sensitivity: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AppEvent {
    pub kind: u32,
    pub key: u32,
    pub x: u32,
    pub y: u32,
    pub left_down: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StorageRequest {
    pub op: u32,
    pub name_ptr: u32,
    pub name_len: u32,
    pub data_ptr: u32,
    pub data_len: u32,
    pub out_ptr: u32,
    pub out_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StorageFileInfo {
    pub name: [u8; 16],
    pub length: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemStats {
    pub ticks: u32,
    pub free_frames: u32,
    pub total_frames: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: [u8; 32],
    pub priority: i32,
    pub state: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryInfo {
    pub total_frames: u32,
    pub free_frames: u32,
    pub allocated_frames: u32,
    pub frame_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DeviceInfo {
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub mouse_x: u32,
    pub mouse_y: u32,
    pub mouse_buttons: u32,
    pub mouse_sensitivity: i32,
    pub mouse_available: u32,
    pub storage_available: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PciDeviceInfo {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub _pad: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SpawnAppRequest {
    pub app_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RedrawRequest {
    pub pid: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub mouse_x: u32,
    pub mouse_y: u32,
    pub left_down: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CursorInfo {
    pub x: u32,
    pub y: u32,
    pub cursor_type: u32,
}

impl DrawCommand {
    pub fn fill_rect(x: u32, y: u32, width: u32, height: u32, color: u32) -> Self {
        Self {
            kind: 0,
            x,
            y,
            width,
            height,
            color,
            ..Default::default()
        }
    }

    pub fn rect(x: u32, y: u32, width: u32, height: u32, color: u32) -> Self {
        Self {
            kind: 1,
            x,
            y,
            width,
            height,
            color,
            ..Default::default()
        }
    }

    pub fn text(x: u32, y: u32, width: u32, height: u32, color: u32, text: &str, scale: u32) -> Self {
        let text_ptr = text.as_ptr() as u32;
        let text_len = text.len() as u32;
        Self {
            kind: 2,
            x,
            y,
            width,
            height,
            color,
            text_ptr,
            text_len,
            scale,
        }
    }
}

#[inline(always)]
fn syscall0(num: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("int 0x80", in("eax") num, lateout("eax") result, options(nostack, preserves_flags));
    }
    result
}

#[inline(always)]
fn syscall1(num: u32, arg1: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("int 0x80", in("eax") num, in("ebx") arg1, lateout("eax") result, options(nostack, preserves_flags));
    }
    result
}

#[inline(always)]
fn syscall2(num: u32, arg1: u32, arg2: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("int 0x80", in("eax") num, in("ebx") arg1, in("ecx") arg2, lateout("eax") result, options(nostack, preserves_flags));
    }
    result
}

#[inline(always)]
fn syscall3(num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("int 0x80", in("eax") num, in("ebx") arg1, in("ecx") arg2, in("edx") arg3, lateout("eax") result, options(nostack, preserves_flags));
    }
    result
}

pub fn exit(code: u32) -> ! {
    syscall1(SYSCALL_EXIT, code);
    loop {}
}

pub fn print(s: &str) {
    let len = s.len().min(MAX_WRITE_LENGTH);
    syscall2(SYSCALL_PRINT, s.as_ptr() as u32, len as u32);
}

pub fn get_ticks() -> u32 {
    syscall0(SYSCALL_GET_TICKS)
}

pub fn draw_fill_rect(cmd: &DrawCommand) -> Result<(), u32> {
    let res = syscall2(SYSCALL_DRAW_FILL_RECT, cmd as *const _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn draw_rect(cmd: &DrawCommand) -> Result<(), u32> {
    let res = syscall2(SYSCALL_DRAW_RECT, cmd as *const _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn draw_text(cmd: &DrawCommand) -> Result<(), u32> {
    let res = syscall2(SYSCALL_DRAW_TEXT, cmd as *const _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn poll_event(event: &mut AppEvent) -> Result<u32, u32> {
    let res = syscall2(SYSCALL_POLL_EVENT, event as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(res) }
}

pub fn get_app_context(ctx: &mut AppContext) -> Result<(), u32> {
    let res = syscall2(SYSCALL_GET_APP_CONTEXT, ctx as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn storage_list(buf: &mut [StorageFileInfo]) -> Result<usize, u32> {
    let mut req = StorageRequest {
        op: 0,
        name_ptr: 0,
        name_len: 0,
        data_ptr: 0,
        data_len: 0,
        out_ptr: buf.as_mut_ptr() as u32,
        out_len: (buf.len() * core::mem::size_of::<StorageFileInfo>()) as u32,
    };
    let res = syscall2(SYSCALL_STORAGE_LIST, &mut req as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(res as usize) }
}

pub fn storage_read(name: &str, buf: &mut [u8]) -> Result<usize, u32> {
    let mut req = StorageRequest {
        op: 1,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        data_ptr: 0,
        data_len: 0,
        out_ptr: buf.as_mut_ptr() as u32,
        out_len: buf.len() as u32,
    };
    let res = syscall2(SYSCALL_STORAGE_READ, &mut req as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(req.out_len as usize) }
}

pub fn storage_write(name: &str, data: &[u8]) -> Result<(), u32> {
    let mut req = StorageRequest {
        op: 2,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        data_ptr: data.as_ptr() as u32,
        data_len: data.len() as u32,
        out_ptr: 0,
        out_len: 0,
    };
    let res = syscall2(SYSCALL_STORAGE_WRITE, &mut req as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn storage_delete(name: &str) -> Result<(), u32> {
    let mut req = StorageRequest {
        op: 3,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        data_ptr: 0,
        data_len: 0,
        out_ptr: 0,
        out_len: 0,
    };
    let res = syscall2(SYSCALL_STORAGE_DELETE, &mut req as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn storage_rename(old_name: &str, new_name: &str) -> Result<(), u32> {
    let mut req = StorageRequest {
        op: 4,
        name_ptr: old_name.as_ptr() as u32,
        name_len: old_name.len() as u32,
        data_ptr: new_name.as_ptr() as u32,
        data_len: new_name.len() as u32,
        out_ptr: 0,
        out_len: 0,
    };
    let res = syscall2(SYSCALL_STORAGE_RENAME, &mut req as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn storage_format() -> Result<(), u32> {
    let mut req = StorageRequest {
        op: 5,
        name_ptr: 0,
        name_len: 0,
        data_ptr: 0,
        data_len: 0,
        out_ptr: 0,
        out_len: 0,
    };
    let res = syscall2(SYSCALL_STORAGE_FORMAT, &mut req as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn system_stats(stats: &mut SystemStats) -> Result<(), u32> {
    let res = syscall2(SYSCALL_SYSTEM_STATS, stats as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn set_dark_theme(enabled: bool) -> Result<(), u32> {
    let res = syscall1(SYSCALL_SET_WALLPAPER_THEME, enabled as u32);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn set_mouse_sensitivity(value: i32) -> Result<(), u32> {
    let res = syscall1(SYSCALL_SET_MOUSE_SENSITIVITY, value as u32);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn process_info(info: &mut ProcessInfo) -> Result<(), u32> {
    let res = syscall2(SYSCALL_PROCESS_INFO, info as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn memory_info(info: &mut MemoryInfo) -> Result<(), u32> {
    let res = syscall2(SYSCALL_MEMORY_INFO, info as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn device_info(info: &mut DeviceInfo) -> Result<(), u32> {
    let res = syscall2(SYSCALL_DEVICE_INFO, info as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn pci_count() -> u32 {
    syscall0(SYSCALL_PCI_COUNT)
}

pub fn pci_device(index: usize, info: &mut PciDeviceInfo) -> Result<(), u32> {
    let res = syscall3(SYSCALL_PCI_DEVICE, index as u32, info as *mut _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn get_hostname(buf: &mut [u8]) -> Result<usize, u32> {
    let res = syscall3(SYSCALL_GET_HOSTNAME, buf.as_mut_ptr() as u32, buf.len() as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(res as usize) }
}

pub fn yield_now() -> Result<(), u32> {
    let res = syscall0(SYSCALL_YIELD);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn nice(increment: i32) -> Result<i32, u32> {
    let res = syscall1(SYSCALL_NICE, increment as u32);
    if res == u32::MAX { Err(res) } else { Ok(res as i32) }
}

pub fn sleep(ticks: u32) -> Result<(), u32> {
    let res = syscall1(SYSCALL_SLEEP, ticks);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn terminate_process(pid: u32) -> Result<(), u32> {
    let res = syscall1(SYSCALL_TERMINATE_PROCESS, pid);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn reboot() -> u32 {
    syscall0(SYSCALL_REBOOT)
}

pub fn shutdown() -> u32 {
    syscall0(SYSCALL_SHUTDOWN)
}

pub fn clipboard_set(data: &[u8]) -> Result<usize, u32> {
    let len = data.len().min(CLIPBOARD_MAX);
    let res = syscall2(SYSCALL_CLIPBOARD_SET, data.as_ptr() as u32, len as u32);
    if res == u32::MAX { Err(res) } else { Ok(res as usize) }
}

pub fn clipboard_get(buf: &mut [u8]) -> Result<usize, u32> {
    let res = syscall2(SYSCALL_CLIPBOARD_GET, buf.as_mut_ptr() as u32, buf.len() as u32);
    if res == u32::MAX { Err(res) } else { Ok(res as usize) }
}

pub fn spawn_app(req: &SpawnAppRequest) -> Result<u32, u32> {
    let res = syscall2(SYSCALL_SPAWN_APP, req as *const _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(res) }
}

pub fn request_redraw(req: &RedrawRequest) -> Result<(), u32> {
    let res = syscall2(SYSCALL_REQUEST_REDRAW, req as *const _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn get_key() -> Option<u8> {
    let res = syscall0(SYSCALL_GET_KEY);
    if res == u32::MAX || res == 0 { None } else { Some(res as u8) }
}

pub fn get_mouse() -> Option<(u32, u32, bool, bool, bool)> {
    let mut x = 0u32;
    let mut y = 0u32;
    let mut buttons = 0u32;
    let res = syscall3(SYSCALL_GET_MOUSE, &mut x as *mut _ as u32, &mut y as *mut _ as u32, &mut buttons as *mut _ as u32);
    if res == u32::MAX { None } else { Some((x, y, buttons & 1 != 0, buttons & 2 != 0, buttons & 4 != 0)) }
}

pub fn present() -> Result<(), u32> {
    let res = syscall0(SYSCALL_PRESENT);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn present_rect(x: u32, y: u32, width: u32, height: u32) -> Result<(), u32> {
    let res = syscall3(SYSCALL_PRESENT_RECT, x, y, (width << 16) | height);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn get_cursor() -> Option<CursorInfo> {
    let mut info = CursorInfo::default();
    let res = syscall2(SYSCALL_GET_CURSOR, &mut info as *mut _ as u32, 0);
    if res == u32::MAX { None } else { Some(info) }
}

pub fn set_cursor(info: &CursorInfo) -> Result<(), u32> {
    let res = syscall2(SYSCALL_SET_CURSOR, info as *const _ as u32, 0);
    if res == u32::MAX { Err(res) } else { Ok(()) }
}

pub fn get_width() -> u32 {
    syscall0(SYSCALL_GET_WIDTH)
}

pub fn get_height() -> u32 {
    syscall0(SYSCALL_GET_HEIGHT)
}