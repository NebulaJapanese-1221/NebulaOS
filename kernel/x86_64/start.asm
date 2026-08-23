; NebulaOS x86_64 Kernel Entry
; =============================
; 
; Assembly entry point for x86_64 kernel
; Multiboot2-compliant entry point for GRUB2 bootloader
; GRUB2 switches to 64-bit long mode before calling this entry point

bits 64

; -----------------------------------------------------------------------------
; Multiboot2 Header
; -----------------------------------------------------------------------------
MB2_MAGIC    equ 0xE85250D6
MB2_ARCH     equ 0           ; i386 architecture (GRUB handles x86_64)
MB2_ALIGN    equ 8

MB2_TAG_TYPE_END        equ 0
MB2_TAG_TYPE_MEMORY     equ 4
MB2_TAG_TYPE_FRAMEBUFFER equ 8
MB2_TAG_FLAG_REQUIRED   equ 0

section .multiboot_header
align 8
multiboot_header:
    dd MB2_MAGIC
    dd MB2_ARCH
    dd multiboot_header_end - multiboot_header

    ; Request memory map
    align 8
    .tag_memory:
        dw MB2_TAG_TYPE_MEMORY
        dw MB2_TAG_FLAG_REQUIRED
        dd 16 - 8

    ; Request framebuffer
    align 8
    .tag_framebuffer:
        dw MB2_TAG_TYPE_FRAMEBUFFER
        dw MB2_TAG_FLAG_REQUIRED
        dd 24 - 8
        dd 0, 0, 0

    ; End tag
    align 8
    .tag_end:
        dw MB2_TAG_TYPE_END
        dw MB2_TAG_FLAG_REQUIRED
        dd 8 - 8

multiboot_header_end:

; -----------------------------------------------------------------------------
; Kernel entry point
; -----------------------------------------------------------------------------
section .text
global _start
extern kernel_main

_start:
    ; GRUB2 has switched to 64-bit long mode
    ; RBX = multiboot2 info structure pointer
    
    ; Disable interrupts
    cli
    
    ; Save multiboot info pointer
    mov [multiboot_info], rbx
    
    ; Set up 64-bit stack
    mov rsp, stack_top
    
    ; Call C kernel entry
    ; Pass multiboot info pointer in RDI (first argument)
    mov rdi, rbx
    call kernel_main
    
    ; Halt on return
    cli
    hlt
    jmp $

; -----------------------------------------------------------------------------
; Data section
; -----------------------------------------------------------------------------
section .data
multiboot_info dq 0

; -----------------------------------------------------------------------------
; BSS section (uninitialized data)
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 32768  ; 32KB stack for 64-bit
stack_top:
