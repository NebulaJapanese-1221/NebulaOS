#!/bin/bash
# NebulaOS Build Script
# =====================
#
# Build system for NebulaOS operating system
# Supports x86 and x86_64 targets with GRUB bootloader
#
# Usage:
#   ./build.sh              - Build all (x86 and x86_64)
#   ./build.sh x86          - Build x86 version
#   ./build.sh x86_64       - Build x86_64 version
#   ./build.sh clean        - Clean all build files
#   ./build.sh iso          - Create bootable ISO images
#   ./build.sh run          - Run in QEMU (x86_64 by default)
#   ./build.sh run ARCH=x86 - Run x86 in QEMU

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"
BUILD_DIR="$PROJECT_DIR/build"
OBJ_DIR="$BUILD_DIR/obj"
ISO_DIR="$BUILD_DIR/iso"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

ARCH="${ARCH:-all}"

# Toolchain
CC_X86="i686-linux-gnu-gcc"
CC_X86_64="x86_64-linux-gnu-gcc"
CXX_X86="i686-linux-gnu-g++"
CXX_X86_64="x86_64-linux-gnu-g++"
AS="nasm"
LD_X86="i686-linux-gnu-ld"
LD_X86_64="x86_64-linux-gnu-ld"
OBJCOPY_X86="i686-linux-gnu-objcopy"
OBJCOPY_X86_64="x86_64-linux-gnu-objcopy"

# Compiler flags
CFLAGS="-ffreestanding -nostdlib -nodefaultlibs -fno-builtin -fno-stack-protector -Wall -Wextra"
CXXFLAGS="-ffreestanding -nostdlib -nodefaultlibs -fno-builtin -fno-stack-protector -Wall -Wextra -fno-exceptions -fno-rtti"

# Include paths
INCLUDES="-Ikernel/common/include -Ikernel/x86/include -Ikernel/x86_64/include -Igui/include -Ilib/include -Idrivers/include -Ikernel/x86/src/device -Ikernel/x86_64/src/device"

# -----------------------------------------------------------------------------
# Source files
# -----------------------------------------------------------------------------

# Bootloader sources
BOOT_SOURCES_X86="boot/x86/boot.asm"
BOOT_SOURCES_X86_64="boot/x86_64/boot.asm"

# Kernel sources
KERNEL_SOURCES_COMMON=" \
    kernel/common/src/vga.c \
    kernel/common/src/memory/memory.c \
    kernel/common/src/fs/fat32.c \
    kernel/common/src/process/elf.c \
    kernel/common/src/process/process.c \
    kernel/common/src/process/scheduler.c \
    kernel/common/src/syscall/syscall.c"

KERNEL_SOURCES_X86=" \
    kernel/x86/entry.c \
    kernel/x86/src/device/gdt.c \
    kernel/x86/src/interrupts/idt.c \
    kernel/x86/src/interrupts/isr.asm \
    kernel/x86/src/syscall/syscall_entry.asm \
    kernel/x86/src/process/shell.c \
    kernel/x86/src/device/realmode.c \
    kernel/x86/src/device/rm_trampoline.asm"

KERNEL_SOURCES_X86_64=" \
    kernel/x86_64/entry.c \
    kernel/x86_64/src/device/gdt.c \
    kernel/x86_64/src/interrupts/idt.c \
    kernel/x86_64/src/interrupts/isr.asm \
    kernel/x86_64/src/syscall/syscall_entry.asm \
    kernel/x86_64/src/process/shell.c \
    kernel/x86_64/src/memory/paging.c \
    kernel/x86_64/src/device/stubs.c"

# GUI sources
GUI_SOURCES=" \
    gui/src/Point.cpp \
    gui/src/Size.cpp \
    gui/src/Rectangle.cpp \
    gui/src/Color.cpp \
    gui/src/GraphicsContext.cpp \
    gui/src/Window.cpp \
    gui/src/WindowManager.cpp \
    gui/src/Control.cpp \
    gui/src/Button.cpp \
    gui/src/Label.cpp \
    gui/src/TextBox.cpp \
    gui/src/Panel.cpp \
    gui/src/Font.cpp \
    gui/src/GUI.cpp \
    gui/src/GuiBridge.cpp \
    gui/src/fonts/Font8x8.cpp \
    gui/src/rendering/Renderer.cpp \
    gui/src/rendering/FontRenderer.cpp \
    gui/src/widgets/Widget.cpp \
    gui/src/widgets/Checkbox.cpp \
    gui/src/widgets/RadioButton.cpp \
    gui/src/widgets/ProgressBar.cpp \
    gui/src/widgets/MenuBar.cpp \
    gui/src/windows/FileManager.cpp \
    gui/src/windows/Terminal.cpp"

# Driver sources
DRIVER_SOURCES=" \
    drivers/src/keyboard/keyboard.c \
    drivers/src/mouse/mouse.c \
    drivers/src/pit.c \
    drivers/src/pic.c \
    drivers/src/pci/pci.c \
    drivers/src/acpi/acpi.c \
    drivers/src/serial/serial.c \
    drivers/src/vesa/vesa.c \
    drivers/src/vesa/vbe.c \
    drivers/src/network/rtl8139.c \
    drivers/src/storage/ata.c"

# Library sources
LIB_SOURCES=" \
    lib/src/string/string.c \
    lib/src/math/math.c \
    lib/src/time/time.c \
    lib/src/stdio/printf.c \
    lib/src/stdio/scanf.c \
    lib/src/stdlib/stdlib.c \
    lib/src/ctype/ctype.c \
    lib/src/cxx/runtime.cpp \
    lib/src/cxx/exception.cpp \
    lib/src/cxx/typeinfo.cpp"

# -----------------------------------------------------------------------------
# Helper functions
# -----------------------------------------------------------------------------

create_dirs() {
    mkdir -p "$BUILD_DIR"
    mkdir -p "$OBJ_DIR/x86"
    mkdir -p "$OBJ_DIR/x86_64"
    mkdir -p "$ISO_DIR/boot/grub"
    mkdir -p "$ISO_DIR/boot/nebulaos"
}

clean() {
    info "Cleaning build directory..."
    rm -rf "$BUILD_DIR"
    info "Clean complete."
}

# Compile a single source file
compile_source() {
    local arch="$1"
    local src="$2"
    local obj="$3"
    
    mkdir -p "$(dirname "$obj")"
    
    case "$src" in
        *.asm)
            if [ "$arch" = "x86" ]; then
                $AS -f elf32 -o "$obj" "$src"
            else
                $AS -f elf64 -o "$obj" "$src"
            fi
            ;;
        *.c)
            if [ "$arch" = "x86" ]; then
                $CC_X86 $CFLAGS $INCLUDES -o "$obj" -c "$src"
            else
                $CC_X86_64 $CFLAGS $INCLUDES -o "$obj" -c "$src"
            fi
            ;;
        *.cpp)
            if [ "$arch" = "x86" ]; then
                $CXX_X86 $CXXFLAGS $INCLUDES -o "$obj" -c "$src"
            else
                $CXX_X86_64 $CXXFLAGS $INCLUDES -o "$obj" -c "$src"
            fi
            ;;
    esac
}

# Build for a specific architecture
build_arch() {
    local arch="$1"
    
    info "Building NebulaOS for $arch..."
    
    local CC LD OBJCOPY NASMFLAGS
    if [ "$arch" = "x86" ]; then
        CC="$CC_X86"
        LD="$LD_X86"
        OBJCOPY="$OBJCOPY_X86"
        NASMFLAGS="-f elf32"
    else
        CC="$CC_X86_64"
        LD="$LD_X86_64"
        OBJCOPY="$OBJCOPY_X86_64"
        NASMFLAGS="-f elf64"
    fi
    
    local OBJ_PREFIX="$OBJ_DIR/$arch"
    local KERNEL_ELF="$BUILD_DIR/nebulaos_${arch}.elf"
    local KERNEL_BIN="$BUILD_DIR/nebulaos_${arch}.bin"
    
    # Compile bootloader
    info "  Compiling bootloader..."
    local boot_obj="$OBJ_PREFIX/boot_${arch}.o"
    if [ "$arch" = "x86" ]; then
        compile_source "$arch" "$BOOT_SOURCES_X86" "$boot_obj"
    else
        compile_source "$arch" "$BOOT_SOURCES_X86_64" "$boot_obj"
    fi
    
    # Compile kernel sources
    info "  Compiling kernel..."
    local kernel_objs=""
    
    # Common kernel sources
    for src in $KERNEL_SOURCES_COMMON; do
        local obj="$OBJ_PREFIX/$(basename "$src" .c).o"
        if [ ! -f "$obj" ] || [ "$src" -nt "$obj" ]; then
            compile_source "$arch" "$src" "$obj"
        fi
        kernel_objs="$kernel_objs $obj"
    done
    
    # Architecture-specific kernel sources
    if [ "$arch" = "x86" ]; then
        for src in $KERNEL_SOURCES_X86; do
            local ext="${src##*.}"
            local obj="$OBJ_PREFIX/$(basename "$src" .$ext).o"
            if [ ! -f "$obj" ] || [ "$src" -nt "$obj" ]; then
                compile_source "$arch" "$src" "$obj"
            fi
            kernel_objs="$kernel_objs $obj"
        done
    else
        for src in $KERNEL_SOURCES_X86_64; do
            local ext="${src##*.}"
            local obj="$OBJ_PREFIX/$(basename "$src" .$ext).o"
            if [ ! -f "$obj" ] || [ "$src" -nt "$obj" ]; then
                compile_source "$arch" "$src" "$obj"
            fi
            kernel_objs="$kernel_objs $obj"
        done
    fi
    
    # Compile GUI sources
    info "  Compiling GUI..."
    for src in $GUI_SOURCES; do
        local obj="$OBJ_PREFIX/$(basename "$src" .cpp).o"
        if [ ! -f "$obj" ] || [ "$src" -nt "$obj" ]; then
            compile_source "$arch" "$src" "$obj"
        fi
        kernel_objs="$kernel_objs $obj"
    done
    
    # Compile driver sources
    info "  Compiling drivers..."
    for src in $DRIVER_SOURCES; do
        local obj="$OBJ_PREFIX/$(basename "$src" .c).o"
        if [ ! -f "$obj" ] || [ "$src" -nt "$obj" ]; then
            compile_source "$arch" "$src" "$obj"
        fi
        kernel_objs="$kernel_objs $obj"
    done
    
    # Compile library sources
    info "  Compiling libraries..."
    for src in $LIB_SOURCES; do
        local ext="${src##*.}"
        local obj="$OBJ_PREFIX/$(basename "$src" .$ext).o"
        if [ ! -f "$obj" ] || [ "$src" -nt "$obj" ]; then
            compile_source "$arch" "$src" "$obj"
        fi
        kernel_objs="$kernel_objs $obj"
    done
    
    # Link kernel
    info "  Linking kernel..."
    local link_script="$PROJECT_DIR/kernel/$arch/link.ld"
    
    # Get libgcc path
    local libgcc
    if [ "$arch" = "x86" ]; then
        libgcc=$($CC -print-libgcc-file-name)
    else
        libgcc=$($CC -print-libgcc-file-name)
    fi
    
    $LD -T "$link_script" -o "$KERNEL_ELF" $kernel_objs $boot_obj $libgcc
    
    # Convert to binary
    info "  Creating kernel binary..."
    $OBJCOPY -O binary "$KERNEL_ELF" "$KERNEL_BIN"
    
    info "  Build complete: $KERNEL_BIN"
}

# Create GRUB ISO
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
    
    # Copy kernel to ISO directory
    mkdir -p "$ISO_DIR/boot/nebulaos"
    cp "$KERNEL_FILE" "$ISO_DIR/boot/nebulaos/nebulaos_${arch}.elf"
    
    # Create GRUB configuration with timeout
    cat > "$ISO_DIR/boot/grub/grub.cfg" <<EOF
set timeout=5
set default=0

menuentry "NebulaOS $arch" {
    multiboot2 /boot/nebulaos/nebulaos_${arch}.elf
    boot
}
EOF
    
    # Create GRUB core image
    local GRUB_CORE="$ISO_DIR/boot/grub/i386-pc/core.img"
    mkdir -p "$(dirname "$GRUB_CORE")"
    
    grub-mkimage -O i386-pc-pxe \
        -o "$GRUB_CORE" \
        -p /boot/grub \
        -d /usr/lib/grub/i386-pc \
        normal configfile multiboot2 linux
    
    # Create ISO image
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

# Run in QEMU
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

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

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
