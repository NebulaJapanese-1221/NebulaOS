[bits 32]

section .text
global _start

_start:
    mov eax, 0
    mov ebx, message
    mov ecx, message_end - message
    int 0x80

    mov eax, 3
    int 0x80
    mov esi, eax

.wait_for_preemption:
    mov eax, 3
    int 0x80
    sub eax, esi
    cmp eax, 300
    jb .wait_for_preemption

    mov eax, 2
    xor ebx, ebx
    int 0x80

.halt:
    jmp .halt

message:
    db "Hello from a ring-3 ELF process.", 10
message_end:
