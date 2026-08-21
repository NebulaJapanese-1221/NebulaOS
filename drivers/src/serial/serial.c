// NebulaOS - Serial/COM Driver
// =============================
//
// Serial port driver implementation (polling mode)

#include "../include/serial.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"

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
// Serial state
// -----------------------------------------------------------------------------

static serial_state_t serial_state;

// -----------------------------------------------------------------------------
// Wait for transmit buffer to be empty
// -----------------------------------------------------------------------------

static void serial_wait_transmit(uint16_t port) {
    while (!(inb(port + SERIAL_LINE_STATUS) & SERIAL_LINE_STATUS_THR_EMPTY)) {
    }
}

// -----------------------------------------------------------------------------
// Initialize serial port
// -----------------------------------------------------------------------------

void serial_init(uint16_t port, uint32_t baud_rate) {
    serial_state.initialized = true;
    serial_state.port = port;
    serial_state.baud_rate = baud_rate;

    // Set baud rate
    serial_set_baud_rate(port, baud_rate);

    // Set line configuration: 8 bits, no parity, 1 stop bit
    outb(port + SERIAL_LINE_CTRL, SERIAL_LINE_8BIT | SERIAL_LINE_1STOP);

    // Enable and clear FIFO
    outb(port + SERIAL_FIFO_CTRL,
         SERIAL_FIFO_ENABLE | SERIAL_FIFO_CLEAR_RX | SERIAL_FIFO_CLEAR_TX | SERIAL_FIFO_RESERVED);

    // Set modem control: DTR=1, RTS=1, OUT2=1 (for IRQ enable in some chips)
    outb(port + SERIAL_MODEM_CTRL, SERIAL_MODEM_DTR | SERIAL_MODEM_RTS | SERIAL_MODEM_OUT2);
}

// -----------------------------------------------------------------------------
// Set baud rate
// -----------------------------------------------------------------------------

void serial_set_baud_rate(uint16_t port, uint32_t baud_rate) {
    uint16_t divisor = 115200 / baud_rate;

    // Enable DLAB (Divisor Latch Access Bit)
    outb(port + SERIAL_LINE_CTRL, SERIAL_LINE_DLAB);

    // Set divisor (low byte then high byte)
    outb(port + SERIAL_DATA_BUFFER, divisor & 0xFF);
    outb(port + SERIAL_DATA_BUFFER, (divisor >> 8) & 0xFF);

    // Disable DLAB
    outb(port + SERIAL_LINE_CTRL, SERIAL_LINE_8BIT | SERIAL_LINE_1STOP);
}

// -----------------------------------------------------------------------------
// Set line configuration
// -----------------------------------------------------------------------------

void serial_set_line_config(uint16_t port, uint8_t config) {
    outb(port + SERIAL_LINE_CTRL, config);
}

// -----------------------------------------------------------------------------
// Put character (polling mode)
// -----------------------------------------------------------------------------

void serial_putchar(char c) {
    uint16_t port = serial_state.port;

    // Wait for transmit buffer to be empty
    serial_wait_transmit(port);

    // Output character
    outb(port + SERIAL_DATA_BUFFER, c);

    // Handle newline conversion
    if (c == '\n') {
        serial_wait_transmit(port);
        outb(port + SERIAL_DATA_BUFFER, '\r');
    }
}

// -----------------------------------------------------------------------------
// Put string
// -----------------------------------------------------------------------------

void serial_puts(const char* str) {
    if (!str) {
        return;
    }

    while (*str) {
        serial_putchar(*str++);
    }
}

// -----------------------------------------------------------------------------
// Get character (polling mode)
// -----------------------------------------------------------------------------

char serial_getchar(void) {
    uint16_t port = serial_state.port;

    // Wait for data to be available
    while (!(inb(port + SERIAL_LINE_STATUS) & SERIAL_LINE_STATUS_DATA_READY)) {
    }

    return inb(port + SERIAL_DATA_BUFFER);
}

// -----------------------------------------------------------------------------
// Check if character is available
// -----------------------------------------------------------------------------

bool serial_available(void) {
    uint16_t port = serial_state.port;
    return (inb(port + SERIAL_LINE_STATUS) & SERIAL_LINE_STATUS_DATA_READY) != 0;
}

// -----------------------------------------------------------------------------
// Get serial state
// -----------------------------------------------------------------------------

serial_state_t* serial_get_state(void) {
    return &serial_state;
}
