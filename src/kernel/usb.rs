//! USB Support and Mass Storage Driver for NebulaOS.

use crate::kernel::pci;
use alloc::vec::Vec;
use spin::Mutex;
use alloc::string::String;

pub struct UsbDrive {
    pub name: String,
    pub size_mb: usize,
    pub port: u8,
}

/// Tracks detected mass storage devices globally for access by the File Manager.
pub static DETECTED_DRIVES: Mutex<Vec<UsbDrive>> = Mutex::new(Vec::new());

pub struct UsbDriver;

impl pci::PciDriver for UsbDriver {
    fn name(&self) -> &'static str { "USB Controller (UHCI/EHCI)" }

    fn device_ids(&self) -> &[(u16, u16)] { &[] }

    fn class_codes(&self) -> &[(u8, u8)] {
        &[(0x0C, 0x03)] // Class: Serial Bus, Subclass: USB
    }

    fn initialize(&self, dev: &pci::PciDevice) {
        let (base, is_io) = dev.get_bar(0);
        crate::serial_println!("[USB] Initializing Controller at {:#x} ({})", base, if is_io { "I/O" } else { "MMIO" });

        // Mock: In a full implementation, we would enumerate the USB bus here.
        // For now, we simulate finding a storage device when the controller is found.
        let mut drives = DETECTED_DRIVES.lock();
        drives.push(UsbDrive {
            name: String::from("USB Flash Drive"),
            size_mb: 4096,
            port: 1,
        });
        
        crate::serial_println!("[USB] Found Mass Storage Device: USB Flash Drive (4GB)");
    }

    fn detach(&self, _dev: &pci::PciDevice) {
        let mut drives = DETECTED_DRIVES.lock();
        drives.clear();
        crate::serial_println!("[USB] Controller detached, all USB drives removed.");
    }
}

pub static USB_DRIVER: UsbDriver = UsbDriver;