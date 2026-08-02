// SD Card Block Device Driver for NebulaOS
// Wraps the SDHCI host controller as a BlockDevice

use crate::block::BlockDevice;
use crate::block::BlockDeviceInfo;
use crate::sdhci::SDHCIHost;
use crate::serial_println;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// SD Card block device implementation
pub struct SdCardBlockDevice {
    controller: SDHCIHost,
    block_size: u64,
    total_blocks: u64,
    initialized: bool,
}

impl SdCardBlockDevice {
    /// Create a new SD card block device
    pub fn new() -> Self {
        // Try to find an SDHCI controller
        if let Some((mmio_base, _bus_info)) = SDHCIHost::find_controller() {
            let controller = SDHCIHost::new(mmio_base);
            SdCardBlockDevice {
                controller,
                block_size: 512,
                total_blocks: 0,
                initialized: false,
            }
        } else {
            serial_println!("SD Card: No SDHCI controller found");
            SdCardBlockDevice {
                controller: SDHCIHost::new(0),
                block_size: 512,
                total_blocks: 0,
                initialized: false,
            }
        }
    }

    /// Initialize the SD card
    pub fn init(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }

        if self.controller.mmio_base == 0 {
            return Err("No SDHCI controller available");
        }

        serial_println!("SD Card: Initializing...");

        // Initialize the SDHCI host controller
        self.controller.init()?;

        // SD Card initialization sequence
        self.init_sd_card()?;

        self.total_blocks = self.controller.get_block_count();
        self.block_size = self.controller.get_block_size();
        self.initialized = true;

        serial_println!("SD Card: Initialized ({} blocks x {} bytes = {} MB)",
            self.total_blocks, self.block_size,
            (self.total_blocks * self.block_size) / (1024 * 1024));

        Ok(())
    }

    /// Initialize the SD card using SD protocol commands
    fn init_sd_card(&self) -> Result<(), &'static str> {
        serial_println!("SD Card: Sending init sequence...");

        // CMD0: GO_IDLE_STATE
        self.controller.send_command(0, 0x00000000, 0)?;
        serial_println!("SD Card: CMD0 (GO_IDLE_STATE) sent");

        // CMD8: SEND_IF_COND (check SDHC support)
        let check_pattern = 0x000001AAu32; // 2.7-3.6V range, check pattern 0xAA
        let response = self.controller.send_command(8, check_pattern, 1)?; // Response type R7
        serial_println!("SD Card: CMD8 (SEND_IF_COND) response = 0x{:08x}", response);

        // Check if SDHC card (response matches our check pattern)
        if (response & 0xFF) != 0xAA {
            serial_println!("SD Card: Not an SDHC card or voltage mismatch");
            // Try SDSC initialization
        }

        // ACMD41: SD_SEND_OP_COND (with HCS bit for SDHC)
        // Send CMD55 first (prefix for ACMD)
        let _ = self.controller.send_command(55, 0x00000000, 0)?;
        let response = self.controller.send_command(41, 0x40000000, 0)?; // HCS bit set
        serial_println!("SD Card: ACMD41 (SEND_OP_COND) response = 0x{:08x}", response);

        // CMD2: ALL_SEND_CID
        let _cid = self.controller.send_command(2, 0x00000000, 2)?; // Response type R2 (136-bit)
        serial_println!("SD Card: CMD2 (ALL_SEND_CID) sent");

        // CMD3: SEND_RELATIVE_ADDR
        let rca = self.controller.send_command(3, 0x00000000, 0)?;
        serial_println!("SD Card: CMD3 (SEND_RCA) response RCA = 0x{:08x}", rca);

        // CMD9: SEND_CSD
        let _csd = self.controller.send_command(9, rca & 0xFFFF0000, 2)?;
        serial_println!("SD Card: CMD9 (SEND_CSD) sent");

        // CMD7: SELECT_CARD
        self.controller.send_command(7, rca & 0xFFFF0000, 1)?;
        serial_println!("SD Card: CMD7 (SELECT_CARD) sent");

        // CMD16: SET_BLOCKLEN to 512
        self.controller.send_command(16, 512, 0)?;
        serial_println!("SD Card: CMD16 (SET_BLOCKLEN) sent");

        serial_println!("SD Card: Init sequence complete");
        Ok(())
    }
}

impl BlockDevice for SdCardBlockDevice {
    /// Read blocks from the SD card
    fn read_blocks(&self, start_block: u64, block_count: u64, buffer: &mut [u8]) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("SD card not initialized");
        }

        let block_size = self.block_size as usize;
        let expected_size = (block_count * block_size as u64) as usize;

        if buffer.len() < expected_size {
            return Err("Buffer too small");
        }

        for i in 0..block_count {
            let lba = (start_block + i) as u32;
            let offset = (i * block_size as u64) as usize;
            
            // Read one block at a time into a temporary 512-byte buffer
            let mut block_buf: [u8; 512] = [0u8; 512];
            self.controller.read_block(lba, &mut block_buf)?;

            // Copy to output buffer
            buffer[offset..offset + block_size].copy_from_slice(&block_buf[..block_size]);
        }

        Ok(())
    }

    /// Write blocks to the SD card
    fn write_blocks(&self, start_block: u64, block_count: u64, buffer: &[u8]) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("SD card not initialized");
        }

        let block_size = self.block_size as usize;
        let expected_size = (block_count * block_size as u64) as usize;

        if buffer.len() < expected_size {
            return Err("Buffer too small");
        }

        for i in 0..block_count {
            let lba = (start_block + i) as u32;
            let offset = (i * block_size as u64) as usize;

            // Copy from input buffer into a 512-byte block
            let mut block_buf: [u8; 512] = [0u8; 512];
            block_buf[..block_size].copy_from_slice(&buffer[offset..offset + block_size]);

            // Write one block
            self.controller.write_block(lba, &block_buf)?;
        }

        Ok(())
    }

    /// Flush (no-op for SD card writes are synchronous)
    fn flush(&self) -> Result<(), &'static str> {
        Ok(())
    }

    /// Get device information
    fn get_info(&self) -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: self.block_size,
            total_blocks: self.total_blocks,
            device_name: "sd_card",
        }
    }
}

/// Probe for SD cards and return a block device if found
pub fn probe_sd_card() -> Option<Box<dyn BlockDevice>> {
    let mut sd_card = SdCardBlockDevice::new();
    
    match sd_card.init() {
        Ok(()) => {
            serial_println!("SD Card: Probed successfully");
            Some(Box::new(sd_card))
        }
        Err(e) => {
            serial_println!("SD Card: Probe failed: {}", e);
            None
        }
    }
}

