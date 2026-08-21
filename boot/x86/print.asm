; NebulaOS Bootloader - Print Functions
; ======================================
; 
; Real-mode printing functions for bootloader

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

; -----------------------------------------------------------------------------
; print_hex_byte - Print byte in AL as hex
; -----------------------------------------------------------------------------
print_hex_byte:
    pusha
    mov ah, 0x0E
    
    ; Print high nibble
    mov bl, al
    shr bl, 4
    call .print_nibble
    
    ; Print low nibble
    mov bl, al
    and bl, 0x0F
    call .print_nibble
    
    popa
    ret

.print_nibble:
    cmp bl, 9
    jbe .digit
    add bl, 7   ; 'A' - '9' - 1 = 7
.digit:
    add bl, '0'
    mov al, bl
    int 0x10
    ret

; -----------------------------------------------------------------------------
; print_hex_word - Print word in AX as hex
; -----------------------------------------------------------------------------
print_hex_word:
    pusha
    xchg al, ah  ; Swap bytes
    call print_hex_byte
    xchg al, ah
    call print_hex_byte
    popa
    ret
