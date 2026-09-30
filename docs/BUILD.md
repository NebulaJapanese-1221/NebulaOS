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
    qemu-system-i386 \
    qemu-system-x86-64 \
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
    qemu-system-i386 \
    qemu-system-x86-64 \
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

Build the verified x86 kernel (the default target):

```bash
make -f makefile.mk
```

Build specific architecture:

```bash
make -f makefile.mk x86      # Build 32-bit x86 kernel
make -f makefile.mk x86_64   # Build 64-bit x86_64 kernel
make -f makefile.mk uefi     # Build UEFI app and FAT staging directory
make -f makefile.mk test     # Run common crate unit tests
```

Or using the ARCH variable:

```bash
make -f makefile.mk x86_64
```

## Running

Run the supported x86 BIOS ISO in QEMU:

```bash
make -f makefile.mk run
```

`run` uses the ISO loader and supplies the VBE framebuffer handoff. `run-elf` boots the kernel directly and does not supply that handoff.

UEFI build and run require the `x86_64-unknown-uefi` Rust target, QEMU with OVMF, and an OVMF firmware image:

```bash
make -f makefile.mk uefi
make -f makefile.mk run-uefi OVMF_CODE=/path/to/OVMF_CODE.fd
make -f makefile.mk smoke-uefi OVMF_CODE=/path/to/OVMF_CODE.fd
```

Run from ISO:

```bash
make -f makefile.mk iso
make -f makefile.mk run-iso
```

Run with custom options:

```bash
make -f makefile.mk run
```

Debug with GDB:

```bash
make -f makefile.mk run
```

## Creating an ISO

```bash
make -f makefile.mk iso
```

The x86 ISO will be created at `build/nebulaos_x86.iso`.

## Cleaning

```bash
make -f makefile.mk clean
```

## Directory Structure

```
NebulaOS/
├── boot/nebula_boot/        # BIOS and experimental UEFI loaders
├── common/src/              # Shared no_std kernel services
├── drivers/src/             # Rust device drivers
├── gui/src/                 # Framebuffer desktop and GUI types
├── kernel/src/              # Rust kernel entry and architecture code
├── kernel/x86*/             # Startup assembly and linker scripts
├── lib/src/                 # no_std utility library
├── targets/                 # Custom Rust target specifications
└── docs/                    # Project documentation
```

`kernel/src/common/mod.rs` re-exports the external `common` crate. The neighboring files in `kernel/src/common/` are not included by that module tree; `common/src/` is the authoritative shared implementation.

## Troubleshooting

### Required tools not found

Install Rust nightly with `rust-src`, GNU Make, NASM, `objcopy`, xorriso, and QEMU. The build uses Rust rather than the C cross-compilers described in some older notes.

### QEMU crashes on boot

Make sure you are using the correct architecture binary. The x86_64 kernel will not boot in qemu-system-i386.

### ISO won't boot

The x86 ISO uses NebulaBoot via an El Torito BIOS image and requires xorriso. UEFI is experimental; run `smoke-uefi` before relying on it.
