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
    mkdir -p "$ISO_DIR/boot/grub"
    mkdir -p "$ISO_DIR/boot/nebulaos"
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
        local TARGET="i686-unknown-none"
        local KERNEL_ELF="$BUILD_DIR/nebulaos_${arch}.elf"
        local KERNEL_BIN="$BUILD_DIR/nebulaos_${arch}.bin"
        local LINK_SCRIPT="$PROJECT_DIR/kernel/x86/link.ld"
        local ASM_FILES="$PROJECT_DIR/kernel/x86/src/interrupts/isr.asm $PROJECT_DIR/kernel/x86/src/syscall/syscall_entry.asm $PROJECT_DIR/kernel/x86/src/device/rm_trampoline.asm"
    else
        local TARGET="x86_64-unknown-none"
        local KERNEL_ELF="$BUILD_DIR/nebulaos_${arch}.elf"
        local KERNEL_BIN="$BUILD_DIR/nebulaos_${arch}.bin"
        local LINK_SCRIPT="$PROJECT_DIR/kernel/x86_64/link.ld"
        local ASM_FILES="$PROJECT_DIR/kernel/x86_64/src/interrupts/isr.asm $PROJECT_DIR/kernel/x86_64/src/syscall/syscall_entry.asm"
    fi

    info "  Building Rust kernel for $TARGET..."
    cd "$PROJECT_DIR"
    if ! cargo build --target "$TARGET" --release; then
        error "Rust build failed for $arch"
        return 1
    fi

    if [ -f "target/$TARGET/release/kernel" ]; then
        cp "target/$TARGET/release/kernel" "$KERNEL_ELF"
    elif [ -f "target/$TARGET/release/libkernel.a" ]; then
        error "Only static lib produced, need full kernel binary"
        return 1
    fi

    if [ -f "$KERNEL_ELF" ]; then
        info "  Converting to binary..."
        if [ "$arch" = "x86" ]; then
            i686-linux-gnu-objcopy -O binary "$KERNEL_ELF" "$KERNEL_BIN"
        else
            x86_64-linux-gnu-objcopy -O binary "$KERNEL_ELF" "$KERNEL_BIN"
        fi
        info "  Build complete: $KERNEL_BIN"
    else
        error "Kernel ELF not found after build"
        return 1
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

    cat > "$ISO_DIR/boot/grub/grub.cfg" <<EOF
set timeout=5
set default=0

menuentry "NebulaOS $arch" {
    multiboot2 /boot/nebulaos/nebulaos_${arch}.elf
    boot
}
EOF

    local GRUB_CORE="$ISO_DIR/boot/grub/i386-pc/core.img"
    mkdir -p "$(dirname "$GRUB_CORE")"

    grub-mkimage -O i386-pc-pxe \
        -o "$GRUB_CORE" \
        -p /boot/grub \
        -d /usr/lib/grub/i386-pc \
        normal configfile multiboot2 linux

    xorriso -as mkisofs \
        -R -b boot/grub/i386-pc/core.img \
        -c boot/grub/boot.cat \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        -o "$ISO_FILE" \
        "$ISO_DIR"

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
