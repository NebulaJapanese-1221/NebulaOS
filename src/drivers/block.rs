// Block Device Interface for NebulaOS
// Provides low-level disk I/O operations

use core::ptr;

/// Block device trait
pub trait BlockDevice {
    /// Read blocks from the device
    fn read_blocks(&self, start_block: u64, block_count: u64, buffer: &mut [u8]) -> Result<(), &'static str>;
    
    /// Write blocks to the device
    fn write_blocks(&self, start_block: u64, block_count: u64, buffer: &[u8]) -> Result<(), &'static str>;
    
    /// Flush writes to the device
    fn flush(&self) -> Result<(), &'static str>;
    
    /// Get device information
    fn get_info(&self) -> BlockDeviceInfo;
}

/// Block device information
#[derive(Debug, Clone, Copy)]
pub struct BlockDeviceInfo {
    pub block_size: u64,
    pub total_blocks: u64,
    pub device_name: &'static str,
}

/// Simple RAM disk implementation for testing
pub struct RamDisk {
    blocks: Vec<u8>,
    block_size: u64,
    total_blocks: u64,
}

impl RamDisk {
    /// Create a new RAM disk
    pub fn new(block_size: u64, total_blocks: u64) -> Self {
        let total_size = (block_size * total_blocks) as usize;
        RamDisk {
            blocks: vec![0; total_size],
            block_size,
            total_blocks,
        }
    }
}

impl BlockDevice for RamDisk {
    fn read_blocks(&self, start_block: u64, block_count: u64, buffer: &mut [u8]) -> Result<(), &'static str> {
        let start_byte = start_block * self.block_size;
        let end_byte = start_byte + (block_count * self.block_size);
        
        // Check bounds
        if end_byte > (self.total_blocks * self.block_size) {
            return Err("Read out of bounds");
        }
        
        // Copy data from RAM disk to buffer
        let start = start_byte as usize;
        let end = end_byte as usize;
        if end <= self.blocks.len() {
            buffer.copy_from_slice(&self.blocks[start..end]);
            Ok(())
        } else {
            Err("Buffer too small")
        }
    }
    
    fn write_blocks(&self, start_block: u64, block_count: u64, buffer: &[u8]) -> Result<(), &'static str> {
        let start_byte = start_block * self.block_size;
        let end_byte = start_byte + (block_count * self.block_size);
        
        // Check bounds
        if end_byte > (self.total_blocks * self.block_size) {
            return Err("Write out of bounds");
        }
        
        // Copy data from buffer to RAM disk
        let start = start_byte as usize;
        let end = end_byte as usize;
        if end <= self.blocks.len() && buffer.len() >= (end - start) {
            self.blocks[start..end].copy_from_slice(&buffer[..(end - start)]);
            Ok(())
        } else {
            Err("Buffer size mismatch")
        }
    }
    
    fn flush(&self) -> Result<(), &'static str> {
        // RAM disk doesn't need flushing
        Ok(())
    }
    
    fn get_info(&self) -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: self.block_size,
            total_blocks: self.total_blocks,
            device_name: "ramdisk",
        }
    }
}

/// ATA PIO (Programmed I/O) disk driver
pub struct ATADisk {
    base_port: u16,
    block_size: u64,
    total_blocks: u64,
}

impl ATADisk {
    /// Create a new ATA disk driver
    pub fn new(base_port: u16, block_size: u64, total_blocks: u64) -> Self {
        ATADisk {
            base_port,
            block_size,
            total_blocks,
        }
    }
    
    /// Wait for the disk to be ready
    fn wait_for_ready(&self) {
        unsafe {
            // Wait for BSY bit to clear and DRQ bit to set
            loop {
                let status = ptr::read_volatile((self.base_port + 7) as *const u8);
                if (status & 0x80) == 0 && (status & 0x08) != 0 {
                    break;
                }
            }
        }
    }
}

impl BlockDevice for ATADisk {
    fn read_blocks(&self, start_block: u64, block_count: u64, buffer: &mut [u8]) -> Result<(), &'static str> {
        // In a real implementation, this would use ATA PIO commands
        // For now, we'll return an error as this is not fully implemented
        Err("ATA disk read not implemented")
    }
    
    fn write_blocks(&self, start_block: u64, block_count: u64, buffer: &[u8]) -> Result<(), &'static str> {
        // In a real implementation, this would use ATA PIO commands
        // For now, we'll return an error as this is not fully implemented
        Err("ATA disk write not implemented")
    }
    
    fn flush(&self) -> Result<(), &'static str> {
        // In a real implementation, this would flush the disk cache
        Ok(())
    }
    
    fn get_info(&self) -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: self.block_size,
            total_blocks: self.total_blocks,
            device_name: "ata",
        }
    }
}

/// Block device manager
pub struct BlockDeviceManager {
    devices: Vec<Box<dyn BlockDevice>>,
}

impl BlockDeviceManager {
    /// Create a new block device manager
    pub fn new() -> Self {
        BlockDeviceManager {
            devices: Vec::new(),
        }
    }
    
    /// Register a block device
    pub fn register_device(&mut self, device: Box<dyn BlockDevice>) {
        self.devices.push(device);
    }
    
    /// Get a block device by index
    pub fn get_device(&self, index: usize) -> Option<&dyn BlockDevice> {
        self.devices.get(index).map(|d| d.as_ref())
    }
    
    /// Get number of registered devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}