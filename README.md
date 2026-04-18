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
*   **Text Editor**
*   **Paint**
*   **Task Manager**: Monitor running processes and system resources.

## 🛠️ Build & Run

To build and run NebulaOS using QEMU:

```bash
make run
```
For Windows (Requires Windows Subsystem for Linux): 
make run

*Requires QEMU and a nightly Rust toolchain.*

## Current Bugs
[Many issues with hangs and paint does not work at all.]

## 📜 License

This project is licensed under the GNU General Public License v3.0.

---
*Created by NebulaJapanese - 2026*