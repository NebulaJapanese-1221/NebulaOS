; NebulaBoot - NebulaOS x86 Bootloader (Loaded by NebulaBoot)
; ============================================================
;
; This is the second stage loader that runs in 32-bit protected mode
; Loaded by NebulaBoot stage 1, switches to protected mode and jumps to kernel
;
; NASM syntax

bits 16
org 0x10000

; -----------------------------------------------------------------------------
; Stage 2 Entry (Real Mode)
; -----------------------------------------------------------------------------
_start16:
    ; Set up segments
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    
    ; Enable A20
    call enable_a20
    
    ; Load GDT
    lgdt [gdt32_ptr]
    
    ; Enable protected mode
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    
    ; Far jump to 32-bit code
    jmp CODE32_SEL:_start32

; -----------------------------------------------------------------------------
; Enable A20 line
; -----------------------------------------------------------------------------
enable_a20:
    call a20_wait
    mov al, 0xAD
    out 0x64, al
    call a20_wait
    mov al, 0xD0
    out 0x64, al
    call a20_wait2
    in al, 0x60
    push ax
    call a20_wait
    mov al, 0xD1
    out 0x64, al
    call a20_wait
    pop ax
    or al, 2
    out 0x60, al
    call a20_wait
    mov al, 0xAE
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
; 32-bit Protected Mode Entry
; -----------------------------------------------------------------------------
bits 32
_start32:
    ; Set up 32-bit segments
    mov ax, DATA32_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, stack_top
    
    ; Enable SSE
    mov eax, cr0
    and eax, ~(1 << 2)  ; Clear EM bit
    or eax, (1 << 1)    ; Set MP bit
    mov cr0, eax
    mov eax, cr4
    or eax, (1 << 9) | (1 << 10)  ; OSFXSR | OSXMMEXCPT
    mov cr4, eax
    
    ; Call C kernel entry
    extern kernel_main
    call kernel_main
    
    ; Halt on return
    cli
    hlt
    jmp $

; -----------------------------------------------------------------------------
; GDT for 32-bit Protected Mode
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
; BSS Section
; -----------------------------------------------------------------------------
section .bss
align 16
stack_bottom:
    resb 16384  ; 16KB stack
stack_top:
