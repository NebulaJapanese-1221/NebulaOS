; NebulaOS x86 Kernel Entry
; ==========================
; 
; Assembly entry point for x86 kernel
; This is the first C code entry point after bootloader

bits 32

; -----------------------------------------------------------------------------
; Multiboot header (for compatibility with multiboot bootloaders)
; -----------------------------------------------------------------------------
MULTIBOOT_HEADER_MAGIC equ 0x1BADB002
MULTIBOOT_HEADER_FLAGS equ 0x00000003
MULTIBOOT_CHECKSUM equ -(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)

section .multiboot_header
align 4
multiboot_header:
    dd MULTIBOOT_HEADER_MAGIC
    dd MULTIBOOT_HEADER_FLAGS
    dd MULTIBOOT_CHECKSUM

; -----------------------------------------------------------------------------
; Kernel entry point
; -----------------------------------------------------------------------------
section .text
global _start
_start:
    ; Bootloader has switched to protected mode
    ; ES:EAX = Multiboot info structure (if using multiboot)
    ; EBX = Multiboot magic number
    
    ; Save multiboot info pointer
    mov [multiboot_ptr], eax
    mov [multiboot_magic], ebx
    
    ; Set up stack
    mov esp, stack_top
    
    ; Push multiboot info to stack for C entry
    push eax
    push ebx
    
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
multiboot_ptr dd 0
multiboot_magic dd 0

; -----------------------------------------------------------------------------
; BSS section (uninitialized data)
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 16384  ; 16KB stack
stack_top:
