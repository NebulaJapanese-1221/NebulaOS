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
# MERCHANTABILITY OR FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

.PHONY: all clean run iso debug data-disk

CARGO ?= cargo
NASM ?= nasm
LD ?= ld
OBJCOPY ?= objcopy
QEMU_X86 ?= qemu-system-i386
# Auto-detect grub-mkrescue binary name (Fedora uses grub2-mkrescue, others use grub-mkrescue)
GRUB_MKRESCUE := $(shell if command -v grub-mkrescue >/dev/null 2>&1; then echo grub-mkrescue; elif command -v grub2-mkrescue >/dev/null 2>&1; then echo grub2-mkrescue; else echo grub-mkrescue; fi)
XORRISO ?= xorriso

TARGET := i686-unknown-nebula
QEMU := $(QEMU_X86)
KERNEL_IMG := boot/kernel.elf
ISO := nebulaos.iso
LD_ARCH_FLAG := -m elf_i386
BOOT_OBJ := kernel/arch/isr.o kernel/arch/ap_trampoline.o

KERNEL_LIB := target/$(TARGET)/release/libnebulaos.a
USERSPACE_LIB := userspace_standalone/target/$(TARGET)/release/libnebulaos_userspace.a
USERSPACE_ELF := userspace_standalone/target/$(TARGET)/release/userspace
USERSPACE_BIN := userspace_standalone/target/$(TARGET)/release/userspace.bin
DISK := nebulaos.disk

export RUSTC_TARGET_PATH := targets

all: $(ISO)

# ---- Userspace ELF ----
$(USERSPACE_LIB): userspace_standalone/Cargo.toml userspace_standalone/Cargo.lock $(shell find userspace_standalone -type f -name '*.rs')
	cd userspace_standalone && $(CARGO) build --release --features user-app

$(USERSPACE_BIN): $(USERSPACE_LIB)
	$(LD) $(LD_ARCH_FLAG) -T linker.ld -o $(USERSPACE_ELF) $(USERSPACE_LIB) --whole-archive $(USERSPACE_LIB) --no-whole-archive
	$(OBJCOPY) -O binary $(USERSPACE_ELF) $@

# ---- Kernel library ----
$(KERNEL_LIB): Cargo.toml Cargo.lock .cargo/config.toml targets/$(TARGET).json $(shell find kernel -type f -name '*.rs') $(USERSPACE_BIN)
	$(CARGO) build --release

# ---- Boot objects ----
$(BOOT_OBJ): kernel/arch/isr.asm kernel/arch/ap_trampoline.asm
	$(NASM) -f elf32 kernel/arch/isr.asm -o kernel/arch/isr.o
	$(NASM) -f elf32 kernel/arch/ap_trampoline.asm -o kernel/arch/ap_trampoline.o

# ---- Kernel ELF ----
$(KERNEL_IMG): $(KERNEL_LIB) $(BOOT_OBJ) linker.ld $(USERSPACE_BIN)
	$(LD) $(LD_ARCH_FLAG) -T linker.ld -o $(KERNEL_IMG) $(BOOT_OBJ) $(KERNEL_LIB) --whole-archive $(KERNEL_LIB) --no-whole-archive
	$(OBJCOPY) --add-section .user_app=$(USERSPACE_BIN) --change-section-vma .user_app=0x40000000 --set-section-flags .user_app=contents,alloc,load,readonly $@ $@

# ---- ISO ----
$(ISO): $(KERNEL_IMG) Makefile
	mkdir -p boot/iso/boot/grub
	cp $(KERNEL_IMG) boot/iso/boot/kernel.elf
	@printf 'serial --unit=0 --speed=115200 --word=8 --parity=no --stop=1\nset gfxmode=1024x768x32;auto\nset gfxpayload=keep\ninsmod vbe\ninsmod gfxterm\nterminal_input console serial\nterminal_output gfxterm serial\nset timeout=5\nset timeout_style=menu\nset default=0\n\nmenuentry "NebulaOS" {\n    multiboot2 /boot/kernel.elf\n}\n' > boot/iso/boot/grub/grub.cfg
	$(GRUB_MKRESCUE) -o $(ISO) boot/iso

# ---- Run targets ----
run: $(ISO)
	@if [ -f "$(DISK)" ]; then \
		$(QEMU) -cdrom $(ISO) -drive file=$(DISK),format=raw,if=ide -m 256 -vga std -serial stdio; \
	else \
		$(QEMU) -cdrom $(ISO) -m 256 -vga std -serial stdio; \
	fi

debug: $(ISO)
	$(QEMU_X86) -cdrom $(ISO) -m 256 -vga std -serial stdio -s -S &
	sleep 1

data-disk:
	@test ! -e "$(DISK)" || { echo "$(DISK) already exists; refusing to overwrite it"; exit 1; }
	dd if=/dev/zero of=$(DISK) bs=1M count=1 status=none
	@echo "Created blank IDE data disk: $(DISK). NebulaOS will offer to format it on first boot."

clean:
	$(CARGO) clean
	cd userspace_standalone && $(CARGO) clean
	rm -rf boot/iso $(KERNEL_IMG) $(BOOT_OBJ) $(USERSPACE_LIB) $(USERSPACE_BIN) nebulaos.img $(ISO)