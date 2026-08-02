// SD Host Controller Interface (SDHCI) Driver for NebulaOS
// PCI-based SD/MMC controller driver using MMIO register access

use core::ptr;
use crate::ps2::{outb, inb};
use crate::serial_println;
use alloc::vec::Vec;

/// PCI Configuration I/O ports
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// SDHCI Register offsets (relative to MMIO base)
const SDHCI_DMA_ADDRESS: usize = 0x00;
const SDHCI_BLOCK_SIZE: usize = 0x04;
const SDHCI_BLOCK_COUNT: usize = 0x06;
const SDHCI_ARGUMENT: usize = 0x08;
const SDHCI_TRANSFER_MODE: usize = 0x0C;
const SDHCI_COMMAND: usize = 0x0E;
const SDHCI_RESPONSE: usize = 0x10; // 16 bytes (4 registers)
const SDHCI_BUFFER_DATA_PORT: usize = 0x20;
const SDHCI_PRESENT_STATE: usize = 0x24;
const SDHCI_HOST_CONTROL: usize = 0x28;
const SDHCI_POWER_CONTROL: usize = 0x29;
const SDHCI_BLOCK_GAP_CONTROL: usize = 0x2A;
const SDHCI_WAKEUP_CONTROL: usize = 0x2B;
const SDHCI_CLOCK_CONTROL: usize = 0x2C;
const SDHCI_TIMEOUT_CONTROL: usize = 0x2E;
const SDHCI_SOFTWARE_RESET: usize = 0x2F;
const SDHCI_INT_STATUS: usize = 0x30;
const SDHCI_INT_ENABLE: usize = 0x34;
const SDHCI_SIGNAL_ENABLE: usize = 0x38;
const SDHCI_CAPABILITIES: usize = 0x40;
const SDHCI_CAPABILITIES_2: usize = 0x44;
const SDHCI_MAX_CURRENT: usize = 0x48;
const SDHCI_ADMA_ERROR_STATUS: usize = 0x54;
const SDHCI_ADMA_ADDRESS: usize = 0x58;
const SDHCI_PRESET_VALUE_INIT: usize = 0x60;
const SDHCI_PRESET_VALUE_DEFAULT: usize = 0x64;
const SDHCI_PRESET_VALUE_HIGH: usize = 0x68;
const SDHCI_SLOT_INT_STATUS: usize = 0xFC;
const SDHCI_HOST_VERSION: usize = 0xFE;

/// SD command types
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SdCommand {
    GoIdleState = 0,
    SendOpCond = 1,
    AllSendCid = 2,
    SendRca = 3,
    SetDsr = 4,
    SelectCard = 7,
    SendIfCond = 8,
    SendCsd = 9,
    SendCid = 10,
    VoltageSwitch = 11,
    StopTransmission = 12,
    SetBlockLen = 16,
    ReadSingleBlock = 17,
    ReadMultipleBlocks = 18,
    WriteBlock = 24,
    WriteMultipleBlocks = 25,
    AppCmd = 55,
    AppOpCond = 41,
}

/// SDHCI host controller
pub struct SDHCIHost {
    mmio_base: usize,
    has_64bit_dma: bool,
    voltage_18v: bool,
    voltage_30v: bool,
    voltage_33v: bool,
    clock_freq: u32,
}

impl SDHCIHost {
    /// Create a new SDHCI host controller instance
    pub fn new(mmio_base: usize) -> Self {
        SDHCIHost {
            mmio_base,
            has_64bit_dma: false,
            voltage_18v: false,
            voltage_30v: true,
            voltage_33v: true,
            clock_freq: 0,
        }
    }

    /// Read a 32-bit MMIO register
    unsafe fn read_reg32(&self, offset: usize) -> u32 {
        ptr::read_volatile((self.mmio_base + offset) as *const u32)
    }

    /// Write a 32-bit MMIO register
    unsafe fn write_reg32(&self, offset: usize, value: u32) {
        ptr::write_volatile((self.mmio_base + offset) as *mut u32, value);
    }

    /// Read a 16-bit MMIO register
    unsafe fn read_reg16(&self, offset: usize) -> u16 {
        ptr::read_volatile((self.mmio_base + offset) as *const u16)
    }

    /// Write a 16-bit MMIO register
    unsafe fn write_reg16(&self, offset: usize, value: u16) {
        ptr::write_volatile((self.mmio_base + offset) as *mut u16, value);
    }

    /// Read an 8-bit MMIO register
    unsafe fn read_reg8(&self, offset: usize) -> u8 {
        ptr::read_volatile((self.mmio_base + offset) as *const u8)
    }

    /// Write an 8-bit MMIO register
    unsafe fn write_reg8(&self, offset: usize, value: u8) {
        ptr::write_volatile((self.mmio_base + offset) as *mut u8, value);
    }

    /// Wait for a condition on a register bit
    unsafe fn wait_for_bit(&self, offset: usize, mask: u32, set: bool, timeout: u32) -> bool {
        let mut elapsed = 0;
        while elapsed < timeout {
            let val = self.read_reg32(offset);
            if ((val & mask) != 0) == set {
                return true;
            }
            elapsed += 1;
        }
        false
    }

    /// Read PCI configuration space
    unsafe fn pci_read_config(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address = (1u32 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | (offset as u32 & 0xFC);
        outb(PCI_CONFIG_ADDRESS as u16, 0);
        outb((PCI_CONFIG_ADDRESS + 1) as u16, (address >> 8) as u8);
        outb((PCI_CONFIG_ADDRESS + 2) as u16, (address >> 16) as u8);
        outb((PCI_CONFIG_ADDRESS + 3) as u16, (address >> 24) as u8);
        
        // Read 4 bytes from config data
        let low = inb(PCI_CONFIG_DATA) as u32;
        let byte1 = inb(PCI_CONFIG_DATA + 1) as u32;
        let byte2 = inb(PCI_CONFIG_DATA + 2) as u32;
        let byte3 = inb(PCI_CONFIG_DATA + 3) as u32;
        low | (byte1 << 8) | (byte2 << 16) | (byte3 << 24)
    }

    /// Find an SDHCI controller via PCI enumeration
    pub fn find_controller() -> Option<(usize, usize)> {
        unsafe {
            for bus in 0..256 {
                for slot in 0..32 {
                    for func in 0..8 {
                        let vendor_device = Self::pci_read_config(bus as u8, slot, func, 0);
                        let vendor = vendor_device & 0xFFFF;
                        let device = (vendor_device >> 16) & 0xFFFF;

                        if vendor == 0xFFFF || vendor == 0 {
                            continue;
                        }

                        let class_rev = Self::pci_read_config(bus as u8, slot, func, 0x08);
                        let class = (class_rev >> 24) & 0xFF;
                        let subclass = (class_rev >> 16) & 0xFF;
                        let prog_if = (class_rev >> 8) & 0xFF;

                        // SD Host Controller: class 0x08, subclass 0x05
                        if class == 0x08 && subclass == 0x05 {
                            let bar0 = Self::pci_read_config(bus as u8, slot, func, 0x10);
                            let mmio_base = (bar0 & 0xFFFFFFF0) as usize;
                            
                            serial_println!("SDHCI: Found controller at PCI {:02x}:{:02x}.{:02x} (vendor={:04x}, device={:04x}, BAR0=0x{:x})",
                                bus, slot, func, vendor, device, mmio_base);
                            
                            let bus_addr = bus as usize;
                            let slot_func = ((slot as usize) << 3) | (func as usize);
                            return Some((mmio_base, bus_addr * 256 + slot_func));
                        }
                    }
                }
            }
        }
        None
    }

    /// Initialize the SDHCI host controller
    pub fn init(&mut self) -> Result<(), &'static str> {
        unsafe {
            serial_println!("SDHCI: Initializing host controller at MMIO 0x{:x}", self.mmio_base);

            // Read capabilities
            let caps = self.read_reg32(SDHCI_CAPABILITIES);
            self.has_64bit_dma = (caps & (1u32 << 28)) != 0;
            
            serial_println!("SDHCI: Capabilities = 0x{:08x} (64-bit DMA: {})", caps, self.has_64bit_dma);

            // Reset the host controller
            self.write_reg8(SDHCI_SOFTWARE_RESET, 0x01); // Software reset for all
            self.wait_for_bit(SDHCI_SOFTWARE_RESET as usize, 0x01, false, 1000)?;
            serial_println!("SDHCI: Software reset complete");

            // Set power control (3.3V)
            self.write_reg8(SDHCI_POWER_CONTROL, 0x0E); // Enable 3.3V + turn on power
            self.wait_for_bit(SDHCI_POWER_CONTROL as usize, 0x02, true, 1000)?;
            serial_println!("SDHCI: Power enabled");

            // Set clock control
            self.write_reg16(SDHCI_CLOCK_CONTROL, 0x0000); // Stop clock first
            self.write_reg16(SDHCI_CLOCK_CONTROL, 0x0802); // Internal clock enable + SD clock enable (divider = 2)
            self.wait_for_bit(SDHCI_CLOCK_CONTROL as usize, 0x0002, true, 1000)?;
            serial_println!("SDHCI: Clock enabled");

            // Set block size to 512 bytes
            self.write_reg16(SDHCI_BLOCK_SIZE, 0x0200); // 512 byte block size + 2-byte DMA boundary

            // Enable interrupts
            self.write_reg16(SDHCI_INT_ENABLE as usize, 0x01FF);
            self.write_reg16(SDHCI_SIGNAL_ENABLE as usize, 0x01FF);

            serial_println!("SDHCI: Initialization complete");
            Ok(())
        }
    }

    /// Send a command to the SD card
    pub fn send_command(&self, cmd: u8, arg: u32, response_type: u8) -> Result<u32, &'static str> {
        unsafe {
            // Wait for command inhibit
            self.wait_for_bit(SDHCI_PRESENT_STATE, 0x00010000, false, 10000)?;

            // Clear any pending interrupts
            let _ = self.read_reg16(SDHCI_INT_STATUS as usize);

            // Set argument
            self.write_reg32(SDHCI_ARGUMENT, arg);

            // Build command register
            let mut cmd_reg = cmd as u16;
            cmd_reg |= (response_type as u16) << 6; // Response type select
            if cmd == 12 { cmd_reg |= 0x0400; } // Enable data present select for stop

            // Write command
            self.write_reg16(SDHCI_COMMAND, cmd_reg);

            // Wait for command complete
            self.wait_for_bit(SDHCI_INT_STATUS as usize, 0x0001, true, 10000)?;

            // Read response (use R3/R4 response for certain commands)
            let response = self.read_reg32(SDHCI_RESPONSE);
            Ok(response)
        }
    }

    /// Read a single block from the SD card
    pub fn read_block(&self, lba: u32, buffer: &mut [u8; 512]) -> Result<(), &'static str> {
        unsafe {
            // Wait for data inhibit
            self.wait_for_bit(SDHCI_PRESENT_STATE, 0x00020000, false, 10000)?;

            // Set block count to 1
            self.write_reg16(SDHCI_BLOCK_COUNT, 1);

            // Set transfer mode (block read, DMA disabled, multi-block off)
            self.write_reg16(SDHCI_TRANSFER_MODE, 0x0001);

            // Send read command
            self.send_command(17, lba, 0)?; // CMD17: READ_SINGLE_BLOCK

            // Wait for buffer ready
            self.wait_for_bit(SDHCI_PRESENT_STATE, 0x00000008, true, 10000)?;

            // Read data from buffer port
            let data_ptr = (self.mmio_base + SDHCI_BUFFER_DATA_PORT) as *const u32;
            for i in 0..128 {
                let val = ptr::read_volatile(data_ptr.add(i));
                let offset = i * 4;
                buffer[offset] = val as u8;
                buffer[offset + 1] = (val >> 8) as u8;
                buffer[offset + 2] = (val >> 16) as u8;
                buffer[offset + 3] = (val >> 24) as u8;
            }

            // Wait for transfer complete
            self.wait_for_bit(SDHCI_INT_STATUS as usize, 0x0002, true, 10000)?;

            Ok(())
        }
    }

    /// Write a single block to the SD card
    pub fn write_block(&self, lba: u32, data: &[u8; 512]) -> Result<(), &'static str> {
        unsafe {
            // Wait for data inhibit
            self.wait_for_bit(SDHCI_PRESENT_STATE, 0x00020000, false, 10000)?;

            // Set block count to 1
            self.write_reg16(SDHCI_BLOCK_COUNT, 1);

            // Set transfer mode (block write, DMA disabled, multi-block off)
            self.write_reg16(SDHCI_TRANSFER_MODE, 0x0005); // Write + block count enable

            // Send write command
            self.send_command(24, lba, 0)?; // CMD24: WRITE_BLOCK

            // Wait for buffer ready
            self.wait_for_bit(SDHCI_PRESENT_STATE, 0x00000004, true, 10000)?;

            // Write data to buffer port
            let data_ptr = (self.mmio_base + SDHCI_BUFFER_DATA_PORT) as *mut u32;
            for i in 0..128 {
                let offset = i * 4;
                let val = (data[offset] as u32)
                    | ((data[offset + 1] as u32) << 8)
                    | ((data[offset + 2] as u32) << 16)
                    | ((data[offset + 3] as u32) << 24);
                ptr::write_volatile(data_ptr.add(i), val);
            }

            // Wait for transfer complete
            self.wait_for_bit(SDHCI_INT_STATUS as usize, 0x0002, true, 10000)?;

            Ok(())
        }
    }

    /// Get the number of blocks on the SD card (placeholder)
    pub fn get_block_count(&self) -> u64 {
        // In a real implementation, we'd parse CSD register
        8388608 // 4GB / 512 = 8,388,608 blocks (placeholder)
    }

    /// Get block size
    pub fn get_block_size(&self) -> u64 {
        512 // Standard SD card block size
    }
}

// Fix typo in init function (SDHHI -> SDHCI)

