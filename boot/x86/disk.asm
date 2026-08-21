; NebulaOS Bootloader - Disk Functions
; ====================================
; 
; Real-mode disk I/O functions using BIOS interrupt 0x13

bits 16

; -----------------------------------------------------------------------------
; disk_load - Load sectors from disk using CHS addressing
; Input:
;   ES:BX - Destination buffer
;   AL    - Number of sectors to read
;   DL    - Drive number (should be set from boot_drive)
;   CX    - Cylinder and sector (CH = cylinder, CL = sector)
;   DH    - Head number
; Output:
;   CF set on error
;   AX   - Error code if CF set
; -----------------------------------------------------------------------------
disk_load:
    pusha
    mov di, 3   ; Retry count

.retry:
    push dx     ; Save DX (contains head number)
    mov ah, 0x02 ; BIOS read sectors function
    ; AL already has number of sectors
    ; CH = cylinder (low 8 bits)
    ; CL = sector (bits 0-5)
    ; DH = head (from input)
    ; DL = drive (from input)
    int 0x13
    jnc .success
    
    ; Error occurred
    dec di
    jz .error
    pop dx
    jmp .retry

.success:
    pop dx
    popa
    ret

.error:
    pop dx
    popa
    stc
    ret

; -----------------------------------------------------------------------------
; disk_load_extended - Load sectors with error recovery and retry
; Input:
;   ES:BX - Destination buffer
;   AL    - Number of sectors to read
;   DL    - Drive number
;   CX    - Cylinder and sector (CH = cylinder, CL = sector)
;   DH    - Head number
; Output:
;   CF set on error
;   AX   - Error code if CF set
; -----------------------------------------------------------------------------
disk_load_extended:
    pusha
    mov si, 5   ; Retry count (more retries for extended)

.retry_loop:
    push si
    push dx
    push cx
    push bx
    push ax
    
    ; Reset disk controller
    call disk_reset
    
    pop ax
    pop bx
    pop cx
    pop dx
    pop si
    
    ; Try to load
    call disk_load
    jnc .done
    
    ; Retry
    dec si
    jnz .retry_loop
    
    ; All retries failed
    popa
    stc
    ret
    
.done:
    popa
    ret

; -----------------------------------------------------------------------------
; disk_reset - Reset disk system
; Input:
;   DL - Drive number
; -----------------------------------------------------------------------------
disk_reset:
    pusha
    mov ah, 0x00
    int 0x13
    jc .error
    popa
    ret
.error:
    popa
    stc
    ret

; -----------------------------------------------------------------------------
; disk_get_params - Get disk parameters
; Input:
;   DL - Drive number
; Output:
;   CF set on error
;   DL - Number of hard drives
;   DH - Max head number
;   CX - Max cylinder and sector (CH = max cylinder, CL = max sector)
; -----------------------------------------------------------------------------
disk_get_params:
    pusha
    mov ah, 0x08
    int 0x13
    jc .error
    popa
    ret
.error:
    popa
    stc
    ret

; -----------------------------------------------------------------------------
; disk_wait - Wait for disk to be ready
; -----------------------------------------------------------------------------
disk_wait:
    pusha
    mov cx, 0xFFFF
.wait_loop:
    mov ah, 0x00
    int 0x13
    jnc .done
    loop .wait_loop
.done:
    popa
    ret
