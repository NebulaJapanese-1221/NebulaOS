TARGET = i686-nebula.json
IMAGE = nebula.iso

BOOTLOADER_ELF = target/i686-nebula/debug/bootloader
KERNEL_ELF = target/i686-nebula/debug/kernel

BOOTLOADER_BIN = target/i686-nebula/debug/bootloader.bin
KERNEL_BIN = target/i686-nebula/debug/kernel.bin

OBJCOPY = llvm-objcopy

ISO_DIR = isodir
BOOT_DIR = $(ISO_DIR)/boot
GRUB_DIR = $(BOOT_DIR)/grub

.PHONY: all clean run build

all: $(IMAGE)

build:
	# Explicitly specifying core/alloc crates to avoid building the incompatible 'std' crate
	RUSTFLAGS="-C link-arg=-Tsrc/boot/linker.ld" cargo build -Zbuild-std=core -Zjson-target-spec --bin bootloader
	RUSTFLAGS="-C link-arg=-Tsrc/kernel/linker.ld" cargo build -Zbuild-std=core,alloc -Zjson-target-spec --bin kernel

$(BOOTLOADER_BIN): build
	$(OBJCOPY) -I elf32-i386 -O binary $(BOOTLOADER_ELF) $(BOOTLOADER_BIN)

$(KERNEL_BIN): build
	$(OBJCOPY) -I elf32-i386 -O binary $(KERNEL_ELF) $(KERNEL_BIN)

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
	# Directly boot the kernel ELF (Multiboot) to simplify development
	qemu-system-i386 -cdrom $(IMAGE) -m 64M -serial stdio -vga vmware

clean:
	cargo clean
	rm -f $(IMAGE)
	rm -rf $(ISO_DIR)