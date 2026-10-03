# NebulaOS

NebulaOS is a small 32-bit x86 hobby operating system. GRUB loads its Multiboot kernel, which uses a 32-bit graphics framebuffer and starts a simple desktop-style shell.

## Copyright and license

Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>.

NebulaOS is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. It is distributed without any warranty; see [LICENCE](./LICENCE) for the complete license text.

Electronic contact: [nebulajapanese@gmail.com](mailto:nebulajapanese@gmail.com). No paper-mail address is available.

## Linux requirements

- GNU Make
- GCC and G++ with 32-bit compilation support
- GNU binutils (`ld`)
- NASM
- GRUB utilities (`grub-mkrescue`, typically provided by `grub-pc-bin`)
- `xorriso` (used by `grub-mkrescue`)
- QEMU (`qemu-system-i386`) to run the system

On Debian or Ubuntu, the tools can be installed with:

```sh
sudo apt install build-essential nasm grub-pc-bin xorriso qemu-system-x86
```

## Build and run

```sh
make
make run
```

`make` creates `nebulaos.iso`, a bootable GRUB ISO containing the Multiboot ELF kernel. `make run` starts it in QEMU. Override tool commands as needed, for example `make QEMU=qemu-system-i386`.

At startup the kernel presents a text-only status screen before launching the graphical desktop. GRUB selects and passes the graphics framebuffer using the Multiboot information structure. Framebuffer access and pixel drawing are isolated in `kernel/framebuffer.cpp` and `kernel/framebuffer.hpp`; the display text and desktop UI are layered on top by the graphics driver. The x86 startup code installs a kernel GDT, IDT exception/IRQ stubs, a remapped PIC, and a PIT-driven system tick used for boot delays.

## Hardware notes

This is a 32-bit x86 BIOS/GRUB Multiboot 1 kernel. It is intended to boot on QEMU and legacy-BIOS or BIOS-compatibility-mode x86 systems with a GRUB-supported 32-bit RGB framebuffer and PS/2-compatible input. Native UEFI-only systems, framebuffer formats other than 32-bit RGB, and framebuffer addresses above 4 GiB are not supported yet. Hardware boot depends on firmware and GRUB providing a Multiboot 1 handoff and usable framebuffer; test the ISO in a virtual machine before trying it on physical hardware.
