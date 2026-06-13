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