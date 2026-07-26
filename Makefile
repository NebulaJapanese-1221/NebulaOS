# NebulaOS Build System
# Supports both x86 (32-bit) and x86_64 (64-bit) architectures
#
# Usage:
#   make          - Build for x86 (default)
#   make ARCH=x86_64 - Build for x86_64
#   make run      - Run x86 build (default)
#   make run64    - Run x86_64 build

ARCH ?= x86

ifeq ($(ARCH),x86_64)
TARGET = x86_64-nebula.json
ELFFORMAT = elf64-x86-64
QEMU = qemu-system-x86_64
else
TARGET = i686-nebula.json
ELFFORMAT = elf32-i386
QEMU = qemu-system-x86_64
endif

ARCH_DIR = target/$(basename $(TARGET))
IMAGE = nebula-$(ARCH).iso

BOOTLOADER_ELF = $(ARCH_DIR)/debug/bootloader
KERNEL_ELF = $(ARCH_DIR)/debug/kernel

BOOTLOADER_BIN = $(ARCH_DIR)/debug/bootloader.bin
KERNEL_BIN = $(ARCH_DIR)/debug/kernel.bin

OBJCOPY = llvm-objcopy

ISO_DIR = isodir
BOOT_DIR = $(ISO_DIR)/boot
GRUB_DIR = $(BOOT_DIR)/grub

.PHONY: all clean run run64 build build-x86 build-x86_64

all: build

# Build for specified architecture (default x86)
build:
ifeq ($(ARCH),x86_64)
	RUSTFLAGS="-C link-arg=-Tsrc/kernel/linker.ld" cargo build -Zbuild-std=core,alloc -Zjson-target-spec --bin kernel --target $(TARGET)
else
	RUSTFLAGS="-C link-arg=-Tsrc/boot/linker.ld" cargo build -Zbuild-std=core -Zjson-target-spec --bin bootloader --target $(TARGET)
	RUSTFLAGS="-C link-arg=-Tsrc/kernel/linker.ld" cargo build -Zbuild-std=core,alloc -Zjson-target-spec --bin kernel --target $(TARGET)
endif

.PHONY: all clean run run64 build build-x86 build-x86_64

# Build for x86 (32-bit, backward compatible)
build-x86:
	$(MAKE) ARCH=x86 build

# Build for x86_64 (64-bit)
build-x86_64:
	$(MAKE) ARCH=x86_64 build

$(BOOTLOADER_BIN): build
	$(OBJCOPY) -I $(ELFFORMAT) -O binary $(BOOTLOADER_ELF) $(BOOTLOADER_BIN)

$(KERNEL_BIN): build
	$(OBJCOPY) -I $(ELFFORMAT) -O binary $(KERNEL_ELF) $(KERNEL_BIN)

$(IMAGE): build
	mkdir -p $(GRUB_DIR)
	cp $(KERNEL_ELF) $(BOOT_DIR)/nebula.elf
	@echo 'set timeout=5' > $(GRUB_DIR)/grub.cfg
	@echo 'set default=0' >> $(GRUB_DIR)/grub.cfg
	@echo 'insmod all_video' >> $(GRUB_DIR)/grub.cfg
	@echo 'set gfxpayload=1024x768x32' >> $(GRUB_DIR)/grub.cfg
	@echo '' >> $(GRUB_DIR)/grub.cfg
	@echo 'menuentry "NebulaOS" {' >> $(GRUB_DIR)/grub.cfg
	@echo '    multiboot /boot/nebula.elf' >> $(GRUB_DIR)/grub.cfg
	@echo '    boot' >> $(GRUB_DIR)/grub.cfg
	@echo '}' >> $(GRUB_DIR)/grub.cfg
	grub-mkrescue -o $(IMAGE) $(ISO_DIR)

run: $(IMAGE)
	$(QEMU) -cdrom $(IMAGE) -m 64M -serial stdio -vga vmware

run64:
	$(MAKE) ARCH=x86_64 run

clean:
	cargo clean
	rm -f nebula-*.iso
	rm -rf $(ISO_DIR)

