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

// Virtual File System core layer

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::boxed::Box;
use alloc::sync::Arc;
use crate::sync::SpinLock;

pub const MAX_PATH: usize = 256;
pub const MAX_FILENAME: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    AlreadyExists,
    NotDirectory,
    IsDirectory,
    NoSpace,
    NoMemory,
    OutOfMemory,
    InvalidName,
    NotEmpty,
    PermissionDenied,
    IoError,
    NotSupported,
    NotMounted,
    TooManyOpenFiles,
    BadFileDescriptor,
    DirectoryFull,
}

impl From<VfsError> for crate::drivers::disk::StorageError {
    fn from(err: VfsError) -> Self {
        match err {
            VfsError::NotFound => crate::drivers::disk::StorageError::NotFound,
            VfsError::AlreadyExists => crate::drivers::disk::StorageError::InvalidName,
            VfsError::NoSpace => crate::drivers::disk::StorageError::FileTooLarge,
            VfsError::NoMemory => crate::drivers::disk::StorageError::OutOfMemory,
            VfsError::OutOfMemory => crate::drivers::disk::StorageError::OutOfMemory,
            VfsError::InvalidName => crate::drivers::disk::StorageError::InvalidName,
            VfsError::NotEmpty => crate::drivers::disk::StorageError::DirectoryFull,
            VfsError::DirectoryFull => crate::drivers::disk::StorageError::DirectoryFull,
            VfsError::IoError => crate::drivers::disk::StorageError::Io,
            _ => crate::drivers::disk::StorageError::Io,
        }
    }
}

impl From<crate::drivers::disk::StorageError> for VfsError {
    fn from(err: crate::drivers::disk::StorageError) -> Self {
        match err {
            crate::drivers::disk::StorageError::NoDisk => VfsError::IoError,
            crate::drivers::disk::StorageError::NotReady => VfsError::IoError,
            crate::drivers::disk::StorageError::ReadError => VfsError::IoError,
            crate::drivers::disk::StorageError::WriteError => VfsError::IoError,
            crate::drivers::disk::StorageError::NotFormatted => VfsError::NotMounted,
            crate::drivers::disk::StorageError::NotFound => VfsError::NotFound,
            crate::drivers::disk::StorageError::InvalidName => VfsError::InvalidName,
            crate::drivers::disk::StorageError::NotBlank => VfsError::AlreadyExists,
            crate::drivers::disk::StorageError::DirectoryFull => VfsError::DirectoryFull,
            crate::drivers::disk::StorageError::FileTooLarge => VfsError::NoSpace,
            crate::drivers::disk::StorageError::Io => VfsError::IoError,
            crate::drivers::disk::StorageError::OutOfMemory => VfsError::OutOfMemory,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    BlockDevice,
    CharDevice,
    Fifo,
    Socket,
}

#[derive(Debug, Clone, Copy)]
pub struct FileAttr {
    pub file_type: FileType,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

impl Default for FileAttr {
    fn default() -> Self {
        Self {
            file_type: FileType::Regular,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            mode: 0o644,
            uid: 0,
            gid: 0,
        }
    }
}

pub trait FileSystem: Send + Sync {
    fn name(&self) -> &'static str;
    fn mount(&mut self, device: u32, options: &str) -> Result<(), VfsError>;
    fn unmount(&mut self) -> Result<(), VfsError>;
    fn root_inode(&self) -> Arc<dyn Inode>;
    fn statfs(&self) -> Result<StatFs, VfsError>;
    fn sync(&mut self) -> Result<(), VfsError>;
}

pub trait Inode: Send + Sync {
    fn inode_id(&self) -> u64;
    fn inode_num(&self) -> u64 { self.inode_id() }
    fn attr(&self) -> Result<FileAttr, VfsError>;
    fn set_attr(&self, attr: &FileAttr) -> Result<(), VfsError>;
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError>;
    fn create(&self, name: &str, file_type: FileType, mode: u32) -> Result<Arc<dyn Inode>, VfsError>;
    fn unlink(&self, name: &str) -> Result<(), VfsError>;
    fn mkdir(&self, name: &str, mode: u32) -> Result<Arc<dyn Inode>, VfsError>;
    fn rmdir(&self, name: &str) -> Result<(), VfsError>;
    fn rename(&self, old_name: &str, target_dir: &dyn Inode, new_name: &str) -> Result<(), VfsError>;
    fn read_dir(&self, offset: u64, entries: &mut Vec<DirEntry>) -> Result<u64, VfsError>;
    fn open_file(&self, flags: u32) -> Result<Arc<dyn File>, VfsError>;
    fn readlink(&self) -> Result<String, VfsError>;
    fn symlink(&self, target: &str, name: &str) -> Result<(), VfsError>;
}

pub trait File: Send + Sync {
    fn read(&self, buf: &mut [u8], offset: u64) -> Result<usize, VfsError>;
    fn write(&self, buf: &[u8], offset: u64) -> Result<usize, VfsError>;
    fn flush(&self) -> Result<(), VfsError>;
    fn fsync(&self) -> Result<(), VfsError>;
    fn ioctl(&self, request: u32, arg: u64) -> Result<u64, VfsError>;
    fn get_inode(&self) -> Arc<dyn Inode>;
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub inode_id: u64,
    pub file_type: FileType,
}

#[derive(Debug, Clone, Copy)]
pub struct StatFs {
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    pub bsize: u32,
    pub namelen: u32,
}

struct MountPoint {
    path: String,
    fs: Box<dyn FileSystem>,
    root_inode: Arc<dyn Inode>,
}

static MOUNTS: SpinLock<Vec<MountPoint>> = SpinLock::new(Vec::new());
static NEXT_FD: SpinLock<u32> = SpinLock::new(3);
static FILE_TABLE: SpinLock<Vec<Option<FileDesc>>> = SpinLock::new(Vec::new());

#[derive(Clone)]
struct FileDesc {
    file: Arc<dyn File>,
    inode: Arc<dyn Inode>,
    flags: u32,
    offset: u64,
    ref_count: u32,
    path: String,
}

pub fn init() {
    let mut mounts = MOUNTS.lock();
    mounts.clear();
    let mut file_table = FILE_TABLE.lock();
    file_table.clear();
    file_table.resize(3, None);
    file_table[0] = Some(FileDesc::stdin());
    file_table[1] = Some(FileDesc::stdout());
    file_table[2] = Some(FileDesc::stderr());
}

pub fn mount(path: &str, fs: Box<dyn FileSystem>, _options: &str) -> Result<(), VfsError> {
    let path = normalize_path(path);
    let mut mounts = MOUNTS.lock();
    
    for m in mounts.iter() {
        if m.path == path {
            return Err(VfsError::AlreadyExists);
        }
    }
    
    let root_inode = fs.root_inode();
    mounts.push(MountPoint {
        path: path.clone(),
        fs,
        root_inode,
    });
    Ok(())
}

pub fn unmount(path: &str) -> Result<(), VfsError> {
    let path = normalize_path(path);
    let mut mounts = MOUNTS.lock();
    
    let index = mounts.iter().position(|m| m.path == path)
        .ok_or(VfsError::NotMounted)?;
    
    mounts[index].fs.unmount()?;
    mounts.remove(index);
    Ok(())
}

fn find_mount(path: &str) -> Option<(usize, String)> {
    let mounts = MOUNTS.lock();
    let mut best_match = None;
    let mut best_len = 0;
    
    for (i, m) in mounts.iter().enumerate() {
        if path.starts_with(&m.path) {
            if m.path.len() > best_len {
                best_len = m.path.len();
                best_match = Some((i, m.path.clone()));
            }
        }
    }
    best_match
}

fn resolve_path(path: &str) -> Result<(Arc<dyn Inode>, String), VfsError> {
    let path = normalize_path(path);
    let (mount_idx, mount_path) = find_mount(&path).ok_or(VfsError::NotMounted)?;
    let mounts = MOUNTS.lock();
    let mount = &mounts[mount_idx];
    
    let relative = if path == mount_path {
        String::new()
    } else if path.starts_with(&mount_path) && path.as_bytes().get(mount_path.len()) == Some(&b'/') {
        path[mount_path.len() + 1..].to_string()
    } else {
        path[mount_path.len()..].to_string()
    };
    
    let mut inode = mount.root_inode.clone();
    
    if relative.is_empty() {
        return Ok((inode, String::new()));
    }
    
    let components: Vec<&str> = relative.split('/').filter(|s| !s.is_empty()).collect();
    let mut last_name = String::new();
    
    for (i, component) in components.iter().enumerate() {
        last_name = component.to_string();
        if i == components.len() - 1 {
            break;
        }
        inode = inode.lookup(component)?;
    }
    
    Ok((inode, last_name))
}

pub fn open(path: &str, flags: u32, mode: u32) -> Result<u32, VfsError> {
    let (parent, name) = resolve_path(path)?;
    
    let inode = if name.is_empty() {
        parent
    } else if flags & 0x40 != 0 { // O_CREAT
        parent.create(&name, FileType::Regular, mode)?
    } else {
        parent.lookup(&name)?
    };
    
    let file = inode.open_file(flags)?;
    
    let mut next_fd = NEXT_FD.lock();
    let fd = *next_fd;
    *next_fd += 1;
    
    let mut file_table = FILE_TABLE.lock();
    if fd as usize >= file_table.len() {
        file_table.resize(fd as usize + 1, None);
    }
    
    let path = normalize_path(path);
    file_table[fd as usize] = Some(FileDesc {
        file,
        inode,
        flags,
        offset: if flags & 0x400 != 0 { 0 } else { 0 }, // O_APPEND handled in write
        ref_count: 1,
        path,
    });
    
    Ok(fd)
}

pub fn close(fd: u32) -> Result<(), VfsError> {
    let mut file_table = FILE_TABLE.lock();
    if fd as usize >= file_table.len() || file_table[fd as usize].is_none() {
        return Err(VfsError::BadFileDescriptor);
    }
    file_table[fd as usize] = None;
    Ok(())
}

pub fn read(fd: u32, buf: &mut [u8]) -> Result<usize, VfsError> {
    let mut file_table = FILE_TABLE.lock();
    let desc = file_table.get_mut(fd as usize).and_then(|d| d.as_mut())
        .ok_or(VfsError::BadFileDescriptor)?;
    
    let mut file = Arc::get_mut(&mut desc.file).ok_or(VfsError::IoError)?;
    let n = file.read(buf, desc.offset)?;
    desc.offset += n as u64;
    Ok(n)
}

pub fn write(fd: u32, buf: &[u8]) -> Result<usize, VfsError> {
    let mut file_table = FILE_TABLE.lock();
    let desc = file_table.get_mut(fd as usize).and_then(|d| d.as_mut())
        .ok_or(VfsError::BadFileDescriptor)?;
    
    let offset = if desc.flags & 0x400 != 0 {
        // O_APPEND - get file size
        let mut inode = Arc::get_mut(&mut desc.inode).ok_or(VfsError::IoError)?;
        let attr = inode.attr()?;
        attr.size
    } else {
        desc.offset
    };
    
    let mut file = Arc::get_mut(&mut desc.file).ok_or(VfsError::IoError)?;
    let n = file.write(buf, offset)?;
    desc.offset = offset + n as u64;
    Ok(n)
}

pub fn lseek(fd: u32, offset: i64, whence: u32) -> Result<u64, VfsError> {
    let mut file_table = FILE_TABLE.lock();
    let desc = file_table.get_mut(fd as usize).and_then(|d| d.as_mut())
        .ok_or(VfsError::BadFileDescriptor)?;
    
    let new_offset = match whence {
        0 => offset as u64, // SEEK_SET
        1 => {
            let mut inode = Arc::get_mut(&mut desc.inode).ok_or(VfsError::IoError)?;
            let attr = inode.attr()?;
            if offset < 0 && (-offset) as u64 > desc.offset {
                return Err(VfsError::InvalidName);
            }
            (desc.offset as i64 + offset) as u64
        }
        2 => {
            let mut inode = Arc::get_mut(&mut desc.inode).ok_or(VfsError::IoError)?;
            let attr = inode.attr()?;
            if offset < 0 && (-offset) as u64 > attr.size {
                return Err(VfsError::InvalidName);
            }
            (attr.size as i64 + offset) as u64
        }
        _ => return Err(VfsError::InvalidName),
    };
    
    desc.offset = new_offset;
    Ok(new_offset)
}

pub fn stat(path: &str) -> Result<FileAttr, VfsError> {
    let (parent, name) = resolve_path(path)?;
    
    let inode = if name.is_empty() {
        parent
    } else {
        let new_inode = parent.lookup(&name)?;
        Arc::from(new_inode)
    };
    
    inode.attr()
}

pub fn fstat(fd: u32) -> Result<FileAttr, VfsError> {
    let file_table = FILE_TABLE.lock();
    let desc = file_table.get(fd as usize).and_then(|d| d.as_ref())
        .ok_or(VfsError::BadFileDescriptor)?;
    
    desc.inode.attr()
}

pub fn mkdir(path: &str, mode: u32) -> Result<(), VfsError> {
    let (parent, name) = resolve_path(path)?;
    if name.is_empty() {
        return Err(VfsError::AlreadyExists);
    }
    parent.mkdir(&name, mode)?;
    Ok(())
}

pub fn unlink(path: &str) -> Result<(), VfsError> {
    let (parent, name) = resolve_path(path)?;
    if name.is_empty() {
        return Err(VfsError::NotFound);
    }
    parent.unlink(&name)
}

pub fn rmdir(path: &str) -> Result<(), VfsError> {
    let (parent, name) = resolve_path(path)?;
    if name.is_empty() {
        return Err(VfsError::NotFound);
    }
    parent.rmdir(&name)
}

pub fn rename(old_path: &str, new_path: &str) -> Result<(), VfsError> {
    let (old_parent, old_name) = resolve_path(old_path)?;
    let (new_parent, new_name) = resolve_path(new_path)?;
    
    if old_name.is_empty() || new_name.is_empty() {
        return Err(VfsError::InvalidName);
    }
    
    old_parent.rename(&old_name, &*new_parent, &new_name)
}

pub fn read_dir(path: &str, offset: u64, entries: &mut Vec<DirEntry>) -> Result<u64, VfsError> {
    let (parent, name) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name.is_empty() {
        parent
    } else {
        parent.lookup(&name)?
    };
    
    inode.read_dir(offset, entries)
}

pub fn freaddir(fd: u32, offset: u64, entries: &mut Vec<DirEntry>) -> Result<u64, VfsError> {
    let file_table = FILE_TABLE.lock();
    let desc = file_table.get(fd as usize).and_then(|d| d.as_ref())
        .ok_or(VfsError::BadFileDescriptor)?;
    
    // Use the inode directly from the file descriptor
    desc.inode.read_dir(offset, entries)
}

pub fn sync() -> Result<(), VfsError> {
    let mut mounts = MOUNTS.lock();
    for mount in mounts.iter_mut() {
        mount.fs.sync()?;
    }
    Ok(())
}

pub fn statfs(path: &str) -> Result<StatFs, VfsError> {
    let (mount_idx, _) = find_mount(path).ok_or(VfsError::NotMounted)?;
    let mounts = MOUNTS.lock();
    mounts[mount_idx].fs.statfs()
}

pub fn readlink(path: &str, buf: &mut [u8]) -> Result<usize, VfsError> {
    let (parent, name) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name.is_empty() {
        return Err(VfsError::InvalidName);
    } else {
        parent.lookup(&name)?
    };
    
    let target = inode.readlink()?;
    let target_bytes = target.as_bytes();
    let copy_len = target_bytes.len().min(buf.len());
    buf[..copy_len].copy_from_slice(&target_bytes[..copy_len]);
    Ok(copy_len)
}

pub fn symlink(target: &str, linkpath: &str) -> Result<(), VfsError> {
    let (parent, name) = resolve_path(linkpath)?;
    
    if name.is_empty() {
        return Err(VfsError::InvalidName);
    }
    
    parent.symlink(target, &name)
}

pub fn chmod(path: &str, mode: u32) -> Result<(), VfsError> {
    let (parent, name) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name.is_empty() {
        parent
    } else {
        parent.lookup(&name)?
    };
    
    let mut attr = inode.attr()?;
    attr.mode = (attr.mode & 0o170000) | (mode & 0o7777);
    inode.set_attr(&attr)
}

pub fn chown(path: &str, uid: u32, gid: u32) -> Result<(), VfsError> {
    let (parent, name) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name.is_empty() {
        parent
    } else {
        parent.lookup(&name)?
    };
    
    let mut attr = inode.attr()?;
    attr.uid = uid;
    attr.gid = gid;
    inode.set_attr(&attr)
}

pub fn utimes(path: &str, atime: u64, mtime: u64) -> Result<(), VfsError> {
    let (parent, name) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name.is_empty() {
        parent
    } else {
        parent.lookup(&name)?
    };
    
    let mut attr = inode.attr()?;
    attr.atime = atime;
    attr.mtime = mtime;
    attr.ctime = crate::drivers::pit::ticks() as u64;
    inode.set_attr(&attr)
}

pub fn xattr_set(path: &str, name: &str, value: &[u8], _flags: u32) -> Result<(), VfsError> {
    let (parent, name_) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name_.is_empty() {
        parent
    } else {
        parent.lookup(&name_)?
    };
    
    // For now, store xattrs as hidden files in a special namespace
    // This is a simple implementation - a real one would use extended attributes in the filesystem
    let xattr_path = alloc::format!(".xattr/{}", name);
    inode.create(&xattr_path, FileType::Regular, 0o600)?;
    let mut file = inode.open_file(0)?;
    file.write(value, 0)?;
    file.flush()?;
    Ok(())
}

pub fn xattr_get(path: &str, name: &str, buf: &mut [u8]) -> Result<usize, VfsError> {
    let (parent, name_) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name_.is_empty() {
        parent
    } else {
        parent.lookup(&name_)?
    };
    
    let xattr_path = alloc::format!(".xattr/{}", name);
    let xattr_inode = inode.lookup(&xattr_path)?;
    let file = xattr_inode.open_file(0)?;
    file.read(buf, 0)
}

pub fn xattr_list(path: &str, buf: &mut [u8]) -> Result<usize, VfsError> {
    let (parent, name_) = resolve_path(path)?;
    
    let inode: Arc<dyn Inode> = if name_.is_empty() {
        parent
    } else {
        parent.lookup(&name_)?
    };
    
    let xattr_dir = inode.lookup(".xattr");
    if xattr_dir.is_err() {
        return Ok(0);
    }
    let xattr_dir = xattr_dir?;
    
    let mut entries = Vec::new();
    xattr_dir.read_dir(0, &mut entries)?;
    
    let mut offset = 0;
    for entry in entries {
        let name = entry.name;
        let name_bytes = name.as_bytes();
        if offset + name.len() + 1 > buf.len() {
            break;
        }
        buf[offset..offset + name.len()].copy_from_slice(name_bytes);
        offset += name.len();
        buf[offset] = 0;
        offset += 1;
    }
    Ok(offset)
}

pub fn xattr_remove(path: &str, name: &str) -> Result<(), VfsError> {
    let (parent, name_) = resolve_path(path)?;
    
    let inode = if name_.is_empty() {
        parent
    } else {
        parent.lookup(&name_)?
    };
    
    let xattr_path = alloc::format!(".xattr/{}", name);
    inode.unlink(&xattr_path)
}

fn normalize_path(path: &str) -> String {
    let mut result = String::new();
    let mut components = Vec::new();
    
    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => { components.pop(); }
            _ => components.push(part),
        }
    }
    
    if components.is_empty() {
        result.push('/');
    } else {
        for comp in components {
            result.push('/');
            result.push_str(comp);
        }
    }
    
    result
}

impl FileDesc {
    fn stdin() -> Self {
        Self {
            file: Arc::new(NullFile),
            inode: Arc::new(NullInode),
            flags: 0,
            offset: 0,
            ref_count: 1,
            path: "/dev/stdin".to_string(),
        }
    }
    
    fn stdout() -> Self {
        Self {
            file: Arc::new(ConsoleFile),
            inode: Arc::new(NullInode),
            flags: 1,
            offset: 0,
            ref_count: 1,
            path: "/dev/stdout".to_string(),
        }
    }
    
    fn stderr() -> Self {
        Self {
            file: Arc::new(ConsoleFile),
            inode: Arc::new(NullInode),
            flags: 1,
            offset: 0,
            ref_count: 1,
            path: "/dev/stderr".to_string(),
        }
    }
}

struct NullFile;
impl File for NullFile {
    fn read(&self, buf: &mut [u8], _offset: u64) -> Result<usize, VfsError> {
        for b in buf.iter_mut() { *b = 0; }
        Ok(buf.len())
    }
    fn write(&self, _buf: &[u8], _offset: u64) -> Result<usize, VfsError> { Ok(0) }
    fn flush(&self) -> Result<(), VfsError> { Ok(()) }
    fn fsync(&self) -> Result<(), VfsError> { Ok(()) }
    fn ioctl(&self, _request: u32, _arg: u64) -> Result<u64, VfsError> { Err(VfsError::NotSupported) }
    fn get_inode(&self) -> Arc<dyn Inode> { Arc::new(NullInode) }
}

struct ConsoleFile;
impl File for ConsoleFile {
    fn read(&self, buf: &mut [u8], _offset: u64) -> Result<usize, VfsError> {
        for b in buf.iter_mut() { *b = 0; }
        Ok(buf.len())
    }
    fn write(&self, buf: &[u8], _offset: u64) -> Result<usize, VfsError> {
        crate::drivers::serial::write_string(core::str::from_utf8(buf).unwrap_or(""));
        Ok(buf.len())
    }
    fn flush(&self) -> Result<(), VfsError> { Ok(()) }
    fn fsync(&self) -> Result<(), VfsError> { Ok(()) }
    fn ioctl(&self, _request: u32, _arg: u64) -> Result<u64, VfsError> { Err(VfsError::NotSupported) }
    fn get_inode(&self) -> Arc<dyn Inode> { Arc::new(NullInode) }
}

struct NullInode;
impl Inode for NullInode {
    fn inode_id(&self) -> u64 { 0 }
    fn attr(&self) -> Result<FileAttr, VfsError> { Ok(FileAttr::default()) }
    fn set_attr(&self, _attr: &FileAttr) -> Result<(), VfsError> { Ok(()) }
    fn lookup(&self, _name: &str) -> Result<Arc<dyn Inode>, VfsError> { Err(VfsError::NotFound) }
    fn create(&self, _name: &str, _file_type: FileType, _mode: u32) -> Result<Arc<dyn Inode>, VfsError> { Err(VfsError::NotSupported) }
    fn unlink(&self, _name: &str) -> Result<(), VfsError> { Err(VfsError::NotSupported) }
    fn mkdir(&self, _name: &str, _mode: u32) -> Result<Arc<dyn Inode>, VfsError> { Err(VfsError::NotSupported) }
    fn rmdir(&self, _name: &str) -> Result<(), VfsError> { Err(VfsError::NotSupported) }
    fn rename(&self, _old_name: &str, _target_dir: &dyn Inode, _new_name: &str) -> Result<(), VfsError> { Err(VfsError::NotSupported) }
    fn read_dir(&self, _offset: u64, _entries: &mut Vec<DirEntry>) -> Result<u64, VfsError> { Ok(0) }
    fn open_file(&self, _flags: u32) -> Result<Arc<dyn File>, VfsError> { Err(VfsError::NotSupported) }
    fn readlink(&self) -> Result<String, VfsError> { Err(VfsError::NotSupported) }
    fn symlink(&self, _target: &str, _name: &str) -> Result<(), VfsError> { Err(VfsError::NotSupported) }
}