# NebulaOS

NebulaOS 0.0.1 boots through GRUB's Multiboot2 interface and requests a 1024x768 RGB
framebuffer. The kernel maps the bootloader-provided framebuffer and renders the
desktop, windows, text, and pointer in graphics mode; it has no VGA text-console
fallback. A graphical boot splash shows “NebulaOS”, cursive-style “Alpha”, and
the package version before the desktop appears. Its progress bar marks completed
graphics, storage, input, and desktop initialization stages, and it remains
visible for at least five seconds.

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

- Click **Start** on the taskbar to open apps, or use number keys `1` through
  `7`: calculator, text editor, file manager, settings, paint, system monitor,
  and Ring-3 Demo.
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
- Ring-3 Demo installs the small `DEMO.ELF` executable into NebulaFS on first
  launch, then loads and validates the stored ELF32 image into a separate
  address space. The timer interrupt preempts runnable user processes, keeps a
  separate kernel stack and CR3 for each one, and reclaims their memory after
  exit. Start the demo twice within its three-second tick-wait to observe concurrent
  processes. The initial syscall ABI uses EAX for the call number, EBX for
  argument one, and ECX for argument two: 0 prints a bounded user string, 2
  exits, and 3 returns uptime ticks. NebulaFS still limits each file to 512
  bytes, so this initial loader accepts only ELF files that fit that limit.
  Other GUI apps still run in kernel space; moving their drawing and input
  through a stable user/kernel interface remains future work.
- Drag a window by its title bar. Use `_` to minimize, `[]` to maximize/restore,
  and `X` to close it. The taskbar shows open apps; click an app button to focus
  or restore its window.
- Press Escape or click the window's close button to close the focused app.
- Buttons activate when the left mouse button is released inside the control.