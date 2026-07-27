# NebulaOS

A hobbyist 32-bit operating system written in Rust for the i686 architecture.

### Credits for NebulaFS

NebulaFS is inspired by and builds upon the groundbreaking work of the ZFS developers:

- **Original ZFS Team at Sun Microsystems**: Jeff Bonwick, Bill Moore, Matthew Ahrens, and many others who designed and implemented the original ZFS file system.

- **OpenZFS Community**: The open-source community that has continued to develop and maintain ZFS across multiple platforms.

- **ZFS on Linux Team**: For bringing ZFS to Linux and continuing its development.

While NebulaFS is not a derivative work of ZFS and does not use any ZFS code, its design and architecture are heavily influenced by the innovative concepts introduced by ZFS. We are grateful to all the developers who have contributed to ZFS over the years.

## License

This project is licensed under the GNU General Public License (GPL) v3.0.

## Apps
- **Calculator**
- **Text Editor**
- **Terminal**
- **System Settings**
- **File Manager**
- **Web Browser**

## Prerequisites

Before building, ensure you have the following installed:

| Component              | Package name (Ubuntu/Debian)        | Notes                                                |
|------------------------|--------------------------------------|------------------------------------------------------|
| Rust Nightly toolchain | `rustup`                             | See install instructions below                       |
| LLVM tools             | `llvm-dev` `lld`                     | Provides `llvm-objcopy` and `rust-lld`               |
| GRUB ISO support       | `grub-pc-bin` `grub-common`          | Provides `grub-mkrescue`                             |
| ISO creation tool      | `xorriso`                            | Required by `grub-mkrescue`                          |
| QEMU                   | `qemu-system-x86`                    | For emulation                                        |

### Installing the Rust nightly toolchain

```bash
rustup install nightly
rustup component add rust-src --toolchain nightly
```

### Installing system dependencies (Ubuntu/Debian)

```bash
sudo apt update && sudo apt install -y llvm-dev lld grub-pc-bin grub-common xorriso qemu-system-x86
```

---

## Building and Running

### Quick start (interactive menu)

```bash
make menu
```

This launches an interactive menu where you can:
1. Select the architecture (x86 or x86_64)
2. Choose to build or build-and-run

### Build for x86 (32-bit, default)

```bash
make build
```

This compiles the bootloader and kernel, targeting i686.

### Build for x86_64 (64-bit)

```bash
make ARCH=x86_64 build
```

### Run in QEMU

**x86 (default):**

```bash
make run
```

This builds the kernel, creates a bootable ISO via `grub-mkrescue`, and boots it in QEMU.

**x86_64:**

```bash
make run64
```

### Build ISO manually

```bash
make ARCH=x86       # or ARCH=x86_64
make nebula-x86.iso # or nebula-x86_64.iso
```

### Clean build artifacts

```bash
make clean
```

---

## Project Structure

```
NebulaOS/
├── src/
│   ├── arch/              # Architecture-specific code
│   │   ├── x86/           #   - x86 (32-bit, i686)
│   │   │   ├── paging.rs  #   - Page table management
│   │   │   ├── gdt.rs     #   - Global Descriptor Table
│   │   │   ├── idt.rs     #   - Interrupt Descriptor Table
│   │   │   └── ...
│   │   └── x86_64/        #   - x86_64 (64-bit)
│   │       ├── paging.rs  #   - Page table management
│   │       └── ...
│   ├── boot/              # Bootloader (x86 real-mode -> protected mode)
│   ├── kernel/            # Core kernel
│   │   ├── main.rs        # Kernel entry point and main loop
│   │   ├── memory/        # Memory management (buddy, slab, paging)
│   │   ├── scheduler.rs   # Process scheduler
│   │   ├── process.rs     # Process/thread management
│   │   └── ...
│   ├── drivers/           # Hardware drivers
│   │   ├── vga.rs         # VGA text mode
│   │   ├── framebuffer.rs # Linear framebuffer
│   │   ├── keyboard.rs    # PS/2 keyboard
│   │   ├── mouse.rs       # PS/2 mouse
│   │   ├── pit.rs         # Programmable Interval Timer
│   │   └── ...
│   ├── fs/                # NebulaFS filesystem
│   └── userspace/         # Userspace apps and GUI
│       ├── apps/          # Calculator, Terminal, File Manager, etc.
│       └── gui/           # Window manager, widgets, login screen
├── Makefile               # Build system
├── Cargo.toml             # Rust package manifest
├── i686-nebula.json       # Target spec for x86 (32-bit)
└── x86_64-nebula.json     # Target spec for x86_64 (64-bit)
```

