// NebulaOS - PS/2 Mouse Driver
// =============================
//
// Implementation of PS/2 mouse driver

#include "../include/mouse.h"
#include "../include/keyboard.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/idt.h"

// Mouse state
static mouse_state_t mouse_state = {0, 0, 0, 0, false, false, false};
static bool mouse_initialized = false;
static mouse_handler_t mouse_callback = NULL;

// Mouse packet buffer
#define MOUSE_BUFFER_SIZE 64
static mouse_packet_t mouse_buffer[MOUSE_BUFFER_SIZE];
static uint32_t mouse_buffer_head = 0;
static uint32_t mouse_buffer_tail = 0;
static uint32_t mouse_buffer_count = 0;

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

static void mouse_wait() {
    // Wait for input buffer to be empty
    while (inb(MOUSE_STATUS_PORT) & KB_STATUS_IBF) {
        // Wait
    }
}

// -----------------------------------------------------------------------------
// Send command to mouse
// -----------------------------------------------------------------------------

static bool mouse_send_command(uint8_t command) {
    // Wait for controller to be ready
    mouse_wait();
    
    // Send command to keyboard controller
    outb(MOUSE_COMMAND_PORT, 0xD4);  // Send to mouse
    
    // Wait for controller to be ready
    mouse_wait();
    
    // Send the actual command
    outb(MOUSE_DATA_PORT, command);
    
    // Wait for acknowledgment
    mouse_wait();
    uint8_t ack = inb(MOUSE_DATA_PORT);
    
    // Acknowledge should be 0xFA
    return ack == 0xFA;
}

// -----------------------------------------------------------------------------
// Initialize mouse
// -----------------------------------------------------------------------------

void mouse_init() {
    if (mouse_initialized) return;
    
    // Clear mouse buffer
    mouse_buffer_head = 0;
    mouse_buffer_tail = 0;
    mouse_buffer_count = 0;
    
    // Clear mouse state
    mouse_state.x = 0;
    mouse_state.y = 0;
    mouse_state.buttons = 0;
    mouse_state.wheel = 0;
    mouse_state.leftButton = false;
    mouse_state.rightButton = false;
    mouse_state.middleButton = false;
    
    // Disable mouse first
    mouse_send_command(MOUSE_CMD_DISABLE_DATA);
    
    // Reset mouse
    mouse_send_command(MOUSE_CMD_RESET);
    mouse_wait();
    uint8_t status = inb(MOUSE_DATA_PORT);
    if (status != 0xAA) {
        // Reset failed
        return;
    }
    
    // Wait for acknowledgment
    mouse_wait();
    inb(MOUSE_DATA_PORT);  // Self-test result
    mouse_wait();
    inb(MOUSE_DATA_PORT);  // Mouse ID
    
    // Set default mode
    mouse_send_command(MOUSE_CMD_SET_DEFAULT);
    
    // Enable data reporting
    mouse_send_command(MOUSE_CMD_ENABLE_DATA);
    
    // Set sample rate
    mouse_send_command(MOUSE_CMD_SET_SAMPLE_RATE);
    mouse_send_command(100);  // 100 samples per second
    
    // Set resolution
    mouse_send_command(MOUSE_CMD_SET_RESOLUTION);
    mouse_send_command(0x00);  // 1 count per mm
    
    // Set scaling
    mouse_send_command(MOUSE_CMD_SET_SCALING_1_1);
    
    mouse_initialized = true;
}

// -----------------------------------------------------------------------------
// Mouse IRQ handler (IRQ12)
// -----------------------------------------------------------------------------

static bool mouse_packet_ready = false;
static uint8_t mouse_packet_bytes[3];
static uint8_t mouse_packet_index = 0;

void mouse_irq_handler() {
    // Read data from mouse
    uint8_t data = inb(MOUSE_DATA_PORT);
    
    // Check if this is a mouse packet
    // Mouse packets have bit 3 set (always 1)
    if (!(data & 0x08)) {
        // Not a mouse packet, maybe keyboard data
        // Send EOI and return
        outb(0x20, 0x20);
        outb(0xA0, 0x20);
        return;
    }
    
    // Store packet byte
    switch (mouse_packet_index) {
        case 0:
            // First byte must have bit 3 set
            if (data & 0x08) {
                mouse_packet_bytes[0] = data;
                mouse_packet_index = 1;
            }
            break;
        case 1:
            mouse_packet_bytes[1] = data;
            mouse_packet_index = 2;
            break;
        case 2:
            mouse_packet_bytes[2] = data;
            mouse_packet_index = 0;
            mouse_packet_ready = true;
            
            // Process the packet
            mouse_packet_t packet;
            packet.buttons = mouse_packet_bytes[0] & 0x07;
            packet.dx = mouse_packet_bytes[1];
            packet.dy = mouse_packet_bytes[2];
            
            // Handle X and Y signs
            if (mouse_packet_bytes[0] & 0x10) {
                packet.dx |= 0xFF00;  // Sign extend
            }
            if (mouse_packet_bytes[0] & 0x20) {
                packet.dy |= 0xFF00;  // Sign extend
            }
            
            // Update mouse state
            mouse_state.buttons = packet.buttons;
            mouse_state.leftButton = (packet.buttons & 0x01) != 0;
            mouse_state.rightButton = (packet.buttons & 0x02) != 0;
            mouse_state.middleButton = (packet.buttons & 0x04) != 0;
            
            mouse_state.x += packet.dx;
            mouse_state.y -= packet.dy;  // Y is inverted in PS/2
            
            // Clamp to screen bounds (would be set by display size)
            if (mouse_state.x < 0) mouse_state.x = 0;
            if (mouse_state.y < 0) mouse_state.y = 0;
            // In a real implementation, clamp to screen width/height
            
            // Add to buffer
            if (mouse_buffer_count < MOUSE_BUFFER_SIZE) {
                mouse_buffer[mouse_buffer_tail] = packet;
                mouse_buffer_tail = (mouse_buffer_tail + 1) % MOUSE_BUFFER_SIZE;
                mouse_buffer_count++;
            }
            
            // Call callback if registered
            if (mouse_callback) {
                mouse_callback(mouse_state.x, mouse_state.y, mouse_state.buttons, mouse_state.wheel);
            }
            
            break;
    }
    
    // Send EOI to both PICs (IRQ12 is on slave PIC)
    outb(0x20, 0x20);  // Master PIC
    outb(0xA0, 0x20);  // Slave PIC
}

// -----------------------------------------------------------------------------
// Set mouse callback
// -----------------------------------------------------------------------------

void mouse_set_handler(mouse_handler_t handler) {
    mouse_callback = handler;
}

// -----------------------------------------------------------------------------
// Enable mouse
// -----------------------------------------------------------------------------

void mouse_enable() {
    mouse_send_command(MOUSE_CMD_ENABLE_DATA);
}

// -----------------------------------------------------------------------------
// Disable mouse
// -----------------------------------------------------------------------------

void mouse_disable() {
    mouse_send_command(MOUSE_CMD_DISABLE_DATA);
}

// -----------------------------------------------------------------------------
// Get mouse state
// -----------------------------------------------------------------------------

mouse_state_t mouse_get_state() {
    return mouse_state;
}

// -----------------------------------------------------------------------------
// Get mouse position
// -----------------------------------------------------------------------------

void mouse_get_position(int32_t* x, int32_t* y) {
    if (x) *x = mouse_state.x;
    if (y) *y = mouse_state.y;
}

// -----------------------------------------------------------------------------
// Set mouse position
// -----------------------------------------------------------------------------

void mouse_set_position(int32_t x, int32_t y) {
    mouse_state.x = x;
    mouse_state.y = y;
}

// -----------------------------------------------------------------------------
// Get mouse buttons
// -----------------------------------------------------------------------------

uint8_t mouse_get_buttons() {
    return mouse_state.buttons;
}

// -----------------------------------------------------------------------------
// Check if mouse button is pressed
// -----------------------------------------------------------------------------

bool mouse_is_button_pressed(uint8_t button) {
    return (mouse_state.buttons & button) != 0;
}

// -----------------------------------------------------------------------------
// Read mouse packet
// -----------------------------------------------------------------------------

bool mouse_read_packet(mouse_packet_t* packet) {
    if (mouse_buffer_count == 0) {
        return false;
    }
    
    *packet = mouse_buffer[mouse_buffer_head];
    mouse_buffer_head = (mouse_buffer_head + 1) % MOUSE_BUFFER_SIZE;
    mouse_buffer_count--;
    
    return true;
}

// -----------------------------------------------------------------------------
// Check if mouse has data
// -----------------------------------------------------------------------------

bool mouse_has_data() {
    return mouse_buffer_count > 0;
}

// -----------------------------------------------------------------------------
// Initialize mouse for use with kernel
// -----------------------------------------------------------------------------

void init_mouse() {
    mouse_init();
    
    // Register IRQ handler
    register_interrupt_handler(IRQ12, (interrupt_handler_t)mouse_irq_handler);
}
