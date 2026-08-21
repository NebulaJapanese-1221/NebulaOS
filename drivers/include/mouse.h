// NebulaOS - PS/2 Mouse Driver
// ============================
//
// PS/2 mouse driver header

#ifndef NEBULAOS_DRIVERS_MOUSE_H
#define NEBULAOS_DRIVERS_MOUSE_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// Mouse ports
#define MOUSE_DATA_PORT    0x60
#define MOUSE_STATUS_PORT  0x64
#define MOUSE_COMMAND_PORT 0x64

// Mouse commands
#define MOUSE_CMD_RESET            0xFF
#define MOUSE_CMD_RESEND          0xFE
#define MOUSE_CMD_SET_DEFAULT     0xF6
#define MOUSE_CMD_DISABLE_DATA    0xF5
#define MOUSE_CMD_ENABLE_DATA     0xF4
#define MOUSE_CMD_SET_SAMPLE_RATE 0xF3
#define MOUSE_CMD_GET_DEVICE_ID   0xF2
#define MOUSE_CMD_SET_REMOTE      0xF0
#define MOUSE_CMD_READ_DATA       0xEB
#define MOUSE_CMD_SET_STREAM      0xEA
#define MOUSE_CMD_STATUS_REQUEST  0xE9
#define MOUSE_CMD_SET_RESOLUTION  0xE8
#define MOUSE_CMD_SET_SCALING_2_1 0xE7
#define MOUSE_CMD_SET_SCALING_1_1 0xE6

// Mouse status bits (from device)
#define MOUSE_STATUS_BUTTON1  0x01
#define MOUSE_STATUS_BUTTON2  0x02
#define MOUSE_STATUS_BUTTON3  0x04
#define MOUSE_STATUS_ALWAYS1  0x08
#define MOUSE_STATUS_X_SIGN   0x10
#define MOUSE_STATUS_Y_SIGN   0x20
#define MOUSE_STATUS_X_OVERFLOW 0x40
#define MOUSE_STATUS_Y_OVERFLOW 0x80

// Mouse packet structure
typedef struct PACKED {
    uint8_t buttons;
    int8_t dx;
    int8_t dy;
} mouse_packet_t;

// Mouse state structure
typedef struct {
    int32_t x;
    int32_t y;
    uint8_t buttons;
    int32_t wheel;
    bool leftButton : 1;
    bool rightButton : 1;
    bool middleButton : 1;
} mouse_state_t;

// Mouse event callback
typedef void (*mouse_handler_t)(int32_t x, int32_t y, uint8_t buttons, int32_t wheel);

// Mouse driver functions
void mouse_init();
void mouse_handler();
void mouse_set_handler(mouse_handler_t handler);
void mouse_enable();
void mouse_disable();

// Mouse state
mouse_state_t mouse_get_state();
void mouse_get_position(int32_t* x, int32_t* y);
void mouse_set_position(int32_t x, int32_t y);

uint8_t mouse_get_buttons();
bool mouse_is_button_pressed(uint8_t button);

// Read mouse packet
bool mouse_read_packet(mouse_packet_t* packet);

// Check if mouse has data
bool mouse_has_data();

#endif // NEBULAOS_DRIVERS_MOUSE_H
