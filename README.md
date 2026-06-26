# NebulaOS

A hobbyist 32-bit operating system written in Rust for the i686 architecture.

# Notice
Development of NebulaOS is currently paused as I am currently working on other projects.

## License

This project is licensed under the GNU General Public License (GPL) v3.0.

## Apps
- **Calculator**
- **Text Editor**
- **Terminal**

## Prerequisites
- Rust Nightly toolchain
- `llvm-objcopy` (usually part of `lld`)
- `grub-mkrescue` and `xorriso` (for ISO creation)
- QEMU for emulation

## Building and Running

To build the kernel and create a bootable ISO:
```bash
make all
```

To launch the OS in QEMU with serial output mirrored to your terminal:
```bash
make run
```

## Donations
Donations are completely voluntary and are welcome.
I currently only accept cryptocurrency donations.
BTC (Lightning Network): shabbyjoke61@walletofsatoshi.com
