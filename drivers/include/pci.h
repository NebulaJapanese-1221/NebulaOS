// NebulaOS - PCI Bus Driver
// =========================
//
// PCI configuration space access and device enumeration

#ifndef NEBULAOS_DRIVERS_PCI_H
#define NEBULAOS_DRIVERS_PCI_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// PCI configuration space ports
#define PCI_CONFIG_ADDRESS_PORT 0xCF8
#define PCI_CONFIG_DATA_PORT    0xCFC

// PCI configuration address format
#define PCI_CONFIG_ENABLE       (1UL << 31)
#define PCI_CONFIG_BUS_SHIFT     16
#define PCI_CONFIG_DEVICE_SHIFT  11
#define PCI_CONFIG_FUNCTION_SHIFT 8
#define PCI_CONFIG_REGISTER_SHIFT 2

// PCI device limits
#define PCI_MAX_BUSES        256
#define PCI_MAX_DEVICES      32
#define PCI_MAX_FUNCTIONS    8
#define PCI_MAX_REGISTERS    64

// PCI class codes
#define PCI_CLASS_CODE_OFFSET 0x0B
#define PCI_SUBCLASS_CODE_OFFSET 0x0A
#define PCI_PROG_IF_OFFSET    0x09

// PCI class codes
typedef enum {
    PCI_CLASS_UNCLASSIFIED = 0x00,
    PCI_CLASS_MASS_STORAGE = 0x01,
    PCI_CLASS_NETWORK      = 0x02,
    PCI_CLASS_DISPLAY      = 0x03,
    PCI_CLASS_MULTIMEDIA   = 0x04,
    PCI_CLASS_MEMORY       = 0x05,
    PCI_CLASS_BRIDGE       = 0x06,
    PCI_CLASS_COMMUNICATION = 0x07,
    PCI_CLASS_PERIPHERAL   = 0x08,
    PCI_CLASS_INPUT        = 0x09,
    PCI_CLASS_DOCKING      = 0x0A,
    PCI_CLASS_PROCESSOR    = 0x0B,
    PCI_CLASS_SERIAL_BUS   = 0x0C,
    PCI_CLASS_WIRELESS     = 0x0D,
    PCI_CLASS_INTELLIGENT  = 0x0E,
    PCI_CLASS_SATELLITE    = 0x0F,
    PCI_CLASS_CRYPTO       = 0x10,
    PCI_CLASS_SIGNAL       = 0x11,
    PCI_CLASS_ACCELERATOR  = 0x12,
    PCI_CLASS_INSTRUMENTATION = 0x13,
    PCI_CLASS_COPROCESSOR  = 0x40,
    PCI_CLASS_UNASSIGNED   = 0xFF
} pci_class_code_t;

// PCI subclass codes for mass storage
#define PCI_SUBCLASS_MASS_STORAGE_SCSI      0x00
#define PCI_SUBCLASS_MASS_STORAGE_IDE       0x01
#define PCI_SUBCLASS_MASS_STORAGE_FLOPPY    0x02
#define PCI_SUBCLASS_MASS_STORAGE_IPI       0x03
#define PCI_SUBCLASS_MASS_STORAGE_RAID      0x04
#define PCI_SUBCLASS_MASS_STORAGE_ATA       0x05
#define PCI_SUBCLASS_MASS_STORAGE_SATA      0x06
#define PCI_SUBCLASS_MASS_STORAGE_SAS       0x07
#define PCI_SUBCLASS_MASS_STORAGE_NVME      0x08

// PCI subclass codes for network
#define PCI_SUBCLASS_NETWORK_ETHERNET  0x00
#define PCI_SUBCLASS_NETWORK_TOKEN_RING 0x01
#define PCI_SUBCLASS_NETWORK_FDDI     0x02
#define PCI_SUBCLASS_NETWORK_ATM      0x03
#define PCI_SUBCLASS_NETWORK_ISDN     0x04
#define PCI_SUBCLASS_NETWORK_WORLDFIP 0x05
#define PCI_SUBCLASS_NETWORK_PICMG    0x06
#define PCI_SUBCLASS_NETWORK_INFINIBAND 0x07

// PCI subclass codes for display
#define PCI_SUBCLASS_DISPLAY_VGA    0x00
#define PCI_SUBCLASS_DISPLAY_XGA    0x01
#define PCI_SUBCLASS_DISPLAY_3D     0x02
#define PCI_SUBCLASS_DISPLAY_DISPLAY 0x80

// PCI header type
#define PCI_HEADER_TYPE_STANDARD 0x00
#define PCI_HEADER_TYPE_PCI_TO_PCI 0x01
#define PCI_HEADER_TYPE_CARDBUS  0x02

// PCI configuration space header (standard)
typedef struct PACKED {
    uint16_t vendor_id;
    uint16_t device_id;
    uint16_t command;
    uint16_t status;
    uint8_t  revision_id;
    uint8_t  prog_if;
    uint8_t  subclass;
    uint8_t  class_code;
    uint8_t  cache_line_size;
    uint8_t  latency_timer;
    uint8_t  header_type;
    uint8_t  bist;
    uint32_t bar[6];
    uint32_t cardbus_cis_ptr;
    uint16_t subsystem_vendor_id;
    uint16_t subsystem_id;
    uint32_t expansion_rom_base;
    uint8_t  capabilities_ptr;
    uint8_t  reserved[3];
    uint32_t reserved1;
    uint8_t  interrupt_line;
    uint8_t  interrupt_pin;
    uint8_t  min_grant;
    uint8_t  max_latency;
} pci_header_t;

// PCI device structure
typedef struct {
    uint8_t  bus;
    uint8_t  device;
    uint8_t  function;
    uint16_t vendor_id;
    uint16_t device_id;
    uint8_t  class_code;
    uint8_t  subclass;
    uint8_t  prog_if;
    uint8_t  header_type;
    uint32_t bar[6];
    uint8_t  interrupt_line;
    uint8_t  interrupt_pin;
} pci_device_t;

// PCI class name lookup
typedef struct {
    uint8_t  class_code;
    uint8_t  subclass;
    const char* class_name;
    const char* subclass_name;
} pci_class_name_t;

// PCI driver structure
typedef struct pci_driver {
    uint16_t vendor_id;
    uint16_t device_id;
    const char* name;
    int (*init)(pci_device_t* device);
    struct pci_driver* next;
} pci_driver_t;

// PCI configuration space access functions
static inline uint32_t pci_make_address(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset) {
    return PCI_CONFIG_ENABLE |
           ((uint32_t)bus << PCI_CONFIG_BUS_SHIFT) |
           ((uint32_t)device << PCI_CONFIG_DEVICE_SHIFT) |
           ((uint32_t)function << PCI_CONFIG_FUNCTION_SHIFT) |
           ((uint32_t)offset & 0xFC);
}

uint8_t pci_read_config_byte(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset);
uint16_t pci_read_config_word(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset);
uint32_t pci_read_config_dword(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset);
void pci_write_config_byte(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset, uint8_t value);
void pci_write_config_word(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset, uint16_t value);
void pci_write_config_dword(uint8_t bus, uint8_t device, uint8_t function, uint8_t offset, uint32_t value);

// Device enumeration
bool pci_enumerate_bus(void);
pci_device_t* pci_find_device(uint16_t vendor_id, uint16_t device_id);
pci_device_t* pci_find_class(uint8_t class_code, uint8_t subclass);

// Class code lookup
const char* pci_get_class_name(uint8_t class_code, uint8_t subclass);
const char* pci_get_subclass_name(uint8_t class_code, uint8_t subclass);

// Driver registration
void pci_register_driver(pci_driver_t* driver);
void pci_init(void);

#endif // NEBULAOS_DRIVERS_PCI_H
