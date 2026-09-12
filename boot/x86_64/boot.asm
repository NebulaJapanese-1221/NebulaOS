; NebulaBoot - NebulaOS x86_64 Bootloader (Loaded by NebulaBoot)
; ================================================================
;
; This is the second stage loader that runs in 64-bit mode
; Loaded by NebulaBoot stage 1, jumps directly to kernel
;
; NASM syntax

bits 64

section .text
global _start
extern kernel_main

_start:
    ; We're already in 64-bit long mode (set up by NebulaBoot)
    ; RDI = kernel load address (passed from bootloader)
    
    ; Disable interrupts
    cli
    
    ; Set up 64-bit stack
    mov rsp, stack_top
    
    ; Call C kernel entry
    ; Pass kernel load address in RDI
    call kernel_main
    
    ; Halt on return
    cli
    hlt
    jmp $

; -----------------------------------------------------------------------------
; Data Section
; -----------------------------------------------------------------------------
section .data

; -----------------------------------------------------------------------------
; BSS Section
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 32768  ; 32KB stack for 64-bit
stack_top:
