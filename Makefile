# NebulaOS Makefile
# ================
#
# Build system for NebulaOS operating system
# Supports x86 and x86_64 targets
#
# Usage:
#   make                - Build all (x86 and x86_64)
#   make ARCH=x86       - Build x86 version
#   make ARCH=x86_64    - Build x86_64 version
#   make x86            - Build x86 version
#   make x86_64         - Build x86_64 version
#   make clean          - Clean all build files
#   make iso            - Create bootable ISO image
#   make run            - Run in QEMU

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

# Architecture (x86 or x86_64)
# Accept both 'ARCH' and 'arch' from command line
ifdef arch
ARCH := $(arch)
endif
ARCH ?= x86

# Build directory
BUILD_DIR := build
OBJ_DIR := $(BUILD_DIR)/obj
ISO_DIR := $(BUILD_DIR)/iso

# Toolchain - use cross-compiler prefixes
ifeq ($(ARCH),x86)
    CROSS_PREFIX := i686-linux-gnu-
    LDFLAGS := -m elf_i386
    NASMFLAGS := -f elf
else ifeq ($(ARCH),x86_64)
    CROSS_PREFIX := x86_64-linux-gnu-
    LDFLAGS := -m elf_x86_64
    NASMFLAGS := -f elf64
else
$(error Unsupported ARCH: $(ARCH). Use x86 or x86_64)
endif

CC := $(CROSS_PREFIX)gcc
CXX := $(CROSS_PREFIX)g++
AS := nasm
LD := $(CROSS_PREFIX)ld
OBJCOPY := $(CROSS_PREFIX)objcopy
OBJDUMP := $(CROSS_PREFIX)objdump
GENISOIMAGE := genisoimage
QEMU := qemu-system-$(ARCH)

# Compiler flags
CFLAGS := -ffreestanding -nostdlib -nodefaultlibs -fno-builtin -fno-stack-protector -Wall -Wextra
CXXFLAGS := -ffreestanding -nostdlib -nodefaultlibs -fno-builtin -fno-stack-protector -Wall -Wextra -fno-exceptions -fno-rtti

# Include paths
INCLUDES := -Ikernel/common/include -Ikernel/$(ARCH)/include -Igui/include -Ilib/include -Idrivers/include

# Source files
# Bootloader
BOOT_SOURCES_x86 := boot/x86/boot.asm
BOOT_SOURCES_x86_64 := boot/x86_64/boot.asm

# Kernel sources (common + architecture-specific)
KERNEL_SOURCES := \
    kernel/$(ARCH)/start.asm \
    kernel/common/src/vga.c \
    kernel/common/src/memory/memory.c \
    kernel/$(ARCH)/src/device/gdt.c \
    kernel/$(ARCH)/src/interrupts/idt.c \
    kernel/$(ARCH)/src/interrupts/isr.asm \
    kernel/$(ARCH)/entry.c

# x86-specific shell (text-mode only)
ifeq ($(ARCH),x86)
KERNEL_SOURCES += kernel/$(ARCH)/src/process/shell.c
endif

# x86_64-specific memory, stubs, and shell
ifeq ($(ARCH),x86_64)
KERNEL_SOURCES += \
    kernel/x86_64/src/memory/paging.c \
    kernel/x86_64/src/device/stubs.c \
    kernel/x86_64/src/process/shell.c
endif

# GUI sources (shared)
GUI_SOURCES := \
    gui/src/Point.cpp \
    gui/src/Size.cpp \
    gui/src/Rectangle.cpp \
    gui/src/Color.cpp \
    gui/src/GraphicsContext.cpp \
    gui/src/Window.cpp \
    gui/src/WindowManager.cpp \
    gui/src/Control.cpp \
    gui/src/Button.cpp \
    gui/src/Label.cpp \
    gui/src/TextBox.cpp \
    gui/src/Panel.cpp \
    gui/src/Font.cpp \
    gui/src/GUI.cpp

# Driver sources (shared)
DRIVER_SOURCES := \
    drivers/src/keyboard/keyboard.c \
    drivers/src/mouse/mouse.c \
    drivers/src/pit.c \
    drivers/src/pic.c

# Library sources (shared)
LIB_SOURCES := \
    lib/src/string/string.c \
    lib/src/math/math.c \
    lib/src/time/time.c

CXX_LIB_SOURCES := \
    lib/src/cxx/runtime.cpp

# Target files
BOOT_OBJECTS := $(OBJ_DIR)/$(ARCH)/boot_$(ARCH).o
BOOT_ENTRY := $(OBJ_DIR)/$(ARCH)/boot_$(ARCH).o

KERNEL_OBJECTS := $(patsubst %.c,$(OBJ_DIR)/$(ARCH)/%.o,$(filter %.c,$(KERNEL_SOURCES)))
KERNEL_ASM_OBJECTS := $(patsubst %.asm,$(OBJ_DIR)/$(ARCH)/%.o,$(filter %.asm,$(KERNEL_SOURCES)))
GUI_OBJECTS := $(patsubst %.cpp,$(OBJ_DIR)/$(ARCH)/%.o,$(filter %.cpp,$(GUI_SOURCES)))
LIB_OBJECTS := $(patsubst %.c,$(OBJ_DIR)/$(ARCH)/%.o,$(filter %.c,$(LIB_SOURCES)))
CXX_LIB_OBJECTS := $(patsubst %.cpp,$(OBJ_DIR)/$(ARCH)/%.o,$(filter %.cpp,$(CXX_LIB_SOURCES)))
DRIVER_OBJECTS := $(patsubst %.c,$(OBJ_DIR)/$(ARCH)/%.o,$(filter %.c,$(DRIVER_SOURCES)))

ALL_OBJECTS := $(KERNEL_OBJECTS) $(KERNEL_ASM_OBJECTS) $(GUI_OBJECTS) $(LIB_OBJECTS) $(CXX_LIB_OBJECTS) $(DRIVER_OBJECTS)

# Final output files
KERNEL_ELF := $(BUILD_DIR)/nebulaos_$(ARCH).elf
KERNEL_BIN := $(BUILD_DIR)/nebulaos_$(ARCH).bin
ISO_IMAGE := $(ISO_DIR)/nebulaos_$(ARCH).iso

# Link script
LINK_SCRIPT := kernel/$(ARCH)/link.ld

# GRUB config
GRUB_CFG := $(ISO_DIR)/grub.cfg

# -----------------------------------------------------------------------------
# Phony targets
# -----------------------------------------------------------------------------

.PHONY: all x86 x86_64 clean iso run _build_arch

# Build all architectures
all: x86 x86_64

# Architecture-specific targets use recursive make to ensure
# all ARCH-dependent variables are properly set
x86:
	@$(MAKE) --no-print-directory ARCH=x86 _build_arch

x86_64:
	@$(MAKE) --no-print-directory ARCH=x86_64 _build_arch

_build_arch: $(KERNEL_BIN)

# Create build directories
$(OBJ_DIR):
	mkdir -p $(OBJ_DIR)

$(OBJ_DIR)/$(ARCH):
	mkdir -p $(OBJ_DIR)/$(ARCH)

$(ISO_DIR):
	mkdir -p $(ISO_DIR)

# -----------------------------------------------------------------------------
# Bootloader rules
# -----------------------------------------------------------------------------

$(OBJ_DIR)/$(ARCH)/boot_$(ARCH).o: boot/$(ARCH)/boot.asm boot/$(ARCH)/print.asm | $(OBJ_DIR)/$(ARCH)
	mkdir -p $(dir $@)
	$(AS) $(NASMFLAGS) -o $@ $<

# -----------------------------------------------------------------------------
# Compilation rules
# -----------------------------------------------------------------------------

$(OBJ_DIR)/$(ARCH)/%.o: %.asm | $(OBJ_DIR)/$(ARCH)
	mkdir -p $(dir $@)
	$(AS) $(NASMFLAGS) -o $@ $<

$(OBJ_DIR)/$(ARCH)/%.o: %.c | $(OBJ_DIR)/$(ARCH)
	mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ -c $<

$(OBJ_DIR)/$(ARCH)/%.o: %.cpp | $(OBJ_DIR)/$(ARCH)
	mkdir -p $(dir $@)
	$(CXX) $(CXXFLAGS) $(INCLUDES) -o $@ -c $<

# -----------------------------------------------------------------------------
# Linking rules
# -----------------------------------------------------------------------------

$(KERNEL_ELF): $(ALL_OBJECTS) $(LINK_SCRIPT) | $(OBJ_DIR)/$(ARCH)
	$(LD) $(LDFLAGS) -T $(LINK_SCRIPT) -o $@ $(ALL_OBJECTS)

$(KERNEL_BIN): $(KERNEL_ELF) | $(OBJ_DIR)/$(ARCH)
	$(OBJCOPY) -O binary $< $@

# -----------------------------------------------------------------------------
# ISO creation
# -----------------------------------------------------------------------------

$(GRUB_CFG): | $(ISO_DIR)
	set -e; \
	echo 'set timeout=0' > $@; \
	echo 'set default=0' >> $@; \
	echo '' >> $@; \
	echo 'menuentry "NebulaOS $(ARCH)" {' >> $@; \
	echo '    multiboot /boot/nebulaos_$(ARCH).bin' >> $@; \
	echo '    boot' >> $@; \
	echo '}' >> $@

$(ISO_DIR)/boot/nebulaos_$(ARCH).bin: $(KERNEL_BIN) | $(ISO_DIR)
	mkdir -p $(ISO_DIR)/boot
	cp $< $@

$(ISO_IMAGE): $(ISO_DIR)/boot/nebulaos_$(ARCH).bin $(GRUB_CFG) | $(ISO_DIR)
	mkdir -p $(ISO_DIR)/boot/grub
	cp $(GRUB_CFG) $(ISO_DIR)/boot/grub/
	$(GENISOIMAGE) -R -b boot/grub/i386-pc/eltorito_boot.img -c boot/grub/boot.cat -no-emul-boot -boot-load-size 4 -boot-info-table -o $@ $(ISO_DIR)

iso: $(ISO_IMAGE)

# -----------------------------------------------------------------------------
# Clean
# -----------------------------------------------------------------------------

clean:
	rm -rf $(BUILD_DIR)

# -----------------------------------------------------------------------------
# Run in QEMU
# -----------------------------------------------------------------------------

run: $(KERNEL_BIN)
	$(QEMU) -kernel $< -m 256M -serial stdio

run-iso: $(ISO_IMAGE)
	$(QEMU) -cdrom $< -m 256M -serial stdio

# -----------------------------------------------------------------------------
# Debug targets
# -----------------------------------------------------------------------------

.PHONY: debug

debug: $(KERNEL_ELF)
	$(OBJDUMP) -d $< > $(BUILD_DIR)/disassembly.txt
	$(OBJDUMP) -t $< > $(BUILD_DIR)/symbols.txt
	$(CROSS_PREFIX)nm $< > $(BUILD_DIR)/nm.txt

.PHONY: gdb

gdb: $(KERNEL_ELF)
	$(QEMU) -kernel $(KERNEL_BIN) -s -S -m 256M -serial stdio &
	gdb -ex "target remote :1234" -ex "break kernel_main" $(KERNEL_ELF)
