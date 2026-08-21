# NebulaOS

## A Lightweight Operating System for x86 Architecture

NebulaOS is a hobby operating system project designed to run on x86 hardware. It provides a basic kernel with memory management, device drivers, and a simple GUI interface.

## Building

### Prerequisites
- NASM (Netwide Assembler)
- GCC with 32-bit support
- GNU Make
- QEMU (for testing)

### Build Commands

```bash
make arch=x86        # Build x86 version
make arch=x86_64    # Build x86_64 version
make                # Build both
make clean          # Clean build files
make run            # Run in QEMU
```

## License

This project is licenced by the GNU General Public Licence v3.0
