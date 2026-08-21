// NebulaOS - Serial/COM Driver
// =============================
//
// Serial port driver for console output and input

#ifndef NEBULAOS_DRIVERS_SERIAL_H
#define NEBULAOS_DRIVERS_SERIAL_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// Serial ports
#define SERIAL_COM1_PORT  0x3F8
#define SERIAL_COM2_PORT  0x2F8
#define SERIAL_COM3_PORT  0x3E8
#define SERIAL_COM4_PORT  0x2E8

// Serial IRQ lines
#define SERIAL_COM1_IRQ  4
#define SERIAL_COM2_IRQ  3
#define SERIAL_COM3_IRQ  4
#define SERIAL_COM4_IRQ  3

// Serial registers (offset from base port)
#define SERIAL_DATA_BUFFER   0x00
#define SERIAL_INT_ENABLE    0x01
#define SERIAL_INT_ID        0x02
#define SERIAL_FIFO_CTRL     0x02
#define SERIAL_LINE_CTRL     0x03
#define SERIAL_MODEM_CTRL    0x04
#define SERIAL_LINE_STATUS   0x05
#define SERIAL_MODEM_STATUS  0x06
#define SERIAL_SCRATCH       0x07

// Line status bits
#define SERIAL_LINE_STATUS_DATA_READY  0x01
#define SERIAL_LINE_STATUS_OVERRUN     0x02
#define SERIAL_LINE_STATUS_PARITY      0x04
#define SERIAL_LINE_STATUS_FRAMING     0x08
#define SERIAL_LINE_STATUS_BREAK       0x10
#define SERIAL_LINE_STATUS_THR_EMPTY   0x20
#define SERIAL_LINE_STATUS_TSR_EMPTY   0x40
#define SERIAL_LINE_STATUS_ERROR       0x80

// FIFO control bits
#define SERIAL_FIFO_ENABLE      0x01
#define SERIAL_FIFO_CLEAR_RX    0x02
#define SERIAL_FIFO_CLEAR_TX    0x04
#define SERIAL_FIFO_DMA_MODE    0x08
#define SERIAL_FIFO_RESERVED    0xF0

// Line control bits
#define SERIAL_LINE_5BIT   0x00
#define SERIAL_LINE_6BIT   0x01
#define SERIAL_LINE_7BIT   0x02
#define SERIAL_LINE_8BIT   0x03
#define SERIAL_LINE_1STOP  0x00
#define SERIAL_LINE_2STOP  0x04
#define SERIAL_LINE_PARITY  0x08
#define SERIAL_LINE_DLAB    0x80

// Modem control bits
#define SERIAL_MODEM_DTR   0x01
#define SERIAL_MODEM_RTS   0x02
#define SERIAL_MODEM_OUT1  0x04
#define SERIAL_MODEM_OUT2  0x08
#define SERIAL_MODEM_LOOP  0x10

// Baud rates (divisor for 115200 base clock)
#define SERIAL_BAUD_115200 1
#define SERIAL_BAUD_57600  2
#define SERIAL_BAUD_38400  3
#define SERIAL_BAUD_19200  6
#define SERIAL_BAUD_9600   12
#define SERIAL_BAUD_4800   24
#define SERIAL_BAUD_2400   48
#define SERIAL_BAUD_1200   96

// Serial state
typedef struct {
    bool initialized;
    uint16_t port;
    uint32_t baud_rate;
} serial_state_t;

// Serial driver functions
void serial_init(uint16_t port, uint32_t baud_rate);
void serial_putchar(char c);
void serial_puts(const char* str);
char serial_getchar(void);
bool serial_available(void);

// Utility functions
void serial_set_baud_rate(uint16_t port, uint32_t baud_rate);
void serial_set_line_config(uint16_t port, uint8_t config);

// Get serial state
serial_state_t* serial_get_state(void);

#endif // NEBULAOS_DRIVERS_SERIAL_H
