// NebulaOS - x86_64 Paging
// =========================
//
// Basic identity-mapped paging for x86_64

#include "../../../common/include/nebula.h"
#include "../../../common/include/stdint.h"

// -----------------------------------------------------------------------------
// Page table structures
// -----------------------------------------------------------------------------

typedef struct PACKED {
    uint64_t present    : 1;
    uint64_t writable   : 1;
    uint64_t user       : 1;
    uint64_t wthrough   : 1;
    uint64_t cache      : 1;
    uint64_t accessed   : 1;
    uint64_t dirty      : 1;
    uint64_t pat        : 1;
    uint64_t global     : 1;
    uint64_t available  : 3;
    uint64_t frame      : 40;
    uint64_t reserved   : 11;
    uint64_t nx         : 1;
} page_entry_t;

#define PAGE_SIZE 4096
#define PAGE_SHIFT 12
#define PAGE_MASK 0xFFFFFFFFFFFFF000

// -----------------------------------------------------------------------------
// Initialize paging
// -----------------------------------------------------------------------------

static page_entry_t* pml4;
static page_entry_t* pdp;
static page_entry_t* pd;
static page_entry_t* pt;

void init_paging64(void) {
    uint64_t phys = 0;

    pml4 = (page_entry_t*)0x1000;
    pdp  = (page_entry_t*)0x2000;
    pd   = (page_entry_t*)0x3000;
    pt   = (page_entry_t*)0x4000;

    for (int i = 0; i < 512; i++) {
        pml4[i].present = 0;
        pdp[i].present = 0;
        pd[i].present = 0;
        pt[i].present = 0;
    }

    pml4[0].present = 1;
    pml4[0].writable = 1;
    pml4[0].frame = (uint64_t)pdp >> PAGE_SHIFT;

    pdp[0].present = 1;
    pdp[0].writable = 1;
    pdp[0].frame = (uint64_t)pd >> PAGE_SHIFT;

    for (int i = 0; i < 512; i++) {
        pd[i].present = 1;
        pd[i].writable = 1;
        pd[i].frame = (uint64_t)(pt + i) >> PAGE_SHIFT;

        for (int j = 0; j < 512; j++) {
            pt[j].present = 1;
            pt[j].writable = 1;
            pt[j].frame = (phys >> PAGE_SHIFT);
            pt += 512;
            phys += PAGE_SIZE;
        }
    }

    __asm__ __volatile__(
        "mov %0, %%cr3\n"
        "mov %%cr0, %%rax\n"
        "orl $0x80000000, %%eax\n"
        "mov %%rax, %%cr0\n"
        "mov %%cr4, %%rax\n"
        "orl $0x20, %%eax\n"
        "mov %%rax, %%cr4\n"
        : : "r"(pml4) : "rax", "memory"
    );
}
