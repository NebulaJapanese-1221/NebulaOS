//! Placeholder Ethernet Driver for NebulaOS.
//! This driver registers with the PCI subsystem but does not implement
//! actual network functionality yet.

use crate::kernel::pci;
use crate::kernel::net::{self, ConnectionType};
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicU16, AtomicUsize, Ordering};

// RTL8139 Registers
const REG_MAC: u8 = 0x00;        // IDR0-5
const REG_MAR: u8 = 0x08;        // Multicast Registers
const REG_RBSTART: u8 = 0x30;    // Receive Buffer Start
const REG_CMD: u8 = 0x37;        // Command Register
const REG_IMR: u8 = 0x3C;        // Interrupt Mask Register
const REG_ISR: u8 = 0x3E;        // Interrupt Status Register
const REG_CAPR: u8 = 0x38;       // Current Address of Packet Read
const REG_RCR: u8 = 0x44;        // Receive Configuration Register
const REG_CONFIG1: u8 = 0x52;    // Config 1
const REG_TX_ADDR0: u8 = 0x20;   // Transmit Start Address 0
const REG_TX_STATUS0: u8 = 0x10; // Transmit Status 0

pub static RTL8139_IO: AtomicU16 = AtomicU16::new(0);
pub static RX_BUFFER: Mutex<Option<Vec<u8>>> = Mutex::new(None);
pub static RX_OFFSET: AtomicUsize = AtomicUsize::new(0);

pub struct EthernetDriver;

impl pci::PciDriver for EthernetDriver {
    fn name(&self) -> &'static str { "Ethernet Controller (RTL8139)" }

    fn device_ids(&self) -> &[(u16, u16)] {
        // Common Vendor/Device ID for Realtek RTL8139
        &[(0x10EC, 0x8139)]
    }

    fn class_codes(&self) -> &[(u8, u8)] {
        &[(0x02, 0x00)] // Class: Network Controller, Subclass: Ethernet
    }

    fn initialize(&self, dev: &pci::PciDevice) {
        let (io_base, is_io) = dev.get_bar(0);
        if is_io {
            let port = io_base as u16;
            RTL8139_IO.store(port, Ordering::SeqCst);
            unsafe {
                // Realtek RTL8139 Wake-up
                crate::kernel::io::outb(port + REG_CONFIG1 as u16, 0x00); // Cast REG_CONFIG1 to u16
                
                // Soft Reset
                crate::kernel::io::outb(port + REG_CMD as u16, 0x10); // Cast REG_CMD to u16
                while (crate::kernel::io::inb(port + REG_CMD as u16) & 0x10) != 0 {} // Cast REG_CMD to u16

                // Read MAC address
                let mut mac = [0u8; 6];
                for i in 0..6 {
                    mac[i] = crate::kernel::io::inb(port + REG_MAC as u16 + i as u16); // Cast REG_MAC and i to u16
                }
                *net::MAC_ADDRESS.lock() = net::MacAddress(mac);

                // Setup Receive Buffer (8KB + 16 bytes + 1.5KB for wrapping safety)
                let mut rx_buf = Vec::with_capacity(8192 + 16 + 1500);
                rx_buf.resize(8192 + 16 + 1500, 0);
                crate::kernel::io::outl(port + REG_RBSTART as u16, rx_buf.as_ptr() as u32); // Cast REG_RBSTART to u16

                // Configure Receive: Accept Broadcast, Multicast, Physical Match, and wrap bit
                // 0x0F: AB + AM + APM + AAP
                // 0x80: Wrap bit (1 = wrap)
                crate::kernel::io::outl(port + REG_RCR as u16, 0x0F | 0x80); // Cast REG_RCR to u16

                // Enable Transmitter and Receiver
                crate::kernel::io::outb(port + REG_CMD as u16, 0x0C); // Cast REG_CMD to u16
                *RX_BUFFER.lock() = Some(rx_buf);
            }
            crate::serial_println!("[ETHERNET] RTL8139 Ready. Hardware initialized.");
        }
        
        net::set_connection(ConnectionType::Ethernet, 100);
    }

    fn detach(&self, _dev: &pci::PciDevice) {
        net::set_connection(ConnectionType::None, 0);
        crate::serial_println!("[ETHERNET] RTL8139 detached.");
    }
}

pub static ETHERNET_DRIVER: EthernetDriver = EthernetDriver;