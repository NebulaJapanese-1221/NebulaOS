; NebulaOS - x86 Syscall Entry
; ==============================
;
; 32-bit syscall interrupt gate handler (INT 0x80)
; Arguments: eax=num, ebx=arg1, ecx=arg2, edx=arg3

bits 32
extern syscall_dispatch
global syscall_entry
syscall_entry:
    pusha
    push ds
    push es
    push fs
    push gs
    
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    push edx
    push ecx
    push ebx
    push eax
    call syscall_dispatch
    add esp, 16
    
    pop gs
    pop fs
    pop es
    pop ds
    popa
    iret