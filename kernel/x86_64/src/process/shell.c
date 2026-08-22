// NebulaOS - x86_64 Simple Kernel Shell
// ======================================
//
// A basic command-line shell for the x86_64 kernel
// Accessed via keyboard input

#include "../../common/include/nebula.h"
#include "../../common/include/stdint.h"
#include "../../common/include/vga.h"
#include "../../lib/include/string.h"
#include "../../drivers/include/keyboard.h"
#include "../../common/include/memory.h"
#include "../../common/include/process.h"
#include "../../common/include/scheduler.h"
#include "../../common/include/syscall.h"
#include "../../common/include/fs.h"
#include "../../common/include/elf.h"

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

    uint8_t good = 0x02;
    while (good & 0x02)
        good = inb(0x64);
    outb(0x64, 0xFE);

    __asm__ __volatile__("int $0");
}

// Memory info command
static void cmd_meminfo(int argc, char** argv) {
    (void)argc; (void)argv;

    size_t total = memory_get_total();
    size_t used = memory_get_used();
    size_t free = memory_get_free();

    char buf[32];

    vga_puts("Memory Information:\n");

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

    if (strcmp(argv[1], "black") == 0) fg = VGA_COLOR_BLACK;
    else if (strcmp(argv[1], "blue") == 0) fg = VGA_COLOR_BLUE;
    else if (strcmp(argv[1], "green") == 0) fg = VGA_COLOR_GREEN;
    else if (strcmp(argv[1], "cyan") == 0) fg = VGA_COLOR_CYAN;
    else if (strcmp(argv[1], "red") == 0) fg = VGA_COLOR_RED;
    else if (strcmp(argv[1], "magenta") == 0) fg = VGA_COLOR_MAGENTA;
    else if (strcmp(argv[1], "brown") == 0) fg = VGA_COLOR_BROWN;
    else if (strcmp(argv[1], "white") == 0) fg = VGA_COLOR_WHITE;

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
    vga_puts("Kernel: x86_64\n");
    vga_puts("Build: " __DATE__ " " __TIME__ "\n");
}

// -----------------------------------------------------------------------------
// Helper functions
// -----------------------------------------------------------------------------

static void itoa(int value, char* str) {
    int i = 0;
    int is_negative = 0;
    
    if (value == 0) {
        str[i++] = '0';
        str[i] = 0;
        return;
    }
    
    if (value < 0) {
        is_negative = 1;
        value = -value;
    }
    
    while (value > 0) {
        str[i++] = '0' + (value % 10);
        value /= 10;
    }
    
    if (is_negative) {
        str[i++] = '-';
    }
    
    str[i] = 0;
    
    for (int j = 0; j < i / 2; j++) {
        char tmp = str[j];
        str[j] = str[i - 1 - j];
        str[i - 1 - j] = tmp;
    }
}

// -----------------------------------------------------------------------------
// ls command
// -----------------------------------------------------------------------------

static void cmd_ls(int argc, char** argv) {
    (void)argc; (void)argv;
    
    fs_dir_t dir;
    if (fs_list("/", &dir) != FS_SUCCESS) {
        vga_puts("Failed to list directory\n");
        return;
    }
    
    for (uint32_t i = 0; i < dir.count; i++) {
        vga_puts(dir.entries[i].name);
        if (dir.entries[i].type == FS_TYPE_DIR) {
            vga_puts("/");
        }
        vga_puts("  ");
        
        char size_str[16];
        itoa(dir.entries[i].size, size_str);
        vga_puts(size_str);
        vga_puts(" bytes\n");
    }
    
    free(dir.entries);
}

// -----------------------------------------------------------------------------
// cat command
// -----------------------------------------------------------------------------

static void cmd_cat(int argc, char** argv) {
    if (argc < 2) {
        vga_puts("Usage: cat <filename>\n");
        return;
    }
    
    fs_file_t file;
    if (fs_open(argv[1], &file, FS_FLAG_READ) != FS_SUCCESS) {
        vga_puts("Failed to open file: ");
        vga_puts(argv[1]);
        vga_puts("\n");
        return;
    }
    
    char buf[256];
    uint32_t total = 0;
    while (total < file.size) {
        int bytes = fs_read(&file, buf, sizeof(buf) - 1);
        if (bytes <= 0) break;
        buf[bytes] = 0;
        vga_puts(buf);
        total += bytes;
    }
    
    fs_close(&file);
}

// -----------------------------------------------------------------------------
// ps command
// -----------------------------------------------------------------------------

static void cmd_ps(int argc, char** argv) {
    (void)argc; (void)argv;
    
    vga_puts("PID  STATE     ENTRY\n");
    
    for (int i = 0; i < MAX_PROCESSES; i++) {
        process_state_t state = process_get_state(i);
        if (state == PROC_ZOMBIE) continue;
        
        uint32_t pid = process_get_pid(i);
        char pid_str[16];
        itoa(pid, pid_str);
        vga_puts(pid_str);
        vga_puts("  ");
        
        switch (state) {
            case PROC_READY: vga_puts("READY   "); break;
            case PROC_RUNNING: vga_puts("RUNNING "); break;
            case PROC_BLOCKED: vga_puts("BLOCKED "); break;
            case PROC_ZOMBIE: vga_puts("ZOMBIE  "); break;
            default: vga_puts("UNKNOWN "); break;
        }
        
        void* entry = process_get_entry_point(i);
        char entry_str[16];
        itoa((int)entry, entry_str);
        vga_puts(entry_str);
        vga_puts("\n");
    }
}

// -----------------------------------------------------------------------------
// exec command
// -----------------------------------------------------------------------------

static void cmd_exec(int argc, char** argv) {
    if (argc < 2) {
        vga_puts("Usage: exec <filename>\n");
        return;
    }
    
    fs_file_t file;
    if (fs_open(argv[1], &file, FS_FLAG_READ) != FS_SUCCESS) {
        vga_puts("Failed to open file: ");
        vga_puts(argv[1]);
        vga_puts("\n");
        return;
    }
    
    if (file.size == 0 || file.size > 65536) {
        vga_puts("Invalid file size\n");
        fs_close(&file);
        return;
    }
    
    char* buf = malloc(file.size);
    if (!buf) {
        vga_puts("Failed to allocate memory\n");
        fs_close(&file);
        return;
    }
    
    int bytes = fs_read(&file, buf, file.size);
    fs_close(&file);
    
    if (bytes <= 0) {
        vga_puts("Failed to read file\n");
        free(buf);
        return;
    }
    
    void* entry_point = NULL;
    if (elf_load(buf, &entry_point) != 0) {
        vga_puts("Failed to load ELF: ");
        vga_puts(argv[1]);
        vga_puts("\n");
        free(buf);
        return;
    }
    
    int pid = process_create(entry_point, 8192);
    if (pid < 0) {
        vga_puts("Failed to create process\n");
        free(buf);
        return;
    }
    
    char pid_str[16];
    itoa(pid, pid_str);
    vga_puts("Created process PID: ");
    vga_puts(pid_str);
    vga_puts("\n");
    
    free(buf);
}

// -----------------------------------------------------------------------------
// kill command
// -----------------------------------------------------------------------------

static void cmd_kill(int argc, char** argv) {
    if (argc < 2) {
        vga_puts("Usage: kill <pid>\n");
        return;
    }
    
    int pid = 0;
    for (int i = 0; argv[1][i]; i++) {
        if (argv[1][i] >= '0' && argv[1][i] <= '9') {
            pid = pid * 10 + (argv[1][i] - '0');
        }
    }
    
    if (pid <= 0) {
        vga_puts("Invalid PID\n");
        return;
    }
    
    process_destroy(pid);
    scheduler_remove(pid);
    
    vga_puts("Killed process ");
    vga_puts(argv[1]);
    vga_puts("\n");
}

// -----------------------------------------------------------------------------
// touch command (stub)
// -----------------------------------------------------------------------------

static void cmd_touch(int argc, char** argv) {
    (void)argc; (void)argv;
    vga_puts("touch: not implemented\n");
}

// -----------------------------------------------------------------------------
// mkdir command (stub)
// -----------------------------------------------------------------------------

static void cmd_mkdir(int argc, char** argv) {
    (void)argc; (void)argv;
    vga_puts("mkdir: not implemented\n");
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
    {"ls", "List directory contents", cmd_ls},
    {"dir", "List directory contents", cmd_ls},
    {"cat", "Display file contents", cmd_cat},
    {"ps", "List processes", cmd_ps},
    {"exec", "Execute ELF binary", cmd_exec},
    {"run", "Execute ELF binary", cmd_exec},
    {"kill", "Terminate process", cmd_kill},
    {"touch", "Create empty file (stub)", cmd_touch},
    {"mkdir", "Create directory (stub)", cmd_mkdir},
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

    while (*ptr == ' ' || *ptr == '\t') ptr++;

    while (*ptr && argc < max_args - 1) {
        if (*ptr == ' ' || *ptr == '\t') {
            if (ptr > start) {
                *ptr = 0;
                argv[argc++] = start;
            }
            start = ptr + 1;
        }
        ptr++;
    }

    if (*start && argc < max_args - 1) {
        argv[argc++] = start;
    }

    argv[argc] = NULL;
    return argc;
}

// -----------------------------------------------------------------------------
// Shell input handling
// -----------------------------------------------------------------------------

static void shell_process_scancode(uint8_t scancode) {
    uint8_t modifiers = keyboard_get_modifiers();

    if (scancode & 0x80) {
        return;
    }

    switch (scancode) {
        case KB_SCANCODE_ENTER:
            vga_putchar('\n');

            if (command_pos > 0) {
                if (command_history_count < SHELL_MAX_COMMANDS) {
                    command_buffer[command_pos] = 0;
                    command_history[command_history_count] = strdup(command_buffer);
                    command_history_count++;
                    command_history_pos = command_history_count;
                }

                char* argv[SHELL_MAX_ARGS + 1];
                int argc = parse_command(command_buffer, argv, SHELL_MAX_ARGS);

                if (argc > 0) {
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

            command_pos = 0;
            command_buffer[0] = 0;
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

        default:
            char c = keyboard_scancode_to_ascii(scancode, modifiers);
            if (c >= 32 && c <= 126) {
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
    command_pos = 0;
    command_buffer[0] = 0;
    command_history_count = 0;
    command_history_pos = 0;

    keyboard_set_handler(shell_keyboard_callback);
    
    fs_mount();
    
    vga_puts("NebulaOS Shell\n");
    vga_puts("Type 'help' for available commands\n");
    vga_puts(SHELL_PROMPT);
}

// -----------------------------------------------------------------------------
// Shell main loop
// -----------------------------------------------------------------------------

void shell_run(void) {
    shell_init();

    __asm__ __volatile__("sti");

    while (1) {
        __asm__ __volatile__("hlt");
    }
}
