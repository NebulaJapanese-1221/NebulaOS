# NebulaOS

NebulaOS is a x86 based hobby operating system written in Rust.

**⚠️ ALPHA STATUS**
NebulaOS is currently in alpha. Problems can range from apps crashing to the OS not booting at all. New versions are released once or twice per week.

## 💻 Devices Tested
*   QEMU
*   Dell Inspiron 630m

## 🌟 Features (Apps)

Included in the userspace:
*   **Terminal**: Command-line interface with history and basic commands (`uname`, etc.).
*   **Calculator**: Basic arithmetic utility.
*   **Settings**: For system configuration.
*   **Text Editor**

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

Note: NebulaOS was created with the help of Gemini Code Assist (About 25-35% of the code where it was hard to program and debug)

Version 0.0.1 of NebulaOS is the codebase that all new versions are built off of (which makes it easier to create new versions as I don't need to create new codebases every version and I can just build off of the first one to add to it (At least until I have way too many code files and need to organize them).