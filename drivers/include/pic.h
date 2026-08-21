// NebulaOS - PIC (Programmable Interrupt Controller) Driver
// =========================================================
//
// 8259 PIC driver header

#ifndef NEBULAOS_DRIVERS_PIC_H
#define NEBULAOS_DRIVERS_PIC_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// PIC ports
#define PIC1_COMMAND_PORT  0x20
#define PIC1_DATA_PORT     0x21
#define PIC2_COMMAND_PORT  0xA0
#define PIC2_DATA_PORT     0xA1

// PIC command register bits
#define PIC_CMD_ICW1        0x10  // Initialization Control Word 1
#define PIC_CMD_ICW2        0x11  // Initialization Control Word 2
#define PIC_CMD_ICW3        0x12  // Initialization Control Word 3
#define PIC_CMD_ICW4        0x13  // Initialization Control Word 4
#define PIC_CMD_OCW1        0x14  // Operation Control Word 1
#define PIC_CMD_OCW2        0x15  // Operation Control Word 2
#define PIC_CMD_OCW3        0x16  // Operation Control Word 3

// ICW1 bits
#define PIC_ICW1_ICW4      0x01  // ICW4 needed
#define PIC_ICW1_SINGLE    0x02  // Single (cascade) mode
#define PIC_ICW1_ADI       0x04  // Address interval
#define PIC_ICW1_LEVEL     0x08  // Level triggered mode
#define PIC_ICW1_INIT      0x10  // Initialization

// ICW2 bits (interrupt vector offset)
#define PIC_ICW2_OFFSET    0xF8  // Interrupt vector offset mask

// ICW3 bits (master/slave connections)
#define PIC_ICW3_SLAVE_MASK 0x07  // Slave PIC mask for master

// ICW4 bits
#define PIC_ICW4_8086      0x01  // 8086/88 mode
#define PIC_ICW4_AUTO_EOI  0x02  // Auto EOI
#define PIC_ICW4_BUF_SLAVE 0x04  // Buffered slave
#define PIC_ICW4_BUF_MASTER 0x08  // Buffered master
#define PIC_ICW4_SFNM      0x10  // Special fully nested mode

// OCW1 bits (interrupt mask)
#define PIC_OCW1_MASK(n)   (1 << (n))  // Mask for interrupt line n

// OCW2 bits
#define PIC_OCW2_EOI       0x20  // End of interrupt
#define PIC_OCW2_PRIORITY  0x40  // Priority selection
#define PIC_OCW2_READ_IRR  0x0A  // Read IRR
#define PIC_OCW2_READ_ISR  0x0B  // Read ISR

// OCW3 bits
#define PIC_OCW3_READ_IRR  0x08  // Read IRR
#define PIC_OCW3_READ_ISR  0x09  // Read ISR
#define PIC_OCW3_POLL      0x04  // Poll command

// IRQ numbers
#define PIC1_IRQ_BASE 0x20  // Master PIC interrupts start at 0x20 (32)
#define PIC2_IRQ_BASE 0x28  // Slave PIC interrupts start at 0x28 (40)

// PIC functions
void pic_init(uint8_t master_offset, uint8_t slave_offset);
void pic_remap(uint8_t master_offset, uint8_t slave_offset);
void pic_enable_irq(uint8_t irq);
void pic_disable_irq(uint8_t irq);
void pic_set_mask(uint8_t irq, bool masked);
void pic_send_eoi(uint8_t irq);

// IRQ handling
void pic_acknowledge(uint8_t irq);

// Read IRR and ISR
uint16_t pic_read_irr();
uint16_t pic_read_isr();

// Check if IRQ is in service
bool pic_is_in_service(uint8_t irq);

// Check if IRQ is pending
bool pic_is_pending(uint8_t irq);

// Serial IRQ remapping (optional)
// COM1/COM3 use IRQ4, COM2/COM4 use IRQ3
#define SERIAL_IRQ_COM1 4
#define SERIAL_IRQ_COM2 3
#define SERIAL_IRQ_COM3 4
#define SERIAL_IRQ_COM4 3

// Enable serial IRQ in PIC
static inline void pic_enable_serial_irq(uint16_t com_port) {
    uint8_t irq;
    switch ((int)com_port) {
        case (int)0x3F8: irq = SERIAL_IRQ_COM1; break;
        case (int)0x2F8: irq = SERIAL_IRQ_COM2; break;
        case (int)0x3E8: irq = SERIAL_IRQ_COM3; break;
        case (int)0x2E8: irq = SERIAL_IRQ_COM4; break;
        default: return;
    }
    pic_enable_irq(irq);
}

#endif // NEBULAOS_DRIVERS_PIC_H
