; NebulaOS x86_64 Bootloader
; ===========================
; 
; Bootloader for x86_64 architecture
; Supports both BIOS and UEFI (BIOS legacy mode shown here)
; Switches to 64-bit long mode
; 
; NASM syntax

bits 16

; -----------------------------------------------------------------------------
; Boot sector entry point
; -----------------------------------------------------------------------------
start:
    ; Disable interrupts
    cli
    
    ; Save boot drive number (DL = boot drive)
    mov [boot_drive], dl
    
    ; Set up segment registers
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00  ; Stack grows downward from 0x7C00
    
    ; Enable interrupts (temporarily)
    sti
    
    ; Print welcome message
    mov si, welcome_msg
    call print_string
    
    ; For now, skip CPUID and long mode checks to fit in boot sector
    ; These would be done in stage 2
    
    ; Switch to 32-bit protected mode first (required for long mode setup)
    ; This call won't work until we have a stage 2 loader
    ; jmp to a simple halt for now
    jmp $

; -----------------------------------------------------------------------------
; Boot signature
; -----------------------------------------------------------------------------

times 510 - ($ - $$) db 0
 dw 0xAA55

; -----------------------------------------------------------------------------
; Stage 2 would start here (loaded at different address)
; -----------------------------------------------------------------------------

; Include functions for stage 2
%include "boot/x86_64/print.asm"
%include "boot/x86_64/pm_switch.asm"
%include "boot/x86_64/cpuid.asm"

; -----------------------------------------------------------------------------
; Data section
; -----------------------------------------------------------------------------

welcome_msg db "NebulaOS 64-bit Bootloader...", 0x0D, 0x0A, 0
no_cpuid_msg db "ERROR: CPUID not supported", 0x0D, 0x0A, 0
no_long_mode_msg db "ERROR: 64-bit long mode not supported", 0x0D, 0x0A, 0

boot_drive db 0
