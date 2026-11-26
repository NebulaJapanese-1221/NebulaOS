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

//! Standard C library interface for NebulaOS userspace
//! Provides POSIX-like system call wrappers

use core::mem::size_of;

// ============================================================================
// Syscall numbers
// ============================================================================

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

// Audio syscalls
pub const SYSCALL_AUDIO_INIT: u32 = 66;
pub const SYSCALL_AUDIO_VOLUME_GET: u32 = 67;
pub const SYSCALL_AUDIO_VOLUME_SET: u32 = 68;
pub const SYSCALL_AUDIO_PLAY_TONE: u32 = 69;
pub const SYSCALL_AUDIO_STOP: u32 = 70;
pub const SYSCALL_AUDIO_BEEP: u32 = 71;
pub const SYSCALL_AUDIO_STARTUP_CHIME: u32 = 72;
pub const SYSCALL_AUDIO_PLAY_MELODY: u32 = 73;
pub const SYSCALL_AUDIO_QUEUE_TONE: u32 = 74;
pub const SYSCALL_AUDIO_POLL: u32 = 75;
pub const SYSCALL_AUDIO_AC97_AVAILABLE: u32 = 76;

// Multi-monitor syscalls
pub const SYSCALL_FB_GET_COUNT: u32 = 77;
pub const SYSCALL_FB_GET_PRIMARY: u32 = 78;
pub const SYSCALL_FB_GET_WIDTH: u32 = 79;
pub const SYSCALL_FB_GET_HEIGHT: u32 = 80;
pub const SYSCALL_FB_SET_CURSOR: u32 = 81;
pub const SYSCALL_FB_GET_CURSOR: u32 = 82;
pub const SYSCALL_FB_PRESENT: u32 = 83;
pub const SYSCALL_FB_PRESENT_RECT: u32 = 84;
pub const SYSCALL_FB_FILL_RECT: u32 = 85;
pub const SYSCALL_FB_DRAW_RECT: u32 = 86;
pub const SYSCALL_FB_DRAW_TEXT: u32 = 87;
pub const SYSCALL_FB_CLEAR: u32 = 88;

// Process management syscalls
pub const SYSCALL_FORK: u32 = 89;
pub const SYSCALL_EXECVE: u32 = 90;
pub const SYSCALL_WAITPID: u32 = 91;
pub const SYSCALL_GETPID: u32 = 92;
pub const SYSCALL_GETPPID: u32 = 93;
pub const SYSCALL_GETUID: u32 = 94;
pub const SYSCALL_GETGID: u32 = 95;
pub const SYSCALL_GETEUID: u32 = 96;
pub const SYSCALL_GETEGID: u32 = 97;
pub const SYSCALL_KILL: u32 = 98;

pub const CLIPBOARD_MAX: usize = 256;
pub const MAX_WRITE_LENGTH: usize = 1024;
pub const MAX_PROCESSES: usize = 8;

// ============================================================================
// Data structures
// ============================================================================

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
pub struct AudioToneRequest {
    pub frequency: u32,
    pub duration: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AudioMelodyRequest {
    pub note_count: u32,
    pub data_ptr: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FbCursorRequest {
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub kind: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FbRectRequest {
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FbFillRectRequest {
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub color: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FbDrawRectRequest {
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub color: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FbDrawTextRequest {
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub color: u32,
    pub scale: u32,
    pub text_ptr: u32,
    pub text_len: u32,
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

// Process management structures
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ForkRequest {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ExecveRequest {
    pub path_ptr: u32,
    pub path_len: u32,
    pub argv_ptr: u32,
    pub argc: u32,
    pub envp_ptr: u32,
    pub envc: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct WaitpidRequest {
    pub pid: i32,
    pub status_ptr: u32,
    pub options: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct KillRequest {
    pub pid: i32,
    pub sig: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GetPidRequest {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GetPpidRequest {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GetUidRequest {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GetGidRequest {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GetEuidRequest {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GetEgidRequest {}

// ============================================================================
// Raw syscall invocation
// ============================================================================

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

// ============================================================================
// Error handling
// ============================================================================

#[cfg(feature = "user-app")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Errno {
    Success = 0,
    Perm = 1,
    NoEnt = 2,
    Io = 5,
    NxIo = 6,
    TooBig = 7,
    NoMem = 12,
    Access = 13,
    Fault = 14,
    NotBlk = 15,
    Busy = 16,
    Exist = 17,
    XDev = 18,
    NotDir = 20,
    IsDir = 21,
    Inval = 22,
    NFile = 23,
    MFile = 24,
    NotTy = 25,
    TxtBsy = 26,
    FBig = 27,
    NoSpc = 28,
    SpiPe = 29,
    ROFS = 30,
    MLink = 31,
    Pipe = 32,
    Dom = 33,
    Range = 34,
    DeadLk = 35,
    NaMl = 36,
    NoSys = 38,
    NotSock = 88,
    OpNotSupp = 95,
}

#[cfg(feature = "user-app")]
impl From<u32> for Errno {
    fn from(v: u32) -> Self {
        match v {
            0 => Errno::Success,
            1 => Errno::Perm,
            2 => Errno::NoEnt,
            5 => Errno::Io,
            6 => Errno::NxIo,
            7 => Errno::TooBig,
            12 => Errno::NoMem,
            13 => Errno::Access,
            14 => Errno::Fault,
            15 => Errno::NotBlk,
            16 => Errno::Busy,
            17 => Errno::Exist,
            18 => Errno::XDev,
            20 => Errno::NotDir,
            21 => Errno::IsDir,
            22 => Errno::Inval,
            23 => Errno::NFile,
            24 => Errno::MFile,
            25 => Errno::NotTy,
            26 => Errno::TxtBsy,
            27 => Errno::FBig,
            28 => Errno::NoSpc,
            29 => Errno::SpiPe,
            30 => Errno::ROFS,
            31 => Errno::MLink,
            32 => Errno::Pipe,
            33 => Errno::Dom,
            34 => Errno::Range,
            35 => Errno::DeadLk,
            36 => Errno::NaMl,
            38 => Errno::NoSys,
            88 => Errno::NotSock,
            95 => Errno::OpNotSupp,
            _ => Errno::Io,
        }
    }
}

// ============================================================================
// POSIX-like wrappers
// ============================================================================

#[cfg(feature = "user-app")]
pub fn write(fd: i32, buf: &[u8]) -> Result<usize, Errno> {
    if buf.len() > MAX_WRITE_LENGTH {
        return Err(Errno::TooBig);
    }
    let result = invoke(SYSCALL_PRINT, buf.as_ptr() as u32, buf.len() as u32);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as usize)
    }
}

#[cfg(feature = "user-app")]
pub fn read(fd: i32, buf: &mut [u8]) -> Result<usize, Errno> {
    let mut req = VfsIoRequest {
        fd: fd as u32,
        buf_ptr: buf.as_mut_ptr() as u32,
        buf_len: buf.len() as u32,
        offset: 0,
    };
    let result = invoke(SYSCALL_VFS_READ, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as usize)
    }
}

#[cfg(feature = "user-app")]
pub fn open(path: &str, flags: i32, mode: u32) -> Result<i32, Errno> {
    let mut req = VfsOpenRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        flags: flags as u32,
        mode,
    };
    let result = invoke(SYSCALL_VFS_OPEN, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as i32)
    }
}

#[cfg(feature = "user-app")]
pub fn close(fd: i32) -> Result<(), Errno> {
    let result = invoke(SYSCALL_VFS_CLOSE, fd as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn lseek(fd: i32, offset: i64, whence: i32) -> Result<i64, Errno> {
    let mut req = VfsLseekRequest {
        fd: fd as u32,
        offset,
        whence: whence as u32,
    };
    let result = invoke(SYSCALL_VFS_LSEEK, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as i64)
    }
}

#[cfg(feature = "user-app")]
pub fn stat(path: &str, stat: &mut VfsFileAttr) -> Result<(), Errno> {
    let mut req = VfsStatRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_STAT, &mut req as *mut _ as u32, stat as *mut _ as u32);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn fstat(fd: i32, stat: &mut VfsFileAttr) -> Result<(), Errno> {
    let mut req = VfsFstatRequest { fd: fd as u32 };
    let result = invoke(SYSCALL_VFS_FSTAT, &mut req as *mut _ as u32, stat as *mut _ as u32);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn mkdir(path: &str, mode: u32) -> Result<(), Errno> {
    let mut req = VfsMkdirRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        mode,
    };
    let result = invoke(SYSCALL_VFS_MKDIR, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn unlink(path: &str) -> Result<(), Errno> {
    let mut req = VfsUnlinkRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_UNLINK, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn rmdir(path: &str) -> Result<(), Errno> {
    let mut req = VfsUnlinkRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_RMDIR, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn rename(old_path: &str, new_path: &str) -> Result<(), Errno> {
    let mut req = VfsRenameRequest {
        old_path_ptr: old_path.as_ptr() as u32,
        old_path_len: old_path.len() as u32,
        new_path_ptr: new_path.as_ptr() as u32,
        new_path_len: new_path.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_RENAME, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn readdir(fd: i32, entries: &mut [VfsDirEntry]) -> Result<usize, Errno> {
    let mut req = VfsReaddirRequest {
        fd: fd as u32,
        offset: 0,
        buf_ptr: entries.as_mut_ptr() as u32,
        buf_len: (entries.len() * size_of::<VfsDirEntry>()) as u32,
    };
    let result = invoke(SYSCALL_VFS_READDIR, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as usize)
    }
}

#[cfg(feature = "user-app")]
pub fn sync() -> Result<(), Errno> {
    let result = invoke(SYSCALL_VFS_SYNC, 0, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn statfs(path: &str, stat: &mut VfsStatFs) -> Result<(), Errno> {
    let mut req = VfsStatRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_STATFS, &mut req as *mut _ as u32, stat as *mut _ as u32);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn mount(path: &str, fs_type: &str, options: &str) -> Result<(), Errno> {
    let mut req = VfsMountRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        fs_type_ptr: fs_type.as_ptr() as u32,
        fs_type_len: fs_type.len() as u32,
        options_ptr: options.as_ptr() as u32,
        options_len: options.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_MOUNT, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn umount(path: &str) -> Result<(), Errno> {
    let mut req = VfsUnmountRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_UNMOUNT, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn readlink(path: &str, buf: &mut [u8]) -> Result<usize, Errno> {
    let mut req = VfsReadlinkRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        buf_ptr: buf.as_mut_ptr() as u32,
        buf_len: buf.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_READLINK, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as usize)
    }
}

#[cfg(feature = "user-app")]
pub fn symlink(target: &str, linkpath: &str) -> Result<(), Errno> {
    let mut req = VfsSymlinkRequest {
        target_ptr: target.as_ptr() as u32,
        target_len: target.len() as u32,
        linkpath_ptr: linkpath.as_ptr() as u32,
        linkpath_len: linkpath.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_SYMLINK, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn chmod(path: &str, mode: u32) -> Result<(), Errno> {
    let mut req = VfsChmodRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        mode,
    };
    let result = invoke(SYSCALL_VFS_CHMOD, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn chown(path: &str, uid: u32, gid: u32) -> Result<(), Errno> {
    let mut req = VfsChownRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        uid,
        gid,
    };
    let result = invoke(SYSCALL_VFS_CHOWN, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn utimes(path: &str, atime: u64, mtime: u64) -> Result<(), Errno> {
    let mut req = VfsUtimesRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        atime,
        mtime,
    };
    let result = invoke(SYSCALL_VFS_UTIMES, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn xattr_set(path: &str, name: &str, value: &[u8], flags: u32) -> Result<(), Errno> {
    let mut req = VfsXattrSetRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        value_ptr: value.as_ptr() as u32,
        value_len: value.len() as u32,
        flags,
    };
    let result = invoke(SYSCALL_VFS_XATTR_SET, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn xattr_get(path: &str, name: &str, buf: &mut [u8]) -> Result<usize, Errno> {
    let mut req = VfsXattrGetRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        value_ptr: buf.as_mut_ptr() as u32,
        value_len: buf.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_XATTR_GET, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as usize)
    }
}

#[cfg(feature = "user-app")]
pub fn xattr_list(path: &str, buf: &mut [u8]) -> Result<usize, Errno> {
    let mut req = VfsXattrListRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        list_ptr: buf.as_mut_ptr() as u32,
        list_len: buf.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_XATTR_LIST, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as usize)
    }
}

#[cfg(feature = "user-app")]
pub fn xattr_remove(path: &str, name: &str) -> Result<(), Errno> {
    let mut req = VfsXattrRemoveRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
    };
    let result = invoke(SYSCALL_VFS_XATTR_REMOVE, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

// ============================================================================
// Audio wrappers
// ============================================================================

#[cfg(feature = "user-app")]
pub fn audio_init() -> Result<(), Errno> {
    let result = invoke(SYSCALL_AUDIO_INIT, 0, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_volume_get() -> Result<u8, Errno> {
    let result = invoke(SYSCALL_AUDIO_VOLUME_GET, 0, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as u8)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_volume_set(level: u8) -> Result<(), Errno> {
    let result = invoke(SYSCALL_AUDIO_VOLUME_SET, level as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_play_tone(frequency_hz: u32, duration_ms: u32) -> Result<(), Errno> {
    let mut req = AudioToneRequest {
        frequency: frequency_hz,
        duration: duration_ms,
    };
    let result = invoke(SYSCALL_AUDIO_PLAY_TONE, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_stop() -> Result<(), Errno> {
    let result = invoke(SYSCALL_AUDIO_STOP, 0, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_beep() -> Result<(), Errno> {
    let result = invoke(SYSCALL_AUDIO_BEEP, 0, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_startup_chime() -> Result<(), Errno> {
    let result = invoke(SYSCALL_AUDIO_STARTUP_CHIME, 0, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_play_melody(notes: &[(u32, u32)]) -> Result<(), Errno> {
    // Pack notes into a buffer (each note = 8 bytes: freq + duration)
    let note_count = notes.len();
    if note_count > 32 {
        return Err(Errno::Io);
    }
    let mut buffer = [0u8; 256];
    for (i, &(freq, dur)) in notes.iter().enumerate() {
        let base = i * 8;
        if base + 8 > 256 {
            break;
        }
        buffer[base..base+4].copy_from_slice(&freq.to_le_bytes());
        buffer[base+4..base+8].copy_from_slice(&dur.to_le_bytes());
    }
    let mut req = AudioMelodyRequest {
        note_count: note_count as u32,
        data_ptr: buffer.as_ptr() as u32,
    };
    let result = invoke(SYSCALL_AUDIO_PLAY_MELODY, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_queue_tone(frequency_hz: u32, duration_ms: u32) -> Result<(), Errno> {
    let mut req = AudioToneRequest {
        frequency: frequency_hz,
        duration: duration_ms,
    };
    let result = invoke(SYSCALL_AUDIO_QUEUE_TONE, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_poll() -> Result<bool, Errno> {
    let result = invoke(SYSCALL_AUDIO_POLL, 0, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result != 0)
    }
}

#[cfg(feature = "user-app")]
pub fn audio_ac97_available() -> Result<bool, Errno> {
    let result = invoke(SYSCALL_AUDIO_AC97_AVAILABLE, 0, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result != 0)
    }
}

// ============================================================================
// Multi-monitor wrappers
// ============================================================================

#[cfg(feature = "user-app")]
pub fn fb_get_count() -> Result<u32, Errno> {
    let result = invoke(SYSCALL_FB_GET_COUNT, 0, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_get_primary() -> Result<u32, Errno> {
    let result = invoke(SYSCALL_FB_GET_PRIMARY, 0, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_get_width(fb_id: u32) -> Result<u32, Errno> {
    let result = invoke(SYSCALL_FB_GET_WIDTH, fb_id, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_get_height(fb_id: u32) -> Result<u32, Errno> {
    let result = invoke(SYSCALL_FB_GET_HEIGHT, fb_id, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_set_cursor(fb_id: u32, x: u32, y: u32, kind: u32) -> Result<(), Errno> {
    let mut req = FbCursorRequest {
        fb_id,
        x,
        y,
        kind,
    };
    let result = invoke(SYSCALL_FB_SET_CURSOR, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_get_cursor(fb_id: u32) -> Result<(u32, u32, u32), Errno> {
    let mut req = FbCursorRequest { fb_id, x: 0, y: 0, kind: 0 };
    let result = invoke(SYSCALL_FB_GET_CURSOR, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok((req.x, req.y, req.kind))
    }
}

#[cfg(feature = "user-app")]
pub fn fb_present(fb_id: u32) -> Result<(), Errno> {
    let result = invoke(SYSCALL_FB_PRESENT, fb_id, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_present_rect(fb_id: u32, x: u32, y: u32, width: u32, height: u32) -> Result<(), Errno> {
    let mut req = FbRectRequest {
        fb_id,
        x,
        y,
        width,
        height,
    };
    let result = invoke(SYSCALL_FB_PRESENT_RECT, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_fill_rect(fb_id: u32, x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<(), Errno> {
    let mut req = FbFillRectRequest {
        fb_id,
        x,
        y,
        width,
        height,
        color,
    };
    let result = invoke(SYSCALL_FB_FILL_RECT, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_draw_rect(fb_id: u32, x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<(), Errno> {
    let mut req = FbDrawRectRequest {
        fb_id,
        x,
        y,
        width,
        height,
        color,
    };
    let result = invoke(SYSCALL_FB_DRAW_RECT, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_draw_text(fb_id: u32, x: u32, y: u32, text: &str, color: u32, scale: u32) -> Result<(), Errno> {
    let text_len = text.len().min(1024);
    let mut req = FbDrawTextRequest {
        fb_id,
        x,
        y,
        width: text_len as u32 * 6 * scale,
        height: 8 * scale,
        color,
        scale,
        text_ptr: text.as_ptr() as u32,
        text_len: text_len as u32,
    };
    let result = invoke(SYSCALL_FB_DRAW_TEXT, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

#[cfg(feature = "user-app")]
pub fn fb_clear(fb_id: u32, color: u32) -> Result<(), Errno> {
    let result = invoke(SYSCALL_FB_CLEAR, fb_id, color);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::Io)
    }
}

// ============================================================================
// Process management wrappers
// ============================================================================

#[cfg(feature = "user-app")]
pub fn fork() -> Result<u32, Errno> {
    let req = ForkRequest::default();
    let result = invoke(SYSCALL_FORK, &req as *const _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result)
    }
}

#[cfg(feature = "user-app")]
pub fn execve(path: &str, argv: &[*const u8], envp: &[*const u8]) -> Result<(), Errno> {
    let mut req = ExecveRequest {
        path_ptr: path.as_ptr() as u32,
        path_len: path.len() as u32,
        argv_ptr: argv.as_ptr() as u32,
        argc: argv.len() as u32,
        envp_ptr: envp.as_ptr() as u32,
        envc: envp.len() as u32,
    };
    let result = invoke(SYSCALL_EXECVE, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn waitpid(pid: i32, status: &mut i32, options: i32) -> Result<i32, Errno> {
    let mut req = WaitpidRequest {
        pid,
        status_ptr: status as *mut i32 as u32,
        options: options as u32,
    };
    let result = invoke(SYSCALL_WAITPID, &mut req as *mut _ as u32, 0);
    if result == u32::MAX {
        Err(Errno::Io)
    } else {
        Ok(result as i32)
    }
}

#[cfg(feature = "user-app")]
pub fn getpid() -> u32 {
    let req = GetPidRequest::default();
    invoke(SYSCALL_GETPID, &req as *const _ as u32, 0)
}

#[cfg(feature = "user-app")]
pub fn getppid() -> u32 {
    let req = GetPpidRequest::default();
    invoke(SYSCALL_GETPPID, &req as *const _ as u32, 0)
}

#[cfg(feature = "user-app")]
pub fn getuid() -> u32 {
    let req = GetUidRequest::default();
    invoke(SYSCALL_GETUID, &req as *const _ as u32, 0)
}

#[cfg(feature = "user-app")]
pub fn getgid() -> u32 {
    let req = GetGidRequest::default();
    invoke(SYSCALL_GETGID, &req as *const _ as u32, 0)
}

#[cfg(feature = "user-app")]
pub fn geteuid() -> u32 {
    let req = GetEuidRequest::default();
    invoke(SYSCALL_GETEUID, &req as *const _ as u32, 0)
}

#[cfg(feature = "user-app")]
pub fn getegid() -> u32 {
    let req = GetEgidRequest::default();
    invoke(SYSCALL_GETEGID, &req as *const _ as u32, 0)
}

#[cfg(feature = "user-app")]
pub fn kill(pid: i32, sig: u32) -> Result<(), Errno> {
    let mut req = KillRequest { pid, sig };
    let result = invoke(SYSCALL_KILL, &mut req as *mut _ as u32, 0);
    if result == 0 {
        Ok(())
    } else {
        Err(Errno::from(result))
    }
}

#[cfg(feature = "user-app")]
pub fn exit(code: i32) -> ! {
    invoke(SYSCALL_EXIT, code as u32, 0);
    loop {}
}

#[cfg(feature = "user-app")]
pub fn get_ticks() -> u32 {
    invoke(SYSCALL_GET_TICKS, 0, 0)
}

// ============================================================================
// SyscallError for backward compatibility
// ============================================================================

#[cfg(feature = "user-app")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyscallError {
    MessageTooLong,
    Unavailable,
    InvalidArgument,
}