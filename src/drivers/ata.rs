use crate::kernel::io;
use alloc::vec::Vec;
use nebulafs::vdev::BlockDevice;

pub const SECTOR_SIZE: usize = 512;

#[repr(u16)]
enum Command {
    Read = 0x20,
    Write = 0x30,
    Identify = 0xEC,
    CacheFlush = 0xE7,
}

#[derive(Debug)]
pub struct AtaDrive {
    port_base: u16,
    is_master: bool,
}

impl AtaDrive {
    /// Creates a new ATA drive handle.
    /// `primary`: true for 0x1F0 (Primary Bus), false for 0x170 (Secondary Bus).
    /// `master`: true for Master drive, false for Slave.
    pub fn new(primary: bool, master: bool) -> Self {
        Self {
            port_base: if primary { 0x1F0 } else { 0x170 },
            is_master: master,
        }
    }

    /// Reads `sectors` count of sectors starting at `lba`.
    pub fn read_sectors(&self, lba: u32, sectors: u8) -> Vec<u8> {
        let mut data = Vec::with_capacity(sectors as usize * SECTOR_SIZE);
        
        unsafe {
            // Select Drive: 0xE0 (LBA Mode) | (Master/Slave << 4) | (LBA >> 24 & 0x0F)
            io::outb(self.port_base + 6, 0xE0 | ((self.is_master as u8) << 4) | ((lba >> 24) as u8 & 0x0F));
            
            // Send NULL byte to Port 0x1F1 (Error/Feature) just in case
            io::outb(self.port_base + 1, 0x00);

            // Sector Count
            io::outb(self.port_base + 2, sectors);
            
            // LBA Low, Mid, High
            io::outb(self.port_base + 3, lba as u8);
            io::outb(self.port_base + 4, (lba >> 8) as u8);
            io::outb(self.port_base + 5, (lba >> 16) as u8);
            
            // Command: Read
            io::outb(self.port_base + 7, Command::Read as u8);
            
            for _ in 0..sectors {
                self.poll();
                
                // Read 256 words (512 bytes)
                for _ in 0..256 {
                    let word = io::inw(self.port_base);
                    data.push((word & 0xFF) as u8);
                    data.push((word >> 8) as u8);
                }
            }
        }
        
        data
    }
    
    /// Writes `data` to sectors starting at `lba`.
    /// Data length must be a multiple of 512.
    pub fn write_sectors(&self, lba: u32, data: &[u8]) {
        if data.len() % SECTOR_SIZE != 0 {
            return; 
        }

        let sectors = (data.len() / SECTOR_SIZE) as u8;
        
         unsafe {
            io::outb(self.port_base + 6, 0xE0 | ((self.is_master as u8) << 4) | ((lba >> 24) as u8 & 0x0F));
            io::outb(self.port_base + 1, 0x00);
            io::outb(self.port_base + 2, sectors);
            io::outb(self.port_base + 3, lba as u8);
            io::outb(self.port_base + 4, (lba >> 8) as u8);
            io::outb(self.port_base + 5, (lba >> 16) as u8);
            io::outb(self.port_base + 7, Command::Write as u8);
            
            for i in 0..sectors {
                self.poll();
                
                for j in 0..256 {
                    let offset = (i as usize * SECTOR_SIZE) + (j * 2);
                    // Little endian word
                    let word = (data[offset] as u16) | ((data[offset + 1] as u16) << 8);
                    io::outw(self.port_base, word);
                }
            }
            
            // Wait for last write to complete before flushing
            self.wait_busy();
            
            // Flush Cache
            io::outb(self.port_base + 7, Command::CacheFlush as u8);
            self.wait_busy();
        }
    }

    /// Waits for the drive to be ready (BSY=0, DRQ=1) for data transfer.
    unsafe fn poll(&self) {
        // Delay (400ns)
        for _ in 0..4 { io::inb(self.port_base + 7); }
        
        loop {
            let status = io::inb(self.port_base + 7);
            // BSY=0 and DRQ=1
            if (status & 0x80) == 0 && (status & 0x08) != 0 { break; }
            if (status & 0x01) != 0 { break; } // Error
        }
    }
    
    /// Waits for the drive to not be busy (BSY=0).
    unsafe fn wait_busy(&self) {
        for _ in 0..4 { io::inb(self.port_base + 7); }
        loop {
            let status = io::inb(self.port_base + 7);
            if (status & 0x80) == 0 { break; }
             if (status & 0x01) != 0 { break; }
        }
    }
}

impl BlockDevice for AtaDrive {
    fn read(&self, offset: u64, size: usize) -> Vec<u8> {
        let lba = (offset / SECTOR_SIZE as u64) as u32;
        let sector_count = ((size + SECTOR_SIZE - 1) / SECTOR_SIZE) as u8;
        let mut data = self.read_sectors(lba, sector_count);
        
        // Trim to exact requested size if necessary
        if data.len() > size {
            data.truncate(size);
        }
        data
    }

    fn write(&self, offset: u64, data: &[u8]) {
        let lba = (offset / SECTOR_SIZE as u64) as u32;
        // Pad data to sector boundary if needed (naive implementation)
        let mut padded = Vec::from(data);
        while padded.len() % SECTOR_SIZE != 0 {
            padded.push(0);
        }
        self.write_sectors(lba, padded.as_slice());
    }
}