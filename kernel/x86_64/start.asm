; NebulaOS x86_64 Kernel Entry
; =============================
; 
; Assembly entry point for x86_64 kernel
; Loaded directly by NebulaBoot (no multiboot)
; Already in 64-bit long mode when called

bits 64

; -----------------------------------------------------------------------------
; Kernel entry point
; -----------------------------------------------------------------------------
section .text
global _start
extern kernel_main

_start:
    ; Already in 64-bit long mode (set up by NebulaBoot)
    ; RDI = kernel load address
    
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
; Data section
; -----------------------------------------------------------------------------
section .data

; -----------------------------------------------------------------------------
; BSS section (uninitialized data)
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 32768  ; 32KB stack for 64-bit
stack_top:
