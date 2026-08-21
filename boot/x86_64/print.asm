; NebulaOS Bootloader - Print Functions (x86_64)
; ================================================
; 
; Real-mode printing functions for 64-bit bootloader
; Same as x86 version for now

bits 16

; -----------------------------------------------------------------------------
; print_string - Print null-terminated string at DS:SI
; -----------------------------------------------------------------------------
print_string:
    pusha
    mov ah, 0x0E  ; BIOS teletype function
    .loop:
        lodsb       ; Load byte from DS:SI into AL
        test al, al ; Check for null terminator
        jz .done
        int 0x10    ; Print character
        jmp .loop
    .done:
        popa
        ret

; -----------------------------------------------------------------------------
; print_char - Print single character in AL
; -----------------------------------------------------------------------------
print_char:
    pusha
    mov ah, 0x0E
    int 0x10
    popa
    ret
