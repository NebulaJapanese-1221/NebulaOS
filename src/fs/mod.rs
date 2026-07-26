use alloc::string::{String, ToString};
use crate::fs::zpl::FileSystemState;
use crate::fs::vdev::VDev;
use crate::fs::dmu::DMU;
use crate::fs::zio::ZIOPipeline;
use crate::fs::arc::ARCCache;
use crate::fs::checksum::ChecksumAlgorithm;
use crate::fs::journal::Journal;

pub mod vdev;
pub mod dmu;
pub mod zio;
pub mod spa;
pub mod zpl;
pub mod arc;
pub mod checksum;
pub mod journal;
pub mod vfs;

#[cfg(test)]
mod test;

#[derive(Debug)]
pub struct NebulaFS {
    pub pool_name: String,
    pub root_vdev: VDev,
    pub block_size: u64,
    pub max_blocks: u64,
    pub dmu: Option<DMU>,
    pub zio: Option<ZIOPipeline>,
    pub arc: Option<ARCCache>,
    pub state: Option<FileSystemState>,
    pub checksum_alg: ChecksumAlgorithm,
    pub journal: Option<Journal>,
}

impl NebulaFS {
    pub fn new(pool_name: &str, block_size: u64, max_blocks: u64) -> Self {
        NebulaFS {
            pool_name: pool_name.to_string(),
            root_vdev: VDev::new(vdev::VDevType::Disk, max_blocks * block_size),
            block_size,
            max_blocks,
            dmu: None,
            zio: None,
            arc: None,
            state: None,
            checksum_alg: ChecksumAlgorithm::Fletcher4,
            journal: None,
        }
    }

    pub fn mount(&mut self) -> Result<(), &'static str> {
        spa::init_pool(&mut self.root_vdev, self.block_size, self.max_blocks)?;
        let dmu = dmu::DMU::init(self.block_size, self.max_blocks, self.root_vdev.clone(), dmu::CompressionType::ZLE)?;
        self.dmu = Some(dmu);
        let zio = zio::ZIOPipeline::new();
        self.zio = Some(zio);
        let arc = arc::ARCCache::new(16 * 1024 * 1024);
        self.arc = Some(arc);
        let journal = journal::Journal::new(1000);
        self.journal = Some(journal);
        if let Some(j) = &mut self.journal { j.recover()?; }
        zpl::init_zpl(self)?;
        Ok(())
    }

    pub fn unmount(&mut self) -> Result<(), &'static str> {
        if let Some(j) = &mut self.journal { j.flush()?; }
        dmu::sync_all()?;
        Ok(())
    }

    pub fn snapshot(&self, name: &str) -> Result<(), &'static str> {
        if let Some(dmu) = &self.dmu { dmu.create_snapshot(name)?; Ok(()) } else { Err("DMU not initialized") }
    }

    pub fn rollback(&self, name: &str) -> Result<(), &'static str> {
        if let Some(dmu) = &self.dmu { dmu.rollback_to_snapshot(name)?; Ok(()) } else { Err("DMU not initialized") }
    }

    pub fn get_dmu(&self) -> &DMU { self.dmu.as_ref().expect("DMU not initialized") }
    pub fn get_dmu_mut(&mut self) -> &mut DMU { self.dmu.as_mut().expect("DMU not initialized") }
    pub fn get_zio(&self) -> &ZIOPipeline { self.zio.as_ref().expect("ZIO not initialized") }
    pub fn get_arc(&self) -> &ARCCache { self.arc.as_ref().expect("ARC not initialized") }
    pub fn get_state(&self) -> &FileSystemState { self.state.as_ref().expect("Filesystem state not initialized") }
    pub fn get_state_mut(&mut self) -> &mut FileSystemState { self.state.as_mut().expect("Filesystem state not initialized") }
    pub fn set_state(&mut self, state: FileSystemState) { self.state = Some(state); }
    pub fn get_vdev(&self) -> &VDev { &self.root_vdev }
    pub fn set_checksum_algorithm(&mut self, alg: ChecksumAlgorithm) { self.checksum_alg = alg; }
    pub fn get_checksum_algorithm(&self) -> ChecksumAlgorithm { self.checksum_alg }
    pub fn get_journal(&self) -> &Journal { self.journal.as_ref().expect("Journal not initialized") }
    pub fn get_journal_mut(&mut self) -> &mut Journal { self.journal.as_mut().expect("Journal not initialized") }
}

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
    fn read(&self, inode: u64, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> { zpl::read_file(self, inode, offset, buffer) }
    fn write(&mut self, inode: u64, offset: u64, data: &[u8]) -> Result<usize, &'static str> { zpl::write_file(self, inode, offset, data) }
    fn create_file(&mut self, parent_inode: u64, name: &str) -> Result<u64, &'static str> { zpl::create_file(self, parent_inode, name) }
    fn create_dir(&mut self, parent_inode: u64, name: &str) -> Result<u64, &'static str> { zpl::create_dir(self, parent_inode, name) }
    fn lookup(&self, parent_inode: u64, name: &str) -> Result<u64, &'static str> { zpl::lookup(self, parent_inode, name) }
    fn link(&mut self, inode: u64, parent_inode: u64, name: &str) -> Result<(), &'static str> { zpl::link_file(self, inode, parent_inode, name) }
    fn unlink(&mut self, parent_inode: u64, name: &str) -> Result<(), &'static str> { zpl::unlink_file(self, parent_inode, name) }
}
