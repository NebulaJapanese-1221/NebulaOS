; ISR stubs and common handler
; Interrupts without error code: push 0 + irq, jmp common
; Interrupts with error code (CPU pushes): push irq, jmp common

[bits 32]
section .text

global isr_stubs_begin
global isr_stubs_end
global isr_common
global _start
global idt_load
global load_gdt
global load_page_dir
global load_tr
global isr_stub_table
extern kernel_entry
extern schedule_rust
extern isr_handler_rust
extern __bss_start
extern __bss_end
extern SCHEDULED_CR3

section .multiboot2
align 8
multiboot2_header:
    dd 0xE85250D6
    dd 0
    dd multiboot2_header_end - multiboot2_header
    dd -(0xE85250D6 + (multiboot2_header_end - multiboot2_header))
    dw 5
    dw 0
    dd 20
    dd 1024
    dd 768
    dd 32
    dd 0
    dd 0
    dd 8
multiboot2_header_end:

section .text.boot
_start:
    cli
    cmp eax, 0x36D76289
    jne .halt
    mov esp, 0x90000
    xor ebp, ebp
    xor eax, eax
    mov edi, __bss_start
    mov ecx, __bss_end
    sub ecx, edi
    rep stosb
    push ebx
    call kernel_entry
    add esp, 4
.halt:
    cli
    hlt
    jmp near .halt

section .text
isr_stubs_begin:

%macro ISR_NOERR 1
isr_stub_%1:
    push strict dword 0
    push strict dword %1
    jmp isr_common
%endmacro

%macro ISR_ERR 1
isr_stub_%1:
    push strict dword %1
    jmp isr_common
%endmacro

%assign i 0
%rep 256
%if i = 8 || i = 10 || i = 11 || i = 12 || i = 13 || i = 14 || i = 17 || i = 21 || i = 29 || i = 30
    ISR_ERR i
%else
    ISR_NOERR i
%endif
%assign i i + 1
%endrep

isr_stubs_end:

; Stub address table for IDT setup
isr_stub_table:
%assign i 0
%rep 256
    dd isr_stub_%+i
%assign i i+1
%endrep

; Common interrupt handler
; Stack layout after all pushes:
;   [esp+0]  = gs
;   [esp+4]  = fs
;   [esp+8]  = es
;   [esp+12] = ds
;   [esp+16] = edi
;   [esp+20] = esi
;   [esp+24] = ebp
;   [esp+28] = esp_dummy
;   [esp+32] = ebx
;   [esp+36] = edx
;   [esp+40] = ecx
;   [esp+44] = eax
;   [esp+48] = interrupt_number
;   [esp+52] = error_code

isr_common:
    pushad
    push ds
    push es
    push fs
    push gs

    mov ax, 0x10      ; kernel data segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; Read interrupt number and error code
    mov eax, [esp + 48]
    mov edx, [esp + 52]

    ; Call Rust interrupt handler
    ; Args: interrupt_num (eax), error_code (edx), regs_ptr (esp)
    mov eax, [esp + 48]
    mov edx, [esp + 52]
    push esp          ; 3rd arg: pointer to saved registers
    push edx          ; 2nd arg: error_code
    push eax          ; 1st arg: interrupt_num
    call isr_handler_rust
    add esp, 12

    ; Switch after handling the interrupt so faults and exits can mark a task dead.
    mov eax, [esp + 48]
    cmp eax, 32
    je .schedule
    cmp eax, 128
    je .schedule
    cmp eax, 32
    jge .no_schedule
    test dword [esp + 60], 3
    jz .no_schedule
.schedule:
    push esp
    call schedule_rust
    add esp, 4
    mov ecx, [SCHEDULED_CR3]
    mov cr3, ecx
    mov esp, eax
.no_schedule:
    pop gs
    pop fs
    pop es
    pop ds
    popad
    add esp, 8        ; remove interrupt number + error code
    iret

; Load IDT
idt_load:
    mov eax, [esp + 4]
    lidt [eax]
    ret

load_gdt:
    mov eax, [esp + 4]
    lgdt [eax]
    jmp 0x08:.reload_segments
.reload_segments:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    ret

load_page_dir:
    mov eax, [esp + 4]
    mov cr3, eax
    ret

load_tr:
    mov ax, [esp + 4]
    ltr ax
    ret

; Enter one ring-3 ELF process. The return path is the int 0x80 exit syscall.
; Arguments: entry point, user stack, user CR3, user code selector, user data selector.
