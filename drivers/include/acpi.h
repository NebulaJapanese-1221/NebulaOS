// NebulaOS - ACPI Driver
// ======================
//
// ACPI table parsing and enumeration

#ifndef NEBULAOS_DRIVERS_ACPI_H
#define NEBULAOS_DRIVERS_ACPI_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// ACPI signature macros
#define ACPI_SIG_RSDP  "RSD PTR "
#define ACPI_SIG_RSDT  "RSDT"
#define ACPI_SIG_XSDT  "XSDT"
#define ACPI_SIG_MADT  "APIC"
#define ACPI_SIG_FADT  "FACP"
#define ACPI_SIG_DSDT  "DSDT"
#define ACPI_SIG_SSDT  "SSDT"
#define ACPI_SIG_HPET  "HPET"
#define ACPI_SIG_MCFG  "MCFG"

// RSDP structure (for ACPI 1.0)
typedef struct PACKED {
    char     signature[8];
    uint8_t  checksum;
    char     oem_id[6];
    uint8_t  revision;
    uint32_t rsdt_address;
} rsdp_t;

// RSDP structure (for ACPI 2.0+)
typedef struct PACKED {
    char     signature[8];
    uint8_t  checksum;
    char     oem_id[6];
    uint8_t  revision;
    uint32_t rsdt_address;
    uint32_t length;
    uint64_t xsdt_address;
    uint8_t  extended_checksum;
    uint8_t  reserved[3];
} rsdp_desc_t;

// RSDT (Root System Description Table)
typedef struct PACKED {
    char     signature[4];
    uint32_t length;
    uint8_t  revision;
    uint8_t  checksum;
    char     oem_id[6];
    char     oem_table_id[8];
    uint32_t oem_revision;
    uint32_t creator_id;
    uint32_t creator_revision;
    uint32_t entries[];
} rsdt_t;

// XSDT (Extended System Description Table)
typedef struct PACKED {
    char     signature[4];
    uint32_t length;
    uint8_t  revision;
    uint8_t  checksum;
    char     oem_id[6];
    char     oem_table_id[8];
    uint32_t oem_revision;
    uint32_t creator_id;
    uint32_t creator_revision;
    uint64_t entries[];
} xsdt_t;

// MADT (Multiple APIC Description Table) - entry types
#define MADT_TYPE_LOCAL_APIC   0x00
#define MADT_TYPE_IO_APIC      0x01
#define MADT_TYPE_INT_SRC      0x02
#define MADT_TYPE_NMI_SRC      0x03
#define MADT_TYPE_LOCAL_APIC_NMI 0x04
#define MADT_TYPE_LOCAL_APIC_ADDR 0x05
#define MADT_TYPE_IO_SAPIC     0x06
#define MADT_TYPE_LOCAL_SAPIC   0x07
#define MADT_TYPE_INT_SRC_OVERRIDE 0x08
#define MADT_TYPE_IO_APIC_EXT   0x09

// MADT header
typedef struct PACKED {
    char     signature[4];
    uint32_t length;
    uint8_t  revision;
    uint8_t  checksum;
    char     oem_id[6];
    char     oem_table_id[8];
    uint32_t oem_revision;
    uint32_t creator_id;
    uint32_t creator_revision;
    uint32_t local_apic_address;
    uint32_t flags;
} madt_t;

// MADT entry header
typedef struct PACKED {
    uint8_t type;
    uint8_t length;
} madt_entry_header_t;

// Local APIC entry
typedef struct PACKED {
    madt_entry_header_t header;
    uint8_t  processor_id;
    uint8_t  apic_id;
    uint32_t flags;
} madt_local_apic_t;

// I/O APIC entry
typedef struct PACKED {
    madt_entry_header_t header;
    uint8_t  apic_id;
    uint8_t  reserved;
    uint32_t address;
    uint32_t gsi_base;
} madt_io_apic_t;

// Local APIC NMI entry
typedef struct PACKED {
    madt_entry_header_t header;
    uint8_t  processor_id;
    uint16_t flags;
    uint8_t  lint;
} madt_local_apic_nmi_t;

// Local APIC address override entry
typedef struct PACKED {
    madt_entry_header_t header;
    uint16_t reserved;
    uint64_t address;
} madt_local_apic_addr_t;

// FADT (Fixed ACPI Description Table) - stub
typedef struct PACKED {
    char     signature[4];
    uint32_t length;
    uint8_t  revision;
    uint8_t  checksum;
    char     oem_id[6];
    char     oem_table_id[8];
    uint32_t oem_revision;
    uint32_t creator_id;
    uint32_t creator_revision;
    uint32_t sdt_address;
    uint8_t  pm_timer_block;
    uint8_t  pm_event_block;
    uint8_t  pm_control_block;
    uint8_t  pm_timer_width;
    uint8_t  reserved[3];
    uint8_t  pm1a_control_block[4];
    uint8_t  pm1b_control_block[4];
    uint8_t  pm2_control_block[4];
    uint8_t  pm_timer_block_ptr[4];
    uint8_t  gpe0_block[4];
    uint8_t  gpe1_block[4];
    uint8_t  pm1_event_length;
    uint8_t  pm1_control_length;
    uint8_t  pm2_control_length;
    uint8_t  pm_timer_length;
    uint8_t  gpe0_length;
    uint8_t  gpe1_length;
    uint8_t  gpe1_base;
    uint8_t  cst_control;
    uint16_t c2_latency;
    uint16_t c3_latency;
    uint16_t cache_size;
    uint16_t cache_flush_stall;
    uint8_t  reserved2[4];
    uint8_t  power_button;
    uint8_t  sleep_button;
    uint8_t  reserved3[2];
    uint8_t  debug_port;
    uint8_t  reserved4;
    uint16_t hv_vendor_id;
    uint8_t  hv_major;
    uint8_t  hv_minor;
} fadt_t;

// ACPI table structure
typedef struct {
    char signature[4];
    uint32_t length;
    void*    ptr;
} acpi_table_t;

// ACPI state
typedef struct {
    bool initialized;
    rsdp_t* rsdp;
    rsdt_t* rsdt;
    xsdt_t* xsdt;
    madt_t* madt;
    fadt_t* fadt;
    uint32_t table_count;
    acpi_table_t tables[32];
} acpi_state_t;

// ACPI functions
bool acpi_init(void);
void* acpi_get_table(const char* signature);
void acpi_enumerate_tables(void);
void acpi_parse_madt(void);

// MADT info
typedef struct {
    uint32_t local_apic_address;
    uint32_t io_apic_address;
    uint32_t local_apic_count;
    uint32_t io_apic_count;
} madt_info_t;

extern madt_info_t acpi_madt_info;

#endif // NEBULAOS_DRIVERS_ACPI_H
