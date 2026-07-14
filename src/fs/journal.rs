// Journaling system for NebulaFS
// Provides crash recovery and transaction logging

use alloc::vec::Vec;
use alloc::collections::VecDeque;
use crate::fs::dmu::BlockPointer;
use crate::fs::checksum::{fletcher4, ChecksumAlgorithm};

/// Journal entry types
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

/// Journal entry
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub entry_type: JournalEntryType,
    pub txg_id: u64,               // Transaction group ID
    pub data: Vec<u8>,             // Entry data
    pub checksum: Vec<u8>,          // Checksum for integrity
    pub checksum_alg: ChecksumAlgorithm, // Checksum algorithm used
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

    /// Calculate checksum for the entry
    pub fn calculate_checksum(&mut self) {
        let data_to_checksum = [
            &[self.entry_type as u8],
            &self.txg_id.to_le_bytes(),
            &self.data,
        ].concat();
        
        match self.checksum_alg {
            ChecksumAlgorithm::Fletcher4 => {
                let (sum1, sum2) = fletcher4(&data_to_checksum);
                self.checksum = [
                    &sum1.to_le_bytes(),
                    &sum2.to_le_bytes(),
                ].concat();
            }
            _ => {
                // For other algorithms, we would calculate accordingly
                self.checksum = Vec::new();
            }
        }
    }

    /// Verify the entry's checksum
    pub fn verify_checksum(&self) -> bool {
        let data_to_checksum = [
            &[self.entry_type as u8],
            &self.txg_id.to_le_bytes(),
            &self.data,
        ].concat();
        
        match self.checksum_alg {
            ChecksumAlgorithm::Fletcher4 => {
                if self.checksum.len() != 8 {
                    return false;
                }
                let expected_sum1 = u32::from_le_bytes([
                    self.checksum[0],
                    self.checksum[1],
                    self.checksum[2],
                    self.checksum[3],
                ]);
                let expected_sum2 = u32::from_le_bytes([
                    self.checksum[4],
                    self.checksum[5],
                    self.checksum[6],
                    self.checksum[7],
                ]);
                let (calculated_sum1, calculated_sum2) = fletcher4(&data_to_checksum);
                expected_sum1 == calculated_sum1 && expected_sum2 == calculated_sum2
            }
            _ => {
                // For other algorithms, we would verify accordingly
                false
            }
        }
    }
}

/// Journal structure
pub struct Journal {
    entries: VecDeque<JournalEntry>,  // In-memory journal entries
    log_device: Option<Box<dyn JournalDevice>>, // Device for persistent logging
    current_txg: u64,               // Current transaction group
    max_entries: usize,             // Maximum number of in-memory entries
    flushed_txg: u64,              // Last flushed transaction group
}

/// Journal device trait
pub trait JournalDevice {
    /// Write journal entries to the device
    fn write_entries(&mut self, entries: &[JournalEntry]) -> Result<(), &'static str>;
    
    /// Read journal entries from the device
    fn read_entries(&mut self) -> Result<Vec<JournalEntry>, &'static str>;
    
    /// Flush writes to the device
    fn flush(&mut self) -> Result<(), &'static str>;
    
    /// Clear the journal on the device
    fn clear(&mut self) -> Result<(), &'static str>;
}

impl Journal {
    /// Create a new journal
    pub fn new(max_entries: usize) -> Self {
        Journal {
            entries: VecDeque::with_capacity(max_entries),
            log_device: None,
            current_txg: 1,
            max_entries,
            flushed_txg: 0,
        }
    }

    /// Set the journal device
    pub fn set_device(&mut self, device: Box<dyn JournalDevice>) {
        self.log_device = Some(device);
    }

    /// Start a new transaction
    pub fn start_transaction(&mut self) -> u64 {
        self.current_txg += 1;
        let entry = JournalEntry::new(JournalEntryType::TransactionStart, self.current_txg, Vec::new());
        self.add_entry(entry);
        self.current_txg
    }

    /// End the current transaction
    pub fn end_transaction(&mut self) {
        let entry = JournalEntry::new(JournalEntryType::TransactionEnd, self.current_txg, Vec::new());
        self.add_entry(entry);
    }

    /// Add an entry to the journal
    pub fn add_entry(&mut self, entry: JournalEntry) {
        if self.entries.len() >= self.max_entries {
            // Flush to make room
            let _ = self.flush();
        }
        self.entries.push_back(entry);
    }

    /// Log a block allocation
    pub fn log_block_allocate(&mut self, bp: &BlockPointer) {
        let mut data = Vec::new();
        data.extend_from_slice(&bp.vdev_id.to_le_bytes());
        data.extend_from_slice(&bp.offset.to_le_bytes());
        data.extend_from_slice(&bp.size.to_le_bytes());
        data.extend_from_slice(&bp.logical_size.to_le_bytes());
        
        let entry = JournalEntry::new(JournalEntryType::BlockAllocate, self.current_txg, data);
        self.add_entry(entry);
    }

    /// Log a block free
    pub fn log_block_free(&mut self, bp: &BlockPointer) {
        let mut data = Vec::new();
        data.extend_from_slice(&bp.vdev_id.to_le_bytes());
        data.extend_from_slice(&bp.offset.to_le_bytes());
        
        let entry = JournalEntry::new(JournalEntryType::BlockFree, self.current_txg, data);
        self.add_entry(entry);
    }

    /// Flush the journal to the device
    pub fn flush(&mut self) -> Result<(), &'static str> {
        if self.entries.is_empty() {
            return Ok(());
        }
        
        if let Some(device) = &mut self.log_device {
            // Convert entries to a vector for writing
            let entries_vec: Vec<JournalEntry> = self.entries.iter().cloned().collect();
            device.write_entries(&entries_vec)?;
            device.flush()?;
            
            // Clear the in-memory journal
            self.flushed_txg = self.current_txg;
            self.entries.clear();
        }
        
        Ok(())
    }

    /// Recover from a crash by replaying the journal
    pub fn recover(&mut self) -> Result<(), &'static str> {
        if let Some(device) = &mut self.log_device {
            // Read entries from the device
            let entries = device.read_entries()?;
            
            // Replay the journal
            for entry in entries {
                if !entry.verify_checksum() {
                    return Err("Journal checksum verification failed");
                }
                
                match entry.entry_type {
                    JournalEntryType::TransactionStart => {
                        self.current_txg = entry.txg_id;
                    }
                    JournalEntryType::TransactionEnd => {
                        // Commit the transaction
                    }
                    JournalEntryType::BlockAllocate => {
                        // Replay block allocation
                    }
                    JournalEntryType::BlockFree => {
                        // Replay block free
                    }
                    _ => {}
                }
            }
            
            // Clear the journal after recovery
            device.clear()?;
        }
        
        Ok(())
    }

    /// Get the current transaction group
    pub fn current_txg(&self) -> u64 {
        self.current_txg
    }

    /// Get the last flushed transaction group
    pub fn flushed_txg(&self) -> u64 {
        self.flushed_txg
    }

    /// Check if there are uncommitted transactions
    pub fn has_uncommitted(&self) -> bool {
        self.current_txg > self.flushed_txg
    }
}

/// Simple in-memory journal device for testing
pub struct MemoryJournalDevice {
    entries: Vec<JournalEntry>,
}

impl MemoryJournalDevice {
    pub fn new() -> Self {
        MemoryJournalDevice {
            entries: Vec::new(),
        }
    }
}

impl JournalDevice for MemoryJournalDevice {
    fn write_entries(&mut self, entries: &[JournalEntry]) -> Result<(), &'static str> {
        self.entries.extend_from_slice(entries);
        Ok(())
    }

    fn read_entries(&mut self) -> Result<Vec<JournalEntry>, &'static str> {
        Ok(self.entries.clone())
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), &'static str> {
        self.entries.clear();
        Ok(())
    }
}

/// Initialize the journal
pub fn init_journal(max_entries: usize) -> Journal {
    Journal::new(max_entries)
}