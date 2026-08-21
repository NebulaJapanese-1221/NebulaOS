// NebulaOS - PS/2 Keyboard Driver
// ================================
//
// PS/2 keyboard driver header

#ifndef NEBULAOS_DRIVERS_KEYBOARD_H
#define NEBULAOS_DRIVERS_KEYBOARD_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// Keyboard ports
#define KEYBOARD_DATA_PORT    0x60
#define KEYBOARD_STATUS_PORT  0x64
#define KEYBOARD_COMMAND_PORT 0x64

// Keyboard status bits
#define KB_STATUS_OBF     0x01  // Output buffer full
#define KB_STATUS_IBF     0x02  // Input buffer full
#define KB_STATUS_SELF    0x04  // Self test
#define KB_STATUS_CMD     0x08  // Command/data
#define KB_STATUS_LOCK    0x10  // Keyboard locked
#define KB_STATUS_AUXB    0x20  // Auxiliary output buffer full
#define KB_STATUS_TIMEOUT 0x40  // Timeout
#define KB_STATUS_PARITY  0x80  // Parity error

// Keyboard commands
#define KB_CMD_READ_MODE      0x20
#define KB_CMD_WRITE_MODE     0x60
#define KB_CMD_SELF_TEST      0xAA
#define KB_CMD_INTERFACE_TEST 0xAB
#define KB_CMD_DISABLE         0xAD
#define KB_CMD_ENABLE          0xAE
#define KB_CMD_READ_INPUT      0xC0
#define KB_CMD_READ_STATUS     0xD0
#define KB_CMD_READ_OUTPUT     0xD1
#define KB_CMD_WRITE_OUTPUT    0xD2
#define KB_CMD_WRITE_INPUT     0xD3
#define KB_CMD_WRITE_STATUS    0xD4

// Keyboard controller commands
#define KBC_CMD_READ_RAM       0x20
#define KBC_CMD_WRITE_RAM      0x60
#define KBC_CMD_SELF_TEST      0xAA
#define KBC_CMD_INTERFACE_TEST 0xAB
#define KBC_CMD_DISABLE_PORT2   0xA7
#define KBC_CMD_ENABLE_PORT2    0xA8
#define KBC_CMD_TEST_PORT2     0xA9
#define KBC_CMD_TEST_PORT1     0xAA
#define KBC_CMD_DISABLE_PORT1   0xAD
#define KBC_CMD_ENABLE_PORT1    0xAE
#define KBC_CMD_READ_VERSION   0xA1
#define KBC_CMD_PULSE_PORT2    0xD3
#define KBC_CMD_PULSE_PORT1    0xD4

// Scan codes
#define KB_SCANCODE_ESCAPE     0x01
#define KB_SCANCODE_1          0x02
#define KB_SCANCODE_2          0x03
#define KB_SCANCODE_3          0x04
#define KB_SCANCODE_4          0x05
#define KB_SCANCODE_5          0x06
#define KB_SCANCODE_6          0x07
#define KB_SCANCODE_7          0x08
#define KB_SCANCODE_8          0x09
#define KB_SCANCODE_9          0x0A
#define KB_SCANCODE_0          0x0B
#define KB_SCANCODE_MINUS      0x0C
#define KB_SCANCODE_EQUALS     0x0D
#define KB_SCANCODE_BACKSPACE  0x0E
#define KB_SCANCODE_TAB        0x0F
#define KB_SCANCODE_Q          0x10
#define KB_SCANCODE_W          0x11
#define KB_SCANCODE_E          0x12
#define KB_SCANCODE_R          0x13
#define KB_SCANCODE_T          0x14
#define KB_SCANCODE_Y          0x15
#define KB_SCANCODE_U          0x16
#define KB_SCANCODE_I          0x17
#define KB_SCANCODE_O          0x18
#define KB_SCANCODE_P          0x19
#define KB_SCANCODE_LBRACKET   0x1A
#define KB_SCANCODE_RBRACKET   0x1B
#define KB_SCANCODE_ENTER      0x1C
#define KB_SCANCODE_LCTRL      0x1D
#define KB_SCANCODE_RCTRL      0x1D  // Note: Right Ctrl uses same scancode with E0 prefix
#define KB_SCANCODE_A          0x1E
#define KB_SCANCODE_S          0x1F
#define KB_SCANCODE_D          0x20
#define KB_SCANCODE_F          0x21
#define KB_SCANCODE_G          0x22
#define KB_SCANCODE_H          0x23
#define KB_SCANCODE_J          0x24
#define KB_SCANCODE_K          0x25
#define KB_SCANCODE_L          0x26
#define KB_SCANCODE_SEMICOLON  0x27
#define KB_SCANCODE_APOSTROPHE 0x28
#define KB_SCANCODE_GRAVE      0x29
#define KB_SCANCODE_LSHIFT     0x2A
#define KB_SCANCODE_BACKSLASH  0x2B
#define KB_SCANCODE_Z          0x2C
#define KB_SCANCODE_X          0x2D
#define KB_SCANCODE_C          0x2E
#define KB_SCANCODE_V          0x2F
#define KB_SCANCODE_B          0x30
#define KB_SCANCODE_N          0x31
#define KB_SCANCODE_M          0x32
#define KB_SCANCODE_COMMA      0x33
#define KB_SCANCODE_DOT        0x34
#define KB_SCANCODE_SLASH      0x35
#define KB_SCANCODE_RSHIFT     0x36
#define KB_SCANCODE_KPASTERISK 0x37
#define KB_SCANCODE_LALT       0x38
#define KB_SCANCODE_RALT       0x38  // Note: Right Alt may use same scancode with E0 prefix
#define KB_SCANCODE_SPACE      0x39
#define KB_SCANCODE_CAPSLOCK   0x3A
#define KB_SCANCODE_F1         0x3B
#define KB_SCANCODE_F2         0x3C
#define KB_SCANCODE_F3         0x3D
#define KB_SCANCODE_F4         0x3E
#define KB_SCANCODE_F5         0x3F
#define KB_SCANCODE_F6         0x40
#define KB_SCANCODE_F7         0x41
#define KB_SCANCODE_F8         0x42
#define KB_SCANCODE_F9         0x43
#define KB_SCANCODE_F10        0x44
#define KB_SCANCODE_NUMLOCK    0x45
#define KB_SCANCODE_SCROLLLOCK 0x46
#define KB_SCANCODE_KP7        0x47
#define KB_SCANCODE_KP8        0x48
#define KB_SCANCODE_KP9        0x49
#define KB_SCANCODE_KPMINUS    0x4A
#define KB_SCANCODE_KP4        0x4B
#define KB_SCANCODE_KP5        0x4C
#define KB_SCANCODE_KP6        0x4D
#define KB_SCANCODE_KPPLUS     0x4E
#define KB_SCANCODE_KP1        0x4F
#define KB_SCANCODE_KP2        0x50
#define KB_SCANCODE_KP3        0x51
#define KB_SCANCODE_KP0        0x52
#define KB_SCANCODE_KPDOT      0x53
#define KB_SCANCODE_F11        0x57
#define KB_SCANCODE_F12        0x58

// Arrow keys (extended scancodes - prefixed with 0xE0)
#define KB_SCANCODE_UP         0x48
#define KB_SCANCODE_DOWN       0x50
#define KB_SCANCODE_LEFT       0x4B
#define KB_SCANCODE_RIGHT      0x4D

// Modifier keys
#define KB_MODIFIER_SHIFT   0x01
#define KB_MODIFIER_CTRL    0x02
#define KB_MODIFIER_ALT     0x04
#define KB_MODIFIER_WIN     0x08
#define KB_MODIFIER_CAPS    0x10
#define KB_MODIFIER_NUM     0x20
#define KB_MODIFIER_SCROLL  0x40

// Key states
#define KB_KEY_PRESSED     0x80
#define KB_KEY_RELEASED    0x00

// Keyboard event callback
typedef void (*keyboard_handler_t)(uint8_t scancode, uint8_t modifiers, bool pressed);

// Keyboard driver functions
void keyboard_init();
void keyboard_handler();
void keyboard_set_handler(keyboard_handler_t handler);
void keyboard_enable();
void keyboard_disable();
void keyboard_set_leds(uint8_t leds);

// Keyboard state
uint8_t keyboard_get_modifiers();
bool keyboard_is_key_pressed(uint8_t scancode);
bool keyboard_is_key_released(uint8_t scancode);

// Read scan code (blocking)
uint8_t keyboard_read_scancode();

// Check if keyboard has data
bool keyboard_has_data();

// Convert scancode to ASCII
char keyboard_scancode_to_ascii(uint8_t scancode, uint8_t modifiers);

#endif // NEBULAOS_DRIVERS_KEYBOARD_H
