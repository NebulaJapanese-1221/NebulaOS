// NebulaOS - x86_64 Hardware Initialization Stubs
// ================================================
//
// Proper implementations for x86_64-specific hardware initialization

#include "../../../common/include/nebula.h"
#include "../../../common/include/stdint.h"
#include "../../../common/include/vga.h"
#include "../../../common/include/memory.h"
#include "../../../drivers/include/keyboard.h"
#include "../../../drivers/include/pit.h"
#include "../../../drivers/include/pic.h"

// Forward declarations of architecture-specific implementations
void gdt64_init(void);
void idt64_init(void);

// Driver initializers
void init_keyboard(void);

// -----------------------------------------------------------------------------
// Initialize GDT for 64-bit
// -----------------------------------------------------------------------------
void init_gdt64(void) {
    gdt64_init();
}

// -----------------------------------------------------------------------------
// Initialize IDT for 64-bit
// -----------------------------------------------------------------------------
void init_idt64(void) {
    idt64_init();
}

// -----------------------------------------------------------------------------
// Initialize PIT timer for x86_64
// -----------------------------------------------------------------------------
void init_timer64(void) {
    // Remap PIC first
    pic_init(0x20, 0x28);

    // Initialize PIT at 1000 Hz
    pit_init(1000);
}

// -----------------------------------------------------------------------------
// Initialize keyboard for x86_64
// -----------------------------------------------------------------------------
void init_keyboard64(void) {
    init_keyboard();
}

// -----------------------------------------------------------------------------
// Initialize VGA text mode for x86_64
// -----------------------------------------------------------------------------
void init_vga64(void) {
    vga_init();
    vga_set_color(VGA_COLOR_WHITE);
    vga_set_bg_color(VGA_COLOR_BLUE);
    vga_clear();
}
