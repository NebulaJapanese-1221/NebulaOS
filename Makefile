# Build rules for the NebulaOS x86 operating system.
# Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
# See LICENCE for the full license text.

CXX ?= g++
LD ?= ld
NASM ?= nasm
GRUB_MKRESCUE ?= grub-mkrescue
QEMU ?= qemu-system-i386

CXXFLAGS := -m32 -std=gnu++17 -ffreestanding -fno-exceptions -fno-rtti \
	-fno-stack-protector -fno-pic -fno-pie -nostdinc++ -Wall -Wextra -O2
LDFLAGS := -m elf_i386 -T kernel/arch/x86/linker.ld -nostdlib

KERNEL_OBJECTS := kernel_entry.o interrupts_entry.o kernel.o kernel/framebuffer.o kernel/heap.o kernel/paging.o kernel/pmm.o kernel/timer.o kernel/syscall.o kernel/arch/x86/interrupts.o kernel/arch/x86/tss.o drivers/graphics.o drivers/keyboard.o drivers/mouse.o drivers/serial.o drivers/vga.o shell/shell.o shell/apps/console.o shell/window_manager.o
ISO_ROOT := build/isodir
INITRD := initrd.cpio
QEMU_FLAGS ?= -m 64M

.PHONY: all clean run run-serial run-debug

all: nebulaos.iso

initrd: $(INITRD)

$(INITRD): userspace/bin/console userspace/bin/settings userspace/lib/crt0.o
	cd initrd && find . -print0 | cpio --null -o --format=newc > ../$@

userspace/bin/console: userspace/lib/crt0.o userspace/lib/libc.o userspace/bin/console.c userspace/lib/syscalls.h userspace/linker.ld
	cd userspace && ./build.sh

userspace/bin/settings: userspace/lib/crt0.o userspace/lib/libc.o userspace/bin/settings.c userspace/lib/syscalls.h userspace/linker.ld
	cd userspace && ./build.sh

kernel_entry.o: kernel/arch/x86/entry.asm
	$(NASM) -f elf32 $< -o $@

interrupts_entry.o: kernel/arch/x86/interrupts.asm
	$(NASM) -f elf32 $< -o $@

kernel.o: kernel/main.cpp kernel/framebuffer.hpp kernel/heap.hpp kernel/multiboot.hpp kernel/paging.hpp kernel/pmm.hpp kernel/timer.hpp kernel/arch/x86/interrupts.hpp drivers/graphics.hpp drivers/keyboard.hpp drivers/mouse.hpp drivers/serial.hpp drivers/vga.hpp shell/shell.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/framebuffer.o: kernel/framebuffer.cpp kernel/framebuffer.hpp kernel/heap.hpp kernel/multiboot.hpp kernel/paging.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/heap.o: kernel/heap.cpp kernel/heap.hpp kernel/pmm.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/paging.o: kernel/paging.cpp kernel/paging.hpp kernel/pmm.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/pmm.o: kernel/pmm.cpp kernel/pmm.hpp kernel/multiboot.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/timer.o: kernel/timer.cpp kernel/timer.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/syscall.o: kernel/syscall.cpp kernel/syscall.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/arch/x86/interrupts.o: kernel/arch/x86/interrupts.cpp kernel/arch/x86/interrupts.hpp kernel/arch/x86/tss.hpp kernel/timer.hpp drivers/serial.hpp drivers/vga.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel/arch/x86/tss.o: kernel/arch/x86/tss.cpp kernel/arch/x86/tss.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/graphics.o: drivers/graphics.cpp drivers/graphics.hpp kernel/framebuffer.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/keyboard.o: drivers/keyboard.cpp drivers/keyboard.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/mouse.o: drivers/mouse.cpp drivers/mouse.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/serial.o: drivers/serial.cpp drivers/serial.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

drivers/vga.o: drivers/vga.cpp drivers/vga.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

shell/shell.o: shell/shell.cpp shell/shell.hpp shell/apps/console.hpp shell/window_manager.hpp drivers/graphics.hpp drivers/keyboard.hpp drivers/mouse.hpp kernel/heap.hpp kernel/paging.hpp kernel/pmm.hpp kernel/timer.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

shell/apps/console.o: shell/apps/console.cpp shell/apps/console.hpp drivers/graphics.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

shell/window_manager.o: shell/window_manager.cpp shell/window_manager.hpp shell/apps/console.hpp drivers/graphics.hpp drivers/mouse.hpp kernel/heap.hpp
	$(CXX) $(CXXFLAGS) -I. -c $< -o $@

kernel.elf: $(KERNEL_OBJECTS) kernel/arch/x86/linker.ld
	$(LD) $(LDFLAGS) $(KERNEL_OBJECTS) -o $@

nebulaos.iso: kernel.elf boot/grub/grub.cfg $(INITRD)
	mkdir -p $(ISO_ROOT)/boot/grub
	cp kernel.elf $(ISO_ROOT)/boot/kernel.elf
	cp boot/grub/grub.cfg $(ISO_ROOT)/boot/grub/grub.cfg
	cp $(INITRD) $(ISO_ROOT)/boot/initrd.cpio
	$(GRUB_MKRESCUE) -o $@ $(ISO_ROOT)

run: nebulaos.iso
	$(QEMU) $(QEMU_FLAGS) -cdrom $<

# Boot with the serial console mirrored to stdout instead of a graphics window.
run-serial: nebulaos.iso
	$(QEMU) $(QEMU_FLAGS) -cdrom $< -display none -serial stdio

# Boot headless and record CPU exceptions, resets and serial output for triage.
run-debug: nebulaos.iso
	$(QEMU) $(QEMU_FLAGS) -cdrom $< -display none -serial stdio -no-reboot \
		-d int,cpu_reset -D build/qemu-debug.log

clean:
	rm -f kernel_entry.o interrupts_entry.o kernel.o kernel/framebuffer.o kernel/heap.o kernel/paging.o kernel/pmm.o kernel/timer.o kernel/syscall.o kernel/arch/x86/interrupts.o kernel/arch/x86/tss.o drivers/graphics.o drivers/keyboard.o drivers/mouse.o drivers/serial.o drivers/vga.o shell/shell.o shell/apps/console.o shell/window_manager.o kernel.elf nebulaos.iso $(INITRD)
	rm -rf build
	rm -f userspace/lib/crt0.o userspace/lib/libc.o userspace/bin/console.o userspace/bin/settings.o
