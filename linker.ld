/* The bootloader will look at this image and start execution at the symbol
   designated as the entry point. */
ENTRY(_start)

SECTIONS
{
    /* The kernel will be loaded by the bootloader at 1MB. */
    . = 1M;

    /* The .text section contains the multiboot header, followed by code. 
       It is crucial that the multiboot header is first. */
    .text :
    {
        *(.multiboot_header)
        *(.text .text.*)
    }

    /* Read-only data. */
    .rodata :
    {
        *(.rodata .rodata.*)
    }

    /* Read-write data. */
    .data :
    {
        *(.data .data.*)
    }

    /* Uninitialized data. */
    .bss :
    {
        *(.bss .bss.*)
    }

    _end = .; /* Symbol for the end of the kernel image. */
}