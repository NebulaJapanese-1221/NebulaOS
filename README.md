# NebulaOS

A hobbyist 32-bit operating system written in Rust for the i686 architecture.

### Credits for NebulaFS

NebulaFS is inspired by and builds upon the groundbreaking work of the ZFS developers:

- **Original ZFS Team at Sun Microsystems**: Jeff Bonwick, Bill Moore, Matthew Ahrens, and many others who designed and implemented the original ZFS file system.

- **OpenZFS Community**: The open-source community that has continued to develop and maintain ZFS across multiple platforms.

- **ZFS on Linux Team**: For bringing ZFS to Linux and continuing its development.

While NebulaFS is not a derivative work of ZFS and does not use any ZFS code, its design and architecture are heavily influenced by the innovative concepts introduced by ZFS. We are grateful to all the developers who have contributed to ZFS over the years.

## License

This project is licensed under the GNU General Public License (GPL) v3.0.

## Apps
- **Calculator**
- **Text Editor**
- **Terminal**
- **System Settings**
- **File Manager** (New!)

## Prerequisites
- Rust Nightly toolchain
- `llvm-objcopy` (usually part of `lld`)
- `grub-mkrescue` and `xorriso` (for ISO creation)
- QEMU for emulation

## Building and Running

To build the kernel and create a bootable ISO:

