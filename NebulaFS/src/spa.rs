//! Storage Pool Allocator (SPA) definitions.
//! Handles the Uberblock and Block Pointers (Merkle Tree).

use core::mem::size_of;

/// The size of a block pointer in bytes (typically 128 bytes in ZFS).
pub const BLKPTR_SIZE: usize = 128;

/// Block Pointer (blkptr_t in ZFS).
/// This is the fundamental building block of the Merkle Tree.
/// It points to a block on disk and contains its checksum.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BlockPointer {
    pub vdevs: [u32; 3],    // Virtual Device IDs (support for up to 3 copies/ditto blocks)
    pub grid: u8,           // RAID-Z layout information
    pub asize: u32,         // Allocated size (physical size on disk)
    pub padding: u64,
    pub checksum: [u64; 4], // 256-bit Checksum (SHA-256 or Fletcher4)
    pub birth_txg: u64,     // Transaction Group this block was born in
    pub fill_count: u64,    // Number of non-zero blocks under this pointer
}

impl BlockPointer {
    pub fn new() -> Self {
        Self {
            vdevs: [0; 3],
            grid: 0,
            asize: 0,
            padding: 0,
            checksum: [0; 4],
            birth_txg: 0,
            fill_count: 0,
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

// Ensure struct sizes match expectations
const _: () = assert!(size_of::<BlockPointer>() <= BLKPTR_SIZE);
const _: () = assert!(size_of::<Uberblock>() == 128 + 8 + 8 + 8 + 8 + 8 + 4); // Roughly check size constraints