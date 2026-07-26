/* 
 * Kernel linker script - supports both x86 (32-bit) and x86_64 (64-bit)
 * x86: kernel loaded at 1MB, Multiboot1
 * x86_64: kernel loaded at 1MB, Multiboot2
 */

ENTRY(_start)

SECTIONS {
    /* GRUB expects the kernel to be loaded at 1MB */
    . = 1M;

    .multiboot : {
        /* Ensure the multiboot header is at the very beginning */
        KEEP(*(.multiboot))
    }

    .text : { *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data : { *(.data .data.*) }
    .bss : { *(.bss .bss.*) }

    /DISCARD/ : { *(.eh_frame) *(.note .note.*) }
}

