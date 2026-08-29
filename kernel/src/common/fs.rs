use crate::common::stdint::*;

pub const FS_SUCCESS: i32 = 0;
pub const FS_ERROR: i32 = -1;
pub const FS_ENOENT: i32 = -2;
pub const FS_EIO: i32 = -3;
pub const FS_FLAG_READ: u32 = 0x01;
pub const FS_FLAG_WRITE: u32 = 0x02;
pub const FS_MAX_NAME: usize = 255;
pub const FS_TYPE_FILE: u8 = 0x00;
pub const FS_TYPE_DIR: u8 = 0x01;

#[repr(C)]
pub struct FsFile {
    pub name: [u8; FS_MAX_NAME + 1],
    pub size: u32,
    pub offset: u32,
    pub start_cluster: u32,
    pub flags: u32,
}

#[repr(C)]
pub struct FsDirent {
    pub name: [u8; FS_MAX_NAME + 1],
    pub size: u32,
    pub r#type: u8,
}

#[repr(C)]
pub struct FsDir {
    pub entries: *mut FsDirent,
    pub count: u32,
    pub position: u32,
    pub internal: *mut u8,
}

static mut MOUNTED: bool = false;
static mut ROOT_CLUSTER: u32 = 0;

pub unsafe fn fs_init() {
    MOUNTED = false;
}

pub unsafe fn fs_mount() -> i32 {
    if MOUNTED {
        return FS_SUCCESS;
    }
    ROOT_CLUSTER = 2;
    MOUNTED = true;
    FS_SUCCESS
}

pub unsafe fn fs_open(path: *const u8, file: *mut FsFile, flags: u32) -> i32 {
    if !MOUNTED {
        return FS_ERROR;
    }
    (*file).size = 0;
    (*file).start_cluster = ROOT_CLUSTER;
    (*file).offset = 0;
    (*file).flags = flags;
    FS_SUCCESS
}

pub unsafe fn fs_read(file: *mut FsFile, buffer: *mut u8, size: u32) -> i32 {
    if !MOUNTED || file.is_null() || buffer.is_null() {
        return FS_ERROR;
    }
    if (*file).offset >= (*file).size {
        return 0;
    }
    let mut to_read = size;
    if (*file).offset + to_read > (*file).size {
        to_read = (*file).size - (*file).offset;
    }
    core::ptr::write_bytes(buffer, 0, to_read as usize);
    (*file).offset += to_read;
    to_read as i32
}

pub unsafe fn fs_write(_file: *mut FsFile, _buffer: *const u8, _size: u32) -> i32 {
    FS_ERROR
}

pub unsafe fn fs_close(_file: *mut FsFile) -> i32 {
    FS_SUCCESS
}

pub unsafe fn fs_list(_path: *const u8, _dir: *mut FsDir) -> i32 {
    FS_ERROR
}
