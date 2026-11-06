# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
#
# This file is part of NebulaOS.
#
# NebulaOS is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License.
#
# NebulaOS is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

.PHONY: all clean run iso debug data-disk run-arm64 debug-arm64

CARGO ?= cargo
NASM ?= nasm
AS ?= as
LD ?= ld
OBJCOPY ?= objcopy
QEMU_X86 ?= qemu-system-i386
QEMU_ARM64 ?= qemu-system-aarch64
GRUB_MKRESCUE ?= grub-mkrescue
XORRISO ?= xorriso

# Architecture selector: i386 (default) or arm64
ARCH ?= i386

ifeq ($(ARCH),arm64)
	TARGET := aarch64-unknown-nebula
	QEMU := $(QEMU_ARM64)
	KERNEL_IMG := boot/kernel-aarch64.elf
	ISO := nebulaos-arm64.iso
	KERNEL_CARGO_TARGET_FLAGS := --target targets/$(TARGET).json
	USER_CARGO_TARGET_FLAGS := --target ../../targets/$(TARGET).json
	LD_ARCH_FLAG := -m aarch64linux
	BOOT_OBJ := kernel/arch/aarch64/boot.o
	USER_BUILD_DIR := target/user-demo-arm64
	USER_ELF_OBJ := $(USER_BUILD_DIR)/demo_elf.o
	USER_OBJCOPY_FMT := elf64-littleaarch64
	USER_OBJCOPY_ARCH := aarch64
else
	TARGET := i686-unknown-nebula
	QEMU := $(QEMU_X86)
	KERNEL_IMG := boot/kernel.elf
	ISO := nebulaos.iso
	CARGO_TARGET_FLAGS :=
	KERNEL_CARGO_TARGET_FLAGS :=
	USER_CARGO_TARGET_FLAGS :=
	LD_ARCH_FLAG := -m elf_i386
	BOOT_OBJ := kernel/arch/isr.o kernel/arch/ap_trampoline.o
	USER_BUILD_DIR := target/user-demo-i386
	USER_ELF_OBJ := $(USER_BUILD_DIR)/demo_elf.o
	USER_OBJCOPY_FMT := elf32-i386
	USER_OBJCOPY_ARCH := i386
endif

KERNEL_LIB := target/$(TARGET)/release/libnebulaos.a
USER_ELF := $(USER_BUILD_DIR)/demo.elf
USER_APP_RS := $(shell find userspace/apps -type f -name '*.rs')
USER_CARGO_TARGET := target/user-demo-$(ARCH)/cargo
USER_CARGO_ELF := $(USER_CARGO_TARGET)/$(TARGET)/release/nebulaos-user-demo
DISK := nebulaos.disk

export RUSTC_TARGET_PATH := targets

all: $(ISO)

# ---- Kernel library ----
$(KERNEL_LIB): Cargo.toml Cargo.lock .cargo/config.toml targets/$(TARGET).json $(shell find kernel userspace -type f -name '*.rs')
ifeq ($(ARCH),arm64)
	RUSTFLAGS="-C link-arg=-Tlinker.ld -C link-arg=-maarch64linux" $(CARGO) build --release $(KERNEL_CARGO_TARGET_FLAGS)
else
	$(CARGO) build --release $(KERNEL_CARGO_TARGET_FLAGS)
endif

# ---- Boot objects ----
ifeq ($(ARCH),i386)
$(BOOT_OBJ): kernel/arch/isr.asm kernel/arch/ap_trampoline.asm
	$(NASM) -f elf32 kernel/arch/isr.asm -o kernel/arch/isr.o
	$(NASM) -f elf32 kernel/arch/ap_trampoline.asm -o kernel/arch/ap_trampoline.o
else ifeq ($(ARCH),arm64)
$(BOOT_OBJ): kernel/arch/aarch64/boot.S
	$(AS) -aarch64 -c kernel/arch/aarch64/boot.S -o $(BOOT_OBJ)
endif

# ---- Userspace demo ELF ----
$(USER_ELF): userspace/demo/Cargo.toml userspace/demo/src/main.rs userspace/demo/linker.ld userspace/mod.rs userspace/syscall.rs userspace/storage.rs $(USER_APP_RS)
	cd userspace/demo && $(CARGO) build --release $(USER_CARGO_TARGET_FLAGS) --target-dir ../../$(USER_CARGO_TARGET)
	$(OBJCOPY) --strip-all $(USER_CARGO_ELF) $(USER_ELF)

$(USER_ELF_OBJ): $(USER_ELF)
	cd $(USER_BUILD_DIR) && $(OBJCOPY) -I binary -O $(USER_OBJCOPY_FMT) -B $(USER_OBJCOPY_ARCH) demo.elf demo_elf.o

# ---- Kernel ELF ----
$(KERNEL_IMG): $(KERNEL_LIB) $(BOOT_OBJ) $(USER_ELF_OBJ) linker.ld
	$(LD) $(LD_ARCH_FLAG) -T linker.ld -o $(KERNEL_IMG) $(BOOT_OBJ) $(KERNEL_LIB) $(USER_ELF_OBJ)

# ---- ISO ----
$(ISO): $(KERNEL_IMG) Makefile
	mkdir -p boot/iso/boot/grub
	cp $(KERNEL_IMG) boot/iso/boot/kernel.elf
ifeq ($(ARCH),arm64)
	@printf 'set timeout=5\nset default=0\n\nmenuentry "NebulaOS arm64" {\n    echo "Loading NebulaOS (aarch64)..."\n    multiboot2 /boot/kernel.elf\n}\n' > boot/iso/boot/grub/grub.cfg
else
	@printf 'serial --unit=0 --speed=115200 --word=8 --parity=no --stop=1\nset gfxmode=1024x768x32;auto\nset gfxpayload=keep\ninsmod vbe\ninsmod gfxterm\nterminal_input console serial\nterminal_output gfxterm serial\nset timeout=5\nset timeout_style=menu\nset default=0\n\nmenuentry "NebulaOS" {\n    multiboot2 /boot/kernel.elf\n}\n' > boot/iso/boot/grub/grub.cfg
endif
	$(GRUB_MKRESCUE) -o $(ISO) boot/iso

# ---- Run targets ----
run: $(ISO)
	@if [ -f "$(DISK)" ]; then \
		$(QEMU) -cdrom $(ISO) -drive file=$(DISK),format=raw,if=ide -m 256 -vga std -serial stdio; \
	else \
		$(QEMU) -cdrom $(ISO) -m 256 -vga std -serial stdio; \
	fi

run-arm64: $(KERNEL_IMG)
	$(QEMU_ARM64) -machine virt -cpu cortex-a72 -m 512 -kernel $(KERNEL_IMG) -nographic -serial stdio

debug: $(ISO)
	$(QEMU_X86) -cdrom $(ISO) -m 256 -vga std -serial stdio -s -S &
	sleep 1

debug-arm64: $(KERNEL_IMG)
	$(QEMU_ARM64) -machine virt -cpu cortex-a72 -m 512 -kernel $(KERNEL_IMG) -nographic -serial stdio -s -S &
	sleep 1

data-disk:
	@test ! -e "$(DISK)" || { echo "$(DISK) already exists; refusing to overwrite it"; exit 1; }
	dd if=/dev/zero of=$(DISK) bs=1M count=1 status=none
	@echo "Created blank IDE data disk: $(DISK). NebulaOS will offer to format it on first boot."

clean:
	$(CARGO) clean
	rm -rf boot/iso $(KERNEL_IMG) $(BOOT_OBJ) $(USER_ELF_OBJ) nebulaos.img $(ISO) nebulaos-arm64.iso
