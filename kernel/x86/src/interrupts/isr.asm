; NebulaOS - x86 Interrupt Service Routines
; ============================================
; 
; Assembly stubs for ISRs and IRQs
; These save the CPU state and call the C handler

bits 32

; -----------------------------------------------------------------------------
; Common ISR macro
; Saves all registers and calls the C handler
; -----------------------------------------------------------------------------
%macro ISR_NOERRCODE 1
isr%1:
    cli
    push 0        ; Dummy error code
    push %1      ; Interrupt number
    jmp isr_common
%endmacro

%macro ISR_ERRCODE 1
isr%1:
    cli
    push %1      ; Interrupt number (error code already pushed by CPU)
    jmp isr_common
%endmacro

; -----------------------------------------------------------------------------
; Common IRQ macro
; -----------------------------------------------------------------------------
%macro IRQ 2
irq%1:
    cli
    push 0        ; Dummy error code
    push %2      ; Interrupt number
    jmp irq_common
%endmacro

; -----------------------------------------------------------------------------
; ISR stubs (exceptions)
; -----------------------------------------------------------------------------
ISR_NOERRCODE 0   ; Divide by zero
ISR_NOERRCODE 1   ; Debug
ISR_NOERRCODE 2   ; Non-maskable interrupt
ISR_NOERRCODE 3   ; Breakpoint
ISR_NOERRCODE 4   ; Overflow
ISR_NOERRCODE 5   ; Bound range
ISR_NOERRCODE 6   ; Invalid opcode
ISR_NOERRCODE 7   ; Device not available
ISR_ERRCODE   8   ; Double fault
ISR_NOERRCODE 9   ; Coprocessor segment overrun
ISR_ERRCODE   10  ; Invalid TSS
ISR_ERRCODE   11  ; Segment not present
ISR_ERRCODE   12  ; Stack segment fault
ISR_ERRCODE   13  ; General protection fault
ISR_ERRCODE   14  ; Page fault
ISR_NOERRCODE 15  ; Reserved
ISR_NOERRCODE 16  ; x87 Floating-point exception
ISR_ERRCODE   17  ; Alignment check
ISR_NOERRCODE 18  ; Machine check
ISR_NOERRCODE 19  ; SIMD floating-point exception
ISR_NOERRCODE 20  ; Virtualization exception
ISR_NOERRCODE 21  ; Reserved
ISR_NOERRCODE 22  ; Reserved
ISR_NOERRCODE 23  ; Reserved
ISR_NOERRCODE 24  ; Reserved
ISR_NOERRCODE 25  ; Reserved
ISR_NOERRCODE 26  ; Reserved
ISR_NOERRCODE 27  ; Reserved
ISR_NOERRCODE 28  ; Reserved
ISR_NOERRCODE 29  ; Reserved
ISR_NOERRCODE 30  ; Reserved
ISR_NOERRCODE 31  ; Reserved

; -----------------------------------------------------------------------------
; IRQ stubs
; -----------------------------------------------------------------------------
IRQ 0, 32   ; PIT Timer
IRQ 1, 33   ; Keyboard
IRQ 2, 34   ; Cascade
IRQ 3, 35   ; Serial port 2
IRQ 4, 36   ; Serial port 1
IRQ 5, 37   ; Parallel port 2
IRQ 6, 38   ; Floppy disk
IRQ 7, 39   ; Parallel port 1
IRQ 8, 40   ; Real-time clock
IRQ 9, 41   ; ACPI
IRQ 10, 42  ; Reserved
IRQ 11, 43  ; Reserved
IRQ 12, 44  ; PS/2 Mouse
IRQ 13, 45  ; Coprocessor
IRQ 14, 46  ; Primary ATA
IRQ 15, 47  ; Secondary ATA

; -----------------------------------------------------------------------------
; Common ISR handler
; Saves CPU state and calls C handler
; -----------------------------------------------------------------------------
isr_common:
    ; Save all general purpose registers
    pusha
    
    ; Save segment registers
    push ds
    push es
    push fs
    push gs
    
    ; Load kernel data segment
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; Call C handler (defined in idt.c)
    push esp
    extern interrupt_handler
    call interrupt_handler
    add esp, 4
    
    ; Restore segment registers
    pop gs
    pop fs
    pop es
    pop ds
    
    ; Restore general purpose registers
    popa
    
    ; Clean up error code and interrupt number
    add esp, 8
    
    ; Return from interrupt
    iret

; -----------------------------------------------------------------------------
; Common IRQ handler
; Sends EOI to PIC and calls C handler
; -----------------------------------------------------------------------------
irq_common:
    ; Save all general purpose registers
    pusha
    
    ; Save segment registers
    push ds
    push es
    push fs
    push gs
    
    ; Load kernel data segment
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; Call C handler (defined in idt.c)
    push esp
    extern interrupt_handler
    call interrupt_handler
    add esp, 4
    
    ; Send EOI to PIC
    mov al, 0x20
    out 0x20, al
    
    ; Check if it's a slave IRQ (8-15)
    cmp [esp + 48], byte 40  ; Compare interrupt number with 40 (IRQ8)
    jb .no_slave_eoi
    out 0xA0, al           ; Send EOI to slave PIC
.no_slave_eoi:
    
    ; Restore segment registers
    pop gs
    pop fs
    pop es
    pop ds
    
    ; Restore general purpose registers
    popa
    
    ; Clean up error code and interrupt number
    add esp, 8
    
    ; Return from interrupt
    iret

; -----------------------------------------------------------------------------
; Data section
; -----------------------------------------------------------------------------
section .data

; -----------------------------------------------------------------------------
; Global definitions for C code
; -----------------------------------------------------------------------------
global isr0
global isr1
global isr2
global isr3
global isr4
global isr5
global isr6
global isr7
global isr8
global isr9
global isr10
global isr11
global isr12
global isr13
global isr14
global isr15
global isr16
global isr17
global isr18
global isr19
global isr20
global isr21
global isr22
global isr23
global isr24
global isr25
global isr26
global isr27
global isr28
global isr29
global isr30
global isr31

global irq0
global irq1
global irq2
global irq3
global irq4
global irq5
global irq6
global irq7
global irq8
global irq9
global irq10
global irq11
global irq12
global irq13
global irq14
global irq15
