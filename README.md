# NebulaOS

NebulaOS is an experimental hobby operating system written from scratch in Rust.
It features a graphical desktop environment with a window manager, user-mode
applications running in Ring-3 isolation, multiboot2 boot via GRUB, PS/2
keyboard and mouse input, RTC clock, PCI enumeration, IDE storage, a syscall
interface, and more.

---

## Table of Contents

1. [Features](#features)
2. [Build Requirements](#build-requirements)
3. [Installing Dependencies](#installing-dependencies)
   - [Fedora](#fedora)
   - [Ubuntu / Debian](#ubuntu--debian)
   - [Arch Linux](#arch-linux)
   - [macOS](#macos)
4. [Building](#building)
5. [Running](#running)
6. [Keyboard Shortcuts](#keyboard-shortcuts)
7. [Projects & Apps](#projects--apps)
8. [Development Notes](#development-notes)
9. [License](#license)

---

## Features

- **Graphical desktop** — VGA framebuffer with double buffering, dirty-rectangle partial updates, window manager with drag/move/resize/maximize/minimize, taskbar, start menu, wallpaper themes
- **User-mode apps** — Ring-3 ELF processes with private address spaces; Calculator, Text Editor (save/load to disk), File Manager, Paint, System Monitor, Terminal, Clock, Calendar, Task Manager, File Viewer (hex dump), Help Viewer (syscall reference)
- **Window manager** — Drag to move, bottom-right handle to resize, X/_/[] window buttons, maximized state, taskbar switching, keyboard app launch (1-8)
- **Syscall interface** — `int 0x80` ABI: print, exit, ticks, draw commands, input polling, app context, storage (list/read/write/delete/rename/format/mkdir), clipboard get/set, process info, memory info, device info, terminate process, reboot, shutdown, sleep, yield
- **Storage** — IDE disk driver, `format_if_blank`, file read/write/list/delete/rename/mkdir
- **Input** — PS/2 keyboard (with F12 diagnostic mode at boot), PS/2 mouse with sensitivity settings
- **Diagnostic mode** — Press **F12** during boot to view hardware info (CPU count, memory frames, RTC date/time, PCI devices, storage status)
- **ACPI/APM-style power management** — `reboot()` and `shutdown()` syscalls exposed via terminal commands
- **Multiboot2** — Boots via GRUB ISO image

---

## Build Requirements

| Tool | Purpose |
|------|---------|
| `rustc` / `cargo` (nightly) | Rust compiler and package manager |
| `nasm` | x86 assembler for boot code |
| `ld` (GNU binutils) | Linker for kernel ELF |
| `objcopy` (GNU binutils) | Convert userspace ELF to binary object |
| `grub-mkrescue` or `grub2-mkrescue` | Build bootable ISO |
| `xorriso` | GRUB rescue image tool |
| `qemu-system-i386` | Emulator for testing (optional) |

---

## Installing Dependencies

### Fedora

```bash
sudo dnf install cargo rustc nasm binutils grub2-tools xorriso qemu-system-i386
rustup toolchain install nightly
rustup default nightly
```

> On Fedora, `grub-mkrescue` may not exist; the Makefile auto-detects `grub2-mkrescue` instead.

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install cargo rustc nasm binutils grub-common grub-pc-bin xorriso qemu-system-i386
rustup toolchain install nightly
rustup default nightly
```

### Arch Linux

```bash
sudo pacman -S cargo rust nasm binutils grub xorriso qemu-base
rustup toolchain install nightly
rustup default nightly
```

### macOS

Cross-compiling to the i686-unknown-nebula target is not supported on macOS out of the box. Use a Linux VM.

---

## Building

Clone the repository and build the bootable ISO:

```bash
git clone https://github.com/NebulaJapanese-1221/NebulaOS.git
cd NebulaOS
make
```

The Makefile auto-detects whether your system has `grub-mkrescue` or `grub2-mkrescue` (Fedora uses the latter).

Build artifacts:

- `target/i686-unknown-nebula/release/libnebulaos.a` — kernel static library
- `boot/kernel.elf` — linked kernel ELF (multiboot2)
- `nebulaos.iso` — bootable GRUB ISO image

To clean:

```bash
make clean
```

---

## Running

### QEMU (x86)

```bash
make run
```

This boots `nebulaos.iso` in QEMU with 256 MB RAM, VGA graphics, and serial output to stdio.

If you created a data disk:

```bash
make data-disk
make run
```

### QEMU with GDB debugging

```bash
make debug
```

Then connect with `gdb` to `localhost:1234`.

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `F12` (at boot) | Enter diagnostic mode (hardware info display) |
| `Tab` | Toggle Start menu |
| `Esc` | Close focused window |
| `1`-`8` | Launch app from Start menu (1=Calculator, 2=Clock, 3=Text Editor, 4=File Manager, 5=Settings, 6=Paint, 7=System Monitor, 8=Terminal) |
| Mouse drag on title bar | Move window |
| Mouse drag on bottom-right corner | Resize window |

### Terminal Commands

| Command | Description |
|---------|-------------|
| `help` | List available commands |
| `echo <text>` | Print text |
| `clear` | Clear terminal screen |
| `uptime` | Show boot ticks |
| `mem` | Show memory statistics |
| `ps` | List processes |
| `devices` | List PCI/storage devices |
| `ls` | List storage files |
| `cat <file>` | Display file contents |
| `mkdir <dir>` | Create a directory |
| `format` | Format the storage disk |
| `copy <text>` | Copy text to clipboard |
| `paste` | Paste text from clipboard |
| `theme` | Toggle dark/light theme |
| `speed <1-4>` | Set mouse sensitivity |
| `sleep <ticks>` | Sleep for N ticks |
| `kill <pid>` | Terminate a process |
| `reboot` | Reboot the system |
| `shutdown` | Power off the system |
| `exit` | Close the terminal app |

---

## Projects & Apps

| App | Description |
|-----|-------------|
| Calculator | Basic calculator GUI |
| Clock | Uptime-based clock |
| Text Editor | Rich text editor with Save/Load to disk |
| File Manager | Browse and manage storage files |
| Paint | Simple drawing application |
| System Monitor | Real-time CPU/memory/process stats |
| Settings | Theme and mouse sensitivity |
| Terminal | Syscall command shell |
| Calendar | Calendar derived from uptime ticks |
| Task Manager | Process list with kill capability |
| File Viewer | Hex dump viewer for storage files |
| Help Viewer | Searchable syscall documentation |

---

## Development Notes

- Target architecture: `i686-unknown-nebula` (custom Rust target in `targets/`)
- Kernel entry: `kernel/src/main.rs`
- Userspace apps live in `userspace/` and are embedded into the kernel as a binary blob
- The Makefile builds both the kernel library and the userspace demo ELF, then links them together
- Syscall numbers are defined in `userspace/syscall.rs` and handled in `kernel/services/syscall.rs`
- Framebuffer uses double buffering (back buffer + display) with dirty-rectangle partial updates to avoid flicker during window drag/resize

---

## License

NebulaOS is released under the GNU General Public License v3.0 (GPL-3.0). See `LICENSE` or the header in each source file for details.
