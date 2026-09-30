; NebulaOS x86 Kernel Entry
; ==========================
; 
; Assembly entry point for x86 kernel
; Loaded directly by NebulaBoot (no multiboot)
; Already in 32-bit protected mode when called

bits 32

section .multiboot_header
align 4
dd 0x1BADB002
dd 0x00000003
dd -(0x1BADB002 + 0x00000003)

; -----------------------------------------------------------------------------
; Kernel entry point
; -----------------------------------------------------------------------------
section .text.start
global _start
extern kernel_main
extern __bss_start
extern __bss_end

_start:
    cli
    cld

    lgdt [gdt_descriptor]
    jmp 0x08:.reload_segments

.reload_segments:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, stack_top
    mov edi, __bss_start
    mov ecx, __bss_end
    sub ecx, edi
    xor eax, eax
    rep stosb

    call kernel_main

.halt:
    cli
    hlt
    jmp .halt

section .rodata
align 8
gdt_start:
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
gdt_end:
gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

; -----------------------------------------------------------------------------
; Data section
; -----------------------------------------------------------------------------
section .data

; -----------------------------------------------------------------------------
; BSS section (uninitialized data)
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 16384  ; 16KB stack
stack_top:
