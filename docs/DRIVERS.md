# NebulaOS Driver Documentation
# ==============================
#
# This document describes the driver architecture and available drivers.

## Driver Architecture

NebulaOS uses an interrupt-driven driver model. Drivers register handler functions with the IDT, and the kernel dispatches interrupts to the appropriate handler.

### Driver Registration

```c
// Register an interrupt handler
void register_interrupt_handler(uint8_t n, interrupt_handler_t handler);
```

### Driver Lifecycle

1. **Initialization**: Driver sets up hardware state
2. **Registration**: Driver registers interrupt handlers
3. **Operation**: Handlers called on interrupts
4. **Shutdown**: Driver cleans up (not yet implemented)

## Available Drivers

### PIC (Programmable Interrupt Controller)

**File**: `drivers/src/pic.c`
**Header**: `drivers/include/pic.h`

The 8259 PIC driver manages hardware interrupt routing.

**Functions**:
- `pic_init(master_offset, slave_offset)` - Initialize and remap PIC
- `pic_remap(master_offset, slave_offset)` - Remap interrupt vectors
- `pic_enable_irq(irq)` - Enable specific IRQ
- `pic_disable_irq(irq)` - Disable specific IRQ
- `pic_send_eoi(irq)` - Send End of Interrupt signal

**Constants**:
- `PIC1_COMMAND_PORT` = 0x20
- `PIC1_DATA_PORT` = 0x21
- `PIC2_COMMAND_PORT` = 0xA0
- `PIC2_DATA_PORT` = 0xA1
- `PIC1_IRQ_BASE` = 0x20 (32)
- `PIC2_IRQ_BASE` = 0x28 (40)

### PIT (Programmable Interval Timer)

**File**: `drivers/src/pit.c`
**Header**: `drivers/include/pit.h`

The 8253/8254 PIT provides system timing.

**Functions**:
- `pit_init(frequency)` - Initialize PIT at specified frequency
- `pit_set_frequency(channel, frequency)` - Set channel frequency
- `pit_get_frequency(channel)` - Get channel frequency
- `timer_init()` - Initialize timer subsystem
- `timer_get_ticks()` - Get tick count
- `timer_get_milliseconds()` - Get elapsed milliseconds
- `sleep(milliseconds)` - Sleep for specified time
- `usleep(microseconds)` - Sleep for specified time

**Constants**:
- `PIT_CHANNEL0_PORT` = 0x40
- `PIT_BASE_FREQ` = 1193182 Hz
- `TIMER_FREQ` = 100 Hz (default)

### Keyboard (PS/2)

**File**: `drivers/src/keyboard/keyboard.c`
**Header**: `drivers/include/keyboard.h`

PS/2 keyboard driver with scancode processing.

**Functions**:
- `keyboard_init()` - Initialize keyboard controller
- `keyboard_enable()` - Enable keyboard
- `keyboard_disable()` - Disable keyboard
- `keyboard_set_handler(handler)` - Set keyboard event callback
- `keyboard_get_modifiers()` - Get current modifier state
- `keyboard_read_scancode()` - Read scancode (blocking)
- `keyboard_has_data()` - Check if data available
- `keyboard_scancode_to_ascii(scancode, modifiers)` - Convert to ASCII

**Constants**:
- `KEYBOARD_DATA_PORT` = 0x60
- `KEYBOARD_STATUS_PORT` = 0x64
- `KB_SCANCODE_*` - Scancode definitions

### Mouse (PS/2)

**File**: `drivers/src/mouse/mouse.c`
**Header**: `drivers/include/mouse.h`

PS/2 mouse driver (stub implementation).

### VGA (Video Graphics Array)

**File**: `kernel/common/src/vga.c`
**Header**: `kernel/common/include/vga.h`

VGA text mode driver for console output.

**Functions**:
- `vga_init()` - Initialize VGA text mode
- `vga_clear()` - Clear screen
- `vga_putchar(c)` - Write character
- `vga_puts(str)` - Write string
- `vga_set_color(color)` - Set text color
- `vga_set_bg_color(bg)` - Set background color
- `vga_set_cursor(x, y)` - Move cursor
- `vga_enable_cursor(start, end)` - Show cursor
- `vga_disable_cursor()` - Hide cursor

## Writing a New Driver

1. Create header in `drivers/include/`
2. Create implementation in `drivers/src/`
3. Add to `DRIVER_SOURCES` in Makefile
4. Register interrupt handler in init function
5. Send EOI in IRQ handler

### Example Driver Template

```c
// drivers/include/mydevice.h
#ifndef NEBULAOS_MYDEVICE_H
#define NEBULAOS_MYDEVICE_H

#include "../../kernel/common/include/stdint.h"

void mydevice_init(void);
void mydevice_irq_handler(void);

#endif

// drivers/src/mydevice/mydevice.c
#include "../include/mydevice.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/idt.h"

static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" : : "a"(value), "dN"(port));
}

void mydevice_init(void) {
    // Initialize hardware
    register_interrupt_handler(IRQn, mydevice_irq_handler);
}

void mydevice_irq_handler(void) {
    // Handle interrupt
    uint8_t data = inb(MYDEVICE_DATA_PORT);

    // Process data...

    // Send EOI
    outb(0x20, 0x20);
}
```

## Driver Status

| Driver | Status | Notes |
|--------|--------|-------|
| PIC | Working | 8259 PIC, master/slave |
| PIT | Working | 100Hz default |
| Keyboard | Working | PS/2, scancode to ASCII |
| Mouse | Stub | PS/2 mouse not implemented |
| VGA | Working | Text mode 80x25 |
