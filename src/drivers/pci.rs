//! PCI Bus Scanning and Device Discovery.

use crate::kernel::io;

pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_id: u8,
    pub subclass_id: u8,
}

/// Reads a 32-bit value from the PCI configuration space.
pub fn read_config_32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset & 0xFC) as u32 | 0x80000000;
    unsafe {
        io::outl(0xCF8, address);
        io::inl(0xCFC)
    }
}

/// Scans the PCI bus for an AC97-compatible audio controller.
/// Returns (BAR0/Mixer, BAR1/BusMaster) if found.
pub fn find_ac97_device() -> Option<(u16, u16)> {
    for bus in 0..256 {
        for slot in 0..32 {
            let vendor_id = (read_config_32(bus as u8, slot as u8, 0, 0) & 0xFFFF) as u16;
            if vendor_id == 0xFFFF { continue; }

            let class_info = read_config_32(bus as u8, slot as u8, 0, 0x08);
            let class_id = (class_info >> 24) as u8;
            let subclass_id = (class_info >> 16) as u8;

            // Class 0x04 (Multimedia), Subclass 0x01 (Audio)
            if class_id == 0x04 && subclass_id == 0x01 {
                let bar0 = read_config_32(bus as u8, slot as u8, 0, 0x10);
                let bar1 = read_config_32(bus as u8, slot as u8, 0, 0x14);
                
                // We only care about I/O space BARs (bit 0 must be 1)
                if (bar0 & 1) != 0 && (bar1 & 1) != 0 {
                    return Some((
                        (bar0 & !0x3) as u16, 
                        (bar1 & !0x3) as u16
                    ));
                }
            }
        }
    }
    None
}