use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use crate::fs::dmu::BlockPointer;
use crate::fs::checksum::{fletcher4, ChecksumAlgorithm};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JournalEntryType {
    TransactionStart,
    TransactionEnd,
    BlockAllocate,
    BlockFree,
    InodeCreate,
    InodeDelete,
    InodeUpdate,
    DirectoryCreate,
    DirectoryDelete,
    DirectoryUpdate,
}

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub entry_type: JournalEntryType,
    pub txg_id: u64,
    pub data: Vec<u8>,
    pub checksum: Vec<u8>,
    pub checksum_alg: ChecksumAlgorithm,
}

impl JournalEntry {
    pub fn new(entry_type: JournalEntryType, txg_id: u64, data: Vec<u8>) -> Self {
        let mut entry = JournalEntry {
            entry_type,
            txg_id,
            data,
            checksum: Vec::new(),
            checksum_alg: ChecksumAlgorithm::Fletcher4,
        };
        entry.calculate_checksum();
        entry
    }

    pub fn calculate_checksum(&mut self) {
        let mut buf = Vec::new();
        buf.push(self.entry_type as u8);
        buf.extend_from_slice(&self.txg_id.to_le_bytes());
        buf.extend_from_slice(self.data.as_slice());
        match self.checksum_alg {
            ChecksumAlgorithm::Fletcher4 => {
                let (sum1, sum2) = fletcher4(buf.as_slice());
                let mut c = Vec::new();
                c.extend_from_slice(&sum1.to_le_bytes());
                c.extend_from_slice(&sum2.to_le_bytes());
                self.checksum = c;
            }
            _ => { self.checksum = Vec::new(); }
        }
    }

    pub fn verify_checksum(&self) -> bool {
        let mut buf = Vec::new();
        buf.push(self.entry_type as u8);
        buf.extend_from_slice(&self.txg_id.to_le_bytes());
        buf.extend_from_slice(self.data.as_slice());
        match self.checksum_alg {
            ChecksumAlgorithm::Fletcher4 => {
                if self.checksum.len() != 8 { return false; }
                let cs = self.checksum.as_slice();
                let expected_sum1 = u32::from_le_bytes([cs[0], cs[1], cs[2], cs[3]]);
                let expected_sum2 = u32::from_le_bytes([cs[4], cs[5], cs[6], cs[7]]);
                let (c1, c2) = fletcher4(buf.as_slice());
                expected_sum1 == c1 && expected_sum2 == c2
            }
            _ => false,
        }
    }
}

pub struct Journal {
    entries: VecDeque<JournalEntry>,
    log_device: Option<Box<dyn JournalDevice>>,
    current_txg: u64,
    max_entries: usize,
    flushed_txg: u64,
}

impl core::fmt::Debug for Journal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Journal")
            .field("entries", &self.entries)
            .field("log_device", &self.log_device.as_ref().map(|_| "..."))
            .field("current_txg", &self.current_txg)
            .field("max_entries", &self.max_entries)
            .field("flushed_txg", &self.flushed_txg)
            .finish()
    }
}

pub trait JournalDevice: Send + Sync {
    fn write_entries(&mut self, entries: &[JournalEntry]) -> Result<(), &'static str>;
    fn read_entries(&mut self) -> Result<Vec<JournalEntry>, &'static str>;
    fn flush(&mut self) -> Result<(), &'static str>;
    fn clear(&mut self) -> Result<(), &'static str>;
}

impl Journal {
    pub fn new(max_entries: usize) -> Self {
        Journal {
            entries: VecDeque::with_capacity(max_entries),
            log_device: None,
            current_txg: 1,
            max_entries,
            flushed_txg: 0,
        }
    }

    pub fn set_device(&mut self, device: Box<dyn JournalDevice>) {
        self.log_device = Some(device);
    }

    pub fn start_transaction(&mut self) -> u64 {
        self.current_txg += 1;
        self.add_entry(JournalEntry::new(JournalEntryType::TransactionStart, self.current_txg, Vec::new()));
        self.current_txg
    }

    pub fn end_transaction(&mut self) {
        self.add_entry(JournalEntry::new(JournalEntryType::TransactionEnd, self.current_txg, Vec::new()));
    }

    pub fn add_entry(&mut self, entry: JournalEntry) {
        if self.entries.len() >= self.max_entries { let _ = self.flush(); }
        self.entries.push_back(entry);
    }

    pub fn log_block_allocate(&mut self, bp: &BlockPointer) {
        let mut data = Vec::new();
        data.extend_from_slice(&bp.vdev_id.to_le_bytes());
        data.extend_from_slice(&bp.offset.to_le_bytes());
        data.extend_from_slice(&bp.size.to_le_bytes());
        data.extend_from_slice(&bp.logical_size.to_le_bytes());
        self.add_entry(JournalEntry::new(JournalEntryType::BlockAllocate, self.current_txg, data));
    }

    pub fn log_block_free(&mut self, bp: &BlockPointer) {
        let mut data = Vec::new();
        data.extend_from_slice(&bp.vdev_id.to_le_bytes());
        data.extend_from_slice(&bp.offset.to_le_bytes());
        self.add_entry(JournalEntry::new(JournalEntryType::BlockFree, self.current_txg, data));
    }

    pub fn flush(&mut self) -> Result<(), &'static str> {
        if self.entries.is_empty() { return Ok(()); }
        if let Some(device) = &mut self.log_device {
            let v: Vec<JournalEntry> = self.entries.iter().cloned().collect();
            device.write_entries(&v)?;
            device.flush()?;
            self.flushed_txg = self.current_txg;
            self.entries.clear();
        }
        Ok(())
    }

    pub fn recover(&mut self) -> Result<(), &'static str> {
        if let Some(device) = &mut self.log_device {
            for entry in device.read_entries()? {
                if !entry.verify_checksum() { return Err("Journal checksum verification failed"); }
                match entry.entry_type {
                    JournalEntryType::TransactionStart => { self.current_txg = entry.txg_id; }
                    _ => {}
                }
            }
            device.clear()?;
        }
        Ok(())
    }

    pub fn current_txg(&self) -> u64 { self.current_txg }
    pub fn flushed_txg(&self) -> u64 { self.flushed_txg }
    pub fn has_uncommitted(&self) -> bool { self.current_txg > self.flushed_txg }
}

pub struct MemoryJournalDevice { entries: Vec<JournalEntry> }

impl MemoryJournalDevice {
    pub fn new() -> Self { MemoryJournalDevice { entries: Vec::new() } }
}

impl JournalDevice for MemoryJournalDevice {
    fn write_entries(&mut self, entries: &[JournalEntry]) -> Result<(), &'static str> {
        self.entries.extend_from_slice(entries);
        Ok(())
    }
    fn read_entries(&mut self) -> Result<Vec<JournalEntry>, &'static str> { Ok(self.entries.clone()) }
    fn flush(&mut self) -> Result<(), &'static str> { Ok(()) }
    fn clear(&mut self) -> Result<(), &'static str> { self.entries.clear(); Ok(()) }
}

pub fn init_journal(max_entries: usize) -> Journal { Journal::new(max_entries) }
