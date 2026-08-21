// NebulaOS - x86_64 IDT Implementation
// ======================================
//
// Interrupt Descriptor Table implementation for x86_64

#include "../../../common/include/idt.h"
#include "../../../common/include/nebula.h"
#include "../../../common/include/io.h"

// IDT table
static idt_entry_t idt[IDT_MAX_DESCRIPTORS];

// IDT pointer
static idt_ptr_t idt_ptr;

// Interrupt handler array
static interrupt_handler_t interrupt_handlers[IDT_MAX_DESCRIPTORS];

// -----------------------------------------------------------------------------
// Initialize IDT for 64-bit
// -----------------------------------------------------------------------------
void idt64_init(void) {
    idt_ptr.limit = sizeof(idt) - 1;
    idt_ptr.base = (uint64_t)&idt;
    idt_ptr.base_upper = (uint32_t)((uint64_t)&idt >> 32);

    for (int i = 0; i < IDT_MAX_DESCRIPTORS; i++) {
        interrupt_handlers[i] = default_interrupt_handler;
    }

    // Set up exception handlers (ISRs 0-31)
    idt_set_gate64(0, (uint64_t)isr0, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(1, (uint64_t)isr1, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(2, (uint64_t)isr2, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(3, (uint64_t)isr3, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(4, (uint64_t)isr4, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(5, (uint64_t)isr5, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(6, (uint64_t)isr6, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(7, (uint64_t)isr7, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(8, (uint64_t)isr8, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(9, (uint64_t)isr9, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(10, (uint64_t)isr10, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(11, (uint64_t)isr11, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(12, (uint64_t)isr12, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(13, (uint64_t)isr13, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(14, (uint64_t)isr14, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(15, (uint64_t)isr15, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(16, (uint64_t)isr16, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(17, (uint64_t)isr17, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(18, (uint64_t)isr18, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(19, (uint64_t)isr19, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(20, (uint64_t)isr20, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(21, (uint64_t)isr21, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(22, (uint64_t)isr22, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(23, (uint64_t)isr23, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(24, (uint64_t)isr24, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(25, (uint64_t)isr25, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(26, (uint64_t)isr26, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(27, (uint64_t)isr27, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(28, (uint64_t)isr28, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(29, (uint64_t)isr29, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(30, (uint64_t)isr30, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(31, (uint64_t)isr31, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);

    // Set up IRQ handlers (IRQ 0-15 map to interrupts 32-47)
    idt_set_gate64(32, (uint64_t)irq0, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(33, (uint64_t)irq1, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(34, (uint64_t)irq2, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(35, (uint64_t)irq3, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(36, (uint64_t)irq4, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(37, (uint64_t)irq5, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(38, (uint64_t)irq6, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(39, (uint64_t)irq7, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(40, (uint64_t)irq8, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(41, (uint64_t)irq9, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(42, (uint64_t)irq10, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(43, (uint64_t)irq11, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(44, (uint64_t)irq12, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(45, (uint64_t)irq13, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(46, (uint64_t)irq14, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    idt_set_gate64(47, (uint64_t)irq15, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);

    // Fill remaining entries with default handler
    for (int i = 48; i < IDT_MAX_DESCRIPTORS; i++) {
        idt_set_gate64(i, (uint64_t)isr0, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }

    // Flush IDT
    idt_flush64();
}

// -----------------------------------------------------------------------------
// Set IDT gate (32-bit compatibility)
// -----------------------------------------------------------------------------
void idt_set_gate(uint8_t num, uint32_t base, uint16_t sel, uint8_t flags) {
    idt[num].base_low = (uint16_t)(base & 0xFFFF);
    idt[num].sel = sel;
    idt[num].ist = 0;
    idt[num].flags = flags;
    idt[num].base_mid = (uint16_t)((base >> 16) & 0xFFFF);
    idt[num].base_high = (uint32_t)(base >> 32);
    idt[num].reserved = 0;
}

// -----------------------------------------------------------------------------
// Set IDT gate (64-bit)
// -----------------------------------------------------------------------------
void idt_set_gate64(uint8_t num, uint64_t base, uint16_t sel, uint8_t ist, uint8_t flags) {
    idt[num].base_low = (uint16_t)(base & 0xFFFF);
    idt[num].sel = sel;
    idt[num].ist = ist;
    idt[num].flags = flags;
    idt[num].base_mid = (uint16_t)((base >> 16) & 0xFFFF);
    idt[num].base_high = (uint32_t)(base >> 32);
    idt[num].reserved = 0;
}

// -----------------------------------------------------------------------------
// Flush IDT (32-bit)
// -----------------------------------------------------------------------------
void idt_flush(void) {
    __asm__ __volatile__("lidt %0" : : "m" (idt_ptr));
}

// -----------------------------------------------------------------------------
// Flush IDT (64-bit)
// -----------------------------------------------------------------------------
void idt_flush64(void) {
    __asm__ __volatile__("lidt %0" : : "m" (idt_ptr));
}

// -----------------------------------------------------------------------------
// Register interrupt handler
// -----------------------------------------------------------------------------
void register_interrupt_handler(uint8_t n, interrupt_handler_t handler) {
    if (n < IDT_MAX_DESCRIPTORS) {
        interrupt_handlers[n] = handler;
    }
}

// -----------------------------------------------------------------------------
// Interrupt handler (called from assembly)
// -----------------------------------------------------------------------------
void interrupt_handler(registers_t* regs) {
    if (regs->int_no < IDT_MAX_DESCRIPTORS && interrupt_handlers[regs->int_no]) {
        interrupt_handlers[regs->int_no](regs);
    } else {
        default_interrupt_handler(regs);
    }
}

// -----------------------------------------------------------------------------
// Default interrupt handler
// -----------------------------------------------------------------------------
void default_interrupt_handler(registers_t* regs) {
    (void)regs;
    if (regs->int_no >= IRQ0 && regs->int_no <= IRQ15) {
        outb(0x20, 0x20);
        if (regs->int_no >= IRQ8) {
            outb(0xA0, 0x20);
        }
    }
}
