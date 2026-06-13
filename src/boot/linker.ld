ENTRY(_start)

SECTIONS {
    /* BIOS loads the bootloader at this address */
    . = 0x7c00;

    .text : {
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
    }
    .data : {
        *(.data .data.*)
    }

    /DISCARD/ : { *(.eh_frame) *(.note .note.*) }
}