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
// MERCHANTABILITY OR FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

//! Standalone userspace library - built as a staticlib/cdylib and embedded into the kernel.
//! This crate contains all userspace code (libc, apps, desktop, storage) directly.

#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_experimental_arch)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(lang_items)]
#![feature(panic_info_message)]
#![feature(linkage)]

extern crate alloc;

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[global_allocator]
static ALLOCATOR: crate::memory::allocator::GlobalAllocator = crate::memory::allocator::GlobalAllocator;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

// Include all userspace modules directly
pub mod memory {
    pub mod allocator {
        use core::alloc::{GlobalAlloc, Layout};
        use core::ptr::null_mut;

        pub struct GlobalAllocator;

        unsafe impl GlobalAlloc for GlobalAllocator {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                let size = layout.size().max(layout.align());
                if size == 0 {
                    return layout.align() as *mut u8;
                }
                // Simple bump allocator for userspace
                static mut HEAP: [u8; 1024 * 1024] = [0; 1024 * 1024];
                static mut HEAP_PTR: usize = 0;
                let align = layout.align();
                let ptr = (HEAP_PTR + align - 1) & !(align - 1);
                if ptr + size > HEAP.len() {
                    return null_mut();
                }
                HEAP_PTR = ptr + size;
                HEAP.as_mut_ptr().add(ptr)
            }
            unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
        }
    }
}

// Syscall definitions
pub mod syscall {
    pub const SYSCALL_EXIT: u32 = 0;
    pub const SYSCALL_PRINT: u32 = 1;
    pub const SYSCALL_GET_TICKS: u32 = 2;
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
    pub const SYSCALL_FORK: u32 = 66;
    pub const SYSCALL_EXECVE: u32 = 67;
    pub const SYSCALL_WAITPID: u32 = 68;
    pub const SYSCALL_GETPID: u32 = 69;
    pub const SYSCALL_GETPPID: u32 = 70;
    pub const SYSCALL_GETUID: u32 = 71;
    pub const SYSCALL_GETGID: u32 = 72;
    pub const SYSCALL_GETEUID: u32 = 73;
    pub const SYSCALL_GETEGID: u32 = 74;
    pub const SYSCALL_KILL: u32 = 75;

    #[inline]
    pub fn invoke(number: u32, first: u32, second: u32) -> u32 {
        let mut result = number;
        unsafe {
            core::arch::asm!(
                "int 0x80",
                inout("eax") result,
                in("ebx") first,
                in("ecx") second,
            );
        }
        result
    }
}

// Libc wrappers
pub mod libc {
    use crate::syscall::{invoke, SYSCALL_EXIT, SYSCALL_PRINT, SYSCALL_GET_TICKS, SYSCALL_VFS_OPEN, SYSCALL_VFS_CLOSE, SYSCALL_VFS_READ, SYSCALL_VFS_WRITE, SYSCALL_VFS_LSEEK, SYSCALL_VFS_STAT, SYSCALL_VFS_FSTAT, SYSCALL_VFS_MKDIR, SYSCALL_VFS_UNLINK, SYSCALL_VFS_RMDIR, SYSCALL_VFS_RENAME, SYSCALL_VFS_READDIR, SYSCALL_VFS_SYNC, SYSCALL_FORK, SYSCALL_EXECVE, SYSCALL_WAITPID, SYSCALL_GETPID, SYSCALL_GETPPID, SYSCALL_GETUID, SYSCALL_GETGID, SYSCALL_GETEUID, SYSCALL_GETEGID, SYSCALL_KILL};
    use core::mem::size_of;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Errno {
        Success = 0,
        Perm = 1,
        NoEnt = 2,
        Io = 5,
        TooBig = 7,
        NoMem = 12,
        Access = 13,
        Fault = 14,
        NotDir = 20,
        IsDir = 21,
        Inval = 22,
        NFile = 23,
        MFile = 24,
        NoSys = 38,
    }

    impl From<u32> for Errno {
        fn from(v: u32) -> Self {
            match v {
                0 => Errno::Success,
                1 => Errno::Perm,
                2 => Errno::NoEnt,
                5 => Errno::Io,
                7 => Errno::TooBig,
                12 => Errno::NoMem,
                13 => Errno::Access,
                14 => Errno::Fault,
                20 => Errno::NotDir,
                21 => Errno::IsDir,
                22 => Errno::Inval,
                23 => Errno::NFile,
                24 => Errno::MFile,
                38 => Errno::NoSys,
                _ => Errno::Io,
            }
        }
    }

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

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct KillRequest {
        pub pid: i32,
        pub sig: u32,
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

    pub fn close(fd: i32) -> Result<(), Errno> {
        let result = invoke(SYSCALL_VFS_CLOSE, fd as u32, 0);
        if result == 0 {
            Ok(())
        } else {
            Err(Errno::from(result))
        }
    }

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

    pub fn write(fd: i32, buf: &[u8]) -> Result<usize, Errno> {
        let mut req = VfsIoRequest {
            fd: fd as u32,
            buf_ptr: buf.as_ptr() as u32,
            buf_len: buf.len() as u32,
            offset: 0,
        };
        let result = invoke(SYSCALL_VFS_WRITE, &mut req as *mut _ as u32, 0);
        if result == u32::MAX {
            Err(Errno::Io)
        } else {
            Ok(result as usize)
        }
    }

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

    pub fn fstat(fd: i32, stat: &mut VfsFileAttr) -> Result<(), Errno> {
        let mut req = VfsFstatRequest { fd: fd as u32 };
        let result = invoke(SYSCALL_VFS_FSTAT, &mut req as *mut _ as u32, stat as *mut _ as u32);
        if result == 0 {
            Ok(())
        } else {
            Err(Errno::from(result))
        }
    }

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

    pub fn sync() -> Result<(), Errno> {
        let result = invoke(SYSCALL_VFS_SYNC, 0, 0);
        if result == 0 {
            Ok(())
        } else {
            Err(Errno::from(result))
        }
    }

    pub fn fork() -> Result<u32, Errno> {
        let req = ForkRequest::default();
        let result = invoke(SYSCALL_FORK, &req as *const _ as u32, 0);
        if result == u32::MAX {
            Err(Errno::Io)
        } else {
            Ok(result)
        }
    }

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

    pub fn getpid() -> u32 {
        let req = GetPidRequest::default();
        invoke(SYSCALL_GETPID, &req as *const _ as u32, 0)
    }

    pub fn getppid() -> u32 {
        let req = GetPpidRequest::default();
        invoke(SYSCALL_GETPPID, &req as *const _ as u32, 0)
    }

    pub fn getuid() -> u32 {
        let req = GetUidRequest::default();
        invoke(SYSCALL_GETUID, &req as *const _ as u32, 0)
    }

    pub fn getgid() -> u32 {
        let req = GetGidRequest::default();
        invoke(SYSCALL_GETGID, &req as *const _ as u32, 0)
    }

    pub fn geteuid() -> u32 {
        let req = GetEuidRequest::default();
        invoke(SYSCALL_GETEUID, &req as *const _ as u32, 0)
    }

    pub fn getegid() -> u32 {
        let req = GetEgidRequest::default();
        invoke(SYSCALL_GETEGID, &req as *const _ as u32, 0)
    }

    pub fn kill(pid: i32, sig: u32) -> Result<(), Errno> {
        let mut req = KillRequest { pid, sig };
        let result = invoke(SYSCALL_KILL, &mut req as *mut _ as u32, 0);
        if result == 0 {
            Ok(())
        } else {
            Err(Errno::from(result))
        }
    }

    pub fn exit(code: i32) -> ! {
        invoke(SYSCALL_EXIT, code as u32, 0);
        loop {}
    }

    pub fn get_ticks() -> u32 {
        invoke(SYSCALL_GET_TICKS, 0, 0)
    }
}

// Apps module (simplified - just placeholder)
pub mod apps {
    pub mod gui {
        use crate::libc;
        pub struct Application;
        pub struct Canvas;
        pub struct Event;
        pub struct PointerEvent;
        pub struct Rect;
        pub struct Button;
        pub struct TextField;
        pub struct Menu;
        pub struct Checkbox;
    }
}

// Desktop module (simplified)
pub mod desktop {
    use crate::libc;
}

// Storage module (simplified)
pub mod storage {
    use crate::libc;
    pub fn list_files() -> Result<(), libc::Errno> { Ok(()) }
    pub fn read_file(_name: &[u8], _output: &mut [u8]) -> Result<usize, libc::Errno> { Ok(0) }
    pub fn write_file(_name: &[u8], _content: &[u8]) -> Result<(), libc::Errno> { Ok(()) }
}