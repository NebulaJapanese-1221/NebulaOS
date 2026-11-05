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

use crate::syscall::{self, StorageFileInfo, StorageRequest};

const FILE_SLOTS: usize = 16;
const FILE_BUFFER_SIZE: usize = 512;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    NoDisk,
    Io,
    NotFormatted,
    NotBlank,
    InvalidName,
    FileTooLarge,
    DirectoryFull,
    NotFound,
}

#[derive(Clone, Copy)]
pub struct FileInfo {
    pub name: [u8; 16],
    pub length: u16,
}

pub fn list_files() -> Result<[FileInfo; FILE_SLOTS], StorageError> {
    let mut files = [StorageFileInfo::default(); FILE_SLOTS];
    let mut request = StorageRequest {
        op: 0,
        out_ptr: files.as_mut_ptr() as u32,
        out_len: core::mem::size_of_val(&files) as u32,
        ..StorageRequest::default()
    };
    check(syscall::storage(&mut request))?;
    Ok(core::array::from_fn(|index| FileInfo {
        name: files[index].name,
        length: files[index].length,
    }))
}

pub fn read_file(name: &[u8], output: &mut [u8; FILE_BUFFER_SIZE]) -> Result<usize, StorageError> {
    let mut request = StorageRequest {
        op: 1,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        out_ptr: output.as_mut_ptr() as u32,
        out_len: output.len() as u32,
        ..StorageRequest::default()
    };
    let length = syscall::storage(&mut request);
    check(length)?;
    let length = request.out_len as usize;
    if length > output.len() {
        return Err(StorageError::Io);
    }
    Ok(length)
}

pub fn write_file(name: &[u8], content: &[u8]) -> Result<(), StorageError> {
    let mut request = StorageRequest {
        op: 2,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        data_ptr: content.as_ptr() as u32,
        data_len: content.len() as u32,
        ..StorageRequest::default()
    };
    check(syscall::storage(&mut request))
}

pub fn delete_file(name: &[u8]) -> Result<(), StorageError> {
    let mut request = StorageRequest {
        op: 3,
        name_ptr: name.as_ptr() as u32,
        name_len: name.len() as u32,
        ..StorageRequest::default()
    };
    check(syscall::storage(&mut request))
}

pub fn rename_file(old_name: &[u8], new_name: &[u8]) -> Result<(), StorageError> {
    let mut request = StorageRequest {
        op: 4,
        name_ptr: old_name.as_ptr() as u32,
        name_len: old_name.len() as u32,
        data_ptr: new_name.as_ptr() as u32,
        data_len: new_name.len() as u32,
        ..StorageRequest::default()
    };
    check(syscall::storage(&mut request))
}

pub fn format_if_blank() -> Result<(), StorageError> {
    let mut request = StorageRequest {
        op: 5,
        ..StorageRequest::default()
    };
    check(syscall::storage(&mut request))
}

fn check(status: u32) -> Result<(), StorageError> {
    match status {
        0 => Ok(()),
        1 => Err(StorageError::NoDisk),
        2 => Err(StorageError::Io),
        3 => Err(StorageError::NotFormatted),
        4 => Err(StorageError::NotBlank),
        5 => Err(StorageError::InvalidName),
        6 => Err(StorageError::FileTooLarge),
        7 => Err(StorageError::DirectoryFull),
        8 => Err(StorageError::NotFound),
        _ => Err(StorageError::Io),
    }
}
