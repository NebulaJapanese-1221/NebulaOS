# NebulaOS Kernel Architecture
# =============================
#
# This document describes the architecture of the NebulaOS kernel.

## Overview

NebulaOS is a hobby x86/x86_64 operating system designed for learning and experimentation. It features:

- Dual architecture support (32-bit x86 and 64-bit x86_64)
- Monolithic kernel design
- VGA text mode console
- Interrupt-driven I/O
- Identity-mapped paging
- Simple shell interface

## Boot Process

### x86 (32-bit)

1. BIOS loads boot sector at 0x7C00
2. Bootloader loads stage 2 from disk
3. Stage 2 loads kernel and switches to protected mode
4. Kernel entry point (`kernel/x86/start.asm`)
5. C entry point (`kernel/x86/entry.c`)
6. `kernel_main()` initializes hardware and enters main loop

### x86_64 (64-bit)

1. GRUB loads kernel at 1MB
2. Bootloader switches to 64-bit long mode
3. Kernel entry point (`kernel/x86_64/start.asm`)
4. C entry point (`kernel/x86_64/entry.c`)
5. `kernel_main()` initializes hardware and enters main loop

## Memory Layout

### x86 (32-bit)

```
0x00000000 - 0x000003FF  Real-mode IVT
0x00000400 - 0x000004FF  BDA
0x00007C00 - 0x00007CFF  Boot sector
0x00010000 - 0x0001FFFF  Page directory/tables
0x00100000 - 0x01FFFFFF  Kernel code/data
0x01000000 - 0x04FFFFFF  Kernel heap
```

### x86_64 (64-bit)

```
0x0000000000000000 - 0x0000000000000FFF  Low memory (identity mapped)
0x0000000000010000 - 0x00000000001FFFFF  Kernel code/data
0x0000000001000000 - 0x0000000004FFFFFF  Kernel heap
```

## Subsystems

### GDT (Global Descriptor Table)

The GDT defines memory segments for the CPU. In 64-bit long mode:

- Segment 0x00: Null descriptor
- Segment 0x08: Kernel code (64-bit, ring 0)
- Segment 0x10: Kernel data (flat, ring 0)
- Segment 0x18: User code (64-bit, ring 3)
- Segment 0x20: User data (flat, ring 3)
- Segment 0x28: TSS (Task State Segment)

### IDT (Interrupt Descriptor Table)

The IDT maps interrupt numbers to handler functions:

- Interrupts 0-31: CPU exceptions
- Interrupts 32-47: Hardware IRQs (via PIC)
- Interrupts 48-255: Available for software interrupts

### Paging

x86_64 uses 4-level paging:

- PML4 (Page Map Level 4)
- PDPT (Page Directory Pointer Table)
- PDT (Page Directory)
- PT (Page Table)

Currently uses identity mapping (virtual = physical).

### Interrupts

- CPU exceptions handled by ISR stubs in assembly
- Hardware interrupts via 8259 PIC
- Timer (PIT) at IRQ0
- Keyboard at IRQ1

## Code Organization

```
kernel/
├── common/
│   ├── include/           # Shared headers
│   │   ├── nebula.h       # Core definitions
│   │   ├── gdt.h          # GDT structures
│   │   ├── idt.h          # IDT structures
│   │   ├── vga.h          # VGA driver
│   │   ├── memory.h       # Memory management
│   │   └── ...
│   └── src/               # Shared implementations
│       ├── vga.c
│       └── memory/
│           └── memory.c
├── x86/                   # 32-bit specific
│   ├── start.asm
│   ├── entry.c
│   ├── link.ld
│   └── src/
│       ├── device/gdt.c
│       ├── interrupts/idt.c
│       ├── interrupts/isr.asm
│       └── process/shell.c
└── x86_64/                # 64-bit specific
    ├── start.asm
    ├── entry.c
    ├── link.ld
    └── src/
        ├── device/gdt.c
        ├── device/stubs.c
        ├── interrupts/idt.c
        ├── interrupts/isr.asm
        ├── memory/paging.c
        └── process/shell.c
```

## Driver Architecture

Drivers are organized by function:

- `drivers/include/` - Driver headers
- `drivers/src/` - Driver implementations
- Each driver provides init, handler, and utility functions
- Drivers register interrupt handlers via `register_interrupt_handler()`

## Future Plans

- Proper multiboot2 support
- ACPI support for modern hardware
- SMP (multi-processor) support
- Virtual filesystem (VFS)
- ELF binary loading
- Process management
- System calls
