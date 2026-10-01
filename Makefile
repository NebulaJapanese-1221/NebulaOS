CXX ?= i686-elf-g++
LD ?= i686-elf-ld
OBJCOPY ?= i686-elf-objcopy
NASM ?= nasm
QEMU ?= qemu-system-i386

CXXFLAGS := -m32 -ffreestanding -fno-exceptions -fno-rtti -fno-stack-protector -fno-pic -fno-pie -nostdinc++ -Wall -Wextra -O2
BOOT_SOURCE := boot/bios/NebulaBoot.asm
KERNEL_ENTRY_SOURCE := kernel/arch/x86/entry.asm
KERNEL_SOURCE := kernel/main.cpp
LINKER_SCRIPT := kernel/arch/x86/linker.ld
KERNEL_OBJECTS := kernel_entry.o kernel.o drivers/vga.o drivers/keyboard.o shell/shell.o

LDFLAGS := -T $(LINKER_SCRIPT) -nostdlib

.PHONY: all clean run

all: nebulaos.img


boot.bin: $(BOOT_SOURCE)
	$(NASM) -f bin $< -o $@

kernel_entry.o: $(KERNEL_ENTRY_SOURCE)
	$(NASM) -f elf32 $< -o $@

kernel.o: $(KERNEL_SOURCE)
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/vga.o: drivers/vga.cpp drivers/vga.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/keyboard.o: drivers/keyboard.cpp drivers/keyboard.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

shell/shell.o: shell/shell.cpp shell/shell.hpp drivers/keyboard.hpp drivers/vga.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel.elf: $(KERNEL_OBJECTS) $(LINKER_SCRIPT)
	$(LD) $(LDFLAGS) $(KERNEL_OBJECTS) -o $@

kernel.bin: kernel.elf
	$(OBJCOPY) -O binary $< $@

nebulaos.img: boot.bin kernel.bin
	cat boot.bin kernel.bin > $@

run: nebulaos.img
	$(QEMU) -drive format=raw,file=$< -m 64M

clean:
	rm -f boot.bin kernel_entry.o kernel.o drivers/*.o shell/*.o kernel.elf kernel.bin nebulaos.img
