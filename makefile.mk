.DEFAULT_GOAL := all
.ONESHELL:
SHELL := sh
.SHELLFLAGS := -ec

ROOT := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
BUILD_DIR := $(ROOT)/build
ISO_DIR := $(BUILD_DIR)/iso
ISO_FILE := $(BUILD_DIR)/nebulaos_x86.iso
OBJCOPY ?= objcopy
OVMF_CODE ?=
UEFI_DIR := $(BUILD_DIR)/uefi
UEFI_APP := $(UEFI_DIR)/EFI/BOOT/BOOTX64.EFI

.PHONY: all help x86 x86_64 iso uefi test smoke-x86 smoke-uefi run run-elf run-iso run-uefi clean

all: x86

help:
	@printf '%s\n' \
	  'make -f makefile.mk [target]' \
	  '  x86       Build the verified 32-bit kernel (default)' \
	  '  x86_64    Build the 64-bit kernel' \
	  '  iso       Build the x86 NebulaBoot ISO' \
	  '  uefi      Build the x86_64 UEFI application and FAT staging directory' \
	  '  test      Run common crate unit tests' \
	  '  smoke-x86 Build the BIOS ISO and verify kernel serial output in QEMU' \
	  '  smoke-uefi Boot UEFI staging directory with OVMF and verify serial output' \
	  '  run       Boot the x86 BIOS ISO in QEMU' \
	  '  run-elf   Boot the x86 ELF directly in QEMU (no BIOS handoff)' \
	  '  run-iso   Boot the x86 ISO in QEMU' \
	  '  run-uefi  Boot the x86_64 UEFI staging directory (set OVMF_CODE)' \
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

uefi: x86_64
	mkdir -p "$(UEFI_DIR)/EFI/BOOT" "$(UEFI_DIR)/EFI/NEBULA"
	cargo +nightly build --manifest-path "$(ROOT)/boot/nebula_boot/x86_64/uefi_loader/Cargo.toml" --target x86_64-unknown-uefi --release
	cp "$(ROOT)/target/x86_64-unknown-uefi/release/nebula_uefi_loader.efi" "$(UEFI_APP)"
	cp "$(BUILD_DIR)/nebulaos_x86_64.elf" "$(UEFI_DIR)/EFI/NEBULA/nebulaos_x86_64.elf"

test:
	cargo +nightly test --manifest-path "$(ROOT)/Cargo.toml" -p common

smoke-x86: iso
	if command -v powershell >/dev/null 2>&1; then
	    powershell -NoProfile -ExecutionPolicy Bypass -File "$(ROOT)/scripts/smoke-x86.ps1" -IsoPath "$(ISO_FILE)"
	elif command -v pwsh >/dev/null 2>&1; then
	    pwsh -NoProfile -File "$(ROOT)/scripts/smoke-x86.ps1" -IsoPath "$(ISO_FILE)"
	else
	    printf 'PowerShell is required by the BIOS smoke test.\n' >&2
	    exit 1
	fi

smoke-uefi: uefi
	if [ -z "$(OVMF_CODE)" ] || [ ! -f "$(OVMF_CODE)" ]; then
	    printf 'Set OVMF_CODE to an OVMF_CODE.fd firmware image.\n' >&2
	    exit 1
	fi
	if command -v powershell >/dev/null 2>&1; then
	    powershell -NoProfile -ExecutionPolicy Bypass -File "$(ROOT)/scripts/smoke-uefi.ps1" -UefiDirectory "$(UEFI_DIR)" -FirmwarePath "$(OVMF_CODE)"
	elif command -v pwsh >/dev/null 2>&1; then
	    pwsh -NoProfile -File "$(ROOT)/scripts/smoke-uefi.ps1" -UefiDirectory "$(UEFI_DIR)" -FirmwarePath "$(OVMF_CODE)"
	else
	    printf 'PowerShell is required by the UEFI smoke test.\n' >&2
	    exit 1
	fi

iso: x86
	mkdir -p "$(ISO_DIR)/boot/nebulaos"
	cp "$(BUILD_DIR)/nebulaos_x86.elf" "$(ISO_DIR)/boot/nebulaos/nebulaos_x86.elf"
	nasm -f bin "$(ROOT)/boot/nebula_boot/x86/boot.asm" -o "$(BUILD_DIR)/nebula_boot_x86.bin"
	cat "$(BUILD_DIR)/nebula_boot_x86.bin" "$(BUILD_DIR)/nebulaos_x86.bin" > "$(BUILD_DIR)/boot_image_x86.bin"
	cp "$(BUILD_DIR)/boot_image_x86.bin" "$(ISO_DIR)/boot/nebulaos/nebula_boot_x86.bin"
	rm -f "$(ISO_FILE)"
	xorriso -as mkisofs -R -b boot/nebulaos/nebula_boot_x86.bin -no-emul-boot -boot-load-size 16 -boot-info-table -o "$(ISO_FILE)" "$(ISO_DIR)"
	printf 'Created %s\n' "$(ISO_FILE)"

run-elf: x86
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

run: run-iso

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
	"$$qemu" -cdrom "$(ISO_FILE)" -m 256M -serial stdio

run-uefi: uefi
	if [ -z "$(OVMF_CODE)" ] || [ ! -f "$(OVMF_CODE)" ]; then
	    printf 'Set OVMF_CODE to an OVMF_CODE.fd firmware image.\n' >&2
	    exit 1
	fi
	qemu="qemu-system-x86_64"
	if ! command -v "$$qemu" >/dev/null 2>&1; then
	    if [ -x "/c/Program Files/qemu/qemu-system-x86_64.exe" ]; then
	        qemu="/c/Program Files/qemu/qemu-system-x86_64.exe"
	    else
	        printf 'qemu-system-x86_64 was not found on PATH or in Program Files/qemu.\n' >&2
	        exit 1
	    fi
	fi
	"$$qemu" -bios "$(OVMF_CODE)" -drive "format=raw,file=fat:rw:$(UEFI_DIR)" -m 256M -serial stdio

clean:
	rm -rf "$(BUILD_DIR)"
	printf 'Cleaned %s\n' "$(BUILD_DIR)"