# NebulaOS

Minimal x86 BIOS operating system using C++ and NASM assembly.

## Requirements

- NASM
- GNU cross compiler (`i686-elf-g++`, `i686-elf-ld`, `i686-elf-objcopy`)
- QEMU
- GNU Make

## Build and run

```sh
make
make run
```

The boot sector loads the C++ kernel from the following sectors, switches to 32-bit protected mode, and jumps to the kernel at `0x00100000`.

## Source layout

- `boot/bios/NebulaBoot.asm` contains the BIOS loader and protected-mode transition.
- `kernel/` contains the C++ kernel entry and x86 linker script.
- `drivers/` contains hardware-facing VGA and PS/2 keyboard code.
- `shell/` contains the text-mode `NebulaBoot>` shell.

The shell currently supports `help`, `clear`, `about`, and `echo`.
