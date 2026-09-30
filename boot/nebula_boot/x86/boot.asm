; NebulaBoot - NebulaOS Custom Bootloader (x86 BIOS)
; ==================================================
; 
; A custom bootloader with menu for NebulaOS
; Loads kernel directly from disk without GRUB/Multiboot
; 
; Build: nasm -f bin boot.asm -o nebula_boot_x86.bin
;
; Sector layout:
; - Sector 0: Boot sector (this file) - loads stage 2
; - Sector 1+: Stage 2 loader with menu and kernel loading

bits 16
org 0x7C00

jmp short boot_start
nop
times 8 - ($ - $$) db 0
boot_info:
times 56 db 0

; -----------------------------------------------------------------------------
; Boot Sector (Stage 1) - 512 bytes
; -----------------------------------------------------------------------------

boot_start:
    ; BIOS loads us at 0x7C00, DL = boot drive
    cli
    
    ; Set up segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    
    ; Save boot drive
    mov [boot_drive], dl
    
    ; Enable A20 line
    call enable_a20
    
    ; Load the remaining 15 sectors of this El Torito image.
    mov eax, [boot_info]
    inc eax
    mov [dap_lba], eax
    mov dword [dap_lba + 4], 0
    mov word [dap_count], 15
    mov word [dap_offset], 0x7E00
    mov word [dap_segment], 0
    mov si, dap
    mov ah, 0x42
    mov dl, [boot_drive]
    int 0x13
    jc disk_error
    
    jmp 0x0000:0x7E00

; -----------------------------------------------------------------------------
; Enable A20 line
; -----------------------------------------------------------------------------
enable_a20:
    ; Try keyboard controller method
    call a20_wait
    mov al, 0xAD        ; Disable keyboard
    out 0x64, al
    call a20_wait
    mov al, 0xD0        ; Read output port
    out 0x64, al
    call a20_wait2
    in al, 0x60
    push ax
    call a20_wait
    mov al, 0xD1        ; Write output port
    out 0x64, al
    call a20_wait
    pop ax
    or al, 2            ; Set A20 bit
    out 0x60, al
    call a20_wait
    mov al, 0xAE        ; Enable keyboard
    out 0x64, al
    call a20_wait
    ret

a20_wait:
    in al, 0x64
    test al, 2
    jnz a20_wait
    ret

a20_wait2:
    in al, 0x64
    test al, 1
    jz a20_wait2
    ret

; -----------------------------------------------------------------------------
; Disk error handler
; -----------------------------------------------------------------------------
disk_error:
    mov si, msg_disk_error
    call print_string
    cli
    hlt
    jmp $

; -----------------------------------------------------------------------------
; Print string (DS:SI)
; -----------------------------------------------------------------------------
print_string:
    pusha
    mov ah, 0x0E        ; Teletype output
.print_loop:
    lodsb
    test al, al
    jz .done
    int 0x10
    jmp .print_loop
.done:
    popa
    ret

; -----------------------------------------------------------------------------
; Data
; -----------------------------------------------------------------------------
boot_drive db 0
kernel_bytes dd 0
msg_disk_error db "NebulaBoot: Disk read error!", 0

align 4
dap:
    db 0x10, 0
dap_count:
    dw 0
dap_offset:
    dw 0
dap_segment:
    dw 0
dap_lba:
    dq 0

; -----------------------------------------------------------------------------
; Boot signature
; -----------------------------------------------------------------------------
times 510 - ($ - $$) db 0
dw 0xAA55

; ==============================================================================
; STAGE 2 - Menu and Kernel Loader (loaded at 0x10000)
; ==============================================================================

bits 16

stage2_start:
    ; Set up segments
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    
    ; Clear screen
    mov ax, 0x03
    int 0x10
    
    ; Print banner
    mov si, banner
    call print_string
    
    ; Print menu
    mov si, menu
    call print_string
    
    ; Wait for key
    xor ah, ah
    int 0x16
    
    ; Process selection
    cmp al, '1'
    je boot_kernel_32
    cmp al, '2'
    je boot_kernel_64
    cmp al, '3'
    je reboot
    cmp al, '4'
    je shutdown
    
    ; Invalid selection - default to 32-bit
    jmp boot_kernel_32

boot_kernel_32:
    mov si, msg_loading_32
    call print_string
    call load_kernel_32
    jmp $

boot_kernel_64:
    mov si, msg_loading_64
    call print_string
    call load_kernel_64
    jmp $

reboot:
    int 0x19

shutdown:
    mov ax, 0x5307
    mov bx, 0x0001
    mov cx, 0x0003
    int 0x15
    cli
    hlt
    jmp $

; -----------------------------------------------------------------------------
; Load 32-bit kernel (from disk)
; -----------------------------------------------------------------------------
load_kernel_32:
    ; Load the flat kernel appended after the 8KB bootloader image.
    mov eax, [boot_info]
    add eax, 16
    mov [dap_lba], eax
    mov dword [dap_lba + 4], 0

    mov eax, [boot_info + 4]
    sub eax, 8192
    mov [kernel_bytes], eax
    add eax, 511
    shr eax, 9
    cmp eax, 127
    ja disk_error
    mov [dap_count], ax
    mov word [dap_offset], 0
    mov word [dap_segment], 0x2000
    mov si, dap
    mov ah, 0x42
    mov dl, [boot_drive]
    int 0x13
    jc disk_error
    
    ; Switch to protected mode and jump to kernel
    call switch_to_pm32
    jmp $

; -----------------------------------------------------------------------------
; Load 64-bit kernel (from disk)
; -----------------------------------------------------------------------------
load_kernel_64:
    ; For x86_64, we'd typically use UEFI
    ; This is a placeholder - in reality we'd chain to UEFI loader
    mov si, msg_uefi_required
    call print_string
    ret

; -----------------------------------------------------------------------------
; Switch to 32-bit protected mode
; -----------------------------------------------------------------------------
switch_to_pm32:
    cli
    
    ; Load GDT
    lgdt [gdt32_ptr]
    
    ; Enable protected mode
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    
    ; Far jump to 32-bit code
    jmp CODE32_SEL:pm32_start

bits 32
pm32_start:
    ; Set up 32-bit segments
    mov ax, DATA32_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000

    cld
    mov esi, 0x20000
    mov edi, 0x100000
    mov ecx, [kernel_bytes]
    add ecx, 3
    shr ecx, 2
    rep movsd
    jmp CODE32_SEL:0x100000

; -----------------------------------------------------------------------------
; 32-bit GDT
; -----------------------------------------------------------------------------
gdt32_start:
    dq 0                            ; Null descriptor
    dq 0x00CF9A000000FFFF           ; Code segment (0x08)
    dq 0x00CF92000000FFFF           ; Data segment (0x10)
gdt32_end:

gdt32_ptr:
    dw gdt32_end - gdt32_start - 1
    dd gdt32_start

CODE32_SEL equ 0x08
DATA32_SEL equ 0x10

; -----------------------------------------------------------------------------
; Strings
; -----------------------------------------------------------------------------
banner db 13, 10, "========================================", 13, 10
       db "      NebulaBoot - NebulaOS Loader     ", 13, 10
       db "========================================", 13, 10, 13, 10, 0

menu db "Select boot option:", 13, 10
     db "  1. NebulaOS (32-bit)", 13, 10
     db "  2. NebulaOS (64-bit) [UEFI required]", 13, 10
     db "  3. Reboot", 13, 10
     db "  4. Shutdown", 13, 10, 13, 10
     db "Choice: ", 0

msg_loading_32 db "Loading NebulaOS 32-bit kernel...", 13, 10, 0
msg_loading_64 db "Loading NebulaOS 64-bit kernel...", 13, 10, 0
msg_uefi_required db "64-bit mode requires UEFI boot.", 13, 10, 0

; -----------------------------------------------------------------------------
; Pad stage 2 to fill remaining space
; -----------------------------------------------------------------------------
times 8192 - ($ - $$) db 0