//! Storage Pool Allocator (SPA) definitions.
//! Handles the Uberblock and Block Pointers (Merkle Tree).

use core::mem::size_of;
use alloc::string::{String, ToString};
use crate::vdev::Vdev;

/// The size of a block pointer in bytes (typically 128 bytes in ZFS).
pub const BLKPTR_SIZE: usize = 128;

/// The offset where the VDEV labels start on a disk (skipping the 8KB boot header).
pub const VDEV_LABEL_OFFSET: u64 = 8 * 1024;
/// The offset where the Uberblock array starts within the label area.
pub const VDEV_UBERBLOCK_OFFSET: u64 = 128 * 1024;

/// Block Pointer (blkptr_t in ZFS).
/// This is the fundamental building block of the Merkle Tree.
/// It points to a block on disk and contains its checksum.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BlockPointer {
    pub vdev_id: u32,       // Virtual Device ID (Simplified from [u32; 3] for now)
    pub pad0: u32,          // Explicit padding for 8-byte alignment on 32-bit systems
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
            pad0: 0,
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

/// The VDEV Label (vdev_label_t in ZFS).
/// This describes the configuration of the pool and the VDEV tree.
/// It is stored at the beginning and end of every leaf VDEV.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VdevLabel {
    pub magic: u64,              // NEBULAFS_MAGIC
    pub pool_guid: u64,          // Unique ID for the entire pool
    pub vdev_guid: u64,          // Unique ID for this specific VDEV
    pub pool_name: [u8; 32],      // ASCII name of the pool
    pub vdev_tree_len: u64,      // Length of the serialized VDEV tree in vdev_tree_data
    pub vdev_tree_data: [u8; 4096], // Serialized VDEV tree structure
}

impl VdevLabel {
    pub fn new(pool_name: &str, pool_guid: u64, vdev_guid: u64) -> Self {
        let mut name = [0u8; 32];
        let bytes = pool_name.as_bytes();
        let len = bytes.len().min(32);
        name[..len].copy_from_slice(&bytes[..len]);

        Self {
            magic: crate::NEBULAFS_MAGIC,
            pool_guid,
            vdev_guid,
            pool_name: name,
            vdev_tree_len: 0,
            vdev_tree_data: [0; 4096],
        }
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

        // 2. Prepare and write the VDEV Label (contains the pool configuration tree)
        let mut label = VdevLabel::new(self.name.as_str(), self.uberblock.guid_sum, self.root_vdev.guid);
        self.serialize_vdev_tree(&mut label);
        
        let label_ptr = &label as *const VdevLabel as *const u8;
        let label_slice = unsafe { core::slice::from_raw_parts(label_ptr, size_of::<VdevLabel>()) };
        self.root_vdev.write_block(VDEV_LABEL_OFFSET, label_slice);

        // 3. Serialize and write the Uberblock
        let ub_ptr = &self.uberblock as *const Uberblock as *const u8;
        let ub_slice = unsafe { core::slice::from_raw_parts(ub_ptr, size_of::<Uberblock>()) };
        self.root_vdev.write_block(VDEV_UBERBLOCK_OFFSET, ub_slice);
    }

    /// Attempts to find and load an existing SPA from the given VDEV.
    pub fn find(mut root_vdev: Vdev) -> Option<Self> {
        // 1. Read and verify VDEV Label
        let label_size = size_of::<VdevLabel>();
        let label_data = root_vdev.read_block(VDEV_LABEL_OFFSET, label_size);
        if label_data.len() < label_size { return None; }
        
        let label = unsafe { core::ptr::read_unaligned(label_data.as_ptr() as *const VdevLabel) };
        if label.magic != crate::NEBULAFS_MAGIC { return None; }

        // 2. Reconstruct the VDEV tree from the label
        let mut reconstructed_root = Self::deserialize_vdev_tree(&label)?;
        
        // Re-attach the physical backend to the reconstructed root (simplified for single-disk)
        if reconstructed_root.children.is_empty() {
            reconstructed_root.backend = root_vdev.backend.take();
        }

        // 3. Read and verify Uberblock
        let ub_size = size_of::<Uberblock>();
        let data = reconstructed_root.read_block(VDEV_UBERBLOCK_OFFSET, ub_size);

        if data.len() >= ub_size {
            let ub = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const Uberblock) };
            
            // Verify magic and ensure the Uberblock belongs to this pool via GUID sum
            if ub.verify_magic() && ub.guid_sum == label.pool_guid {
                // Extract the pool name from the label's fixed-size array
                let name_bytes = &label.pool_name;
                let len = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
                let name = core::str::from_utf8(&name_bytes[..len]).unwrap_or("imported").to_string();

                return Some(Self {
                    name,
                    root_vdev: reconstructed_root,
                    uberblock: ub,
                });
            }
        }
        None
    }

    /// Serializes the current VDEV tree into the provided VDEV Label.
    pub fn serialize_vdev_tree(&self, label: &mut VdevLabel) {
        let mut offset = 0;
        self.serialize_vdev_recursive(&self.root_vdev, &mut label.vdev_tree_data, &mut offset);
        label.vdev_tree_len = offset as u64;
    }

    fn serialize_vdev_recursive(&self, vdev: &Vdev, buffer: &mut [u8], offset: &mut usize) {
        // Node header: Type(8) + GUID(8) + Ashift(1) + ChildCount(8) = 25 bytes.
        // We check for 32 bytes to be safe for alignment or future fields.
        if *offset + 32 > buffer.len() { return; }

        // Write VdevType (u64)
        let vtype = vdev.type_ as u64;
        buffer[*offset..*offset + 8].copy_from_slice(&vtype.to_le_bytes());
        *offset += 8;

        // Write GUID (u64)
        buffer[*offset..*offset + 8].copy_from_slice(&vdev.guid.to_le_bytes());
        *offset += 8;

        // Write Ashift (u8)
        buffer[*offset] = vdev.ashift;
        *offset += 1;

        // Write Children Count (u64)
        let count = vdev.children.len() as u64;
        buffer[*offset..*offset + 8].copy_from_slice(&count.to_le_bytes());
        *offset += 8;

        // Recurse through children
        for child in &vdev.children {
            self.serialize_vdev_recursive(child, buffer, offset);
        }
    }

    /// Reconstructs a VDEV tree from the serialized data in a VDEV Label.
    pub fn deserialize_vdev_tree(label: &VdevLabel) -> Option<Vdev> {
        let mut offset = 0;
        Self::deserialize_vdev_recursive(&label.vdev_tree_data[..label.vdev_tree_len as usize], &mut offset)
    }

    fn deserialize_vdev_recursive(buffer: &[u8], offset: &mut usize) -> Option<Vdev> {
        // Ensure we have at least the header (25 bytes)
        if *offset + 25 > buffer.len() { return None; }

        // Read VdevType
        let vtype_raw = u64::from_le_bytes(buffer[*offset..*offset + 8].try_into().ok()?);
        *offset += 8;
        let vtype = match vtype_raw {
            0 => crate::vdev::VdevType::Disk,
            1 => crate::vdev::VdevType::File,
            2 => crate::vdev::VdevType::Mirror,
            3 => crate::vdev::VdevType::RaidZ1,
            4 => crate::vdev::VdevType::RaidZ2,
            5 => crate::vdev::VdevType::RaidZ3,
            6 => crate::vdev::VdevType::Spare,
            7 => crate::vdev::VdevType::Log,
            8 => crate::vdev::VdevType::Cache,
            9 => crate::vdev::VdevType::Root,
            _ => return None,
        };

        let guid = u64::from_le_bytes(buffer[*offset..*offset + 8].try_into().ok()?);
        *offset += 8;

        let ashift = buffer[*offset];
        *offset += 1;

        let child_count = u64::from_le_bytes(buffer[*offset..*offset + 8].try_into().ok()?);
        *offset += 8;

        let mut children = alloc::vec::Vec::new();
        for _ in 0..child_count {
            children.push(Self::deserialize_vdev_recursive(buffer, offset)?);
        }

        Some(Vdev {
            id: 0, // Positional IDs can be assigned after reconstruction
            guid,
            type_: vtype,
            state: crate::vdev::VdevState::Online,
            path: String::from("imported"),
            dev_id: None,
            asize: 0, // Size should be recalculated based on children
            ashift,
            children,
            parent_id: None,
            backend: None,
        })
    }
}

// Ensure struct sizes match expectations
const _: () = assert!(size_of::<BlockPointer>() == BLKPTR_SIZE);
const _: () = assert!(size_of::<Uberblock>() >= 168); // Check size constraints (128 BP + 40 Fields)