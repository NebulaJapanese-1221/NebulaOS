.DEFAULT_GOAL := all
.ONESHELL:
SHELL := sh
.SHELLFLAGS := -ec

ROOT := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
BUILD_DIR := $(ROOT)/build
ISO_DIR := $(BUILD_DIR)/iso
OBJCOPY ?= objcopy

.PHONY: all help x86 x86_64 iso run run-iso clean

all: x86

help:
	@printf '%s\n' \
	  'make -f makefile.mk [target]' \
	  '  x86       Build the verified 32-bit kernel (default)' \
	  '  x86_64    Build the 64-bit kernel' \
	  '  iso       Build the x86 NebulaBoot ISO' \
	  '  run       Boot the x86 ELF directly in QEMU' \
	  '  run-iso   Boot the x86 ISO in QEMU' \
	  '  clean     Remove generated build files'

define build_kernel
mkdir -p "$(BUILD_DIR)"
cargo +nightly -Zjson-target-spec -Zbuild-std=core build --manifest-path "$(ROOT)/kernel/Cargo.toml" --target "$(ROOT)/targets/$(ARCH).json" --release
artifact="$(ROOT)/target/$(ARCH)/release/kernel"
if [ ! -f "$$artifact" ]; then artifact="$$artifact.exe"; fi
if [ ! -f "$$artifact" ]; then
    printf 'Kernel artifact not found: %s\n' "$$artifact" >&2
    exit 1
fi
cp "$$artifact" "$(BUILD_DIR)/nebulaos_$(ARCH).elf"
"$(OBJCOPY)" -O binary "$(BUILD_DIR)/nebulaos_$(ARCH).elf" "$(BUILD_DIR)/nebulaos_$(ARCH).bin"
printf 'Built %s\n' "$(BUILD_DIR)/nebulaos_$(ARCH).bin"
endef

x86: ARCH := x86
x86_64: ARCH := x86_64
x86 x86_64:
	$(build_kernel)

iso: x86
	mkdir -p "$(ISO_DIR)/boot/nebulaos"
	cp "$(BUILD_DIR)/nebulaos_x86.elf" "$(ISO_DIR)/boot/nebulaos/nebulaos_x86.elf"
	nasm -f bin "$(ROOT)/boot/nebula_boot/x86/boot.asm" -o "$(BUILD_DIR)/nebula_boot_x86.bin"
	cat "$(BUILD_DIR)/nebula_boot_x86.bin" "$(BUILD_DIR)/nebulaos_x86.bin" > "$(BUILD_DIR)/boot_image_x86.bin"
	cp "$(BUILD_DIR)/boot_image_x86.bin" "$(ISO_DIR)/boot/nebulaos/nebula_boot_x86.bin"
	xorriso -as mkisofs -R -b boot/nebulaos/nebula_boot_x86.bin -no-emul-boot -boot-load-size 16 -boot-info-table -o "$(ISO_DIR)/nebulaos_x86.iso" "$(ISO_DIR)"
	printf 'Created %s\n' "$(ISO_DIR)/nebulaos_x86.iso"

run: x86
	qemu="qemu-system-i386"
	if ! command -v "$$qemu" >/dev/null 2>&1; then
	    if [ -x "/c/Program Files/qemu/qemu-system-i386.exe" ]; then
	        qemu="/c/Program Files/qemu/qemu-system-i386.exe"
	    else
	        printf 'qemu-system-i386 was not found on PATH or in Program Files/qemu.\n' >&2
	        exit 1
	    fi
	fi
	"$$qemu" -kernel "$(BUILD_DIR)/nebulaos_x86.elf" -m 256M -serial stdio

run-iso: iso
	qemu="qemu-system-i386"
	if ! command -v "$$qemu" >/dev/null 2>&1; then
	    if [ -x "/c/Program Files/qemu/qemu-system-i386.exe" ]; then
	        qemu="/c/Program Files/qemu/qemu-system-i386.exe"
	    else
	        printf 'qemu-system-i386 was not found on PATH or in Program Files/qemu.\n' >&2
	        exit 1
	    fi
	fi
	"$$qemu" -cdrom "$(ISO_DIR)/nebulaos_x86.iso" -m 256M -serial stdio

clean:
	rm -rf "$(BUILD_DIR)"
	printf 'Cleaned %s\n' "$(BUILD_DIR)"