; Interrupt and exception stubs for the NebulaOS x86 operating system.
; Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
;
; This program is free software: you can redistribute it and/or modify
; it under the terms of the GNU General Public License as published by
; the Free Software Foundation, either version 3 of the License, or
; (at your option) any later version.
;
; This program is distributed in the hope that it will be useful,
; but WITHOUT ANY WARRANTY; without even the implied warranty of
; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
; GNU General Public License for more details.
;
; You should have received a copy of the GNU General Public License
; along with this program. If not, see <https://www.gnu.org/licenses/>.
; See LICENCE for the full license text.

bits 32

section .text
extern interrupt_dispatch
global isr_stub_table
global isr_spurious

%macro ISR_NO_ERROR 1
isr_%1:
    push dword 0
    push dword %1
    jmp isr_common
%endmacro

%macro ISR_ERROR 1
isr_%1:
    push dword %1
    jmp isr_common
%endmacro

ISR_NO_ERROR 0
ISR_NO_ERROR 1
ISR_NO_ERROR 2
ISR_NO_ERROR 3
ISR_NO_ERROR 4
ISR_NO_ERROR 5
ISR_NO_ERROR 6
ISR_NO_ERROR 7
ISR_ERROR 8
ISR_NO_ERROR 9
ISR_ERROR 10
ISR_ERROR 11
ISR_ERROR 12
ISR_ERROR 13
ISR_ERROR 14
ISR_NO_ERROR 15
ISR_NO_ERROR 16
ISR_ERROR 17
ISR_NO_ERROR 18
ISR_NO_ERROR 19
ISR_NO_ERROR 20
ISR_ERROR 21
ISR_NO_ERROR 22
ISR_NO_ERROR 23
ISR_NO_ERROR 24
ISR_NO_ERROR 25
ISR_NO_ERROR 26
ISR_NO_ERROR 27
ISR_NO_ERROR 28
ISR_ERROR 29
ISR_ERROR 30
ISR_NO_ERROR 31

%assign irq 32
%rep 16
ISR_NO_ERROR irq
%assign irq irq + 1
%endrep

isr_spurious:
    push dword 0
    push dword 255
    jmp isr_common

isr_common:
    pusha
    mov eax, [esp + 32]      ; vector
    mov edx, [esp + 36]      ; error_code
    push ds
    push es
    push fs
    push gs
    mov bx, 0x10
    mov ds, bx
    mov es, bx
    mov fs, bx
    mov gs, bx
    
    ; Save all registers for error screen
    push dword [esp + 32 + 8 + 4]  ; eip (pushed by CPU)
    push dword [esp + 32 + 8 + 8]  ; cs
    push dword [esp + 32 + 8 + 12] ; eflags
    push dword [esp + 32 + 8 + 16] ; esp (user stack pointer if privilege change)
    push dword [esp + 32 + 8 + 20] ; ss
    
    push edx                     ; error_code
    push eax                     ; vector
    call interrupt_dispatch
    add esp, 28                  ; clean up 7 pushed args (vector, error_code, eip, cs, eflags, esp, ss)
    pop gs
    pop fs
    pop es
    pop ds
    popa
    add esp, 8
    iretd

section .rodata
align 4
isr_stub_table:
%assign vector 0
%rep 48
    dd isr_%+vector
%assign vector vector + 1
%endrep

section .note.GNU-stack noalloc noexec nowrite progbits
