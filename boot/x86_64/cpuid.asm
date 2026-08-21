; NebulaOS Bootloader - CPUID Functions
; ======================================
; 
; CPUID detection for 64-bit support

bits 16

; -----------------------------------------------------------------------------
; check_cpuid - Check if CPUID instruction is supported
; Output:
;   EAX = 1 if supported, 0 if not
; -----------------------------------------------------------------------------
check_cpuid:
    pushfd
    pop eax
    
    ; Save original flags
    mov ecx, eax
    
    ; Flip bit 21 (ID bit)
    xor eax, 0x200000
    
    ; Push modified flags and pop back
    push eax
    popfd
    
    ; Get new flags
    pushfd
    pop eax
    
    ; Restore original flags
    push ecx
    popfd
    
    ; Compare
    xor eax, ecx
    and eax, 0x200000
    jz .no_cpuid
    
    mov eax, 1
    ret
    
.no_cpuid:
    xor eax, eax
    ret

; -----------------------------------------------------------------------------
; check_long_mode - Check if 64-bit long mode is supported
; Requires: CPUID is supported
; Output:
;   EAX = 1 if supported, 0 if not
; -----------------------------------------------------------------------------
check_long_mode:
    ; Check extended function support (CPUID function 0x80000000)
    mov eax, 0x80000000
    cpuid
    
    ; Check if function 0x80000001 is available
    cmp eax, 0x80000001
    jb .no_long_mode
    
    ; Get extended feature flags (CPUID function 0x80000001)
    mov eax, 0x80000001
    cpuid
    
    ; Check LM bit (bit 29 of EDX)
    test edx, 0x20000000
    jz .no_long_mode
    
    mov eax, 1
    ret
    
.no_long_mode:
    xor eax, eax
    ret
