use alloc::vec::Vec;
use core::mem;
use crate::fs::vdev::VDev;
use crate::fs::checksum::{fletcher2, fletcher4, sha256_simple, ChecksumAlgorithm, verify_checksum};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IOType { Read, Write, Free, Claim }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IOPriority { SyncRead, SyncWrite, AsyncRead, AsyncWrite, Scrub, Resilver }

#[derive(Debug)]
pub struct IOOperation {
    pub io_type: IOType,
    pub priority: IOPriority,
    pub vdev: VDev,
    pub offset: u64,
    pub size: u64,
    pub data: Vec<u8>,
    pub checksum: Vec<u8>,
    pub checksum_alg: ChecksumAlgorithm,
    pub error: Option<u32>,
}

impl IOOperation {
    pub fn new(io_type: IOType, priority: IOPriority, vdev: VDev, offset: u64, size: u64) -> Self {
        IOOperation { io_type, priority, vdev, offset, size, data: Vec::new(), checksum: Vec::new(), checksum_alg: ChecksumAlgorithm::Fletcher4, error: None }
    }
    pub fn set_checksum_algorithm(&mut self, alg: ChecksumAlgorithm) { self.checksum_alg = alg; }
    pub fn execute(&mut self) -> Result<(), &'static str> {
        match self.io_type { IOType::Read => self.execute_read(), IOType::Write => self.execute_write(), IOType::Free => self.execute_free(), IOType::Claim => self.execute_claim() }
    }
    fn execute_read(&mut self) -> Result<(), &'static str> {
        self.vdev.read(self.offset, self.data.as_mut_slice())?;
        if !self.checksum.is_empty() {
            if !verify_checksum(self.data.as_slice(), self.checksum.as_slice(), self.checksum_alg) {
                self.error = Some(1); return Err("Checksum mismatch");
            }
        }
        Ok(())
    }
    fn execute_write(&mut self) -> Result<(), &'static str> {
        self.calculate_checksum();
        self.vdev.write(self.offset, self.data.as_slice())?;
        Ok(())
    }
    fn calculate_checksum(&mut self) {
        let data_slice = self.data.as_slice();
        match self.checksum_alg {
            ChecksumAlgorithm::Fletcher2 => { self.checksum = fletcher2(data_slice).to_le_bytes().to_vec(); }
            ChecksumAlgorithm::Fletcher4 => { let (sum1, sum2) = fletcher4(data_slice); let mut b = Vec::with_capacity(8); b.extend_from_slice(&sum1.to_le_bytes()); b.extend_from_slice(&sum2.to_le_bytes()); self.checksum = b; }
            ChecksumAlgorithm::SHA256 => { self.checksum = sha256_simple(data_slice).to_vec(); }
        };
    }
    fn execute_free(&mut self) -> Result<(), &'static str> { Ok(()) }
    fn execute_claim(&mut self) -> Result<(), &'static str> { Ok(()) }
}

#[derive(Debug)]
pub struct ZIOPipeline {
    pub pending_ops: Vec<IOOperation>,
    pub inflight_ops: Vec<IOOperation>,
    pub completed_ops: Vec<IOOperation>,
}

impl ZIOPipeline {
    pub fn new() -> Self { ZIOPipeline { pending_ops: Vec::new(), inflight_ops: Vec::new(), completed_ops: Vec::new() } }
    pub fn issue(&mut self, op: IOOperation) { self.pending_ops.push(op); }
    pub fn process(&mut self) -> Result<(), &'static str> {
        self.inflight_ops.append(&mut self.pending_ops);
        for op in &mut self.inflight_ops { op.execute()?; }
        self.completed_ops.append(&mut self.inflight_ops);
        Ok(())
    }
    pub fn wait(&mut self) -> Result<(), &'static str> { while !self.pending_ops.is_empty() || !self.inflight_ops.is_empty() { self.process()?; } Ok(()) }
    pub fn get_completed(&mut self) -> Vec<IOOperation> { mem::replace(&mut self.completed_ops, Vec::new()) }
}

pub fn init_zio() -> ZIOPipeline { ZIOPipeline::new() }
