# NebulaOS

NebulaOS boots through GRUB's Multiboot2 interface and requests a 1024x768 RGB
framebuffer. The kernel maps the bootloader-provided framebuffer and renders the
desktop, windows, text, and pointer in graphics mode; it has no VGA text-console
fallback.

GUI applications implement the shared application/event/canvas API in
`userspace/apps/gui.rs`. The window manager owns desktop/window presentation,
focus and input routing; each app owns its model and renders into its content
area. Reusable buttons track pointer press and release, and activate only when
released inside their bounds. This follows the event-driven, backend-independent
rendering style used by embedded graphics systems while keeping the current
framebuffer backend small.

## Build and run

Run `make` to build `nebulaos.iso`, then `make run` to boot it in QEMU. GRUB
shows a five-second boot menu before starting NebulaOS. Kernel diagnostics are
available on the serial port.

## Desktop controls

- Press `1` to open the calculator or `2` to open the text editor.
- In the calculator, click its keypad or type an integer expression such as
  `12+3` and press Enter. Press `C` or click `C` to clear the expression; click
  `=` to calculate.
- Type text in the editor; Backspace deletes and Enter starts a new line. Click
  `CLEAR` to erase the text.
- Press Escape or click the window's close button to return to the desktop.
- Click either desktop shortcut to open its app.
- Buttons activate when the left mouse button is released inside the control.