# NebulaOS

NebulaOS boots through GRUB's Multiboot2 interface and requests a 1024x768 RGB
framebuffer. The kernel maps the bootloader-provided framebuffer and renders the
desktop, windows, text, and pointer in graphics mode; it has no VGA text-console
fallback.

GUI applications implement the shared application/event/canvas API in
`userspace/apps/gui.rs`. The window manager owns desktop/window presentation,
focus and input routing; apps own their state and render into content areas.
The desktop supports multiple windows, raise-on-click, dragging, minimize,
maximize, and close. GUI controls include buttons, text fields, checkboxes,
menus, scroll areas, and tab-based focus selection. Buttons activate on
left-button release inside their bounds.

## Build and run

Run `make` to build `nebulaos.iso`, then `make run` to boot it in QEMU. GRUB
shows a five-second boot menu before starting NebulaOS. Kernel diagnostics are
available on the serial port. `make` does not create a disk image. To use
persistent files, run `make data-disk` once to create the separate
`nebulaos.disk` data drive, then boot with `make run`. The file manager offers
to format a blank disk and refuses to format disks containing other data.
`make clean` preserves this data disk.

## Desktop controls

- Open apps using the desktop cards or number keys `1` through `6`: calculator,
  text editor, file manager, settings, paint, and system monitor.
- In the calculator, click its keypad or type an integer expression such as
  `12+3` and press Enter. Press `C` or click `C` to clear the expression; click
  `=` to calculate.
- Type text in the editor; Backspace deletes and Enter starts a new line. Click
  `CLEAR` to erase the text.
- The file manager supports listing, creating, saving, renaming, and deleting
  root files on NebulaFS. Select a filename field with Tab and enter content
  in the content field. NebulaFS supports 16 files, up to 512 bytes each; it is
  an initial fixed-size filesystem, not FAT-compatible.
- Paint supports freehand mouse drawing, color selection, and clearing.
- Settings provides a wallpaper theme toggle and live mouse speed selection.
  The framebuffer mode remains configured by GRUB at boot; the current layout
  is 1024x768. The system monitor reports uptime ticks and physical memory
  frame counts.
- Drag a window by its title bar. Use `_` to minimize, `[]` to maximize/restore,
  and `X` to close it. Click a minimized window in the bottom bar to restore it.
- Press Escape or click the window's close button to close the focused app.
- Buttons activate when the left mouse button is released inside the control.