#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"
BUILD_DIR="$PROJECT_DIR/build"
ISO_DIR="$BUILD_DIR/iso"
ARCH="${ARCH:-all}"

info() { echo -e "\033[0;32m[INFO]\033[0m $1"; }
warn() { echo -e "\033[1;33m[WARN]\033[0m $1"; }
error() { echo -e "\033[0;31m[ERROR]\033[0m $1"; }

require_tool() {
    local tool="$1"
    if ! command -v "$tool" >/dev/null 2>&1; then
        error "Required tool '$tool' is not installed or not on PATH."
        return 1
    fi
}

ensure_nightly() {
    if ! rustup toolchain list 2>/dev/null | grep -q '^nightly'; then
        error "Rust nightly is required. Install it with: rustup toolchain install nightly"
        return 1
    fi
}

target_json_for() {
    local arch="$1"
    if [ "$arch" = "x86" ]; then
        echo "$PROJECT_DIR/targets/x86.json"
    else
        echo "$PROJECT_DIR/targets/x86_64.json"
    fi
}

target_triple_for() {
    local arch="$1"
    if [ "$arch" = "x86" ]; then
        echo "x86"
    else
        echo "x86_64"
    fi
}

objcopy_for() {
    local arch="$1"
    if [ "$arch" = "x86" ]; then
        if command -v i686-linux-gnu-objcopy >/dev/null 2>&1; then
            echo "i686-linux-gnu-objcopy"
        elif command -v objcopy >/dev/null 2>&1; then
            echo "objcopy"
        else
            echo ""
        fi
    else
        if command -v x86_64-linux-gnu-objcopy >/dev/null 2>&1; then
            echo "x86_64-linux-gnu-objcopy"
        elif command -v llvm-objcopy >/dev/null 2>&1; then
            echo "llvm-objcopy"
        elif command -v objcopy >/dev/null 2>&1; then
            echo "objcopy"
        else
            echo ""
        fi
    fi
}

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
    local target_json
    local target_triple
    local kernel_elf
    local kernel_bin
    local rust_artifact
    local objcopy_cmd

    info "Building NebulaOS for $arch..."

    target_json="$(target_json_for "$arch")"
    target_triple="$(target_triple_for "$arch")"
    kernel_elf="$BUILD_DIR/nebulaos_${arch}.elf"
    kernel_bin="$BUILD_DIR/nebulaos_${arch}.bin"

    require_tool cargo || return 1
    ensure_nightly || return 1
    require_tool nasm || return 1

    info "  Building Rust kernel for $target_json..."
    cargo +nightly -Zjson-target-spec -Zbuild-std=core build --manifest-path "$PROJECT_DIR/kernel/Cargo.toml" --target "$target_json" --release

    rust_artifact="$PROJECT_DIR/target/$target_triple/release/kernel"
    if [ ! -f "$rust_artifact" ]; then
        rust_artifact="$PROJECT_DIR/target/$target_triple/release/kernel.exe"
    fi
    if [ ! -f "$rust_artifact" ]; then
        rust_artifact="$(find "$PROJECT_DIR/target" -path "*/$target_triple/release/*" -type f \( -name kernel -o -name kernel.exe \) 2>/dev/null | head -n 1)"
    fi

    if [ -n "$rust_artifact" ] && [ -f "$rust_artifact" ]; then
        cp "$rust_artifact" "$kernel_elf"
    fi

    if [ ! -f "$kernel_elf" ]; then
        error "Kernel ELF not found after build"
        return 1
    fi

    objcopy_cmd="$(objcopy_for "$arch")"
    if [ -z "$objcopy_cmd" ]; then
        error "Could not find an objcopy tool for $arch. Install binutils or a cross-objcopy package."
        return 1
    fi

    "$objcopy_cmd" -O binary "$kernel_elf" "$kernel_bin"
    info "  Build complete: $kernel_bin"
}

build_nebula_boot() {
    local arch="$1"
    info "Building NebulaBoot for $arch..."

    require_tool nasm || return 1

    if [ "$arch" = "x86" ]; then
        nasm -f bin "$PROJECT_DIR/boot/nebula_boot/x86/boot.asm" -o "$BUILD_DIR/nebula_boot_x86.bin"
    else
        require_tool x86_64-w64-mingw32-gcc || return 1
        nasm -f win64 "$PROJECT_DIR/boot/nebula_boot/x86_64/boot.asm" -o "$BUILD_DIR/boot_x86_64.obj"
        x86_64-w64-mingw32-gcc -nostdlib -Wl,-entry,efi_main -Wl,-subsystem,efi_application -o "$BUILD_DIR/bootx64.efi" "$BUILD_DIR/boot_x86_64.obj"
    fi
}

create_iso() {
    local arch="$1"
    local kernel_file
    local iso_file

    info "Creating ISO for $arch..."

    kernel_file="$BUILD_DIR/nebulaos_${arch}.elf"
    iso_file="$ISO_DIR/nebulaos_${arch}.iso"

    if [ ! -f "$kernel_file" ]; then
        error "Kernel ELF not found: $kernel_file"
        error "Build the kernel first: ./build.sh $arch"
        return 1
    fi

    mkdir -p "$ISO_DIR/boot/nebulaos"
    cp "$kernel_file" "$ISO_DIR/boot/nebulaos/nebulaos_${arch}.elf"

    if [ "$arch" = "x86" ]; then
        build_nebula_boot x86 || return 1
        cp "$BUILD_DIR/nebula_boot_x86.bin" "$ISO_DIR/boot/nebulaos/nebula_boot_x86.bin"
        require_tool xorriso || return 1
        xorriso -as mkisofs -R -b boot/nebulaos/nebula_boot_x86.bin -no-emul-boot -boot-load-size 4 -boot-info-table -o "$iso_file" "$ISO_DIR"
    else
        build_nebula_boot x86_64 || return 1
        cp "$BUILD_DIR/bootx64.efi" "$ISO_DIR/EFI/BOOT/BOOTX64.EFI"
        cp "$kernel_file" "$ISO_DIR/EFI/NEBULA/nebulaos_x86_64.elf"
        require_tool xorriso || return 1
        xorriso -as mkisofs -R -eltorito-alt-boot -e EFI/BOOT/BOOTX64.EFI -no-emul-boot -o "$iso_file" "$ISO_DIR"
    fi

    info "ISO created: $iso_file"
}

run_qemu() {
    local arch="$1"
    local qemu_bin
    local iso_file

    if [ "$arch" = "x86" ]; then
        qemu_bin="qemu-system-i386"
        iso_file="$ISO_DIR/nebulaos_x86.iso"
    else
        qemu_bin="qemu-system-x86_64"
        iso_file="$ISO_DIR/nebulaos_x86_64.iso"
    fi

    if [ ! -f "$iso_file" ]; then
        error "ISO not found: $iso_file"
        error "Build the ISO first: ./build.sh iso"
        return 1
    fi

    require_tool "$qemu_bin" || return 1

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
