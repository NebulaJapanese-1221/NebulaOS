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
extern __bss_start
extern __bss_end

_start:
    ; Already in 64-bit long mode (set up by NebulaBoot)
    ; RDI = kernel load address
    
    ; Disable interrupts
    cli
    cld
    mov r12, rdi
    
    ; Set up 64-bit stack
    mov rsp, stack_top
    lea rdi, [rel __bss_start]
    lea rcx, [rel __bss_end]
    sub rcx, rdi
    xor eax, eax
    rep stosb
    mov rdi, r12
    
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
