#![no_std]

extern crate alloc;

//! NebulaFS - A ZFS-inspired Copy-On-Write Filesystem
//! 
//! Core Concepts:
//! - SPA (Storage Pool Allocator): Manages storage devices (VDEVs).
//! - DMU (Data Management Unit): Presents a transactional object model.
//! - ZAP (ZFS Attribute Processor): Stores properties/directories as keys/values.

pub mod spa;
pub mod dmu;
pub mod vdev;
pub mod zap;

/// The Magic Number for NebulaFS (ASCII 'NBFS' + high bit or similar)
/// ZFS uses different endian magics, we'll stick to one for now.
pub const NEBULAFS_MAGIC: u64 = 0x00_4E_42_46_53_00_00_01; 

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum Endianness {
    Big = 0x00_00_00_00_00_00_00_00,
    Little = 0x01_00_00_00_00_00_00_00,
}

// Note: This filesystem is in early stages of development.