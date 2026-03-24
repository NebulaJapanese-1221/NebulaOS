//! Data Management Unit (DMU).
//! Defines objects (Dnodes) and Object Sets (Objsets).

use crate::spa::BlockPointer;

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
    pub bonus_type: u8,         // Type of data in the bonus buffer
    pub blkptr: [BlockPointer; 3], // Pointers to data (triple redundancy max)
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