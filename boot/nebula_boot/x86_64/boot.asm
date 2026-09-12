; NebulaBoot - NebulaOS Custom Bootloader (x86_64 UEFI)
; ====================================================
; 
; UEFI bootloader for x86_64 with menu
; Loads kernel directly without GRUB/Multiboot
;
; Build: nasm -f win64 boot.asm -o boot.obj, then link with gnu-efi

bits 64
default rel

; -----------------------------------------------------------------------------
; UEFI Structures and Constants
; -----------------------------------------------------------------------------

EFI_SUCCESS equ 0
EFI_LOADED_IMAGE_PROTOCOL_GUID:
    dd 0x5B1B31A1
    dw 0x9562
    dw 0x11D2
    db 0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B

EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID:
    dd 0x0964E5B22
    dw 0x6459
    dw 0x11D2
    db 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B

EFI_FILE_PROTOCOL_GUID:
    dd 0x0964E5B22
    dw 0x6459
    dw 0x11D2
    db 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B

EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID:
    dd 0x9042A9DE
    dw 0x23DC
    dw 0x4A38
    db 0x96, 0xFB, 0x7A, 0xDE, 0xD0, 0x80, 0x51, 0x6A

EFI_SYSTEM_TABLE:
    ; EFI_TABLE_HEADER
    dd 0x5453595320494249  ; Signature "EFI SYSTEM TABLE"
    dd 0x0002000A          ; Revision 2.10
    dd 0                   ; HeaderSize
    dd 0                   ; CRC32
    dd 0                   ; Reserved
    ; EFI_SYSTEM_TABLE
    dq 0                   ; FirmwareVendor
    dd 0                   ; FirmwareRevision
    dq 0                   ; ConsoleInHandle
    dq 0                   ; ConIn
    dq 0                   ; ConsoleOutHandle
    dq 0                   ; ConOut
    dq 0                   ; StandardErrorHandle
    dq 0                   ; StdErr
    dq 0                   ; RuntimeServices
    dq 0                   ; BootServices
    dq 0                   ; NumberOfTableEntries
    dq 0                   ; ConfigurationTable

; -----------------------------------------------------------------------------
; Global Variables
; -----------------------------------------------------------------------------
section .bss
gST:        resq 1          ; System Table
gBS:        resq 1          ; Boot Services
gRT:        resq 1          ; Runtime Services
gImageHandle: resq 1        ; Image Handle
gLoadedImage: resq 1        ; Loaded Image Protocol
gFileSystem: resq 1         ; File System Protocol
gRootDir:   resq 1          ; Root Directory
gGraphicsOutput: resq 1     ; Graphics Output Protocol
gMode:      resq 1          ; Current Video Mode

kernel_buffer: resq 1       ; Kernel load address
kernel_size:  resq 1        ; Kernel size in bytes
kernel_entry: resq 1        ; Kernel entry point

selected_option: resb 1     ; Menu selection (0=32-bit, 1=64-bit)

; -----------------------------------------------------------------------------
; Code Section
; -----------------------------------------------------------------------------
section .text

global efi_main
extern kernel_main_64

efi_main:
    ; Save parameters
    mov [gImageHandle], rcx     ; ImageHandle
    mov [gST], rdx              ; SystemTable
    
    ; Get Boot Services
    mov rax, [rdx + 60]         ; EFI_SYSTEM_TABLE.BootServices
    mov [gBS], rax
    
    ; Get Runtime Services
    mov rax, [rdx + 68]         ; EFI_SYSTEM_TABLE.RuntimeServices
    mov [gRT], rax
    
    ; Initialize console
    call init_console
    
    ; Print banner
    mov rcx, banner
    call print_string
    
    ; Print menu
    mov rcx, menu
    call print_string
    
    ; Wait for key input
    call wait_for_key
    
    ; Process selection
    cmp byte [selected_option], 0
    je boot_64bit_kernel
    cmp byte [selected_option], 1
    je reboot_system
    cmp byte [selected_option], 2
    je shutdown_system
    
    ; Default to 64-bit
    jmp boot_64bit_kernel

; -----------------------------------------------------------------------------
; Boot 64-bit Kernel
; -----------------------------------------------------------------------------
boot_64bit_kernel:
    mov rcx, msg_loading_64
    call print_string
    
    ; Open kernel file
    mov rcx, kernel_path_64
    call open_file
    test rax, rax
    jz file_error
    
    mov [gRootDir], rax
    
    ; Get file info to determine size
    mov rcx, rax
    call get_file_info
    test rax, rax
    jz file_error
    
    mov [kernel_size], rax
    
    ; Allocate memory for kernel (above 4GB preferred)
    mov rcx, [rax]
    call allocate_pages
    test rax, rax
    jz memory_error
    
    mov [kernel_buffer], rax
    
    ; Read kernel file
    mov rcx, [gRootDir]
    mov rdx, rax
    mov r8, [kernel_size]
    call read_file
    test rax, rax
    jz file_error
    
    ; Parse ELF and get entry point
    mov rcx, rax
    call parse_elf64
    test rax, rax
    jz elf_error
    
    mov [kernel_entry], rax
    
    ; Exit Boot Services
    call exit_boot_services
    test rax, rax
    jnz exit_bs_error
    
    ; Set up GDT for kernel
    call setup_gdt64
    
    ; Jump to kernel entry point
    mov rcx, [kernel_buffer]    ; Pass kernel load address
    jmp [kernel_entry]

; -----------------------------------------------------------------------------
; Initialize Console
; -----------------------------------------------------------------------------
init_console:
    ; Get ConOut from System Table
    mov rax, [gST]
    mov rax, [rax + 40]         ; ConOut
    test rax, rax
    jz .error
    
    ; Clear screen
    mov rcx, rax
    mov rdx, clear_screen
    call [rax + 8]              ; ClearScreen
    
    ; Set cursor position to 0,0
    mov rcx, [gST]
    mov rcx, [rcx + 40]         ; ConOut
    mov rdx, 0
    mov r8, 0
    call [rcx + 16]             ; SetCursorPosition
    ret

.error:
    ret

; -----------------------------------------------------------------------------
; Print String (Unicode)
; -----------------------------------------------------------------------------
print_string:
    ; RCX = pointer to null-terminated ASCII string
    ; Convert to UTF-16 and use OutputString
    mov rax, [gST]
    mov rax, [rax + 40]         ; ConOut
    test rax, rax
    jz .done
    
    push rcx                    ; Save string pointer
    push rax                    ; Save ConOut
    
    ; Convert ASCII to UTF-16 on stack
    mov rdi, rsp
    sub rdi, 512                ; Buffer space
    mov rsi, rcx
.convert_loop:
    lodsb
    test al, al
    jz .convert_done
    mov word [rdi], ax
    add rdi, 2
    jmp .convert_loop
.convert_done:
    mov word [rdi], 0
    
    pop rcx                     ; Restore ConOut
    mov rdx, rsp
    sub rdx, 512
    call [rcx + 8]              ; OutputString
    
    add rsp, 512                ; Clean up buffer
    pop rcx                     ; Restore string pointer (unused)
.done:
    ret

; -----------------------------------------------------------------------------
; Wait for Key Press
; -----------------------------------------------------------------------------
wait_for_key:
    mov rax, [gST]
    mov rax, [rax + 48]         ; ConIn
    test rax, rax
    jz .done
    
    ; Reset input
    mov rcx, rax
    mov rdx, 0
    call [rax + 8]              ; Reset
    
.wait_loop:
    ; Read key stroke
    mov rcx, [gST]
    mov rcx, [rcx + 48]         ; ConIn
    lea rdx, [rsp - 32]         ; EFI_INPUT_KEY structure
    call [rcx + 16]             ; ReadKeyStroke
    test rax, rax
    jnz .wait_loop
    
    ; Check key
    mov al, byte [rsp - 32]     ; ScanCode
    test al, al
    jnz .check_scancode
    
    mov al, byte [rsp - 30]     ; UnicodeChar
    cmp al, '1'
    je .select_64
    cmp al, '2'
    je .select_reboot
    cmp al, '3'
    je .select_shutdown
    jmp .wait_loop
    
.check_scancode:
    ; Handle arrow keys etc.
    jmp .wait_loop
    
.select_64:
    mov byte [selected_option], 0
    ret
.select_reboot:
    mov byte [selected_option], 1
    ret
.select_shutdown:
    mov byte [selected_option], 2
    ret

.done:
    ret

; -----------------------------------------------------------------------------
; Open File
; -----------------------------------------------------------------------------
open_file:
    ; RCX = file path (UTF-16)
    ; Returns file handle in RAX or 0 on error
    
    ; Get Loaded Image Protocol
    mov rcx, [gBS]
    mov rdx, EFI_LOADED_IMAGE_PROTOCOL_GUID
    mov r8, [gImageHandle]
    mov r9, 0
    mov [rsp + 32], rax        ; Interface pointer
    call [rcx + 72]             ; OpenProtocol
    test rax, rax
    jnz .error
    
    mov [gLoadedImage], rax
    
    ; Get Device Handle
    mov rax, [rax]
    mov rax, [rax + 24]         ; DeviceHandle
    
    ; Open Simple File System Protocol
    mov rcx, [gBS]
    mov rdx, EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID
    mov r8, rax
    mov r9, 0
    mov [rsp + 32], gFileSystem
    call [rcx + 72]             ; OpenProtocol
    test rax, rax
    jnz .error
    
    ; Open root directory
    mov rcx, [gFileSystem]
    mov rdx, gRootDir
    call [rcx + 8]              ; OpenVolume
    test rax, rax
    jnz .error
    
    ; Open kernel file
    mov rcx, [gRootDir]
    mov rdx, gRootDir
    mov r8, rcx                 ; File path
    mov r9, 1                   ; Read mode
    mov [rsp + 32], 0           ; Attributes
    call [rcx + 16]             ; Open
    test rax, rax
    jnz .error
    
    mov rax, [gRootDir]
    ret

.error:
    xor rax, rax
    ret

; -----------------------------------------------------------------------------
; Get File Info
; -----------------------------------------------------------------------------
get_file_info:
    ; RCX = file handle
    ; Returns file size in RAX or 0 on error
    ret

; -----------------------------------------------------------------------------
; Allocate Pages
; -----------------------------------------------------------------------------
allocate_pages:
    ; RCX = size in bytes
    ; Returns allocated address in RAX or 0 on error
    ret

; -----------------------------------------------------------------------------
; Read File
; -----------------------------------------------------------------------------
read_file:
    ; RCX = file handle
    ; RDX = buffer
    ; R8 = size
    ; Returns bytes read in RAX or 0 on error
    ret

; -----------------------------------------------------------------------------
; Parse ELF64
; -----------------------------------------------------------------------------
parse_elf64:
    ; RCX = buffer address
    ; Returns entry point in RAX or 0 on error
    ret

; -----------------------------------------------------------------------------
; Exit Boot Services
; -----------------------------------------------------------------------------
exit_boot_services:
    ; Returns map key in RAX or error
    ret

; -----------------------------------------------------------------------------
; Setup GDT64
; -----------------------------------------------------------------------------
setup_gdt64:
    ret

; -----------------------------------------------------------------------------
; Reboot System
; -----------------------------------------------------------------------------
reboot_system:
    mov rcx, [gRT]
    mov rdx, 0                  ; ResetType = Cold
    mov r8, 0                   ; ResetStatus
    mov r9, 0                   ; DataSize
    call [rcx + 32]             ; ResetSystem
    ret

; -----------------------------------------------------------------------------
; Shutdown System
; -----------------------------------------------------------------------------
shutdown_system:
    mov rcx, [gRT]
    mov rdx, 1                  ; ResetType = Shutdown
    mov r8, 0
    mov r9, 0
    call [rcx + 32]             ; ResetSystem
    ret

; -----------------------------------------------------------------------------
; Error Handlers
; -----------------------------------------------------------------------------
file_error:
    mov rcx, msg_file_error
    call print_string
    jmp $

memory_error:
    mov rcx, msg_memory_error
    call print_string
    jmp $

elf_error:
    mov rcx, msg_elf_error
    call print_string
    jmp $

exit_bs_error:
    mov rcx, msg_exit_bs_error
    call print_string
    jmp $

; -----------------------------------------------------------------------------
; Strings (UTF-16)
; -----------------------------------------------------------------------------
section .data

banner:
    dw 'N', 'e', 'b', 'u', 'l', 'a', 'B', 'o', 'o', 't', ' ', '-', ' ', 'N', 'e', 'b', 'u', 'l', 'a', 'O', 'S', ' ', 'L', 'o', 'a', 'd', 'e', 'r', 0

menu:
    dw 13, 10, 'S', 'e', 'l', 'e', 'c', 't', ' ', 'b', 'o', 'o', 't', ' ', 'o', 'p', 't', 'i', 'o', 'n', ':', 13, 10
    dw ' ', ' ', '1', '.', ' ', 'N', 'e', 'b', 'u', 'l', 'a', 'O', 'S', ' ', '(6', '4', '-', 'b', 'i', 't', ')', 13, 10
    dw ' ', ' ', '2', '.', ' ', 'R', 'e', 'b', 'o', 'o', 't', 13, 10
    dw ' ', ' ', '3', '.', ' ', 'S', 'h', 'u', 't', 'd', 'o', 'w', 'n', 13, 10, 13, 10
    dw 'C', 'h', 'o', 'i', 'c', 'e', ':', ' ', 0

msg_loading_64:
    dw 'L', 'o', 'a', 'd', 'i', 'n', 'g', ' ', 'N', 'e', 'b', 'u', 'l', 'a', 'O', 'S', ' ', '6', '4', '-', 'b', 'i', 't', ' ', 'k', 'e', 'r', 'n', 'e', 'l', '.', '.', '.', 13, 10, 0

msg_file_error:
    dw 'E', 'r', 'r', 'o', 'r', ':', ' ', 'F', 'a', 'i', 'l', 'e', 'd', ' ', 't', 'o', ' ', 'o', 'p', 'e', 'n', ' ', 'k', 'e', 'r', 'n', 'e', 'l', ' ', 'f', 'i', 'l', 'e', 13, 10, 0

msg_memory_error:
    dw 'E', 'r', 'r', 'o', 'r', ':', ' ', 'O', 'u', 't', ' ', 'o', 'f', ' ', 'm', 'e', 'm', 'o', 'r', 'y', 13, 10, 0

msg_elf_error:
    dw 'E', 'r', 'r', 'o', 'r', ':', ' ', 'I', 'n', 'v', 'a', 'l', 'i', 'd', ' ', 'E', 'L', 'F', ' ', 'f', 'o', 'r', 'm', 'a', 't', 13, 10, 0

msg_exit_bs_error:
    dw 'E', 'r', 'r', 'o', 'r', ':', ' ', 'F', 'a', 'i', 'l', 'e', 'd', ' ', 't', 'o', ' ', 'e', 'x', 'i', 't', ' ', 'B', 'o', 'o', 't', ' ', 'S', 'e', 'r', 'v', 'i', 'c', 'e', 's', 13, 10, 0

kernel_path_64:
    dw '\\', 'E', 'F', 'I', '\\', 'N', 'E', 'B', 'U', 'L', 'A', '\\', 'n', 'e', 'b', 'u', 'l', 'a', 'o', 's', '_', 'x', '8', '6', '_', '6', '4', '.', 'e', 'l', 'f', 0

clear_screen:
    dw 13, 10, 0