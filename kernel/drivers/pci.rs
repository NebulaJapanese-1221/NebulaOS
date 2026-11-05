// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

// PCI configuration space enumeration

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

const PCI_CLASS_STORAGE: u8 = 0x01;
const PCI_CLASS_NETWORK: u8 = 0x02;
const PCI_CLASS_DISPLAY: u8 = 0x03;
const PCI_CLASS_MULTIMEDIA: u8 = 0x04;
const PCI_CLASS_BRIDGE: u8 = 0x06;
const PCI_CLASS_SERIAL: u8 = 0x0C;
const PCI_CLASS_AUDIO: u8 = 0x04;

const MAX_PCI_DEVICES: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision_id: u8,
    pub header_type: u8,
    pub irq_line: u8,
}

static mut PCI_DEVICES: [PciDevice; MAX_PCI_DEVICES] = unsafe {
    core::mem::zeroed()
};
static mut PCI_DEVICE_COUNT: usize = 0;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

fn port_dword_out(port: u16, data: u32) {
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") data);
    }
}

fn port_dword_in(port: u16) -> u32 {
    let val: u32;
    unsafe {
        core::arch::asm!("in eax, dx", out("eax") val, in("dx") port);
    }
    val
}

fn pci_config_address(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32 & 0xFC)
}

fn pci_read_dword(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    port_dword_out(PCI_CONFIG_ADDRESS, pci_config_address(bus, device, function, offset));
    port_dword_in(PCI_CONFIG_DATA)
}

fn pci_read_word(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let value = pci_read_dword(bus, device, function, offset);
    match offset % 4 {
        0 => (value & 0xFFFF) as u16,
        2 => ((value >> 16) & 0xFFFF) as u16,
        _ => 0,
    }
}

fn pci_read_byte(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let value = pci_read_dword(bus, device, function, offset);
    match offset % 4 {
        0 => value as u8,
        1 => ((value >> 8) & 0xFF) as u8,
        2 => ((value >> 16) & 0xFF) as u8,
        3 => ((value >> 24) & 0xFF) as u8,
        _ => 0,
    }
}

fn pci_multi_function(bus: u8, device: u8) -> bool {
    pci_read_byte(bus, device, 0, 0x0E) & 0x80 != 0
}

fn class_name(class: u8, subclass: u8) -> &'static str {
    match class {
        PCI_CLASS_STORAGE => match subclass {
            0x01 => "IDE controller",
            0x06 => "SATA controller",
            0x07 => "Serial ATA",
            _ => "storage controller",
        },
        PCI_CLASS_NETWORK => "network controller",
        PCI_CLASS_DISPLAY => "display controller",
        PCI_CLASS_MULTIMEDIA => match subclass {
            0x01 => "audio device",
            _ => "multimedia device",
        },
        PCI_CLASS_BRIDGE => "bridge device",
        PCI_CLASS_SERIAL => "serial bus controller",
        PCI_CLASS_AUDIO => "audio device",
        _ => "unknown device",
    }
}

pub fn enumerate() -> usize {
    unsafe {
        PCI_DEVICE_COUNT = 0;
    }

    for bus in 0..=255u8 {
        for device in 0..32u8 {
            let vendor_id = pci_read_word(bus, device, 0, 0x00);
            if vendor_id == 0xFFFF {
                continue;
            }

            let functions = if pci_multi_function(bus, device) {
                8
            } else {
                1
            };

            for function in 0..functions {
                let vendor_id = pci_read_word(bus, device, function, 0x00);
                if vendor_id == 0xFFFF {
                    continue;
                }

                let device_id = pci_read_word(bus, device, function, 0x02);
                let class = pci_read_byte(bus, device, function, 0x0B);
                let subclass = pci_read_byte(bus, device, function, 0x0A);
                let prog_if = pci_read_byte(bus, device, function, 0x09);
                let revision_id = pci_read_byte(bus, device, function, 0x08);
                let header_type = pci_read_byte(bus, device, function, 0x0E) & 0x7F;
                let irq_line = pci_read_byte(bus, device, function, 0x3C);

                unsafe {
                    if PCI_DEVICE_COUNT < MAX_PCI_DEVICES {
                        let index = PCI_DEVICE_COUNT;
                        PCI_DEVICES[index] = PciDevice {
                            bus,
                            device,
                            function,
                            vendor_id,
                            device_id,
                            class,
                            subclass,
                            prog_if,
                            revision_id,
                            header_type,
                            irq_line,
                        };
                        PCI_DEVICE_COUNT = index + 1;
                    }
                }

                crate::drivers::serial::write_string("PCI: ");
                crate::drivers::serial::write_dec(bus as u32);
                crate::drivers::serial::write_string(":");
                crate::drivers::serial::write_dec(device as u32);
                crate::drivers::serial::write_string(":");
                crate::drivers::serial::write_dec(function as u32);
                crate::drivers::serial::write_string(" vendor=0x");
                crate::drivers::serial::write_hex(vendor_id as u32);
                crate::drivers::serial::write_string(" device=0x");
                crate::drivers::serial::write_hex(device_id as u32);
                crate::drivers::serial::write_string(" class=");
                crate::drivers::serial::write_string(class_name(class, subclass));
                crate::drivers::serial::write_string("\r\n");
            }
        }
    }

    unsafe { PCI_DEVICE_COUNT }
}

pub fn device_count() -> usize {
    unsafe { PCI_DEVICE_COUNT }
}

pub fn device_at(index: usize) -> Option<PciDevice> {
    unsafe {
        if index < PCI_DEVICE_COUNT {
            Some(PCI_DEVICES[index])
        } else {
            None
        }
    }
}

pub fn find_by_class(class: u8) -> Option<PciDevice> {
    unsafe {
        for index in 0..PCI_DEVICE_COUNT {
            if PCI_DEVICES[index].class == class {
                return Some(PCI_DEVICES[index]);
            }
        }
        None
    }
}

pub fn find_by_class_and_subclass(class: u8, subclass: u8) -> Option<PciDevice> {
    unsafe {
        for index in 0..PCI_DEVICE_COUNT {
            if PCI_DEVICES[index].class == class && PCI_DEVICES[index].subclass == subclass {
                return Some(PCI_DEVICES[index]);
            }
        }
        None
    }
}

pub fn init() {
    let count = enumerate();
    crate::drivers::serial::write_string("PCI enumeration complete: ");
    crate::drivers::serial::write_dec(count as u32);
    crate::drivers::serial::write_string(" devices\r\n");
}
