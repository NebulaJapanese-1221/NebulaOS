// NebulaOS x86 Kernel Entry
// ===========================
//
// C entry point for x86 kernel
// Called from start.asm after GRUB loads kernel in protected mode

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
#include "../../drivers/include/mouse.h"
#include "../../drivers/include/pit.h"
#include "../../drivers/include/pic.h"
#include "../../drivers/include/serial.h"
#include "../../drivers/include/pci.h"
#include "../../drivers/include/acpi.h"
#include "../../drivers/include/storage.h"
#include "../../drivers/include/vesa.h"
#include "../../drivers/include/vbe.h"

// Real-mode BIOS interface (x86 only)
#include "realmode.h"

// Kernel subsystem includes
#include "../../kernel/common/include/process.h"
#include "../../kernel/common/include/scheduler.h"
#include "../../kernel/common/include/syscall.h"
#include "../../kernel/common/include/fs.h"

// GUI <-> kernel bridge
extern int nebula_gui_enter_graphics(void);
extern void nebula_gui_run(void);

// Forward declarations
void kernel_early_init(void);
void kernel_init(void);
void kernel_main_loop(void);

// Driver initializers
void init_keyboard(void);
void init_mouse(void);

// External assembly functions
extern void init_gdt(void);
extern void init_idt(void);

// -----------------------------------------------------------------------------
// Kernel main entry point
// Parameters from multiboot:
//   magic: Multiboot magic number
//   info:  Pointer to multiboot info structure
// -----------------------------------------------------------------------------
void kernel_main(uint32_t magic, uint32_t info) {
    // Verify multiboot magic
    if (magic != 0x2BADB002) {
        // Not loaded by multiboot - this is an error
        vga_init();
        vga_set_color(VGA_COLOR_RED);
        vga_set_bg_color(VGA_COLOR_BLACK);
        vga_clear();
        vga_puts("ERROR: Invalid multiboot magic!\n");
        vga_puts("Expected: 0x2BADB002\n");
        vga_puts("Got: 0x");
        // Print hex value
        for (int i = 28; i >= 0; i -= 4) {
            uint8_t nibble = (magic >> i) & 0xF;
            char c = nibble < 10 ? '0' + nibble : 'A' + nibble - 10;
            vga_putchar(c);
        }
        vga_puts("\n");
        kernel_panic("Invalid multiboot magic");
    }
    
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
    // Initialize VGA text mode first for output
    vga_init();
    vga_set_color(VGA_COLOR_WHITE);
    vga_set_bg_color(VGA_COLOR_BLUE);
    vga_clear();
    
    vga_puts("NebulaOS x86 Kernel Booting...\n");
    vga_puts("Multiboot magic verified.\n");
    
    // Initialize GDT
    vga_puts("Initializing GDT...\n");
    init_gdt();
    
    // Initialize IDT
    vga_puts("Initializing IDT...\n");
    init_idt();
    
    // Initialize PIC (Programmable Interrupt Controller)
    vga_puts("Initializing PIC...\n");
    pic_init(0x20, 0x28);
    
    // Initialize PIT (Programmable Interval Timer)
    vga_puts("Initializing PIT...\n");
    pit_init(1000);  // 1000 Hz timer
    
    vga_puts("Early initialization complete.\n");
}

// -----------------------------------------------------------------------------
// Main kernel initialization
// -----------------------------------------------------------------------------
void kernel_init(void) {
    // Initialize console
    vga_puts("Initializing memory management...\n");
    
    // Initialize memory management
    memory_init();
    vga_puts("  Memory: ");

    // Initialize the real-mode BIOS interface (VBE INT 0x10 trampoline).
    // Must run after paging is enabled (low memory is identity-mapped).
    if (realmode_init()) {
        vga_puts("  Real-mode BIOS interface: Ready\n");
        vbe_info_block_t vinfo;
        if (vbe_get_info(&vinfo)) {
            vga_puts("  VBE: version ");
            vga_putchar('0' + ((vinfo.version >> 8) & 0xF));
            vga_putchar('.');
            vga_putchar('0' + (vinfo.version & 0xF));
            vga_puts("\n");
        }
    } else {
        vga_puts("  Real-mode BIOS interface: Unavailable (static VBE data)\n");
    }
    
    size_t total = memory_get_total();
    size_t free = memory_get_free();
    char buf[32];
    
    // Simple itoa for display
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
    
    // Initialize mouse
    vga_puts("Initializing mouse...\n");
    init_mouse();
    vga_puts("  Mouse: Ready\n");
    
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
    serial_init(SERIAL_COM1_PORT, 115200);
    vga_puts("  Serial: Ready\n");
    
    // Initialize storage
    ata_init();
    
    // Initialize VESA
    vesa_init();
    
    // Initialize devices
    // device_init();
    
    // Initialize GUI
    vga_puts("Initializing GUI...\n");
    vga_puts("  Switching to real VBE graphics mode...\n");
    if (nebula_gui_enter_graphics()) {
        // Graphics mode is now active and the desktop is drawn to the real
        // linear framebuffer. Run the GUI message loop (does not return).
        nebula_gui_run();
    }
    vga_puts("  GUI: text mode (graphics unavailable)\n");
    
    // Create a test process
    process_create((void*)0x100000, 4096);
    
    // Print welcome message
    vga_puts("\n");
    vga_puts("NebulaOS x86 Kernel Initialized\n");
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
        // Halt the CPU until next interrupt
        __asm__ __volatile__("hlt");
    }
}

// -----------------------------------------------------------------------------
// Panic function - stop the kernel on unrecoverable error
// -----------------------------------------------------------------------------
void NORETURN kernel_panic(const char* message) {
    // Disable interrupts
    __asm__ __volatile__("cli");
    
    // Print panic message
    vga_set_color(VGA_COLOR_RED);
    vga_set_bg_color(VGA_COLOR_BLACK);
    vga_puts("\n\nKERNEL PANIC: ");
    vga_puts(message);
    vga_puts("\n\nSystem halted.");
    
    // Halt
    while (1) {
        __asm__ __volatile__("hlt");
    }
}

// -----------------------------------------------------------------------------
// Assert function
// -----------------------------------------------------------------------------
void kernel_assert(const char* file, int line, const char* condition) {
    // In a real implementation, this would print the assertion failure
    // and call kernel_panic
    char buf[64];
    vga_set_color(VGA_COLOR_YELLOW);
    vga_puts("\nASSERTION FAILED: ");
    vga_puts(condition);
    vga_puts(" at ");
    vga_puts(file);
    vga_puts(":");
    
    // Simple itoa for line number
    int l = line;
    int i = 0;
    if (l >= 100) {
        buf[i++] = '0' + l / 100;
        l %= 100;
    }
    if (l >= 10) {
        buf[i++] = '0' + l / 10;
        l %= 10;
    }
    buf[i++] = '0' + l;
    buf[i] = 0;
    vga_puts(buf);
    vga_puts("\n");
    
    kernel_panic("Assertion failed");
}
