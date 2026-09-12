; NebulaOS x86 Kernel Entry
; ==========================
; 
; Assembly entry point for x86 kernel
; Loaded directly by NebulaBoot (no multiboot)
; Already in 32-bit protected mode when called

bits 32

; -----------------------------------------------------------------------------
; Kernel entry point
; -----------------------------------------------------------------------------
section .text
global _start
extern kernel_main

_start:
    ; Already in 32-bit protected mode (set up by NebulaBoot)
    
    ; Disable interrupts
    cli

    ; Set up stack from linker script symbols
    mov esp, stack_top

    ; Call C kernel entry
    call kernel_main
    
    ; Halt on return
    cli
    hlt
    jmp $

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
