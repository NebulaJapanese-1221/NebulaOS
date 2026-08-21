#!/bin/bash
# NebulaOS Cross-Compiler Toolchain Install Script
# =================================================
#
# Sets up the cross-compilation environment for NebulaOS
# Supports Ubuntu/Debian, Fedora, and Arch Linux

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

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

# -----------------------------------------------------------------------------
# Detect OS
# -----------------------------------------------------------------------------

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        VERSION=$VERSION_ID
    elif [ -f /etc/lsb-release ]; then
        . /etc/lsb-release
        OS=$DISTRIB_ID
        VERSION=$DISTRIB_RELEASE
    else
        OS=$(uname -s)
        VERSION=$(uname -r)
    fi

    OS=$(echo "$OS" | tr '[:upper:]' '[:lower:]')
    info "Detected OS: $OS $VERSION"
}

# -----------------------------------------------------------------------------
# Install dependencies
# -----------------------------------------------------------------------------

install_dependencies() {
    case "$OS" in
        ubuntu|debian|linuxmint)
            info "Installing dependencies for Debian/Ubuntu..."
            sudo apt-get update
            sudo apt-get install -y \
                gcc-multilib \
                g++-multilib \
                nasm \
                xorriso \
                qemu-system-x86 \
                qemu-system-x86_64 \
                build-essential \
                git \
                curl \
                wget
            ;;
        fedora|rhel|centos)
            info "Installing dependencies for Fedora/RHEL..."
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
                git \
                curl \
                wget
            ;;
        arch|manjaro)
            info "Installing dependencies for Arch Linux..."
            sudo pacman -Syu --noconfirm \
                gcc \
                make \
                nasm \
                xorriso \
                qemu-system-x86 \
                git \
                curl \
                wget
            ;;
        *)
            error "Unsupported OS: $OS"
            error "Please install the following packages manually:"
            error "  - gcc-multilib (or equivalent 32-bit support)"
            error "  - g++-multilib"
            error "  - nasm"
            error "  - xorriso"
            error "  - qemu-system-x86"
            error "  - qemu-system-x86_64"
            exit 1
            ;;
    esac
}

# -----------------------------------------------------------------------------
# Install cross-compilers
# -----------------------------------------------------------------------------

install_cross_compilers() {
    info "Installing cross-compilers..."

    case "$OS" in
        ubuntu|debian|linuxmint)
            sudo apt-get install -y \
                gcc-i686-linux-gnu \
                g++-i686-linux-gnu \
                gcc-x86-64-linux-gnu \
                g++-x86-64-linux-gnu \
                binutils-i686-linux-gnu \
                binutils-x86-64-linux-gnu
            ;;
        fedora|rhel|centos)
            sudo dnf install -y \
                gcc-i686-redhat-linux \
                gcc-x86_64-redhat-linux \
                binutils-i686-redhat-linux \
                binutils-x86_64-redhat-linux
            ;;
        arch|manjaro)
            sudo pacman -S --noconfirm \
                multilib-devel
            ;;
    esac
}

# -----------------------------------------------------------------------------
# Create build directories
# -----------------------------------------------------------------------------

create_build_dirs() {
    info "Creating build directories..."
    mkdir -p "$PROJECT_DIR/build/x86"
    mkdir -p "$PROJECT_DIR/build/x86_64"
    mkdir -p "$PROJECT_DIR/build/iso"
}

# -----------------------------------------------------------------------------
# Test cross-compilers
# -----------------------------------------------------------------------------

test_cross_compilers() {
    info "Testing cross-compilers..."

    local all_ok=true

    # Test i686 cross-compiler
    if command -v i686-linux-gnu-gcc &> /dev/null; then
        info "  i686-linux-gnu-gcc: OK"
    else
        warn "  i686-linux-gnu-gcc: NOT FOUND"
        all_ok=false
    fi

    # Test x86_64 cross-compiler
    if command -v x86_64-linux-gnu-gcc &> /dev/null; then
        info "  x86_64-linux-gnu-gcc: OK"
    else
        warn "  x86_64-linux-gnu-gcc: NOT FOUND"
        all_ok=false
    fi

    # Test nasm
    if command -v nasm &> /dev/null; then
        info "  nasm: OK ($(nasm -v | head -n1))"
    else
        warn "  nasm: NOT FOUND"
        all_ok=false
    fi

    # Test xorriso
    if command -v xorriso &> /dev/null; then
        info "  xorriso: OK"
    else
        warn "  xorriso: NOT FOUND"
        all_ok=false
    fi

    # Test QEMU
    if command -v qemu-system-i386 &> /dev/null; then
        info "  qemu-system-i386: OK"
    else
        warn "  qemu-system-i386: NOT FOUND"
        all_ok=false
    fi

    if command -v qemu-system-x86_64 &> /dev/null; then
        info "  qemu-system-x86_64: OK"
    else
        warn "  qemu-system-x86_64: NOT FOUND"
        all_ok=false
    fi

    if [ "$all_ok" = true ]; then
        info "All cross-compilers installed successfully!"
        return 0
    else
        warn "Some tools are missing. The build may fail."
        return 1
    fi
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    info "NebulaOS Cross-Compiler Toolchain Setup"
    info "========================================"
    echo ""

    detect_os
    echo ""

    install_dependencies
    echo ""

    install_cross_compilers
    echo ""

    create_build_dirs
    echo ""

    test_cross_compilers
    echo ""

    info "Toolchain setup complete!"
    info "You can now build NebulaOS with: make"
}

main "$@"
