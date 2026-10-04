#!/bin/bash
# Build userspace programs for NebulaOS
# Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>

set -e

CC="gcc"
LD="ld"
CFLAGS="-m32 -std=gnu11 -ffreestanding -fno-stack-protector -nostdlib -nostdinc -Wall -Wextra -O2 -I./lib"
LDFLAGS="-m elf_i386 -T ./linker.ld -nostdlib"

OUT_DIR="../initrd"

mkdir -p "$OUT_DIR"

echo "Building userspace library..."
$CC $CFLAGS -c ./lib/crt0.c -o ./lib/crt0.o

echo "Building console..."
$CC $CFLAGS -c ./bin/console.c -o ./bin/console.o
$LD $LDFLAGS ./lib/crt0.o ./bin/console.o -o "$OUT_DIR/console"

echo "Userspace build complete!"
ls -la "$OUT_DIR/"