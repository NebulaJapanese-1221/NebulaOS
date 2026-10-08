// PCI Subsystem Implementation for NebulaOS
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

#include "pci.hpp"
#include "heap_new.hpp"
#include "../drivers/serial.hpp"
#include <cstring>

namespace kernel::pci {

namespace {

// Device list
Device* device_list = nullptr;
std::size_t device_count_value = 0;

// Driver list
Driver* driver_list = nullptr;

// PCI initialized
bool pci_initialized = false;

// Helper: create a configuration address
inline std::uint32_t create_address(std::uint8_t bus,
                                          std::uint8_t slot,
                                          std::uint8_t function,
                                          std::uint32_t offset) {
    return (1U << 31) |
           (static_cast<std::uint32_t>(bus) << 16) |
           (static_cast<std::uint32_t>(slot) << 11) |
           (static_cast<std::uint32_t>(function) << 8) |
           (offset & 0xFC);
}

// Helper: read a 32-bit value from PCI configuration space
inline std::uint32_t read_raw(std::uint8_t bus, std::uint8_t slot,
                                    std::uint8_t function,
                                    std::uint32_t offset) {
    const std::uint32_t address = create_address(bus, slot, function, offset);

    // Write the address to the configuration address port
    asm volatile("outl %0, %1"
                 :
                 : "a"(address), "Nd"(PCI_CONFIG_ADDRESS));

    // Read the data from the configuration data port
    std::uint32_t value;
    asm volatile("inl %1, %0"
                 : "=a"(value)
                 : "Nd"(PCI_CONFIG_DATA));

    return value;
}

// Helper: write a 32-bit value to PCI configuration space
inline void write_raw(std::uint8_t bus, std::uint8_t slot,
                           std::uint8_t function,
                           std::uint32_t offset,
                           std::uint32_t value) {
    const std::uint32_t address = create_address(bus, slot, function, offset);

    // Write the address to the configuration address port
    asm volatile("outl %0, %1"
                 :
                 : "a"(address), "Nd"(PCI_CONFIG_ADDRESS));

    // Write the data to the configuration data port
    asm volatile("outl %0, %1"
                 :
                 : "a"(value), "Nd"(PCI_CONFIG_DATA));
}

// Helper: read a 16-bit value
inline std::uint16_t read_word_raw(std::uint8_t bus, std::uint8_t slot,
                                         std::uint8_t function,
                                         std::uint32_t offset) {
    const std::uint32_t value = read_raw(bus, slot, function, offset);
    const std::uint32_t shift = (offset & 2) * 8;
    return (value >> shift) & 0xFFFF;
}

// Helper: write a 16-bit value
inline void write_word_raw(std::uint8_t bus, std::uint8_t slot,
                                 std::uint8_t function,
                                 std::uint32_t offset,
                                 std::uint16_t value) {
    const std::uint32_t shift = (offset & 2) * 8;
    const std::uint32_t mask = 0xFFFF << shift;
    const std::uint32_t old = read_raw(bus, slot, function, offset);
    const std::uint32_t new_value =
        (old & ~mask) | (static_cast<std::uint32_t>(value) << shift);
    write_raw(bus, slot, function, offset, new_value);
}

// Helper: read an 8-bit value
inline std::uint8_t read_byte_raw(std::uint8_t bus, std::uint8_t slot,
                                        std::uint8_t function,
                                        std::uint32_t offset) {
    const std::uint32_t value = read_raw(bus, slot, function, offset);
    const std::uint32_t shift = (offset & 3) * 8;
    return (value >> shift) & 0xFF;
}

// Helper: write an 8-bit value
inline void write_byte_raw(std::uint8_t bus, std::uint8_t slot,
                                 std::uint8_t function,
                                 std::uint32_t offset,
                                 std::uint8_t value) {
    const std::uint32_t shift = (offset & 3) * 8;
    const std::uint32_t mask = 0xFF << shift;
    const std::uint32_t old = read_raw(bus, slot, function, offset);
    const std::uint32_t new_value =
        (old & ~mask) | (static_cast<std::uint32_t>(value) << shift);
    write_raw(bus, slot, function, offset, new_value);
}

// Helper: check if a device is present
bool device_present(std::uint8_t bus, std::uint8_t slot,
                         std::uint8_t function) {
    const std::uint32_t vendor_id = read_raw(bus, slot, function,
                                                  PCI_VENDOR_ID);
    return vendor_id != 0xFFFFFFFF && vendor_id != 0x00000000;
}

// Helper: probe a device
void probe_device(std::uint8_t bus, std::uint8_t slot,
                       std::uint8_t function) {
    if (!device_present(bus, slot, function)) {
        return;
    }

    // Read device information
    const std::uint32_t vendor_device = read_raw(bus, slot, function,
                                                      PCI_VENDOR_ID);
    const std::uint16_t vendor_id = vendor_device & 0xFFFF;
    const std::uint16_t device_id = (vendor_device >> 16) & 0xFFFF;

    const std::uint32_t class_info = read_raw(bus, slot, function,
                                                     PCI_CLASS_CODE);
    const std::uint8_t class_code = (class_info >> 24) & 0xFF;
    const std::uint8_t subclass = (class_info >> 16) & 0xFF;
    const std::uint8_t programming_interface = (class_info >> 8) & 0xFF;
    const std::uint8_t revision_id = class_info & 0xFF;

    const std::uint8_t header_type = read_byte_raw(bus, slot, function,
                                                          PCI_HEADER_TYPE);
    const std::uint8_t interrupt_line = read_byte_raw(bus, slot, function,
                                                            PCI_INTERRUPT_LINE);
    const std::uint8_t interrupt_pin = read_byte_raw(bus, slot, function,
                                                           PCI_INTERRUPT_PIN);

    // Allocate device structure
    Device* device = static_cast<Device*>(
        heap::allocate(sizeof(Device)));
    if (device == nullptr) {
        return;
    }

    std::memset(device, 0, sizeof(Device));

    device->bus = bus;
    device->slot = slot;
    device->function = function;
    device->vendor_id = vendor_id;
    device->device_id = device_id;
    device->class_code = class_code;
    device->subclass = subclass;
    device->programming_interface = programming_interface;
    device->revision_id = revision_id;
    device->header_type = header_type & 0x7F;
    device->interrupt_line = interrupt_line;
    device->interrupt_pin = interrupt_pin;
    device->driver_data = nullptr;
    device->next = nullptr;

    // Read BARs
    for (std::uint32_t i = 0; i < 6; ++i) {
        const std::uint32_t bar_offset = PCI_BAR0 + i * 4;
        device->bar[i] = read_raw(bus, slot, function, bar_offset);
        device->bar_size[i] = 0;

        // Determine BAR size
        if ((device->bar[i] & PCI_BAR_IO) != 0) {
            // I/O BAR
            write_raw(bus, slot, function, bar_offset, 0xFFFFFFFF);
            const std::uint32_t size = read_raw(bus, slot, function,
                                                     bar_offset);
            write_raw(bus, slot, function, bar_offset, device->bar[i]);
            device->bar_size[i] = ~(size & 0xFFFFFFFC) + 1;
        } else {
            // Memory BAR
            write_raw(bus, slot, function, bar_offset, 0xFFFFFFFF);
            const std::uint32_t size = read_raw(bus, slot, function,
                                                     bar_offset);
            write_raw(bus, slot, function, bar_offset, device->bar[i]);

            const std::uint32_t mem_type =
                size & PCI_BAR_MEM_TYPE_MASK;
            if (mem_type == PCI_BAR_MEM_TYPE_64) {
                // 64-bit BAR, read the upper 32 bits
                const std::uint32_t bar_offset_upper =
                    bar_offset + 4;
                write_raw(bus, slot, function, bar_offset_upper,
                              0xFFFFFFFF);
                const std::uint32_t size_upper = read_raw(bus, slot,
                                                               function,
                                                               bar_offset_upper);
                write_raw(bus, slot, function, bar_offset_upper,
                              device->bar[i + 1]);
                // Combine sizes (simplified)
                device->bar_size[i] = ~(size & 0xFFFFFFF0) + 1;
                device->bar_size[i + 1] = ~(size_upper & 0xFFFFFFFF) + 1;
                ++i;  // Skip the upper 32 bits
            } else {
                device->bar_size[i] = ~(size & 0xFFFFFFF0) + 1;
            }
        }
    }

    // Add to device list
    if (device_list == nullptr) {
        device_list = device;
    } else {
        Device* tail = device_list;
        while (tail->next != nullptr) {
            tail = tail->next;
        }
        tail->next = device;
    }

    ++device_count_value;

    // Try to match a driver
    for (Driver* driver = driver_list; driver != nullptr;
         driver = driver->next) {
        bool match = true;

        if (driver->vendor_id != 0 &&
            driver->vendor_id != vendor_id) {
            match = false;
        }
        if (driver->device_id != 0 &&
            driver->device_id != device_id) {
            match = false;
        }
        if (driver->class_code != 0 &&
            driver->class_code != class_code) {
            match = false;
        }
        if (driver->subclass != 0 &&
            driver->subclass != subclass) {
            match = false;
        }

        if (match && driver->probe != nullptr) {
            if (driver->probe(device)) {
                device->driver_data = driver;
                break;
            }
        }
    }

    // Log the device
    char buffer[128];
    std::snprintf(buffer, sizeof(buffer),
                  "[pci] %02x:%02x.%x %04x:%04x %s\n",
                  bus, slot, function, vendor_id, device_id,
                  class_name(class_code));
    drivers::serial::write_line(buffer);
}

// Helper: scan a bus
void scan_bus(std::uint8_t bus) {
    for (std::uint8_t slot = 0; slot < MAX_DEVICES; ++slot) {
        // Check function 0
        probe_device(bus, slot, 0);

        // Check for multi-function devices
        if (device_present(bus, slot, 0)) {
            const std::uint8_t header_type =
                read_byte_raw(bus, slot, 0, PCI_HEADER_TYPE);
            if ((header_type & 0x80) != 0) {
                // Multi-function device
                for (std::uint8_t function = 1;
                     function < MAX_FUNCTIONS;
                     ++function) {
                    probe_device(bus, slot, function);
                }
            }
        }
    }
}

} // namespace

bool initialize() {
    if (pci_initialized) {
        return true;
    }

    device_list = nullptr;
    device_count_value = 0;
    driver_list = nullptr;

    pci_initialized = true;

    return true;
}

void scan() {
    if (!pci_initialized) {
        return;
    }

    // Scan all buses
    for (std::uint8_t bus = 0; bus < MAX_BUSES; ++bus) {
        scan_bus(bus);
    }
}

std::size_t device_count() noexcept {
    return device_count_value;
}

Device* get_device(std::size_t index) noexcept {
    std::size_t i = 0;
    for (Device* device = device_list; device != nullptr;
         device = device->next) {
        if (i == index) {
            return device;
        }
        ++i;
    }
    return nullptr;
}

Device* find_device(std::uint16_t vendor_id,
                         std::uint16_t device_id) noexcept {
    for (Device* device = device_list; device != nullptr;
         device = device->next) {
        if (device->vendor_id == vendor_id &&
            device->device_id == device_id) {
            return device;
        }
    }
    return nullptr;
}

Device* find_device_by_class(std::uint8_t class_code,
                                  std::uint8_t subclass) noexcept {
    for (Device* device = device_list; device != nullptr;
         device = device->next) {
        if (device->class_code == class_code &&
            device->subclass == subclass) {
            return device;
        }
    }
    return nullptr;
}

std::uint32_t read_config(std::uint8_t bus, std::uint8_t slot,
                              std::uint8_t function, std::uint32_t offset) {
    return read_raw(bus, slot, function, offset & 0xFC);
}

void write_config(std::uint8_t bus, std::uint8_t slot,
                      std::uint8_t function, std::uint32_t offset,
                      std::uint32_t value) {
    write_raw(bus, slot, function, offset & 0xFC, value);
}

std::uint16_t read_config_word(std::uint8_t bus, std::uint8_t slot,
                                    std::uint8_t function, std::uint32_t offset) {
    return read_word_raw(bus, slot, function, offset);
}

void write_config_word(std::uint8_t bus, std::uint8_t slot,
                          std::uint8_t function, std::uint32_t offset,
                          std::uint16_t value) {
    write_word_raw(bus, slot, function, offset, value);
}

std::uint8_t read_config_byte(std::uint8_t bus, std::uint8_t slot,
                                 std::uint8_t function, std::uint32_t offset) {
    return read_byte_raw(bus, slot, function, offset);
}

void write_config_byte(std::uint8_t bus, std::uint8_t slot,
                          std::uint8_t function, std::uint32_t offset,
                          std::uint8_t value) {
    write_byte_raw(bus, slot, function, offset, value);
}

std::uint32_t read_config(Device* device, std::uint32_t offset) {
    if (device == nullptr) {
        return 0;
    }
    return read_raw(device->bus, device->slot, device->function,
                         offset & 0xFC);
}

void write_config(Device* device, std::uint32_t offset,
                      std::uint32_t value) {
    if (device == nullptr) {
        return;
    }
    write_raw(device->bus, device->slot, device->function,
                   offset & 0xFC, value);
}

std::uint16_t read_config_word(Device* device, std::uint32_t offset) {
    if (device == nullptr) {
        return 0;
    }
    return read_word_raw(device->bus, device->slot, device->function, offset);
}

void write_config_word(Device* device, std::uint32_t offset,
                          std::uint16_t value) {
    if (device == nullptr) {
        return;
    }
    write_word_raw(device->bus, device->slot, device->function, offset, value);
}

std::uint8_t read_config_byte(Device* device, std::uint32_t offset) {
    if (device == nullptr) {
        return 0;
    }
    return read_byte_raw(device->bus, device->slot, device->function, offset);
}

void write_config_byte(Device* device, std::uint32_t offset,
                          std::uint8_t value) {
    if (device == nullptr) {
        return;
    }
    write_byte_raw(device->bus, device->slot, device->function, offset, value);
}

void enable_bus_mastering(Device* device) {
    if (device == nullptr) {
        return;
    }
    const std::uint16_t command =
        read_config_word(device, PCI_COMMAND);
    write_config_word(device, PCI_COMMAND,
                          command | PCI_COMMAND_MASTER);
}

void enable_io(Device* device) {
    if (device == nullptr) {
        return;
    }
    const std::uint16_t command =
        read_config_word(device, PCI_COMMAND);
    write_config_word(device, PCI_COMMAND,
                          command | PCI_COMMAND_IO);
}

void enable_memory(Device* device) {
    if (device == nullptr) {
        return;
    }
    const std::uint16_t command =
        read_config_word(device, PCI_COMMAND);
    write_config_word(device, PCI_COMMAND,
                          command | PCI_COMMAND_MEMORY);
}

std::uint32_t get_bar_size(Device* device, std::uint32_t bar_index) {
    if (device == nullptr || bar_index >= 6) {
        return 0;
    }
    return device->bar_size[bar_index];
}

void register_driver(Driver* driver) {
    if (driver == nullptr) {
        return;
    }
    driver->next = driver_list;
    driver_list = driver;

    // Try to match the driver to existing devices
    for (Device* device = device_list; device != nullptr;
         device = device->next) {
        bool match = true;

        if (driver->vendor_id != 0 &&
            driver->vendor_id != device->vendor_id) {
            match = false;
        }
        if (driver->device_id != 0 &&
            driver->device_id != device->device_id) {
            match = false;
        }
        if (driver->class_code != 0 &&
            driver->class_code != device->class_code) {
            match = false;
        }
        if (driver->subclass != 0 &&
            driver->subclass != device->subclass) {
            match = false;
        }

        if (match && driver->probe != nullptr) {
            if (driver->probe(device)) {
                device->driver_data = driver;
            }
        }
    }
}

void unregister_driver(Driver* driver) {
    if (driver == nullptr) {
        return;
    }

    // Remove from driver list
    if (driver_list == driver) {
        driver_list = driver->next;
    } else {
        for (Driver* d = driver_list; d != nullptr; d = d->next) {
            if (d->next == driver) {
                d->next = driver->next;
                break;
            }
        }
    }

    // Remove driver data from devices
    for (Device* device = device_list; device != nullptr;
         device = device->next) {
        if (device->driver_data == driver) {
            if (driver->remove != nullptr) {
                driver->remove(device);
            }
            device->driver_data = nullptr;
        }
    }

    driver->next = nullptr;
}

std::uint8_t get_interrupt_line(Device* device) noexcept {
    if (device == nullptr) {
        return 0;
    }
    return device->interrupt_line;
}

std::uint8_t get_interrupt_pin(Device* device) noexcept {
    if (device == nullptr) {
        return 0;
    }
    return device->interrupt_pin;
}

bool is_bridge(Device* device) noexcept {
    if (device == nullptr) {
        return false;
    }
    return device->header_type == PCI_HEADER_TYPE_BRIDGE;
}

bool is_function(Device* device) noexcept {
    if (device == nullptr) {
        return false;
    }
    return device->function != 0;
}

const char* class_name(std::uint8_t class_code) noexcept {
    switch (class_code) {
        case PCI_CLASS_OLD: return "Old";
        case PCI_CLASS_MASS_STORAGE: return "Mass Storage";
        case PCI_CLASS_NETWORK: return "Network";
        case PCI_CLASS_DISPLAY: return "Display";
        case PCI_CLASS_MULTIMEDIA: return "Multimedia";
        case PCI_CLASS_MEMORY: return "Memory";
        case PCI_CLASS_BRIDGE: return "Bridge";
        case PCI_CLASS_COMMUNICATION: return "Communication";
        case PCI_CLASS_PERIPHERAL: return "Peripheral";
        case PCI_CLASS_INPUT: return "Input";
        case PCI_CLASS_DOCK: return "Dock";
        case PCI_CLASS_PROCESSOR: return "Processor";
        case PCI_CLASS_SERIAL: return "Serial";
        case PCI_CLASS_WIRELESS: return "Wireless";
        case PCI_CLASS_INTELLIGENT: return "Intelligent";
        case PCI_CLASS_SATELLITE: return "Satellite";
        case PCI_CLASS_CRYPT: return "Crypt";
        case PCI_CLASS_SIGNAL: return "Signal";
        case PCI_CLASS_SYSTEM: return "System";
        default: return "Unknown";
    }
}

const char* subclass_name(std::uint8_t class_code,
                              std::uint8_t subclass) noexcept {
    switch (class_code) {
        case PCI_CLASS_MASS_STORAGE:
            switch (subclass) {
                case 0x00: return "SCSI";
                case 0x01: return "IDE";
                case 0x02: return "Floppy";
                case 0x03: return "IPI";
                case 0x04: return "RAID";
                case 0x05: return "ATA";
                case 0x06: return "SATA";
                case 0x07: return "SAS";
                case 0x08: return "NVM";
                default: return "Unknown";
            }
        case PCI_CLASS_NETWORK:
            switch (subclass) {
                case 0x00: return "Ethernet";
                case 0x01: return "Token Ring";
                case 0x02: return "FDDI";
                case 0x03: return "ATM";
                case 0x04: return "ISDN";
                case 0x05: return "WorldFip";
                case 0x06: return "PICMG";
                default: return "Unknown";
            }
        case PCI_CLASS_DISPLAY:
            switch (subclass) {
                case 0x00: return "VGA";
                case 0x01: return "XGA";
                case 0x02: return "3D";
                default: return "Unknown";
            }
        case PCI_CLASS_BRIDGE:
            switch (subclass) {
                case 0x00: return "Host/PCI";
                case 0x01: return "PCI/ISA";
                case 0x02: return "PCI/EISA";
                case 0x03: return "PCI/MicroChannel";
                case 0x04: return "PCI/PCI";
                case 0x05: return "PCI/PCMCIA";
                case 0x06: return "PCI/NuBus";
                case 0x07: return "PCI/CardBus";
                case 0x08: return "PCI/RACEway";
                case 0x09: return "PCI/ST";
                case 0x0A: return "PCI/InfiniBand";
                default: return "Unknown";
            }
        case PCI_CLASS_COMMUNICATION:
            switch (subclass) {
                case 0x00: return "Serial";
                case 0x01: return "Parallel";
                case 0x02: return "Multiport";
                case 0x03: return "Modem";
                case 0x04: return "GPIB";
                case 0x05: return "SmartCard";
                default: return "Unknown";
            }
        case PCI_CLASS_INPUT:
            switch (subclass) {
                case 0x00: return "Keyboard";
                case 0x01: return "Digitizer";
                case 0x02: return "Mouse";
                case 0x03: return "Scanner";
                case 0x04: return "Gameport";
                default: return "Unknown";
            }
        case PCI_CLASS_SYSTEM:
            switch (subclass) {
                case 0x00: return "PIC";
                case 0x01: return "DMA";
                case 0x02: return "Timer";
                case 0x03: return "RTC";
                case 0x04: return "PCI Hot-Plug";
                case 0x05: return "SDHC";
                case 0x06: return "IOMMU";
                default: return "Unknown";
            }
        default:
            return "Unknown";
    }
}

} // namespace kernel::pci