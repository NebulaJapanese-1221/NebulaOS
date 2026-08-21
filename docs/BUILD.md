# NebulaOS Build Instructions
# ============================
#
# This document describes how to build NebulaOS from source.

## Prerequisites

### Linux (Ubuntu/Debian)

```bash
# Run the automated setup script
chmod +x scripts/install-toolchain.sh
sudo scripts/install-toolchain.sh
```

Or install manually:

```bash
sudo apt-get update
sudo apt-get install -y \
    gcc-multilib \
    g++-multilib \
    nasm \
    xorriso \
    qemu-system-x86 \
    qemu-system-x86_64 \
    build-essential \
    git
```

### Linux (Fedora/RHEL)

```bash
sudo dnf install -y \
    gcc \
    gcc-c++ \
    glibc-devel \
    glibc-devel.i686 \
    nasm \
    xorriso \
    qemu-system-x86 \
    qemu-system-x86_64 \
    make \
    git
```

### Linux (Arch)

```bash
sudo pacman -Syu --noconfirm \
    gcc \
    make \
    nasm \
    xorriso \
    qemu-system-x86 \
    multilib-devel \
    git
```

## Building

Build all architectures:

```bash
make
```

Build specific architecture:

```bash
make x86      # Build 32-bit x86 kernel
make x86_64   # Build 64-bit x86_64 kernel
```

Or using the ARCH variable:

```bash
make ARCH=x86_64
```

## Running

Run in QEMU:

```bash
make run ARCH=x86_64
```

Run from ISO:

```bash
make iso
make run-iso ARCH=x86_64
```

Run with custom options:

```bash
./scripts/run.sh --arch=x86_64 --ram=512M
```

Debug with GDB:

```bash
make gdb ARCH=x86_64
```

## Creating an ISO

```bash
make iso
```

The ISO will be created at `build/iso/nebulaos_x86_64.iso`.

## Cleaning

```bash
make clean
```

## Directory Structure

```
NebulaOS/
├── boot/                    # Bootloaders
│   ├── x86/                 # 32-bit bootloader
│   └── x86_64/              # 64-bit bootloader
├── config/                  # Configuration files
│   └── kernel.conf          # Kernel build configuration
├── docs/                    # Documentation
├── drivers/                 # Hardware drivers
│   ├── include/             # Driver headers
│   └── src/                 # Driver implementations
├── gui/                     # GUI framework (text mode)
├── kernel/                  # Kernel source
│   ├── common/              # Architecture-independent code
│   ├── x86/                 # 32-bit specific code
│   └── x86_64/              # 64-bit specific code
├── lib/                     # Kernel libraries
│   ├── include/             # Library headers
│   └── src/                 # Library implementations
├── scripts/                 # Build and utility scripts
└── tools/                   # Development tools
```

## Troubleshooting

### Cross-compiler not found

If you see errors about `i686-linux-gnu-gcc` or `x86_64-linux-gnu-gcc` not being found, make sure you have installed the multilib cross-compiler packages.

### QEMU crashes on boot

Make sure you are using the correct architecture binary. The x86_64 kernel will not boot in qemu-system-i386.

### ISO won't boot

The ISO uses GRUB for booting. Make sure genisoimage is installed. If you are using UEFI, you may need to add UEFI support to the build system.
