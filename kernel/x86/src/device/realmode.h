// NebulaOS - Real Mode BIOS Interface (x86)
// ==========================================
//
// Allows the 32-bit protected-mode kernel to issue real-mode BIOS
// interrupts (e.g. VBE INT 0x10) by briefly switching the CPU back
// to real mode through a low-memory trampoline.
//
// The trampoline, temporary GDT, parameter block and scratch buffers
// all live below 1MB (identity-mapped by the kernel's page tables),
// which is required because real-mode addressing cannot reach the
// high kernel image.

#ifndef NEBULAOS_X86_REALMODE_H
#define NEBULAOS_X86_REALMODE_H

#include "../../common/include/stdint.h"
#include "../../common/include/nebula.h"

// Register block passed to/from a real-mode interrupt.
// Field offsets are shared with rm_trampoline.asm.
typedef struct PACKED {
    uint32_t eax, ebx, ecx, edx, esi, edi, ebp;
    uint16_t ds, es, fs, gs, ss, flags;
    uint8_t  int_num;
    uint8_t  _pad;
} rm_regs_t;

// Initialize the real-mode interface. Must be called after paging is
// enabled (low memory must be identity-mapped).
bool realmode_init(void);

// True once the interface is usable.
bool realmode_available(void);

// Issue a real-mode interrupt. Returns true on success (carry clear).
bool realmode_call(uint8_t int_num, rm_regs_t* regs);

// Low-memory scratch buffers (identity-mapped, usable as ES:DI targets).
void* realmode_vbe_info_buffer(void);
void* realmode_vbe_mode_buffer(void);

#endif // NEBULAOS_X86_REALMODE_H
