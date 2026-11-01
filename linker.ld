OUTPUT_FORMAT(elf32-i386)
ENTRY(_start)

SECTIONS
{
    . = 0x10000;
    .text : {
        *(.text.boot)
        KEEP(*(.multiboot2))
        *(.text)
        *(.text.*)
    }
    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
    }
    . = ALIGN(4K);
    .bss : {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        . = ALIGN(4K);
        __bss_end = .;
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.note.GNU-stack)
        *(.gcc_except_table)
    }
}
