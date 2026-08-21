; NebulaOS Bootloader - Protected Mode & Long Mode Switch
; ========================================================
; 
; Code to switch from real mode to 32-bit protected mode,
; then to 64-bit long mode

bits 16

; -----------------------------------------------------------------------------
; switch_to_pm - Switch from real mode to protected mode
; This is the first step towards long mode
; -----------------------------------------------------------------------------
switch_to_pm:
    cli             ; Disable interrupts
    
    ; Print message
    push si
    mov si, pm_msg
    call print_string
    pop si
    
    ; Load GDT (32-bit compatible)
    lgdt [gdt32_descriptor]
    
    ; Set PE (Protection Enable) bit in CR0
    mov eax, cr0
    or eax, 0x1
    mov cr0, eax
    
    ; Far jump to 32-bit code segment
    jmp CODE32_SEG:init_pm

; -----------------------------------------------------------------------------
; 32-bit protected mode initialization
; -----------------------------------------------------------------------------
bits 32
init_pm:
    ; Initialize segment registers with data selector
    mov ax, DATA32_SEG
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; Set up stack
    mov ebp, 0x90000
    mov esp, ebp
    
    ; Now set up for long mode
    call setup_long_mode
    
    ; Switch to long mode
    call switch_to_long_mode
    
    ; We should never get here
    jmp $

; -----------------------------------------------------------------------------
; setup_long_mode - Configure CPU for long mode
; -----------------------------------------------------------------------------
setup_long_mode:
    ; Check for long mode support (already checked in boot.asm)
    ; Now we need to set up the 64-bit GDT and enable PAE paging
    
    ; Step 1: Enable PAE (Physical Address Extension) in CR4
    mov eax, cr4
    or eax, 1 << 5  ; PAE bit
    mov cr4, eax
    
    ; Step 2: Set up Page Directory Pointer Table (PDPT)
    ; For simplicity, we'll use a minimal 4-level paging structure
    ; Map first 2MB to identity mapping
    
    ; Build a temporary PDPT at 0x9000
    mov edi, 0x9000
    
    ; PDPT entry 0: points to PDT at 0x9100
    mov dword [edi + 0], 0x9100 | 0x83  ; present, writable, PWT, PCD
    mov dword [edi + 4], 0x0          ; upper 32 bits
    
    ; Fill rest of PDPT with zeros
    mov ecx, 3
    add edi, 8
    xor eax, eax
    rep stosd
    
    ; Build PDT at 0x9100
    mov edi, 0x9100
    
    ; PDT entry 0: points to PT at 0x9200
    mov dword [edi + 0], 0x9200 | 0x83
    mov dword [edi + 4], 0x0
    
    ; Fill rest of PDT with zeros
    mov ecx, 511
    add edi, 8
    xor eax, eax
    rep stosd
    
    ; Build PT at 0x9200 (2MB page)
    mov edi, 0x9200
    mov dword [edi + 0], 0x83        ; 2MB page, present, writable
    mov dword [edi + 4], 0x0
    
    ; Fill rest of PT with zeros
    mov ecx, 511
    add edi, 8
    xor eax, eax
    rep stosd
    
    ; Step 3: Load PDPT address into CR3
    mov eax, 0x9000
    mov cr3, eax
    
    ; Step 4: Enable long mode in EFER (Extended Feature Enable Register)
    mov ecx, 0xC0000080  ; EFER MSR
    rdmsr
    or eax, 1 << 8       ; LM bit
    wrmsr
    
    ; Step 5: Enable paging in CR0
    mov eax, cr0
    or eax, 1 << 31      ; PG bit
    mov cr0, eax
    
    ret

; -----------------------------------------------------------------------------
; switch_to_long_mode - Switch to 64-bit long mode
; -----------------------------------------------------------------------------
switch_to_long_mode:
    ; Load 64-bit GDT
    lgdt [gdt64_descriptor]
    
    ; Far jump to 64-bit code segment
    jmp CODE64_SEG:init_long_mode

; -----------------------------------------------------------------------------
; 64-bit long mode initialization
; -----------------------------------------------------------------------------
bits 64
init_long_mode:
    ; Initialize segment registers with 64-bit data selector
    mov ax, DATA64_SEG
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; Set up 64-bit stack
    mov rbp, 0x900000
    mov rsp, rbp
    
    ; Jump to kernel entry point
    ; For now, just hang
    jmp $

; -----------------------------------------------------------------------------
; Global Descriptor Tables
; -----------------------------------------------------------------------------
bits 32

; 32-bit GDT (for protected mode)
_gdt32:
    dq 0x0                ; Null segment descriptor
    
    ; 32-bit Code segment
    dw 0xFFFF             ; Limit (bits 0-15)
    dw 0x0                ; Base (bits 0-15)
    db 0x0                ; Base (bits 16-23)
    dw 0xCF9A            ; Access + flags (present, ring 0, code, 32-bit)
    db 0xCF              ; Flags + limit (bits 16-19)
    db 0x0                ; Base (bits 24-31)
    
    ; 32-bit Data segment
    dw 0xFFFF             ; Limit (bits 0-15)
    dw 0x0                ; Base (bits 0-15)
    db 0x0                ; Base (bits 16-23)
    dw 0xCF92            ; Access + flags (present, ring 0, data, 32-bit)
    db 0xCF              ; Flags + limit (bits 16-19)
    db 0x0                ; Base (bits 24-31)

_gdt32_end:

gdt32_descriptor:
    dw _gdt32_end - _gdt32 - 1
    dd _gdt32

CODE32_SEG equ _gdt32 + 8
DATA32_SEG equ _gdt32 + 16

; 64-bit GDT (for long mode)
_gdt64:
    dq 0x0                ; Null segment descriptor
    
    ; 64-bit Code segment
    dw 0x0                ; Limit (bits 0-15) - ignored in long mode
    dw 0x0                ; Base (bits 0-15)
    db 0x0                ; Base (bits 16-23)
    dw 0xAF9A            ; Access + flags (present, ring 0, code, 64-bit)
    db 0x00              ; Flags + limit (bits 16-19)
    db 0x0                ; Base (bits 24-31)
    
    ; 64-bit Data segment
    dw 0x0                ; Limit (bits 0-15)
    dw 0x0                ; Base (bits 0-15)
    db 0x0                ; Base (bits 16-23)
    dw 0xCF92            ; Access + flags (present, ring 0, data)
    db 0x00              ; Flags + limit (bits 16-19)
    db 0x0                ; Base (bits 24-31)

_gdt64_end:

gdt64_descriptor:
    dw _gdt64_end - _gdt64 - 1
    dq _gdt64

CODE64_SEG equ _gdt64 + 8
DATA64_SEG equ _gdt64 + 16

; Messages
pm_msg db "Switching to 32-bit protected mode...", 0x0D, 0x0A, 0
lm_msg db "Switching to 64-bit long mode...", 0x0D, 0x0A, 0
