; NebulaOS x86_64 Kernel Entry
; =============================
; 
; Assembly entry point for x86_64 kernel
; This is the first C code entry point after bootloader

bits 64

; -----------------------------------------------------------------------------
; Kernel entry point
; -----------------------------------------------------------------------------
section .text
global _start
_start:
    ; Bootloader has switched to long mode
    ; RDI = Boot info pointer (architecture specific)
    
    ; Set up stack
    mov rsp, stack_top
    
    ; Save boot info pointer
    mov [boot_info_ptr], rdi
    
    ; Push boot info to stack for C entry
    push rdi
    
    ; Call C kernel entry
    extern kernel_main
    call kernel_main
    
    ; Halt on return
    cli
    hlt
    jmp $

; -----------------------------------------------------------------------------
; Data section
; -----------------------------------------------------------------------------
section .data
boot_info_ptr dq 0

; -----------------------------------------------------------------------------
; BSS section (uninitialized data)
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 32768  ; 32KB stack for 64-bit
stack_top:
