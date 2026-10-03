; Multiboot entry point for the NebulaOS x86 operating system.
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

section .multiboot
align 4
    dd 0x1BADB002
    dd 0x00000007
    dd -(0x1BADB002 + 0x00000007)
    dd 0
    dd 800
    dd 600
    dd 32

section .text
    global _start
    extern kmain

_start:
    cli
    mov esi, eax
    mov edi, ebx

    lgdt [gdt_descriptor]
    jmp 0x08:.reload_segments

.reload_segments:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000
    push edi
    push esi
    call kmain
.halt:
    cli
    hlt
    jmp .halt

section .rodata
align 8
gdt:
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
gdt_end:
gdt_descriptor:
    dw gdt_end - gdt - 1
    dd gdt

section .note.GNU-stack noalloc noexec nowrite progbits
