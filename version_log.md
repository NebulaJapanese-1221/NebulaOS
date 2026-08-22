# NebulaOS Version Log

## 0.0.2 (2026-08-21)
- Memory management: replaced bump allocator with buddy system + slab allocator
- Filesystem: added FAT32 driver skeleton and ATA PIO driver
- ELF loader: added ELF32/ELF64 binary loading support
- Process management: added PCB, process_create/destroy, context switch stubs
- Scheduler: round-robin scheduler with 10ms time slice
- Syscall interface: added dispatch table, INT 0x80 handlers for x86/x86_64
- Drivers: added PCI, ACPI, serial (COM1), VESA, RTL8139, ATA
- x86_64 port: completed GDT/IDT/ISR/shell/stubs
- Build system: real ISO creation with GRUB El Torito, scripts (install-toolchain, run)
- Configuration: added config/kernel.conf
- Documentation: added docs/BUILD.md, docs/KERNEL.md, docs/DRIVERS.md
- Standard library: added stdio (printf), stdlib (atoi, rand, qsort, etc.), ctype
- C++ runtime: added new/delete, exception stubs, RTTI stubs
- GUI: added Font8x8, Renderer, FontRenderer, Checkbox, RadioButton, ProgressBar, MenuBar, FileManager, Terminal

## 0.0.1 (2026-08-21)
- Initial release
- x86 bootloader with multiboot header support
- Basic kernel entry point and early initialization
- GDT/IDT/ISR/PIC/PIT implementation for x86
- VGA text mode driver with color support
- Basic memory management framework
- Keyboard and mouse drivers
- Simple shell with command history and basic commands
- GUI class stubs (C++): Window, Button, Label, Panel, etc.
- Standard C library stubs: string, math, time
- Cross-compilation build system (Makefile) for x86 and x86_64
- Project structure reorganization:
  - Drivers moved into `drivers/src/<name>/` subdirectories
  - Kernel sources moved into architecture-specific `src/` subdirectories
  - Library sources moved into `lib/src/<name>/` subdirectories
- Updated version from 0.1.0 to 0.0.1

### Known Issues
- x86_64 kernel sources are not yet implemented
- No filesystem driver
- No process management / scheduler
- No syscall interface
- GUI is text-mode only (no VESA/graphics mode)
- ISO creation is a placeholder