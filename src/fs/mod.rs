pub mod vdev; // Virtual device management
pub mod dmu;  // Data Management Unit (block allocation)
pub mod zio;  // ZFS I/O pipeline
pub mod spa;  // Storage Pool Allocator
pub mod zpl;  // ZFS POSIX Layer (file system interface)
pub mod arc;  // Adaptive Replacement Cache
#[cfg(test)]
mod test;

use alloc::string::String;
use alloc::vec::Vec;

/// Main NebulaFS structure
pub struct NebulaFS {
    pub pool_name: String,
    pub root_vdev: vdev::VDev,
    pub block_size: u64,
    pub max_blocks: u64,
}

impl NebulaFS {
    /// Create a new NebulaFS instance
    pub fn new(pool_name: &str, block_size: u64, max_blocks: u64) -> Self {
        NebulaFS {
            pool_name: pool_name.to_string(),
            root_vdev: vdev::VDev::new(vdev::VDevType::Disk, max_blocks * block_size),
            block_size,
            max_blocks,
        }
    }

    /// Mount the file system
    pub fn mount(&mut self) -> Result<(), &'static str> {
        // Initialize the storage pool
        spa::init_pool(&mut self.root_vdev, self.block_size, self.max_blocks)?;
        
        // Initialize the DMU
        dmu::init_dmu(self.block_size, self.max_blocks)?;
        
        Ok(())
    }

    /// Unmount the file system
    pub fn unmount(&self) -> Result<(), &'static str> {
        // Sync all pending writes
        dmu::sync_all()?;
        
        Ok(())
    }

    /// Create a snapshot of the current file system state
    pub fn snapshot(&self, name: &str) -> Result<(), &'static str> {
        dmu::create_snapshot(name)?;
        Ok(())
    }

    /// Rollback to a previous snapshot
    pub fn rollback(&self, name: &str) -> Result<(), &'static str> {
        dmu::rollback_to_snapshot(name)?;
        Ok(())
    }
}

/// File system operations trait
pub trait FileSystemOps {
    fn read(&self, inode: u64, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str>;
    fn write(&mut self, inode: u64, offset: u64, data: &[u8]) -> Result<usize, &'static str>;
    fn create_file(&mut self, parent_inode: u64, name: &str) -> Result<u64, &'static str>;
    fn create_dir(&mut self, parent_inode: u64, name: &str) -> Result<u64, &'static str>;
    fn lookup(&self, parent_inode: u64, name: &str) -> Result<u64, &'static str>;
    fn link(&mut self, inode: u64, parent_inode: u64, name: &str) -> Result<(), &'static str>;
    fn unlink(&mut self, parent_inode: u64, name: &str) -> Result<(), &'static str>;
}

impl FileSystemOps for NebulaFS {
    fn read(&self, inode: u64, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> {
        zpl::read_file(self, inode, offset, buffer)
    }

    fn write(&mut self, inode: u64, offset: u64, data: &[u8]) -> Result<usize, &'static str> {
        zpl::write_file(self, inode, offset, data)
    }

    fn create_file(&mut self, parent_inode: u64, name: &str) -> Result<u64, &'static str> {
        zpl::create_file(self, parent_inode, name)
    }

    fn create_dir(&mut self, parent_inode: u64, name: &str) -> Result<u64, &'static str> {
        zpl::create_dir(self, parent_inode, name)
    }

    fn lookup(&self, parent_inode: u64, name: &str) -> Result<u64, &'static str> {
        zpl::lookup(self, parent_inode, name)
    }

    fn link(&mut self, inode: u64, parent_inode: u64, name: &str) -> Result<(), &'static str> {
        zpl::link_file(self, inode, parent_inode, name)
    }

    fn unlink(&mut self, parent_inode: u64, name: &str) -> Result<(), &'static str> {
        zpl::unlink_file(self, parent_inode, name)
    }
}