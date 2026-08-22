// NebulaOS - ELF Binary Loader Implementation
// ============================================
//
// ELF32/ELF64 binary loader
// Parses ELF headers, validates magic, loads PT_LOAD segments

#include "../include/elf.h"
#include "../include/fs.h"
#include "../include/nebula.h"
#include "../include/stdint.h"
#include "../include/memory.h"

// -----------------------------------------------------------------------------
// Internal helper functions
// -----------------------------------------------------------------------------

static int elf_validate_magic(const uint8_t* ident) {
    return (ident[EI_MAG0] == ELFMAG0 &&
            ident[EI_MAG1] == ELFMAG1 &&
            ident[EI_MAG2] == ELFMAG2 &&
            ident[EI_MAG3] == ELFMAG3);
}

static int elf_load32(const void* image, void** entry_point) {
    const elf32_header_t* header = (const elf32_header_t*)image;
    const uint8_t* phdr_base = (const uint8_t*)image + header->e_phoff;

    *entry_point = (void*)header->e_entry;

    for (int i = 0; i < header->e_phnum; i++) {
        const elf32_program_header_t* ph = (const elf32_program_header_t*)(phdr_base + i * header->e_phentsize);

        if (ph->p_type != PT_LOAD) continue;

        size_t page_count = ALIGN_UP(ph->p_memsz, PAGE_SIZE) / PAGE_SIZE;
        if (page_count == 0) page_count = 1;

        void* mem = NULL;
#ifdef NEBULAOS_ARCH_X86
        mem = memory_alloc_virtual(page_count * PAGE_SIZE);
#else
        mem = malloc(page_count * PAGE_SIZE);
#endif
        if (!mem) return FS_ERROR;

        memset(mem, 0, page_count * PAGE_SIZE);
        memcpy(mem, (const uint8_t*)image + ph->p_offset, ph->p_filesz);
    }

    return FS_SUCCESS;
}

static int elf_load64(const void* image, void** entry_point) {
    const elf64_header_t* header = (const elf64_header_t*)image;
    const uint8_t* phdr_base = (const uint8_t*)image + header->e_phoff;

    *entry_point = (void*)header->e_entry;

    for (int i = 0; i < header->e_phnum; i++) {
        const elf64_program_header_t* ph = (const elf64_program_header_t*)(phdr_base + i * header->e_phentsize);

        if (ph->p_type != PT_LOAD) continue;

        size_t page_count = ALIGN_UP(ph->p_memsz, PAGE_SIZE) / PAGE_SIZE;
        if (page_count == 0) page_count = 1;

        void* mem = malloc(page_count * PAGE_SIZE);
        if (!mem) return FS_ERROR;

        memset(mem, 0, page_count * PAGE_SIZE);
        memcpy(mem, (const uint8_t*)image + ph->p_offset, ph->p_filesz);
    }

    return FS_SUCCESS;
}

// -----------------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------------

int elf_load(const void* image, void** entry_point) {
    const uint8_t* ident = (const uint8_t*)image;

    if (!elf_validate_magic(ident)) {
        return FS_ERROR;
    }

    if (ident[EI_CLASS] == ELFCLASS64) {
        return elf_load64(image, entry_point);
    } else if (ident[EI_CLASS] == ELFCLASS32) {
        return elf_load32(image, entry_point);
    }

    return FS_ERROR;
}
