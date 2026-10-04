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

The kernel image is linked to run from the higher half at `0xC0100000` while GRUB still loads it at physical `0x00100000`; `kernel/arch/x86/entry.asm` builds a temporary 4 MiB-page mapping, switches to the higher half, and `kernel/paging.cpp` then installs the permanent mapping. The lower 3 GB is therefore free for user space once processes exist. Low memory stays identity mapped so the page frame allocator and kernel heap can keep handing out directly dereferenceable physical addresses.

## Desktop and console

The desktop is a taskbar with a start menu, an information panel, a mouse pointer, and a text console filling the area below the taskbar. The console is implemented in `shell/console.cpp` as a character grid drawn straight into the framebuffer with the same 5x7 font the rest of the desktop uses, and `shell/shell.cpp` provides the command line and command set.

Commands: `help`, `clear`, `uptime`, `mem`, `display`, `echo`, `ver`, `about`, `reboot`. Command names are matched without regard to case, because the font has no lowercase glyphs and draws every character as uppercase.

Editing keys: `Backspace`, `Delete`, `Home`, `End`, and the arrow keys. Control combinations arrive as the usual control codes, so `Ctrl+L` clears the screen, `Ctrl+U` clears the line, and `Ctrl+C` abandons the line.

The built-in font covers `A` to `Z`, `0` to `9`, and common punctuation, and folds lowercase to uppercase. Symbols such as `@`, `#`, `$`, `{`, and `}` have no glyph and are not drawn, so the input buffer stores them but they are invisible on screen.

## Hardware notes

This is a 32-bit x86 BIOS/GRUB Multiboot 1 kernel. It is intended to boot on QEMU and legacy-BIOS or BIOS-compatibility-mode x86 systems with a GRUB-supported 32-bit RGB framebuffer and PS/2-compatible input. Native UEFI-only systems, framebuffer formats other than 32-bit RGB, and framebuffer addresses above 4 GiB are not supported yet. Hardware boot depends on firmware and GRUB providing a Multiboot 1 handoff and usable framebuffer; test the ISO in a virtual machine before trying it on physical hardware.
