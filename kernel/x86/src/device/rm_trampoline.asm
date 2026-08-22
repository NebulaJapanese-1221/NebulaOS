; NebulaOS - Real Mode BIOS Interface (x86)
; ==========================================
;
; Provides real-mode INT calls (e.g. VBE INT 0x10) by switching the CPU
; from protected mode back to real mode via a low-memory trampoline.
;
; Layout of reserved low memory (identity-mapped, all < 1MB):
;   0x9000  GDTR (6 bytes) + temporary GDT (5 entries, 40 bytes)
;   0x9040  rm_regs_t parameter block (42 bytes)
;   0x9080  VBE controller info scratch buffer (512 bytes)
;   0x9280  VBE mode info scratch buffer (256 bytes)
;   0x9400  16-bit trampoline code (copied here at init)
;   0x8000  real-mode stack (grows up to 0x9000)

bits 32

; Low memory addresses
%define RM_BASE           0x9000
%define RM_PARAMS_PHYS    0x9040
%define RM_VBE_INFO_PHYS  0x9080
%define RM_VBE_MODE_PHYS  0x9280
%define RM_CODE_PHYS      0x9400
%define RM_STACK_PHYS     0x9000   ; top of real-mode stack

; real-mode segment values (seg = phys >> 4)
%define RM_PARAMS_SEG     0x0904
%define RM_VBE_INFO_SEG   0x0908
%define RM_VBE_MODE_SEG   0x0928
%define RM_CODE_SEG       0x0940
%define RM_STACK_SEG      0x0800
%define RM_STACK_OFF      0x1000

; rm_regs_t field offsets (must match realmode.h)
%define R_EAX    0
%define R_EBX    4
%define R_ECX    8
%define R_EDX    12
%define R_ESI    16
%define R_EDI    20
%define R_EBP    24
%define R_DS     28
%define R_ES     30
%define R_FS     32
%define R_GS     34
%define R_SS     36
%define R_FLAGS  38
%define R_INT    40

; selectors inside the temporary real-mode GDT
%define SEL_CODE32 0x08
%define SEL_DATA32 0x10
%define SEL_CODE16 0x18
%define SEL_DATA16 0x20

; kernel GDT selectors (match gdt.h)
%define KERNEL_CODE_SEL 0x08
%define KERNEL_DATA_SEL 0x10

global realmode_install
global realmode_call
global realmode_available_flag

section .bss
realmode_available_flag: resd 1
realmode_saved_gdtr:     resd 2
realmode_saved_idtr:     resd 2
realmode_saved_cr0:      resd 1
realmode_saved_cr4:      resd 1
realmode_saved_cr3:      resd 1
realmode_saved_esp:      resd 1

section .text

; ---------------------------------------------------------------------------
; realmode_install: copy the 16-bit trampoline into low memory.
; ---------------------------------------------------------------------------
realmode_install:
    mov esi, rm_trampoline_start
    mov edi, RM_CODE_PHYS
    mov ecx, rm_trampoline_end - rm_trampoline_start
    cld
    rep movsb
    mov dword [realmode_available_flag], 1
    ret

; ---------------------------------------------------------------------------
; realmode_call(uint8_t int_num, rm_regs_t* regs)
;   int_num at [esp+4], regs at [esp+8]
; Returns 1 on success (carry clear), 0 otherwise.
; ---------------------------------------------------------------------------
realmode_call:
    cmp dword [realmode_available_flag], 0
    je rm_unavail
    mov [realmode_saved_esp], esp          ; save entry esp

    pushfd
    cli
    push ds
    push es
    push fs
    push gs
    push ebp
    push ebx
    push esi
    push edi

    ; copy caller registers into low-memory parameter block
    mov esi, [esp+44]                      ; regs pointer
    mov edi, RM_PARAMS_PHYS
    mov ecx, 42
    cld
    rep movsb
    mov al, [esp+40]                       ; int_num
    mov [RM_PARAMS_PHYS + R_INT], al

    ; load the temporary real-mode GDT (in low memory)
    lgdt [RM_BASE]

    ; disable paging
    mov eax, cr0
    and eax, 0x7FFFFFFF
    mov cr0, eax
    jmp SEL_CODE32:.pm_flat

.pm_flat:
    mov ax, SEL_DATA32
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, RM_STACK_PHYS

    ; switch to real mode via the 16-bit code segment
    jmp SEL_CODE16:RM_CODE_PHYS

; ===========================================================================
; 16-bit real-mode trampoline (copied to RM_CODE_PHYS)
; ===========================================================================
rm_trampoline_start:
bits 16
    ; real-mode data segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; real-mode stack
    mov ax, RM_STACK_SEG
    mov ss, ax
    mov sp, RM_STACK_OFF

    ; load general registers from the parameter block
    mov ax, RM_PARAMS_SEG
    mov ds, ax
    mov eax, [R_EAX]
    mov ebx, [R_EBX]
    mov ecx, [R_ECX]
    mov edx, [R_EDX]
    mov esi, [R_ESI]
    mov edi, [R_EDI]
    mov ebp, [R_EBP]

    ; load the segment registers the caller requested
    mov ds, [R_DS]
    mov es, [R_ES]
    mov fs, [R_FS]
    mov gs, [R_GS]

    ; dispatch the requested interrupt
    mov al, [R_INT]
    cmp al, 0x10
    je .do_int10
    jmp .after

.do_int10:
    int 0x10
.after:
    ; save results back into the parameter block
    mov ax, RM_PARAMS_SEG
    mov ds, ax
    mov [R_EAX], eax
    mov [R_EBX], ebx
    mov [R_ECX], ecx
    mov [R_EDX], edx
    mov [R_ESI], esi
    mov [R_EDI], edi
    mov [R_EBP], ebp
    pushf
    pop ax
    mov [R_FLAGS], ax

    ; return to protected mode
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    ; 32-bit far jump back into the kernel (selector 0x08)
    db 0x66
    db 0xEA
    dd return_to_pm
    dw SEL_CODE32
rm_trampoline_end:

; ===========================================================================
; return to protected mode (kernel high memory)
; ===========================================================================
return_to_pm:
bits 32
    mov ax, SEL_DATA32
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; restore control registers
    mov eax, [realmode_saved_cr4]
    mov cr4, eax
    mov eax, [realmode_saved_cr3]
    mov cr3, eax
    mov eax, [realmode_saved_cr0]
    mov cr0, eax
    jmp SEL_CODE32:.pm_resume

.pm_resume:
    lgdt [realmode_saved_gdtr]
    lidt [realmode_saved_idtr]
    mov esp, [realmode_saved_esp]

    ; copy parameter block back to caller
    mov esi, RM_PARAMS_PHYS
    mov edi, [esp+8]
    mov ecx, 42
    cld
    rep movsb

    ; build return value from the carry flag (success = carry clear)
    mov esi, RM_PARAMS_PHYS
    mov ax, [esi + R_FLAGS]
    and ax, 1
    mov ecx, eax
    mov eax, 1
    test cl, 1
    jz .ret_ok
    xor eax, eax
.ret_ok:
    pop edi
    pop esi
    pop ebx
    pop ebp
    pop gs
    pop fs
    pop es
    pop ds
    popfd
    ret

 rm_unavail:
    xor eax, eax
    ret
