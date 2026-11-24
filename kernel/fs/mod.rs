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

// Filesystem modules

pub mod vfs;
pub mod nebula_fs;

// Re-export VFS functions
pub use vfs::{
    unmount as vfs_unmount,
    open as vfs_open,
    close as vfs_close,
    read as vfs_read,
    write as vfs_write,
    lseek as vfs_lseek,
    stat as vfs_stat,
    fstat as vfs_fstat,
    mkdir as vfs_mkdir,
    unlink as vfs_unlink,
    rmdir as vfs_rmdir,
    rename as vfs_rename,
    freaddir as vfs_freaddir,
    sync as vfs_sync,
    statfs as vfs_statfs,
    readlink as vfs_readlink,
    symlink as vfs_symlink,
    chmod as vfs_chmod,
    chown as vfs_chown,
    utimes as vfs_utimes,
    xattr_set as vfs_xattr_set,
    xattr_get as vfs_xattr_get,
    xattr_list as vfs_xattr_list,
    xattr_remove as vfs_xattr_remove,
    FileAttr as VfsFileAttr,
    StatFs as VfsStatFs,
};