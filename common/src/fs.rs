pub const FS_SUCCESS: i32 = 0;
pub const FS_ERROR: i32 = -1;
pub const FS_ENOENT: i32 = -2;
pub const FS_EIO: i32 = -3;
pub const FS_EINVAL: i32 = -4;
pub const FS_FLAG_READ: u32 = 0x01;
pub const FS_FLAG_WRITE: u32 = 0x02;
pub const FS_FLAG_CREATE: u32 = 0x04;
pub const FS_MAX_NAME: usize = 255;
pub const FS_TYPE_FILE: u8 = 0x00;
pub const FS_TYPE_DIR: u8 = 0x01;
const FS_NODE_CAPACITY: usize = 16;
const FS_FILE_CAPACITY: usize = 4096;

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

struct FsNode {
    path: [u8; FS_MAX_NAME + 1],
    data: [u8; FS_FILE_CAPACITY],
    size: u32,
    kind: u8,
    used: bool,
}

impl FsNode {
    const fn empty() -> Self {
        Self { path: [0; FS_MAX_NAME + 1], data: [0; FS_FILE_CAPACITY], size: 0, kind: FS_TYPE_FILE, used: false }
    }
}

static mut MOUNTED: bool = false;
static mut NODES: [FsNode; FS_NODE_CAPACITY] = [const { FsNode::empty() }; FS_NODE_CAPACITY];

pub unsafe fn fs_init() {
    MOUNTED = false;
    for node in &mut NODES {
        *node = FsNode::empty();
    }
}

pub unsafe fn fs_mount() -> i32 {
    if MOUNTED {
        return FS_SUCCESS;
    }
    let root = &mut NODES[0];
    root.path[0] = b'/';
    root.kind = FS_TYPE_DIR;
    root.used = true;
    MOUNTED = true;
    FS_SUCCESS
}

pub unsafe fn fs_open(path: *const u8, file: *mut FsFile, flags: u32) -> i32 {
    if !MOUNTED || path.is_null() || file.is_null() {
        return FS_EINVAL;
    }
    let path_length = path_length(path);
    if path_length == 0 || path_length > FS_MAX_NAME {
        return FS_EINVAL;
    }
    let index = match find_node(path, path_length) {
        Some(index) => index,
        None if flags & FS_FLAG_CREATE != 0 => match create_node(path, path_length, FS_TYPE_FILE) {
            Some(index) => index,
            None => return FS_EIO,
        },
        None => return FS_ENOENT,
    };
    let node = &NODES[index];
    if node.kind != FS_TYPE_FILE {
        return FS_EINVAL;
    }
    core::ptr::write_bytes(file, 0, 1);
    copy_path(&mut (*file).name, path, path_length);
    (*file).size = node.size;
    (*file).offset = 0;
    (*file).start_cluster = index as u32 + 1;
    (*file).flags = flags & (FS_FLAG_READ | FS_FLAG_WRITE);
    FS_SUCCESS
}

pub unsafe fn fs_read(file: *mut FsFile, buffer: *mut u8, size: u32) -> i32 {
    if !MOUNTED || file.is_null() || buffer.is_null() || (*file).flags & FS_FLAG_READ == 0 {
        return FS_EINVAL;
    }
    let index = node_index((*file).start_cluster);
    if index >= FS_NODE_CAPACITY || !NODES[index].used || NODES[index].kind != FS_TYPE_FILE {
        return FS_EINVAL;
    }
    let node = &NODES[index];
    if (*file).offset >= node.size {
        return 0;
    }
    let to_read = size.min(node.size - (*file).offset);
    core::ptr::copy_nonoverlapping(node.data.as_ptr().add((*file).offset as usize), buffer, to_read as usize);
    (*file).offset += to_read;
    to_read as i32
}

pub unsafe fn fs_write(file: *mut FsFile, buffer: *const u8, size: u32) -> i32 {
    if !MOUNTED || file.is_null() || buffer.is_null() || (*file).flags & FS_FLAG_WRITE == 0 {
        return FS_EINVAL;
    }
    let index = node_index((*file).start_cluster);
    if index >= FS_NODE_CAPACITY || !NODES[index].used || NODES[index].kind != FS_TYPE_FILE {
        return FS_EINVAL;
    }
    let Some(end) = (*file).offset.checked_add(size) else {
        return FS_EINVAL;
    };
    if end as usize > FS_FILE_CAPACITY {
        return FS_EIO;
    }
    let node = &mut NODES[index];
    core::ptr::copy_nonoverlapping(buffer, node.data.as_mut_ptr().add((*file).offset as usize), size as usize);
    (*file).offset = end;
    node.size = node.size.max(end);
    (*file).size = node.size;
    size as i32
}

pub unsafe fn fs_close(file: *mut FsFile) -> i32 {
    if file.is_null() {
        return FS_EINVAL;
    }
    (*file).start_cluster = 0;
    (*file).size = 0;
    (*file).offset = 0;
    (*file).flags = 0;
    FS_SUCCESS
}

pub unsafe fn fs_mkdir(path: *const u8) -> i32 {
    if !MOUNTED || path.is_null() {
        return FS_EINVAL;
    }
    let length = path_length(path);
    if length <= 1 || length > FS_MAX_NAME {
        return FS_EINVAL;
    }
    if find_node(path, length).is_some() {
        return FS_EINVAL;
    }
    if create_node(path, length, FS_TYPE_DIR).is_some() { FS_SUCCESS } else { FS_EIO }
}

pub unsafe fn fs_list(path: *const u8, dir: *mut FsDir) -> i32 {
    if !MOUNTED || path.is_null() || dir.is_null() || (*dir).entries.is_null() {
        return FS_EINVAL;
    }
    let path_len = path_length(path);
    if path_len == 0 || path_len > FS_MAX_NAME {
        return FS_EINVAL;
    }
    let directory = match find_node(path, path_len) {
        Some(index) if NODES[index].kind == FS_TYPE_DIR => index,
        _ => return FS_ENOENT,
    };
    let capacity = (*dir).count as usize;
    let mut written = 0usize;
    for index in 0..FS_NODE_CAPACITY {
        if index == directory || !NODES[index].used || !is_direct_child(path, path_len, &NODES[index].path) {
            continue;
        }
        if written == capacity {
            break;
        }
        let entry = &mut *(*dir).entries.add(written);
        core::ptr::write_bytes(entry, 0, 1);
        let name_start = child_name_start(path_len);
        let name_length = path_length(NODES[index].path.as_ptr().add(name_start));
        copy_path(&mut entry.name, NODES[index].path.as_ptr().add(name_start), name_length);
        entry.size = NODES[index].size;
        entry.r#type = NODES[index].kind;
        written += 1;
    }
    (*dir).count = written as u32;
    (*dir).position = 0;
    FS_SUCCESS
}

unsafe fn path_length(path: *const u8) -> usize {
    let mut length = 0;
    while length <= FS_MAX_NAME && *path.add(length) != 0 {
        length += 1;
    }
    length
}

unsafe fn find_node(path: *const u8, length: usize) -> Option<usize> {
    for index in 0..FS_NODE_CAPACITY {
        if NODES[index].used && NODES[index].path[..length] == core::slice::from_raw_parts(path, length)[..] {
            return Some(index);
        }
    }
    None
}

unsafe fn create_node(path: *const u8, length: usize, kind: u8) -> Option<usize> {
    for index in 1..FS_NODE_CAPACITY {
        if !NODES[index].used {
            let node = &mut NODES[index];
            node.path = [0; FS_MAX_NAME + 1];
            copy_path(&mut node.path, path, length);
            node.data = [0; FS_FILE_CAPACITY];
            node.size = 0;
            node.kind = kind;
            node.used = true;
            return Some(index);
        }
    }
    None
}

unsafe fn copy_path(destination: &mut [u8; FS_MAX_NAME + 1], source: *const u8, length: usize) {
    core::ptr::copy_nonoverlapping(source, destination.as_mut_ptr(), length);
    destination[length] = 0;
}

fn node_index(handle: u32) -> usize {
    handle.saturating_sub(1) as usize
}

unsafe fn is_direct_child(parent: *const u8, parent_length: usize, child: &[u8; FS_MAX_NAME + 1]) -> bool {
    if parent_length == 1 && *parent == b'/' {
        if child[0] != b'/' {
            return false;
        }
        return !child[1..].contains(&b'/');
    }
    if child[..parent_length] != core::slice::from_raw_parts(parent, parent_length)[..]
        || child[parent_length] != b'/'
    {
        return false;
    }
    !child[parent_length + 1..].contains(&b'/')
}

fn child_name_start(parent_length: usize) -> usize {
    if parent_length == 1 { 1 } else { parent_length + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_filesystem_round_trips_data_and_lists_files() {
        unsafe {
            fs_init();
            assert_eq!(fs_mount(), FS_SUCCESS);
            let path = b"/note.txt\0";
            let mut file = core::mem::MaybeUninit::<FsFile>::uninit();
            assert_eq!(fs_open(path.as_ptr(), file.as_mut_ptr(), FS_FLAG_WRITE | FS_FLAG_CREATE), FS_SUCCESS);
            let mut file = file.assume_init();
            let content = b"NebulaOS";
            assert_eq!(fs_write(&mut file, content.as_ptr(), content.len() as u32), content.len() as i32);
            assert_eq!(fs_close(&mut file), FS_SUCCESS);
            assert_eq!(fs_open(path.as_ptr(), &mut file, FS_FLAG_READ), FS_SUCCESS);
            let mut buffer = [0u8; 16];
            assert_eq!(fs_read(&mut file, buffer.as_mut_ptr(), buffer.len() as u32), content.len() as i32);
            assert_eq!(&buffer[..content.len()], content);

            let mut entries = core::array::from_fn::<_, 4, _>(|_| core::mem::MaybeUninit::<FsDirent>::uninit());
            let mut directory = FsDir { entries: entries.as_mut_ptr() as *mut FsDirent, count: 4, position: 0, internal: core::ptr::null_mut() };
            assert_eq!(fs_list(b"/\0".as_ptr(), &mut directory), FS_SUCCESS);
            assert_eq!(directory.count, 1);
            assert_eq!(&(*directory.entries).name[..9], b"note.txt\0");
        }
    }
}
