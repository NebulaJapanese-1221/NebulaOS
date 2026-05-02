KERNEL_BIN := target/i686-unknown-none/debug/nebula_os

all: iso

build:
	RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo +nightly build --target i686-unknown-none.json -Z build-std=core,compiler_builtins,alloc -Z unstable-options -Z json-target-spec

iso: build
	mkdir -p isofiles/boot/grub
	cp $(KERNEL_BIN) isofiles/boot/kernel.bin
	echo 'set timeout=5' > isofiles/boot/grub/grub.cfg
	echo 'set default=0' >> isofiles/boot/grub/grub.cfg
	echo 'set gfxpayload=auto' >> isofiles/boot/grub/grub.cfg
	echo 'menuentry "NebulaOS" {' >> isofiles/boot/grub/grub.cfg
	echo '  multiboot /boot/kernel.bin' >> isofiles/boot/grub/grub.cfg
	echo '  boot' >> isofiles/boot/grub/grub.cfg
	echo '}' >> isofiles/boot/grub/grub.cfg
	echo 'menuentry "NebulaOS (Safe Mode)" {' >> isofiles/boot/grub/grub.cfg
	echo '  multiboot /boot/kernel.bin safemode' >> isofiles/boot/grub/grub.cfg
	echo '  boot' >> isofiles/boot/grub/grub.cfg
	echo '}' >> isofiles/boot/grub/grub.cfg
	grub-mkrescue -o nebula_os.iso isofiles
	rm -r isofiles

run: iso
	qemu-system-i386 -cdrom nebula_os.iso -m 1024 -cpu max -accel kvm -accel tcg -serial stdio

run-x64: iso
	qemu-system-x86_64 -cdrom nebula_os.iso -m 1024 -cpu max -accel kvm -accel tcg -serial stdio

clean:
	cargo clean
	rm -f nebula_os.iso
.PHONY: all build iso run run-x64 clean