// NebulaOS - VGA Text Mode Driver
// ================================
//
// VGA text mode display driver for kernel debugging and console output

#ifndef NEBULAOS_VGA_H
#define NEBULAOS_VGA_H

#include "nebula.h"
#include "stdint.h"

// VGA text mode constants
#define VGA_WIDTH 80
#define VGA_HEIGHT 25
#define VGA_MEMORY 0xB8000

// VGA colors
#define VGA_COLOR_BLACK         0
#define VGA_COLOR_BLUE          1
#define VGA_COLOR_GREEN         2
#define VGA_COLOR_CYAN          3
#define VGA_COLOR_RED           4
#define VGA_COLOR_MAGENTA       5
#define VGA_COLOR_BROWN         6
#define VGA_COLOR_LIGHT_GRAY    7
#define VGA_COLOR_DARK_GRAY     8
#define VGA_COLOR_LIGHT_BLUE    9
#define VGA_COLOR_LIGHT_GREEN   10
#define VGA_COLOR_LIGHT_CYAN    11
#define VGA_COLOR_LIGHT_RED     12
#define VGA_COLOR_LIGHT_MAGENTA 13
#define VGA_COLOR_LIGHT_BROWN   14
#define VGA_COLOR_YELLOW        14
#define VGA_COLOR_WHITE         15

// Make color from foreground and background
#define VGA_COLOR(fg, bg) ((bg << 4) | fg)

// VGA character (2 bytes: char + color)
typedef struct PACKED {
    uint8_t character;
    uint8_t color;
} vga_char_t;

// VGA memory pointer
typedef volatile vga_char_t* vga_ptr_t;

// Cursor position
typedef struct {
    uint8_t x;
    uint8_t y;
} vga_cursor_t;

// -----------------------------------------------------------------------------
// VGA Functions
// -----------------------------------------------------------------------------

// Initialize VGA text mode
void vga_init(void);

// Clear the screen
void vga_clear(void);

// Clear a single character at position (x, y)
void vga_clear_char(uint8_t x, uint8_t y);

// Set character at position (x, y) with color
void vga_set_char(uint8_t x, uint8_t y, char c, uint8_t color);

// Get character at position (x, y)
vga_char_t vga_get_char(uint8_t x, uint8_t y);

// Scroll the screen up by one line
void vga_scroll_up(void);

// Move cursor to position (x, y)
void vga_set_cursor(uint8_t x, uint8_t y);

// Get cursor position
vga_cursor_t vga_get_cursor(void);

// Enable cursor
void vga_enable_cursor(uint8_t start, uint8_t end);

// Disable cursor
void vga_disable_cursor(void);

// Write a single character to VGA (handles newline, backspace, etc.)
void vga_putchar(char c);

// Write a string to VGA
void vga_puts(const char* str);

// Write a formatted string to VGA (simple printf-like)
void vga_printf(const char* format, ...);

// Set text color
void vga_set_color(uint8_t color);

// Get text color
uint8_t vga_get_color(void);

// Set background color
void vga_set_bg_color(uint8_t bg);

// Get background color
uint8_t vga_get_bg_color(void);

// Save VGA state (for switching modes)
void vga_save_state(void);

// Restore VGA state
void vga_restore_state(void);

// IO port functions for VGA control registers
void vga_outb(uint16_t port, uint8_t value);
uint8_t vga_inb(uint16_t port);

#endif // NEBULAOS_VGA_H
