/* NebulaOS x86_64 Linker Script */
/* =============================== */

OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)

ENTRY(_start)

/* Memory layout */
MEMORY {
    /* Kernel starts at 1MB (higher half for 64-bit) */
    kernel (rwx) : ORIGIN = 0x00100000, LENGTH = 32M
    
    /* Heap starts at 32MB */
    heap (rwx) : ORIGIN = 0x02000000, LENGTH = 128M + 32K
    
    /* Extended memory */
    extmem (rwx) : ORIGIN = 0x10000000, LENGTH = 512M
}

/* Sections */
SECTIONS {
    /* Multiboot2 header must be in first 8KB and aligned to 8 bytes */
    .multiboot_header : {
        *(.multiboot_header)
    } > kernel
    
    /* Text section (code) - 64-bit */
    .text : {
        *(.text*)
    } > kernel
    
    /* Read-only data */
    .rodata : {
        *(.rodata*)
    } > kernel
    
    /* Data section (initialized) */
    .data : {
        *(.data*)
    } > kernel
    
    /* BSS section (uninitialized) */
    .bss : {
        *(.bss*)
        *(COMMON)
    } > kernel
    
    /* Heap */
    .heap : {
        . = ALIGN(4096);
        _heap_start = .;
        . += 128M;
        _heap_end = .;
    } > heap
    
    /* Stack (grows downward from end of heap) */
    .stack : {
        . = ALIGN(16);
        _stack_start = .;
        . += 32K;
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
