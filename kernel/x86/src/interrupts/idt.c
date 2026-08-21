// NebulaOS - x86 IDT Implementation
// ===================================
//
// Interrupt Descriptor Table implementation for x86

#include "../../common/include/idt.h"
#include "../../common/include/nebula.h"
#include "../../common/include/gdt.h"
#include "../../common/include/io.h"

// IDT table
static idt_entry_t idt[IDT_MAX_DESCRIPTORS];

// IDT pointer
static idt_ptr_t idt_ptr;

// Interrupt handler array
static interrupt_handler_t interrupt_handlers[IDT_MAX_DESCRIPTORS];

// -----------------------------------------------------------------------------
// Initialize IDT
// -----------------------------------------------------------------------------
void init_idt(void) {
    // Set up IDT pointer
    idt_ptr.limit = sizeof(idt) - 1;
    idt_ptr.base = (uint32_t)&idt;
    
    // Initialize all handlers to default
    for (int i = 0; i < IDT_MAX_DESCRIPTORS; i++) {
        interrupt_handlers[i] = default_interrupt_handler;
    }
    
    // Set up exception handlers (ISRs)
    idt_set_gate(0, (uint32_t)isr0, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(1, (uint32_t)isr1, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(2, (uint32_t)isr2, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(3, (uint32_t)isr3, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(4, (uint32_t)isr4, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(5, (uint32_t)isr5, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(6, (uint32_t)isr6, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(7, (uint32_t)isr7, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(8, (uint32_t)isr8, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(9, (uint32_t)isr9, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(10, (uint32_t)isr10, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(11, (uint32_t)isr11, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(12, (uint32_t)isr12, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(13, (uint32_t)isr13, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(14, (uint32_t)isr14, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(15, (uint32_t)isr15, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(16, (uint32_t)isr16, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(17, (uint32_t)isr17, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(18, (uint32_t)isr18, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(19, (uint32_t)isr19, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(20, (uint32_t)isr20, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(21, (uint32_t)isr21, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(22, (uint32_t)isr22, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(23, (uint32_t)isr23, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(24, (uint32_t)isr24, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(25, (uint32_t)isr25, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(26, (uint32_t)isr26, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(27, (uint32_t)isr27, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(28, (uint32_t)isr28, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(29, (uint32_t)isr29, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(30, (uint32_t)isr30, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(31, (uint32_t)isr31, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    
    // Set up IRQ handlers
    idt_set_gate(IRQ0, (uint32_t)irq0, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ1, (uint32_t)irq1, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ2, (uint32_t)irq2, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ3, (uint32_t)irq3, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ4, (uint32_t)irq4, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ5, (uint32_t)irq5, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ6, (uint32_t)irq6, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ7, (uint32_t)irq7, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ8, (uint32_t)irq8, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ9, (uint32_t)irq9, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ10, (uint32_t)irq10, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ11, (uint32_t)irq11, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ12, (uint32_t)irq12, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ13, (uint32_t)irq13, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ14, (uint32_t)irq14, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    idt_set_gate(IRQ15, (uint32_t)irq15, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    
    // Fill remaining entries with default handler
    for (int i = 32; i < IDT_MAX_DESCRIPTORS; i++) {
        idt_set_gate(i, (uint32_t)isr0, GDT_CODE_SEL, IDT_FLAG_PRESENT | IDT_FLAG_32BIT | IDT_FLAG_INTERRUPT);
    }
    
    // Flush IDT
    idt_flush();
}

// -----------------------------------------------------------------------------
// Set IDT gate
// -----------------------------------------------------------------------------
void idt_set_gate(uint8_t num, uint32_t base, uint16_t sel, uint8_t flags) {
    idt[num].base_low = (uint16_t)(base & 0xFFFF);
    idt[num].base_high = (uint16_t)((base >> 16) & 0xFFFF);
    idt[num].sel = sel;
    idt[num].zero = 0;
    idt[num].flags = flags;
}

// -----------------------------------------------------------------------------
// Flush IDT (reload IDTR)
// -----------------------------------------------------------------------------
void idt_flush(void) {
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
    // In a real implementation, this would log the interrupt
    // and call kernel_panic for unhandled interrupts
    
    // For exceptions, print error message
    if (regs->int_no < 32) {
        // Exception occurred
        // console_printf("Exception %d occurred!\n", regs->int_no);
        // kernel_panic("Unhandled exception");
    } else {
        // IRQ occurred
        // console_printf("Unhandled IRQ %d\n", regs->int_no - 32);
    }
    
    // Send EOI to PIC if it's an IRQ
    if (regs->int_no >= IRQ0 && regs->int_no <= IRQ15) {
        outb(0x20, 0x20);  // Master PIC
        if (regs->int_no >= IRQ8) {
            outb(0xA0, 0x20);  // Slave PIC
        }
    }
}
