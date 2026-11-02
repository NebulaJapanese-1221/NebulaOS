.PHONY: all clean run iso debug data-disk

CARGO ?= cargo
NASM ?= nasm
LD ?= ld
OBJCOPY ?= objcopy
QEMU ?= qemu-system-i386
GRUB_MKRESCUE ?= grub-mkrescue
XORRISO ?= xorriso

TARGET ?= i686-unknown-nebula
KERNEL_LIB := target/$(TARGET)/release/libnebulaos.a
ISR_OBJ := kernel/arch/isr.o
USER_ELF := target/user-demo/demo.elf
USER_ELF_OBJ := target/user-demo/demo_elf.o
USER_APP_RS := $(shell find userspace/apps -type f -name '*.rs')
USER_CARGO_TARGET := target/user-demo/cargo
USER_CARGO_ELF := $(USER_CARGO_TARGET)/$(TARGET)/release/nebulaos-user-demo
KERNEL_ELF := boot/kernel.elf
ISO := nebulaos.iso
DISK := nebulaos.disk

all: $(ISO)

$(KERNEL_LIB): Cargo.toml Cargo.lock .cargo/config.toml targets/i686-unknown-nebula.json $(shell find kernel userspace -type f -name '*.rs')
	$(CARGO) build --release

$(ISR_OBJ): kernel/arch/isr.asm
	$(NASM) -f elf32 kernel/arch/isr.asm -o $(ISR_OBJ)

$(USER_ELF): userspace/demo/Cargo.toml userspace/demo/src/main.rs userspace/demo/linker.ld userspace/mod.rs userspace/syscall.rs userspace/storage.rs $(USER_APP_RS)
	cd userspace/demo && $(CARGO) build --release --target ../../targets/$(TARGET).json --target-dir ../../$(USER_CARGO_TARGET)
	$(OBJCOPY) --strip-all $(USER_CARGO_ELF) $(USER_ELF)

$(USER_ELF_OBJ): $(USER_ELF)
	cd target/user-demo && $(OBJCOPY) -I binary -O elf32-i386 -B i386 demo.elf demo_elf.o

$(KERNEL_ELF): $(KERNEL_LIB) $(ISR_OBJ) $(USER_ELF_OBJ) linker.ld
	$(LD) -m elf_i386 -T linker.ld -o $(KERNEL_ELF) $(ISR_OBJ) $(KERNEL_LIB) $(USER_ELF_OBJ)

$(ISO): $(KERNEL_ELF) Makefile
	mkdir -p boot/iso/boot/grub
	cp $(KERNEL_ELF) boot/iso/boot/kernel.elf
	@printf 'serial --unit=0 --speed=115200 --word=8 --parity=no --stop=1\nset gfxmode=1024x768x32;auto\nset gfxpayload=keep\ninsmod vbe\ninsmod gfxterm\nterminal_input console serial\nterminal_output gfxterm serial\nset timeout=5\nset timeout_style=menu\nset default=0\n\nmenuentry "NebulaOS" {\n    multiboot2 /boot/kernel.elf\n}\n' > boot/iso/boot/grub/grub.cfg
	$(GRUB_MKRESCUE) -o $(ISO) boot/iso

run: $(ISO)
	@if [ -f "$(DISK)" ]; then \
		$(QEMU) -cdrom $(ISO) -drive file=$(DISK),format=raw,if=ide -m 256 -vga std -serial stdio; \
	else \
		$(QEMU) -cdrom $(ISO) -m 256 -vga std -serial stdio; \
	fi

data-disk:
	@test ! -e "$(DISK)" || { echo "$(DISK) already exists; refusing to overwrite it"; exit 1; }
	dd if=/dev/zero of=$(DISK) bs=1M count=1 status=none
	@echo "Created blank IDE data disk: $(DISK). NebulaOS will offer to format it on first boot."

debug: $(ISO)
	$(QEMU) -cdrom $(ISO) -m 256 -vga std -serial stdio -s -S &
	sleep 1

clean:
	$(CARGO) clean
	rm -rf boot/iso $(KERNEL_ELF) $(ISR_OBJ) nebulaos.img $(ISO)
