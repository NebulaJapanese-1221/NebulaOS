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
    
    ; The El Torito entry preloads all 16 sectors of this boot image.
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
blocks_remaining dd 0
load_segment dw 0
chunk_blocks dw 0
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
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    
    mov ax, 0x03
    int 0x10

    mov si, banner
    call print_string

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
    ; xorriso stores the boot image LBA and total image length in this table.
    mov eax, [boot_info + 4]
    add eax, 4
    mov [dap_lba], eax
    mov dword [dap_lba + 4], 0

    mov eax, [boot_info + 8]
    sub eax, 8192
    mov [kernel_bytes], eax
    test eax, eax
    jz disk_error
    cmp eax, 0x70000
    ja disk_error

    ; El Torito BIOS reads use 2048-byte CD blocks. Keep each transfer below
    ; 64 KiB and within the temporary buffer below the real-mode stack.
    add eax, 2047
    shr eax, 11
    mov [blocks_remaining], eax
    mov word [load_segment], 0x2000

.read_kernel:
    mov eax, [blocks_remaining]
    test eax, eax
    jz .kernel_loaded
    cmp eax, 31
    jbe .chunk_ready
    mov eax, 31
.chunk_ready:
    mov [chunk_blocks], ax
    mov [dap_count], ax
    mov word [dap_offset], 0
    mov ax, [load_segment]
    mov [dap_segment], ax
    mov si, dap
    mov ah, 0x42
    mov dl, [boot_drive]
    int 0x13
    jc disk_error

    movzx eax, word [chunk_blocks]
    add [dap_lba], eax
    adc dword [dap_lba + 4], 0
    sub [blocks_remaining], eax
    shl eax, 7
    add [load_segment], ax
    jmp .read_kernel

.kernel_loaded:
    call set_graphics_mode
    jc graphics_error
    call collect_memory_map

    ; Switch to protected mode and jump to kernel
    call switch_to_pm32
    jmp $

collect_memory_map:
    push es
    pushad
    mov ax, 0x9100
    mov es, ax
    xor ebx, ebx
    xor bp, bp
.next_region:
    cmp bp, 128
    jae .done
    mov di, bp
    imul di, 24
    mov dword [es:di + 20], 1
    mov eax, 0xE820
    mov edx, 0x534D4150
    mov ecx, 24
    int 0x15
    jc .done
    cmp eax, 0x534D4150
    jne .done
    inc bp
    test ebx, ebx
    jnz .next_region
.done:
    mov dword [0x502C], 0x91000
    movzx eax, bp
    mov [0x5030], eax
    popad
    pop es
    ret

; Configure a VBE linear framebuffer while BIOS services are still available.
; The kernel reads the resulting handoff structure at physical address 0x5000.
set_graphics_mode:
    push es
    xor ax, ax
    mov es, ax
    mov di, 0x6000
    mov cx, 128
    cld
    rep stosw

    mov ax, 0x4F01
    mov cx, 0x0118
    int 0x10
    cmp ax, 0x004F
    jne .failed
    xor ax, ax
    mov es, ax

    ; Require a supported graphics mode with a linear framebuffer.
    mov ax, [es:0x6000]
    and ax, 0x0091
    cmp ax, 0x0091
    jne .failed
    cmp byte [es:0x6019], 24
    je .valid_bpp
    cmp byte [es:0x6019], 32
    jne .failed
.valid_bpp:
    cmp byte [es:0x601B], 6
    jne .failed
    cmp dword [es:0x6028], 0
    je .failed

    ; Store framebuffer, dimensions, pitch, pixel depth, and RGB layout.
    mov eax, [es:0x6028]
    mov [0x5000], eax
    movzx eax, word [es:0x6012]
    mov [0x5004], eax
    movzx eax, word [es:0x6014]
    mov [0x5008], eax
    movzx eax, word [es:0x6032]
    test eax, eax
    jnz .have_pitch
    movzx eax, word [es:0x6010]
.have_pitch:
    mov [0x500C], eax
    movzx eax, byte [es:0x6019]
    mov [0x5010], eax

    movzx eax, byte [es:0x6036]
    test eax, eax
    jnz .have_red
    movzx eax, byte [es:0x601F]
.have_red:
    mov [0x5014], eax
    movzx eax, byte [es:0x6037]
    test eax, eax
    jnz .have_red_pos
    movzx eax, byte [es:0x6020]
.have_red_pos:
    mov [0x5018], eax
    movzx eax, byte [es:0x6038]
    test eax, eax
    jnz .have_green
    movzx eax, byte [es:0x6021]
.have_green:
    mov [0x501C], eax
    movzx eax, byte [es:0x6039]
    test eax, eax
    jnz .have_green_pos
    movzx eax, byte [es:0x6022]
.have_green_pos:
    mov [0x5020], eax
    movzx eax, byte [es:0x603A]
    test eax, eax
    jnz .have_blue
    movzx eax, byte [es:0x6023]
.have_blue:
    mov [0x5024], eax
    movzx eax, byte [es:0x603B]
    test eax, eax
    jnz .have_blue_pos
    movzx eax, byte [es:0x6024]
.have_blue_pos:
    mov [0x5028], eax

    mov ax, 0x4F02
    mov bx, 0x4118
    int 0x10
    cmp ax, 0x004F
    jne .failed
    clc
    pop es
    ret
.failed:
    stc
    pop es
    ret

graphics_error:
    mov si, msg_graphics_error
    call print_string
    cli
.halt:
    hlt
    jmp .halt

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
msg_graphics_error db "VESA graphics mode unavailable; stopping in text mode.", 13, 10, 0

; -----------------------------------------------------------------------------
; Pad stage 2 to fill remaining space
; -----------------------------------------------------------------------------
times 8192 - ($ - $$) db 0