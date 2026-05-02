//! PCI Bus Scanning and Configuration for NebulaOS.

use alloc::vec::Vec;
use spin::Mutex;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
}

impl PciDevice {
    pub fn read_config_u32(&self, offset: u8) -> u32 {
        unsafe { pci_config_read_u32(self.bus, self.device, self.function, offset) }
    }

    /// Returns the BAR address and whether it is I/O mapped.
    pub fn get_bar(&self, bar_index: u8) -> (u32, bool) {
        let bar = self.read_config_u32(0x10 + (bar_index * 4));
        let is_io = (bar & 1) != 0;
        let address = if is_io { bar & !0x3 } else { bar & !0xF };
        (address, is_io)
    }
}

/// Interface for all PCI hardware drivers.
pub trait PciDriver: Send + Sync {
    /// The human-readable name of the driver.
    fn name(&self) -> &'static str;
    /// List of (Vendor ID, Device ID) pairs this driver supports.
    fn device_ids(&self) -> &[(u16, u16)];
    /// Optional list of (Class, Subclass) pairs this driver supports.
    fn class_codes(&self) -> &[(u8, u8)] { &[] }
    /// Initialization logic called when a matching device is found.
    fn initialize(&self, device: &PciDevice);
    /// Cleanup logic called when a device is removed from the bus.
    fn detach(&self, device: &PciDevice);
}

/// Tracks a device that has been successfully matched with a driver.
struct AttachedDevice {
    device: PciDevice,
    driver: &'static dyn PciDriver,
}

static ATTACHED_DEVICES: Mutex<Vec<AttachedDevice>> = Mutex::new(Vec::new());

unsafe fn pci_config_read_u32(bus: u8, device: u8, func: u8, offset: u8) -> u32 {
    let address = ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x80000000;

    // outl equivalent
    core::arch::asm!("out dx, eax", in("dx") PCI_CONFIG_ADDRESS, in("eax") address, options(nomem, nostack, preserves_flags));
    
    let data: u32;
    // inl equivalent
    core::arch::asm!("in eax, dx", out("eax") data, in("dx") PCI_CONFIG_DATA, options(nomem, nostack, preserves_flags));
    data
}

/// Scans the PCI bus and updates driver attachments based on current hardware state.
pub fn init_drivers() {
    rescan_bus();
}

/// Performs a stateful scan of the PCI bus to handle discovery and hot-unplug.
pub fn rescan_bus() {
    // Central registry of all compiled-in PCI drivers.
    let drivers: &[&dyn PciDriver] = &[
        &crate::kernel::audio::AC97_DRIVER,
        &crate::kernel::usb::USB_DRIVER,
    ];

    let mut attached = ATTACHED_DEVICES.lock();
    let mut found_this_scan: Vec<(u8, u8, u8)> = Vec::new();

    for bus in 0..=255 {
        for dev in 0..31 {
            // Check Vendor ID of function 0 to see if device exists
            let val = unsafe { pci_config_read_u32(bus as u8, dev as u8, 0, 0) };
            let vendor = (val & 0xFFFF) as u16;
            
            if vendor == 0xFFFF { continue; }

            for func in 0..8 {
                let id_reg = unsafe { pci_config_read_u32(bus as u8, dev as u8, func as u8, 0) };
                let v = (id_reg & 0xFFFF) as u16;
                let d = (id_reg >> 16) as u16;

                if v == 0xFFFF { continue; }

                let class_reg = unsafe { pci_config_read_u32(bus as u8, dev as u8, func as u8, 0x08) };
                let class = (class_reg >> 24) as u8;
                let subclass = (class_reg >> 16) as u8;

                let pci_dev = PciDevice { 
                    bus: bus as u8, device: dev as u8, function: func as u8, 
                    vendor_id: v, device_id: d, class, subclass 
                };
                found_this_scan.push((bus as u8, dev as u8, func as u8));

                // Only initialize if not already tracked
                if !attached.iter().any(|a| a.device.bus == pci_dev.bus && a.device.device == pci_dev.device && a.device.function == pci_dev.function) {
                    // Attempt to match discovered hardware with a driver
                    for driver in drivers {
                        for (drv_v, drv_d) in driver.device_ids() {
                            if *drv_v == v && *drv_d == d {
                                crate::serial_println!("[PCI] Discovery: Attaching {} to {}:{}:{}", driver.name(), bus, dev, func);
                                driver.initialize(&pci_dev);
                                attached.push(AttachedDevice { device: pci_dev, driver: *driver });
                            }
                        }

                        // Match by Class/Subclass (Generic Drivers)
                        for (c, s) in driver.class_codes() {
                            if *c == class && *s == subclass {
                                crate::serial_println!("[PCI] Discovery: Attaching {} (Generic) to {}:{}:{}", driver.name(), bus, dev, func);
                                driver.initialize(&pci_dev);
                                attached.push(AttachedDevice { device: pci_dev, driver: *driver });
                            }
                        }
                    }
                }

                if func == 0 && (unsafe { pci_config_read_u32(bus as u8, dev as u8, 0, 0x0C) } & 0x800000) == 0 {
                    break;
                }
            }
        }
    }

    // Handle Removal: Find devices in 'attached' that were NOT found this scan
    let mut i = 0;
    while i < attached.len() {
        let a = &attached[i];
        if !found_this_scan.iter().any(|&(b, d, f)| a.device.bus == b && a.device.device == d && a.device.function == f) {
            crate::serial_println!("[PCI] Removal: Detaching {} from {}:{}:{}", a.driver.name(), a.device.bus, a.device.device, a.device.function);
            a.driver.detach(&a.device);
            attached.remove(i);
        } else {
            i += 1;
        }
    }
}

pub fn find_device(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    for bus in 0..=255 {
        for dev in 0..31 {
            // Check Vendor ID of function 0
            let val = unsafe { pci_config_read_u32(bus as u8, dev as u8, 0, 0) };
            let vendor = (val & 0xFFFF) as u16;
            
            if vendor == 0xFFFF { continue; } // Device doesn't exist

            // Check all 8 functions
            for func in 0..8 {
                let val = unsafe { pci_config_read_u32(bus as u8, dev as u8, func as u8, 0) };
                let v = (val & 0xFFFF) as u16;
                let d = (val >> 16) as u16;

                if v == vendor_id && d == device_id {
                    let class_reg = unsafe { pci_config_read_u32(bus as u8, dev as u8, func as u8, 0x08) };
                    return Some(PciDevice {
                        bus: bus as u8,
                        device: dev as u8,
                        function: func as u8,
                        vendor_id: v,
                        device_id: d,
                        class: (class_reg >> 24) as u8,
                        subclass: (class_reg >> 16) as u8,
                    });
                }
                
                // If header type bit 7 is 0, it's a single-function device
                if func == 0 && (unsafe { pci_config_read_u32(bus as u8, dev as u8, 0, 0x0C) } & 0x800000) == 0 {
                    break;
                }
            }
        }
    }
    None
}