//! ZFS Attribute Processor (ZAP) - Simplified
//! Handles directory entries and key-value lookups.

use alloc::vec::Vec;
use alloc::string::String;
use core::mem::size_of;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DirentPhys {
    pub inode: u64,
    pub type_: u8,      // 0=None, 3=File, 4=Dir
    pub name_len: u8,
    pub name: [u8; 54], // Fixed size name for simplicity in this MicroZAP version
}

pub const DIRENT_SIZE: usize = size_of::<DirentPhys>();

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub inode: u64,
    pub type_: u8,
    pub name: String,
}

/// Parses a block of data as a list of directory entries.
pub fn parse_directory(data: &[u8]) -> Vec<DirectoryEntry> {
    let mut entries = Vec::new();
    let count = data.len() / DIRENT_SIZE;

    for i in 0..count {
        let offset = i * DIRENT_SIZE;
        if offset + DIRENT_SIZE > data.len() { break; }
        
        let dirent = unsafe { &*(data.as_ptr().add(offset) as *const DirentPhys) };
        
        if dirent.inode != 0 {
            let name_len = dirent.name_len.min(54) as usize;
            let name_bytes = &dirent.name[..name_len];
            if let Ok(name) = core::str::from_utf8(name_bytes) {
                entries.push(DirectoryEntry {
                    inode: dirent.inode,
                    type_: dirent.type_,
                    name: String::from(name),
                });
            }
        }
    }
    entries
}