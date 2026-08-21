// NebulaOS - Global Descriptor Table (GDT)
// ==========================================
//
// GDT definitions for both x86 and x86_64

#ifndef NEBULAOS_GDT_H
#define NEBULAOS_GDT_H

#include "nebula.h"
#include "stdint.h"

// Segment selector values
#define GDT_NULL_SEL       0x00
#define GDT_CODE_SEL       0x08
#define GDT_DATA_SEL       0x10
#define GDT_USER_CODE_SEL  0x18
#define GDT_USER_DATA_SEL  0x20
#define GDT_TSS_SEL        0x28

// GDT entry structure (8 bytes)
typedef struct PACKED {
    uint16_t limit_low;   // Segment limit bits 0-15
    uint16_t base_low;    // Segment base bits 0-15
    uint8_t  base_mid;    // Segment base bits 16-23
    uint8_t  access;      // Access byte
    uint8_t  limit_high;  // Segment limit bits 16-19 + flags
    uint8_t  base_high;   // Segment base bits 24-31
} gdt_entry_t;

// GDT pointer structure (6 bytes on x86, 8 bytes on x86_64)
typedef struct PACKED {
    uint16_t limit;
    uint32_t base;
#ifdef NEBULAOS_ARCH_X86_64
    uint32_t base_upper;
    uint16_t reserved;
#endif
} gdt_ptr_t;

// TSS structure for x86 (104 bytes)
typedef struct PACKED {
    uint16_t prev_tss;
    uint16_t reserved1;
    uint32_t esp0;        // Stack pointer for ring 0
    uint16_t ss0;         // Stack segment for ring 0
    uint16_t reserved2;
    uint32_t esp1;
    uint16_t ss1;
    uint16_t reserved3;
    uint32_t esp2;
    uint16_t ss2;
    uint16_t reserved4;
    uint32_t cr3;
    uint32_t eip;
    uint32_t eflags;
    uint32_t eax;
    uint32_t ecx;
    uint32_t edx;
    uint32_t ebx;
    uint32_t esp;
    uint32_t ebp;
    uint32_t esi;
    uint32_t edi;
    uint16_t es;
    uint16_t reserved5;
    uint16_t cs;
    uint16_t reserved6;
    uint16_t ss;
    uint16_t reserved7;
    uint16_t ds;
    uint16_t reserved8;
    uint16_t fs;
    uint16_t reserved9;
    uint16_t gs;
    uint16_t reserved10;
    uint16_t ldt;
    uint16_t reserved11;
    uint16_t trap;
    uint16_t iomap_base;
} tss_t;

// Access byte flags
#define GDT_ACCESS_PRESENT     0x80
#define GDT_ACCESS_PRIV_0      0x00  // Ring 0
#define GDT_ACCESS_PRIV_3      0x60  // Ring 3
#define GDT_ACCESS_TYPE_CODE   0x08  // Code segment
#define GDT_ACCESS_TYPE_DATA   0x00  // Data segment
#define GDT_ACCESS_CONFORMING  0x04  // Conforming (for code)
#define GDT_ACCESS_EXPAND_DOWN 0x04  // Expand down (for data)
#define GDT_ACCESS_READABLE    0x02  // Readable (for code)
#define GDT_ACCESS_WRITABLE    0x02  // Writable (for data)
#define GDT_ACCESS_ACCESSED    0x01  // Accessed

// Limit/flags byte flags
#define GDT_FLAGS_GRANULARITY  0x80  // Granularity (byte vs page)
#define GDT_FLAGS_32BIT        0x40  // 32-bit segment
#define GDT_FLAGS_64BIT        0x20  // 64-bit code segment (long mode)
#define GDT_FLAGS_AVL          0x10  // Available for use

// Function prototypes
void init_gdt(void);
void init_gdt64(void);
void gdt_set_gate(int32_t num, uint32_t base, uint32_t limit, uint8_t access, uint8_t flags);
void gdt_flush(void);
void gdt_flush64(void);
void tss_flush(void);

#endif // NEBULAOS_GDT_H
