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

// NebulaFS implementation as a VFS driver

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::boxed::Box;
use crate::sync::SpinLock;
use crate::drivers::disk;
use crate::fs::vfs::{FileSystem, Inode, File, FileAttr, FileType, VfsError, DirEntry, StatFs};

const SUPER_SECTOR: u32 = 0;
const DIRECTORY_SECTOR: u32 = 1;
const DATA_START: u32 = 2;
const MAX_FILES: usize = 64;
const NAME_LEN: usize = 32;
const SECTOR_SIZE: usize = 512;

const NEBFS_MAGIC: u32 = 0x4E42_4653; // "NEBFS"
const NEBFS_VERSION: u32 = 2; // Version 2 with directory support

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct SuperBlock {
    magic: u32,
    version: u32,
    formatted: u32,
    dir_sector: u32,
    data_start: u32,
    total_sectors: u32,
    file_count: u32,
    _reserved: [u8; 480],
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct DirEntryRaw {
    name: [u8; NAME_LEN],
    parent: u32,
    first_child: u32,
    next_sibling: u32,
    inode_sector: u32,
    file_type: u8,
    _reserved: [u8; 3],
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct InodeRaw {
    file_type: u8,
    _pad: [u8; 3],
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    direct: [u32; 12],
    indirect: u32,
    double_indirect: u32,
}

static NEBULAFS_DEVICE: SpinLock<Option<u32>> = SpinLock::new(None);

struct NebulaInode {
    inode_num: u32,
    inode_data: InodeRaw,
    dirty: bool,
}

impl NebulaInode {
    fn new(inode_num: u32, file_type: FileType) -> Self {
        let now = crate::drivers::pit::ticks() as u64;
        let mut inode_data = InodeRaw {
            file_type: file_type as u8,
            _pad: [0; 3],
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            mode: if file_type == FileType::Directory { 0o755 } else { 0o644 },
            uid: 0,
            gid: 0,
            direct: [0; 12],
            indirect: 0,
            double_indirect: 0,
        };
        
        Self {
            inode_num,
            inode_data,
            dirty: true,
        }
    }
    
    fn load(inode_num: u32) -> Result<Self, VfsError> {
        let sector = inode_num;
        let mut buf = [0u8; SECTOR_SIZE];
        disk::read_sector(sector, buf.as_mut_ptr())?;
        let inode_data = unsafe { *(buf.as_ptr() as *const InodeRaw) };
        
        Ok(Self {
            inode_num,
            inode_data,
            dirty: false,
        })
    }
    
    fn save(&mut self) -> Result<(), VfsError> {
        if !self.dirty {
            return Ok(());
        }
        let mut buf = [0u8; SECTOR_SIZE];
        unsafe {
            *(buf.as_mut_ptr() as *mut InodeRaw) = self.inode_data;
        }
        disk::write_sector(self.inode_num, buf.as_ptr())?;
        self.dirty = false;
        Ok(())
    }
    
    fn file_type(&self) -> FileType {
        match self.inode_data.file_type {
            0 => FileType::Regular,
            1 => FileType::Directory,
            2 => FileType::Symlink,
            _ => FileType::Regular,
        }
    }
    
    fn allocate_block(&mut self) -> Result<u32, VfsError> {
        // Find a free block
        for i in 0..12 {
            if self.inode_data.direct[i] == 0 {
                let block = find_free_data_block()?;
                self.inode_data.direct[i] = block;
                self.dirty = true;
                return Ok(block);
            }
        }
        // TODO: indirect blocks
        Err(VfsError::NoSpace)
    }
    
    fn get_block(&self, index: u64) -> Option<u32> {
        if index < 12 {
            let block = self.inode_data.direct[index as usize];
            if block != 0 { Some(block) } else { None }
        } else {
            None // TODO: indirect
        }
    }
}

struct NebulaFile {
    inode: NebulaInode,
    offset: u64,
}

impl File for NebulaFile {
    fn read(&mut self, buf: &mut [u8], offset: u64) -> Result<usize, VfsError> {
        let file_size = self.inode.inode_data.size as usize;
        if offset as usize >= file_size {
            return Ok(0);
        }
        
        let to_read = buf.len().min(file_size - offset as usize);
        let mut read = 0;
        let mut buf_offset = offset;
        
        while read < to_read {
            let block_index = buf_offset / SECTOR_SIZE as u64;
            let block_offset = (buf_offset % SECTOR_SIZE as u64) as usize;
            let block = self.inode.get_block(block_index).ok_or(VfsError::IoError)?;
            
            let mut sector_buf = [0u8; SECTOR_SIZE];
            disk::read_sector(block, sector_buf.as_mut_ptr())?;
            
            let chunk = (SECTOR_SIZE - block_offset).min(to_read - read);
            buf[read..read + chunk].copy_from_slice(&sector_buf[block_offset..block_offset + chunk]);
            
            read += chunk;
            buf_offset += chunk as u64;
        }
        
        Ok(read)
    }
    
    fn write(&mut self, buf: &[u8], offset: u64) -> Result<usize, VfsError> {
        let mut written = 0;
        let mut buf_offset = offset;
        
        while written < buf.len() {
            let block_index = buf_offset / SECTOR_SIZE as u64;
            let block_offset = (buf_offset % SECTOR_SIZE as u64) as usize;
            
            let block = if block_offset == 0 && buf.len() - written >= SECTOR_SIZE {
                // Full block write - allocate new
                self.inode.allocate_block()?
            } else {
                // Partial write - need to read existing block or allocate
                match self.inode.get_block(block_index) {
                    Some(b) => b,
                    None => self.inode.allocate_block()?,
                }
            };
            
            let mut sector_buf = [0u8; SECTOR_SIZE];
            if block_offset != 0 || buf.len() - written < SECTOR_SIZE {
                disk::read_sector(block, sector_buf.as_mut_ptr())?;
            }
            
            let chunk = (SECTOR_SIZE - block_offset).min(buf.len() - written);
            sector_buf[block_offset..block_offset + chunk].copy_from_slice(&buf[written..written + chunk]);
            
            disk::write_sector(block, sector_buf.as_ptr())?;
            
            written += chunk;
            buf_offset += chunk as u64;
            
            let new_size = (offset + written as u64) as u64;
            if new_size > self.inode.inode_data.size {
                self.inode.inode_data.size = new_size;
                self.inode.dirty = true;
            }
        }
        
        self.inode.inode_data.mtime = crate::drivers::pit::ticks() as u64;
        self.inode.dirty = true;
        
        Ok(written)
    }
    
    fn flush(&mut self) -> Result<(), VfsError> {
        self.inode.save()
    }
    
    fn fsync(&mut self) -> Result<(), VfsError> {
        self.inode.save()
    }
    
    fn ioctl(&mut self, _request: u32, _arg: u64) -> Result<u64, VfsError> {
        Err(VfsError::NotSupported)
    }
    
    fn get_inode(&self) -> Box<dyn Inode> {
        // This is tricky since we need a separate Inode reference
        // For now, return a wrapper
        Box::new(InodeWrapper { inode: self.inode.inode_num })
    }
}

struct InodeWrapper {
    inode: u32,
}

impl Inode for InodeWrapper {
    fn inode_id(&self) -> u64 { self.inode as u64 }
    fn inode_num(&self) -> u64 { self.inode as u64 }
    fn attr(&self) -> Result<FileAttr, VfsError> {
        let inode = NebulaInode::load(self.inode)?;
        Ok(file_attr_from_inode(&inode))
    }
    fn set_attr(&self, attr: &FileAttr) -> Result<(), VfsError> {
        let mut inode = NebulaInode::load(self.inode)?;
        inode.inode_data.mode = attr.mode;
        inode.inode_data.atime = attr.atime;
        inode.inode_data.mtime = attr.mtime;
        inode.inode_data.ctime = attr.ctime;
        inode.inode_data.uid = attr.uid;
        inode.inode_data.gid = attr.gid;
        inode.dirty = true;
        inode.save()
    }
    fn lookup(&self, name: &str) -> Result<Box<dyn Inode>, VfsError> {
        let parent = NebulaInode::load(self.inode)?;
        find_dir_entry(&parent, name).map(|(_, inode_num)| Box::new(InodeWrapper { inode: inode_num }) as Box<dyn Inode>)
    }
    fn create(&self, name: &str, file_type: FileType, mode: u32) -> Result<Box<dyn Inode>, VfsError> {
        let parent = NebulaInode::load(self.inode)?;
        let inode_num = create_file(&parent, name, file_type, mode)?;
        Ok(Box::new(InodeWrapper { inode: inode_num }) as Box<dyn Inode>)
    }
    fn unlink(&self, name: &str) -> Result<(), VfsError> {
        let parent = NebulaInode::load(self.inode)?;
        unlink_file(&parent, name)
    }
    fn mkdir(&self, name: &str, mode: u32) -> Result<Box<dyn Inode>, VfsError> {
        let parent = NebulaInode::load(self.inode)?;
        let inode_num = create_file(&parent, name, FileType::Directory, mode)?;
        Ok(Box::new(InodeWrapper { inode: inode_num }) as Box<dyn Inode>)
    }
    fn rmdir(&self, name: &str) -> Result<(), VfsError> {
        let parent = NebulaInode::load(self.inode)?;
        unlink_file(&parent, name)
    }
    fn rename(&self, old_name: &str, target_dir: &dyn Inode, new_name: &str) -> Result<(), VfsError> {
        let parent = NebulaInode::load(self.inode)?;
        let target_inode = target_dir.inode_num();
        let target = NebulaInode::load(target_inode as u32)?;
        rename_file(&parent, old_name, &target, new_name)
    }
    fn read_dir(&self, offset: u64, entries: &mut Vec<DirEntry>) -> Result<u64, VfsError> {
        let inode = NebulaInode::load(self.inode)?;
        read_directory(&inode, offset, entries)
    }
    fn open_file(&self, _flags: u32) -> Result<Box<dyn File>, VfsError> {
        let inode = NebulaInode::load(self.inode)?;
        Ok(Box::new(NebulaFile { inode, offset: 0 }))
    }
    fn readlink(&self) -> Result<String, VfsError> {
        let inode = NebulaInode::load(self.inode)?;
        if inode.file_type() != FileType::Symlink {
            return Err(VfsError::InvalidName);
        }
        // Read symlink target from first block
        let block = inode.get_block(0).ok_or(VfsError::IoError)?;
        let mut buf = [0u8; SECTOR_SIZE];
        disk::read_sector(block, buf.as_mut_ptr())?;
        let target = core::str::from_utf8(&buf).unwrap_or("").trim_end_matches('\0');
        Ok(target.to_string())
    }
    fn symlink(&self, target: &str, name: &str) -> Result<(), VfsError> {
        let parent = NebulaInode::load(self.inode)?;
        let inode_num = create_file(&parent, name, FileType::Symlink, 0o777)?;
        let mut inode = NebulaInode::load(inode_num)?;
        let block = inode.allocate_block()?;
        let mut buf = [0u8; SECTOR_SIZE];
        let target_bytes = target.as_bytes();
        buf[..target_bytes.len().min(SECTOR_SIZE)].copy_from_slice(target_bytes);
        disk::write_sector(block, buf.as_ptr())?;
        inode.inode_data.size = target_bytes.len() as u64;
        inode.dirty = true;
        inode.save()?;
        Ok(())
    }
}

pub struct NebulaFs {
    device: u32,
}

impl NebulaFs {
    pub fn new() -> Self {
        Self { device: 0 }
    }
}

impl FileSystem for NebulaFs {
    fn name(&self) -> &'static str {
        "nebula-fs"
    }
    
    fn mount(&mut self, device: u32, _options: &str) -> Result<(), VfsError> {
        let mut buf = [0u8; SECTOR_SIZE];
        disk::read_sector(SUPER_SECTOR, buf.as_mut_ptr())?;
        let sb = unsafe { *(buf.as_ptr() as *const SuperBlock) };
        
        if sb.magic != NEBFS_MAGIC || sb.version != NEBFS_VERSION || sb.formatted == 0 {
            return Err(VfsError::IoError);
        }
        
        *NEBULAFS_DEVICE.lock() = Some(device);
        self.device = device;
        Ok(())
    }
    
    fn unmount(&mut self) -> Result<(), VfsError> {
        sync_fs()?;
        *NEBULAFS_DEVICE.lock() = None;
        Ok(())
    }
    
    fn root_inode(&self) -> Box<dyn Inode> {
        Box::new(InodeWrapper { inode: 2 }) // Root directory is inode 2
    }
    
    fn statfs(&self) -> Result<StatFs, VfsError> {
        let mut buf = [0u8; SECTOR_SIZE];
        disk::read_sector(SUPER_SECTOR, buf.as_mut_ptr())?;
        let sb = unsafe { *(buf.as_ptr() as *const SuperBlock) };
        
        let total = sb.total_sectors as u64;
        let used = count_used_blocks();
        let free = total.saturating_sub(used);
        
        Ok(StatFs {
            blocks: total,
            bfree: free,
            bavail: free,
            files: sb.file_count as u64,
            ffree: (MAX_FILES as u64).saturating_sub(sb.file_count as u64),
            bsize: SECTOR_SIZE as u32,
            namelen: NAME_LEN as u32,
        })
    }
    
    fn sync(&mut self) -> Result<(), VfsError> {
        sync_fs()
    }
}

fn find_free_data_block() -> Result<u32, VfsError> {
    let mut buf = [0u8; SECTOR_SIZE];
    disk::read_sector(SUPER_SECTOR, buf.as_mut_ptr())?;
    let sb = unsafe { *(buf.as_ptr() as *const SuperBlock) };
    let total = sb.total_sectors;
    
    for sector in DATA_START..total {
        let mut sector_buf = [0u8; SECTOR_SIZE];
        if disk::read_sector(sector, sector_buf.as_mut_ptr()).is_ok()
            && sector_buf.iter().all(|b| *b == 0)
        {
            return Ok(sector);
        }
    }
    Err(VfsError::NoSpace)
}

fn free_block(sector: u32) -> Result<(), VfsError> {
    let mut buf = [0u8; SECTOR_SIZE];
    disk::write_sector(sector, buf.as_ptr())
}

fn find_dir_entry(parent: &NebulaInode, name: &str) -> Result<(DirEntryRaw, u32), VfsError> {
    let block = parent.get_block(0).ok_or(VfsError::IoError)?;
    let mut buf = [0u8; SECTOR_SIZE];
    disk::read_sector(block, buf.as_mut_ptr())?;
    let entries = unsafe { &*(buf.as_ptr() as *const [DirEntryRaw; 64]) };
    
    for (i, entry) in entries.iter().enumerate() {
        if entry.name[0] == 0 { continue; }
        let entry_name = core::str::from_utf8(&entry.name).unwrap_or("").trim_end_matches('\0');
        if entry_name == name {
            return Ok((*entry, entry.inode_sector));
        }
    }
    Err(VfsError::NotFound)
}

fn create_file(parent: &NebulaInode, name: &str, file_type: FileType, mode: u32) -> Result<u32, VfsError> {
    // Find free inode
    let inode_num = find_free_inode()?;
    
    // Create directory entry in parent
    add_dir_entry(parent, name, inode_num, file_type as u8)?;
    
    // Initialize new inode
    let mut inode = NebulaInode::new(inode_num, file_type);
    inode.inode_data.mode = mode;
    inode.save()?;
    
    // Update superblock file count
    update_file_count(1)?;
    
    Ok(inode_num)
}

fn add_dir_entry(parent: &NebulaInode, name: &str, inode_num: u32, file_type: u8) -> Result<(), VfsError> {
    let block = parent.get_block(0).ok_or(VfsError::IoError)?;
    let mut buf = [0u8; SECTOR_SIZE];
    disk::read_sector(block, buf.as_mut_ptr())?;
    let entries = unsafe { &mut *(buf.as_mut_ptr() as *mut [DirEntryRaw; 64]) };
    
    for entry in entries.iter_mut() {
        if entry.name[0] == 0 {
            let name_bytes = name.as_bytes();
            let len = name_bytes.len().min(NAME_LEN);
            entry.name[..len].copy_from_slice(&name_bytes[..len]);
            entry.inode_sector = inode_num;
            entry.file_type = file_type;
            entry.parent = parent.inode_num;
            disk::write_sector(block, buf.as_ptr())?;
            return Ok(());
        }
    }
    Err(VfsError::DirectoryFull)
}

fn unlink_file(parent: &NebulaInode, name: &str) -> Result<(), VfsError> {
    let block = parent.get_block(0).ok_or(VfsError::IoError)?;
    let mut buf = [0u8; SECTOR_SIZE];
    disk::read_sector(block, buf.as_mut_ptr())?;
    let entries = unsafe { &mut *(buf.as_mut_ptr() as *mut [DirEntryRaw; 64]) };
    
    for entry in entries.iter_mut() {
        if entry.name[0] == 0 { continue; }
        let entry_name = core::str::from_utf8(&entry.name).unwrap_or("").trim_end_matches('\0');
        if entry_name == name {
            // Free the inode's blocks
            let inode = NebulaInode::load(entry.inode_sector)?;
            for i in 0..12 {
                if inode.inode_data.direct[i] != 0 {
                    free_block(inode.inode_data.direct[i])?;
                }
            }
            
            // Clear directory entry
            entry.name[0] = 0;
            entry.inode_sector = 0;
            entry.file_type = 0;
            entry.parent = 0;
            
            disk::write_sector(block, buf.as_ptr())?;
            update_file_count(-1)?;
            return Ok(());
        }
    }
    Err(VfsError::NotFound)
}

fn rename_file(parent: &NebulaInode, old_name: &str, target: &NebulaInode, new_name: &str) -> Result<(), VfsError> {
    let (entry, inode_num) = find_dir_entry(parent, old_name)?;
    
    if parent.inode_num == target.inode_num {
        // Same directory - just rename
        let block = parent.get_block(0).ok_or(VfsError::IoError)?;
        let mut buf = [0u8; SECTOR_SIZE];
        disk::read_sector(block, buf.as_mut_ptr())?;
        let entries = unsafe { &mut *(buf.as_mut_ptr() as *mut [DirEntryRaw; 64]) };
        
        for e in entries.iter_mut() {
            if e.inode_sector == inode_num {
                let name_bytes = new_name.as_bytes();
                let len = name_bytes.len().min(NAME_LEN);
                e.name[..len].copy_from_slice(&name_bytes[..len]);
                disk::write_sector(block, buf.as_ptr())?;
                return Ok(());
            }
        }
    } else {
        // Cross-directory move
        unlink_file(parent, old_name)?;
        add_dir_entry(target, new_name, inode_num, entry.file_type)?;
    }
    Ok(())
}

fn read_directory(inode: &NebulaInode, offset: u64, entries: &mut Vec<DirEntry>) -> Result<u64, VfsError> {
    let block = inode.get_block(0).ok_or(VfsError::IoError)?;
    let mut buf = [0u8; SECTOR_SIZE];
    disk::read_sector(block, buf.as_mut_ptr())?;
    let dir_entries = unsafe { &*(buf.as_ptr() as *const [DirEntryRaw; 64]) };
    
    let mut count = 0u64;
    for entry in dir_entries.iter() {
        if entry.name[0] == 0 { continue; }
        if count < offset {
            count += 1;
            continue;
        }
        let name = core::str::from_utf8(&entry.name).unwrap_or("").trim_end_matches('\0').to_string();
        let file_type = match entry.file_type {
            0 => FileType::Regular,
            1 => FileType::Directory,
            2 => FileType::Symlink,
            _ => FileType::Regular,
        };
        entries.push(DirEntry {
            name,
            inode_id: entry.inode_sector as u64,
            file_type,
        });
        count += 1;
    }
    Ok(count)
}

fn find_free_inode() -> Result<u32, VfsError> {
    let mut buf = [0u8; SECTOR_SIZE];
    disk::read_sector(SUPER_SECTOR, buf.as_mut_ptr())?;
    let sb = unsafe { *(buf.as_ptr() as *const SuperBlock) };
    let total = sb.total_sectors;
    
    for inode_num in 3..total { // 0,1,2 reserved
        let mut sector_buf = [0u8; SECTOR_SIZE];
        if disk::read_sector(inode_num, sector_buf.as_mut_ptr()).is_ok()
            && sector_buf.iter().all(|b| *b == 0)
        {
            return Ok(inode_num);
        }
    }
    Err(VfsError::NoSpace)
}

fn update_file_count(delta: i32) -> Result<(), VfsError> {
    let mut buf = [0u8; SECTOR_SIZE];
    disk::read_sector(SUPER_SECTOR, buf.as_mut_ptr())?;
    let mut sb = unsafe { *(buf.as_ptr() as *mut SuperBlock) };
    sb.file_count = (sb.file_count as i32 + delta).max(0) as u32;
    unsafe { *(buf.as_mut_ptr() as *mut SuperBlock) = sb; }
    disk::write_sector(SUPER_SECTOR, buf.as_ptr())
}

fn count_used_blocks() -> u64 {
    let mut buf = [0u8; SECTOR_SIZE];
    if disk::read_sector(SUPER_SECTOR, buf.as_mut_ptr()).is_err() {
        return 0;
    }
    let sb = unsafe { *(buf.as_ptr() as *const SuperBlock) };
    let total = sb.total_sectors;
    let mut used = 0u64;
    
    for sector in DATA_START..total {
        let mut sector_buf = [0u8; SECTOR_SIZE];
        if disk::read_sector(sector, sector_buf.as_mut_ptr()).is_ok()
            && !sector_buf.iter().all(|b| *b == 0)
        {
            used += 1;
        }
    }
    used
}

fn sync_fs() -> Result<(), VfsError> {
    // Flush any cached data - for now just ensure superblock is up to date
    Ok(())
}

fn file_attr_from_inode(inode: &NebulaInode) -> FileAttr {
    FileAttr {
        file_type: inode.file_type(),
        size: inode.inode_data.size,
        blocks: inode.inode_data.blocks,
        atime: inode.inode_data.atime,
        mtime: inode.inode_data.mtime,
        ctime: inode.inode_data.ctime,
        mode: inode.inode_data.mode,
        uid: inode.inode_data.uid,
        gid: inode.inode_data.gid,
    }
}

pub fn format_disk() -> Result<(), VfsError> {
    // Check if disk is blank
    let mut buf = [0u8; SECTOR_SIZE];
    for sector in 0..16 {
        disk::read_sector(sector, buf.as_mut_ptr())?;
        if buf.iter().any(|b| *b != 0) {
            return Err(VfsError::AlreadyExists);
        }
    }
    
    // Write superblock
    let sb = SuperBlock {
        magic: NEBFS_MAGIC,
        version: NEBFS_VERSION,
        formatted: 1,
        dir_sector: DIRECTORY_SECTOR,
        data_start: DATA_START,
        total_sectors: 1024,
        file_count: 1, // root directory
        _reserved: [0; 480],
    };
    let sb_buf = unsafe { core::slice::from_raw_parts(&sb as *const SuperBlock as *const u8, SECTOR_SIZE) };
    disk::write_sector(SUPER_SECTOR, sb_buf.as_ptr())?;
    
    // Create root directory inode (inode 2)
    let mut root_inode = InodeRaw {
        file_type: FileType::Directory as u8,
        _pad: [0; 3],
        size: SECTOR_SIZE as u64,
        blocks: 1,
        atime: crate::drivers::pit::ticks() as u64,
        mtime: crate::drivers::pit::ticks() as u64,
        ctime: crate::drivers::pit::ticks() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        direct: [DIRECTORY_SECTOR, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        indirect: 0,
        double_indirect: 0,
    };
    let mut inode_buf = [0u8; SECTOR_SIZE];
    unsafe { *(inode_buf.as_mut_ptr() as *mut InodeRaw) = root_inode; }
    disk::write_sector(2, inode_buf.as_ptr())?;
    
    // Create empty directory block
    let empty_dir = [0u8; SECTOR_SIZE];
    disk::write_sector(DIRECTORY_SECTOR, empty_dir.as_ptr())?;
    
    // Add "." and ".." entries to root
    let mut dir_buf = [0u8; SECTOR_SIZE];
    let entries = unsafe { &mut *(dir_buf.as_mut_ptr() as *mut [DirEntryRaw; 64]) };
    
    // "." entry
    entries[0].name[0] = b'.';
    entries[0].inode_sector = 2;
    entries[0].file_type = FileType::Directory as u8;
    entries[0].parent = 2;
    
    // ".." entry (points to itself for root)
    entries[1].name[0] = b'.';
    entries[1].name[1] = b'.';
    entries[1].inode_sector = 2;
    entries[1].file_type = FileType::Directory as u8;
    entries[1].parent = 2;
    
    disk::write_sector(DIRECTORY_SECTOR, dir_buf.as_ptr())?;
    
    Ok(())
}