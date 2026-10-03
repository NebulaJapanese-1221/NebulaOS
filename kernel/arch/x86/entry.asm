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
    dd 0
    dd 0
    dd 0

section .text
    global _start
    extern kmain

_start:
    cli
    mov esp, 0x90000
    push ebx
    push eax
    call kmain
.halt:
    cli
    hlt
    jmp .halt

section .note.GNU-stack noalloc noexec nowrite progbits
