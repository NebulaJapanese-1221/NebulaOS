// Dynamic Linker for NebulaOS
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#pragma once

#include <cstdint>
#include <cstddef>

namespace kernel::elf {

// ELF types
typedef uint32_t Elf32_Addr;
typedef uint16_t Elf32_Half;
typedef uint32_t Elf32_Off;
typedef int32_t  Elf32_Sword;
typedef uint32_t Elf32_Word;

// ELF header
struct Elf32_Ehdr {
    unsigned char e_ident[16];
    Elf32_Half    e_type;
    Elf32_Half    e_machine;
    Elf32_Word    e_version;
    Elf32_Addr    e_entry;
    Elf32_Off     e_phoff;
    Elf32_Off     e_shoff;
    Elf32_Word    e_flags;
    Elf32_Half    e_ehsize;
    Elf32_Half    e_phentsize;
    Elf32_Half    e_phnum;
    Elf32_Half    e_shentsize;
    Elf32_Half    e_shnum;
    Elf32_Half    e_shstrndx;
};

// Program header
struct Elf32_Phdr {
    Elf32_Word    p_type;
    Elf32_Off     p_offset;
    Elf32_Addr    p_vaddr;
    Elf32_Addr    p_paddr;
    Elf32_Word    p_filesz;
    Elf32_Word    p_memsz;
    Elf32_Word    p_flags;
    Elf32_Word    p_align;
};

// Section header
struct Elf32_Shdr {
    Elf32_Word    sh_name;
    Elf32_Word    sh_type;
    Elf32_Word    sh_flags;
    Elf32_Addr    sh_addr;
    Elf32_Off     sh_offset;
    Elf32_Word    sh_size;
    Elf32_Word    sh_link;
    Elf32_Word    sh_info;
    Elf32_Word    sh_addralign;
    Elf32_Word    sh_entsize;
};

// Symbol table entry
struct Elf32_Sym {
    Elf32_Word    st_name;
    Elf32_Addr    st_value;
    Elf32_Word    st_size;
    unsigned char st_info;
    unsigned char st_other;
    Elf32_Half    st_shndx;
};

// Dynamic section entry
struct Elf32_Dyn {
    Elf32_Sword   d_tag;
    union {
        Elf32_Word d_val;
        Elf32_Addr d_ptr;
    } d_un;
};

// ELF constants
enum {
    ET_NONE   = 0,
    ET_REL    = 1,
    ET_EXEC   = 2,
    ET_DYN    = 3,
    ET_CORE   = 4,

    EM_386    = 3,

    PT_NULL    = 0,
    PT_LOAD    = 1,
    PT_DYNAMIC = 2,
    PT_INTERP  = 3,
    PT_NOTE    = 4,
    PT_SHLIB   = 5,
    PT_PHDR    = 6,

    PF_X = 1,
    PF_W = 2,
    PF_R = 4,

    SHT_NULL     = 0,
    SHT_PROGBITS = 1,
    SHT_SYMTAB   = 2,
    SHT_STRTAB   = 3,
    SHT_RELA     = 4,
    SHT_NOBITS   = 8,
    SHT_DYNAMIC  = 6,
    SHT_DYNSYM   = 11,

    DT_NULL         = 0,
    DT_NEEDED       = 1,
    DT_PLTRELSZ     = 2,
    DT_PLTGOT       = 3,
    DT_HASH         = 4,
    DT_STRTAB       = 5,
    DT_SYMTAB       = 6,
    DT_RELA         = 7,
    DT_RELASZ       = 8,
    DT_RELAENT      = 9,
    DT_STRSZ        = 10,
    DT_SYMENT       = 11,
    DT_INIT         = 12,
    DT_FINI         = 13,
    DT_SONAME       = 14,
    DT_RPATH        = 15,
    DT_SYMBOLIC     = 16,
    DT_REL          = 17,
    DT_RELSZ        = 18,
    DT_RELENT       = 19,
    DT_PLTREL       = 20,
    DT_DEBUG        = 21,
    DT_TEXTREL      = 22,
    DT_JMPREL       = 23,
    DT_BIND_NOW     = 24,
    DT_INIT_ARRAY   = 25,
    DT_FINI_ARRAY   = 26,
    DT_INIT_ARRAYSZ = 27,
    DT_FINI_ARRAYSZ = 28,

    STB_LOCAL  = 0,
    STB_GLOBAL = 1,
    STB_WEAK   = 2,

    STT_NOTYPE  = 0,
    STT_OBJECT  = 1,
    STT_FUNC    = 2,
    STT_SECTION = 3,
    STT_FILE    = 4,

    SHN_UNDEF = 0,
    SHN_ABS   = 0xfff1,
    SHN_COMMON = 0xfff2,

    ELF_MAGIC = 0x464C457F,
};

// Result type
template<typename T>
struct Result {
    T value;
    bool ok;

    constexpr operator bool() const noexcept { return ok; }
    constexpr T& operator*() noexcept { return value; }
    constexpr const T& operator*() const noexcept { return value; }
    constexpr T* operator->() noexcept { return &value; }
    constexpr const T* operator->() const noexcept { return &value; }
};

// ELF info for loaded modules
struct ModuleInfo {
    Elf32_Addr base;
    Elf32_Addr entry;
    Elf32_Addr dynamic;
    const char* name;
    void* handle;
};

// Load an ELF executable into a new address space
Result<ModuleInfo> load_executable(const void* data, std::size_t size,
                                       const char* name);

// Load a shared library
Result<ModuleInfo> load_library(const void* data, std::size_t size,
                                    const char* name);

// Unload a module
void unload_module(ModuleInfo& module);

// Resolve a symbol in a module
Result<void*> resolve_symbol(ModuleInfo& module, const char* name);

// Get the address of a symbol by name
Result<void*> get_symbol(const char* module, const char* name);

// Dynamic linker initialization
bool initialize();

// Register a library for symbol resolution
void register_library(ModuleInfo& module);

// Unregister a library
void unregister_library(ModuleInfo& module);

// Get the number of loaded modules
std::size_t module_count() noexcept;

// Get a loaded module by index
ModuleInfo* get_module(std::size_t index) noexcept;

// Find a library by name
ModuleInfo* find_library(const char* name) noexcept;

// Relocation types
enum RelocationType : std::uint32_t {
    R_386_NONE      = 0,
    R_386_32        = 1,
    R_386_PC32      = 2,
    R_386_GOT32     = 3,
    R_386_PLT32     = 4,
    R_386_COPY      = 5,
    R_386_GLOB_DAT  = 6,
    R_386_JMP_SLOT  = 7,
    R_386_RELATIVE  = 8,
    R_386_GOTOFF    = 9,
    R_386_GOTPC     = 10,
};

// Relocation entry (REL)
struct Elf32_Rel {
    Elf32_Addr r_offset;
    Elf32_Word r_info;
};

// Relocation entry (RELA)
struct Elf32_Rela {
    Elf32_Addr  r_offset;
    Elf32_Word  r_info;
    Elf32_Sword r_addend;
};

// Apply relocations to a loaded module
bool apply_relocations(ModuleInfo& module, bool lazy);

// Hash function for symbol lookup
std::uint32_t elf_hash(const char* name) noexcept;

// Lookup a symbol in the symbol table
const Elf32_Sym* lookup_symbol(const Elf32_Sym* symtab,
                                    const char* strtab,
                                    std::size_t sym_count,
                                    const char* name) noexcept;

} // namespace kernel::elf