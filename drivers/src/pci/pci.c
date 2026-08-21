// NebulaOS - PCI Bus Driver
// =========================
//
// PCI configuration space access and device enumeration via I/O ports

#include "../include/pci.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/vga.h"
#include "../../lib/include/string.h"

// -----------------------------------------------------------------------------
// I/O functions
// -----------------------------------------------------------------------------

static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" : : "a"(value), "dN"(port));
}

static inline uint16_t inw(uint16_t port) {
    uint16_t value;
    __asm__ __volatile__("inw %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outw(uint16_t port, uint16_t value) {
    __asm__ __volatile__("outw %0, %1" : : "a"(value), "dN"(port));
}

static inline uint32_t inl(uint16_t port) {
    uint32_t value;
    __asm__ __volatile__("inl %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outl(uint16_t port, uint32_t value) {
    __asm__ __volatile__("outl %0, %1" : : "a"(value), "dN"(port));
}

// -----------------------------------------------------------------------------
// PCI configuration space access
// -----------------------------------------------------------------------------

uint8_t pci_read_config_byte(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset) {
    uint32_t address = pci_make_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS_PORT, address);
    return inb(PCI_CONFIG_DATA_PORT + (offset & 3));
}

uint16_t pci_read_config_word(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset) {
    uint32_t address = pci_make_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS_PORT, address);
    return inw(PCI_CONFIG_DATA_PORT + (offset & 2));
}

uint32_t pci_read_config_dword(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset) {
    uint32_t address = pci_make_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS_PORT, address);
    return inl(PCI_CONFIG_DATA_PORT);
}

void pci_write_config_byte(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset, uint8_t value) {
    uint32_t address = pci_make_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS_PORT, address);
    outb(PCI_CONFIG_DATA_PORT + (offset & 3), value);
}

void pci_write_config_word(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset, uint16_t value) {
    uint32_t address = pci_make_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS_PORT, address);
    outw(PCI_CONFIG_DATA_PORT + (offset & 2), value);
}

void pci_write_config_dword(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset, uint32_t value) {
    uint32_t address = pci_make_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS_PORT, address);
    outl(PCI_CONFIG_DATA_PORT, value);
}

// -----------------------------------------------------------------------------
// PCI class code lookup table
// -----------------------------------------------------------------------------

static const pci_class_name_t pci_class_names[] = {
    {PCI_CLASS_UNCLASSIFIED, 0x00, "Unclassified", "Unclassified"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_SCSI, "Mass Storage", "SCSI"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_IDE, "Mass Storage", "IDE"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_FLOPPY, "Mass Storage", "Floppy"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_IPI, "Mass Storage", "IPI"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_RAID, "Mass Storage", "RAID"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_ATA, "Mass Storage", "ATA"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_SATA, "Mass Storage", "SATA"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_SAS, "Mass Storage", "SAS"},
    {PCI_CLASS_MASS_STORAGE, PCI_SUBCLASS_MASS_STORAGE_NVME, "Mass Storage", "NVMe"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_ETHERNET, "Network", "Ethernet"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_TOKEN_RING, "Network", "Token Ring"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_FDDI, "Network", "FDDI"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_ATM, "Network", "ATM"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_ISDN, "Network", "ISDN"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_WORLDFIP, "Network", "WorldFIP"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_PICMG, "Network", "PICMG"},
    {PCI_CLASS_NETWORK, PCI_SUBCLASS_NETWORK_INFINIBAND, "Network", "InfiniBand"},
    {PCI_CLASS_DISPLAY, PCI_SUBCLASS_DISPLAY_VGA, "Display", "VGA"},
    {PCI_CLASS_DISPLAY, PCI_SUBCLASS_DISPLAY_XGA, "Display", "XGA"},
    {PCI_CLASS_DISPLAY, PCI_SUBCLASS_DISPLAY_3D, "Display", "3D"},
    {PCI_CLASS_DISPLAY, PCI_SUBCLASS_DISPLAY_DISPLAY, "Display", "Display"},
    {PCI_CLASS_MULTIMEDIA, 0x00, "Multimedia", "Video"},
    {PCI_CLASS_MULTIMEDIA, 0x01, "Multimedia", "Audio"},
    {PCI_CLASS_MULTIMEDIA, 0x80, "Multimedia", "Multimedia"},
    {PCI_CLASS_MEMORY, 0x00, "Memory", "RAM"},
    {PCI_CLASS_MEMORY, 0x01, "Memory", "Flash"},
    {PCI_CLASS_MEMORY, 0x80, "Memory", "Memory"},
    {PCI_CLASS_BRIDGE, 0x00, "Bridge", "Host"},
    {PCI_CLASS_BRIDGE, 0x01, "Bridge", "ISA"},
    {PCI_CLASS_BRIDGE, 0x02, "Bridge", "EISA"},
    {PCI_CLASS_BRIDGE, 0x03, "Bridge", "MCA"},
    {PCI_CLASS_BRIDGE, 0x04, "Bridge", "PCI-to-PCI"},
    {PCI_CLASS_BRIDGE, 0x05, "Bridge", "PCMCIA"},
    {PCI_CLASS_BRIDGE, 0x06, "Bridge", "NuBus"},
    {PCI_CLASS_BRIDGE, 0x07, "Bridge", "CardBus"},
    {PCI_CLASS_BRIDGE, 0x08, "Bridge", "RACEway"},
    {PCI_CLASS_BRIDGE, 0x09, "Bridge", "PCI-to-PCI"},
    {PCI_CLASS_BRIDGE, 0x0A, "Bridge", "InfiniBand"},
    {PCI_CLASS_SERIAL_BUS, 0x00, "Serial Bus", "FireWire"},
    {PCI_CLASS_SERIAL_BUS, 0x01, "Serial Bus", "ACCESS"},
    {PCI_CLASS_SERIAL_BUS, 0x02, "Serial Bus", "SSA"},
    {PCI_CLASS_SERIAL_BUS, 0x03, "Serial Bus", "USB"},
    {PCI_CLASS_SERIAL_BUS, 0x04, "Serial Bus", "Fibre Channel"},
    {PCI_CLASS_SERIAL_BUS, 0x05, "Serial Bus", "SMBus"},
    {PCI_CLASS_SERIAL_BUS, 0x06, "Serial Bus", "InfiniBand"},
    {PCI_CLASS_SERIAL_BUS, 0x07, "Serial Bus", "IPMI"},
    {PCI_CLASS_SERIAL_BUS, 0x08, "Serial Bus", "SERCOS"},
    {PCI_CLASS_SERIAL_BUS, 0x09, "Serial Bus", "CANbus"},
    {PCI_CLASS_SERIAL_BUS, 0x0A, "Serial Bus", "MIPI"},
    {PCI_CLASS_WIRELESS, 0x00, "Wireless", "IRDA"},
    {PCI_CLASS_WIRELESS, 0x01, "Wireless", "Consumer IR"},
    {PCI_CLASS_WIRELESS, 0x02, "Wireless", "RF"},
    {PCI_CLASS_WIRELESS, 0x03, "Wireless", "Bluetooth"},
    {PCI_CLASS_WIRELESS, 0x04, "Wireless", "Broadband"},
    {PCI_CLASS_WIRELESS, 0x05, "Wireless", "Ethernet (802.1a)"},
    {PCI_CLASS_WIRELESS, 0x06, "Wireless", "Ethernet (802.1b)"},
    {PCI_CLASS_WIRELESS, 0x07, "Wireless", "Cellular"},
    {PCI_CLASS_WIRELESS, 0x08, "Wireless", "GPS"},
    {PCI_CLASS_SATELLITE, 0x00, "Satellite", "TV"},
    {PCI_CLASS_SATELLITE, 0x01, "Satellite", "Audio"},
    {PCI_CLASS_SATELLITE, 0x02, "Satellite", "Voice"},
    {PCI_CLASS_SATELLITE, 0x03, "Satellite", "Data"},
    {PCI_CLASS_CRYPTO, 0x00, "Crypto", "Network/Comp"},
    {PCI_CLASS_CRYPTO, 0x01, "Crypto", "Entertainment"},
    {PCI_CLASS_SIGNAL, 0x00, "Signal Processing", "DPIO"},
    {PCI_CLASS_SIGNAL, 0x01, "Signal Processing", "Perf Counters"},
    {PCI_CLASS_SIGNAL, 0x02, "Signal Processing", "Sync/Time"},
    {PCI_CLASS_SIGNAL, 0x03, "Signal Processing", "Management"},
    {0xFF, 0xFF, "Unassigned", "Unassigned"}
};

// -----------------------------------------------------------------------------
// PCI driver list
// -----------------------------------------------------------------------------

static pci_driver_t* pci_drivers = NULL;

// -----------------------------------------------------------------------------
// Class name lookup
// -----------------------------------------------------------------------------

const char* pci_get_class_name(uint8_t class_code, uint8_t subclass) {
    for (size_t i = 0; pci_class_names[i].class_code != 0xFF; i++) {
        if (pci_class_names[i].class_code == class_code &&
            pci_class_names[i].subclass == subclass) {
            return pci_class_names[i].class_name;
        }
    }
    return "Unknown";
}

const char* pci_get_subclass_name(uint8_t class_code, uint8_t subclass) {
    for (size_t i = 0; pci_class_names[i].class_code != 0xFF; i++) {
        if (pci_class_names[i].class_code == class_code &&
            pci_class_names[i].subclass == subclass) {
            return pci_class_names[i].subclass_name;
        }
    }
    return "Unknown";
}

// -----------------------------------------------------------------------------
// Register PCI driver
// -----------------------------------------------------------------------------

void pci_register_driver(pci_driver_t* driver) {
    driver->next = pci_drivers;
    pci_drivers = driver;
}

// -----------------------------------------------------------------------------
// Find device by vendor/device ID
// -----------------------------------------------------------------------------

static pci_device_t pci_devices[PCI_MAX_BUSES * PCI_MAX_DEVICES];
static uint32_t pci_device_count = 0;

pci_device_t* pci_find_device(uint16_t vendor_id, uint16_t device_id) {
    for (uint32_t i = 0; i < pci_device_count; i++) {
        if (pci_devices[i].vendor_id == vendor_id &&
            pci_devices[i].device_id == device_id) {
            return &pci_devices[i];
        }
    }
    return NULL;
}

// -----------------------------------------------------------------------------
// Find device by class
// -----------------------------------------------------------------------------

pci_device_t* pci_find_class(uint8_t class_code, uint8_t subclass) {
    for (uint32_t i = 0; i < pci_device_count; i++) {
        if (pci_devices[i].class_code == class_code &&
            pci_devices[i].subclass == subclass) {
            return &pci_devices[i];
        }
    }
    return NULL;
}

// -----------------------------------------------------------------------------
// Check if device function exists
// -----------------------------------------------------------------------------

static bool pci_device_exists(uint8_t bus, uint8_t device, uint8_t function) {
    uint16_t vendor_id = pci_read_config_word(bus, device, function, 0x00);
    return vendor_id != 0xFFFF;
}

// -----------------------------------------------------------------------------
// Enumerate PCI bus
// -----------------------------------------------------------------------------

bool pci_enumerate_bus(void) {
    pci_device_count = 0;
    vga_puts("PCI: Enumerating bus...\n");

    for (uint16_t bus = 0; bus < PCI_MAX_BUSES; bus++) {
        for (uint8_t device = 0; device < PCI_MAX_DEVICES; device++) {
            if (!pci_device_exists(bus, device, 0)) {
                continue;
            }

            for (uint8_t function = 0; function < PCI_MAX_FUNCTIONS; function++) {
                if (!pci_device_exists(bus, device, function)) {
                    continue;
                }

                if (pci_device_count >= PCI_MAX_BUSES * PCI_MAX_DEVICES) {
                    break;
                }

                pci_device_t* pdev = &pci_devices[pci_device_count];
                pdev->bus = bus;
                pdev->device = device;
                pdev->function = function;
                pdev->vendor_id = pci_read_config_word(bus, device, function, 0x00);
                pdev->device_id = pci_read_config_word(bus, device, function, 0x02);
                pdev->class_code = pci_read_config_byte(bus, device, function, PCI_CLASS_CODE_OFFSET);
                pdev->subclass = pci_read_config_byte(bus, device, function, PCI_SUBCLASS_CODE_OFFSET);
                pdev->prog_if = pci_read_config_byte(bus, device, function, PCI_PROG_IF_OFFSET);
                pdev->header_type = pci_read_config_byte(bus, device, function, 0x0E);
                pdev->interrupt_line = pci_read_config_byte(bus, device, function, 0x3C);
                pdev->interrupt_pin = pci_read_config_byte(bus, device, function, 0x3D);

                for (int i = 0; i < 6; i++) {
                    pdev->bar[i] = pci_read_config_dword(bus, device, function, 0x10 + i * 4);
                }

                pci_device_count++;

                char buf[128];
                snprintf(buf, sizeof(buf),
                    "PCI: %02X:%02X.%d - Vendor: %04X, Device: %04X, Class: %s (%s)\n",
                    bus, device, function,
                    pdev->vendor_id, pdev->device_id,
                    pci_get_class_name(pdev->class_code, pdev->subclass),
                    pci_get_subclass_name(pdev->class_code, pdev->subclass));
                vga_puts(buf);

                // Call registered drivers
                pci_driver_t* drv = pci_drivers;
                while (drv) {
                    if (drv->vendor_id == pdev->vendor_id &&
                        drv->device_id == pdev->device_id) {
                        if (drv->init) {
                            drv->init(pdev);
                        }
                    }
                    drv = drv->next;
                }
            }
        }
    }

    vga_puts("PCI: Found ");
    char buf[16];
    int len = 0;
    uint32_t count = pci_device_count;
    if (count == 0) {
        buf[0] = '0';
        buf[1] = 0;
    } else {
        while (count > 0 && len < 15) {
            buf[len++] = '0' + (count % 10);
            count /= 10;
        }
        buf[len] = 0;
        for (int i = 0; i < len / 2; i++) {
            char tmp = buf[i];
            buf[i] = buf[len - 1 - i];
            buf[len - 1 - i] = tmp;
        }
    }
    vga_puts(buf);
    vga_puts(" devices\n");

    return pci_device_count > 0;
}

// -----------------------------------------------------------------------------
// Initialize PCI subsystem
// -----------------------------------------------------------------------------

void pci_init(void) {
    vga_puts("Initializing PCI subsystem...\n");
    pci_enumerate_bus();
}
