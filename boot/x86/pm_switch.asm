; NebulaOS Bootloader - Protected Mode Switch
; ============================================
; 
; Code to switch from real mode to 32-bit protected mode

bits 16

; Declare external functions
extern print_string

; -----------------------------------------------------------------------------
; Global Descriptor Table (GDT)
; -----------------------------------------------------------------------------
bits 16

; GDT starts with a null descriptor
_gdt:
    dq 0x0                ; Null segment descriptor
    
    ; Code segment descriptor (0-4GB)
    dw 0xFFFF             ; Limit (bits 0-15)
    dw 0x0                ; Base (bits 0-15)
    db 0x0                ; Base (bits 16-23)
    dw 0x9A00            ; Access byte (present, ring 0, code, executable, direction 0, readable)
    db 0xCF              ; Flags + limit (granularity, 32-bit, limit 0xFFFFF)
    db 0x0                ; Base (bits 24-31)
    
    ; Data segment descriptor (0-4GB)
    dw 0xFFFF             ; Limit (bits 0-15)
    dw 0x0                ; Base (bits 0-15)
    db 0x0                ; Base (bits 16-23)
    dw 0x9200            ; Access byte (present, ring 0, data, expand up, writable)
    db 0xCF              ; Flags + limit (granularity, 32-bit, limit 0xFFFFF)
    db 0x0                ; Base (bits 24-31)

_gdt_end:

; GDT descriptor
gdt_descriptor:
    dw _gdt_end - _gdt - 1  ; Size of GDT
    dd _gdt                  ; Start address of GDT

; Define segment descriptors
CODE_SEG equ _gdt + 8
DATA_SEG equ _gdt + 16

; -----------------------------------------------------------------------------
; switch_to_pm - Switch from real mode to protected mode (stage 1)
; -----------------------------------------------------------------------------
switch_to_pm:
    cli             ; Disable interrupts
    
    ; Print message
    push si
    mov si, pm_msg
    call print_string
    pop si
    
    ; Load GDT
    lgdt [gdt_descriptor]
    
    ; Set PE (Protection Enable) bit in CR0
    mov eax, cr0
    or eax, 0x1
    mov cr0, eax
    
    ; Far jump to 32-bit code segment
    jmp CODE_SEG:init_pm

; -----------------------------------------------------------------------------
; switch_to_pm_kernel - Switch to protected mode and jump to kernel
; Called by stage 2 bootloader
; -----------------------------------------------------------------------------
switch_to_pm_kernel:
    cli             ; Disable interrupts
    
    ; Print message
    push si
    mov si, pm_msg
    call print_string
    pop si
    
    ; Load GDT
    lgdt [gdt_descriptor]
    
    ; Set PE (Protection Enable) bit in CR0
    mov eax, cr0
    or eax, 0x1
    mov cr0, eax
    
    ; Far jump to 32-bit code segment to flush pipeline
    jmp CODE_SEG:init_pm_kernel

; -----------------------------------------------------------------------------
; 32-bit protected mode initialization (stage 1)
; -----------------------------------------------------------------------------
bits 32
init_pm:
    ; Initialize segment registers with data selector
    mov ax, DATA_SEG
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; Set up stack
    mov ebp, 0x90000
    mov esp, ebp
    
    ; Jump to kernel entry point
    ; For now, we'll hang here
    ; In a real implementation: jmp 0x1000
    
    ; Print success message (if we had 32-bit print)
    ; For now, just hang
    jmp $

; -----------------------------------------------------------------------------
; 32-bit protected mode initialization for kernel entry
; -----------------------------------------------------------------------------
bits 32
init_pm_kernel:
    ; Initialize segment registers with data selector
    mov ax, DATA_SEG
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; Set up stack at 2MB (end of kernel memory area)
    mov ebp, 0x00200000
    mov esp, ebp
    
    ; Jump to kernel entry point at 1MB
    ; Kernel entry is at _start symbol
    ; Push multiboot-compatible parameters (0, 0)
    xor eax, eax
    xor ebx, ebx
    
    ; Jump to kernel entry at 0x100000
    jmp CODE_SEG:0x00100000

bits 16

; Message
pm_msg db "Switched to 32-bit protected mode", 0x0D, 0x0A, 0
