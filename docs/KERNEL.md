# NebulaOS Kernel Architecture
# =============================
#
# This document describes the architecture of the NebulaOS kernel.

## Overview

NebulaOS is a hobby x86/x86_64 operating system designed for learning and experimentation. It features:

- Dual architecture support (32-bit x86 and 64-bit x86_64)
- Monolithic kernel design
- BIOS text boot screen followed by a framebuffer desktop on x86
- Interrupt-driven I/O
- Identity-mapped paging
- Simple shell interface

## Boot Process

### x86 (32-bit)

1. BIOS loads boot sector at 0x7C00
2. Bootloader loads stage 2 from disk
3. Stage 2 loads the kernel, configures VBE, and collects E820 memory regions
4. Kernel entry (`kernel/x86/start.asm`) clears BSS and calls Rust `kernel_main`
5. The kernel reserves the kernel and framebuffer before initializing the desktop

### x86_64 (64-bit)

The Rust UEFI loader in `boot/nebula_boot/x86_64/uefi_loader` loads ELF segments, selects GOP, collects conventional-memory descriptors, exits boot services, and passes the shared framebuffer/memory handoff. The path is experimental until the OVMF smoke test passes.

## Memory Layout

### x86 (32-bit)

```
0x00000000 - 0x000003FF  Real-mode IVT
0x00000400 - 0x000004FF  BDA
0x00007C00 - 0x00007CFF  Boot sector
0x00010000 - 0x0001FFFF  Page directory/tables
0x00100000 - __kernel_end  Kernel image and BSS
0x00000000 - 0xFFFFFFFF  Pages tracked from BIOS E820 usable regions
```

### x86_64 (64-bit)

```
The UEFI loader supplies conventional-memory regions to the x86_64 allocator; firmware-owned and loader-owned ranges are omitted from the free-region list.
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

The x86_64 kernel currently retains firmware paging; a kernel-owned paging and mapping policy is still required.

### Interrupts

- CPU exceptions handled by ISR stubs in assembly
- Hardware interrupts via 8259 PIC
- Timer (PIT) at IRQ0
- Keyboard at IRQ1

## Code Organization

- `common/src/` is the authoritative shared Rust implementation.
- `kernel/src/common/mod.rs` re-exports that crate; nearby duplicate `.rs` files are not included in the module tree.
- `kernel/src/x86/` and `kernel/src/x86_64/` contain architecture-specific Rust entry and descriptor-table setup.
- `kernel/x86/` and `kernel/x86_64/` contain startup assembly and linker scripts.
- `gui/src/desktop.rs` draws the shared framebuffer desktop; the remaining widget/window types are not yet a complete windowing system.

## Driver Architecture

Drivers are organized by function:

- `drivers/src/` contains Rust driver modules.
- `drivers/src/common/` re-exports the shared `common` crate.
- Several drivers remain partial; see `docs/DRIVERS.md` for their current limits.

## Future Plans

- Complete and test UEFI file loading and ExitBootServices support
- Pass UEFI memory maps to the allocator and establish kernel-owned page tables
- Replace the RAM filesystem with a block-backed filesystem and VFS
- Add real context switching, user-mode mappings, and validated syscalls
- Add automated BIOS and UEFI QEMU smoke tests
