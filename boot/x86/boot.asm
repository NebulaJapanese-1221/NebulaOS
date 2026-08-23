; NebulaOS x86 GRUB Multiboot Bootloader
; ========================================
;
; Multiboot-compliant bootloader for x86 architecture
; Loaded by GRUB, which provides multiboot information
;
; NASM syntax

bits 32

; -----------------------------------------------------------------------------
; Multiboot Header
; -----------------------------------------------------------------------------
; Must be aligned to 4 bytes and in the first 8KB of the kernel
; Multiboot magic: 0x1BADB002
; Flags: 0x00010003 (align modules, request memory map, request graphics)

MBALIGN     equ  1 << 0
MEMINFO     equ  1 << 1
GRAPHICS    equ  1 << 2
FLAGS       equ  MBALIGN | MEMINFO
MAGIC       equ  0x1BADB002
CHECKSUM    equ  -(MAGIC + FLAGS)

section .multiboot_header
align 4
multiboot_header:
    dd MAGIC
    dd FLAGS
    dd CHECKSUM
    dd 0x1000      ; header_addr
    dd 0x1000      ; load_addr
    dd 0            ; load_end_addr
    dd 0            ; bss_end_addr
    dd 0            ; entry_addr

; -----------------------------------------------------------------------------
; Kernel Entry Point
; -----------------------------------------------------------------------------
section .text
global _start
extern kernel_main

_start:
    ; Disable interrupts
    cli

    ; Set up stack
    mov esp, stack_top

    ; Save multiboot magic and info pointer
    ; EAX = multiboot magic (0x2BADB002)
    ; EBX = multiboot info structure pointer
    mov [multiboot_magic], eax
    mov [multiboot_info], ebx

    ; Print boot message using VGA (since we're in protected mode)
    mov esi, boot_msg
    call vga_puts

    ; Call C kernel entry
    push ebx        ; multiboot info pointer
    push eax        ; multiboot magic
    call kernel_main

    ; If kernel returns, halt
.halt:
    cli
    hlt
    jmp .halt

; -----------------------------------------------------------------------------
; Simple VGA output for early boot messages
; -----------------------------------------------------------------------------
vga_puts:
    pusha
    mov edi, (VGA_BUFFER + 0x0F00)  ; White on blue, start at row 24
.next_char:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0F
    stosw
    jmp .next_char
.done:
    popa
    ret

; -----------------------------------------------------------------------------
; Data Section
; -----------------------------------------------------------------------------
section .data
boot_msg db "NebulaOS x86 Kernel Booting via GRUB...", 0

; Multiboot info storage
multiboot_magic dd 0
multiboot_info  dd 0

; VGA constants
VGA_BUFFER equ 0xB8000

; -----------------------------------------------------------------------------
; BSS Section
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 16384      ; 16KB stack
stack_top:
