; NebulaOS x86 Kernel Entry
; ==========================
; 
; Assembly entry point for x86 kernel
; Multiboot-compliant entry point for GRUB bootloader
; This is the first C code entry point after bootloader
; Note: boot.asm contains the actual multiboot header and entry point.
;       This file is kept for reference but is not linked into the kernel.

bits 32

; -----------------------------------------------------------------------------
; Kernel entry point
; -----------------------------------------------------------------------------
section .text
global _start
extern kernel_main

_start:
    ; Bootloader has loaded us in protected mode via GRUB
    ; EAX = multiboot magic number (0x2BADB002)
    ; EBX = multiboot info structure pointer
    
    ; Disable interrupts
    cli

    ; Save multiboot info
    mov [multiboot_magic], eax
    mov [multiboot_info], ebx

    ; Set up stack from linker script symbols
    mov esp, stack_top

    ; Push arguments for kernel_main
    push ebx        ; multiboot info pointer
    push eax        ; multiboot magic
    
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
multiboot_magic dd 0
multiboot_info  dd 0

; -----------------------------------------------------------------------------
; BSS section (uninitialized data)
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 16384  ; 16KB stack
stack_top:
