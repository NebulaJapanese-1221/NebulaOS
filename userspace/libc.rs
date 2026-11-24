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

use crate::syscall::{
    invoke,
    SYSCALL_EXIT, SYSCALL_PRINT, SYSCALL_GET_TICKS,
    SYSCALL_VFS_OPEN, SYSCALL_VFS_CLOSE, SYSCALL_VFS_READ, SYSCALL_VFS_WRITE,
    SYSCALL_VFS_LSEEK, SYSCALL_VFS_STAT, SYSCALL_VFS_FSTAT,
    SYSCALL_VFS_MKDIR, SYSCALL_VFS_UNLINK, SYSCALL_VFS_RMDIR,
    SYSCALL_VFS_RENAME, SYSCALL_VFS_READDIR, SYSCALL_VFS_SYNC,
    SYSCALL_VFS_STATFS, SYSCALL_VFS_MOUNT, SYSCALL_VFS_UNMOUNT,
    SYSCALL_VFS_READLINK, SYSCALL_VFS_SYMLINK, SYSCALL_VFS_CHMOD,
    SYSCALL_VFS_CHOWN, SYSCALL_VFS_UTIMES,
    SYSCALL_VFS_XATTR_SET, SYSCALL_VFS_XATTR_GET,
    SYSCALL_VFS_XATTR_LIST, SYSCALL_VFS_XATTR_REMOVE,
    VfsOpenRequest, VfsIoRequest, VfsLseekRequest, VfsStatRequest,
    VfsFstatRequest, VfsMkdirRequest, VfsUnlinkRequest, VfsRenameRequest,
    VfsReaddirRequest, VfsMountRequest, VfsUnmountRequest,
    VfsReadlinkRequest, VfsSymlinkRequest, VfsChmodRequest,
    VfsChownRequest, VfsUtimesRequest,
    VfsXattrSetRequest, VfsXattrGetRequest, VfsXattrListRequest,
    VfsXattrRemoveRequest,
    VfsFileAttr, VfsDirEntry, VfsStatFs,
    MAX_WRITE_LENGTH,
};
use core::mem::size_of;

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

#[cfg(feature = "user-app")]
pub fn errno() -> Errno {
    unsafe { core::arch::asm!("", out("eax") let e: u32); Errno::from(e) }
}

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

#[cfg(feature = "user-app")]
pub fn exit(code: i32) -> ! {
    invoke(SYSCALL_EXIT, code as u32, 0);
    loop {}
}

#[cfg(feature = "user-app")]
pub fn get_ticks() -> u32 {
    invoke(SYSCALL_GET_TICKS, 0, 0)
}