use crate::fs::NebulaFS;
use crate::fs::dmu::{Object, ObjectType};
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;

/// Inode structure
#[derive(Debug, Clone)]
pub struct Inode {
    pub ino: u64,           // Inode number
    pub obj_id: u64,        // Object ID in the DMU
    pub mode: u32,          // File mode (permissions, type)
    pub uid: u32,           // User ID
    pub gid: u32,           // Group ID
    pub size: u64,          // File size
    pub atime: u64,         // Access time
    pub mtime: u64,         // Modification time
    pub ctime: u64,         // Change time
    pub nlink: u32,         // Number of hard links
}

impl Inode {
    pub fn new(ino: u64, obj_id: u64, mode: u32) -> Self {
        Inode {
            ino,
            obj_id,
            mode,
            uid: 0,
            gid: 0,
            size: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            nlink: 1,
        }
    }

    pub fn is_dir(&self) -> bool {
        (self.mode & 0o170000) == 0o040000
    }

    pub fn is_file(&self) -> bool {
        (self.mode & 0o170000) == 0o100000
    }
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub ino: u64,           // Inode number
    pub name: String,       // Entry name
    pub name_len: u8,       // Name length
    pub type_indicator: u8, // File type indicator
}

impl DirEntry {
    pub fn new(ino: u64, name: &str, type_indicator: u8) -> Self {
        DirEntry {
            ino,
            name: name.to_string(),
            name_len: name.len() as u8,
            type_indicator,
        }
    }
}

/// File system superblock
#[derive(Debug)]
pub struct Superblock {
    pub magic: u32,         // Magic number
    pub version: u32,       // File system version
    pub block_size: u64,    // Block size
    pub root_ino: u64,      // Root inode
    pub pool_name: String,   // Storage pool name
}

impl Superblock {
    pub fn new(pool_name: &str, block_size: u64) -> Self {
        Superblock {
            magic: 0x5a465342, // "ZFSB" in ASCII
            version: 1,
            block_size,
            root_ino: 2, // Traditional root inode number
            pool_name: pool_name.to_string(),
        }
    }
}

/// Directory structure
#[derive(Debug)]
pub struct Directory {
    pub entries: BTreeMap<String, u64>, // Map names to inode numbers
}

impl Directory {
    pub fn new() -> Self {
        Directory {
            entries: BTreeMap::new(),
        }
    }

    pub fn add_entry(&mut self, name: &str, inode: u64) {
        self.entries.insert(name.to_string(), inode);
    }

    pub fn remove_entry(&mut self, name: &str) -> Option<u64> {
        self.entries.remove(name)
    }

    pub fn lookup(&self, name: &str) -> Option<u64> {
        self.entries.get(name).copied()
    }
}

/// File system state
#[derive(Debug)]
pub struct FileSystemState {
    pub superblock: Superblock,
    pub inodes: BTreeMap<u64, Inode>, // Inode cache
    pub directories: BTreeMap<u64, Directory>, // Directory cache
    pub next_inode: u64, // Next available inode number
}

impl FileSystemState {
    pub fn new(pool_name: &str, block_size: u64) -> Self {
        let mut fs = FileSystemState {
            superblock: Superblock::new(pool_name, block_size),
            inodes: BTreeMap::new(),
            directories: BTreeMap::new(),
            next_inode: 100, // Start inodes at 100
        };
        
        // Create root directory
        fs.create_root_directory();
        fs
    }

    fn create_root_directory(&mut self) {
        // Create root inode
        let root_inode = Inode::new(
            2, // Root inode number
            2, // Object ID
            0o040755 | 0o040000, // Directory with rwxr-xr-x permissions
        );
        self.inodes.insert(2, root_inode);
        
        // Create root directory
        let mut root_dir = Directory::new();
        root_dir.add_entry(".", 2); // Self reference
        root_dir.add_entry("..", 2); // Parent reference (root's parent is itself)
        self.directories.insert(2, root_dir);
    }

    pub fn create_inode(&mut self, mode: u32) -> u64 {
        let inode_num = self.next_inode;
        self.next_inode += 1;
        
        let inode = Inode::new(inode_num, inode_num, mode);
        self.inodes.insert(inode_num, inode);
        inode_num
    }

    pub fn get_object(&self, _obj_id: u64) -> Option<&Object> {
        None
    }

    pub fn get_object_mut(&mut self, _obj_id: u64) -> Option<&mut Object> {
        None
    }

    pub fn create_object(&mut self, inode_num: u64) -> Object {
        let mut obj = Object::new(inode_num);
        if let Some(inode) = self.inodes.get(&inode_num) {
            if inode.is_dir() {
                obj.set_type(ObjectType::Directory);
            }
        }
        obj
    }
}
    
/// File system operations
pub fn read_file(fs: &NebulaFS, inode: u64, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> {
    let state = fs.get_state();
    let inode_data = state.inodes.get(&inode)
        .ok_or("Inode not found")?;
    
    if !inode_data.is_file() {
        return Err("Not a regular file");
    }
    
    let file_size = inode_data.size;
    let bytes_to_read = buffer.len().min((file_size.saturating_sub(offset)) as usize);
    
    if bytes_to_read == 0 {
        return Ok(0);
    }

    let read_size = bytes_to_read.min(buffer.len());
    buffer[..read_size].fill(0);
    Ok(read_size)
}

pub fn write_file(fs: &mut NebulaFS, inode: u64, offset: u64, data: &[u8]) -> Result<usize, &'static str> {
    let state = fs.get_state_mut();
    let inode_data = state.inodes.get_mut(&inode)
        .ok_or("Inode not found")?;
    
    if !inode_data.is_file() {
        return Err("Not a regular file");
    }
    
    let bytes_written = data.len();
    let new_size = offset + bytes_written as u64;
    if new_size > inode_data.size {
        inode_data.size = new_size;
    }
    
    Ok(bytes_written)
}

pub fn create_file(fs: &mut NebulaFS, parent_inode: u64, name: &str) -> Result<u64, &'static str> {
    let (_parent_exists, parent_is_dir, parent_ino) = {
        let state = fs.get_state();
        let parent = state.inodes.get(&parent_inode).ok_or("Parent inode not found")?;
        (true, parent.is_dir(), parent.ino)
    };
    
    if !parent_is_dir {
        return Err("Parent is not a directory");
    }
    
    {
        let state = fs.get_state();
        let parent_dir = state.directories.get(&parent_ino).ok_or("Parent directory not found")?;
        if parent_dir.lookup(name).is_some() {
            return Err("File already exists");
        }
    }
    
    let state = fs.get_state_mut();
    let inode_num = state.create_inode(0o100644 | 0o100000);
    
    if let Some(dir) = state.directories.get_mut(&parent_ino) {
        dir.add_entry(name, inode_num);
    }
    
    Ok(inode_num)
}

pub fn create_dir(fs: &mut NebulaFS, parent_inode: u64, name: &str) -> Result<u64, &'static str> {
    let (_parent_exists, parent_is_dir, parent_ino) = {
        let state = fs.get_state();
        let parent = state.inodes.get(&parent_inode).ok_or("Parent inode not found")?;
        (true, parent.is_dir(), parent.ino)
    };
    
    if !parent_is_dir {
        return Err("Parent is not a directory");
    }
    
    {
        let state = fs.get_state();
        let parent_dir = state.directories.get(&parent_ino).ok_or("Parent directory not found")?;
        if parent_dir.lookup(name).is_some() {
            return Err("Directory already exists");
        }
    }
    
    let state = fs.get_state_mut();
    let inode_num = state.create_inode(0o040755 | 0o040000);
    
    let mut new_dir = Directory::new();
    new_dir.add_entry(".", inode_num);
    new_dir.add_entry("..", parent_ino);
    state.directories.insert(inode_num, new_dir);
    
    if let Some(dir) = state.directories.get_mut(&parent_ino) {
        dir.add_entry(name, inode_num);
    }
    
    Ok(inode_num)
}

pub fn lookup(fs: &NebulaFS, parent_inode: u64, name: &str) -> Result<u64, &'static str> {
    let state = fs.get_state();
    let parent = state.inodes.get(&parent_inode)
        .ok_or("Parent inode not found")?;
    
    if !parent.is_dir() {
        return Err("Parent is not a directory");
    }
    
    let parent_dir = state.directories.get(&parent.ino)
        .ok_or("Parent directory not found")?;
    parent_dir.lookup(name)
        .ok_or("File not found")
}

pub fn link_file(fs: &mut NebulaFS, inode: u64, parent_inode: u64, name: &str) -> Result<(), &'static str> {
    let state = fs.get_state_mut();
    
    if state.inodes.get(&inode).is_none() {
        return Err("Source inode not found");
    }
    
    let parent = state.inodes.get(&parent_inode)
        .ok_or("Parent inode not found")?;
    
    if !parent.is_dir() {
        return Err("Parent is not a directory");
    }
    
    let parent_dir = state.directories.get(&parent.ino)
        .ok_or("Parent directory not found")?;
    
    if parent_dir.lookup(name).is_some() {
        return Err("File already exists");
    }
    
    if let Some(dir) = state.directories.get_mut(&parent.ino) {
        dir.add_entry(name, inode);
    }
    
    if let Some(inode_data) = state.inodes.get_mut(&inode) {
        inode_data.nlink += 1;
    }
    Ok(())
}

pub fn unlink_file(fs: &mut NebulaFS, parent_inode: u64, name: &str) -> Result<(), &'static str> {
    let state = fs.get_state_mut();
    
    let parent = state.inodes.get(&parent_inode)
        .ok_or("Parent inode not found")?;
    
    if !parent.is_dir() {
        return Err("Parent is not a directory");
    }
    
    let parent_dir = state.directories.get_mut(&parent.ino)
        .ok_or("Parent directory not found")?;
    
    let inode_num = parent_dir.lookup(name)
        .ok_or("File not found")?;
    
    parent_dir.remove_entry(name);
    
    if let Some(inode_data) = state.inodes.get_mut(&inode_num) {
        inode_data.nlink -= 1;
        if inode_data.nlink == 0 {
            state.inodes.remove(&inode_num);
            state.directories.remove(&inode_num);
        }
    }
    Ok(())
}

/// Initialize the ZPL layer
pub fn init_zpl(fs: &mut NebulaFS) -> Result<(), &'static str> {
    let pool_name = &fs.pool_name;
    let block_size = fs.block_size;
    let state = FileSystemState::new(pool_name, block_size);
    fs.set_state(state);
    Ok(())
}
