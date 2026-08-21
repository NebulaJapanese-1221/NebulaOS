// NebulaOS - Memory Management
// ==============================
//
// Physical and virtual memory management definitions

#ifndef NEBULAOS_MEMORY_H
#define NEBULAOS_MEMORY_H

#include "nebula.h"
#include "stdint.h"
#include "idt.h"

// Memory constants
#define PAGE_SIZE 4096          // 4KB pages
#define PAGE_SHIFT 12            // 2^12 = 4096
#define PAGE_MASK (~(PAGE_SIZE - 1))

#define HEAP_START 0x1000000    // 16MB - start of kernel heap
#define HEAP_SIZE  0x4000000    // 64MB - size of kernel heap
#define HEAP_END   (HEAP_START + HEAP_SIZE)

// Physical memory constants
#define PHYSICAL_MEMORY_START 0x100000  // 1MB - start of available physical memory
#define PHYSICAL_MEMORY_END   0xFFFFFFFF // 4GB - end of 32-bit address space
#ifdef NEBULAOS_ARCH_X86_64
#define PHYSICAL_MEMORY_END   0x00007FFFFFFFFFFF // 128TB for 48-bit addressing
#endif

// Memory types
#define MEMORY_TYPE_FREE 0
#define MEMORY_TYPE_RESERVED 1
#define MEMORY_TYPE_ACPI 2
#define MEMORY_TYPE_KERNEL 3
#define MEMORY_TYPE_USER 4

// Page frame structure
typedef struct {
    uint8_t allocated : 1;    // Is this frame allocated?
    uint8_t reserved : 1;     // Is this frame reserved?
    uint8_t type : 4;        // Memory type
    uint8_t count : 2;       // Count for buddy system (unused in bitmap)
} page_frame_t;

// Memory block structure (for heap allocation)
typedef struct memory_block {
    size_t size;             // Size of this block
    struct memory_block* next; // Next block in free list
    struct memory_block* prev; // Previous block in free list
    bool free;               // Is this block free?
} memory_block_t;

// Page directory entry (x86)
typedef struct PACKED {
    uint8_t present : 1;
    uint8_t writable : 1;
    uint8_t user : 1;
    uint8_t write_through : 1;
    uint8_t cache_disabled : 1;
    uint8_t accessed : 1;
    uint8_t dirty : 1;
    uint8_t page_size : 1;    // 0 = 4KB, 1 = 4MB (if PS bit set in CR4)
    uint8_t global : 1;       // Global TLB entry
    uint8_t ignored : 3;
    uint32_t address : 20;    // Physical address (shifted right by 12 bits)
} page_dir_entry_t;

// Page table entry (x86)
typedef struct PACKED {
    uint8_t present : 1;
    uint8_t writable : 1;
    uint8_t user : 1;
    uint8_t write_through : 1;
    uint8_t cache_disabled : 1;
    uint8_t accessed : 1;
    uint8_t dirty : 1;
    uint8_t pat : 1;          // Page Attribute Table
    uint8_t global : 1;       // Global TLB entry
    uint8_t ignored : 3;
    uint32_t address : 20;    // Physical address (shifted right by 12 bits)
} page_table_entry_t;

// x86_64 page table entries
#ifdef NEBULAOS_ARCH_X86_64

// PML4 entry
typedef struct PACKED {
    uint8_t present : 1;
    uint8_t writable : 1;
    uint8_t user : 1;
    uint8_t write_through : 1;
    uint8_t cache_disabled : 1;
    uint8_t accessed : 1;
    uint8_t ignored1 : 1;
    uint8_t page_size : 1;    // Must be 0 for PML4
    uint8_t ignored2 : 4;
    uint32_t address : 28;    // Physical address (shifted right by 12 bits)
    uint32_t ignored3 : 12;
    uint8_t xd : 1;          // Execute disable
} pml4_entry_t;

// PDPT entry
typedef struct PACKED {
    uint8_t present : 1;
    uint8_t writable : 1;
    uint8_t user : 1;
    uint8_t write_through : 1;
    uint8_t cache_disabled : 1;
    uint8_t accessed : 1;
    uint8_t ignored1 : 1;
    uint8_t page_size : 1;    // 1 for 1GB pages
    uint8_t ignored2 : 4;
    uint32_t address : 28;    // Physical address (shifted right by 12 bits)
    uint32_t ignored3 : 12;
    uint8_t xd : 1;          // Execute disable
} pdpt_entry_t;

// PDT entry
typedef struct PACKED {
    uint8_t present : 1;
    uint8_t writable : 1;
    uint8_t user : 1;
    uint8_t write_through : 1;
    uint8_t cache_disabled : 1;
    uint8_t accessed : 1;
    uint8_t ignored1 : 1;
    uint8_t page_size : 1;    // 1 for 2MB pages
    uint8_t ignored2 : 4;
    uint32_t address : 28;    // Physical address (shifted right by 12 bits)
    uint32_t ignored3 : 12;
    uint8_t xd : 1;          // Execute disable
} pdt_entry_t;

// PT entry
typedef struct PACKED {
    uint8_t present : 1;
    uint8_t writable : 1;
    uint8_t user : 1;
    uint8_t write_through : 1;
    uint8_t cache_disabled : 1;
    uint8_t accessed : 1;
    uint8_t dirty : 1;
    uint8_t pat : 1;          // Page Attribute Table
    uint8_t global : 1;       // Global TLB entry
    uint8_t ignored1 : 3;
    uint32_t address : 28;    // Physical address (shifted right by 12 bits)
    uint32_t ignored2 : 12;
    uint8_t xd : 1;          // Execute disable
} pt_entry_t;

#endif // NEBULAOS_ARCH_X86_64

// Memory management functions

// Physical memory management
void memory_init(void);
void memory_init_physical(void);
void memory_init_heap(void);
void memory_init_paging(void);
void* memory_alloc_physical(size_t pages);
void memory_free_physical(void* ptr, size_t pages);
void* memory_alloc_page(void);
void memory_free_page(void* ptr);

// Virtual memory management
void* memory_alloc_virtual(size_t size);
void memory_free_virtual(void* ptr, size_t size);
void memory_map_page(uint32_t virtual_addr, uint32_t physical_addr, uint8_t flags);
void memory_unmap_page(uint32_t virtual_addr);

// Heap management
void* malloc(size_t size);
void* calloc(size_t num, size_t size);
void* realloc(void* ptr, size_t size);
void free(void* ptr);

// Page fault handler
void page_fault_handler(registers_t* regs);

#ifdef NEBULAOS_ARCH_X86_64
// Initialize 64-bit paging (defined in kernel/x86_64/src/memory/paging.c)
void init_paging64(void);
#endif

// Memory utility functions
void* memset(void* dest, int value, size_t count);
void* memcpy(void* dest, const void* src, size_t count);
int memcmp(const void* a, const void* b, size_t count);
void* memset16(void* dest, uint16_t value, size_t count);
void* memset32(void* dest, uint32_t value, size_t count);

// I/O functions for memory-mapped I/O
uint8_t mmio_read8(volatile uint8_t* addr);
uint16_t mmio_read16(volatile uint16_t* addr);
uint32_t mmio_read32(volatile uint32_t* addr);
void mmio_write8(volatile uint8_t* addr, uint8_t value);
void mmio_write16(volatile uint16_t* addr, uint16_t value);
void mmio_write32(volatile uint8_t* addr, uint32_t value);

#ifdef NEBULAOS_ARCH_X86_64
uint64_t mmio_read64(volatile uint64_t* addr);
void mmio_write64(volatile uint64_t* addr, uint64_t value);
#endif

// Memory barriers
void memory_barrier(void);
void read_barrier(void);
void write_barrier(void);

// Cache control
void invalidate_tlb(void);
void invalidate_tlb_entry(uint32_t addr);
void flush_cache(void);

// Memory map functions (for detecting available memory)
void memory_detect(void);
size_t memory_get_total(void);
size_t memory_get_used(void);
size_t memory_get_free(void);

// Page flags
#define PAGE_PRESENT   0x01
#define PAGE_WRITABLE  0x02
#define PAGE_USER      0x04
#define PAGE_WRITETHROUGH 0x08
#define PAGE_CACHEDIS  0x10
#define PAGE_ACCESS    0x20
#define PAGE_DIRTY     0x40
#define PAGE_GLOBAL    0x80
#define PAGE_PAT       0x80

#endif // NEBULAOS_MEMORY_H
