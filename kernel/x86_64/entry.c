// NebulaOS x86_64 Kernel Entry
// ==============================
//
// C entry point for x86_64 kernel
// Called from start.asm after bootloader sets up long mode

#include "../common/include/nebula.h"
#include "../common/include/stdint.h"
#include "../common/include/vga.h"
#include "../common/include/memory.h"
#include "../common/include/gdt.h"
#include "../common/include/idt.h"
#include "../common/include/isr.h"
#include "../common/include/shell.h"

// Driver includes
#include "../../drivers/include/keyboard.h"
#include "../../drivers/include/pit.h"
#include "../../drivers/include/pic.h"
#include "../../drivers/include/serial.h"
#include "../../drivers/include/pci.h"
#include "../../drivers/include/acpi.h"
#include "../../drivers/include/storage.h"

// Kernel subsystem includes
#include "../../kernel/common/include/process.h"
#include "../../kernel/common/include/scheduler.h"
#include "../../kernel/common/include/syscall.h"
#include "../../kernel/common/include/fs.h"

// Forward declarations
void kernel_early_init(void);
void kernel_init(void);
void kernel_main_loop(void);

// Driver initializers
void init_keyboard(void);

// External functions
extern void init_gdt64(void);
extern void init_idt64(void);
extern void init_paging64(void);
extern void init_timer64(void);
extern void init_keyboard64(void);
extern void init_vga64(void);

// -----------------------------------------------------------------------------
// Kernel main entry point
// Parameters:
//   info: Pointer to boot info structure (unused for now)
// -----------------------------------------------------------------------------
void kernel_main(uint64_t info) {
    (void)info;

    // Early initialization (before memory is available)
    kernel_early_init();

    // Main kernel initialization
    kernel_init();

    // Enter main loop
    kernel_main_loop();
}

// -----------------------------------------------------------------------------
// Early initialization
// Sets up basic CPU state before memory management is available
// -----------------------------------------------------------------------------
void kernel_early_init(void) {
    // Initialize GDT for 64-bit
    init_gdt64();

    // Initialize IDT for 64-bit
    init_idt64();

    // Initialize basic paging (identity mapping)
    init_paging64();

    // Initialize VGA text mode for early output
    init_vga64();

    // Print early boot message
    vga_puts("NebulaOS x86_64 Kernel Booting...\n");
    vga_puts("Initializing hardware...\n");

    // Initialize PIC (Programmable Interrupt Controller)
    pic_init(0x20, 0x28);

    // Initialize PIT (Programmable Interval Timer)
    init_timer64();

    // Initialize keyboard
    init_keyboard64();
}

// -----------------------------------------------------------------------------
// Main kernel initialization
// -----------------------------------------------------------------------------
void kernel_init(void) {
    vga_puts("Initializing memory management...\n");

    // Initialize memory management
    memory_init();
    vga_puts("  Memory: ");

    size_t total = memory_get_total();
    size_t free = memory_get_free();
    char buf[32];

    int len = 0;
    if (total >= 1024 * 1024) {
        len = total / (1024 * 1024);
        buf[0] = '0' + len;
        buf[1] = 'M';
        buf[2] = 'B';
        buf[3] = 0;
    } else {
        len = total / 1024;
        buf[0] = '0' + len / 100;
        buf[1] = '0' + (len / 10) % 10;
        buf[2] = '0' + len % 10;
        buf[3] = 'K';
        buf[4] = 'B';
        buf[5] = 0;
    }
    vga_puts(buf);
    vga_puts(" total, ");

    if (free >= 1024 * 1024) {
        len = free / (1024 * 1024);
        buf[0] = '0' + len;
        buf[1] = 'M';
        buf[2] = 'B';
        buf[3] = 0;
    } else {
        len = free / 1024;
        buf[0] = '0' + len / 100;
        buf[1] = '0' + (len / 10) % 10;
        buf[2] = '0' + len % 10;
        buf[3] = 'K';
        buf[4] = 'B';
        buf[5] = 0;
    }
    vga_puts(buf);
    vga_puts(" free\n");

    // Initialize keyboard
    vga_puts("Initializing keyboard...\n");
    init_keyboard();
    vga_puts("  Keyboard: Ready\n");

    // Initialize process management
    process_init();
    vga_puts("  Processes: Ready\n");

    // Initialize scheduler
    scheduler_init();
    vga_puts("  Scheduler: Ready\n");

    // Initialize syscalls
    syscall_init();
    vga_puts("  Syscalls: Ready\n");

    // Initialize filesystem
    fs_init();
    vga_puts("  Filesystem: Ready\n");

    // Initialize PCI
    pci_init();

    // Initialize ACPI
    acpi_init();

    // Initialize serial
    serial_init();
    vga_puts("  Serial: Ready\n");

    // Initialize storage
    ata_init();

    // Initialize VESA
    vesa_init();

    // Print welcome message
    vga_puts("\n");
    vga_puts("NebulaOS x86_64 Kernel Initialized\n");
    vga_puts("Version: " NEBULAOS_VERSION "\n");

    // Initialize and start the shell
    shell_init();
}

// -----------------------------------------------------------------------------
// Main kernel loop
// -----------------------------------------------------------------------------
void kernel_main_loop(void) {
    // Enable interrupts
    __asm__ __volatile__("sti");

    // Main loop
    while (1) {
        __asm__ __volatile__("hlt");
    }
}

// -----------------------------------------------------------------------------
// Panic function - stop the kernel on unrecoverable error
// -----------------------------------------------------------------------------
void NORETURN kernel_panic(const char* message) {
    __asm__ __volatile__("cli");

    vga_set_color(VGA_COLOR_RED);
    vga_set_bg_color(VGA_COLOR_BLACK);
    vga_puts("\n\nKERNEL PANIC: ");
    vga_puts(message);
    vga_puts("\n\nSystem halted.");

    while (1) {
        __asm__ __volatile__("hlt");
    }
}

// -----------------------------------------------------------------------------
// Assert function
// -----------------------------------------------------------------------------
void kernel_assert(const char* file, int line, const char* condition) {
    kernel_panic("Assertion failed");
}
