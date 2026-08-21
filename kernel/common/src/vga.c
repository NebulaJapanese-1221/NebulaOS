// NebulaOS - VGA Text Mode Driver Implementation
// ================================================
//
// Implementation of VGA text mode display driver

#include "../include/vga.h"
#include "../include/nebula.h"
#include "../include/stdint.h"

// VGA memory pointer
static vga_ptr_t vga_memory = (vga_ptr_t)VGA_MEMORY;

// Current cursor position
static vga_cursor_t cursor = {0, 0};

// Current text color
static uint8_t current_color = VGA_COLOR(VGA_COLOR_WHITE, VGA_COLOR_BLACK);

// Saved VGA state
static uint8_t saved_cursor_start = 0;
static uint8_t saved_cursor_end = 0;
static uint16_t saved_cursor_pos = 0;

// -----------------------------------------------------------------------------
// Initialize VGA text mode
// -----------------------------------------------------------------------------
void vga_init(void) {
    // Clear the screen
    vga_clear();
    
    // Set cursor to (0, 0)
    vga_set_cursor(0, 0);
    
    // Enable cursor (blinking block)
    vga_enable_cursor(0x0E, 0x0F);  // Start=14, End=15 (full block)
}

// -----------------------------------------------------------------------------
// Clear the screen
// -----------------------------------------------------------------------------
void vga_clear(void) {
    for (uint16_t y = 0; y < VGA_HEIGHT; y++) {
        for (uint16_t x = 0; x < VGA_WIDTH; x++) {
            vga_set_char(x, y, ' ', current_color);
        }
    }
    vga_set_cursor(0, 0);
}

// -----------------------------------------------------------------------------
// Clear a single character at position (x, y)
// -----------------------------------------------------------------------------
void vga_clear_char(uint8_t x, uint8_t y) {
    if (x < VGA_WIDTH && y < VGA_HEIGHT) {
        vga_memory[y * VGA_WIDTH + x].character = ' ';
        vga_memory[y * VGA_WIDTH + x].color = current_color;
    }
}

// -----------------------------------------------------------------------------
// Set character at position (x, y) with color
// -----------------------------------------------------------------------------
void vga_set_char(uint8_t x, uint8_t y, char c, uint8_t color) {
    if (x < VGA_WIDTH && y < VGA_HEIGHT) {
        vga_memory[y * VGA_WIDTH + x].character = c;
        vga_memory[y * VGA_WIDTH + x].color = color;
    }
}

// -----------------------------------------------------------------------------
// Get character at position (x, y)
// -----------------------------------------------------------------------------
vga_char_t vga_get_char(uint8_t x, uint8_t y) {
    if (x < VGA_WIDTH && y < VGA_HEIGHT) {
        return vga_memory[y * VGA_WIDTH + x];
    }
    vga_char_t empty = {' ', current_color};
    return empty;
}

// -----------------------------------------------------------------------------
// Scroll the screen up by one line
// -----------------------------------------------------------------------------
void vga_scroll_up(void) {
    // Move all lines up
    for (uint16_t y = 1; y < VGA_HEIGHT; y++) {
        for (uint16_t x = 0; x < VGA_WIDTH; x++) {
            vga_memory[(y - 1) * VGA_WIDTH + x] = vga_memory[y * VGA_WIDTH + x];
        }
    }
    
    // Clear the last line
    for (uint16_t x = 0; x < VGA_WIDTH; x++) {
        vga_set_char(x, VGA_HEIGHT - 1, ' ', current_color);
    }
    
    // Move cursor up
    if (cursor.y > 0) {
        cursor.y--;
    }
    vga_set_cursor(cursor.x, cursor.y);
}

// -----------------------------------------------------------------------------
// Move cursor to position (x, y)
// -----------------------------------------------------------------------------
void vga_set_cursor(uint8_t x, uint8_t y) {
    if (x < VGA_WIDTH && y < VGA_HEIGHT) {
        cursor.x = x;
        cursor.y = y;
        
        // Calculate cursor position in VGA register format
        uint16_t pos = y * VGA_WIDTH + x;
        
        // Set cursor position (CRTC registers)
        vga_outb(0x3D4, 0x0F);  // High byte
        vga_outb(0x3D5, (uint8_t)(pos >> 8));
        vga_outb(0x3D4, 0x0E);  // Low byte
        vga_outb(0x3D5, (uint8_t)(pos & 0xFF));
    }
}

// -----------------------------------------------------------------------------
// Get cursor position
// -----------------------------------------------------------------------------
vga_cursor_t vga_get_cursor(void) {
    return cursor;
}

// -----------------------------------------------------------------------------
// Enable cursor
// -----------------------------------------------------------------------------
void vga_enable_cursor(uint8_t start, uint8_t end) {
    vga_outb(0x3D4, 0x0A);
    vga_outb(0x3D5, (vga_inb(0x3D5) & 0xC0) | start);
    
    vga_outb(0x3D4, 0x0B);
    vga_outb(0x3D5, (vga_inb(0x3D5) & 0xE0) | end);
    
    saved_cursor_start = start;
    saved_cursor_end = end;
}

// -----------------------------------------------------------------------------
// Disable cursor
// -----------------------------------------------------------------------------
void vga_disable_cursor(void) {
    vga_outb(0x3D4, 0x0A);
    vga_outb(0x3D5, 0x20);  // Set start to 32 (off-screen)
}

// -----------------------------------------------------------------------------
// Write a single character to VGA
// -----------------------------------------------------------------------------
void vga_putchar(char c) {
    switch (c) {
        case '\n':
            cursor.x = 0;
            cursor.y++;
            if (cursor.y >= VGA_HEIGHT) {
                vga_scroll_up();
                cursor.y = VGA_HEIGHT - 1;
            }
            vga_set_cursor(cursor.x, cursor.y);
            break;
            
        case '\r':
            cursor.x = 0;
            vga_set_cursor(cursor.x, cursor.y);
            break;
            
        case '\t':
            // Tab: move to next multiple of 4
            cursor.x = (cursor.x + 4) & ~3;
            if (cursor.x >= VGA_WIDTH) {
                cursor.x = 0;
                cursor.y++;
                if (cursor.y >= VGA_HEIGHT) {
                    vga_scroll_up();
                    cursor.y = VGA_HEIGHT - 1;
                }
            }
            vga_set_cursor(cursor.x, cursor.y);
            break;
            
        case '\b':
            // Backspace
            if (cursor.x > 0) {
                cursor.x--;
            } else if (cursor.y > 0) {
                cursor.y--;
                cursor.x = VGA_WIDTH - 1;
            }
            vga_set_char(cursor.x, cursor.y, ' ', current_color);
            vga_set_cursor(cursor.x, cursor.y);
            break;
            
        default:
            vga_set_char(cursor.x, cursor.y, c, current_color);
            cursor.x++;
            if (cursor.x >= VGA_WIDTH) {
                cursor.x = 0;
                cursor.y++;
                if (cursor.y >= VGA_HEIGHT) {
                    vga_scroll_up();
                    cursor.y = VGA_HEIGHT - 1;
                }
            }
            vga_set_cursor(cursor.x, cursor.y);
            break;
    }
}

// -----------------------------------------------------------------------------
// Write a string to VGA
// -----------------------------------------------------------------------------
void vga_puts(const char* str) {
    if (str == NULL) {
        return;
    }
    
    while (*str) {
        vga_putchar(*str++);
    }
}

// -----------------------------------------------------------------------------
// Simple printf-like function for VGA
// -----------------------------------------------------------------------------
void vga_printf(const char* format, ...) {
    // In a real implementation, this would parse format specifiers
    // For now, just print the format string as-is
    vga_puts(format);
}

// -----------------------------------------------------------------------------
// Set text color
// -----------------------------------------------------------------------------
void vga_set_color(uint8_t color) {
    current_color = (current_color & 0xF0) | (color & 0x0F);
}

// -----------------------------------------------------------------------------
// Get text color
// -----------------------------------------------------------------------------
uint8_t vga_get_color(void) {
    return current_color & 0x0F;
}

// -----------------------------------------------------------------------------
// Set background color
// -----------------------------------------------------------------------------
void vga_set_bg_color(uint8_t bg) {
    current_color = (current_color & 0x0F) | ((bg & 0x0F) << 4);
}

// -----------------------------------------------------------------------------
// Get background color
// -----------------------------------------------------------------------------
uint8_t vga_get_bg_color(void) {
    return (current_color >> 4) & 0x0F;
}

// -----------------------------------------------------------------------------
// Save VGA state
// -----------------------------------------------------------------------------
void vga_save_state(void) {
    // Save cursor registers
    vga_outb(0x3D4, 0x0A);
    saved_cursor_start = vga_inb(0x3D5);
    
    vga_outb(0x3D4, 0x0B);
    saved_cursor_end = vga_inb(0x3D5);
    
    vga_outb(0x3D4, 0x0E);
    saved_cursor_pos = vga_inb(0x3D5);
    
    vga_outb(0x3D4, 0x0F);
    saved_cursor_pos |= (uint16_t)vga_inb(0x3D5) << 8;
    
    // Save cursor position
    cursor.x = saved_cursor_pos % VGA_WIDTH;
    cursor.y = saved_cursor_pos / VGA_WIDTH;
}

// -----------------------------------------------------------------------------
// Restore VGA state
// -----------------------------------------------------------------------------
void vga_restore_state(void) {
    // Restore cursor registers
    vga_outb(0x3D4, 0x0A);
    vga_outb(0x3D5, saved_cursor_start);
    
    vga_outb(0x3D4, 0x0B);
    vga_outb(0x3D5, saved_cursor_end);
    
    vga_outb(0x3D4, 0x0E);
    vga_outb(0x3D5, (uint8_t)(saved_cursor_pos & 0xFF));
    
    vga_outb(0x3D4, 0x0F);
    vga_outb(0x3D5, (uint8_t)(saved_cursor_pos >> 8));
    
    // Restore cursor position
    cursor.x = saved_cursor_pos % VGA_WIDTH;
    cursor.y = saved_cursor_pos / VGA_WIDTH;
}

// -----------------------------------------------------------------------------
// VGA IO port functions
// -----------------------------------------------------------------------------
void vga_outb(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" : : "a"(value), "dN"(port));
}

uint8_t vga_inb(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "dN"(port));
    return value;
}
