# NebulaOS

NebulaOS is a small 32-bit x86 hobby operating system. GRUB loads its Multiboot kernel, which requests a 32-bit graphics framebuffer and starts a simple desktop-style shell.

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

## Shell

The initial GUI shell draws a desktop header, status tile, terminal window, and taskbar using the GRUB-provided linear framebuffer. It accepts keyboard input and supports:

- `help`
- `clear`
- `about`
- `echo <text>`

The display currently uses a basic built-in bitmap font and a fixed 32-bit RGB framebuffer mode request (800x600).

## Source layout

- `boot/grub/grub.cfg` defines the GRUB boot menu.
- `kernel/` contains the Multiboot entry point, kernel startup, and x86 linker script.
- `drivers/graphics.*` draws the framebuffer interface; `drivers/keyboard.*` reads PS/2 keyboard input.
- `shell/` contains the graphical terminal shell.
