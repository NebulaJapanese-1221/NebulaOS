/* NebulaOS x86 Linker Script */
/* ============================ */

OUTPUT_FORMAT("elf32-i386")
OUTPUT_ARCH(i386)

ENTRY(_start)

/* Memory layout */
MEMORY {
    /* Kernel starts at 1MB */
    kernel (rwx) : ORIGIN = 0x00100000, LENGTH = 16M
    
    /* Heap starts at 16MB */
    heap (rwx) : ORIGIN = 0x01000000, LENGTH = 64M + 32K
    
    /* Extended memory */
    extmem (rwx) : ORIGIN = 0x09000000, LENGTH = 128M
}

/* Sections */
SECTIONS {
    /* Text section (code) */
    .text : {
        KEEP(*(.text.start))
        *(.text*)
    } > kernel

    /* Keep the Multiboot header within the first 8KB of the kernel image. */
    .multiboot_header : {
        KEEP(*(.multiboot_header))
    } > kernel
    
    /* Read-only data */
    .rodata : {
        *(.rodata*)
    } > kernel
    
    /* Data section (initialized) */
    .data : {
        *(.data*)
        *(.got .got.* .got.plt .igot.*)
    } > kernel
    
    /* BSS section (uninitialized) */
    .bss : {
        *(.bss*)
        *(COMMON)
    } > kernel
    
    /* Heap */
    .heap (NOLOAD) : {
        . = ALIGN(4096);
        _heap_start = .;
        . += 64M;
        _heap_end = .;
    } > heap
    
    /* Stack (grows downward from end of heap) */
    .stack (NOLOAD) : {
        . = ALIGN(16);
        _stack_start = .;
        . += 16K;
        _stack_end = .;
    } > heap
    
    /* Symbols */
    _end = .;
    PROVIDE(end = _end);
    
    /* Discard sections */
    /DISCARD/ : {
        *(.comment)
        *(.note)
        *(.note.gnu.build-id)
    }
}
