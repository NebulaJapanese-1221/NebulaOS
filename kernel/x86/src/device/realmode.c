// NebulaOS - Real Mode BIOS Interface (x86)
// ==========================================
//
// Builds the temporary real-mode GDT in low memory and exposes the
// real-mode call primitive implemented in rm_trampoline.asm.

#include "../../common/include/stdint.h"
#include "../../common/include/nebula.h"
#include "realmode.h"

// Reserved low-memory layout (must match rm_trampoline.asm).
#define RM_BASE            0x9000
#define RM_GDT_BASE        (RM_BASE + 8)
#define RM_PARAMS_PHYS     0x9040
#define RM_VBE_INFO_PHYS   0x9080
#define RM_VBE_MODE_PHYS   0x9280

// Temporary GDT selectors.
#define SEL_CODE32 0x08
#define SEL_DATA32 0x10
#define SEL_CODE16 0x18
#define SEL_DATA16 0x20

static bool rm_ready = false;

// -----------------------------------------------------------------------------
// Build one GDT entry (8 bytes, little-endian) at `base`.
// -----------------------------------------------------------------------------
static void rm_set_gdt_entry(uint8_t* base, uint32_t limit,
                             uint8_t access, uint8_t flags) {
    base[0] = limit & 0xFF;
    base[1] = (limit >> 8) & 0xFF;
    base[2] = 0;           // base low
    base[3] = 0;
    base[4] = 0;           // base mid
    base[5] = access;
    base[6] = ((limit >> 16) & 0x0F) | (flags & 0xF0);
    base[7] = 0;           // base high
}

// -----------------------------------------------------------------------------
// Install the temporary real-mode GDT into low memory:
//   [RM_BASE]      = 6-byte GDTR (limit, base)
//   [RM_GDT_BASE]  = 5 descriptors
// -----------------------------------------------------------------------------
static void rm_install_gdt(void) {
    uint8_t* g = (uint8_t*)RM_BASE;

    // GDTR: limit = 5*8 - 1 = 39, base = RM_GDT_BASE
    g[0] = 39;
    g[1] = 0;
    *((uint32_t*)(g + 2)) = RM_GDT_BASE;

    uint8_t* e = (uint8_t*)RM_GDT_BASE;
    rm_set_gdt_entry(e + 0,  0,       0x00, 0x00);  // null
    rm_set_gdt_entry(e + 8,  0xFFFFF, 0x9A, 0xC0);  // 32-bit code
    rm_set_gdt_entry(e + 16, 0xFFFFF, 0x92, 0xC0);  // 32-bit data
    rm_set_gdt_entry(e + 24, 0xFFFF,  0x9A, 0x00);  // 16-bit code
    rm_set_gdt_entry(e + 32, 0xFFFF,  0x92, 0x00);  // 16-bit data
}

// -----------------------------------------------------------------------------
// External assembly primitives.
// -----------------------------------------------------------------------------
extern void realmode_install(void);
extern uint32_t realmode_available_flag;

// -----------------------------------------------------------------------------
// Initialize the real-mode BIOS interface.
// -----------------------------------------------------------------------------
bool realmode_init(void) {
    rm_install_gdt();
    realmode_install();
    rm_ready = (realmode_available_flag != 0);
    return rm_ready;
}

bool realmode_available(void) {
    return rm_ready;
}

// -----------------------------------------------------------------------------
// Low-memory scratch buffers for VBE ES:DI targets.
// -----------------------------------------------------------------------------
void* realmode_vbe_info_buffer(void) {
    return (void*)RM_VBE_INFO_PHYS;
}

void* realmode_vbe_mode_buffer(void) {
    return (void*)RM_VBE_MODE_PHYS;
}
