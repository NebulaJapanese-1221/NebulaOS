// NebulaOS - x86 GDT Implementation
// ===================================
//
// GDT implementation for x86 architecture

#include "../../common/include/gdt.h"
#include "../../common/include/nebula.h"

// GDT table (256 entries)
static gdt_entry_t gdt[256];

// GDT pointer
static gdt_ptr_t gdt_ptr;

// TSS entry
static tss_t tss;

// -----------------------------------------------------------------------------
// Initialize GDT
// -----------------------------------------------------------------------------
void init_gdt(void) {
    // Set up GDT pointer
    gdt_ptr.limit = sizeof(gdt) - 1;
    gdt_ptr.base = (uint32_t)&gdt;
    
    // NULL descriptor
    gdt_set_gate(0, 0, 0, 0, 0);
    
    // Kernel code segment
    gdt_set_gate(1, 0, 0xFFFFFFFF, 
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE,
                 GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    
    // Kernel data segment
    gdt_set_gate(2, 0, 0xFFFFFFFF,
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_0 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE,
                 GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    
    // User code segment
    gdt_set_gate(3, 0, 0xFFFFFFFF,
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_CODE | GDT_ACCESS_READABLE,
                 GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    
    // User data segment
    gdt_set_gate(4, 0, 0xFFFFFFFF,
                 GDT_ACCESS_PRESENT | GDT_ACCESS_PRIV_3 | GDT_ACCESS_TYPE_DATA | GDT_ACCESS_WRITABLE,
                 GDT_FLAGS_GRANULARITY | GDT_FLAGS_32BIT);
    
    // TSS segment (index 5)
    // Note: TSS setup requires special descriptor format, not using gdt_set_gate
    // For now, we'll skip TSS setup as it's not critical for basic operation
    // gdt_set_tss_gate(5, (uint32_t)&tss, sizeof(tss_t));
    
    // Flush GDT
    gdt_flush();
    
    // Flush TSS (commented out until TSS is properly set up)
    // tss_flush();
}

// -----------------------------------------------------------------------------
// Set GDT gate
// -----------------------------------------------------------------------------
void gdt_set_gate(int32_t num, uint32_t base, uint32_t limit, uint8_t access, uint8_t flags) {
    // Base address
    gdt[num].base_low = (uint16_t)(base & 0xFFFF);
    gdt[num].base_mid = (uint8_t)((base >> 16) & 0xFF);
    gdt[num].base_high = (uint8_t)((base >> 24) & 0xFF);
    
    // Limit
    gdt[num].limit_low = (uint16_t)(limit & 0xFFFF);
    gdt[num].limit_high = (uint8_t)((flags & 0xF0) | ((limit >> 16) & 0x0F));
    
    // Access and flags
    gdt[num].access = access;
}

// -----------------------------------------------------------------------------
// Flush GDT (reload GDTR)
// Defined in assembly
// -----------------------------------------------------------------------------
void gdt_flush(void) {
    __asm__ __volatile__(
        "lgdt %0\n"
        "mov $0x10, %%eax\n"
        "mov %%eax, %%ds\n"
        "mov %%eax, %%es\n"
        "mov %%eax, %%fs\n"
        "mov %%eax, %%gs\n"
        "mov %%eax, %%ss\n"
        "ljmp $0x08, $1f\n"
        "1:\n"
        : : "m" (gdt_ptr) : "eax", "memory"
    );
}

// -----------------------------------------------------------------------------
// Flush TSS (load TR register)
// -----------------------------------------------------------------------------
void tss_flush(void) {
    __asm__ __volatile__("ltr %%ax" : : "a" (GDT_TSS_SEL));
}
