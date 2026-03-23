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
cargo run
```

*Requires QEMU and a nightly Rust toolchain.*

## Current Bugs
[None]

## 📜 License

This project is licensed under the GNU General Public License v3.0.

---
*Created by NebulaJapanese - 2026*

Note: NebulaOS is programmed with the help of Gemini Code Assist (About 25-35% of NebulaOS code) which is why I can release versions so quickly. Usually my code has many bugs so I use Gemini Code Assist to debug. Eventually I will use less and less of it which will make releases much slower (I am only a solo developer)