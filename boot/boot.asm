[bits 16]
org 0x7C00

boot_start:
    cli
    cld
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    mov [boot_drive], dl

    mov dx, 0x3F9
    xor al, al
    out dx, al
    mov dx, 0x3FB
    mov al, 0x80
    out dx, al
    mov dx, 0x3F8
    mov al, 0x01
    out dx, al
    mov dx, 0x3F9
    xor al, al
    out dx, al
    mov dx, 0x3FB
    mov al, 0x03
    out dx, al

    mov si, msg_boot
    call print_string

    mov word [remaining], 512
    mov word [load_segment], 0x1000
    mov byte [cylinder], 0
    mov byte [head], 0
    mov byte [sector], 2

.read_sector:
    mov ax, [load_segment]
    mov es, ax
    xor bx, bx
    mov ah, 0x02
    mov al, 1
    mov ch, [cylinder]
    mov cl, [sector]
    mov dh, [head]
    mov dl, [boot_drive]
    int 0x13
    jc disk_error

    add word [load_segment], 0x20
    inc byte [sector]
    cmp byte [sector], 19
    jb .next_sector
    mov byte [sector], 1
    inc byte [head]
    cmp byte [head], 2
    jb .next_sector
    mov byte [head], 0
    inc byte [cylinder]
.next_sector:
    dec word [remaining]
    jnz .read_sector

    mov si, msg_loaded
    call print_string

    cli
    lgdt [gdt32_ptr]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:.protected_mode

[bits 32]
.protected_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000
    mov eax, 0x10000
    jmp eax

[bits 16]
print_string:
    lodsb
    test al, al
    jz .done
    mov bl, al
    mov dx, 0x3FD
.wait_tx:
    in al, dx
    test al, 0x20
    jz .wait_tx
    mov dx, 0x3F8
    mov al, bl
    out dx, al
    jmp print_string
.done:
    ret

disk_error:
    mov si, msg_error
    call print_string
    cli
.halt:
    hlt
    jmp .halt

boot_drive: db 0
cylinder: db 0
head: db 0
sector: db 0
remaining: dw 0
load_segment: dw 0
msg_boot: db "NebulaOS booting...", 0x0D, 0x0A, 0
msg_loaded: db "Kernel loaded.", 0x0D, 0x0A, 0
msg_error: db "Disk read error.", 0x0D, 0x0A, 0

align 8
gdt32_start:
    dq 0
    dw 0xFFFF, 0
    db 0, 10011010b, 11001111b, 0
    dw 0xFFFF, 0
    db 0, 10010010b, 11001111b, 0
gdt32_end:
gdt32_ptr:
    dw gdt32_end - gdt32_start - 1
    dd gdt32_start

times 510 - ($ - $$) db 0
dw 0xAA55
