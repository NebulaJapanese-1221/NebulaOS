; NebulaOS x86 Bootloader
; ==========================
; 
; Stage 1 Bootloader for x86 architecture
; Loads at 0x7C00, loads stage 2, then stage 2 loads kernel
; 
; NASM syntax

bits 16

; -----------------------------------------------------------------------------
; Constants
; -----------------------------------------------------------------------------

KERNEL_LOAD_ADDR equ 0x00100000  ; Load kernel at 1MB
STAGE2_LOAD_ADDR equ 0x00007E00  ; Load stage 2 at 0x7E00
SECTORS_PER_TRACK equ 18
HEAD_COUNT equ 2

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
    
    ; Load stage 2 bootloader from disk
    mov si, loading_stage2_msg
    call print_string
    
    ; Set ES:BX to stage 2 load address
    mov ax, 0x0000
    mov es, ax
    mov bx, STAGE2_LOAD_ADDR
    
    ; Load 4 sectors starting from sector 2
    mov dl, [boot_drive]
    mov cx, 0x0002           ; Cylinder 0, Sector 2
    mov dh, 0x00             ; Head 0
    mov al, 4                ; Number of sectors
    call disk_load_extended
    
    jc .stage2_load_failed
    
    ; Verify stage 2 signature
    mov si, STAGE2_LOAD_ADDR
    cmp dword [si], 0x55AA55AA
    jne .stage2_load_failed
    
    ; Jump to stage 2
    mov si, switching_msg
    call print_string
    
    jmp STAGE2_LOAD_ADDR
    
.stage2_load_failed:
    mov si, stage2_fail_msg
    call print_string
    jmp $

; -----------------------------------------------------------------------------
; Include disk functions for stage 1
; -----------------------------------------------------------------------------

%include "boot/x86/disk.asm"

; -----------------------------------------------------------------------------
; Data section
; -----------------------------------------------------------------------------

welcome_msg db "NebulaOS Bootloader v1.0", 0x0D, 0x0A, 0
loading_stage2_msg db "Loading stage 2...", 0x0D, 0x0A, 0
switching_msg db "Switching to protected mode...", 0x0D, 0x0A, 0
stage2_fail_msg db "ERROR: Failed to load stage 2!", 0x0D, 0x0A, 0

boot_drive db 0

; -----------------------------------------------------------------------------
; Boot signature
; -----------------------------------------------------------------------------

times 510 - ($ - $$) db 0
 dw 0xAA55

; -----------------------------------------------------------------------------
; Stage 2 Bootloader
; This will be loaded at 0x7E00 by stage 1
; -----------------------------------------------------------------------------

; Stage 2 signature (must match what stage 1 checks)
stage2_signature: dd 0x55AA55AA

; -----------------------------------------------------------------------------
; Include functions for stage 2
; -----------------------------------------------------------------------------

%include "boot/x86/pm_switch.asm"
%include "boot/x86/print.asm"

; Stage 2 entry point
stage2_start:
    ; Set up segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, STAGE2_LOAD_ADDR  ; Stack at end of stage 2
    
    ; Print message
    mov si, stage2_welcome_msg
    call print_string
    
    ; Load kernel from disk
    ; Kernel is located starting at sector 7 (after stage 1 and stage 2)
    ; For now, we'll use BIOS int 13h extensions if available
    
    mov si, loading_kernel_msg
    call print_string
    
    ; Check for LBA support (int 13h extensions)
    mov ah, 0x41
    mov bx, 0x55AA
    int 0x13
    jc .no_lba
    cmp bx, 0xAA55
    jne .no_lba
    
    ; LBA supported - use extended read
    ; Load kernel at 0x100000 (1MB)
    mov ax, 0x0000
    mov es, ax
    mov edi, KERNEL_LOAD_ADDR
    
    ; Load 128 sectors (64KB) starting from LBA sector 6
    ; (sector 1 = boot, sectors 2-5 = stage 2, sector 6+ = kernel)
    mov ecx, 6   ; Starting LBA sector
    mov eax, 128 ; Number of sectors to read
    call read_sectors_lba
    
    jc .kernel_load_failed
    jmp .kernel_loaded
    
.no_lba:
    ; Use CHS addressing
    mov ax, KERNEL_LOAD_ADDR >> 4
    mov es, ax
    xor bx, bx
    
    mov dl, [boot_drive]
    mov cx, 0x0007                ; Cylinder 0, Sector 7
    mov dh, 0x00                 ; Head 0
    
    ; Load kernel (128 sectors)
    mov di, 128
.load_loop:
    push cx
    push dx
    mov dh, 1                    ; Read 1 sector at a time
    call disk_load
    pop dx
    pop cx
    jc .kernel_load_failed
    
    ; Advance to next sector
    inc cl
    cmp cl, SECTORS_PER_TRACK + 1
    jb .next_sector
    
    ; Move to next head
    mov cl, 1
    inc dh
    cmp dh, HEAD_COUNT
    jb .next_sector
    
    ; Move to next cylinder
    mov cl, 1
    xor dh, dh
    inc ch
    
.next_sector:
    ; Advance buffer by 512 bytes
    add bx, 512
    cmp bx, 0
    jne .no_segment_wrap
    mov ax, es
    add ax, 0x1000
    mov es, ax
.no_segment_wrap:
    
    dec di
    jnz .load_loop
    
.kernel_loaded:
    ; Kernel loaded successfully
    mov si, kernel_loaded_msg
    call print_string
    
    ; Switch to protected mode and jump to kernel
    call switch_to_pm_kernel
    
    jmp $
    
.kernel_load_failed:
    mov si, kernel_fail_msg
    call print_string
    jmp $

; -----------------------------------------------------------------------------
; LBA disk read function (int 13h extensions)
; Input: ECX = LBA sector, EAX = number of sectors, ES:EDI = destination
; -----------------------------------------------------------------------------
read_sectors_lba:
    pusha
    
    ; Convert LBA to CHS for compatibility
    ; For simplicity, we'll use extended read (AH=42h)
    
    ; Build DAP (Disk Address Packet)
    push es
    push edi
    
    ; DAP structure (16 bytes):
    ; Byte 0: Packet size (0x10)
    ; Byte 1: Reserved (0)
    ; Bytes 2-3: Number of blocks to transfer
    ; Bytes 4-7: Transfer buffer address (48-bit)
    ; Bytes 8-15: LBA address (48-bit)
    
    mov si, dap_buffer
    mov byte [si], 0x10       ; Packet size
    mov byte [si+1], 0        ; Reserved
    mov word [si+2], ax       ; Number of blocks (low word)
    mov word [si+4], di       ; Buffer offset
    mov word [si+6], es       ; Buffer segment
    mov dword [si+8], ecx     ; LBA address (low 32 bits)
    mov dword [si+12], 0      ; LBA address (high 32 bits)
    
    ; Call int 13h AH=42h
    mov ah, 0x42
    mov dl, [boot_drive]
    mov si, dap_buffer
    int 0x13
    
    pop edi
    pop es
    popa
    ret

; DAP buffer (16 bytes)
dap_buffer: times 16 db 0

; -----------------------------------------------------------------------------
; Stage 2 data
; -----------------------------------------------------------------------------

stage2_welcome_msg db "Stage 2: Loading NebulaOS kernel...", 0x0D, 0x0A, 0
loading_kernel_msg db "Loading kernel...", 0x0D, 0x0A, 0
kernel_loaded_msg db "Kernel loaded. Switching to 32-bit mode...", 0x0D, 0x0A, 0
kernel_fail_msg db "ERROR: Failed to load kernel!", 0x0D, 0x0A, 0

; Pad stage 2 to 4 sectors (2048 bytes)
times 2048 - ($ - stage2_signature) db 0
