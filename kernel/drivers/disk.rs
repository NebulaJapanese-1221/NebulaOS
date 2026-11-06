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

// IDE disk driver (x86) / stub (aarch64)

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
const DIRECTORY_SECTOR: u32 = 1;
const DATA_START: u32 = 2;

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

#[cfg(target_arch = "x86")]
static DISK: SpinLock<Option<DiskState>> = SpinLock::new(None);

struct DiskState {
    sectors: u32,
    formatted: bool,
}

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

pub fn init() -> Result<(), StorageError> {
    #[cfg(target_arch = "x86")]
    {
        port_byte_out(DRIVE, 0xE0);
        // Simple detect
        let _ = port_byte_in(STATUS);
        DISK.lock().replace(DiskState {
            sectors: 1024,
            formatted: false,
        });
        Ok(())
    }
    #[cfg(target_arch = "aarch64")]
    {
        Err(StorageError::NoDisk)
    }
}

pub fn is_formatted() -> bool {
    #[cfg(target_arch = "x86")]
    {
        DISK.lock().map(|d| d.formatted).unwrap_or(false)
    }
    #[cfg(target_arch = "aarch64")]
    {
        false
    }
}

pub fn read_sector(_lba: u32, _buf: *mut u8) -> Result<(), StorageError> {
    #[cfg(target_arch = "x86")]
    {
        Err(StorageError::ReadError)
    }
    #[cfg(target_arch = "aarch64")]
    {
        Err(StorageError::NoDisk)
    }
}

pub fn write_sector(_lba: u32, _buf: *const u8) -> Result<(), StorageError> {
    #[cfg(target_arch = "x86")]
    {
        Err(StorageError::WriteError)
    }
    #[cfg(target_arch = "aarch64")]
    {
        Err(StorageError::NoDisk)
    }
}

pub fn is_available() -> bool {
    #[cfg(target_arch = "x86")]
    {
        DISK.lock().is_some()
    }
    #[cfg(target_arch = "aarch64")]
    {
        false
    }
}

pub fn list_files() -> Result<Vec<(&'static str, u32)>, StorageError> {
    #[cfg(target_arch = "x86")]
    {
        Ok(Vec::new())
    }
    #[cfg(target_arch = "aarch64")]
    {
        Ok(Vec::new())
    }
}

pub fn read_file(_name: &str, _output: &mut [u8]) -> Result<u32, StorageError> {
    Err(StorageError::NotFound)
}

pub fn write_file(_name: &[u8], _data: &[u8]) -> Result<(), StorageError> {
    Err(StorageError::NotFound)
}

pub fn delete_file(_name: &[u8]) -> Result<(), StorageError> {
    Err(StorageError::NotFound)
}

pub fn rename_file(_old_name: &[u8], _new_name: &[u8]) -> Result<(), StorageError> {
    Err(StorageError::NotFound)
}

pub fn format_if_blank() -> Result<(), StorageError> {
    Err(StorageError::Io)
}
