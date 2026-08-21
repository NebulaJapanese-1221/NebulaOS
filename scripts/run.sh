#!/bin/bash
# NebulaOS QEMU Launch Script
# =============================
#
# Launches NebulaOS in QEMU with configurable options

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/build"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Default values
ARCH="x86_64"
USE_ISO=false
DEBUG=false
GDB=false
RAM=256M
BIOS=""
EXTRA_ARGS=""

# -----------------------------------------------------------------------------
# Parse arguments
# -----------------------------------------------------------------------------

while [ $# -gt 0 ]; do
    case "$1" in
        --arch=*)
            ARCH="${1#*=}"
            shift
            ;;
        --arch)
            ARCH="$2"
            shift 2
            ;;
        --iso)
            USE_ISO=true
            shift
            ;;
        --debug)
            DEBUG=true
            shift
            ;;
        --gdb)
            GDB=true
            shift
            ;;
        --ram=*)
            RAM="${1#*=}"
            shift
            ;;
        --bios=*)
            BIOS="${1#*=}"
            shift
            ;;
        --help)
            echo "NebulaOS QEMU Launch Script"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --arch=ARCH       Set architecture (x86 or x86_64, default: x86_64)"
            echo "  --iso             Run from ISO image instead of kernel binary"
            echo "  --debug           Enable debug output"
            echo "  --gdb             Start GDB stub and wait for connection"
            echo "  --ram=SIZE        Set RAM size (default: 256M)"
            echo "  --bios=PATH       Path to custom BIOS firmware"
            echo "  --help            Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                           Run x86_64 kernel"
            echo "  $0 --arch=x86                Run x86 kernel"
            echo "  $0 --iso                     Run from ISO"
            echo "  $0 --gdb                     Debug with GDB"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Validate architecture
if [ "$ARCH" != "x86" ] && [ "$ARCH" != "x86_64" ]; then
    error "Invalid architecture: $ARCH"
    error "Use x86 or x86_64"
    exit 1
fi

# -----------------------------------------------------------------------------
# Determine QEMU binary
# -----------------------------------------------------------------------------

if [ "$ARCH" = "x86" ]; then
    QEMU_BIN="qemu-system-i386"
    KERNEL_BIN="$BUILD_DIR/nebulaos_x86.bin"
    ISO_FILE="$BUILD_DIR/iso/nebulaos_x86.iso"
else
    QEMU_BIN="qemu-system-x86_64"
    KERNEL_BIN="$BUILD_DIR/nebulaos_x86_64.bin"
    ISO_FILE="$BUILD_DIR/iso/nebulaos_x86_64.iso"
fi

# Check if QEMU is installed
if ! command -v "$QEMU_BIN" &> /dev/null; then
    error "QEMU binary not found: $QEMU_BIN"
    error "Please install QEMU first: sudo apt install qemu-system-x86 qemu-system-x86_64"
    exit 1
fi

# -----------------------------------------------------------------------------
# Determine boot mode
# -----------------------------------------------------------------------------

if [ "$USE_ISO" = true ]; then
    BOOT_FILE="$ISO_FILE"
    BOOT_MODE="-cdrom"
    info "Booting from ISO: $BOOT_FILE"
else
    BOOT_FILE="$KERNEL_BIN"
    BOOT_MODE="-kernel"
    info "Booting kernel: $BOOT_FILE"
fi

# Check if boot file exists
if [ ! -f "$BOOT_FILE" ]; then
    error "Boot file not found: $BOOT_FILE"
    error "Please build the kernel first: make ARCH=$ARCH"
    exit 1
fi

# -----------------------------------------------------------------------------
# Build QEMU arguments
# -----------------------------------------------------------------------------

QEMU_ARGS=(
    "$BOOT_MODE" "$BOOT_FILE"
    -m "$RAM"
    -serial stdio
)

# Add BIOS if specified
if [ -n "$BIOS" ]; then
    QEMU_ARGS+=(-bios "$BIOS")
fi

# Add debug options
if [ "$DEBUG" = true ]; then
    QEMU_ARGS+=(-d int,cpu_reset,guest_errors)
    QEMU_ARGS+=(-D "$BUILD_DIR/qemu_debug.log")
fi

# Add GDB stub
if [ "$GDB" = true ]; then
    QEMU_ARGS+=(-s -S)
    info "GDB stub listening on localhost:1234"
fi

# Architecture-specific options
if [ "$ARCH" = "x86_64" ]; then
    QEMU_ARGS+=(-cpu qemu64)
else
    QEMU_ARGS+=(-cpu qemu32)
fi

# Add any extra arguments
QEMU_ARGS+=($EXTRA_ARGS)

# -----------------------------------------------------------------------------
# Launch QEMU
# -----------------------------------------------------------------------------

info "Launching QEMU..."
info "  Architecture: $ARCH"
info "  RAM: $RAM"
info "  Serial: stdio"

exec "$QEMU_BIN" "${QEMU_ARGS[@]}"
