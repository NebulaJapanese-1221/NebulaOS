// NebulaOS - ACPI Driver
// ======================
//
// ACPI table parsing and enumeration

#include "../include/acpi.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/vga.h"
#include "../../lib/include/string.h"

// -----------------------------------------------------------------------------
// ACPI state
// -----------------------------------------------------------------------------

static acpi_state_t acpi_state;
madt_info_t acpi_madt_info;

// -----------------------------------------------------------------------------
// Calculate checksum
// -----------------------------------------------------------------------------

static uint8_t acpi_checksum(const uint8_t* data, uint32_t length) {
    uint8_t sum = 0;
    for (uint32_t i = 0; i < length; i++) {
        sum += data[i];
    }
    return sum;
}

// -----------------------------------------------------------------------------
// Search for RSDP in a memory region
// -----------------------------------------------------------------------------

static rsdp_t* acpi_find_rsdp(uint8_t* start, uint32_t length) {
    for (uint32_t i = 0; i < length; i += 16) {
        rsdp_t* rsdp = (rsdp_t*)(start + i);

        if (__builtin_memcmp(rsdp->signature, ACPI_SIG_RSDP, 8) == 0) {
            if (acpi_checksum((uint8_t*)rsdp, 20) == 0) {
                return rsdp;
            }
        }
    }

    return NULL;
}

// -----------------------------------------------------------------------------
// Verify RSDT/XSDT
// -----------------------------------------------------------------------------

static bool acpi_verify_sdt(const char* data, uint32_t length) {
    return acpi_checksum((const uint8_t*)data, length) == 0;
}

// -----------------------------------------------------------------------------
// Initialize ACPI
// -----------------------------------------------------------------------------

bool acpi_init(void) {
    memset(&acpi_state, 0, sizeof(acpi_state));
    memset(&acpi_madt_info, 0, sizeof(acpi_madt_info));

    vga_puts("ACPI: Initializing...\n");

    // Search EBDA
    uint16_t ebda_segment = *((uint16_t*)0x040E);
    uint32_t ebda_address = (uint32_t)ebda_segment * 16;

    rsdp_t* rsdp = acpi_find_rsdp((uint8_t*)(size_t)ebda_address, 1024);
    if (!rsdp) {
        // Search BIOS memory 0xE0000-0xFFFFF
        rsdp = acpi_find_rsdp((uint8_t*)0xE0000, 0x20000);
    }

    if (!rsdp) {
        vga_puts("ACPI: RSDP not found\n");
        return false;
    }

    acpi_state.rsdp = rsdp;
    vga_puts("ACPI: RSDP found\n");
    vga_puts("  OEM ID: ");
    char buf[7];
    memcpy(buf, rsdp->oem_id, 6);
    buf[6] = 0;
    vga_puts(buf);
    vga_puts("\n  Revision: ");
    buf[0] = '0' + rsdp->revision;
    buf[1] = 0;
    vga_puts(buf);
    vga_puts("\n");

    // Check if we have XSDT (ACPI 2.0+)
    if (rsdp->revision >= 2 && ((rsdp_desc_t*)rsdp)->xsdt_address != 0) {
        rsdp_desc_t* rsdp_desc = (rsdp_desc_t*)rsdp;
        acpi_state.xsdt = (xsdt_t*)(size_t)rsdp_desc->xsdt_address;

        if (acpi_verify_sdt((char*)acpi_state.xsdt, acpi_state.xsdt->length)) {
            vga_puts("ACPI: XSDT found\n");
        } else {
            vga_puts("ACPI: XSDT checksum invalid\n");
            acpi_state.xsdt = NULL;
        }
    }

    // Fall back to RSDT
    if (!acpi_state.xsdt) {
        acpi_state.rsdt = (rsdt_t*)(size_t)rsdp->rsdt_address;

        if (acpi_verify_sdt((char*)acpi_state.rsdt, acpi_state.rsdt->length)) {
            vga_puts("ACPI: RSDT found\n");
        } else {
            vga_puts("ACPI: RSDT checksum invalid\n");
            return false;
        }
    }

    acpi_state.initialized = true;

    // Enumerate tables
    acpi_enumerate_tables();

    // Parse MADT
    acpi_parse_madt();

    return true;
}

// -----------------------------------------------------------------------------
// Get ACPI table by signature
// -----------------------------------------------------------------------------

void* acpi_get_table(const char* signature) {
    if (!acpi_state.initialized) {
        return NULL;
    }

    for (uint32_t i = 0; i < acpi_state.table_count; i++) {
        if (memcmp(acpi_state.tables[i].signature, signature, 4) == 0) {
            return acpi_state.tables[i].ptr;
        }
    }

    return NULL;
}

// -----------------------------------------------------------------------------
// Enumerate ACPI tables
// -----------------------------------------------------------------------------

void acpi_enumerate_tables(void) {
    if (!acpi_state.initialized) {
        return;
    }

    vga_puts("ACPI: Enumerating tables...\n");

    uint32_t entry_count = 0;

    if (acpi_state.xsdt) {
        entry_count = (acpi_state.xsdt->length - sizeof(xsdt_t)) / 8;
        for (uint32_t i = 0; i < entry_count && i < 32; i++) {
        size_t ptr = (size_t)acpi_state.xsdt->entries[i];
        char* sig = (char*)ptr;
        uint32_t len = *(uint32_t*)(ptr + 4);

            if (acpi_verify_sdt(sig, len)) {
                memcpy(acpi_state.tables[i].signature, sig, 4);
                acpi_state.tables[i].length = len;
                acpi_state.tables[i].ptr = (void*)ptr;
                acpi_state.table_count++;
            }
        }
    } else if (acpi_state.rsdt) {
        entry_count = (acpi_state.rsdt->length - sizeof(rsdt_t)) / 4;
        for (uint32_t i = 0; i < entry_count && i < 32; i++) {
            size_t ptr = (size_t)acpi_state.rsdt->entries[i];
            char* sig = (char*)ptr;
            uint32_t len = *(uint32_t*)(ptr + 4);

            if (acpi_verify_sdt(sig, len)) {
                memcpy(acpi_state.tables[i].signature, sig, 4);
                acpi_state.tables[i].length = len;
                acpi_state.tables[i].ptr = (void*)ptr;
                acpi_state.table_count++;
            }
        }
    }

    char buf[64];
    snprintf(buf, sizeof(buf), "ACPI: Found %d tables\n", acpi_state.table_count);
    vga_puts(buf);
}

// -----------------------------------------------------------------------------
// Parse MADT
// -----------------------------------------------------------------------------

void acpi_parse_madt(void) {
    madt_t* madt = (madt_t*)acpi_get_table(ACPI_SIG_MADT);

    if (!madt) {
        vga_puts("ACPI: MADT not found\n");
        return;
    }

    vga_puts("ACPI: Parsing MADT...\n");

    acpi_madt_info.local_apic_address = madt->local_apic_address;
    acpi_madt_info.local_apic_count = 0;
    acpi_madt_info.io_apic_count = 0;
    acpi_madt_info.io_apic_address = 0;

    char buf[64];
    snprintf(buf, sizeof(buf), "  Local APIC: 0x%08X\n", madt->local_apic_address);
    vga_puts(buf);

    uint32_t offset = sizeof(madt_t);
    uint32_t end = madt->length;

    while (offset < end) {
        madt_entry_header_t* entry = (madt_entry_header_t*)((size_t)madt + offset);

        if (entry->length == 0) {
            break;
        }

        switch (entry->type) {
            case MADT_TYPE_LOCAL_APIC: {
                madt_local_apic_t* lapic = (madt_local_apic_t*)entry;
                if (lapic->flags & 1) {
                    acpi_madt_info.local_apic_count++;
                    snprintf(buf, sizeof(buf),
                        "  CPU: ID=%d, APIC=%d\n",
                        lapic->processor_id, lapic->apic_id);
                    vga_puts(buf);
                }
                break;
            }
            case MADT_TYPE_IO_APIC: {
                madt_io_apic_t* io_apic = (madt_io_apic_t*)entry;
                acpi_madt_info.io_apic_count++;
                acpi_madt_info.io_apic_address = io_apic->address;
                snprintf(buf, sizeof(buf),
                    "  I/O APIC: ID=%d, Address=0x%08X, GSI=%d\n",
                    io_apic->apic_id, io_apic->address, io_apic->gsi_base);
                vga_puts(buf);
                break;
            }
            case MADT_TYPE_LOCAL_APIC_NMI: {
                madt_local_apic_nmi_t* nmi = (madt_local_apic_nmi_t*)entry;
                snprintf(buf, sizeof(buf),
                    "  Local APIC NMI: CPU=%d, LINT=%d, Flags=0x%04X\n",
                    nmi->processor_id, nmi->lint, nmi->flags);
                vga_puts(buf);
                break;
            }
            case MADT_TYPE_LOCAL_APIC_ADDR: {
                madt_local_apic_addr_t* addr = (madt_local_apic_addr_t*)entry;
                snprintf(buf, sizeof(buf),
                    "  Local APIC Address Override: 0x%016llX\n",
                    addr->address);
                vga_puts(buf);
                break;
            }
            default:
                break;
        }

        offset += entry->length;
    }

    snprintf(buf, sizeof(buf),
        "ACPI: %d local APICs, %d I/O APICs\n",
        acpi_madt_info.local_apic_count, acpi_madt_info.io_apic_count);
    vga_puts(buf);
}
