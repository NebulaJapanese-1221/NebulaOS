# NebulaOS

NebulaOS 0.0.1 boots through GRUB's Multiboot2 interface and requests a 1024x768 RGB
framebuffer. The kernel maps the bootloader-provided framebuffer and renders the
desktop, windows, text, and pointer through a page-backed double buffer in
graphics mode; completed frames are copied to the display together, and it has no VGA text-console
fallback. A graphical boot splash shows “NebulaOS”, cursive-style “Alpha”, and
the package version before the desktop appears. The Alpha lettering uses a
brighter stroke and dark offset shadow for improved contrast. Its progress bar marks completed
graphics, storage, input, and desktop initialization stages, and it remains
visible for at least five seconds.

Kernel code is organized by responsibility: `kernel/drivers` contains
hardware-facing drivers, `kernel/memory` contains allocation and paging,
`kernel/arch` contains CPU tables and interrupt-controller support,
`kernel/services` contains process and syscall services, and `kernel/graphics`
contains the boot screen and window manager.

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