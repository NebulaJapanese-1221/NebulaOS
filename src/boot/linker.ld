/* 
 * Bootloader linker script (x86 only - bootloader is always 16/32-bit)
 * BIOS loads the bootloader at 0x7C00
 */

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

