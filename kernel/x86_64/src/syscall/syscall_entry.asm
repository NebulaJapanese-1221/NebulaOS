; NebulaOS - x86_64 Syscall Entry
; =================================
;
; 64-bit syscall interrupt gate handler (INT 0x80)
; User convention: rax=num, rbx=arg1, rcx=arg2, rdx=arg3, rsi=arg4, rdi=arg5

bits 64
global syscall_entry
syscall_entry:
    push rbp
    mov rbp, rsp
    push rbx
    push r12
    push r13
    push r14
    push r15
    
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax
    call syscall_dispatch
    add rsp, 48
    
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    
    iretq