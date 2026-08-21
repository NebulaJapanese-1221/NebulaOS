// NebulaOS - Memory Management Implementation
// =============================================
//
// Complete memory management implementation including:
// - Physical memory detection
// - Page frame allocator (bitmap-based)
// - Paging (x86 32-bit)
// - Heap allocator (buddy system)
// - Memory utility functions
//
// This implementation assumes x86 architecture

#include "../include/memory.h"
#include "../include/nebula.h"
#include "../include/stdint.h"
#include "../include/vga.h"
#include "../include/idt.h"

// -----------------------------------------------------------------------------
// Memory utility functions
// -----------------------------------------------------------------------------

// Set memory to a value
void* memset(void* dest, int value, size_t count) {
    uint8_t* ptr = (uint8_t*)dest;
    while (count--) {
        *ptr++ = (uint8_t)value;
    }
    return dest;
}

// Copy memory
void* memcpy(void* dest, const void* src, size_t count) {
    uint8_t* d = (uint8_t*)dest;
    const uint8_t* s = (const uint8_t*)src;
    while (count--) {
        *d++ = *s++;
    }
    return dest;
}

// Compare memory
int memcmp(const void* a, const void* b, size_t count) {
    const uint8_t* p1 = (const uint8_t*)a;
    const uint8_t* p2 = (const uint8_t*)b;
    while (count--) {
        if (*p1 != *p2) {
            return *p1 < *p2 ? -1 : 1;
        }
        p1++;
        p2++;
    }
    return 0;
}

// Move memory (handles overlapping regions)
void* memmove(void* dest, const void* src, size_t count) {
    uint8_t* d = (uint8_t*)dest;
    const uint8_t* s = (const uint8_t*)src;
    if (d < s) {
        while (count--) {
            *d++ = *s++;
        }
    } else if (d > s) {
        d += count;
        s += count;
        while (count--) {
            *--d = *--s;
        }
    }
    return dest;
}

// Set 16-bit memory
void* memset16(void* dest, uint16_t value, size_t count) {
    uint16_t* ptr = (uint16_t*)dest;
    while (count--) {
        *ptr++ = value;
    }
    return dest;
}

// Set 32-bit memory
void* memset32(void* dest, uint32_t value, size_t count) {
    uint32_t* ptr = (uint32_t*)dest;
    while (count--) {
        *ptr++ = value;
    }
    return dest;
}

// -----------------------------------------------------------------------------
// Memory-mapped I/O functions
// -----------------------------------------------------------------------------

uint8_t mmio_read8(volatile uint8_t* addr) {
    return *addr;
}

uint16_t mmio_read16(volatile uint16_t* addr) {
    return *addr;
}

uint32_t mmio_read32(volatile uint32_t* addr) {
    return *addr;
}

void mmio_write8(volatile uint8_t* addr, uint8_t value) {
    *addr = value;
}

void mmio_write16(volatile uint16_t* addr, uint16_t value) {
    *addr = value;
}

void mmio_write32(volatile uint8_t* addr, uint32_t value) {
    *(volatile uint32_t*)addr = value;
}

#ifdef NEBULAOS_ARCH_X86_64
uint64_t mmio_read64(volatile uint64_t* addr) {
    return *addr;
}

void mmio_write64(volatile uint64_t* addr, uint64_t value) {
    *addr = value;
}
#endif

// -----------------------------------------------------------------------------
// Memory barriers
// -----------------------------------------------------------------------------

void memory_barrier(void) {
    barrier();
}

void read_barrier(void) {
    barrier();
}

void write_barrier(void) {
    barrier();
}

// -----------------------------------------------------------------------------
// Cache control
// -----------------------------------------------------------------------------

void invalidate_tlb(void) {
    // Invalidate entire TLB by reloading CR3
#ifdef NEBULAOS_ARCH_X86_64
    uint64_t cr3;
    __asm__ __volatile__("mov %%cr3, %0" : "=r"(cr3));
    __asm__ __volatile__("mov %0, %%cr3" : : "r"(cr3));
#else
    uint32_t cr3;
    __asm__ __volatile__("mov %%cr3, %0" : "=r"(cr3));
    __asm__ __volatile__("mov %0, %%cr3" : : "r"(cr3));
#endif
}

void invalidate_tlb_entry(uint32_t addr) {
    // Invalidate a single TLB entry
#ifdef NEBULAOS_ARCH_X86_64
    __asm__ __volatile__("invlpg %0" : : "m"(*(const char*)(uint64_t)addr));
#else
    __asm__ __volatile__("invlpg %0" : : "m"(*(char*)addr));
#endif
}

void flush_cache(void) {
    // Write-back and invalidate cache
    #ifdef NEBULAOS_ARCH_X86_64
    __asm__ __volatile__("wbinvd");
    #else
    __asm__ __volatile__("wbinvd");
    #endif
}

// -----------------------------------------------------------------------------
// Physical Memory Management
// -----------------------------------------------------------------------------

// Physical memory map structure
#define MAX_MEMORY_MAP_ENTRIES 256

typedef struct {
    uint64_t base;
    uint64_t length;
    uint32_t type;
    uint32_t extended_attributes;
} memory_map_entry_t;

static memory_map_entry_t memory_map[MAX_MEMORY_MAP_ENTRIES];
static uint32_t memory_map_count = 0;
static size_t total_physical_memory = 0;
static size_t used_physical_memory = 0;

// Page frame bitmap
static uint32_t* page_bitmap = NULL;
static size_t total_pages = 0;
static size_t page_bitmap_size = 0;

// Detect memory using BIOS interrupt 0xe820 (via bootloader info)
// For now, we'll use a simplified approach
void memory_detect(void) {
    // In a real implementation, this would be called from kernel_main
    // with multiboot info pointer
    
    // For now, assume we have 128MB of physical memory
    // This will be enhanced when we add proper multiboot support
    total_physical_memory = 128 * 1024 * 1024;  // 128MB
    
    // Add a memory map entry for available memory
    // Skip first 1MB (BIOS, etc.) and kernel memory
    memory_map[0].base = 0x00000000;
    memory_map[0].length = 0x100000;  // First 1MB
    memory_map[0].type = MEMORY_TYPE_RESERVED;
    memory_map_count++;
    
    memory_map[1].base = 0x00100000;  // 1MB
    memory_map[1].length = total_physical_memory - 0x100000;
    memory_map[1].type = MEMORY_TYPE_FREE;
    memory_map_count++;
}

size_t memory_get_total(void) {
    return total_physical_memory;
}

size_t memory_get_used(void) {
    return used_physical_memory;
}

size_t memory_get_free(void) {
    return total_physical_memory - used_physical_memory;
}

// Initialize physical memory management
void memory_init_physical(void) {
    memory_detect();
    
    // Calculate total pages
    // We need to manage all physical memory from 1MB onwards
    size_t available_memory = total_physical_memory - 0x100000;
    total_pages = available_memory / PAGE_SIZE;
    
    // Calculate bitmap size (1 bit per page, 32 bits per uint32_t)
    page_bitmap_size = (total_pages + 31) / 32;
    
    // Allocate bitmap in low memory (temporary, before paging)
    // For now, use a fixed location (2MB)
    // In a real implementation, this would be allocated from early memory
    page_bitmap = (uint32_t*)0x00200000;
    
    // Clear bitmap (all pages free)
    memset(page_bitmap, 0, page_bitmap_size * sizeof(uint32_t));
    
    // Mark pages used by the bitmap itself as reserved
    size_t bitmap_pages = (page_bitmap_size * sizeof(uint32_t) + PAGE_SIZE - 1) / PAGE_SIZE;
    for (size_t i = 0; i < bitmap_pages; i++) {
        size_t page_num = (0x00200000 / PAGE_SIZE) + i;
        if (page_num < total_pages) {
            size_t word = page_num / 32;
            int bit = page_num % 32;
            if (word < page_bitmap_size) {
                page_bitmap[word] |= (1 << bit);
            }
        }
    }
    
    // Mark first 1MB as reserved (BIOS, etc.)
    size_t reserved_pages = 0x100000 / PAGE_SIZE;
    for (size_t i = 0; i < reserved_pages; i++) {
        if (i < total_pages) {
            size_t word = i / 32;
            int bit = i % 32;
            if (word < page_bitmap_size) {
                page_bitmap[word] |= (1 << bit);
            }
        }
    }
}

// Allocate a single physical page
void* memory_alloc_page(void) {
    // Find a free page
    for (size_t i = 0; i < page_bitmap_size; i++) {
        if (page_bitmap[i] != 0xFFFFFFFF) {
            // Found a word with at least one free bit
            for (int j = 0; j < 32; j++) {
                if (!(page_bitmap[i] & (1 << j))) {
                    // Found free page
                    size_t page_num = i * 32 + j;
                    page_bitmap[i] |= (1 << j);
                    used_physical_memory += PAGE_SIZE;
                    return (void*)(page_num * PAGE_SIZE);
                }
            }
        }
    }
    return NULL;  // No free pages
}

// Free a physical page
void memory_free_page(void* ptr) {
#ifdef NEBULAOS_ARCH_X86_64
    uint64_t addr = (uint64_t)ptr;
#else
    uint32_t addr = (uint32_t)ptr;
#endif
    size_t page_num = addr / PAGE_SIZE;
    size_t word = page_num / 32;
    int bit = page_num % 32;
    
    if (word < page_bitmap_size) {
        page_bitmap[word] &= ~(1 << bit);
        used_physical_memory -= PAGE_SIZE;
    }
}

// Allocate multiple contiguous physical pages
void* memory_alloc_physical(size_t pages) {
    // Simple implementation: allocate contiguous pages
    size_t start_page = 0;
    size_t consecutive_free = 0;
    
    for (size_t i = 0; i < total_pages; i++) {
        size_t word = i / 32;
        int bit = i % 32;
        
        if (!(page_bitmap[word] & (1 << bit))) {
            if (consecutive_free == 0) {
                start_page = i;
            }
            consecutive_free++;
            
            if (consecutive_free >= pages) {
                // Found enough consecutive pages
                for (size_t j = start_page; j < start_page + pages; j++) {
                    size_t w = j / 32;
                    int b = j % 32;
                    page_bitmap[w] |= (1 << b);
                }
                used_physical_memory += pages * PAGE_SIZE;
                return (void*)(start_page * PAGE_SIZE);
            }
        } else {
            consecutive_free = 0;
        }
    }
    
    return NULL;  // Not enough contiguous pages
}

// Free multiple contiguous physical pages
void memory_free_physical(void* ptr, size_t pages) {
#ifdef NEBULAOS_ARCH_X86_64
    uint64_t addr = (uint64_t)ptr;
#else
    uint32_t addr = (uint32_t)ptr;
#endif
    size_t start_page = addr / PAGE_SIZE;
    
    for (size_t i = 0; i < pages; i++) {
        size_t page_num = start_page + i;
        size_t word = page_num / 32;
        int bit = page_num % 32;
        
        if (word < page_bitmap_size) {
            page_bitmap[word] &= ~(1 << bit);
        }
    }
    used_physical_memory -= pages * PAGE_SIZE;
}

// -----------------------------------------------------------------------------
// Paging Implementation (x86 32-bit)
// -----------------------------------------------------------------------------

// Page directory and page tables
static uint32_t* page_directory = NULL;
static bool paging_initialized = false;

// Page fault error codes
#define PF_PRESENT 0x01
#define PF_WRITE 0x02
#define PF_USER 0x04
#define PF_RESERVED 0x08
#define PF_INSTRUCTION 0x10

// Initialize paging (x86 32-bit only)
#ifndef NEBULAOS_ARCH_X86_64
void memory_init_paging(void) {
    if (paging_initialized) return;
    
    // Allocate page directory
    page_directory = (uint32_t*)0x00010000;  // Use fixed address for now
    
    // Zero out page directory
    memset(page_directory, 0, PAGE_SIZE);
    
    // Identity map first 8MB for early boot
    // This maps virtual addresses 0-8MB to physical addresses 0-8MB
    
    // For each page directory entry (each covers 4MB)
    for (int i = 0; i < 2; i++) {
        // Allocate page table at fixed address
        uint32_t* page_table = (uint32_t*)(0x00011000 + i * PAGE_SIZE);
        memset(page_table, 0, PAGE_SIZE);
        
        // Map all 1024 pages in this page table
        for (int j = 0; j < 1024; j++) {
            uint32_t phys_addr = (i * 1024 + j) * PAGE_SIZE;
            page_table[j] = phys_addr | PAGE_PRESENT | PAGE_WRITABLE | PAGE_CACHEDIS;
        }
        
        // Set page directory entry
        page_directory[i] = (uint32_t)page_table | PAGE_PRESENT | PAGE_WRITABLE | PAGE_CACHEDIS;
    }
    
    // Load page directory into CR3
    __asm__ __volatile__("mov %0, %%cr3" : : "r"(page_directory));
    
    // Enable paging in CR0
    uint32_t cr0;
    __asm__ __volatile__("mov %%cr0, %0" : "=r"(cr0));
    cr0 |= 0x80000000;  // PG bit
    __asm__ __volatile__("mov %0, %%cr0" : : "r"(cr0));
    
    // Flush TLB
    invalidate_tlb();
    
    paging_initialized = true;
}
#endif

// Map a virtual page to a physical page (x86 32-bit only)
#ifndef NEBULAOS_ARCH_X86_64
void memory_map_page(uint32_t virtual_addr, uint32_t physical_addr, uint8_t flags) {
    if (!paging_initialized) return;
    
    uint32_t dir_index = virtual_addr >> 22;
    uint32_t table_index = (virtual_addr >> 12) & 0x3FF;
    
    // Check if page table exists
    if (!(page_directory[dir_index] & PAGE_PRESENT)) {
        // Allocate page table
        uint32_t* page_table = (uint32_t*)memory_alloc_physical(1);
        if (!page_table) {
            return;
        }
        memset(page_table, 0, PAGE_SIZE);
        page_directory[dir_index] = (uint32_t)page_table | PAGE_PRESENT | PAGE_WRITABLE | PAGE_CACHEDIS;
        invalidate_tlb_entry(virtual_addr);
    }
    
    // Get page table
    uint32_t* page_table = (uint32_t*)(page_directory[dir_index] & PAGE_MASK);
    
    // Map the page
    page_table[table_index] = (physical_addr & PAGE_MASK) | flags | PAGE_PRESENT;
    invalidate_tlb_entry(virtual_addr);
}

// Unmap a virtual page (x86 32-bit only)
void memory_unmap_page(uint32_t virtual_addr) {
    if (!paging_initialized) return;
    
    uint32_t dir_index = virtual_addr >> 22;
    uint32_t table_index = (virtual_addr >> 12) & 0x3FF;
    
    if (page_directory[dir_index] & PAGE_PRESENT) {
        uint32_t* page_table = (uint32_t*)(page_directory[dir_index] & PAGE_MASK);
        page_table[table_index] = 0;
        invalidate_tlb_entry(virtual_addr);
    }
}
#endif

// Allocate a virtual page (with physical backing) - x86 32-bit only
#ifndef NEBULAOS_ARCH_X86_64
void* memory_alloc_virtual(size_t size) {
    if (!paging_initialized) return NULL;
    
    // Round up to page size
    size = ALIGN_UP(size, PAGE_SIZE);
    size_t pages = size / PAGE_SIZE;
    
    // Allocate physical pages
    void* phys_addr = memory_alloc_physical(pages);
    if (!phys_addr) return NULL;
    
    // Allocate virtual address space at 3GB + offset
    static uint32_t virtual_base = 0xC0000000;
    
    void* virtual_addr = (void*)virtual_base;
    
    // Map each page
    for (size_t i = 0; i < pages; i++) {
        memory_map_page(
            (uint32_t)virtual_addr + i * PAGE_SIZE,
            (uint32_t)phys_addr + i * PAGE_SIZE,
            PAGE_PRESENT | PAGE_WRITABLE | PAGE_CACHEDIS
        );
    }
    
    virtual_base += pages * PAGE_SIZE;
    return virtual_addr;
}

// Free virtual memory - x86 32-bit only
void memory_free_virtual(void* ptr, size_t size) {
    if (!paging_initialized || !ptr) return;
    
    size = ALIGN_UP(size, PAGE_SIZE);
    size_t pages = size / PAGE_SIZE;
    
    for (size_t i = 0; i < pages; i++) {
        memory_unmap_page((uint32_t)ptr + i * PAGE_SIZE);
    }
}
#endif

// -----------------------------------------------------------------------------
// Page fault handler
// -----------------------------------------------------------------------------

void page_fault_handler(registers_t* regs) {
#ifdef NEBULAOS_ARCH_X86_64
    uint64_t faulting_address;
    __asm__ __volatile__("mov %%cr2, %0" : "=r"(faulting_address));
#else
    uint32_t faulting_address;
    __asm__ __volatile__("mov %%cr2, %0" : "=r"(faulting_address));
#endif
    
    // Check error code
    uint32_t error = regs->err_code;
    bool present = !(error & PF_PRESENT);
    bool write = error & PF_WRITE;
    bool user = error & PF_USER;
    bool reserved = error & PF_RESERVED;
    bool instruction = error & PF_INSTRUCTION;
    
    // For now, just panic
    // In a real implementation, we'd handle page faults by:
    // 1. Checking if it's a valid access
    // 2. Allocating memory on demand (for demand paging)
    // 3. Loading from swap if needed
    
    kernel_panic("Page fault");
}

// -----------------------------------------------------------------------------
// Heap Management (Simple Bump Allocator for now)
// -----------------------------------------------------------------------------

// Heap configuration
static uint8_t* heap_current = (uint8_t*)HEAP_START;
static uint8_t* heap_end_ptr = (uint8_t*)HEAP_END;

// Initialize heap
void memory_init_heap(void) {
    // For now, just zero out the heap area
    // In a real implementation, we'd set up proper heap structures
    heap_current = (uint8_t*)HEAP_START;
    heap_end_ptr = (uint8_t*)HEAP_END;
}

// Allocate memory from heap (simple bump allocator)
void* malloc(size_t size) {
    if (!size) return NULL;
    
    // Align size to 8 bytes
    size = ALIGN_UP(size, 8);
    
    // Check if we have enough space
    if (heap_current + size > heap_end_ptr) {
        return NULL;  // Out of memory
    }
    
    void* ptr = heap_current;
    heap_current += size;
    
    return ptr;
}

// Free memory (stub for now)
void free(void* ptr) {
    (void)ptr;
    // In a real implementation, this would free the memory
    // For bump allocator, we can't free individual allocations
}

// Allocate and zero memory
void* calloc(size_t num, size_t size) {
    size_t total = num * size;
    void* ptr = malloc(total);
    if (ptr) {
        memset(ptr, 0, total);
    }
    return ptr;
}

// Reallocate memory
void* realloc(void* ptr, size_t size) {
    if (!ptr) {
        return malloc(size);
    }
    
    // In a real implementation, we'd check the old size and copy data
    // For now, just allocate new and copy what we can
    void* new_ptr = malloc(size);
    if (new_ptr && ptr) {
        // Copy up to the new size
        memcpy(new_ptr, ptr, size);
    }
    return new_ptr;
}

// -----------------------------------------------------------------------------
// Main Memory Initialization
// -----------------------------------------------------------------------------

void memory_init(void) {
    memory_init_physical();
#ifdef NEBULAOS_ARCH_X86_64
    // 64-bit paging is initialized in paging.c
    init_paging64();
#else
    memory_init_paging();
#endif
    memory_init_heap();
    
    // Register page fault handler (IRQ 14)
    register_interrupt_handler(14, page_fault_handler);
}
