; NebulaOS x86_64 GRUB Multiboot2 Bootloader
; ===========================================
;
; Multiboot2-compliant bootloader for x86_64 architecture
; Loaded by GRUB2, which provides multiboot information
; GRUB2 switches to 64-bit long mode before calling this entry point
;
; NASM syntax

; -----------------------------------------------------------------------------
; Multiboot2 Header (32-bit compatible data)
; -----------------------------------------------------------------------------
MB2_MAGIC    equ 0xE85250D6
MB2_ARCH     equ 0           ; i386 architecture
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
; Kernel Entry Point (64-bit)
; -----------------------------------------------------------------------------
bits 64

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
; Simple VGA output for early boot messages
; -----------------------------------------------------------------------------
vga_puts:
    push rax
    push rdi
    mov edi, (VGA_BUFFER + 0x0F00)  ; White on blue, start at row 24
.next_char:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0F
    stosw
    jmp .next_char
.done:
    pop rdi
    pop rax
    ret

; -----------------------------------------------------------------------------
; Data Section
; -----------------------------------------------------------------------------
section .data
boot_msg db "NebulaOS x86_64 Kernel Booting via GRUB2...", 0

; Multiboot info storage
multiboot_info dq 0

; VGA constants
VGA_BUFFER equ 0xB8000

; -----------------------------------------------------------------------------
; BSS Section
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 32768  ; 32KB stack for 64-bit
stack_top:
