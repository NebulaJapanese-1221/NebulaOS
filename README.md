# NebulaOS

NebulaOS is a x86 based hobby operating system written in Rust.

**⚠️ ALPHA STATUS**
NebulaOS is currently in alpha. Problems can range from apps crashing to the OS not booting at all.

## 💻 Devices Tested
*   QEMU
*   Dell Inspiron 630m

## 🌟 Features (Apps)

Included in the userspace:
*   **Terminal**: Command-line interface with history and basic commands (`uname`, etc.).
*   **Calculator**: Basic arithmetic utility.
*   **Settings**: For system configuration.

## 🛠️ Build & Run

To build and run NebulaOS using QEMU:

```bash
cargo run
```

*Requires QEMU and a nightly Rust toolchain.*

## Current Bugs
- The keyboard does not work (in QEMU and real hardware)
- The mouse freezes at random times (in QEMU and real hardware)
- Shutdown always crashes the system (in QEMU and real hardware)

## 📜 License

This project is licensed under the GNU General Public License v3.0.

---
*Created by NebulaJapanese - 2026*