// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

// IDE disk driver with NebulaFS filesystem for x86.

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use crate::sync::SpinLock;

#[cfg(target_arch = "x86")]
const DATA: u16 = 0x1F0;
#[cfg(target_arch = "x86")]
const ERROR: u16 = 0x1F1;
#[cfg(target_arch = "x86")]
const SECTOR_COUNT: u16 = 0x1F2;
#[cfg(target_arch = "x86")]
const LBA_LOW: u16 = 0x1F3;
#[cfg(target_arch = "x86")]
const LBA_MID: u16 = 0x1F4;
#[cfg(target_arch = "x86")]
const LBA_HIGH: u16 = 0x1F5;
#[cfg(target_arch = "x86")]
const DRIVE: u16 = 0x1F6;
#[cfg(target_arch = "x86")]
const STATUS: u16 = 0x1F7;
#[cfg(target_arch = "x86")]
const COMMAND: u16 = 0x1F7;

const SUPER_SECTOR: u32 = 0;
const DIRECTORY_SECTOR: u32 = 1;
const DATA_START: u32 = 2;
const MAX_FILES: usize = 16;
const NAME_LEN: usize = 16;
const SECTOR_SIZE: usize = 512;

const CMD_IDENTIFY: u8 = 0xEC;
const CMD_READ_SECTORS: u8 = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;

const STATUS_BSY: u8 = 0x80;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

const NEBFS_MAGIC: u32 = 0x4E42_4653; // "NEBFS"
const NEBFS_VERSION: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StorageError {
    NoDisk,
    NotReady,
    ReadError,
    WriteError,
    NotFormatted,
    NotFound,
    InvalidName,
    NotBlank,
    DirectoryFull,
    FileTooLarge,
    Io,
}

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
struct DirEntry {
    name: [u8; NAME_LEN],
    length: u32,
    start_sector: u32,
    _reserved: [u8; 16],
}

#[derive(Clone, Copy)]
struct DiskState {
    sectors: u32,
    formatted: bool,
}

#[cfg(target_arch = "x86")]
static DISK: SpinLock<Option<DiskState>> = SpinLock::new(None);

#[cfg(target_arch = "x86")]
fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

#[cfg(target_arch = "x86")]
fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

#[cfg(target_arch = "x86")]
fn port_word_out(port: u16, data: u16) {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") port, in("ax") data);
    }
}

#[cfg(target_arch = "x86")]
fn port_word_in(port: u16) -> u16 {
    let val: u16;
    unsafe {
        core::arch::asm!("in ax, dx", out("ax") val, in("dx") port);
    }
    val
}

#[cfg(target_arch = "x86")]
fn wait_ready() -> Result<(), StorageError> {
    for _ in 0..1_000_000 {
        let status = port_byte_in(STATUS);
        if status & STATUS_BSY == 0 {
            if status & STATUS_ERR != 0 {
                return Err(StorageError::ReadError);
            }
            return Ok(());
        }
    }
    Err(StorageError::NotReady)
}

#[cfg(target_arch = "x86")]
fn wait_drq() -> Result<(), StorageError> {
    for _ in 0..1_000_000 {
        let status = port_byte_in(STATUS);
        if status & STATUS_BSY == 0 && status & STATUS_DRQ != 0 {
            return Ok(());
        }
    }
    Err(StorageError::NotReady)
}

#[cfg(target_arch = "x86")]
fn select_drive(lba: u32) -> Result<(), StorageError> {
    port_byte_out(DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);
    wait_ready()
}

#[cfg(target_arch = "x86")]
fn read_sector_raw(lba: u32, buf: *mut u8) -> Result<(), StorageError> {
    select_drive(lba)?;
    port_byte_out(SECTOR_COUNT, 1);
    port_byte_out(LBA_LOW, lba as u8);
    port_byte_out(LBA_MID, (lba >> 8) as u8);
    port_byte_out(LBA_HIGH, (lba >> 16) as u8);
    port_byte_out(COMMAND, CMD_READ_SECTORS);
    wait_drq()?;
    for i in 0..(SECTOR_SIZE / 2) {
        let word = port_word_in(DATA);
        unsafe {
            *buf.add(i * 2) = word as u8;
            *buf.add(i * 2 + 1) = (word >> 8) as u8;
        }
    }
    Ok(())
}

#[cfg(target_arch = "x86")]
fn write_sector_raw(lba: u32, buf: *const u8) -> Result<(), StorageError> {
    select_drive(lba)?;
    port_byte_out(SECTOR_COUNT, 1);
    port_byte_out(LBA_LOW, lba as u8);
    port_byte_out(LBA_MID, (lba >> 8) as u8);
    port_byte_out(LBA_HIGH, (lba >> 16) as u8);
    port_byte_out(COMMAND, CMD_WRITE_SECTORS);
    wait_drq()?;
    for i in 0..(SECTOR_SIZE / 2) {
        let word = unsafe { (buf.add(i * 2).read() as u16) | ((buf.add(i * 2 + 1).read() as u16) << 8) };
        port_word_out(DATA, word);
    }
    wait_ready()?;
    Ok(())
}

#[cfg(target_arch = "x86")]
fn identify_drive() -> bool {
    port_byte_out(DRIVE, 0xA0);
    wait_ready().is_ok() && {
        port_byte_out(SECTOR_COUNT, 0);
        port_byte_out(LBA_LOW, 0);
        port_byte_out(LBA_MID, 0);
        port_byte_out(LBA_HIGH, 0);
        port_byte_out(COMMAND, CMD_IDENTIFY);
        match wait_drq() {
            Ok(()) => {
                let _ = port_word_in(DATA);
                true
            }
            Err(_) => false,
        }
    }
}

pub fn init() -> Result<(), StorageError> {
    #[cfg(target_arch = "x86")]
    {
        if !identify_drive() {
            return Err(StorageError::NoDisk);
        }
        let sectors = 1024u32; // fixed-size virtual disk for QEMU IDE
        let formatted = {
            let mut buf = [0u8; SECTOR_SIZE];
            if read_sector_raw(SUPER_SECTOR, buf.as_mut_ptr()).is_ok() {
                let sb = unsafe { &*(buf.as_ptr() as *const SuperBlock) };
                sb.magic == NEBFS_MAGIC && sb.version == NEBFS_VERSION && sb.formatted != 0
            } else {
                false
            }
        };
        DISK.lock().replace(DiskState { sectors, formatted });
    }
    Ok(())
}

pub fn is_formatted() -> bool {
    DISK.lock().as_ref().map(|d| d.formatted).unwrap_or(false)
}

pub fn read_sector(lba: u32, buf: *mut u8) -> Result<(), StorageError> {
    #[cfg(target_arch = "x86")]
    {
        let disk = DISK.lock();
        let Some(_) = disk.as_ref() else {
            return Err(StorageError::NoDisk);
        };
        drop(disk);
        read_sector_raw(lba, buf)
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (lba, buf);
        Err(StorageError::NoDisk)
    }
}

pub fn write_sector(lba: u32, buf: *const u8) -> Result<(), StorageError> {
    #[cfg(target_arch = "x86")]
    {
        let disk = DISK.lock();
        let Some(_) = disk.as_ref() else {
            return Err(StorageError::NoDisk);
        };
        drop(disk);
        write_sector_raw(lba, buf)
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (lba, buf);
        Err(StorageError::NoDisk)
    }
}

pub fn is_available() -> bool {
    DISK.lock().is_some()
}

pub fn list_files() -> Result<Vec<(&'static str, u32)>, StorageError> {
    let _ = is_available();
    let mut buf = [0u8; SECTOR_SIZE];
    read_sector(DIRECTORY_SECTOR, buf.as_mut_ptr())?;
    let entries = unsafe { &*(buf.as_ptr() as *const [DirEntry; MAX_FILES]) };
    let mut files = Vec::new();
    for entry in entries.iter() {
        if entry.name[0] == 0 {
            continue;
        }
        let name_len = entry
            .name
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(NAME_LEN);
        let name = core::str::from_utf8(&entry.name[..name_len]).unwrap_or("");
        // Leak the string to satisfy &'static str lifetime
        let static_name: &'static str = Box::leak(name.to_string().into_boxed_str());
        files.push((static_name, entry.length));
    }
    Ok(files)
}

fn find_dir_entry(name: &[u8]) -> Option<(usize, DirEntry)> {
    let mut buf = [0u8; SECTOR_SIZE];
    read_sector(DIRECTORY_SECTOR, buf.as_mut_ptr()).ok()?;
    let entries = unsafe { &*(buf.as_ptr() as *const [DirEntry; MAX_FILES]) };
    for (index, entry) in entries.iter().enumerate() {
        if entry.name[0] == 0 {
            continue;
        }
        let name_len = entry
            .name
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(NAME_LEN);
        if name_len == name.len() && entry.name[..name_len] == name[..name_len] {
            return Some((index, *entry));
        }
    }
    None
}

fn find_free_dir_entry() -> Option<usize> {
    let mut buf = [0u8; SECTOR_SIZE];
    read_sector(DIRECTORY_SECTOR, buf.as_mut_ptr()).ok()?;
    let entries = unsafe { &*(buf.as_ptr() as *const [DirEntry; MAX_FILES]) };
    entries.iter().position(|e| e.name[0] == 0)
}

fn write_dir_entry(index: usize, entry: DirEntry) -> Result<(), StorageError> {
    let mut buf = [0u8; SECTOR_SIZE];
    read_sector(DIRECTORY_SECTOR, buf.as_mut_ptr())?;
    let entries = unsafe { &mut *(buf.as_ptr() as *mut [DirEntry; MAX_FILES]) };
    entries[index] = entry;
    write_sector(DIRECTORY_SECTOR, buf.as_ptr())
}

pub fn read_file(name: &str, output: &mut [u8]) -> Result<u32, StorageError> {
    let name_bytes = name.as_bytes();
    if name_bytes.len() > NAME_LEN {
        return Err(StorageError::InvalidName);
    }
    let Some((_, entry)) = find_dir_entry(name_bytes) else {
        return Err(StorageError::NotFound);
    };
    let length = entry.length as usize;
    if length > output.len() {
        return Err(StorageError::FileTooLarge);
    }
    let sectors_needed = (length + SECTOR_SIZE - 1) / SECTOR_SIZE;
    let mut offset = 0;
    for i in 0..sectors_needed {
        let sector = entry.start_sector + i as u32;
        let mut sector_buf = [0u8; SECTOR_SIZE];
        read_sector(sector, sector_buf.as_mut_ptr())?;
        let copy_len = (SECTOR_SIZE).min(length - offset);
        output[offset..offset + copy_len].copy_from_slice(&sector_buf[..copy_len]);
        offset += copy_len;
    }
    Ok(length as u32)
}

fn find_free_data_sector(excluding_start: u32, count_needed: u32) -> Option<u32> {
    let mut buf = [0u8; SECTOR_SIZE];
    read_sector(SUPER_SECTOR, buf.as_mut_ptr()).ok()?;
    let sb = unsafe { &*(buf.as_ptr() as *const SuperBlock) };
    let total = sb.total_sectors;
    let _data_start = sb.data_start;
    let end_sector = total.min(DATA_START + count_needed * 2);
    for sector in DATA_START..end_sector {
        if sector == excluding_start {
            continue;
        }
        let mut sector_buf = [0u8; SECTOR_SIZE];
        if read_sector(sector, sector_buf.as_mut_ptr()).is_ok()
            && sector_buf.iter().all(|b| *b == 0)
        {
            return Some(sector);
        }
    }
    None
}

pub fn write_file(name: &[u8], data: &[u8]) -> Result<(), StorageError> {
    if name.len() > NAME_LEN || name.is_empty() {
        return Err(StorageError::InvalidName);
    }
    if data.len() > MAX_FILES * SECTOR_SIZE {
        return Err(StorageError::FileTooLarge);
    }
    if let Some((index, existing)) = find_dir_entry(name) {
        // Overwrite existing file
        let old_sectors = (existing.length as usize + SECTOR_SIZE - 1) / SECTOR_SIZE;
        let new_sectors = (data.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;
        let start_sector = if new_sectors <= old_sectors {
            existing.start_sector
        } else {
            find_free_data_sector(existing.start_sector, new_sectors as u32)
                .ok_or(StorageError::DirectoryFull)?
        };
        let mut offset = 0;
        for i in 0..new_sectors {
            let sector = start_sector + i as u32;
            let mut sector_buf = [0u8; SECTOR_SIZE];
            let copy_len = SECTOR_SIZE.min(data.len() - offset);
            sector_buf[..copy_len].copy_from_slice(&data[offset..offset + copy_len]);
            write_sector(sector, sector_buf.as_ptr())?;
            offset += copy_len;
        }
        let entry = DirEntry {
            name: {
                let mut n = [0u8; NAME_LEN];
                n[..name.len()].copy_from_slice(name);
                n
            },
            length: data.len() as u32,
            start_sector,
            _reserved: [0; 16],
        };
        write_dir_entry(index, entry)
    } else {
        let index = find_free_dir_entry().ok_or(StorageError::DirectoryFull)?;
        let sectors_needed = (data.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;
        let start_sector = find_free_data_sector(0, sectors_needed as u32)
            .ok_or(StorageError::DirectoryFull)?;
        let mut offset = 0;
        for i in 0..sectors_needed {
            let sector = start_sector + i as u32;
            let mut sector_buf = [0u8; SECTOR_SIZE];
            let copy_len = SECTOR_SIZE.min(data.len() - offset);
            sector_buf[..copy_len].copy_from_slice(&data[offset..offset + copy_len]);
            write_sector(sector, sector_buf.as_ptr())?;
            offset += copy_len;
        }
        let entry = DirEntry {
            name: {
                let mut n = [0u8; NAME_LEN];
                n[..name.len()].copy_from_slice(name);
                n
            },
            length: data.len() as u32,
            start_sector,
            _reserved: [0; 16],
        };
        write_dir_entry(index, entry)
    }
}

pub fn delete_file(name: &[u8]) -> Result<(), StorageError> {
    let Some((index, _entry)) = find_dir_entry(name) else {
        return Err(StorageError::NotFound);
    };
    let empty = DirEntry {
        name: [0; NAME_LEN],
        length: 0,
        start_sector: 0,
        _reserved: [0; 16],
    };
    write_dir_entry(index, empty)
}

pub fn rename_file(old_name: &[u8], new_name: &[u8]) -> Result<(), StorageError> {
    if new_name.len() > NAME_LEN || new_name.is_empty() {
        return Err(StorageError::InvalidName);
    }
    let Some((index, mut entry)) = find_dir_entry(old_name) else {
        return Err(StorageError::NotFound);
    };
    if find_dir_entry(new_name).is_some() {
        return Err(StorageError::InvalidName);
    }
    entry.name = {
        let mut n = [0u8; NAME_LEN];
        n[..new_name.len()].copy_from_slice(new_name);
        n
    };
    write_dir_entry(index, entry)
}

pub fn mkdir(_name: &[u8]) -> Result<(), StorageError> {
    // NebulaFS is flat; directories are not supported.
    Err(StorageError::InvalidName)
}

pub fn format_if_blank() -> Result<(), StorageError> {
    if is_formatted() {
        return Err(StorageError::NotBlank);
    }
    // Check if disk is blank (all zeros)
    let mut buf = [0u8; SECTOR_SIZE];
    for sector in 0..16 {
        read_sector(sector, buf.as_mut_ptr())?;
        if buf.iter().any(|b| *b != 0) {
            return Err(StorageError::NotBlank);
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
        file_count: 0,
        _reserved: [0; 480],
    };
    let sb_buf = unsafe { core::slice::from_raw_parts(&sb as *const SuperBlock as *const u8, SECTOR_SIZE) };
    write_sector(SUPER_SECTOR, sb_buf.as_ptr())?;
    // Write empty directory
    let empty_dir = [0u8; SECTOR_SIZE];
    write_sector(DIRECTORY_SECTOR, empty_dir.as_ptr())?;
    DISK.lock().as_mut().map(|d| d.formatted = true);
    Ok(())
}
