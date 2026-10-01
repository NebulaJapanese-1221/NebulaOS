bits 32

section .text
    global _start
    extern kmain

_start:
    cli
    mov esp, 0x90000
    call kmain
.halt:
    cli
    hlt
    jmp .halt
