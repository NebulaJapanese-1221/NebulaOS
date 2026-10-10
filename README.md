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

On Arch Linux, the tools can be installed with:

```sh
sudo pacman -S base-devel nasm grub xorriso qemu-system-x86
```

On Fedora, the tools can be installed with:

```sh
sudo dnf install gcc gcc-c++ make nasm grub2 xorriso qemu-system-x86
```

## Build and run

```sh
make
make run
```

`make` creates `nebulaos.iso`, a bootable GRUB ISO containing the Multiboot ELF kernel. `make run` starts it in QEMU. Override tool commands as needed, for example `make QEMU=qemu-system-i386`.
