//! Data Management Unit (DMU).
//! Defines objects (Dnodes) and Object Sets (Objsets).

use crate::spa::BlockPointer;
use crate::vdev::Vdev;
use alloc::vec::Vec;
use core::mem::size_of;

/// Type of object (file, directory, etc.)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    None = 0,
    MasterNode = 1,
    ObjectDirectory = 2,
    PlainFile = 3,
    Directory = 4,
}

/// Data Node (dnode_phys_t in ZFS).
/// Describes an object (file/dataset) and points to its data blocks.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DnodePhys {
    pub object_type: u8,
    pub indirection_levels: u8, // 0 = data blocks are direct, >0 = indirect blocks
    pub nblkptr: u8,            // Number of block pointers used
    pub datablksz: u16,         // Logical block size (in bytes)
    pub bonus_type: u16,        // Type of data in the bonus buffer
    pub blkptr: [BlockPointer; 3], // Pointers to data (only using idx 0 for simplicity now)
    pub bonus: [u8; 64],        // Bonus buffer (e.g., ZPL metadata like permissions/size)
}

/// Object Set (objset_phys_t in ZFS).
/// Represents a filesystem or dataset (a collection of objects).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObjsetPhys {
    pub metadnode: DnodePhys, // The "meta" dnode that describes the object array
    pub zil_header: DnodePhys, // ZFS Intent Log header
    pub type_: u64,           // Dataset type (Filesystem, Snapshot, Volume)
}

impl DnodePhys {
    /// Reads logical data described by this dnode from the VDEV.
    /// Handles reading from the appropriate block pointer(s).
    pub fn read_data(&self, vdev: &Vdev, offset: u64, size: usize) -> Option<Vec<u8>> {
        if self.datablksz == 0 { return None; }
        let block_size = self.datablksz as u64;
        
        let mut result = Vec::new();
        let mut current_offset = offset;
        let mut remaining_size = size;

        while remaining_size > 0 {
            // Identify which logical block contains the data
            let blk_idx = (current_offset / block_size) as usize;
            let blk_offset = (current_offset % block_size) as usize;
            let bytes_to_read = core::cmp::min(block_size as usize - blk_offset, remaining_size);

            if blk_idx >= self.blkptr.len() {
                // Support only direct blocks in small array for now (no indirect block logic yet)
                break; 
            }

            let bp = &self.blkptr[blk_idx];
            if bp.is_hole() {
                // Hole: Append zeros
                result.resize(result.len() + bytes_to_read, 0);
            } else {
                // Read the physical block
                let block_data = vdev.read_block(bp.offset, bp.asize as usize);
                // Copy relevant slice
                if block_data.len() >= blk_offset + bytes_to_read {
                    result.extend_from_slice(&block_data[blk_offset..blk_offset + bytes_to_read]);
                } else {
                    result.resize(result.len() + bytes_to_read, 0); // Read fail fallback
                }
            }

            current_offset += bytes_to_read as u64;
            remaining_size -= bytes_to_read;
        }
        Some(result)
    }
}

impl ObjsetPhys {
    /// Reads a dnode from the Object Set by its ID (index).
    pub fn get_dnode(&self, vdev: &Vdev, id: u64) -> Option<DnodePhys> {
        let dnode_size = size_of::<DnodePhys>() as u64;
        // The metadnode contains the Dnode array as its data
        let data = self.metadnode.read_data(vdev, id * dnode_size, dnode_size as usize)?;
        if data.len() < dnode_size as usize { return None; }
        unsafe { Some(core::ptr::read_unaligned(data.as_ptr() as *const DnodePhys)) }
    }
}