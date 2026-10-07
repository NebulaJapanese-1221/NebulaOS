#!/bin/bash
# Build userspace programs for NebulaOS
# Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>

set -e

# Detect platform and set toolchain.
OS="$(uname -s)"
case "$OS" in
    MINGW*|MSYS*|CYGWIN*)
        # Windows: prefer gcc from MinGW, nasm from PATH.
        CC="${CC:-gcc}"
        LD="${LD:-ld}"
        NASM="${NASM:-nasm}"
        CPIO="${CPIO:-cpio}"
        ;;
    *)
        CC="${CC:-gcc}"
        LD="${LD:-ld}"
        NASM="${NASM:-nasm}"
        CPIO="${CPIO:-cpio}"
        ;;
esac

CFLAGS="-m32 -std=gnu11 -ffreestanding -fno-stack-protector -nostdlib -nostdinc -Wall -Wextra -O2 -I./lib"
LDFLAGS="-m elf_i386 -T ./linker.ld -nostdlib"

OUT_DIR="../initrd"

mkdir -p "$OUT_DIR"

echo "Building userspace crt0 assembly..."
$NASM -f elf32 ./lib/crt0.asm -o ./lib/crt0.o

echo "Building userspace library..."
$CC $CFLAGS -c ./lib/crt0.c -o ./lib/crt0_c.o
$CC $CFLAGS -c ./lib/libc.c -o ./lib/libc.o

echo "Building console..."
$CC $CFLAGS -c ./bin/console.c -o ./bin/console.o
$LD $LDFLAGS ./lib/crt0.o ./lib/crt0_c.o ./lib/libc.o ./bin/console.o -o "$OUT_DIR/console"

echo "Building settings..."
$CC $CFLAGS -c ./bin/settings.c -o ./bin/settings.o
$LD $LDFLAGS ./lib/crt0.o ./lib/crt0_c.o ./lib/libc.o ./bin/settings.o -o "$OUT_DIR/settings"

echo "Userspace build complete!"
ls -la "$OUT_DIR/"