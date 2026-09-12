#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"
BUILD_DIR="$PROJECT_DIR/build"
ISO_DIR="$BUILD_DIR/iso"

info() { echo -e "\033[0;32m[INFO]\033[0m $1"; }
warn() { echo -e "\033[1;33m[WARN]\033[0m $1"; }
error() { echo -e "\033[0;31m[ERROR]\033[0m $1"; }

ARCH="${ARCH:-all}"

create_dirs() {
    mkdir -p "$BUILD_DIR"
    mkdir -p "$ISO_DIR/boot/nebulaos"
    mkdir -p "$ISO_DIR/EFI/BOOT"
    mkdir -p "$ISO_DIR/EFI/NEBULA"
}

clean() {
    info "Cleaning build directory..."
    rm -rf "$BUILD_DIR"
    info "Clean complete."
}

build_arch() {
    local arch="$1"
    info "Building NebulaOS for $arch..."

    if [ "$arch" = "x86" ]; then
        local TARGET="$PROJECT_DIR/targets/x86.json"
        local KERNEL_ELF="$BUILD_DIR/nebulaos_${arch}.elf"
        local KERNEL_BIN="$BUILD_DIR/nebulaos_${arch}.bin"
    else
        local TARGET="x86_64-unknown-none"
        local KERNEL_ELF="$BUILD_DIR/nebulaos_${arch}.elf"
        local KERNEL_BIN="$BUILD_DIR/nebulaos_${arch}.bin"
    fi

    info "  Building Rust kernel for $TARGET..."
    cd "$PROJECT_DIR"
    if ! cargo build -Zjson-target-spec -Zbuild-std=core,alloc --target "$TARGET" --release; then
        error "Rust build failed for $arch"
        return 1
    fi

    # Find the kernel binary
    find "target" -name "kernel" -type f 2>/dev/null | head -1 | xargs -I {} cp {} "$KERNEL_ELF" 2>/dev/null || true
    
    if [ -f "$KERNEL_ELF" ]; then
        info "  Build complete: $KERNEL_ELF"
    else
        error "Kernel ELF not found after build"
        return 1
    fi
}

build_nebula_boot() {
    local arch="$1"
    info "Building NebulaBoot for $arch..."

    if [ "$arch" = "x86" ]; then
        # BIOS bootloader (flat binary)
        nasm -f bin "$PROJECT_DIR/boot/nebula_boot/x86/boot.asm" -o "$BUILD_DIR/nebula_boot_x86.bin"
        if [ $? -ne 0 ]; then
            error "NebulaBoot x86 build failed"
            return 1
        fi
    else
        # UEFI bootloader (PE32+)
        nasm -f win64 "$PROJECT_DIR/boot/nebula_boot/x86_64/boot.asm" -o "$BUILD_DIR/boot_x86_64.obj"
        if [ $? -ne 0 ]; then
            error "NebulaBoot x86_64 build failed"
            return 1
        fi

        x86_64-w64-mingw32-gcc -nostdlib -Wl,-entry,efi_main -Wl,-subsystem,efi_application \
            -o "$BUILD_DIR/bootx64.efi" "$BUILD_DIR/boot_x86_64.obj"
        if [ $? -ne 0 ]; then
            error "NebulaBoot x86_64 link failed"
            return 1
        fi
    fi
}

create_iso() {
    local arch="$1"
    info "Creating ISO for $arch..."

    local KERNEL_FILE="$BUILD_DIR/nebulaos_${arch}.elf"
    local ISO_FILE="$ISO_DIR/nebulaos_${arch}.iso"

    if [ ! -f "$KERNEL_FILE" ]; then
        error "Kernel ELF not found: $KERNEL_FILE"
        error "Build the kernel first: ./build.sh $arch"
        return 1
    fi

    mkdir -p "$ISO_DIR/boot/nebulaos"
    cp "$KERNEL_FILE" "$ISO_DIR/boot/nebulaos/nebulaos_${arch}.elf"

    if [ "$arch" = "x86" ]; then
        # Build NebulaBoot for x86
        build_nebula_boot x86 || return 1

        # Copy bootloader to ISO
        cp "$BUILD_DIR/nebula_boot_x86.bin" "$ISO_DIR/boot/nebulaos/nebula_boot_x86.bin"
        if [ $? -ne 0 ]; then
            error "Failed to copy NebulaBoot x86"
            return 1
        fi

        # Create BIOS-bootable ISO with El Torito
        xorriso -as mkisofs \
            -R -b boot/nebulaos/nebula_boot_x86.bin \
            -no-emul-boot \
            -boot-load-size 4 \
            -boot-info-table \
            -o "$ISO_FILE" \
            "$ISO_DIR"
    else
        # Build NebulaBoot for x86_64 (UEFI)
        build_nebula_boot x86_64 || return 1

        # Copy UEFI bootloader
        cp "$BUILD_DIR/bootx64.efi" "$ISO_DIR/EFI/BOOT/BOOTX64.EFI"
        if [ $? -ne 0 ]; then
            error "Failed to copy UEFI bootloader"
            return 1
        fi

        # Also copy kernel to EFI/NEBULA for the bootloader to find
        cp "$KERNEL_FILE" "$ISO_DIR/EFI/NEBULA/nebulaos_x86_64.elf"
        if [ $? -ne 0 ]; then
            error "Failed to copy kernel to EFI"
            return 1
        fi

        # Create UEFI-bootable ISO
        xorriso -as mkisofs \
            -R \
            -eltorito-alt-boot \
            -e EFI/BOOT/BOOTX64.EFI \
            -no-emul-boot \
            -o "$ISO_FILE" \
            "$ISO_DIR"
    fi

    info "ISO created: $ISO_FILE"
}

run_qemu() {
    local arch="$1"

    if [ "$arch" = "x86" ]; then
        local qemu_bin="qemu-system-i386"
        local iso_file="$ISO_DIR/nebulaos_x86.iso"
    else
        local qemu_bin="qemu-system-x86_64"
        local iso_file="$ISO_DIR/nebulaos_x86_64.iso"
    fi

    if [ ! -f "$iso_file" ]; then
        error "ISO not found: $iso_file"
        error "Build the ISO first: ./build.sh iso"
        return 1
    fi

    info "Running NebulaOS ($arch) in QEMU..."
    info "  ISO: $iso_file"
    info "  Press Ctrl+Alt+G to release mouse, Ctrl+Alt+Q to quit"

    exec "$qemu_bin" -cdrom "$iso_file" -m 256M -serial stdio
}

case "${1:-all}" in
    clean)
        clean
        ;;
    x86)
        create_dirs
        build_arch x86
        ;;
    x86_64)
        create_dirs
        build_arch x86_64
        ;;
    iso)
        if [ "$ARCH" = "all" ] || [ "$ARCH" = "x86" ]; then
            create_iso x86
        fi
        if [ "$ARCH" = "all" ] || [ "$ARCH" = "x86_64" ]; then
            create_iso x86_64
        fi
        ;;
    run)
        if [ "$ARCH" = "all" ] || [ "$ARCH" = "x86" ]; then
            run_qemu x86
        else
            run_qemu "$ARCH"
        fi
        ;;
    all|*)
        create_dirs
        build_arch x86
        build_arch x86_64
        ;;
esac
