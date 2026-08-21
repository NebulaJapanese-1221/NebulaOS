// NebulaOS - Interrupt Descriptor Table (IDT)
// ============================================
//
// IDT definitions for interrupt handling

#ifndef NEBULAOS_IDT_H
#define NEBULAOS_IDT_H

#include "nebula.h"
#include "stdint.h"

// IDT entry structure (8 bytes on x86, 16 bytes on x86_64)
#ifdef NEBULAOS_ARCH_X86

typedef struct PACKED {
    uint16_t base_low;    // Handler address bits 0-15
    uint16_t sel;         // Code segment selector
    uint8_t  zero;        // Unused (set to 0)
    uint8_t  flags;       // Flags
    uint16_t base_high;   // Handler address bits 16-31
} idt_entry_t;

#else // x86_64

typedef struct PACKED {
    uint16_t base_low;    // Handler address bits 0-15
    uint16_t sel;         // Code segment selector
    uint8_t  ist;         // Interrupt Stack Table index
    uint8_t  flags;       // Flags
    uint16_t base_mid;    // Handler address bits 16-31
    uint32_t base_high;   // Handler address bits 32-63
    uint32_t reserved;    // Reserved
} idt_entry_t;

#endif

// IDT pointer structure (same for both architectures)
typedef struct PACKED {
    uint16_t limit;
    uint32_t base;
#ifdef NEBULAOS_ARCH_X86_64
    uint32_t base_upper;
#endif
} idt_ptr_t;

// IDT flags
#define IDT_FLAG_PRESENT     0x80
#define IDT_FLAG_DPL_0       0x00  // Ring 0
#define IDT_FLAG_DPL_3       0x60  // Ring 3
#define IDT_FLAG_32BIT       0x08  // 32-bit gate (x86 only)
#define IDT_FLAG_16BIT       0x00  // 16-bit gate
#define IDT_FLAG_INTERRUPT   0x0E  // Interrupt gate
#define IDT_FLAG_TRAP        0x0F  // Trap gate

// Interrupt numbers
#define IRQ0    32  // PIT Timer
#define IRQ1    33  // Keyboard
#define IRQ2    34  // Cascade (from master to slave)
#define IRQ3    35  // Serial port 2
#define IRQ4    36  // Serial port 1
#define IRQ5    37  // Parallel port 2
#define IRQ6    38  // Floppy disk
#define IRQ7    39  // Parallel port 1
#define IRQ8    40  // Real-time clock
#define IRQ9    41  // ACPI (or legacy SCI)
#define IRQ10   42  // Reserved
#define IRQ11   43  // Reserved
#define IRQ12   44  // PS/2 Mouse
#define IRQ13   45  // Coprocessor
#define IRQ14   46  // Primary ATA
#define IRQ15   47  // Secondary ATA

// Exception numbers
#define EX_DE   0  // Divide by zero
#define EX_DB   1  // Debug
#define EX_NMI  2  // Non-maskable interrupt
#define EX_BP   3  // Breakpoint
#define EX_OF   4  // Overflow
#define EX_BR   5  // Bound range
#define EX_UD   6  // Invalid opcode
#define EX_NM   7  // Device not available
#define EX_DF   8  // Double fault
#define EX_CSO  9  // Coprocessor segment overrun
#define EX_TS   10 // Invalid TSS
#define EX_NP   11 // Segment not present
#define EX_SS   12 // Stack segment fault
#define EX_GP   13 // General protection fault
#define EX_PF   14 // Page fault
#define EX_MF   16 // x87 Floating-point exception
#define EX_AC   17 // Alignment check
#define EX_MC   18 // Machine check
#define EX_XM   19 // SIMD floating-point exception
#define EX_VE   20 // Virtualization exception

// Number of interrupts to support
#define IDT_MAX_DESCRIPTORS 256

// ISR function pointer type
typedef void (*isr_t)(void);

// Registers structure for interrupt handlers
typedef struct {
    uint32_t ds;
    uint32_t edi;
    uint32_t esi;
    uint32_t ebp;
    uint32_t esp;
    uint32_t ebx;
    uint32_t edx;
    uint32_t ecx;
    uint32_t eax;
    uint32_t int_no;
    uint32_t err_code;
    uint32_t eip;
    uint32_t cs;
    uint32_t eflags;
    uint32_t useresp;
    uint32_t ss;
#ifdef NEBULAOS_ARCH_X86_64
    uint64_t r8;
    uint64_t r9;
    uint64_t r10;
    uint64_t r11;
    uint64_t r12;
    uint64_t r13;
    uint64_t r14;
    uint64_t r15;
#endif
} registers_t;

// Function prototypes
void init_idt(void);
void init_idt64(void);
void idt_set_gate(uint8_t num, uint32_t base, uint16_t sel, uint8_t flags);
void idt_set_gate64(uint8_t num, uint64_t base, uint16_t sel, uint8_t ist, uint8_t flags);
void idt_flush(void);
void idt_flush64(void);

// ISR handlers
extern void isr0(void);
extern void isr1(void);
extern void isr2(void);
extern void isr3(void);
extern void isr4(void);
extern void isr5(void);
extern void isr6(void);
extern void isr7(void);
extern void isr8(void);
extern void isr9(void);
extern void isr10(void);
extern void isr11(void);
extern void isr12(void);
extern void isr13(void);
extern void isr14(void);
extern void isr15(void);
extern void isr16(void);
extern void isr17(void);
extern void isr18(void);
extern void isr19(void);
extern void isr20(void);
extern void isr21(void);
extern void isr22(void);
extern void isr23(void);
extern void isr24(void);
extern void isr25(void);
extern void isr26(void);
extern void isr27(void);
extern void isr28(void);
extern void isr29(void);
extern void isr30(void);
extern void isr31(void);

// IRQ handlers
extern void irq0(void);
extern void irq1(void);
extern void irq2(void);
extern void irq3(void);
extern void irq4(void);
extern void irq5(void);
extern void irq6(void);
extern void irq7(void);
extern void irq8(void);
extern void irq9(void);
extern void irq10(void);
extern void irq11(void);
extern void irq12(void);
extern void irq13(void);
extern void irq14(void);
extern void irq15(void);

// Interrupt handler function type
typedef void (*interrupt_handler_t)(registers_t*);

// Register interrupt handler
void register_interrupt_handler(uint8_t n, interrupt_handler_t handler);

// Default interrupt handler
void default_interrupt_handler(registers_t* regs);

#endif // NEBULAOS_IDT_H
