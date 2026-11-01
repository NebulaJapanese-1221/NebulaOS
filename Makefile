PHONY: all clean run iso debug

CARGO ?= cargo
NASM ?= nasm
LD ?= ld
QEMU ?= qemu-system-i386
GRUB_MKRESCUE ?= grub-mkrescue
XORRISO ?= xorriso

TARGET ?= i686-unknown-nebula
KERNEL_LIB := target/$(TARGET)/release/libnebulaos.a
ISR_OBJ := kernel/arch/isr.o
KERNEL_ELF := boot/kernel.elf
ISO := nebulaos.iso

all: $(ISO)

$(KERNEL_LIB): Cargo.toml Cargo.lock .cargo/config.toml targets/i686-unknown-nebula.json $(shell find kernel -type f -name '*.rs')
	$(CARGO) build --release

$(ISR_OBJ): kernel/arch/isr.asm
	$(NASM) -f elf32 kernel/arch/isr.asm -o $(ISR_OBJ)

$(KERNEL_ELF): $(KERNEL_LIB) $(ISR_OBJ) linker.ld
	$(LD) -m elf_i386 -T linker.ld -o $(KERNEL_ELF) $(ISR_OBJ) $(KERNEL_LIB)

$(ISO): $(KERNEL_ELF) Makefile
	mkdir -p boot/iso/boot/grub
	cp $(KERNEL_ELF) boot/iso/boot/kernel.elf
	@printf 'set timeout=5\nset timeout_style=menu\nset default=0\n\nmenuentry "NebulaOS" {\n    multiboot2 /boot/kernel.elf\n}\n' > boot/iso/boot/grub/grub.cfg
	$(GRUB_MKRESCUE) -o $(ISO) boot/iso

run: $(ISO)
	$(QEMU) -cdrom $(ISO) -m 256 -vga std -serial stdio

debug: $(ISO)
	$(QEMU) -cdrom $(ISO) -m 256 -vga std -serial stdio -s -S &
	sleep 1

clean:
	$(CARGO) clean
	rm -rf boot/iso $(KERNEL_ELF) $(ISR_OBJ) nebulaos.img $(ISO)
