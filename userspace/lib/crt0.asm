; Userspace C runtime entry point for NebulaOS.
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
    global _start
    extern _start_c

; The kernel pushes the following onto the user stack before iret:
;   [esp]     = return address (ignored, we re-enter via _start_c)
; Actually, the kernel sets up the user stack and passes argc/argv via
; the stack. When the kernel switches to user mode, the stack pointer
; points to:
;   [esp+0]  = argc
;   [esp+4]  = argv[0]
;   [esp+8]  = argv[1]
;   ...
;   [esp+4+4*argc] = NULL (argv terminator)
;
; This entry point is reached via a call from the kernel, so the stack
; is already set up with argc/argv.
_start:
    ; The kernel calls _start directly with the user stack containing
    ; argc at [esp] and argv at [esp+4]. Pop argc into esi and pass
    ; argv (already at esp+4) as the second argument.
    pop esi              ; esi = argc
    mov edx, esp         ; edx = argv (pointer to argv[0])

    ; Align the stack to 16 bytes before calling the C runtime.
    ; Standard calling convention requires 16-byte alignment at function entry.
    and esp, 0xFFFFFFF0

    ; Push argv and argc in the correct order for _start_c(argc, argv).
    push edx             ; argv
    push esi             ; argc
    call _start_c

    ; _start_c never returns; if it does, halt.
.halt:
    cli
    hlt
    jmp .halt