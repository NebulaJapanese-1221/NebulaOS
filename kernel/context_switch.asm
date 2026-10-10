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

; ThreadContext layout (offset from the start of the context):
;   +0  edi, +4  esi, +8  ebp, +12 esp
;   +16 ebx, +20 edx, +24 ecx, +28 eax
;   +32 ds,   +36 es,   +40 fs,   +44 gs
;   +48 cr3,  +52 kernel_esp, +56 user_esp
;   +60 eip,  +64 eflags
;
; ThreadContext begins at offset 16 within Thread, so every
; offset below is the field offset plus 16.
; FPU state is at offset 80 in Thread (after context + fpu_state array)

%define CONTEXT_OFFSET 16
%define FPU_STATE_OFFSET 80
%define FPU_USED_OFFSET 592  ; FPU_STATE_OFFSET + 512

%define CT_EDI        (CONTEXT_OFFSET + 0)
%define CT_ESI        (CONTEXT_OFFSET + 4)
%define CT_EBP        (CONTEXT_OFFSET + 8)
%define CT_ESP        (CONTEXT_OFFSET + 12)
%define CT_EBX        (CONTEXT_OFFSET + 16)
%define CT_EDX        (CONTEXT_OFFSET + 20)
%define CT_ECX        (CONTEXT_OFFSET + 24)
%define CT_EAX        (CONTEXT_OFFSET + 28)
%define CT_DS         (CONTEXT_OFFSET + 32)
%define CT_ES         (CONTEXT_OFFSET + 36)
%define CT_FS         (CONTEXT_OFFSET + 40)
%define CT_GS         (CONTEXT_OFFSET + 44)
%define CT_CR3        (CONTEXT_OFFSET + 48)
%define CT_KERNEL_ESP (CONTEXT_OFFSET + 52)
%define CT_USER_ESP   (CONTEXT_OFFSET + 56)
%define CT_EIP        (CONTEXT_OFFSET + 60)
%define CT_EFLAGS     (CONTEXT_OFFSET + 64)

bits 32

section .text

; void context_switch(Thread* old_thread, Thread* new_thread)
;
; The C calling convention has already pushed the arguments on the
; stack. Callee-saved registers are restored on return, so only the
; registers that the ThreadContext tracks need to be moved here.
global context_switch
context_switch:
    push ebx
    push esi
    push edi
    push ebp

    ; Arguments, accounting for the four pushes above.
    mov esi, [esp + 16 + 4]     ; old_thread
    mov edi, [esp + 16 + 8]     ; new_thread

    ; Save the live callee-saved registers into the old context.
    test esi, esi
    jz .skip_save
    mov [esi + CT_EBX], ebx
    mov [esi + CT_ESI], esi
    mov [esi + CT_EDI], edi
    mov [esi + CT_EBP], ebp
    ; The stack pointer of this frame is the old thread's kernel
    ; stack pointer at the moment of the switch.
    lea eax, [esp + 16]
    mov [esi + CT_KERNEL_ESP], eax
    
    ; Save FPU state if the old thread used the FPU
    cmp byte [esi + FPU_USED_OFFSET], 0
    je .skip_fpu_save
    fxsave [esi + FPU_STATE_OFFSET]
.skip_fpu_save:
.skip_save:

    test edi, edi
    jz .done

    ; Restore the segment selectors first so the data accesses
    ; below stay in the kernel data segment.
    mov ax, [edi + CT_DS]
    test ax, ax
    jz .skip_ds
    mov ds, ax
.skip_ds:
    mov ax, [edi + CT_ES]
    test ax, ax
    jz .skip_es
    mov es, ax
.skip_es:
    mov ax, [edi + CT_FS]
    test ax, ax
    jz .skip_fs
    mov fs, ax
.skip_fs:
    mov ax, [edi + CT_GS]
    test ax, ax
    jz .skip_gs
    mov gs, ax
.skip_gs:

    ; Switch the address space. A zero cr3 would fault, so a
    ; thread that has no address space of its own keeps the
    ; current one.
    mov eax, [edi + CT_CR3]
    test eax, eax
    jz .skip_cr3
    mov cr3, eax
.skip_cr3:

    ; Restore the callee-saved registers from the new context.
    mov ebx, [edi + CT_EBX]
    mov esi, [edi + CT_ESI]
    mov edi, [edi + CT_EDI]
    mov ebp, [edi + CT_EBP]

    ; Restore FPU state if the new thread used the FPU
    cmp byte [edi + FPU_USED_OFFSET], 0
    je .skip_fpu_restore
    fxrstor [edi + FPU_STATE_OFFSET]
.skip_fpu_restore:

    ; The new thread resumes on its own kernel stack. The saved
    ; kernel_esp points at the frame that context_switch itself
    ; pushed for the previous switch, so restoring it here and
    ; returning through the saved frame is what makes the switch
    ; transparent to the caller.
    mov eax, [edi + CT_KERNEL_ESP]
    test eax, eax
    jz .done

    ; Drop the saved registers and return through the restored
    ; stack. The pops below unwind the four pushes from the top of
    ; this function, so the return address is the one the new
    ; thread was entered with.
    mov esp, eax
    pop ebp
    pop edi
    pop esi
    pop ebx
    ret

.done:
    pop ebp
    pop edi
    pop esi
    pop ebx
    ret

section .note.GNU-stack noalloc noexec nowrite progbits