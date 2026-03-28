# NebulaOS

NebulaOS is a x86 based hobby operating system written in Rust.

**⚠️ ALPHA STATUS**
NebulaOS is in alpha. **If you are looking for a stable operating system for daily use, this is not it (at least until beta, which will start at version 0.5.0).** 

### Why is it unstable?
*   **Experimental Kernel**: The core logic is built on a nightly Rust toolchain with many `unsafe` blocks directly manipulating hardware.
*   **CoW Filesystem Risks**: NebulaFS is a complex ZFS-inspired filesystem still in early development; bugs in the Storage Pool Allocator (SPA) can lead to immediate and total data loss.
*   **Hardware Specificity**: Testing is limited primarily to QEMU and a single Dell model, meaning "Red Screens of Death" (RSOD) are likely on other machines.

Problems can range from applications crashing to catastrophic kernel panics that require a hard reboot.

## 💻 Devices Tested
*   QEMU
*   Dell Inspiron 630m

## 🌟 Features (Apps)

Included in the userspace:
*   **Terminal**: Command-line interface with history and basic commands (`uname`, etc.).
*   **Calculator**: Basic arithmetic utility.
*   **Settings**: For system configuration.
*   **Text Editor**
*   **Paint**
*   **Task Manager**: Monitor running processes and system resources.

## 🛠️ Build & Run

To build and run NebulaOS using QEMU:

```bash
cargo run
```

*Requires QEMU and a nightly Rust toolchain.*

## 📜 License

This project is licensed under the GNU General Public License v3.0.

---
*Created by NebulaJapanese - 2026*

Note: NebulaOS version 0.0.3-dev2 has started development and will be released on April 14 2026 (The iso file). (Commits will still be made to the repository).