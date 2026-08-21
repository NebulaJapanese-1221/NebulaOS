// NebulaOS - x86_64 GDT Implementation
// ======================================
//
// GDT implementation for x86_64 architecture

#include "../../../common/include/gdt.h"
#include "../../../common/include/nebula.h"

// GDT table (6 entries: null, kernel code, kernel data, user code, user data, TSS)
static gdt_entry_t gdt[6];

// GDT pointer
static gdt_ptr_t gdt_ptr;

// TSS entry
static tss_t tss;

// -----------------------------------------------------------------------------
// Initialize GDT for 64-bit long mode
// -----------------------------------------------------------------------------
void gdt64_init(void) {
    gdt_ptr.limit = sizeof(gdt) - 1;
    gdt_ptr.base = (uint64_t)&gdt;
    gdt_ptr.base_upper = (uint32_t)((uint64_t)&gdt >> 32);

    // NULL descriptor
    gdt_set_gate(0, 0, 0, 0, 0);

    // Kernel code segment (0x08) - 64-bit long mode
    gdt_set_gate(1, 0, 0,
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE,
                 GDT_FLAGS_GRANULARITY | GDT_FLAGS_64BIT);

    // Kernel data segment (0x10) - flat data
    gdt_set_gate(2, 0, 0,
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE,
                 GDT_FLAGS_GRANULARITY);

    // User code segment (0x18) - ring 3 code
    gdt_set_gate(3, 0, 0,
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE,
                 GDT_FLAGS_GRANULARITY | GDT_FLAGS_64BIT);

    // User data segment (0x20) - ring 3 data
    gdt_set_gate(4, 0, 0,
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE,
                 GDT_FLAGS_GRANULARITY);

    // TSS segment (0x28) - Task State Segment
    memset(&tss, 0, sizeof(tss_t));
    tss.ss0 = 0x10;
    tss.esp0 = 0;
    tss.iomap_base = sizeof(tss_t);

    // Set up TSS descriptor using a 64-bit compatible approach
    // For x86_64, the TSS descriptor is 16 bytes
    uint64_t tss_base = (uint64_t)&tss;
    uint64_t tss_limit = sizeof(tss_t) - 1;

    gdt[5].base_low = (uint16_t)(tss_base & 0xFFFF);
    gdt[5].base_mid = (uint8_t)((tss_base >> 16) & 0xFF);
    gdt[5].base_high = (uint8_t)((tss_base >> 24) & 0xFF);
    gdt[5].limit_low = (uint16_t)(tss_limit & 0xFFFF);
    gdt[5].access = GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | 0x09;  // Available 64-bit TSS
    gdt[5].limit_high = (uint8_t)((tss_limit >> 16) & 0x0F);

    // Flush GDT
    gdt_flush64();
}

// -----------------------------------------------------------------------------
// Set GDT gate (shared with x86, handles flat 64-bit segments)
// -----------------------------------------------------------------------------
void gdt_set_gate(int32_t num, uint32_t base, uint32_t limit, uint8_t access, uint8_t flags) {
    gdt[num].base_low = (uint16_t)(base & 0xFFFF);
    gdt[num].base_mid = (uint8_t)((base >> 16) & 0xFF);
    gdt[num].base_high = (uint8_t)((base >> 24) & 0xFF);
    gdt[num].limit_low = (uint16_t)(limit & 0xFFFF);
    gdt[num].limit_high = (uint8_t)((flags & 0xF0) | ((limit >> 16) & 0x0F));
    gdt[num].access = access;
}

// -----------------------------------------------------------------------------
// Flush GDT for 64-bit
// Reload GDTR and all segment registers
// -----------------------------------------------------------------------------
void gdt_flush64(void) {
    __asm__ __volatile__(
        "lgdt %0\n"
        "mov $0x10, %%rax\n"
        "mov %%rax, %%ds\n"
        "mov %%rax, %%es\n"
        "mov %%rax, %%fs\n"
        "mov %%rax, %%gs\n"
        "mov %%rax, %%ss\n"
        "pushq $0x08\n"
        "lea .lgdt_return(%%rip), %%rax\n"
        "pushq %%rax\n"
        "lretq\n"
        ".lgdt_return:\n"
        : : "m" (gdt_ptr) : "rax", "memory"
    );
}

// -----------------------------------------------------------------------------
// Flush TSS (load TR register)
// -----------------------------------------------------------------------------
void tss_flush(void) {
    __asm__ __volatile__("ltr %%ax" : : "a" (GDT_TSS_SEL));
}
