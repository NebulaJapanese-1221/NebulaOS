use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum VdevType {
    Disk = 0,      // Physical disk or partition
    File = 1,      // Loopback file
    Mirror = 2,    // n-way mirror
    RaidZ1 = 3,    // Single parity
    RaidZ2 = 4,    // Double parity
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum VdevState {
    Offline = 0,
    Online = 1,
    Degraded = 2,
    Faulted = 3,
    Removed = 4,
}

/// Trait representing a physical block device interface.
pub trait BlockDevice: Send + Sync {
    /// Read `size` bytes from `offset`.
    fn read(&self, offset: u64, size: usize) -> Vec<u8>;
    fn write(&self, offset: u64, data: &[u8]);
}

/// A Virtual Device (VDEV) node in the SPA tree.
/// This represents the configuration and state of a device in the pool.
#[derive(Debug, Clone)]
pub struct Vdev {
    pub id: u64,               // ID within the pool config
    pub guid: u64,             // Unique GUID for this device
    pub type_: VdevType,
    pub state: VdevState,
    pub path: String,          // Device path (e.g. "ata:0:0" or "ram:0")
    pub dev_id: Option<usize>, // Internal handle/index to the physical driver if open
    
    pub asize: u64,            // Allocatable size in bytes
    pub ashift: u8,            // Alignment shift (9=512, 12=4096)
    
    pub children: Vec<Vdev>,   // Children VDEVs (for mirrors/raidz)
    pub parent_id: Option<u64>,

    pub backend: Option<Arc<dyn BlockDevice>>, // Abstract storage backend
}

impl Vdev {
    /// Creates a new leaf VDEV (e.g., a physical disk).
    pub fn new_leaf(id: u64, guid: u64, path: &str, size: u64, ashift: u8) -> Self {
        Self {
            id,
            guid,
            type_: VdevType::Disk,
            state: VdevState::Online, 
            path: String::from(path),
            dev_id: None,
            asize: size,
            ashift,
            children: Vec::new(),
            parent_id: None,
            backend: None,
        }
    }

    /// Creates a new Mirror VDEV wrapping the provided children.
    pub fn new_mirror(id: u64, guid: u64, children: Vec<Vdev>) -> Self {
        // Mirror size is the size of the smallest child
        let size = children.iter().map(|c| c.asize).min().unwrap_or(0);
        // Mirror alignment is the max of children
        let ashift = children.iter().map(|c| c.ashift).max().unwrap_or(9);
        
        Self {
            id,
            guid,
            type_: VdevType::Mirror,
            state: VdevState::Online,
            path: String::from("mirror"),
            dev_id: None,
            asize: size,
            ashift,
            children,
            parent_id: None,
            backend: None,
        }
    }

    /// Simulates reading a block from the device.
    pub fn read_block(&self, offset: u64, size: usize) -> Vec<u8> {
        if self.type_ == VdevType::Mirror {
            // Forward read to the first child (primary)
            if let Some(child) = self.children.first() {
                return child.read_block(offset, size);
            }
        }

        // Use backend if available (e.g. ATA drive)
        if let Some(backend) = &self.backend {
            return backend.read(offset, size);
        }

        // Fallback for missing backend
        alloc::vec![0; size]
    }

    /// Simulates writing a block to the device.
    pub fn write_block(&self, offset: u64, data: &[u8]) {
        if self.type_ == VdevType::Mirror {
            // Write to all children for redundancy
            for child in &self.children {
                child.write_block(offset, data);
            }
        }
        
        if let Some(backend) = &self.backend {
            backend.write(offset, data);
        }
    }
}