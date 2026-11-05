; SPDX-License-Identifier: GPL-3.0-only
; Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
;
; This file is part of NebulaOS.
;
; NebulaOS is free software: you can redistribute it and/or modify
; it under the terms of the GNU General Public License as published by
; the Free Software Foundation, either version 3 of the License.
;
; NebulaOS is distributed in the hope that it will be useful,
; but WITHOUT ANY WARRANTY; without even the implied warranty of
; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
; GNU General Public License for more details.
;
; You should have received a copy of the GNU General Public License
; along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

[bits 16]
section .ap_trampoline
global ap_trampoline_start
global ap_trampoline_stack_top
global ap_trampoline_cr3
global ap_trampoline_entry
global ap_trampoline_cpu_index

ap_trampoline_start:
    cli
    cld
    xor ax, ax
    mov ds, ax
    lgdt [ap_trampoline_gdt_pointer]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp dword 0x08:ap_trampoline_protected

[bits 32]
ap_trampoline_protected:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, [ap_trampoline_stack_top]
    mov eax, [ap_trampoline_cr3]
    mov cr3, eax
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax
    mov eax, [ap_trampoline_entry]
    push dword [ap_trampoline_cpu_index]
    call eax

.halt:
    cli
    hlt
    jmp .halt

align 8
ap_trampoline_gdt:
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
ap_trampoline_gdt_end:

ap_trampoline_gdt_pointer:
    dw ap_trampoline_gdt_end - ap_trampoline_gdt - 1
    dd ap_trampoline_gdt

align 4
ap_trampoline_stack_top:
    dd 0
ap_trampoline_cr3:
    dd 0
ap_trampoline_entry:
    dd 0
ap_trampoline_cpu_index:
    dd 0
