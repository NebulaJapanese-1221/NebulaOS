# NebulaOS Version History

## v0.0.3-dev2 (04-07-2026)
- Implemented NebulaFS: A ZFS-inspired Copy-On-Write (COW) filesystem.
- Added SPA (Storage Pool Allocator) with VDEV hierarchy support (Leaf, Mirror, RAID-Z).
- Implemented VDEV tree serialization and persistence via Uberblocks and VDEV Labels.
- Added Partition Manager
- Added MBR initialization and OS installation features to Partition Manager.
- Added keyboard navigation (Arrow keys + Enter) to the Start Menu.
- Added search bar to the Start Menu with real-time application filtering.
- Added a clear button to the Start Menu search bar for quick reset.
- Implemented initial USB Host Controller support (UHCI) for peripheral discovery.
- Fixed alignment issues for filesystem structures on 32-bit targets.

## v0.0.3-dev (03-22-2026)
- Added proper System Information detection (CPUID for Brand String).
- Added basic support for detecting multiple CPU cores (SMP detection).
- Added a task manager app.
- Refactored `InputManager` to support modifier keys and shortcuts.
- Added `Alt+Tab` window switching.
- Added CPU Usage detection logic (for task manager)
- Added ELF Executable Loader (kernel support).
- Standardized application format to `.app` (ELF binaries).

## v0.0.2 (03-22-2026)
- Added basic `Paint` application
- Greatly expanded the Japanese font with more common-use Kanji.
- Updated Japanese localization to use more Kanji strings.
- Fixed some issues with resizing windows
- Removed Virtual File System (VFS) filesystem code as it was unused.
- Added detailed system information (Resolution, Memory, Uptime) to Settings.
- Added Scientific Mode to Calculator (Mod, Pow, Sqrt, Factorial).

## 0.0.2-dev3 (03-21-2026)
- Fixed keyboard not working (QEMU and Real Hardware)
- Fixed mouse freezing at random times (QEMU and Real Hardware)
- Fixed shutdown crashing NebulaOS (QEMU and Real Hardware)
- Added mouse wheel support (Intellimouse extension)
- Added `uptime` command to Terminal
- Fixed input freezing/deadlocks in keyboard driver and scrolling in Text Editor
- Fixed some UI issues


## v0.0.2-dev2 (03-21-2026)
- Added full Hiragana and Katakana character sets with common Kanji characters to the Japanese font. Also fixed many issues from the previous 2 versions (graphics and lag) and added better window dragging.

## v0.0.2-dev (03-21-2026)
- Added a proper settings app to NebulaOS and added localisation support with basic (placeholder) support for Japanese

## v0.0.1 (03-20-2026)
- Initial Alpha Release.
