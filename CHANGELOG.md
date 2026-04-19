# Changelog

All notable changes to NebulaOS will be documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.0.3-dev3] - In Progress
### Changed
- Revamped Settings App and added Power Settings and Display Settings.
- Implemented dynamic CPU frequency detection and calibrated NOP-based delay loops for consistent timing across hardware.
- Synchronized boot sequence and UI animations to the system timer, fixing the 'too fast' execution on real hardware.
- Enhanced Task Manager bars with automatic percentage text and better visibility.
- Optimized ACPI power status updates by caching DSDT pointers, significantly reducing CPU overhead in the GUI loop.
- Improved overall system responsiveness and fixed 'soft' hangs during battery polling.
### Added
- **Midnight Theme**: A new dark preset for the desktop environment using a deep blue-black gradient.
- Added "Midnight" localization strings to both English and Japanese (`ja_jp.rs`).
- UI button for the Midnight theme in the Settings app under the "Theme" tab.
- Added 'NebulaBrowser' app (experimental/non-functional mockup).
- Added thermal zone (_TZ) monitoring support in ACPI.
- Added CPU Temperature display to Task Manager.
- Added a ProgressBar component to the GUI.
### Fixed
- Fixed 'Clear' button in Paint app by returning a dirty rectangle to the GUI manager.
- Fixed some issues that caused the OS to hang.
### Removed
- Temporarily removed performance graphs from Task Manager to improve UI stability.

## [0.0.3-dev2] - 2026-04-17
### Added
- Added robust Framebuffer initialization with support for 15-bit (RGB555) and 16-bit (RGB565) color depths.
- Implemented a streamlined PAE Paging system for improved kernel identity mapping stability.
- Added a 'Brightness' system with real control and OSD popup.
- Added a 'Battery' indicator to the taskbar with ACPI device detection hooks.
- Implemented a dynamic ToolTip system for the taskbar.
- Added a system top bar with the Start menu and status indicators (Battery and Date/Time).
- Added a better boot screen.
- Added adjustable Mouse Sensitivity in Settings with sub-pixel accumulation logic.

## [0.0.3-dev] - 2026-03-22
### Added
- Added proper System Information detection (CPUID for Brand String).
- Added basic support for detecting multiple CPU cores (SMP detection).
- Added a task manager app.
- Refactored `InputManager` to support modifier keys and shortcuts.
- Added `Alt+Tab` window switching.
- Added CPU Usage detection logic.
- Added ELF Executable Loader (kernel support) with `.app` standardization.

## [0.0.2] - 2026-03-22
### Added
- Added basic `Paint` application.
- Greatly expanded the Japanese font with more common-use Kanji.
- Added detailed system information (Resolution, Memory, Uptime) to Settings.
- Added Scientific Mode to Calculator (Mod, Pow, Sqrt, Factorial).
### Fixed
- Fixed some issues with resizing windows.
### Removed
- Removed Virtual File System (VFS) filesystem code as it was unused.

## [0.0.2-dev] - 2026-03-21
### Added
- Added a proper settings app and localisation support for English and Japanese.
- Added full Hiragana/Katakana character sets.
- Added mouse wheel support (Intellimouse extension).
- Fixed keyboard and mouse freezing issues on real hardware.
- Fixed shutdown crashes.

## [0.0.1] - 2026-03-20
- Initial Alpha Release.