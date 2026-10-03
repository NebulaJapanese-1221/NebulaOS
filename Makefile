CXX ?= g++
LD ?= ld
NASM ?= nasm
GRUB_MKRESCUE ?= grub-mkrescue
QEMU ?= qemu-system-i386

CXXFLAGS := -m32 -std=gnu++17 -ffreestanding -fno-exceptions -fno-rtti \
	-fno-stack-protector -fno-pic -fno-pie -nostdinc++ -Wall -Wextra -O2
LDFLAGS := -m elf_i386 -T kernel/arch/x86/linker.ld -nostdlib

KERNEL_OBJECTS := kernel_entry.o kernel.o drivers/graphics.o drivers/keyboard.o shell.o
ISO_ROOT := build/isodir

.PHONY: all clean run

all: nebulaos.iso

kernel_entry.o: kernel/arch/x86/entry.asm
	$(NASM) -f elf32 $< -o $@

kernel.o: kernel/main.cpp drivers/graphics.hpp drivers/keyboard.hpp shell/shell.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/graphics.o: drivers/graphics.cpp drivers/graphics.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/keyboard.o: drivers/keyboard.cpp drivers/keyboard.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

shell.o: shell/shell.cpp shell/shell.hpp drivers/graphics.hpp drivers/keyboard.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel.elf: $(KERNEL_OBJECTS) kernel/arch/x86/linker.ld
	$(LD) $(LDFLAGS) $(KERNEL_OBJECTS) -o $@

nebulaos.iso: kernel.elf boot/grub/grub.cfg
	mkdir -p $(ISO_ROOT)/boot/grub
	cp kernel.elf $(ISO_ROOT)/boot/kernel.elf
	cp boot/grub/grub.cfg $(ISO_ROOT)/boot/grub/grub.cfg
	$(GRUB_MKRESCUE) -o $@ $(ISO_ROOT)

run: nebulaos.iso
	$(QEMU) -cdrom $< -m 64M

clean:
	rm -f kernel_entry.o kernel.o drivers/graphics.o drivers/keyboard.o shell.o kernel.elf nebulaos.iso
	rm -rf build
