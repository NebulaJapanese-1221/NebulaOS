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
    mov esp, 0x90000
    push ebx
    push eax
    call kmain
.halt:
    cli
    hlt
    jmp .halt

section .note.GNU-stack noalloc noexec nowrite progbits
