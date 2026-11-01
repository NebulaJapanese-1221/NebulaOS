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

section .multiboot2
align 8
multiboot2_header:
    dd 0xE85250D6
    dd 0
    dd multiboot2_header_end - multiboot2_header
    dd -(0xE85250D6 + (multiboot2_header_end - multiboot2_header))
    dd 0
    dd 8
multiboot2_header_end:

section .text.boot
_start:
    cli
    mov esp, 0x90000
    xor ebp, ebp
    xor eax, eax
    mov edi, __bss_start
    mov ecx, __bss_end
    sub ecx, edi
    rep stosb
    call kernel_entry
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

    ; Check if this is the timer IRQ (interrupt 32)
    cmp eax, 32
    jne .no_schedule

    ; Timer tick - run scheduler
    push esp          ; argument: current saved stack pointer
    call schedule_rust
    add esp, 4        ; clean up argument
    mov esp, eax      ; switch to next process stack

.no_schedule:
    mov eax, [esp + 48]
    mov edx, [esp + 52]

    ; Call Rust interrupt handler
    ; Args: interrupt_num (eax), error_code (edx), regs_ptr (esp)
    push esp          ; 3rd arg: pointer to saved registers
    push edx          ; 2nd arg: error_code
    push eax          ; 1st arg: interrupt_num
    call isr_handler_rust
    add esp, 12

    ; Send EOI for PIC hardware interrupts (32-47)
    cmp dword [esp + 48], 32
    jl near .no_eoi
    cmp dword [esp + 48], 48
    jge near .no_eoi
    cmp dword [esp + 48], 40
    jl near .no_slave
    mov al, 0x20
    out 0xA0, al
.no_slave:
    mov al, 0x20
    out 0x20, al
.no_eoi:

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
    ; Just return eax unchanged for now (single process)
    ret
