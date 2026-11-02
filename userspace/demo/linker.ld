OUTPUT_FORMAT(elf32-i386)
ENTRY(_start)

PHDRS
{
    text PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS
{
    . = 0x40000000;
    .text : {
        *(.text*)
        *(.rodata*)
    } :text
    . = ALIGN(0x1000);
    .data : {
        *(.data* .sdata* .got*)
    } :data
    .bss : {
        *(.bss* .sbss*)
        *(COMMON)
    } :data
    /DISCARD/ : {
        *(.comment)
        *(.note*)
        *(.eh_frame)
    }
}
