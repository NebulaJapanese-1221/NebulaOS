# NebulaOS

NebulaOS is an experimental hobby operating system written from scratch in Rust.

## Build Requirements

| Tool | Purpose |
|------|---------|
| `rustc` / `cargo` (nightly) | Rust compiler and package manager |
| `nasm` | x86 assembler for boot code |
| `ld` / `objcopy` (GNU binutils) | Linker and object tools |
| `grub-mkrescue` or `grub2-mkrescue` | Build bootable ISO |
| `xorriso` | GRUB rescue image tool |
| `qemu-system-i386` | Emulator for testing (optional) |

## Installing Dependencies

### Fedora

```bash
sudo dnf install cargo rustc nasm binutils grub2-tools xorriso qemu-system-i386 rustup
rustup toolchain install nightly
rustup default nightly
rustup component add rust-src
```

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install cargo rustc nasm binutils grub-common grub-pc-bin xorriso qemu-system-i386
rustup toolchain install nightly
rustup default nightly
rustup component add rust-src
```

### Arch Linux

```bash
sudo pacman -S cargo rust nasm binutils grub xorriso qemu-base
rustup toolchain install nightly
rustup default nightly
rustup component add rust-src
```

## Building

Clone the repository and build the bootable ISO:

```bash
git clone https://github.com/NebulaJapanese-1221/NebulaOS.git
cd NebulaOS
make
```

Build artifacts:

- `target/i686-unknown-nebula/release/libnebulaos.a` — kernel static library
- `boot/kernel.elf` — linked kernel ELF (multiboot2)
- `nebulaos.iso` — bootable GRUB ISO image

To clean:

```bash
make clean
```

## Running

### QEMU (x86)

```bash
make run
Default username is user, default password is none.
```

This boots `nebulaos.iso` in QEMU with 256 MB RAM, VGA graphics, and serial output to stdio.

If you created a data disk:

```bash
make data-disk
make run
```

### QEMU with GDB debugging

```bash
make debug
```

Then connect with `gdb` to `localhost:1234`.

## License

NebulaOS is released under the GNU General Public License v3.0 (GPL-3.0).
