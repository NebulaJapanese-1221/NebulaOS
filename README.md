# NebulaOS

## A Lightweight Operating System for x86 Architecture

NebulaOS is a hobby operating system project designed to run on x86 and x86_64 hardware. It provides a kernel with memory management, device drivers, process management, filesystem support, and a simple GUI framework.

## Building

### Prerequisites
- NASM (Netwide Assembler)
- GCC with 32-bit and 64-bit support
- GNU Make
- QEMU (for testing)
- xorriso (for ISO creation)
- grub-pc-bin (for bootable ISO)

### Build Commands

```bash
./build.sh              # Build all (x86 and x86_64)
./build.sh x86          # Build x86 version
./build.sh x86_64       # Build x86_64 version
./build.sh clean        # Clean all build files
./build.sh iso          # Create bootable ISO images
./build.sh run          # Run in QEMU (x86_64 by default)
./build.sh run ARCH=x86 # Run x86 in QEMU
```

## License

This project is licenced by the GNU General Public Licence v3.0

## Version

Current version: 0.0.1

## Features

- x86 and x86_64 kernel support
- Buddy system + slab allocator for kernel heap
- FAT32 filesystem driver with ATA PIO backend
- ELF32/ELF64 binary loader
- Process management with round-robin scheduler
- Syscall interface (INT 0x80)
- PCI, ACPI, serial, VESA, RTL8139, ATA drivers
- VGA text mode and linear framebuffer GUI
- Standard C library: stdio, stdlib, ctype
- C++ runtime: new/delete, exception stubs, RTTI stubs
- GUI framework: fonts, rendering, widgets, windows
