// NebulaOS - PS/2 Keyboard Driver
// ==================================
//
// Implementation of PS/2 keyboard driver

#include "../include/keyboard.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/idt.h"

// Keyboard state
static uint8_t keyboard_modifiers = 0;
static uint8_t keyboard_leds = 0;
static bool keyboard_initialized = false;
static keyboard_handler_t keyboard_callback = NULL;

// Key state array (for tracking pressed keys)
static bool key_pressed[256] = {false};

// Keyboard buffer
#define KEYBOARD_BUFFER_SIZE 256
static uint8_t keyboard_buffer[KEYBOARD_BUFFER_SIZE];
static uint32_t keyboard_buffer_head = 0;
static uint32_t keyboard_buffer_tail = 0;
static uint32_t keyboard_buffer_count = 0;

// -----------------------------------------------------------------------------
// I/O functions
// -----------------------------------------------------------------------------

static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" : : "a"(value), "dN"(port));
}

// -----------------------------------------------------------------------------
// Wait for keyboard controller to be ready
// -----------------------------------------------------------------------------

static void keyboard_wait() {
    // Wait for input buffer to be empty
    while (inb(KEYBOARD_STATUS_PORT) & KB_STATUS_IBF) {
        // Wait
    }
}

// -----------------------------------------------------------------------------
// Wait for keyboard data to be available
// -----------------------------------------------------------------------------

static bool keyboard_data_available() {
    return (inb(KEYBOARD_STATUS_PORT) & KB_STATUS_OBF) != 0;
}

// -----------------------------------------------------------------------------
// Initialize keyboard
// -----------------------------------------------------------------------------

void keyboard_init() {
    if (keyboard_initialized) return;
    
    // Clear keyboard buffer
    keyboard_buffer_head = 0;
    keyboard_buffer_tail = 0;
    keyboard_buffer_count = 0;
    
    // Clear key states
    for (int i = 0; i < 256; i++) {
        key_pressed[i] = false;
    }
    
    // Disable keyboard
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_DISABLE);
    
    // Wait for acknowledgment
    keyboard_wait();
    
    // Read current mode
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_READ_MODE);
    keyboard_wait();
    uint8_t mode = inb(KEYBOARD_DATA_PORT);
    
    // Set mode (disable translation, enable interrupt)
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_WRITE_MODE);
    keyboard_wait();
    outb(KEYBOARD_DATA_PORT, mode & 0x7F);  // Clear bit 7 (translation)
    
    // Enable keyboard
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_ENABLE);
    
    // Self test
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_SELF_TEST);
    keyboard_wait();
    uint8_t result = inb(KEYBOARD_DATA_PORT);
    if (result != 0x55) {
        // Self test failed
        return;
    }
    
    // Test keyboard interface
    keyboard_wait();
    outb(KEYBOARD_DATA_PORT, KB_CMD_INTERFACE_TEST);
    keyboard_wait();
    result = inb(KEYBOARD_DATA_PORT);
    if (result != 0x00) {
        // Interface test failed
        return;
    }
    
    // Set LED state
    keyboard_set_leds(0);
    
    keyboard_initialized = true;
    keyboard_modifiers = 0;
}

// -----------------------------------------------------------------------------
// Set keyboard LEDs
// -----------------------------------------------------------------------------

void keyboard_set_leds(uint8_t leds) {
    keyboard_leds = leds;
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_WRITE_OUTPUT);
    keyboard_wait();
    outb(KEYBOARD_DATA_PORT, leds);
}

// -----------------------------------------------------------------------------
// Keyboard IRQ handler (IRQ1)
// -----------------------------------------------------------------------------

void keyboard_irq_handler() {
    // Read scan code from keyboard
    uint8_t scancode = inb(KEYBOARD_DATA_PORT);
    
    // Check if it's a break code (key released)
    bool pressed = !(scancode & 0x80);
    uint8_t code = scancode & 0x7F;
    
    // Update key state
    key_pressed[code] = pressed;
    
    // Update modifiers
    switch (code) {
        case KB_SCANCODE_LSHIFT:
        case KB_SCANCODE_RSHIFT:
            if (pressed) {
                keyboard_modifiers |= KB_MODIFIER_SHIFT;
            } else {
                keyboard_modifiers &= ~KB_MODIFIER_SHIFT;
            }
            break;
        case KB_SCANCODE_LCTRL:
            if (pressed) {
                keyboard_modifiers |= KB_MODIFIER_CTRL;
            } else {
                keyboard_modifiers &= ~KB_MODIFIER_CTRL;
            }
            break;
        case KB_SCANCODE_LALT:
            if (pressed) {
                keyboard_modifiers |= KB_MODIFIER_ALT;
            } else {
                keyboard_modifiers &= ~KB_MODIFIER_ALT;
            }
            break;
        case KB_SCANCODE_CAPSLOCK:
            if (pressed) {
                keyboard_modifiers ^= KB_MODIFIER_CAPS;
                keyboard_set_leds(keyboard_leds ^ 0x04);  // Toggle caps lock LED
            }
            break;
        case KB_SCANCODE_NUMLOCK:
            if (pressed) {
                keyboard_modifiers ^= KB_MODIFIER_NUM;
                keyboard_set_leds(keyboard_leds ^ 0x02);  // Toggle num lock LED
            }
            break;
        case KB_SCANCODE_SCROLLLOCK:
            if (pressed) {
                keyboard_modifiers ^= KB_MODIFIER_SCROLL;
                keyboard_set_leds(keyboard_leds ^ 0x01);  // Toggle scroll lock LED
            }
            break;
    }
    
    // Add to buffer
    if (keyboard_buffer_count < KEYBOARD_BUFFER_SIZE) {
        keyboard_buffer[keyboard_buffer_tail] = scancode;
        keyboard_buffer_tail = (keyboard_buffer_tail + 1) % KEYBOARD_BUFFER_SIZE;
        keyboard_buffer_count++;
    }
    
    // Call callback if registered
    if (keyboard_callback) {
        keyboard_callback(code, keyboard_modifiers, pressed);
    }
    
    // Send EOI to PIC
    outb(0x20, 0x20);
}

// -----------------------------------------------------------------------------
// Set keyboard callback
// -----------------------------------------------------------------------------

void keyboard_set_handler(keyboard_handler_t handler) {
    keyboard_callback = handler;
}

// -----------------------------------------------------------------------------
// Enable keyboard
// -----------------------------------------------------------------------------

void keyboard_enable() {
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_ENABLE);
}

// -----------------------------------------------------------------------------
// Disable keyboard
// -----------------------------------------------------------------------------

void keyboard_disable() {
    keyboard_wait();
    outb(KEYBOARD_COMMAND_PORT, KB_CMD_DISABLE);
}

// -----------------------------------------------------------------------------
// Get modifiers
// -----------------------------------------------------------------------------

uint8_t keyboard_get_modifiers() {
    return keyboard_modifiers;
}

// -----------------------------------------------------------------------------
// Check if key is pressed
// -----------------------------------------------------------------------------

bool keyboard_is_key_pressed(uint8_t scancode) {
    return key_pressed[scancode & 0x7F];
}

// -----------------------------------------------------------------------------
// Check if key is released
// -----------------------------------------------------------------------------

bool keyboard_is_key_released(uint8_t scancode) {
    return !key_pressed[scancode & 0x7F];
}

// -----------------------------------------------------------------------------
// Read scan code (blocking)
// -----------------------------------------------------------------------------

uint8_t keyboard_read_scancode() {
    while (keyboard_buffer_count == 0) {
        // Wait for data
        __asm__ __volatile__("pause");
    }
    
    uint8_t scancode = keyboard_buffer[keyboard_buffer_head];
    keyboard_buffer_head = (keyboard_buffer_head + 1) % KEYBOARD_BUFFER_SIZE;
    keyboard_buffer_count--;
    
    return scancode;
}

// -----------------------------------------------------------------------------
// Check if keyboard has data
// -----------------------------------------------------------------------------

bool keyboard_has_data() {
    return keyboard_buffer_count > 0;
}

// -----------------------------------------------------------------------------
// Convert scan code to ASCII
// -----------------------------------------------------------------------------

char keyboard_scancode_to_ascii(uint8_t scancode, uint8_t modifiers) {
    // Handle shift
    bool shifted = (modifiers & KB_MODIFIER_SHIFT) || (modifiers & KB_MODIFIER_CAPS);
    
    switch (scancode) {
        case KB_SCANCODE_ESCAPE: return '\x1B';
        case KB_SCANCODE_1: return shifted ? '!' : '1';
        case KB_SCANCODE_2: return shifted ? '@' : '2';
        case KB_SCANCODE_3: return shifted ? '#' : '3';
        case KB_SCANCODE_4: return shifted ? '$' : '4';
        case KB_SCANCODE_5: return shifted ? '%' : '5';
        case KB_SCANCODE_6: return shifted ? '^' : '6';
        case KB_SCANCODE_7: return shifted ? '&' : '7';
        case KB_SCANCODE_8: return shifted ? '*' : '8';
        case KB_SCANCODE_9: return shifted ? '(' : '9';
        case KB_SCANCODE_0: return shifted ? ')' : '0';
        case KB_SCANCODE_MINUS: return shifted ? '_' : '-';
        case KB_SCANCODE_EQUALS: return shifted ? '+' : '=';
        case KB_SCANCODE_BACKSPACE: return '\b';
        case KB_SCANCODE_TAB: return '\t';
        case KB_SCANCODE_Q: return shifted ? 'Q' : 'q';
        case KB_SCANCODE_W: return shifted ? 'W' : 'w';
        case KB_SCANCODE_E: return shifted ? 'E' : 'e';
        case KB_SCANCODE_R: return shifted ? 'R' : 'r';
        case KB_SCANCODE_T: return shifted ? 'T' : 't';
        case KB_SCANCODE_Y: return shifted ? 'Y' : 'y';
        case KB_SCANCODE_U: return shifted ? 'U' : 'u';
        case KB_SCANCODE_I: return shifted ? 'I' : 'i';
        case KB_SCANCODE_O: return shifted ? 'O' : 'o';
        case KB_SCANCODE_P: return shifted ? 'P' : 'p';
        case KB_SCANCODE_LBRACKET: return shifted ? '{' : '[';
        case KB_SCANCODE_RBRACKET: return shifted ? '}' : ']';
        case KB_SCANCODE_ENTER: return '\n';
        case KB_SCANCODE_A: return shifted ? 'A' : 'a';
        case KB_SCANCODE_S: return shifted ? 'S' : 's';
        case KB_SCANCODE_D: return shifted ? 'D' : 'd';
        case KB_SCANCODE_F: return shifted ? 'F' : 'f';
        case KB_SCANCODE_G: return shifted ? 'G' : 'g';
        case KB_SCANCODE_H: return shifted ? 'H' : 'h';
        case KB_SCANCODE_J: return shifted ? 'J' : 'j';
        case KB_SCANCODE_K: return shifted ? 'K' : 'k';
        case KB_SCANCODE_L: return shifted ? 'L' : 'l';
        case KB_SCANCODE_SEMICOLON: return shifted ? ':' : ';';
        case KB_SCANCODE_APOSTROPHE: return shifted ? '"' : '\'';
        case KB_SCANCODE_GRAVE: return shifted ? '~' : '`';
        case KB_SCANCODE_BACKSLASH: return shifted ? '|' : '\\';
        case KB_SCANCODE_Z: return shifted ? 'Z' : 'z';
        case KB_SCANCODE_X: return shifted ? 'X' : 'x';
        case KB_SCANCODE_C: return shifted ? 'C' : 'c';
        case KB_SCANCODE_V: return shifted ? 'V' : 'v';
        case KB_SCANCODE_B: return shifted ? 'B' : 'b';
        case KB_SCANCODE_N: return shifted ? 'N' : 'n';
        case KB_SCANCODE_M: return shifted ? 'M' : 'm';
        case KB_SCANCODE_COMMA: return shifted ? '<' : ',';
        case KB_SCANCODE_DOT: return shifted ? '>' : '.';
        case KB_SCANCODE_SLASH: return shifted ? '?' : '/';
        case KB_SCANCODE_SPACE: return ' ';
        default: return 0;
    }
}

// -----------------------------------------------------------------------------
// Initialize keyboard for use with kernel
// -----------------------------------------------------------------------------

void init_keyboard() {
    keyboard_init();
    
    // Register IRQ handler
    register_interrupt_handler(IRQ1, (interrupt_handler_t)keyboard_irq_handler);
}
