// NebulaOS - PIC (Programmable Interrupt Controller) Driver
// =========================================================
//
// Implementation of 8259 PIC driver

#include "../include/pic.h"
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
// Initialize PIC
// -----------------------------------------------------------------------------

void pic_init(uint8_t master_offset, uint8_t slave_offset) {
    pic_remap(master_offset, slave_offset);
    
    // Mask all interrupts initially
    outb(PIC1_DATA_PORT, 0xFF);
    outb(PIC2_DATA_PORT, 0xFF);
}

// -----------------------------------------------------------------------------
// Remap PIC interrupts
// -----------------------------------------------------------------------------

void pic_remap(uint8_t master_offset, uint8_t slave_offset) {
    // Save masks
    uint8_t mask1 = inb(PIC1_DATA_PORT);
    uint8_t mask2 = inb(PIC2_DATA_PORT);
    
    // Send ICW1 to master
    outb(PIC1_COMMAND_PORT, PIC_ICW1_INIT | PIC_ICW1_ICW4);
    
    // Send ICW2 to master (interrupt vector offset)
    outb(PIC1_DATA_PORT, master_offset);
    
    // Send ICW3 to master (slave connected to IRQ2)
    outb(PIC1_DATA_PORT, 0x04);  // Slave at IRQ2
    
    // Send ICW4 to master
    outb(PIC1_DATA_PORT, PIC_ICW4_8086);
    
    // Send ICW1 to slave
    outb(PIC2_COMMAND_PORT, PIC_ICW1_INIT | PIC_ICW1_ICW4);
    
    // Send ICW2 to slave (interrupt vector offset)
    outb(PIC2_DATA_PORT, slave_offset);
    
    // Send ICW3 to slave (slave ID = 2)
    outb(PIC2_DATA_PORT, 0x02);  // Slave ID = 2
    
    // Send ICW4 to slave
    outb(PIC2_DATA_PORT, PIC_ICW4_8086);
    
    // Restore masks
    outb(PIC1_DATA_PORT, mask1);
    outb(PIC2_DATA_PORT, mask2);
}

// -----------------------------------------------------------------------------
// Enable IRQ
// -----------------------------------------------------------------------------

void pic_enable_irq(uint8_t irq) {
    uint8_t port;
    uint8_t mask;
    
    if (irq < 8) {
        // Master PIC
        port = PIC1_DATA_PORT;
        mask = inb(port) & ~(1 << irq);
    } else {
        // Slave PIC
        port = PIC2_DATA_PORT;
        mask = inb(port) & ~(1 << (irq - 8));
    }
    
    outb(port, mask);
}

// -----------------------------------------------------------------------------
// Disable IRQ
// -----------------------------------------------------------------------------

void pic_disable_irq(uint8_t irq) {
    uint8_t port;
    uint8_t mask;
    
    if (irq < 8) {
        // Master PIC
        port = PIC1_DATA_PORT;
        mask = inb(port) | (1 << irq);
    } else {
        // Slave PIC
        port = PIC2_DATA_PORT;
        mask = inb(port) | (1 << (irq - 8));
    }
    
    outb(port, mask);
}

// -----------------------------------------------------------------------------
// Set IRQ mask
// -----------------------------------------------------------------------------

void pic_set_mask(uint8_t irq, bool masked) {
    if (masked) {
        pic_disable_irq(irq);
    } else {
        pic_enable_irq(irq);
    }
}

// -----------------------------------------------------------------------------
// Send EOI (End of Interrupt)
// -----------------------------------------------------------------------------

void pic_send_eoi(uint8_t irq) {
    if (irq >= 8) {
        // Slave PIC
        outb(PIC2_COMMAND_PORT, PIC_OCW2_EOI);
    }
    
    // Master PIC
    outb(PIC1_COMMAND_PORT, PIC_OCW2_EOI);
}

// -----------------------------------------------------------------------------
// Acknowledge interrupt
// -----------------------------------------------------------------------------

void pic_acknowledge(uint8_t irq) {
    pic_send_eoi(irq);
}

// -----------------------------------------------------------------------------
// Read IRR (Interrupt Request Register)
// -----------------------------------------------------------------------------

uint16_t pic_read_irr() {
    // OCW3: Read IRR
    outb(PIC1_COMMAND_PORT, PIC_OCW3_READ_IRR);
    uint8_t irr1 = inb(PIC1_COMMAND_PORT);
    
    outb(PIC2_COMMAND_PORT, PIC_OCW3_READ_IRR);
    uint8_t irr2 = inb(PIC2_COMMAND_PORT);
    
    return (irr2 << 8) | irr1;
}

// -----------------------------------------------------------------------------
// Read ISR (In-Service Register)
// -----------------------------------------------------------------------------

uint16_t pic_read_isr() {
    // OCW3: Read ISR
    outb(PIC1_COMMAND_PORT, PIC_OCW3_READ_ISR);
    uint8_t isr1 = inb(PIC1_COMMAND_PORT);
    
    outb(PIC2_COMMAND_PORT, PIC_OCW3_READ_ISR);
    uint8_t isr2 = inb(PIC2_COMMAND_PORT);
    
    return (isr2 << 8) | isr1;
}

// -----------------------------------------------------------------------------
// Check if IRQ is in service
// -----------------------------------------------------------------------------

bool pic_is_in_service(uint8_t irq) {
    uint16_t isr = pic_read_isr();
    
    if (irq < 8) {
        return (isr & (1 << irq)) != 0;
    } else {
        return (isr & (1 << (irq - 8 + 8))) != 0;
    }
}

// -----------------------------------------------------------------------------
// Check if IRQ is pending
// -----------------------------------------------------------------------------

bool pic_is_pending(uint8_t irq) {
    uint16_t irr = pic_read_irr();
    
    if (irq < 8) {
        return (irr & (1 << irq)) != 0;
    } else {
        return (irr & (1 << (irq - 8 + 8))) != 0;
    }
}
