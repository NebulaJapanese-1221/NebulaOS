# NebulaOS

## A Lightweight Operating System for x86 Architecture

NebulaOS is a hobby operating system project designed to run on x86 and x86_64 hardware. It provides a kernel with memory management, device drivers, process management, filesystem support, and a simple GUI framework.

## Building

### Prerequisites (Linux)
- NASM (Netwide Assembler)
- QEMU (for testing)
- xorriso (for ISO creation)
- Rust toolchain (nightly)

### Prerequisites (Windows)
- NASM (Netwide Assembler, installed to PATH)
- QEMU (for testing)
- xorriso (for ISO creation)
- mingw-w64 (for UEFI bootloader on x86_64)
- Rust toolchain (nightly)
### Build Commands

```bash
make -f makefile.mk          # Build the x86 kernel
make -f makefile.mk x86_64   # Build the x86_64 kernel
make -f makefile.mk iso      # Create the x86 NebulaBoot ISO
make -f makefile.mk run      # Boot the x86 ELF directly in QEMU
make -f makefile.mk run-iso  # Boot the x86 ISO in QEMU
make -f makefile.mk clean    # Remove generated build files
```

GNU Make, Rust nightly with `rust-src`, NASM, and `objcopy` are required to build. QEMU is required to run the kernel.

## Bootloader

NebulaOS uses **NebulaBoot**, a custom bootloader:
- **x86 (BIOS)**: NebulaBoot loads the appended kernel image and enters protected mode
- **x86_64 (UEFI)**: NebulaBoot UEFI application presents a menu and loads the kernel directly in long mode

The x86 BIOS ISO boots through NebulaBoot and passes VBE and E820 information to the kernel. Use `make -f makefile.mk run` or `run-iso`; `run-elf` bypasses the loader and framebuffer handoff.

The x86_64 UEFI loader is a separate Rust `no_std` application under `boot/nebula_boot/x86_64/uefi_loader`. Build it with `make -f makefile.mk uefi`; boot with `make -f makefile.mk run-uefi OVMF_CODE=/path/to/OVMF_CODE.fd`. This path is experimental and still needs firmware/QEMU validation.

## License

This project is licenced by the GNU General Public Licence v3.0

## Version

