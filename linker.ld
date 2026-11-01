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
    .rodata : {
        *(.rodata)
    }
    .data : {
        *(.data .data.*)
    }
    .bss : {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        __bss_end = .;
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.note.GNU-stack)
        *(.gcc_except_table)
    }
}
