; Context switching for the NebulaOS x86 scheduler.
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

; void context_switch(Thread* old_thread, Thread* new_thread)
; Saves the current context into old_thread->context and
; restores new_thread->context.
;
; Layout of ThreadContext (32-byte aligned):
;   +0  edi, +4  esi, +8  ebp, +12 esp (unused, saved via stack)
;   +16 ebx, +20 edx, +24 ecx, +28 eax
;   +32 ds, +36 es, +40 fs, +44 gs
;   +48 cr3 (only for user threads)
;   +52 kernel_esp
;   +56 user_esp
;   +60 eip
;   +64 eflags

global context_switch
extern scheduler_current

context_switch:
    ; Save callee-saved registers
    pushad
    push ds
    push es
    push fs
    push gs

    ; Get old_thread pointer from stack (first argument)
    ; At this point, stack layout is:
    ;   [esp+0]  = return address
    ;   [esp+4]  = old_thread
    ;   [esp+8]  = new_thread
    mov eax, [esp + 4 + 16]   ; old_thread (pushad = 16 bytes)

    test eax, eax
    jz .no_old

    ; Save the current stack pointer into old_thread
    mov ebx, eax
    ; Compute the address of the saved register block
    ; The register block is at the top of the kernel stack
    ; We save the current esp into the context
    mov ecx, [ebx + 52]        ; kernel_esp field offset
    ; We need to save the current esp to the old thread's context
    ; The context is at old_thread + offsetof(Thread, context)
    ; offsetof(Thread, context) is after the other fields
    ; For simplicity, we assume context is at a fixed offset
    ; We'll use a simpler approach: store esp directly

    ; Save registers to old_thread->context
    ; First, get pointer to context
    ; Thread structure layout:
    ;   id (4), state (1), policy (1), priority (1), time_slice (4)
    ;   total_time (4) = 12 bytes of padding
    ;   context (ThreadContext) starts at offset 20
    ;   But we need to know the exact offset
    ;
    ; Let's just save the register block pointer
    mov [ebx + 12], esp        ; Save current esp as kernel_esp

.no_old:
    ; Load new_thread
    mov eax, [esp + 4 + 20]   ; new_thread (pushad = 16 bytes, +4 for old_thread)

    ; Restore registers from new_thread->context
    mov ebx, eax

    ; Restore segment registers
    mov ax, [ebx + 32]        ; ds
    mov ds, ax
    mov ax, [ebx + 36]        ; es
    mov es, ax
    mov ax, [ebx + 40]        ; fs
    mov fs, ax
    mov ax, [ebx + 44]        ; gs
    mov gs, ax

    ; Restore CR3 (page directory)
    mov eax, [ebx + 48]       ; cr3
    mov cr3, eax

    ; Restore general purpose registers
    ; We need to restore in reverse order
    ; First, restore the stack pointer
    mov esp, [ebx + 12]       ; kernel_esp

    ; Restore the register block from the stack
    pop gs
    pop fs
    pop es
    pop ds
    popad

    ret

; Alternative: save to a known location in the thread structure
global save_context
save_context:
    pushad
    push ds
    push es
    push fs
    push gs

    ; Get thread pointer
    mov eax, [esp + 4 + 16]   ; thread pointer

    ; Save registers
    mov ebx, eax
    mov [ebx + 0], edi        ; context.edi
    mov [ebx + 4], esi        ; context.esi
    mov [ebx + 8], ebp        ; context.ebp
    ; esp is saved separately
    mov [ebx + 16], ebx       ; context.ebx (temporary)
    mov [ebx + 20], edx       ; context.edx
    mov [ebx + 24], ecx       ; context.ecx
    mov [ebx + 28], eax       ; context.eax

    ; Save segment registers
    mov ax, ds
    mov [ebx + 32], ax        ; context.ds
    mov ax, es
    mov [ebx + 36], ax        ; context.es
    mov ax, fs
    mov [ebx + 40], ax        ; context.fs
    mov ax, gs
    mov [ebx + 44], ax        ; context.gs

    ; Save stack pointer
    mov [ebx + 12], esp       ; kernel_esp

    pop gs
    pop fs
    pop es
    pop ds
    popad
    ret

global restore_context
restore_context:
    ; Get thread pointer
    mov ebx, [esp + 4]

    ; Restore segment registers
    mov ax, [ebx + 32]
    mov ds, ax
    mov ax, [ebx + 36]
    mov es, ax
    mov ax, [ebx + 40]
    mov fs, ax
    mov ax, [ebx + 44]
    mov gs, ax

    ; Restore CR3
    mov eax, [ebx + 48]
    mov cr3, eax

    ; Restore stack pointer
    mov esp, [ebx + 12]

    ; Restore general purpose registers
    mov edi, [ebx + 0]
    mov esi, [ebx + 4]
    mov ebp, [ebx + 8]
    ; ebx, edx, ecx, eax are restored by popad
    ; But we need to set them first

    ; Push the register values for popad
    push dword [ebx + 28]     ; eax
    push dword [ebx + 24]     ; ecx
    push dword [ebx + 20]     ; edx
    push dword [ebx + 16]     ; ebx
    push dword [ebx + 8]      ; ebp (original esp slot, unused)
    push dword [ebx + 8]      ; ebp
    push dword [ebx + 4]      ; esi
    push dword [ebx + 0]      ; edi

    popad

    ret

section .note.GNU-stack noalloc noexec nowrite progbits