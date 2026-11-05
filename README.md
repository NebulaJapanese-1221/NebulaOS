# NebulaOS

NebulaOS is an experimental hobby operating system written in Rust. It features
a graphical desktop, user-mode applications, and support for multiple CPUs.

## Build and run

Build the bootable ISO with `make`, then run it in QEMU with `make run`.

The build requires Rust nightly, NASM, GNU binutils, GRUB tools, and xorriso.
