//! Virtual File System (VFS) layer.
//! Provides a Unix-like file structure and mounting mechanism.

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use spin::Mutex;
use alloc::collections::BTreeMap;
use nebulafs::vdev::VdevRam;
// In a full implementation, we would import DMU/ZAP layers here.
// For this step, we will wrap the VdevRam to serve as the persistent storage.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Device, // For /dev
}

/// Trait representing an Inode (file or directory) in the VFS.
pub trait INode: Send + Sync {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, String>;
    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize, String>;
    fn lookup(&self, name: &str) -> Result<Arc<dyn INode>, String>;
    fn create(&self, name: &str, file_type: FileType) -> Result<Arc<dyn INode>, String>;
    fn get_type(&self) -> FileType;
    fn size(&self) -> usize;
    fn list(&self) -> Result<Vec<String>, String>;
}

/// A VFS node backed by RAM (simulated filesystem).
pub struct FSNode {
    name: String,
    // In a real implementation, this would hold the Object ID (u64) from NebulaFS
    // and a reference to the DMU.
    // For now, we maintain the tree structure in memory but backing data could be
    // routed to the Vdev.
    data: Mutex<Vec<u8>>, 
    children: Mutex<BTreeMap<String, Arc<NebulaNode>>>,
    file_type: FileType,
}

impl FSNode {
    pub fn new_dir(name: &str) -> Self {
        Self {
            name: String::from(name),
            data: Mutex::new(Vec::new()),
            children: Mutex::new(BTreeMap::new()),
            file_type: FileType::Directory,
        }
    }
    
    pub fn new_file(name: &str) -> Self {
         Self {
            name: String::from(name),
            data: Mutex::new(Vec::new()),
            children: Mutex::new(BTreeMap::new()),
            file_type: FileType::File,
        }
    }
}

impl INode for FSNode {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, String> {
        if self.file_type == FileType::Directory {
            return Err("Is a directory".to_string());
        }
        let data = self.data.lock();
        if offset >= data.len() {
            return Ok(0);
        }
        let read_len = core::cmp::min(buf.len(), data.len() - offset);
        buf[0..read_len].copy_from_slice(&data[offset..offset+read_len]);
        Ok(read_len)
    }

    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize, String> {
        if self.file_type == FileType::Directory {
            return Err("Is a directory".to_string());
        }
        let mut data = self.data.lock();
        if offset + buf.len() > data.len() {
            data.resize(offset + buf.len(), 0);
        }
        data[offset..offset+buf.len()].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn INode>, String> {
        if self.file_type != FileType::Directory {
            return Err("Not a directory".to_string());
        }
        // Handle "." and ".." in a real implementation
        let children = self.children.lock();
        children.get(name).cloned().map(|n| n as Arc<dyn INode>).ok_or("File not found".to_string())
    }

    fn create(&self, name: &str, file_type: FileType) -> Result<Arc<dyn INode>, String> {
        if self.file_type != FileType::Directory {
            return Err("Not a directory".to_string());
        }
        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err("File exists".to_string());
        }
        let node = Arc::new(match file_type {
            FileType::File => FSNode::new_file(name),
            FileType::Directory => FSNode::new_dir(name),
            FileType::Device => FSNode::new_file(name), // Treat as file for now
        });
        children.insert(String::from(name), node.clone());
        Ok(node)
    }
    
    fn get_type(&self) -> FileType {
        self.file_type
    }
    
    fn size(&self) -> usize {
        self.data.lock().len()
    }

    fn list(&self) -> Result<Vec<String>, String> {
         if self.file_type != FileType::Directory {
            return Err("Not a directory".to_string());
        }
        let children = self.children.lock();
        Ok(children.keys().cloned().collect())
    }
}

pub struct FileSystem {
    root: Arc<FSNode>,
    // The underlying storage device for NebulaFS
    _device: Mutex<VdevRam>,
}

impl FileSystem {
    pub fn new() -> Self {
        // Initialize a 64MB RAM disk for NebulaFS
        let ram_disk = VdevRam::new(0, 64 * 1024 * 1024);
        
        Self {
            root: Arc::new(FSNode::new_dir("/")),
            _device: Mutex::new(ram_disk),
        }
    }
    
    pub fn root(&self) -> Arc<dyn INode> {
        self.root.clone()
    }
}

pub static ROOT_FS: Mutex<Option<Arc<FileSystem>>> = Mutex::new(None);

pub fn init() {
    let fs = Arc::new(FileSystem::new());
    
    // Create standard unix directories
    let root = fs.root();
    let _ = root.create("bin", FileType::Directory);
    let _ = root.create("etc", FileType::Directory);
    let _ = root.create("home", FileType::Directory);
    let _ = root.create("dev", FileType::Directory);
    let _ = root.create("tmp", FileType::Directory);

    // Create a default file
    if let Ok(readme) = root.create("README.txt", FileType::File) {
        let text = "Welcome to NebulaOS!\n\nFilesystem: RamFS\nStorage: VdevRam (64MB)";
        let _ = readme.write(0, text.as_bytes());
    }
    
    // Setup /home/user
    if let Ok(home) = root.lookup("home") {
        let _ = home.create("user", FileType::Directory);
        if let Ok(user) = home.lookup("user") {
            if let Ok(notes) = user.create("notes.txt", FileType::File) {
                let _ = notes.write(0, b"RamFS is active.");
            }
        }
    }

    *ROOT_FS.lock() = Some(fs);
}