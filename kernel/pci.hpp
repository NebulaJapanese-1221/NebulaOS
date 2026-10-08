// PCI Subsystem for NebulaOS
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#pragma once

#include <cstdint>
#include <cstddef>

namespace kernel::pci {

// PCI constants
constexpr std::uint32_t MAX_BUSES = 256;
constexpr std::uint32_t MAX_DEVICES = 32;
constexpr std::uint32_t MAX_FUNCTIONS = 8;

constexpr std::uint32_t PCI_CONFIG_ADDRESS = 0xCF8;
constexpr std::uint32_t PCI_CONFIG_DATA = 0xCFC;

// PCI configuration space offsets
constexpr std::uint32_t PCI_VENDOR_ID = 0x00;
constexpr std::uint32_t PCI_DEVICE_ID = 0x02;
constexpr std::uint32_t PCI_COMMAND = 0x04;
constexpr std::uint32_t PCI_STATUS = 0x06;
constexpr std::uint32_t PCI_REVISION_ID = 0x08;
constexpr std::uint32_t PCI_CLASS_CODE = 0x09;
constexpr std::uint32_t PCI_CACHE_LINE_SIZE = 0x0C;
constexpr std::uint32_t PCI_LATENCY_TIMER = 0x0D;
constexpr std::uint32_t PCI_HEADER_TYPE = 0x0E;
constexpr std::uint32_t PCI_BIST = 0x0F;
constexpr std::uint32_t PCI_BAR0 = 0x10;
constexpr std::uint32_t PCI_BAR1 = 0x14;
constexpr std::uint32_t PCI_BAR2 = 0x18;
constexpr std::uint32_t PCI_BAR3 = 0x1C;
constexpr std::uint32_t PCI_BAR4 = 0x20;
constexpr std::uint32_t PCI_BAR5 = 0x24;
constexpr std::uint32_t PCI_INTERRUPT_LINE = 0x3C;
constexpr std::uint32_t PCI_INTERRUPT_PIN = 0x3D;
constexpr std::uint32_t PCI_MIN_GNT = 0x3E;
constexpr std::uint32_t PCI_MAX_LAT = 0x3F;

// PCI command register bits
constexpr std::uint16_t PCI_COMMAND_IO = 0x0001;
constexpr std::uint16_t PCI_COMMAND_MEMORY = 0x0002;
constexpr std::uint16_t PCI_COMMAND_MASTER = 0x0004;
constexpr std::uint16_t PCI_COMMAND_SPECIAL = 0x0008;
constexpr std::uint16_t PCI_COMMAND_INVALIDATE = 0x0010;
constexpr std::uint16_t PCI_COMMAND_VGA_PALETTE = 0x0020;
constexpr std::uint16_t PCI_COMMAND_PARITY = 0x0040;
constexpr std::uint16_t PCI_COMMAND_WAIT = 0x0080;
constexpr std::uint16_t PCI_COMMAND_SERR = 0x0100;
constexpr std::uint16_t PCI_COMMAND_FAST_BACK = 0x0200;

// PCI status register bits
constexpr std::uint16_t PCI_STATUS_CAPABILITY = 0x0010;
constexpr std::uint16_t PCI_STATUS_66MHZ = 0x0020;
constexpr std::uint16_t PCI_STATUS_UDF = 0x0040;
constexpr std::uint16_t PCI_STATUS_FAST_BACK = 0x0080;
constexpr std::uint16_t PCI_STATUS_PARITY_ERROR = 0x0100;
constexpr std::uint16_t PCI_STATUS_DEVSEL_MASK = 0x0600;
constexpr std::uint16_t PCI_STATUS_DEVSEL_FAST = 0x0000;
constexpr std::uint16_t PCI_STATUS_DEVSEL_MEDIUM = 0x0200;
constexpr std::uint16_t PCI_STATUS_DEVSEL_SLOW = 0x0400;
constexpr std::uint16_t PCI_STATUS_SIGNALED_TARGET_ABORT = 0x0800;
constexpr std::uint16_t PCI_STATUS_RECEIVED_TARGET_ABORT = 0x1000;
constexpr std::uint16_t PCI_STATUS_RECEIVED_MASTER_ABORT = 0x2000;
constexpr std::uint16_t PCI_STATUS_SIGNALED_SYSTEM_ERROR = 0x4000;
constexpr std::uint16_t PCI_STATUS_DETECTED_PARITY = 0x8000;

// PCI header types
constexpr std::uint8_t PCI_HEADER_TYPE_DEVICE = 0x00;
constexpr std::uint8_t PCI_HEADER_TYPE_BRIDGE = 0x01;
constexpr std::uint8_t PCI_HEADER_TYPE_CARDBUS = 0x02;

// Base device classes
constexpr std::uint8_t PCI_CLASS_OLD = 0x00;
constexpr std::uint8_t PCI_CLASS_MASS_STORAGE = 0x01;
constexpr std::uint8_t PCI_CLASS_NETWORK = 0x02;
constexpr std::uint8_t PCI_CLASS_DISPLAY = 0x03;
constexpr std::uint8_t PCI_CLASS_MULTIMEDIA = 0x04;
constexpr std::uint8_t PCI_CLASS_MEMORY = 0x05;
constexpr std::uint8_t PCI_CLASS_BRIDGE = 0x06;
constexpr std::uint8_t PCI_CLASS_COMMUNICATION = 0x07;
constexpr std::uint8_t PCI_CLASS_PERIPHERAL = 0x08;
constexpr std::uint8_t PCI_CLASS_INPUT = 0x09;
constexpr std::uint8_t PCI_CLASS_DOCK = 0x0A;
constexpr std::uint8_t PCI_CLASS_PROCESSOR = 0x0B;
constexpr std::uint8_t PCI_CLASS_SERIAL = 0x0C;
constexpr std::uint8_t PCI_CLASS_WIRELESS = 0x0D;
constexpr std::uint8_t PCI_CLASS_INTELLIGENT = 0x0E;
constexpr std::uint8_t PCI_CLASS_SATELLITE = 0x0F;
constexpr std::uint8_t PCI_CLASS_CRYPT = 0x10;
constexpr std::uint8_t PCI_CLASS_SIGNAL = 0x11;
constexpr std::uint8_t PCI_CLASS_SYSTEM = 0xFF;

// BAR flags
constexpr std::uint32_t PCI_BAR_IO = 0x00000001;
constexpr std::uint32_t PCI_BAR_MEM_TYPE_MASK = 0x00000006;
constexpr std::uint32_t PCI_BAR_MEM_TYPE_32 = 0x00000000;
constexpr std::uint32_t PCI_BAR_MEM_TYPE_1M = 0x00000002;
constexpr std::uint32_t PCI_BAR_MEM_TYPE_64 = 0x00000004;
constexpr std::uint32_t PCI_BAR_PREFETCHABLE = 0x00000008;

// PCI device structure
struct Device {
    std::uint8_t bus;
    std::uint8_t slot;
    std::uint8_t function;

    std::uint16_t vendor_id;
    std::uint16_t device_id;

    std::uint8_t class_code;
    std::uint8_t subclass;
    std::uint8_t programming_interface;
    std::uint8_t revision_id;

    std::uint8_t header_type;
    std::uint8_t interrupt_line;
    std::uint8_t interrupt_pin;

    std::uint32_t bar[6];
    std::uint32_t bar_size[6];

    // Driver data
    void* driver_data;

    // Linked list
    Device* next;
};

// PCI device driver interface
struct Driver {
    const char* name;
    std::uint16_t vendor_id;
    std::uint16_t device_id;
    std::uint8_t class_code;
    std::uint8_t subclass;

    bool (*probe)(Device* device);
    void (*remove)(Device* device);
    void (*shutdown)(Device* device);

    Driver* next;
};

// Result type
template<typename T>
struct Result {
    T value;
    bool ok;

    constexpr operator bool() const noexcept { return ok; }
    constexpr T& operator*() noexcept { return value; }
    constexpr const T& operator*() const noexcept { return value; }
    constexpr T* operator->() noexcept { return &value; }
    constexpr const T* operator->() const noexcept { return &value; }
};

// Initialize the PCI subsystem
bool initialize();

// Scan for PCI devices
void scan();

// Get the number of devices
std::size_t device_count() noexcept;

// Get a device by index
Device* get_device(std::size_t index) noexcept;

// Find a device by vendor and device ID
Device* find_device(std::uint16_t vendor_id,
                         std::uint16_t device_id) noexcept;

// Find a device by class code
Device* find_device_by_class(std::uint8_t class_code,
                                  std::uint8_t subclass) noexcept;

// Read a 32-bit value from PCI configuration space
std::uint32_t read_config(std::uint8_t bus, std::uint8_t slot,
                              std::uint8_t function, std::uint32_t offset);

// Write a 32-bit value to PCI configuration space
void write_config(std::uint8_t bus, std::uint8_t slot,
                      std::uint8_t function, std::uint32_t offset,
                      std::uint32_t value);

// Read a 16-bit value from PCI configuration space
std::uint16_t read_config_word(std::uint8_t bus, std::uint8_t slot,
                                    std::uint8_t function, std::uint32_t offset);

// Write a 16-bit value to PCI configuration space
void write_config_word(std::uint8_t bus, std::uint8_t slot,
                          std::uint8_t function, std::uint32_t offset,
                          std::uint16_t value);

// Read an 8-bit value from PCI configuration space
std::uint8_t read_config_byte(std::uint8_t bus, std::uint8_t slot,
                                 std::uint8_t function, std::uint32_t offset);

// Write an 8-bit value to PCI configuration space
void write_config_byte(std::uint8_t bus, std::uint8_t slot,
                          std::uint8_t function, std::uint32_t offset,
                          std::uint8_t value);

// Read from a device's configuration space
std::uint32_t read_config(Device* device, std::uint32_t offset);
void write_config(Device* device, std::uint32_t offset,
                      std::uint32_t value);
std::uint16_t read_config_word(Device* device, std::uint32_t offset);
void write_config_word(Device* device, std::uint32_t offset,
                          std::uint16_t value);
std::uint8_t read_config_byte(Device* device, std::uint32_t offset);
void write_config_byte(Device* device, std::uint32_t offset,
                          std::uint8_t value);

// Enable bus mastering
void enable_bus_mastering(Device* device);

// Enable I/O and memory access
void enable_io(Device* device);
void enable_memory(Device* device);

// Get the BAR size
std::uint32_t get_bar_size(Device* device, std::uint32_t bar_index);

// Register a PCI driver
void register_driver(Driver* driver);

// Unregister a PCI driver
void unregister_driver(Driver* driver);

// Get the interrupt line for a device
std::uint8_t get_interrupt_line(Device* device) noexcept;

// Get the interrupt pin for a device
std::uint8_t get_interrupt_pin(Device* device) noexcept;

// Check if a device is a bridge
bool is_bridge(Device* device) noexcept;

// Check if a device is a function
bool is_function(Device* device) noexcept;

// Get the class name
const char* class_name(std::uint8_t class_code) noexcept;

// Get the subclass name
const char* subclass_name(std::uint8_t class_code,
                              std::uint8_t subclass) noexcept;

} // namespace kernel::pci