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

//! Raw system-call ABI definitions for Ring-3 processes.
//! Use libc module for POSIX-like wrappers.

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

// VFS syscalls
pub const SYSCALL_VFS_OPEN: u32 = 41;
pub const SYSCALL_VFS_CLOSE: u32 = 42;
pub const SYSCALL_VFS_READ: u32 = 43;
pub const SYSCALL_VFS_WRITE: u32 = 44;
pub const SYSCALL_VFS_LSEEK: u32 = 45;
pub const SYSCALL_VFS_STAT: u32 = 46;
pub const SYSCALL_VFS_FSTAT: u32 = 47;
pub const SYSCALL_VFS_MKDIR: u32 = 48;
pub const SYSCALL_VFS_UNLINK: u32 = 49;
pub const SYSCALL_VFS_RMDIR: u32 = 50;
pub const SYSCALL_VFS_RENAME: u32 = 51;
pub const SYSCALL_VFS_READDIR: u32 = 52;
pub const SYSCALL_VFS_SYNC: u32 = 53;
pub const SYSCALL_VFS_STATFS: u32 = 54;
pub const SYSCALL_VFS_MOUNT: u32 = 55;
pub const SYSCALL_VFS_UNMOUNT: u32 = 56;
pub const SYSCALL_VFS_READLINK: u32 = 57;
pub const SYSCALL_VFS_SYMLINK: u32 = 58;
pub const SYSCALL_VFS_CHMOD: u32 = 59;
pub const SYSCALL_VFS_CHOWN: u32 = 60;
pub const SYSCALL_VFS_UTIMES: u32 = 61;
pub const SYSCALL_VFS_XATTR_SET: u32 = 62;
pub const SYSCALL_VFS_XATTR_GET: u32 = 63;
pub const SYSCALL_VFS_XATTR_LIST: u32 = 64;
pub const SYSCALL_VFS_XATTR_REMOVE: u32 = 65;

pub const CLIPBOARD_MAX: usize = 256;
pub const MAX_WRITE_LENGTH: usize = 1024;
pub const MAX_PROCESSES: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ProcessEntry {
    pub pid: u32,
    pub app_id: u32,
    pub state: u32,
    pub pending_events: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
pub struct AppRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
pub struct AppEvent {
    pub kind: u32,
    pub key: u32,
    pub x: u32,
    pub y: u32,
    pub left_down: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
pub struct StorageFileInfo {
    pub name: [u8; 16],
    pub length: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SystemStats {
    pub ticks: u32,
    pub free_frames: u32,
    pub total_frames: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ProcessInfo {
    pub current_pid: u32,
    pub count: u32,
    pub entries: [ProcessEntry; MAX_PROCESSES],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MemoryInfo {
    pub total_frames: u32,
    pub free_frames: u32,
    pub allocated_frames: u32,
    pub frame_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
pub struct SpawnAppRequest {
    pub app_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
pub struct CursorInfo {
    pub x: u32,
    pub y: u32,
    pub cursor_type: u32,
}

// VFS structures
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsOpenRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub flags: u32,
    pub mode: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsIoRequest {
    pub fd: u32,
    pub buf_ptr: u32,
    pub buf_len: u32,
    pub offset: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsLseekRequest {
    pub fd: u32,
    pub offset: i64,
    pub whence: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsStatRequest {
    pub path_ptr: u32,
    pub path_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsFstatRequest {
    pub fd: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsMkdirRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub mode: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsUnlinkRequest {
    pub path_ptr: u32,
    pub path_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsRenameRequest {
    pub old_path_ptr: u32,
    pub old_path_len: u32,
    pub new_path_ptr: u32,
    pub new_path_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsReaddirRequest {
    pub fd: u32,
    pub offset: u64,
    pub buf_ptr: u32,
    pub buf_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsMountRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub fs_type_ptr: u32,
    pub fs_type_len: u32,
    pub options_ptr: u32,
    pub options_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsUnmountRequest {
    pub path_ptr: u32,
    pub path_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsReadlinkRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub buf_ptr: u32,
    pub buf_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsSymlinkRequest {
    pub target_ptr: u32,
    pub target_len: u32,
    pub linkpath_ptr: u32,
    pub linkpath_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsChmodRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub mode: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsChownRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub uid: u32,
    pub gid: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsUtimesRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub atime: u64,
    pub mtime: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsXattrSetRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub name_ptr: u32,
    pub name_len: u32,
    pub value_ptr: u32,
    pub value_len: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsXattrGetRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub name_ptr: u32,
    pub name_len: u32,
    pub value_ptr: u32,
    pub value_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsXattrListRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub list_ptr: u32,
    pub list_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsXattrRemoveRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub name_ptr: u32,
    pub name_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsFileAttr {
    pub file_type: u32,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VfsDirEntry {
    pub name: [u8; 64],
    pub inode_id: u64,
    pub file_type: u32,
}

impl Default for VfsDirEntry {
    fn default() -> Self {
        Self {
            name: [0; 64],
            inode_id: 0,
            file_type: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VfsStatFs {
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    pub bsize: u32,
    pub namelen: u32,
}

#[cfg(feature = "user-app")]
#[inline]
pub fn invoke(number: u32, first_argument: u32, second_argument: u32) -> u32 {
    let mut result = number;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inout("eax") result,
            in("ebx") first_argument,
            in("ecx") second_argument,
        );
    }
    result
}

#[cfg(feature = "user-app")]
pub fn call_struct<T>(number: u32, value: &mut T) -> Result<(), SyscallError> {
    if invoke(number, value as *mut T as u32, 0) == 0 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
pub fn uptime_ticks() -> Result<u32, SyscallError> {
    let ticks = invoke(SYSCALL_GET_TICKS, 0, 0);
    if ticks == u32::MAX {
        Err(SyscallError::Unavailable)
    } else {
        Ok(ticks)
    }
}

#[cfg(feature = "user-app")]
pub fn system_stats(stats: &mut SystemStats) -> Result<(), SyscallError> {
    call_struct(SYSCALL_SYSTEM_STATS, stats)
}

#[cfg(feature = "user-app")]
pub fn pci_count() -> Result<u32, SyscallError> {
    let result = invoke(SYSCALL_PCI_COUNT, 0, 0);
    if result == u32::MAX {
        Err(SyscallError::Unavailable)
    } else {
        Ok(result)
    }
}

#[cfg(feature = "user-app")]
pub fn pci_device(index: u32, info: &mut PciDeviceInfo) -> Result<(), SyscallError> {
    if invoke(SYSCALL_PCI_DEVICE, index, info as *mut PciDeviceInfo as u32) == 0 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
pub fn set_mouse_sensitivity(sensitivity: u32) -> Result<(), SyscallError> {
    if !matches!(sensitivity, 1..=4) {
        return Err(SyscallError::InvalidArgument);
    }
    if invoke(SYSCALL_SET_MOUSE_SENSITIVITY, sensitivity, 0) == 0 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
pub fn set_dark_theme(enabled: bool) -> Result<(), SyscallError> {
    if invoke(SYSCALL_SET_WALLPAPER_THEME, u32::from(enabled), 0) == 0 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
pub fn storage(request: &mut StorageRequest) -> u32 {
    let number = match request.op {
        0 => SYSCALL_STORAGE_LIST,
        1 => SYSCALL_STORAGE_READ,
        2 => SYSCALL_STORAGE_WRITE,
        3 => SYSCALL_STORAGE_DELETE,
        4 => SYSCALL_STORAGE_RENAME,
        5 => SYSCALL_STORAGE_FORMAT,
        _ => return u32::MAX,
    };
    invoke(number, request as *mut StorageRequest as u32, 0)
}

#[cfg(feature = "user-app")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyscallError {
    MessageTooLong,
    Unavailable,
    InvalidArgument,
}