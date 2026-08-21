// NebulaOS - Simple Kernel Shell
// ================================
//
// A basic command-line shell for the kernel
// Accessed via keyboard input

#include "../../common/include/nebula.h"
#include "../../common/include/stdint.h"
#include "../../common/include/vga.h"
#include "../../lib/include/string.h"
#include "../../drivers/include/keyboard.h"

// I/O functions
static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" : : "a"(value), "dN"(port));
}

// -----------------------------------------------------------------------------
// Shell command definitions
// -----------------------------------------------------------------------------

#define SHELL_PROMPT "> "
#define SHELL_BUFFER_SIZE 256
#define SHELL_MAX_COMMANDS 16
#define SHELL_MAX_ARGS 8

// Command structure
typedef struct {
    const char* name;
    const char* description;
    void (*handler)(int argc, char** argv);
} shell_command_t;

// Command buffer
static char command_buffer[SHELL_BUFFER_SIZE];
static uint32_t command_pos = 0;
static uint32_t command_history_pos = 0;
static char* command_history[SHELL_MAX_COMMANDS];
static uint32_t command_history_count = 0;

// -----------------------------------------------------------------------------
// Command handlers
// -----------------------------------------------------------------------------

// Help command
static void cmd_help(int argc, char** argv) {
    (void)argc; (void)argv;
    vga_puts("Available commands:\n");
    vga_puts("  help       - Show this help message\n");
    vga_puts("  clear      - Clear the screen\n");
    vga_puts("  reboot     - Reboot the system\n");
    vga_puts("  meminfo    - Show memory information\n");
    vga_puts("  echo       - Echo arguments\n");
    vga_puts("  color      - Change text color\n");
    vga_puts("  version    - Show kernel version\n");
}

// Clear command
static void cmd_clear(int argc, char** argv) {
    (void)argc; (void)argv;
    vga_clear();
    vga_puts("NebulaOS Shell\n");
    vga_puts(SHELL_PROMPT);
}

// Reboot command
static void cmd_reboot(int argc, char** argv) {
    (void)argc; (void)argv;
    vga_puts("Rebooting...\n");
    
    // Reset the system
    uint8_t good = 0x02;
    while (good & 0x02)
        good = inb(0x64);
    outb(0x64, 0xFE);
    
    // If that doesn't work, triple fault
    __asm__ __volatile__("int $0");
}

// Memory info command
#include "../../common/include/memory.h"

static void cmd_meminfo(int argc, char** argv) {
    (void)argc; (void)argv;
    
    size_t total = memory_get_total();
    size_t used = memory_get_used();
    size_t free = memory_get_free();
    
    char buf[32];
    
    vga_puts("Memory Information:\n");
    
    // Total memory
    vga_puts("  Total: ");
    if (total >= 1024 * 1024) {
        int mb = total / (1024 * 1024);
        buf[0] = '0' + mb;
        buf[1] = 'M';
        buf[2] = 'B';
        buf[3] = 0;
    } else {
        int kb = total / 1024;
        buf[0] = '0' + kb / 100;
        buf[1] = '0' + (kb / 10) % 10;
        buf[2] = '0' + kb % 10;
        buf[3] = 'K';
        buf[4] = 'B';
        buf[5] = 0;
    }
    vga_puts(buf);
    vga_puts("\n");
    
    // Used memory
    vga_puts("  Used: ");
    if (used >= 1024 * 1024) {
        int mb = used / (1024 * 1024);
        buf[0] = '0' + mb;
        buf[1] = 'M';
        buf[2] = 'B';
        buf[3] = 0;
    } else {
        int kb = used / 1024;
        buf[0] = '0' + kb / 100;
        buf[1] = '0' + (kb / 10) % 10;
        buf[2] = '0' + kb % 10;
        buf[3] = 'K';
        buf[4] = 'B';
        buf[5] = 0;
    }
    vga_puts(buf);
    vga_puts("\n");
    
    // Free memory
    vga_puts("  Free: ");
    if (free >= 1024 * 1024) {
        int mb = free / (1024 * 1024);
        buf[0] = '0' + mb;
        buf[1] = 'M';
        buf[2] = 'B';
        buf[3] = 0;
    } else {
        int kb = free / 1024;
        buf[0] = '0' + kb / 100;
        buf[1] = '0' + (kb / 10) % 10;
        buf[2] = '0' + kb % 10;
        buf[3] = 'K';
        buf[4] = 'B';
        buf[5] = 0;
    }
    vga_puts(buf);
    vga_puts("\n");
}

// Echo command
static void cmd_echo(int argc, char** argv) {
    if (argc < 2) {
        vga_puts("Usage: echo <message>\n");
        return;
    }
    
    for (int i = 1; i < argc; i++) {
        vga_puts(argv[i]);
        if (i < argc - 1) vga_puts(" ");
    }
    vga_puts("\n");
}

// Color command
static void cmd_color(int argc, char** argv) {
    if (argc < 2) {
        vga_puts("Usage: color <foreground> [background]\n");
        vga_puts("Colors: black, blue, green, cyan, red, magenta, brown, white\n");
        return;
    }
    
    uint8_t fg = VGA_COLOR_WHITE;
    uint8_t bg = VGA_COLOR_BLACK;
    
    // Parse foreground
    if (strcmp(argv[1], "black") == 0) fg = VGA_COLOR_BLACK;
    else if (strcmp(argv[1], "blue") == 0) fg = VGA_COLOR_BLUE;
    else if (strcmp(argv[1], "green") == 0) fg = VGA_COLOR_GREEN;
    else if (strcmp(argv[1], "cyan") == 0) fg = VGA_COLOR_CYAN;
    else if (strcmp(argv[1], "red") == 0) fg = VGA_COLOR_RED;
    else if (strcmp(argv[1], "magenta") == 0) fg = VGA_COLOR_MAGENTA;
    else if (strcmp(argv[1], "brown") == 0) fg = VGA_COLOR_BROWN;
    else if (strcmp(argv[1], "white") == 0) fg = VGA_COLOR_WHITE;
    
    // Parse background if provided
    if (argc >= 3) {
        if (strcmp(argv[2], "black") == 0) bg = VGA_COLOR_BLACK;
        else if (strcmp(argv[2], "blue") == 0) bg = VGA_COLOR_BLUE;
        else if (strcmp(argv[2], "green") == 0) bg = VGA_COLOR_GREEN;
        else if (strcmp(argv[2], "cyan") == 0) bg = VGA_COLOR_CYAN;
        else if (strcmp(argv[2], "red") == 0) bg = VGA_COLOR_RED;
        else if (strcmp(argv[2], "magenta") == 0) bg = VGA_COLOR_MAGENTA;
        else if (strcmp(argv[2], "brown") == 0) bg = VGA_COLOR_BROWN;
        else if (strcmp(argv[2], "white") == 0) bg = VGA_COLOR_WHITE;
    }
    
    vga_set_color(fg);
    vga_set_bg_color(bg);
}

// Version command
static void cmd_version(int argc, char** argv) {
    (void)argc; (void)argv;
    vga_puts("NebulaOS Version: " NEBULAOS_VERSION "\n");
    vga_puts("Kernel: x86\n");
    vga_puts("Build: " __DATE__ " " __TIME__ "\n");
}

// -----------------------------------------------------------------------------
// Command table
// -----------------------------------------------------------------------------

static shell_command_t commands[] = {
    {"help", "Show this help message", cmd_help},
    {"clear", "Clear the screen", cmd_clear},
    {"reboot", "Reboot the system", cmd_reboot},
    {"meminfo", "Show memory information", cmd_meminfo},
    {"echo", "Echo arguments", cmd_echo},
    {"color", "Change text color", cmd_color},
    {"version", "Show kernel version", cmd_version},
    {NULL, NULL, NULL}
};

// -----------------------------------------------------------------------------
// Shell utility functions
// -----------------------------------------------------------------------------

// Parse command line into arguments
static int parse_command(char* input, char** argv, int max_args) {
    int argc = 0;
    char* ptr = input;
    char* start = ptr;
    
    // Skip leading whitespace
    while (*ptr == ' ' || *ptr == '\t') ptr++;
    
    while (*ptr && argc < max_args - 1) {
        if (*ptr == ' ' || *ptr == '\t') {
            // End of argument
            if (ptr > start) {
                *ptr = 0;
                argv[argc++] = start;
            }
            start = ptr + 1;
        }
        ptr++;
    }
    
    // Last argument
    if (*start && argc < max_args - 1) {
        argv[argc++] = start;
    }
    
    argv[argc] = NULL;
    return argc;
}

// -----------------------------------------------------------------------------
// Shell input handling
// -----------------------------------------------------------------------------

// Process a scancode
static void shell_process_scancode(uint8_t scancode) {
    uint8_t modifiers = keyboard_get_modifiers();
    
    // Check for break code
    if (scancode & 0x80) {
        // Key released
        return;
    }
    
    // Handle special keys
    switch (scancode) {
        case KB_SCANCODE_ENTER:
            vga_putchar('\n');
            
            if (command_pos > 0) {
                // Save command to history
                if (command_history_count < SHELL_MAX_COMMANDS) {
                    command_buffer[command_pos] = 0;
                    command_history[command_history_count] = strdup(command_buffer);
                    command_history_count++;
                    command_history_pos = command_history_count;
                }
                
                // Parse and execute command
                char* argv[SHELL_MAX_ARGS + 1];
                int argc = parse_command(command_buffer, argv, SHELL_MAX_ARGS);
                
                if (argc > 0) {
                    // Find and execute command
                    bool found = false;
                    for (int i = 0; commands[i].name != NULL; i++) {
                        if (strcmp(argv[0], commands[i].name) == 0) {
                            commands[i].handler(argc, argv);
                            found = true;
                            break;
                        }
                    }
                    
                    if (!found) {
                        vga_puts("Unknown command: ");
                        vga_puts(argv[0]);
                        vga_puts("\n");
                    }
                }
            }
            
            // Reset command buffer
            command_pos = 0;
            command_buffer[0] = 0;
            
            // Print prompt
            vga_puts(SHELL_PROMPT);
            break;
            
        case KB_SCANCODE_BACKSPACE:
            if (command_pos > 0) {
                command_pos--;
                command_buffer[command_pos] = 0;
                vga_putchar('\b');
                vga_putchar(' ');
                vga_putchar('\b');
            }
            break;
            
        case KB_SCANCODE_UP:
            // History up
            if (command_history_count > 0 && command_history_pos > 0) {
                command_history_pos--;
                
                // Clear current line
                while (command_pos > 0) {
                    command_pos--;
                    vga_putchar('\b');
                    vga_putchar(' ');
                    vga_putchar('\b');
                }
                
                // Copy history entry
                char* hist = command_history[command_history_pos];
                strcpy(command_buffer, hist);
                command_pos = strlen(hist);
                vga_puts(hist);
            }
            break;
            
        case KB_SCANCODE_DOWN:
            // History down
            if (command_history_pos < command_history_count - 1) {
                command_history_pos++;
                
                // Clear current line
                while (command_pos > 0) {
                    command_pos--;
                    vga_putchar('\b');
                    vga_putchar(' ');
                    vga_putchar('\b');
                }
                
                // Copy history entry
                char* hist = command_history[command_history_pos];
                strcpy(command_buffer, hist);
                command_pos = strlen(hist);
                vga_puts(hist);
            } else if (command_history_pos == command_history_count - 1) {
                // Clear line
                while (command_pos > 0) {
                    command_pos--;
                    vga_putchar('\b');
                    vga_putchar(' ');
                    vga_putchar('\b');
                }
                command_buffer[0] = 0;
                command_pos = 0;
                command_history_pos = command_history_count;
            }
            break;
            
        default:
            // Regular character
            char c = keyboard_scancode_to_ascii(scancode, modifiers);
            if (c >= 32 && c <= 126) {  // Printable ASCII
                if (command_pos < SHELL_BUFFER_SIZE - 1) {
                    command_buffer[command_pos++] = c;
                    command_buffer[command_pos] = 0;
                    vga_putchar(c);
                }
            }
            break;
    }
}

// -----------------------------------------------------------------------------
// Keyboard callback
// -----------------------------------------------------------------------------

static void shell_keyboard_callback(uint8_t scancode, uint8_t modifiers, bool pressed) {
    if (pressed) {
        shell_process_scancode(scancode);
    }
}

// -----------------------------------------------------------------------------
// Shell initialization
// -----------------------------------------------------------------------------

void shell_init(void) {
    // Initialize command buffer
    command_pos = 0;
    command_buffer[0] = 0;
    command_history_count = 0;
    command_history_pos = 0;
    
    // Set keyboard callback
    keyboard_set_handler(shell_keyboard_callback);
    
    // Print welcome message
    vga_puts("NebulaOS Shell\n");
    vga_puts("Type 'help' for available commands\n");
    vga_puts(SHELL_PROMPT);
}

// -----------------------------------------------------------------------------
// Shell main loop
// -----------------------------------------------------------------------------

void shell_run(void) {
    shell_init();
    
    // Enable interrupts
    __asm__ __volatile__("sti");
    
    // Main loop - just wait for interrupts
    while (1) {
        __asm__ __volatile__("hlt");
    }
}
