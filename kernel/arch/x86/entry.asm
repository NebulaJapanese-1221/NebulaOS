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

KERNEL_VIRTUAL_BASE equ 0xC0000000

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

    ; GRUB passes the magic in eax and the boot information pointer in ebx.
    ; Both are parked in registers the trampoline never writes, because a far
    ; jump into the higher half drops anything left on the boot loader's stack.
    mov esi, eax                      ; magic
    ; ebx already holds the boot information pointer and is left untouched.

    ; Everything up to the switch into the higher half runs on physical
    ; addresses, so the trampoline tolerates whatever flat descriptor table the
    ; boot loader left installed and touches the boot page directory through
    ; its physical address. NebulaOS installs its own GDT once the higher half
    ; is reachable.
    ;
    ; The mapping is built with 4 MiB pages, which keeps the whole boot mapping
    ; inside a single page of tables. The low 64 MiB stay identity mapped so
    ; the boot structures and the page frame allocator can keep handing out
    ; directly dereferenceable physical addresses, and the kernel is mirrored
    ; at 3 GB, leaving everything below it free for user space.
    mov edi, boot_page_directory - KERNEL_VIRTUAL_BASE
    xor eax, eax
    xor edx, edx
    mov ecx, 1024
.clear_directory:
    mov [edi], edx
    add edi, 4
    dec ecx
    jnz .clear_directory

    mov edi, boot_page_directory - KERNEL_VIRTUAL_BASE
    mov ecx, 16
    mov eax, 0x83
.map_identity:
    mov [edi], eax
    add edi, 4
    dec ecx
    jnz .map_identity

    ; Directory entry 768 covers 0xC0000000, which is where the image links.
    mov dword [boot_page_directory - KERNEL_VIRTUAL_BASE + 768 * 4], 0x83

    ; CR3 holds a physical address and the directory lives inside the image.
    mov eax, boot_page_directory - KERNEL_VIRTUAL_BASE
    mov cr3, eax

    ; The boot mapping above is built from 4 MiB pages, which the CPU only
    ; honours once CR4.PSE is set; with it clear the page size bit counts as a
    ; reserved bit and the very next instruction fetch faults. PAE is cleared
    ; because 32-bit paging requires the two level directory these entries are
    ; written for. The remaining control bits the boot loader may have set are
    ; left alone.
    mov eax, cr4
    and eax, ~(1 << 5)
    or eax, 1 << 4
    mov cr4, eax

    ; Enable paging and write protection together, so the kernel traps a write
    ; to a read-only page instead of quietly dropping it.
    mov eax, cr0
    or eax, 0x80010000
    mov cr0, eax

    ; Only addressable from here on. Reload the descriptor table with the
    ; higher half base and reload the segment registers that depend on it.
    lgdt [gdt_descriptor_virtual]
    jmp 0x08:.reload_segments

.reload_segments:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; The boot stack lives in the image and is only mapped from the higher half.
    mov esp, stack_top
    xor ebp, ebp

    ; The boot information structures sit in low memory, which the identity
    ; mapping still covers, so the physical pointer is passed on unchanged.
    push ebx
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

; Kept for the trampoline, which runs before paging makes the higher half
; addressable and therefore cannot use a higher half table base.
gdt_descriptor_physical:
    dw gdt_end - gdt - 1
    dd gdt - KERNEL_VIRTUAL_BASE

gdt_descriptor_virtual:
    dw gdt_end - gdt - 1
    dd gdt

section .bss
align 4096
boot_page_directory:
    resb 4096

align 16
stack_bottom:
    resb 65536
stack_top:

section .note.GNU-stack noalloc noexec nowrite progbits
