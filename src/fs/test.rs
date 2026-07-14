// Simple tests for NebulaFS

#[cfg(test)]
use super::*;

#[test]
fn test_vdev_creation() {
    use super::vdev::{VDev, VDevType};
    
    let vdev = VDev::new(VDevType::Disk, 1024 * 1024);
    assert_eq!(vdev.vdev_type, VDevType::Disk);
    assert_eq!(vdev.size, 1024 * 1024);
    assert_eq!(vdev.state, super::vdev::VDevState::Unknown);
}

#[test]
fn test_vdev_open_close() {
    use super::vdev::{VDev, VDevType};
    
    let mut vdev = VDev::new(VDevType::Disk, 1024 * 1024);
    assert!(vdev.open().is_ok());
    assert_eq!(vdev.state, super::vdev::VDevState::Online);
    
    assert!(vdev.close().is_ok());
    assert_eq!(vdev.state, super::vdev::VDevState::Offline);
}

#[test]
fn test_vdev_mirror() {
    use super::vdev::{VDev, VDevType};
    
    let child1 = VDev::new(VDevType::Disk, 1024 * 1024);
    let child2 = VDev::new(VDevType::Disk, 1024 * 1024);
    let mirror = VDev::new_mirror(vec![child1, child2]);
    
    assert_eq!(mirror.vdev_type, VDevType::Mirror);
    assert_eq!(mirror.size, 1024 * 1024); // Should be the minimum of children
    assert_eq!(mirror.children.len(), 2);
}

#[test]
fn test_dmu_init() {
    use super::dmu::DMU;
    use super::vdev::{VDev, VDevType};
    
    let vdev = VDev::new(VDevType::Disk, 1024 * 1024);
    let dmu = DMU::init(4096, 256, vdev).unwrap();
    
    assert_eq!(dmu.block_size, 4096);
    assert_eq!(dmu.max_blocks, 256);
    assert_eq!(dmu.used_blocks, 0);
}

#[test]
fn test_dmu_allocate() {
    use super::dmu::DMU;
    use super::vdev::{VDev, VDevType};
    
    let vdev = VDev::new(VDevType::Disk, 1024 * 1024);
    let mut dmu = DMU::init(4096, 256, vdev).unwrap();
    
    let bp = dmu.allocate_block().unwrap();
    assert_eq!(dmu.used_blocks, 1);
    assert_eq!(bp.size, 4096);
}

#[test]
fn test_zio_operations() {
    use super::zio::{IOOperation, IOType, IOPriority, ZIOPipeline};
    use super::vdev::{VDev, VDevType};
    
    let vdev = VDev::new(VDevType::Disk, 1024 * 1024);
    let mut op = IOOperation::new(IOType::Read, IOPriority::SyncRead, vdev, 0, 512);
    
    assert_eq!(op.io_type, IOType::Read);
    assert_eq!(op.priority, IOPriority::SyncRead);
    assert_eq!(op.size, 512);
    
    let mut pipeline = ZIOPipeline::new();
    pipeline.issue(op);
    assert_eq!(pipeline.pending_ops.len(), 1);
}

#[test]
fn test_filesystem_creation() {
    let mut fs = NebulaFS::new("test_pool", 4096, 1024 * 1024);
    assert_eq!(fs.pool_name, "test_pool");
    assert_eq!(fs.block_size, 4096);
    assert_eq!(fs.max_blocks, 1024 * 1024);
}