//! Storage Pool Allocator (SPA) definitions.
//! Handles the Uberblock and Block Pointers (Merkle Tree).

use core::mem::size_of;
use alloc::string::String;
use crate::vdev::Vdev;

/// The size of a block pointer in bytes (typically 128 bytes in ZFS).
pub const BLKPTR_SIZE: usize = 128;

/// Block Pointer (blkptr_t in ZFS).
/// This is the fundamental building block of the Merkle Tree.
/// It points to a block on disk and contains its checksum.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BlockPointer {
    pub vdev_id: u32,       // Virtual Device ID (Simplified from [u32; 3] for now)
    pub offset: u64,        // Physical offset on the VDEV
    pub asize: u32,         // Allocated size (physical size on disk)
    pub padding: u32,
    pub checksum: [u64; 4], // 256-bit Checksum (SHA-256 or Fletcher4)
    pub birth_txg: u64,     // Transaction Group this block was born in
    pub fill_count: u64,    // Number of non-zero blocks under this pointer
    pub padding_end: [u64; 7], // Fill to ~128 bytes
}

impl BlockPointer {
    pub fn new() -> Self {
        Self {
            vdev_id: 0,
            offset: 0,
            asize: 0,
            padding: 0,
            checksum: [0; 4],
            birth_txg: 0,
            fill_count: 0,
            padding_end: [0; 7],
        }
    }

    pub fn is_hole(&self) -> bool {
        self.asize == 0
    }
}

/// The Uberblock (Uberblock_t in ZFS).
/// This is the root of the filesystem state. It is updated atomically
/// in a ring buffer at the beginning of the disk.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Uberblock {
    pub magic: u64,         // NEBULAFS_MAGIC
    pub version: u64,       // Filesystem version
    pub txg: u64,           // Transaction Group Number (Monotonically increasing)
    pub guid_sum: u64,      // Sum of all VDEV GUIDs (to detect missing devices)
    pub timestamp: u64,     // Time when this uberblock was written
    pub rootbp: BlockPointer, // Pointer to the MOS (Meta Object Set)
}

impl Uberblock {
    pub fn new(txg: u64, rootbp: BlockPointer) -> Self {
        Self {
            magic: crate::NEBULAFS_MAGIC,
            version: 1,
            txg,
            guid_sum: 0,
            timestamp: 0,
            rootbp,
        }
    }

    /// Validates the uberblock magic and checksum.
    /// (Checksum logic would depend on how it's stored on disk, usually in a label).
    pub fn verify_magic(&self) -> bool {
        self.magic == crate::NEBULAFS_MAGIC
    }
}

/// Storage Pool Allocator (SPA).
/// The high-level object representing a NebulaFS pool.
pub struct Spa {
    pub name: String,
    pub root_vdev: Vdev,
    pub uberblock: Uberblock,
}

impl Spa {
    pub fn create(name: &str, root_vdev: Vdev) -> Self {
        Self {
            name: String::from(name),
            root_vdev,
            uberblock: Uberblock::new(0, BlockPointer::new()),
        }
    }

    /// Syncs the state to disk (Commits the Transaction Group).
    /// This writes the updated Uberblock to the root VDEV.
    pub fn sync(&mut self) {
        // 1. Advance Transaction Group
        self.uberblock.txg += 1;
        // In a real FS, we would update timestamp and checksum here.
        
        // 2. Serialize Uberblock
        let ub_ptr = &self.uberblock as *const Uberblock as *const u8;
        let ub_slice = unsafe { core::slice::from_raw_parts(ub_ptr, size_of::<Uberblock>()) };

        // 3. Write to VDEV (Using a fixed offset for the label area, e.g., 128KB)
        self.root_vdev.write_block(128 * 1024, ub_slice);
    }

    /// Attempts to find and load an existing SPA from the given VDEV.
    pub fn find(root_vdev: Vdev) -> Option<Self> {
        let ub_size = size_of::<Uberblock>();
        // Read potential Uberblock from the label area (128KB offset)
        let data = root_vdev.read_block(128 * 1024, ub_size);

        if data.len() >= ub_size {
            let ub = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const Uberblock) };
            if ub.verify_magic() {
                return Some(Self {
                    name: String::from("imported"), // In a real FS, name is in MOS
                    root_vdev,
                    uberblock: ub,
                });
            }
        }
        None
    }
}

// Ensure struct sizes match expectations
const _: () = assert!(size_of::<BlockPointer>() <= BLKPTR_SIZE);
const _: () = assert!(size_of::<Uberblock>() == 128 + 8 + 8 + 8 + 8 + 8); // Roughly check size constraints (128 BP + 40 Fields)