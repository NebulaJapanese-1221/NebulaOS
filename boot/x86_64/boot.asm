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
    cli
    
    mov [boot_drive], dl
    
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    
    sti
    
    mov si, welcome_msg
    call print_string
    
    call check_cpuid
    cmp eax, 1
    jne no_cpuid
    
    call check_long_mode
    cmp eax, 1
    jne no_long_mode
    
    call switch_to_pm
    
no_cpuid:
    mov si, no_cpuid_msg
    call print_string
    jmp halt

no_long_mode:
    mov si, no_long_mode_msg
    call print_string
    jmp halt

halt:
    cli
    hlt
    jmp halt

; -----------------------------------------------------------------------------
; Boot signature
; -----------------------------------------------------------------------------

times 510 - ($ - $$) db 0
 dw 0xAA55

; -----------------------------------------------------------------------------
; Stage 2 would start here (loaded at different address)
; -----------------------------------------------------------------------------

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
